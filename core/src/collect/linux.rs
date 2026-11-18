//! Linux (any distribution, x64 or ARM): /sys/class/dmi (maker, model, serial number), lscpu or /proc/cpuinfo,
//! /proc/meminfo, lsblk (drives and LUKS), lspci (graphics), /sys/class/net (network cards), /etc/os-release,
//! realm (Active Directory), /sys/class/power_supply (battery) and the built-in screen's EDID.
//!
//! The serial number and memory details are only readable by root on most systems: run with sudo (or pkexec)
//! to include them; without it, the app asks for the serial number.

use super::util::{self, list, num, text};
use crate::facts::{Battery, Cpu, Disk, Facts, Gpu, Nic, Os, Ram};
use serde_json::Value;
use std::collections::HashMap;

/// A network interface as seen in /sys/class/net.
pub struct NetDevice {
    pub name: String,
    pub mac: String,
    pub wireless: bool,
    pub physical: bool,
    /// "0000:00:14.3" for cards on the PCI bus, to find their name in lspci
    pub pci_slot: String,
    /// connected by USB (an adapter or dock, which moves between computers)
    pub usb: bool,
}

/// What the system said, as given to `parse`.
#[derive(Default)]
pub struct Inputs {
    pub dmi: HashMap<String, String>,
    pub devicetree_serial: String,
    pub cpuinfo: String,
    pub lscpu: Value,
    pub meminfo: String,
    pub dmidecode_memory: String,
    pub lsblk: Value,
    pub lspci: String,
    pub nets: Vec<NetDevice>,
    pub os_release: String,
    pub realm: Option<String>,
    pub battery: Option<(f64, f64)>,
    pub edid: Option<Vec<u8>>,
    pub hostname: String,
    pub is_root: bool,
    /// `iw phy`: what the Wi-Fi card supports
    pub iw: String,
    /// `hciconfig -a`: the Bluetooth radio's version
    pub hci: String,
    /// each drive's link speed ("sda" -> "6.0 Gb/s" for SATA, "nvme0n1" -> "8.0 GT/s PCIe" for NVMe), from /sys
    pub disk_links: HashMap<String, String>,
    /// `ethtool` for each Ethernet port ("eth0" -> its output), for the speeds it supports
    pub ethtool: HashMap<String, String>,
}

/// The link speed /sys reports for a drive: SATA through its ata link, NVMe through its PCI device.
#[cfg(target_os = "linux")]
fn disk_link(name: &str) -> Option<String> {
    use util::read;
    if name.starts_with("nvme") {
        return read(&format!("/sys/block/{name}/device/device/current_link_speed"));
    }
    // /sys/block/sda points into .../ata3/host2/...; ata3's link is /sys/class/ata_link/link3
    let path = std::fs::canonicalize(format!("/sys/block/{name}")).ok()?;
    let ata = path.iter().map(|p| p.to_string_lossy().into_owned()).find(|p| p.starts_with("ata") && p[3..].chars().all(|c| c.is_ascii_digit()))?;
    read(&format!("/sys/class/ata_link/link{}/sata_spd", &ata[3..])).filter(|s| !s.contains("unknown"))
}

#[cfg(target_os = "linux")]
pub fn collect() -> Facts {
    use util::{read, run};
    let mut dmi = HashMap::new();
    for key in ["sys_vendor", "product_name", "product_version", "product_serial", "board_serial", "chassis_type"] {
        if let Some(value) = read(&format!("/sys/class/dmi/id/{key}")) {
            dmi.insert(key.to_string(), value);
        }
    }
    let mut nets = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let path = entry.path();
            nets.push(NetDevice {
                name: entry.file_name().to_string_lossy().into_owned(),
                mac: read(&format!("{}/address", path.display())).unwrap_or_default(),
                wireless: path.join("wireless").exists() || path.join("phy80211").exists(),
                physical: path.join("device").exists(),
                pci_slot: read(&format!("{}/device/uevent", path.display())).unwrap_or_default().lines()
                    .find_map(|l| l.strip_prefix("PCI_SLOT_NAME=").map(String::from)).unwrap_or_default(),
                usb: std::fs::canonicalize(path.join("device")).map(|p| p.to_string_lossy().contains("/usb")).unwrap_or(false),
            });
        }
    }
    let battery = std::fs::read_dir("/sys/class/power_supply").ok().and_then(|entries| {
        entries.flatten().map(|e| e.path()).filter(|p| p.file_name().map_or(false, |n| n.to_string_lossy().starts_with("BAT"))).find_map(|p| {
            let get = |name: &str| read(&format!("{}/{name}", p.display())).and_then(|v| v.parse::<f64>().ok());
            get("energy_full").zip(get("energy_full_design")).or_else(|| get("charge_full").zip(get("charge_full_design")))
        })
    });
    let edid = std::fs::read_dir("/sys/class/drm").ok().and_then(|entries| {
        entries.flatten().map(|e| e.path())
            .filter(|p| { let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(); n.contains("-eDP-") || n.contains("-LVDS-") || n.contains("-DSI-") })
            .find_map(|p| std::fs::read(p.join("edid")).ok().filter(|b| b.len() >= 128))
    });
    let is_root = run("id", &["-u"]).map(|u| u.trim() == "0").unwrap_or(false);
    let lsblk: Value = run("lsblk", &["-J", "-b", "-o", "NAME,SIZE,ROTA,TRAN,TYPE,RM,MODEL,MOUNTPOINT"]).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let disk_links = util::list(&lsblk["blockdevices"]).iter().map(|d| util::text(&d["name"]))
        .filter_map(|name| disk_link(&name).map(|speed| (name, speed))).collect();
    let ethtool = nets.iter().filter(|n| n.physical && !n.wireless).filter_map(|n| run("ethtool", &[&n.name]).map(|out| (n.name.clone(), out))).collect();
    parse(&Inputs {
        dmi,
        devicetree_serial: read("/sys/firmware/devicetree/base/serial-number").unwrap_or_default().trim_end_matches('\u{0}').to_string(),
        cpuinfo: read("/proc/cpuinfo").unwrap_or_default(),
        lscpu: run("lscpu", &["-J"]).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null),
        meminfo: read("/proc/meminfo").unwrap_or_default(),
        dmidecode_memory: if is_root { run("dmidecode", &["-t", "memory"]).unwrap_or_default() } else { String::new() },
        lsblk,
        lspci: run("lspci", &["-mm"]).unwrap_or_default(),
        nets,
        os_release: read("/etc/os-release").or_else(|| read("/usr/lib/os-release")).unwrap_or_default(),
        realm: run("realm", &["list", "--name-only"]),
        battery,
        edid,
        hostname: util::hostname(),
        is_root,
        iw: run("iw", &["phy"]).unwrap_or_default(),
        hci: run("hciconfig", &["-a"]).unwrap_or_default(),
        disk_links,
        ethtool,
    })
}

fn release_field(os_release: &str, key: &str) -> String {
    os_release.lines().find_map(|l| l.strip_prefix(&format!("{key}="))).map(|v| v.trim().trim_matches('"').to_string()).unwrap_or_default()
}

/// "Ubuntu" / "24.04 LTS"; "Debian GNU/Linux" -> "Debian"; "Fedora Linux" -> "Fedora".
pub fn distro_and_version(os_release: &str) -> (String, String) {
    let name = release_field(os_release, "NAME").replace(" GNU/Linux", "");
    let name = if name == "Fedora Linux" { "Fedora".to_string() } else { name };
    let id = release_field(os_release, "VERSION_ID");
    let full = release_field(os_release, "VERSION");
    let version = if full.contains("LTS") && !id.is_empty() { format!("{id} LTS") } else if id.is_empty() { full } else { id };
    (name, version)
}

fn lscpu_field(lscpu: &Value, field: &str) -> String {
    // lscpu -J lists {"field": "Model name:", "data": "..."}, possibly nested under "children"
    fn walk(items: &[Value], field: &str) -> Option<String> {
        for item in items {
            if text(&item["field"]).trim_end_matches(':') == field {
                return Some(text(&item["data"]));
            }
            if let Some(children) = item["children"].as_array() {
                if let Some(found) = walk(children, field) {
                    return Some(found);
                }
            }
        }
        None
    }
    lscpu["lscpu"].as_array().and_then(|items| walk(items, field)).unwrap_or_default()
}

fn flag(value: &Value) -> bool {
    matches!(value, Value::Bool(true)) || text(value) == "1"
}

pub fn parse(i: &Inputs) -> Facts {
    let dmi = |key: &str| i.dmi.get(key).map(|v| util::clean(v)).filter(|v| !util::is_placeholder(v)).unwrap_or_default();
    let mut f = Facts { hostname: i.hostname.clone(), ..Facts::default() };
    f.serial = [dmi("product_serial"), dmi("board_serial"), i.devicetree_serial.trim().to_string()].into_iter().find(|s| !s.is_empty()).unwrap_or_default();
    if f.serial.is_empty() && !i.is_root && !i.dmi.is_empty() {
        f.warnings.push("serial_needs_admin".into());
    }
    f.manufacturer = util::manufacturer(&dmi("sys_vendor"));
    let version = dmi("product_version");
    f.model = if f.manufacturer == "Lenovo" && !version.is_empty() { version } else { dmi("product_name") };
    if f.manufacturer.is_empty() && f.model.is_empty() {
        // single-board computers such as the Raspberry Pi say what they are in /proc/cpuinfo
        if let Some(model) = i.cpuinfo.lines().find_map(|l| l.strip_prefix("Model").and_then(|v| v.split_once(':')).map(|(_, v)| util::clean(v))) {
            f.model = model;
        }
    }
    f.chassis = i.dmi.get("chassis_type").and_then(|c| c.trim().parse::<i64>().ok()).map(util::chassis_from_smbios)
        .filter(|c| !c.is_empty()).unwrap_or(if i.battery.is_some() { "laptop" } else { "unknown" }).into();

    let cpuinfo_field = |key: &str| i.cpuinfo.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim() == key).map(|(_, v)| util::clean(v))).unwrap_or_default();
    let mut name = lscpu_field(&i.lscpu, "Model name");
    if name.is_empty() || name == "-" {
        name = cpuinfo_field("model name");
    }
    let vendor = lscpu_field(&i.lscpu, "Vendor ID");
    if name.is_empty() || name == "-" {
        name = cpuinfo_field("Hardware");
    }
    if (name.is_empty() || name == "-") && !vendor.is_empty() && vendor != "-" {
        name = vendor.clone(); // some ARM systems only name the maker
    }
    if name == "-" {
        name.clear();
    }
    if !name.is_empty() {
        if vendor == "ARM" && !name.starts_with("ARM") && !name.contains("Cortex") {
            name = format!("ARM {name}");
        }
        let per_socket: Option<u32> = lscpu_field(&i.lscpu, "Core(s) per socket").parse().ok().or_else(|| lscpu_field(&i.lscpu, "Core(s) per cluster").parse().ok());
        let sockets: u32 = lscpu_field(&i.lscpu, "Socket(s)").parse().ok().or_else(|| lscpu_field(&i.lscpu, "Cluster(s)").parse().ok()).unwrap_or(1);
        let cores = per_socket.map(|c| c * sockets.max(1)).or_else(|| cpuinfo_field("cpu cores").parse().ok());
        let max_ghz = lscpu_field(&i.lscpu, "CPU max MHz").replace(',', ".").parse::<f64>().ok().filter(|m| *m > 0.0).map(|m| (m / 10.0).round() / 100.0);
        f.cpu = Some(Cpu { name: util::cpu_name(&name), cores, max_ghz });
    }

    // memory: dmidecode (root) knows modules, type and speed; /proc/meminfo only the total
    let mut ram = Ram::default();
    let total_kb = i.meminfo.lines().find_map(|l| l.strip_prefix("MemTotal:")).and_then(|v| util::number_and_unit(v)).map(|(n, _)| n);
    ram.gb = total_kb.map(|kb| util::ram_gb_from_bytes(kb * 1024.0));
    let devices: Vec<&str> = i.dmidecode_memory.split("Memory Device").skip(1).collect();
    let installed: Vec<&&str> = devices.iter().filter(|d| d.lines().any(|l| l.trim().starts_with("Size:") && !l.contains("No Module"))).collect();
    if let Some(first) = installed.first() {
        let field = |key: &str| first.lines().find_map(|l| l.trim().strip_prefix(&format!("{key}:")).map(|v| v.trim().to_string())).unwrap_or_default();
        ram.kind = field("Type").replace("Unknown", "");
        ram.mts = util::number_and_unit(&field("Configured Memory Speed")).or_else(|| util::number_and_unit(&field("Speed"))).map(|(n, _)| n).filter(|n| *n > 0.0);
        let form = field("Form Factor");
        let socketed = form == "SODIMM" || form == "DIMM";
        ram.replaceable = if socketed { "yes" } else if form == "Row Of Chips" || ram.kind.starts_with("LP") { "no" } else { "" }.into();
        // the installed modules add up to the exact total (/proc/meminfo leaves out what firmware keeps)
        let sizes: Vec<f64> = installed.iter().filter_map(|d| d.lines().find_map(|l| l.trim().strip_prefix("Size:")).and_then(util::number_and_unit))
            .map(|(n, unit)| if unit.starts_with("MB") { n / 1024.0 } else if unit.starts_with("TB") { n * 1024.0 } else { n }).collect();
        if !sizes.is_empty() && sizes.len() == installed.len() {
            ram.gb = Some(sizes.iter().sum());
        }
        if ram.kind.starts_with("LP") {
            ram.replaceable = "no".into(); // LPDDR is soldered whatever the table says
        } else if socketed {
            ram.form_factor = if form == "SODIMM" { "SO-DIMM" } else { "DIMM" }.into();
            ram.modules = Some(installed.len() as u32);
            ram.module_gb = util::number_and_unit(&field("Size")).map(|(n, unit)| if unit.starts_with("MB") { n / 1024.0 } else { n });
        }
    }
    f.ram = Some(ram);

    for dev in list(&i.lsblk["blockdevices"]) {
        let name = text(&dev["name"]);
        let tran = text(&dev["tran"]).to_lowercase();
        if text(&dev["type"]) != "disk" || flag(&dev["rm"]) || tran == "usb"
            || ["loop", "zram", "ram", "sr", "fd", "nbd", "dm-", "md"].iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let Some(bytes) = num(&dev["size"]) else { continue };
        let (kind, connector) = if name.starts_with("mmcblk") {
            ("eMMC", "")
        } else if tran == "nvme" || name.starts_with("nvme") {
            ("SSD", "NVMe")
        } else {
            (if flag(&dev["rota"]) { "HDD" } else { "SSD" }, match tran.as_str() { "sata" => "SATA", "sas" => "SAS", _ => "" })
        };
        // the generation, when /sys says it: SATA III, NVMe PCIe 4.0...
        let connector = match (connector, i.disk_links.get(&name)) {
            ("SATA", Some(speed)) => util::sata_connector(speed).to_string(),
            ("NVMe", Some(speed)) => util::nvme_connector(speed),
            (c, _) => c.to_string(),
        };
        let replaceable = util::pc_disk_replaceable(kind, &connector, &tran, &text(&dev["model"]));
        let form_factor = util::pc_disk_form(kind, &connector, replaceable, &f.chassis).into();
        f.disks.push(Disk { gb: util::disk_gb(bytes), kind: kind.into(), connector, replaceable: replaceable.into(), form_factor });
    }

    // lspci -mm: 00:02.0 "VGA compatible controller" "Intel Corporation" "Alder Lake-P GT2 [Iris Xe Graphics]" ...
    for line in i.lspci.lines() {
        let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        let (Some(class), Some(vendor), Some(device)) = (quoted.first(), quoted.get(1), quoted.get(2)) else { continue };
        if !(class.contains("VGA") || class.contains("3D controller") || class.contains("Display controller")) {
            continue;
        }
        let brand = util::manufacturer(vendor.split_whitespace().next().unwrap_or(vendor));
        let brand = match brand.as_str() { "Advanced" => "AMD".to_string(), "NVIDIA" | "Nvidia" => "NVIDIA".to_string(), other => other.replace("Corporation", "").trim().to_string() };
        let model = device.split('[').nth(1).map(|s| s.trim_end_matches(']').to_string()).unwrap_or_else(|| device.to_string());
        let model = model.split(" / ").next().unwrap_or(&model).to_string();
        let name = util::clean(&format!("{brand} {model}"));
        let lower = name.to_lowercase();
        let kind = if lower.contains("nvidia") || lower.contains("radeon rx") || lower.contains("radeon pro") || lower.contains("arc a") {
            "dedicated"
        } else if lower.contains("intel") || lower.contains("vc4") || lower.contains("mali") {
            "integrated"
        } else {
            ""
        };
        f.gpus.push(Gpu { name, kind: kind.into(), vram_gb: None });
    }
    if f.gpus.is_empty() && i.lspci.is_empty() && f.model.contains("Raspberry Pi") {
        f.gpus.push(Gpu { name: "Broadcom VideoCore".into(), kind: "integrated".into(), vram_gb: None });
    }

    for net in &i.nets {
        if !net.physical || net.name == "lo" || net.usb {
            continue;
        }
        let mut nic = Nic { kind: if net.wireless { "wifi" } else { "eth" }.into(), mac: net.mac.clone(), ..Nic::default() };
        nic.model = pci_name(&i.lspci, &net.pci_slot);
        if !net.wireless {
            // ethtool lists the modes the port supports ("1000baseT/Full 2500baseT/Full")
            nic.speed_mbps = i.ethtool.get(&net.name).and_then(|out| {
                let supported: String = out.lines().skip_while(|l| !l.contains("Supported link modes")).take_while(|l| !l.contains("Supported pause")).collect::<Vec<_>>().join(" ");
                util::max_ethernet_mbps(&supported)
            }).or_else(|| util::ethernet_mbps_from_name(&nic.model));
        }
        if net.wireless {
            nic.wifi_gen = wifi_gen_from_iw(&i.iw);
            if nic.wifi_gen.is_empty() {
                nic.wifi_gen = util::wifi_gen_from_name(&nic.model);
            }
            nic.bt_ver = bt_from_hciconfig(&i.hci);
        }
        f.nics.push(nic);
    }

    let (distro, version) = distro_and_version(&i.os_release);
    if !distro.is_empty() {
        f.os = Some(Os { family: "linux".into(), version, edition: String::new(), distro });
    }
    // encrypted when the system itself ("/") sits on an encrypted (LUKS) volume; a second encrypted drive does not count
    f.encryption = match root_encrypted(&i.lsblk["blockdevices"], false) {
        Some(true) => "yes".into(),
        Some(false) => "no".into(),
        None => {
            // lsblk without mount points (older versions): any encrypted volume, as before
            let types: Vec<String> = { let mut t = Vec::new(); collect_types(&i.lsblk["blockdevices"], &mut t); t };
            if types.iter().any(|t| t == "crypt") { "yes".into() } else if types.is_empty() { String::new() } else { "no".into() }
        }
    };
    f.domain_joined = match &i.realm { Some(out) if !out.trim().is_empty() => "yes".into(), Some(_) => "no".into(), None => String::new() };
    if let Some((full, design)) = i.battery {
        if design > 0.0 && full > 0.0 {
            f.battery = Some(Battery { health_pct: (full / design * 100.0).round().min(100.0) });
        }
    }
    if let Some(edid) = &i.edid {
        if edid.len() > 22 {
            f.screen_inches = util::inches_from_cm(edid[21] as f64, edid[22] as f64);
        }
    }
    f
}

/// A PCI card's name from `lspci -mm`: "Intel Wi-Fi 6E AX211 160MHz", "Realtek RTL8111/8168/8411 Gigabit Ethernet".
pub fn pci_name(lspci: &str, slot: &str) -> String {
    let slot = slot.strip_prefix("0000:").unwrap_or(slot);
    if slot.is_empty() {
        return String::new();
    }
    let Some(line) = lspci.lines().find(|l| l.split_whitespace().next() == Some(slot)) else { return String::new() };
    let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
    let (Some(vendor), Some(device)) = (quoted.get(1), quoted.get(2)) else { return String::new() };
    let brand = util::manufacturer(vendor.split_whitespace().next().unwrap_or(vendor));
    let brand = match brand.as_str() { "Advanced" => "AMD".to_string(), other => other.replace("Corporation", "").trim().to_string() };
    // Intel names the chip itself in the subsystem ("Wi-Fi 6E AX211 160MHz"), others in brackets or the device name
    let sub_vendor = quoted.get(3).copied().unwrap_or("");
    let sub_device = quoted.get(4).copied().unwrap_or("");
    let model = if let Some(inside) = device.split('[').nth(1) {
        inside.trim_end_matches(']').to_string()
    } else if sub_vendor == *vendor && !sub_device.is_empty() && !sub_device.starts_with("Device ") {
        sub_device.to_string()
    } else {
        device.to_string()
    };
    let model = model.replace(" PCI Express", "").replace(" Controller", "");
    util::clean(&format!("{brand} {model}"))
}

/// `iw phy`: EHT means Wi-Fi 7, HE Wi-Fi 6 (6E with the 6 GHz band, "Band 4"), VHT Wi-Fi 5, HT Wi-Fi 4.
pub fn wifi_gen_from_iw(iw: &str) -> String {
    if iw.contains("EHT Iftypes") { "7" }
    else if iw.contains("HE Iftypes") && iw.contains("Band 4:") { "6E" }
    else if iw.contains("HE Iftypes") { "6" }
    else if iw.contains("VHT Capabilities") { "5" }
    else if iw.contains("HT Capabilities") { "4" }
    else { "" }.into()
}

/// `hciconfig -a`: "LMP Version: 5.3 (0xc)" -> "5.3".
pub fn bt_from_hciconfig(hci: &str) -> String {
    for key in ["LMP Version:", "HCI Version:"] {
        if let Some(v) = hci.lines().find_map(|l| l.split(key).nth(1)).and_then(|v| v.split_whitespace().next()) {
            if v.len() == 3 && v.as_bytes()[1] == b'.' && v.chars().filter(|c| c.is_ascii_digit()).count() == 2 {
                return v.to_string();
            }
        }
    }
    String::new()
}

/// Every device type in the lsblk tree (to spot "crypt", which LUKS uses).
/// Whether the volume mounted at "/" is (inside) an encrypted one; None when no "/" is listed.
fn root_encrypted(value: &Value, under_crypt: bool) -> Option<bool> {
    for dev in list(value) {
        let crypt = under_crypt || text(&dev["type"]) == "crypt";
        if text(&dev["mountpoint"]) == "/" {
            return Some(crypt);
        }
        if let Some(found) = root_encrypted(&dev["children"], crypt) {
            return Some(found);
        }
    }
    None
}

fn collect_types(value: &Value, out: &mut Vec<String>) {
    for dev in list(value) {
        out.push(text(&dev["type"]));
        collect_types(&dev["children"], out);
    }
}
