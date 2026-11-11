//! Windows 10 and 11: one PowerShell script reads everything through WMI/CIM and the Storage and NetAdapter
//! modules, and prints it as JSON. Nothing here needs administrator rights; the battery's design capacity is
//! sometimes only visible to administrators, in which case battery health is left out.

use super::util::{self, list, num, text};
use crate::facts::{Battery, Cpu, Disk, Facts, Gpu, Nic, Os, Ram};
use serde_json::Value;

pub const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$cs = Get-CimInstance Win32_ComputerSystem
$product = Get-CimInstance Win32_ComputerSystemProduct
$os = Get-CimInstance Win32_OperatingSystem
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$bitlocker = $null
try { $bitlocker = (New-Object -ComObject Shell.Application).NameSpace($env:SystemDrive + '\').Self.ExtendedProperty('System.Volume.BitLockerProtection') } catch {}
$full = (Get-CimInstance -Namespace root\wmi -ClassName BatteryFullChargedCapacity | Select-Object -First 1).FullChargedCapacity
$design = (Get-CimInstance -Namespace root\wmi -ClassName BatteryStaticData | Select-Object -First 1).DesignedCapacity
$internal = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorConnectionParams | Where-Object { $_.VideoOutputTechnology -eq 2147483648 -or $_.VideoOutputTechnology -eq -2147483648 } | ForEach-Object { $_.InstanceName })
$screen = Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorBasicDisplayParams | Where-Object { $internal -contains $_.InstanceName } | Select-Object -First 1
[pscustomobject]@{
  hostname = $env:COMPUTERNAME
  cs = $cs | Select-Object Manufacturer, Model, PartOfDomain, PCSystemType
  product = $product | Select-Object Name, Version, Vendor
  serial = (Get-CimInstance Win32_BIOS).SerialNumber
  chassis = @((Get-CimInstance Win32_SystemEnclosure).ChassisTypes)
  cpu = Get-CimInstance Win32_Processor | Select-Object -First 1 Name, NumberOfCores, MaxClockSpeed
  memory = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Capacity, Speed, ConfiguredClockSpeed, SMBIOSMemoryType, FormFactor)
  gpus = @(Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM)
  disks = @(Get-PhysicalDisk | ForEach-Object { [pscustomobject]@{ Size = $_.Size; MediaType = [string]$_.MediaType; BusType = [string]$_.BusType } })
  nics = @(Get-NetAdapter -Physical | ForEach-Object { [pscustomobject]@{ Description = $_.InterfaceDescription; MacAddress = $_.MacAddress; Media = [string]$_.PhysicalMediaType } })
  os = [pscustomobject]@{ Caption = $os.Caption; DisplayVersion = $cv.DisplayVersion; ReleaseId = $cv.ReleaseId; Build = $os.BuildNumber }
  bitlocker = $bitlocker
  battery = [pscustomobject]@{ Full = $full; Design = $design }
  screen = if ($screen) { [pscustomobject]@{ Width = $screen.MaxHorizontalImageSize; Height = $screen.MaxVerticalImageSize } } else { $null }
} | ConvertTo-Json -Depth 4 -Compress
"#;

#[cfg(windows)]
pub fn collect() -> Facts {
    let output = util::run("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT]);
    match output.and_then(|s| serde_json::from_str::<Value>(s.trim_start_matches('\u{feff}')).ok()) {
        Some(json) => parse(&json),
        None => Facts { hostname: std::env::var("COMPUTERNAME").unwrap_or_default(), warnings: vec!["windows_tools_failed".into()], ..Facts::default() },
    }
}

/// SMBIOS memory type codes -> names.
fn memory_type(code: i64) -> &'static str {
    match code {
        20 => "DDR", 21 => "DDR2", 24 => "DDR3", 26 => "DDR4", 34 => "DDR5", 27 => "LPDDR", 28 => "LPDDR2",
        29 => "LPDDR3", 30 => "LPDDR4", 35 => "LPDDR5", _ => "",
    }
}

pub fn parse(j: &Value) -> Facts {
    let mut f = Facts { hostname: text(&j["hostname"]), serial: text(&j["serial"]), ..Facts::default() };
    let cs = &j["cs"];
    f.manufacturer = util::manufacturer(&text(&cs["Manufacturer"]));
    // Lenovo keeps the readable model name ("ThinkPad T14 Gen 4") in the product version, not in "Model" ("21HD003XSP")
    let version = text(&j["product"]["Version"]);
    let model = text(&cs["Model"]);
    f.model = if f.manufacturer == "Lenovo" && !util::is_placeholder(&version) { version } else if util::is_placeholder(&model) { text(&j["product"]["Name"]) } else { model };
    if util::is_placeholder(&f.model) {
        f.model.clear();
    }
    let chassis = list(&j["chassis"]).iter().filter_map(|c| num(c)).map(|c| util::chassis_from_smbios(c as i64)).find(|c| !c.is_empty());
    f.chassis = chassis.unwrap_or(if num(&cs["PCSystemType"]) == Some(2.0) { "laptop" } else { "unknown" }).into();

    let cpu = &j["cpu"];
    if !text(&cpu["Name"]).is_empty() {
        f.cpu = Some(Cpu {
            name: util::cpu_name(&text(&cpu["Name"])),
            cores: num(&cpu["NumberOfCores"]).map(|n| n as u32),
            max_ghz: num(&cpu["MaxClockSpeed"]).filter(|m| *m > 0.0).map(|m| (m / 10.0).round() / 100.0),
        });
    }

    let modules = list(&j["memory"]);
    let total: f64 = modules.iter().filter_map(|m| num(&m["Capacity"])).sum();
    let first = modules.first();
    let kind = first.and_then(|m| num(&m["SMBIOSMemoryType"])).map(|c| memory_type(c as i64)).unwrap_or("");
    let form = first.and_then(|m| num(&m["FormFactor"])).unwrap_or(0.0) as i64;
    let socketed = form == 8 || form == 12; // DIMM or SO-DIMM
    f.ram = Some(Ram {
        gb: (total > 0.0).then(|| util::ram_gb_from_bytes(total)),
        kind: kind.into(),
        mts: first.and_then(|m| num(&m["ConfiguredClockSpeed"]).filter(|s| *s > 0.0).or_else(|| num(&m["Speed"]))).filter(|s| *s > 0.0),
        replaceable: if socketed { "yes" } else if kind.starts_with("LP") { "no" } else { "" }.into(),
        modules: socketed.then_some(modules.len() as u32),
        module_gb: if socketed { first.and_then(|m| num(&m["Capacity"])).map(|c| (c / 1024.0 / 1024.0 / 1024.0).round()) } else { None },
        form_factor: match form { 12 => "SO-DIMM", 8 => "DIMM", _ => "" }.into(),
    });

    for gpu in list(&j["gpus"]) {
        let name = util::clean(&text(&gpu["Name"]));
        let lower = name.to_lowercase();
        if name.is_empty() || ["basic display", "basic render", "remote", "virtual", "parsec", "mirage", "citrix", "displaylink", "idd "].iter().any(|s| lower.contains(s)) {
            continue;
        }
        let dedicated = lower.contains("nvidia") || lower.contains("radeon rx") || lower.contains("radeon pro") || (lower.contains("arc") && lower.contains(" a"));
        let integrated = lower.contains("uhd") || lower.contains("iris") || lower.contains("intel(r) graphics") || lower.contains("intel graphics")
            || lower.contains("radeon graphics") || lower.contains("adreno") || lower.contains("radeon(tm) graphics") || (lower.contains("radeon") && lower.contains("m"));
        let vram = num(&gpu["AdapterRAM"]).filter(|b| *b > 256.0 * 1024.0 * 1024.0).map(|b| (b / 1024.0 / 1024.0 / 1024.0).round());
        f.gpus.push(Gpu { name, kind: if dedicated { "dedicated" } else if integrated { "integrated" } else { "" }.into(), vram_gb: if dedicated { vram } else { None } });
    }

    for disk in list(&j["disks"]) {
        let bus = text(&disk["BusType"]);
        // removable or virtual drives (by name, or by number if Windows wrote the number): USB, SD, iSCSI, virtual
        if ["USB", "SD", "iSCSI", "Virtual", "FileBackedVirtual", "7", "9", "12", "14", "15"].contains(&bus.as_str()) {
            continue;
        }
        let Some(bytes) = num(&disk["Size"]) else { continue };
        let media = text(&disk["MediaType"]);
        let kind = match media.as_str() { "SSD" | "4" => "SSD", "HDD" | "3" => "HDD", "SCM" => "SSD", _ if bus == "NVMe" || bus == "17" => "SSD", _ if bus == "MMC" => "eMMC", _ => "" };
        let connector = match bus.as_str() { "NVMe" | "17" => "NVMe", "SATA" | "11" => "SATA", "SAS" | "10" => "SAS", "RAID" | "8" => "RAID", _ => "" };
        f.disks.push(Disk { gb: util::disk_gb(bytes), kind: kind.into(), connector: connector.into(), replaceable: String::new() });
    }

    for nic in list(&j["nics"]) {
        let media = text(&nic["Media"]).to_lowercase();
        let kind = if media.contains("802.11") || media.contains("wireless") { "wifi" } else if media.contains("802.3") { "eth" } else { continue };
        f.nics.push(Nic { kind: kind.into(), model: util::clean(&text(&nic["Description"])), mac: text(&nic["MacAddress"]) });
    }

    // "Microsoft Windows 11 Pro" + "23H2" -> version "11 23H2", edition "Pro"
    let caption = text(&j["os"]["Caption"]);
    let after = caption.split("Windows").nth(1).unwrap_or("").trim().to_string();
    let mut words = after.split_whitespace();
    let major = words.next().unwrap_or("").to_string();
    let edition = words.collect::<Vec<_>>().join(" ");
    let release = [text(&j["os"]["DisplayVersion"]), text(&j["os"]["ReleaseId"])].into_iter().find(|r| !r.is_empty()).unwrap_or_default();
    if !major.is_empty() {
        f.os = Some(Os { family: "win".into(), version: format!("{major} {release}").trim().to_string(), edition, distro: String::new() });
    }

    // BitLocker on the system drive: 1 on, 3 encrypting, 5 suspended (still encrypted) / 2 off
    f.encryption = match num(&j["bitlocker"]).map(|v| v as i64) { Some(1) | Some(3) | Some(5) => "yes", Some(2) | Some(0) => "no", _ => "" }.into();
    f.domain_joined = match &cs["PartOfDomain"] { Value::Bool(true) => "yes", Value::Bool(false) => "no", _ => "" }.into();

    if let (Some(full), Some(design)) = (num(&j["battery"]["Full"]), num(&j["battery"]["Design"])) {
        if design > 0.0 && full > 0.0 {
            f.battery = Some(Battery { health_pct: (full / design * 100.0).round().min(100.0) });
        }
    } else if f.chassis == "laptop" {
        f.warnings.push("battery_needs_admin".into());
    }
    if let (Some(w), Some(h)) = (num(&j["screen"]["Width"]), num(&j["screen"]["Height"])) {
        f.screen_inches = util::inches_from_cm(w, h);
    }
    if f.serial.is_empty() || util::is_placeholder(&f.serial) {
        f.serial.clear();
    }
    f
}
