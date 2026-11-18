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
    /// `nvram` lines for the variables OpenCore and OpenCore Legacy Patcher set at startup (empty when absent)
    pub nvram_oclp: String,
    pub nvram_opencore: String,
    /// OpenCore Legacy Patcher's record of its system patches, as JSON (empty when the Mac was never patched)
    pub oclp_patches: String,
    /// `ifconfig -m` for each Ethernet port ("en0" -> its output), for the speeds it supports
    pub media: std::collections::HashMap<String, String>,
}

/// The vendor id OpenCore puts its NVRAM variables under.
const OPENCORE_GUID: &str = "4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102";
const OCLP_PATCHES: &str = "/System/Library/CoreServices/OpenCore-Legacy-Patcher.plist";

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
                                                "SPSerialATADataType", "SPDisplaysDataType", "SPPowerDataType", "SPEthernetDataType"])
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    let hardware_ports = run("networksetup", &["-listallhardwareports"]).unwrap_or_default();
    let media = hardware_ports.lines().filter_map(|l| l.strip_prefix("Device:").map(|d| d.trim().to_string()))
        .filter(|d| d.starts_with("en"))
        .filter_map(|d| run("ifconfig", &["-m", &d]).map(|out| (d, out)))
        .collect();
    let mut inputs = Inputs {
        profiler,
        cpu_brand: run("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default(),
        product_version: run("sw_vers", &["-productVersion"]).unwrap_or_default(),
        hardware_ports,
        filevault: run("fdesetup", &["status"]).unwrap_or_default(),
        directory: run("dsconfigad", &["-show"]),
        ioreg_battery: run("ioreg", &["-rn", "AppleSmartBattery"]).unwrap_or_default(),
        hostname: util::hostname(),
        screen_mm: builtin_screen_mm(),
        nvram_oclp: run("nvram", &[&format!("{OPENCORE_GUID}:OCLP-Version")]).unwrap_or_default(),
        nvram_opencore: run("nvram", &[&format!("{OPENCORE_GUID}:opencore-version")]).unwrap_or_default(),
        oclp_patches: if std::path::Path::new(OCLP_PATCHES).exists() {
            run("plutil", &["-convert", "json", "-o", "-", OCLP_PATCHES]).unwrap_or_default()
        } else {
            String::new()
        },
        media,
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
    // newer Macs give the chip number (0x4378 -> BCM4378, 0x7932 -> MT7932); older ones a short Apple card number
    // (0x111 on a 2013 iMac), which is not a chip: then only the maker is known
    let is_chip = chip.len() >= 4;
    match vendor.as_str() {
        "0x14e4" if is_chip => format!("Broadcom BCM{chip}"),
        "0x14e4" => "Broadcom".into(),
        "0x14c3" if is_chip => format!("MediaTek MT{chip}"),
        "0x14c3" => "MediaTek".into(),
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

/// The value of an `nvram` line ("GUID:name\tvalue"), without the trailing NUL bytes nvram shows as %00.
fn nvram_value(line: &str) -> String {
    line.split_once('\t').map(|(_, v)| v.replace("%00", "").trim().to_string()).unwrap_or_default()
}

/// OpenCore Legacy Patcher's version, from what it set at startup or from its record of system patches; "yes" when
/// the Mac started through OpenCore without saying which patcher; "" otherwise.
pub fn oclp(input: &Inputs) -> String {
    let clean = |v: String| v.trim_start_matches(['v', 'V']).to_string();
    let booted = nvram_value(&input.nvram_oclp);
    if !booted.is_empty() {
        return clean(booted);
    }
    let patched = serde_json::from_str::<Value>(&input.oclp_patches).ok()
        .map(|j| text(&j["OpenCore Legacy Patcher"])).unwrap_or_default();
    if !patched.is_empty() {
        return clean(patched);
    }
    if nvram_value(&input.nvram_opencore).is_empty() { String::new() } else { "yes".into() }
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
    f.oclp = Some(oclp(input));
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
        // laptops, iMacs and the Mac mini take laptop-size SO-DIMM modules; only the Mac Pro takes full-size DIMMs
        ram.form_factor = if identifier.starts_with("MacPro") { "DIMM" } else { "SO-DIMM" }.into();
    }
    f.ram = Some(ram);

    // drives (internal only); a SATA controller's negotiated link speed gives the SATA generation
    let mut drives: Vec<(Value, String)> = items(&input.profiler, "SPNVMeDataType").into_iter().map(|d| (d, "NVMe".to_string())).collect();
    for controller in section(&input.profiler, "SPSerialATADataType") {
        let link = |v: &Value| [&v["spsata_negotiatedlinkspeed"], &v["spsata_portspeed"]].iter().map(|x| text(x)).find(|x| !x.is_empty()).unwrap_or_default();
        let nested = util::list(&controller["_items"]);
        let drives_here: Vec<Value> = if nested.is_empty() { vec![controller.clone()] } else { nested.into_iter().cloned().collect() };
        for drive in drives_here {
            let speed = if link(&drive).is_empty() { link(controller) } else { link(&drive) };
            // 2013-2015 Macs list their PCIe SSD here too, with a speed in GT/s (PCIe), not Gigabit (SATA)
            let connector = if speed.contains("GT/s") {
                let generation = util::pcie_generation(&speed);
                if generation.is_empty() { "PCIe".to_string() } else { format!("PCIe {generation}") }
            } else {
                util::sata_connector(&speed).to_string()
            };
            drives.push((drive, connector));
        }
    }
    for (drive, connector) in drives {
        if text(&drive["detachable_drive"]) == "yes" || text(&drive["removable_media"]) == "yes" {
            continue;
        }
        let Some(bytes) = num(&drive["size_in_bytes"]) else { continue };
        let medium = text(&drive["spsata_medium_type"]);
        let kind = if connector == "NVMe" || connector.starts_with("PCIe") || medium.contains("Solid") { "SSD" } else if medium.contains("Rotational") { "HDD" } else { "" };
        let apple_ssd = kind == "SSD" && (apple_silicon || text(&drive["_name"]).to_uppercase().starts_with("APPLE SSD") || text(&drive["device_model"]).to_uppercase().starts_with("APPLE SSD"));
        f.disks.push(Disk { gb: util::disk_gb_sold(bytes, apple_ssd), kind: kind.into(), connector, replaceable: util::mac_disk_replaceable(&identifier, apple_silicon).into(), form_factor: String::new() });
    }

    // graphics
    for gpu in items(&input.profiler, "SPDisplaysDataType") {
        let name = text(&gpu["sppci_model"]);
        if name.is_empty() {
            continue;
        }
        // AMD (ATI) and NVIDIA chips in Macs are separate cards; Intel and Apple graphics are built into the processor
        let vendor = text(&gpu["spdisplays_vendor"]).to_lowercase();
        let lower = name.to_lowercase();
        let dedicated = ["sppci_vendor_amd", "sppci_vendor_ati", "sppci_vendor_nvidia", "amd", "ati", "nvidia"].contains(&vendor.as_str())
            || ["radeon", "geforce", "quadro", "firepro", "nvidia"].iter().any(|w| lower.contains(w));
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
        } else if port == "Ethernet" || (port.starts_with("Ethernet ") && !port.contains("Adapter")) {
            "eth" // the Mac's own ports ("Ethernet", "Ethernet 1"); USB and Thunderbolt adapters move between computers
        } else {
            continue; // adapters, Thunderbolt bridges, Bluetooth, iPhone USB, VPNs
        };
        let mut nic = Nic { kind: kind.into(), mac, ..Nic::default() };
        if kind == "eth" {
            if let Some(port) = section(&input.profiler, "SPEthernetDataType").into_iter().find(|e| text(&e["spethernet_BSD_Device_Name"]) == device) {
                nic.model = ethernet_chip(port);
            }
            nic.speed_mbps = input.media.get(&device).and_then(|out| util::max_ethernet_mbps(out)).or_else(|| util::ethernet_mbps_from_name(&nic.model));
        }
        if kind == "wifi" {
            if let Some(card) = airport.iter().find(|c| text(&c["_name"]) == device).or_else(|| airport.first()) {
                let six_ghz = util::list(&card["spairport_supported_channels"]).iter().any(|c| text(c).contains("6GHz"));
                nic.wifi_gen = util::wifi_gen_from_standards(&text(&card["spairport_supported_phymodes"]), six_ghz);
                nic.model = wifi_chip(&text(&card["spairport_wireless_card_type"]));
            }
        }
        f.nics.push(nic);
    }

    // the drives' form factor needs the screen size (a 27-inch iMac's hard drive is 3.5")
    for disk in &mut f.disks {
        disk.form_factor = util::mac_disk_form(&identifier, &disk.kind, &disk.connector, &disk.replaceable, f.screen_inches).into();
    }
    f
}

/// A built-in Ethernet port's chip: its own name when macOS gives one, else the maker and PCI device id.
pub fn ethernet_chip(port: &Value) -> String {
    let name = text(&port["_name"]);
    if !name.is_empty() && !name.eq_ignore_ascii_case("ethernet") && name.chars().any(|c| c.is_ascii_digit()) {
        return name;
    }
    let device = text(&port["spethernet_device-id"]).to_lowercase();
    let vendor = match text(&port["spethernet_vendor-id"]).to_lowercase().as_str() {
        "0x14e4" => "Broadcom", "0x8086" => "Intel", "0x10ec" => "Realtek", "0x11ab" | "0x1b4b" => "Marvell",
        "0x1d6a" => "Aquantia", "0x106b" => "Apple", _ => return name,
    };
    // the PCI device id is not the chip's name; the chips Macs used are known
    let chip = match (vendor, device.as_str()) {
        ("Broadcom", "0x16b4") => "BCM57765", ("Broadcom", "0x1682") => "BCM57762", ("Broadcom", "0x1686") => "BCM57766",
        ("Broadcom", "0x16b0") => "BCM57761", ("Broadcom", "0x1684") => "BCM5764M", ("Broadcom", "0x16a3") => "BCM57786",
        ("Aquantia", "0x07b1") => "AQC107", ("Aquantia", "0x87b1") => "AQC107", ("Aquantia", "0x04c0") => "AQC113",
        ("Intel", "0x1533") => "I210", ("Intel", "0x15f3") => "I225-V",
        _ => "",
    };
    if chip.is_empty() { format!("{vendor} Ethernet") } else { format!("{vendor} {chip}") }
}
