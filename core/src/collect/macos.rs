//! macOS: system_profiler (hardware, memory, drives, graphics, battery), sysctl (processor), networksetup
//! (network cards; Wi-Fi chip and generation from system_profiler), fdesetup (FileVault), dsconfigad (Active Directory). None of them need administrator rights.

use super::util::{self, num, number_and_unit, text};
use crate::facts::{Battery, Cpu, Disk, Facts, Gpu, Nic, Os, Ram};
use serde_json::Value;

/// What the system tools printed, as given to `parse`.
#[derive(Default)]
pub struct Inputs {
    pub profiler: Value,
    pub cpu_brand: String,
    pub product_version: String,
    pub hardware_ports: String,
    pub filevault: String,
    pub directory: Option<String>,
    pub ioreg_battery: String,
    pub hostname: String,
    /// width and height of the built-in screen in millimetres (from CoreGraphics), when it is on
    pub screen_mm: Option<(f64, f64)>,
}

#[cfg(target_os = "macos")]
pub fn collect() -> Facts {
    use util::run;
    // the Wi-Fi details take macOS several seconds (it looks at nearby networks), so they are read alongside the rest
    let airport = std::thread::spawn(|| {
        run("system_profiler", &["-json", "-detailLevel", "mini", "SPAirPortDataType"])
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .map(|v| v["SPAirPortDataType"].clone())
            .unwrap_or(Value::Null)
    });
    let profiler = run("system_profiler", &["-json", "SPHardwareDataType", "SPMemoryDataType", "SPNVMeDataType",
                                                "SPSerialATADataType", "SPDisplaysDataType", "SPPowerDataType"])
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let mut inputs = Inputs {
        profiler,
        cpu_brand: run("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default(),
        product_version: run("sw_vers", &["-productVersion"]).unwrap_or_default(),
        hardware_ports: run("networksetup", &["-listallhardwareports"]).unwrap_or_default(),
        filevault: run("fdesetup", &["status"]).unwrap_or_default(),
        directory: run("dsconfigad", &["-show"]),
        ioreg_battery: run("ioreg", &["-rn", "AppleSmartBattery"]).unwrap_or_default(),
        hostname: util::hostname(),
        screen_mm: builtin_screen_mm(),
    };
    inputs.profiler["SPAirPortDataType"] = airport.join().unwrap_or(Value::Null);
    parse(&inputs)
}

/// The built-in screen's size in millimetres, as CoreGraphics reports it. Needs no permission. None for Macs
/// without a built-in screen, or a laptop running with its lid closed.
#[cfg(target_os = "macos")]
fn builtin_screen_mm() -> Option<(f64, f64)> {
    #[repr(C)]
    struct Size { width: f64, height: f64 }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGGetOnlineDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
        fn CGDisplayIsBuiltin(display: u32) -> u32;
        fn CGDisplayScreenSize(display: u32) -> Size;
    }
    let mut displays = [0u32; 16];
    let mut count = 0u32;
    // SAFETY: the list has room for 16 displays and CoreGraphics writes at most that many
    if unsafe { CGGetOnlineDisplayList(16, displays.as_mut_ptr(), &mut count) } != 0 {
        return None;
    }
    displays.iter().take(count as usize).find(|d| unsafe { CGDisplayIsBuiltin(**d) } != 0).map(|d| {
        let size = unsafe { CGDisplayScreenSize(*d) };
        (size.width, size.height)
    }).filter(|(w, h)| *w > 0.0 && *h > 0.0)
}

/// "spairport_wireless_card_type_wifi (0x14C3, 0x7932)" -> "MediaTek MT7932": the maker's PCI vendor id and the chip.
pub fn wifi_chip(card_type: &str) -> String {
    let ids: Vec<String> = card_type.split(['(', ')', ',']).map(|p| p.trim().to_lowercase()).filter(|p| p.starts_with("0x")).collect();
    let (Some(vendor), Some(device)) = (ids.first(), ids.get(1)) else { return String::new() };
    let chip = device.trim_start_matches("0x").to_uppercase();
    match vendor.as_str() {
        "0x14e4" => format!("Broadcom BCM{chip}"),
        "0x14c3" => format!("MediaTek MT{chip}"),
        "0x106b" => "Apple".into(),
        "0x8086" => "Intel".into(),
        "0x168c" | "0x17cb" => "Qualcomm".into(),
        _ => String::new(),
    }
}

/// The marketing name for each macOS version number; newer versions are shown by number.
pub fn version_name(product_version: &str) -> String {
    let parts: Vec<&str> = product_version.trim().split('.').collect();
    let major: u32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let name = match (major, minor) {
        (10, 15) => "Catalina", (11, _) => "Big Sur", (12, _) => "Monterey", (13, _) => "Ventura",
        (14, _) => "Sonoma", (15, _) => "Sequoia", (26, _) => "Tahoe", _ => "",
    };
    let number = if major == 10 { format!("10.{minor}") } else { major.to_string() };
    if major == 0 { String::new() } else if name.is_empty() { number } else { format!("{number} {name}") }
}

fn section<'a>(profiler: &'a Value, name: &str) -> Vec<&'a Value> {
    util::list(&profiler[name])
}

/// Every item of a system_profiler section, including the ones nested under "_items".
fn items(profiler: &Value, name: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for entry in section(profiler, name) {
        let nested = util::list(&entry["_items"]);
        if nested.is_empty() { out.push(entry.clone()) } else { out.extend(nested.into_iter().cloned()) }
    }
    out
}

pub fn parse(input: &Inputs) -> Facts {
    let mut f = Facts::default();
    let hw = section(&input.profiler, "SPHardwareDataType").into_iter().next().cloned().unwrap_or(Value::Null);
    f.serial = text(&hw["serial_number"]);
    f.hostname = input.hostname.clone();
    f.manufacturer = "Apple".into();
    let machine = text(&hw["machine_name"]);
    let identifier = text(&hw["machine_model"]);
    f.model = match (machine.is_empty(), identifier.is_empty()) {
        (false, false) => format!("{machine} ({identifier})"),
        (false, true) => machine.clone(),
        _ => identifier.clone(),
    };
    f.chassis = if machine.contains("MacBook") { "laptop" } else if machine.contains("iMac") { "all_in_one" } else if machine.is_empty() { "unknown" } else { "desktop" }.into();

    // processor: "Apple M3 Pro" on Apple silicon, "Intel(R) Core(TM) i7-8850H CPU @ 2.60GHz" on Intel
    let brand = if input.cpu_brand.trim().is_empty() { text(&hw["chip_type"]) } else { input.cpu_brand.clone() };
    let cores = match &hw["number_processors"] {
        Value::String(s) => number_and_unit(s).map(|(n, _)| n as u32), // "proc 11:5:6" -> 11
        other => num(other).map(|n| n as u32),
    };
    let max_ghz = number_and_unit(&text(&hw["current_processor_speed"])).map(|(n, unit)| if unit.starts_with("MHz") { n / 1000.0 } else { n });
    if !brand.trim().is_empty() {
        f.cpu = Some(Cpu { name: util::cpu_name(&brand), cores, max_ghz });
    }

    // memory
    let mut ram = Ram { gb: number_and_unit(&text(&hw["physical_memory"])).map(|(n, unit)| if unit.starts_with("MB") { n / 1024.0 } else { n }), ..Ram::default() };
    let mem = items(&input.profiler, "SPMemoryDataType");
    let modules: Vec<&Value> = mem.iter().filter(|m| { let s = text(&m["dimm_size"]); !s.is_empty() && s != "empty" }).collect();
    if let Some(first) = mem.iter().find(|m| !text(&m["dimm_type"]).is_empty() && text(&m["dimm_type"]) != "empty") {
        ram.kind = text(&first["dimm_type"]);
        ram.mts = number_and_unit(&text(&first["dimm_speed"])).map(|(n, _)| n);
    }
    let upgradeable = mem.iter().chain(section(&input.profiler, "SPMemoryDataType").into_iter())
        .map(|m| text(&m["is_memory_upgradeable"]).to_lowercase()).find(|v| !v.is_empty());
    let apple_silicon = brand.starts_with("Apple");
    ram.replaceable = match upgradeable.as_deref() {
        Some("yes") => "yes".into(),
        Some("no") => "no".into(),
        _ if apple_silicon => "no".into(), // memory is part of the chip
        _ => String::new(),
    };
    if ram.replaceable == "yes" && !modules.is_empty() {
        ram.modules = Some(modules.len() as u32);
        ram.module_gb = number_and_unit(&text(&modules[0]["dimm_size"])).map(|(n, _)| n);
        ram.form_factor = if f.chassis == "laptop" { "SO-DIMM" } else { "DIMM" }.into();
    }
    f.ram = Some(ram);

    // drives (internal only)
    for (section_name, connector) in [("SPNVMeDataType", "NVMe"), ("SPSerialATADataType", "SATA")] {
        for drive in items(&input.profiler, section_name) {
            if text(&drive["detachable_drive"]) == "yes" || text(&drive["removable_media"]) == "yes" {
                continue;
            }
            let Some(bytes) = num(&drive["size_in_bytes"]) else { continue };
            let medium = text(&drive["spsata_medium_type"]);
            let kind = if connector == "NVMe" || medium.contains("Solid") { "SSD" } else if medium.contains("Rotational") { "HDD" } else { "" };
            f.disks.push(Disk { gb: util::disk_gb(bytes), kind: kind.into(), connector: connector.into(), replaceable: if apple_silicon { "no".into() } else { String::new() } });
        }
    }

    // graphics
    for gpu in items(&input.profiler, "SPDisplaysDataType") {
        let name = text(&gpu["sppci_model"]);
        if name.is_empty() {
            continue;
        }
        let vendor = text(&gpu["spdisplays_vendor"]).to_lowercase() + &name.to_lowercase();
        let dedicated = vendor.contains("amd") || vendor.contains("ati") || vendor.contains("nvidia") || vendor.contains("radeon");
        let vram = [&gpu["spdisplays_vram"], &gpu["sppci_vram"], &gpu["_spdisplays_vram"]].iter()
            .find_map(|v| number_and_unit(&text(v))).map(|(n, unit)| if unit.starts_with("MB") { n / 1024.0 } else { n });
        let name = if name.starts_with("Apple") && !name.ends_with("GPU") { format!("{name} GPU") } else { name };
        f.gpus.push(Gpu { name, kind: if dedicated { "dedicated" } else { "integrated" }.into(), vram_gb: if dedicated { vram } else { None } });
    }

    // battery health: system_profiler's "Maximum Capacity", else ioreg's capacities
    for power in section(&input.profiler, "SPPowerDataType") {
        let pct = number_and_unit(&text(&power["sppower_battery_health_info"]["sppower_battery_health_maximum_capacity"]));
        if let Some((value, _)) = pct {
            f.battery = Some(Battery { health_pct: value });
        }
    }
    if f.battery.is_none() && f.chassis == "laptop" {
        let field = |name: &str| input.ioreg_battery.lines().find_map(|l| {
            let l = l.trim().trim_start_matches('|').trim();
            l.strip_prefix(&format!("\"{name}\" = ")).and_then(|v| v.trim().parse::<f64>().ok())
        });
        let design = field("DesignCapacity");
        let max = field("AppleRawMaxCapacity").or_else(|| field("NominalChargeCapacity")).or_else(|| field("MaxCapacity"));
        if let (Some(max), Some(design)) = (max, design) {
            if design > 100.0 && max > 0.0 {
                f.battery = Some(Battery { health_pct: (max / design * 100.0).round().min(100.0) });
            }
        }
    }

    if let Some((w, h)) = input.screen_mm {
        f.screen_inches = util::inches_from_cm(w / 10.0, h / 10.0);
    }

    f.os = Some(Os { family: "mac".into(), version: version_name(&input.product_version), ..Os::default() });
    f.encryption = if input.filevault.contains("FileVault is On") { "yes" } else if input.filevault.contains("FileVault is Off") { "no" } else { "" }.into();
    f.domain_joined = match &input.directory {
        Some(out) if out.contains("Active Directory Domain") => "yes".into(),
        Some(_) => "no".into(),
        None => String::new(),
    };

    // network cards: built-in Wi-Fi and Ethernet ports
    let airport: Vec<Value> = items(&input.profiler, "SPAirPortDataType").into_iter()
        .flat_map(|entry| util::list(&entry["spairport_airport_interfaces"]).into_iter().cloned().collect::<Vec<_>>())
        .collect();
    for block in input.hardware_ports.split("\n\n") {
        let field = |name: &str| block.lines().find_map(|l| l.strip_prefix(name).map(|v| v.trim().to_string())).unwrap_or_default();
        let port = field("Hardware Port:");
        let mac = field("Ethernet Address:");
        let device = field("Device:");
        let kind = if port.contains("Wi-Fi") || port.contains("AirPort") {
            "wifi"
        } else if port == "Ethernet" || port.starts_with("Ethernet ") || port.contains("Ethernet Adapter") && !port.contains("Thunderbolt") {
            "eth"
        } else {
            continue; // Thunderbolt bridges, Bluetooth, iPhone USB, VPNs
        };
        let mut nic = Nic { kind: kind.into(), mac, ..Nic::default() };
        if kind == "wifi" {
            if let Some(card) = airport.iter().find(|c| text(&c["_name"]) == device).or_else(|| airport.first()) {
                let six_ghz = util::list(&card["spairport_supported_channels"]).iter().any(|c| text(c).contains("6GHz"));
                nic.wifi_gen = util::wifi_gen_from_standards(&text(&card["spairport_supported_phymodes"]), six_ghz);
                nic.model = wifi_chip(&text(&card["spairport_wireless_card_type"]));
            }
        }
        f.nics.push(nic);
    }
    f
}
