//! Windows 10 and 11: one PowerShell script reads everything through WMI/CIM and the Storage and NetAdapter
//! modules, and prints it as JSON. Nothing here needs administrator rights. Battery health comes from WMI's battery
//! classes, which many laptops leave empty (even for administrators); then from Windows' own battery report
//! (powercfg /batteryreport), which reads the same figures the way Settings does.

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
$hasBattery = [bool](Get-CimInstance Win32_Battery)
$cycles = (Get-CimInstance -Namespace root\wmi -ClassName BatteryCycleCount | Select-Object -First 1).CycleCount
if ($hasBattery -and (-not $full -or -not $design -or -not $cycles)) {
  # Windows' battery report: an XML file with each battery's design and full-charge capacity
  $report = Join-Path $env:TEMP ('inventario-battery-' + [guid]::NewGuid() + '.xml')
  powercfg /batteryreport /xml /output $report | Out-Null
  if (Test-Path $report) {
    $b = ([xml](Get-Content $report -Raw)).BatteryReport.Batteries.Battery | Select-Object -First 1
    if ($b) {
      if (-not $full -or -not $design) { $design = [double]$b.DesignCapacity; $full = [double]$b.FullChargeCapacity }
      if (-not $cycles -and $b.CycleCount) { $cycles = [int]$b.CycleCount }
    }
    Remove-Item $report -Force
  }
}
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
# the built-in screen: "internal", or the connections only built-in panels use (LVDS 6, embedded DisplayPort 11, UDI 13)
$internal = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorConnectionParams | Where-Object { @(2147483648, -2147483648, 6, 11, 13) -contains $_.VideoOutputTechnology } | ForEach-Object { $_.InstanceName })
# Wi-Fi standards the card supports ("802.11ax", the same in every language) and the Bluetooth radio's LMP version
$wlan = (netsh wlan show drivers) -join "`n"
$lmp = Get-PnpDevice -Class Bluetooth -PresentOnly | ForEach-Object { (Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName '{A92F26CA-EDA7-4B1D-9DB2-27B68AA5A2EB} 4').Data } | Where-Object { $_ -ne $null } | Select-Object -First 1
# video memory: Win32_VideoController's AdapterRAM stops at 4 GB, the driver's registry entry has the real size
$vram = @{}
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' | ForEach-Object { $q = $_.'HardwareInformation.qwMemorySize'; if ($q -and $_.DriverDesc) { try { $vram[$_.DriverDesc] = if ($q -is [byte[]]) { [BitConverter]::ToUInt64($q, 0) } else { [uint64]$q } } catch {} } }
$screen = Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorBasicDisplayParams | Where-Object { $internal -contains $_.InstanceName } | Select-Object -First 1
[pscustomobject]@{
  hostname = $env:COMPUTERNAME
  cs = $cs | Select-Object Manufacturer, Model, PartOfDomain, PCSystemType
  product = $product | Select-Object Name, Version, Vendor
  serial = (Get-CimInstance Win32_BIOS).SerialNumber
  chassis = @((Get-CimInstance Win32_SystemEnclosure).ChassisTypes)
  cpu = $(($p = @(Get-CimInstance Win32_Processor)) | Select-Object -First 1 Name, MaxClockSpeed, @{ n = 'NumberOfCores'; e = { ($p | Measure-Object NumberOfCores -Sum).Sum } })
  memory = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Capacity, Speed, ConfiguredClockSpeed, SMBIOSMemoryType, FormFactor)
  gpus = @(Get-CimInstance Win32_VideoController | ForEach-Object { [pscustomobject]@{ Name = $_.Name; AdapterRAM = $_.AdapterRAM; Vram = $vram[$_.Name] } })
  disks = @(Get-PhysicalDisk | ForEach-Object {
    # the drive's own health data (SMART), read only: wear and hours need administrator rights, which the app has
    $r = $_ | Get-StorageReliabilityCounter
    [pscustomobject]@{ Size = $_.Size; MediaType = [string]$_.MediaType; BusType = [string]$_.BusType; Model = [string]$_.FriendlyName; Spindle = $_.SpindleSpeed;
      Health = [string]$_.HealthStatus; Wear = $r.Wear; Hours = $r.PowerOnHours } })
  nics = @(Get-NetAdapter -Physical | ForEach-Object { [pscustomobject]@{ Description = $_.InterfaceDescription; MacAddress = $_.MacAddress; Media = [string]$_.PhysicalMediaType; Pnp = [string]$_.PnPDeviceID; Speed = $_.Speed; Modes = ((Get-NetAdapterAdvancedProperty -Name $_.Name -RegistryKeyword '*SpeedDuplex' -ErrorAction SilentlyContinue).ValidDisplayValues -join ';') } })
  os = [pscustomobject]@{ Caption = $os.Caption; EditionID = $cv.EditionID; DisplayVersion = $cv.DisplayVersion; ReleaseId = $cv.ReleaseId; Build = $os.BuildNumber }
  bitlocker = $bitlocker
  wlan = $wlan
  lmp = $lmp
  battery = [pscustomobject]@{ Full = $full; Design = $design; Cycles = $cycles; Present = $hasBattery; Admin = $admin }
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

/// "dedicated" (its own card and memory), "integrated" (part of the processor) or "" from a graphics name.
/// Intel's "Arc" is both: the A- and B-series cards (Arc A770, Arc B580, Arc Pro A60) are separate; "Arc Graphics" and
/// "Arc 140V" are built into Core Ultra processors.
pub fn gpu_kind(lower: &str) -> &'static str {
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let arc_card = words.windows(2).any(|w| w[0] == "arc" && (w[1].starts_with('a') || w[1].starts_with('b')) && w[1][1..].chars().all(|c| c.is_ascii_digit()) && w[1].len() >= 3)
        || lower.contains("arc pro a") || lower.contains("arc pro b");
    if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("quadro") || lower.contains("radeon rx") || lower.contains("radeon pro") || arc_card {
        "dedicated"
    } else if lower.contains("intel") || lower.contains("adreno") || lower.contains("qualcomm") || lower.contains("radeon") {
        "integrated" // Intel UHD/Iris/Arc Graphics, Snapdragon, and Radeon without RX/Pro (780M, Radeon Graphics, Vega)
    } else {
        ""
    }
}

/// Windows' edition code (the same in every language) -> the edition's name.
pub fn edition_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "Core" => "Home", "CoreN" => "Home N", "CoreSingleLanguage" => "Home Single Language", "CoreCountrySpecific" => "Home China",
        "Professional" => "Pro", "ProfessionalN" => "Pro N", "ProfessionalEducation" => "Pro Education",
        "ProfessionalWorkstation" => "Pro for Workstations", "Enterprise" => "Enterprise", "EnterpriseN" => "Enterprise N",
        "EnterpriseS" => "Enterprise LTSC", "EnterpriseG" => "Enterprise G", "IoTEnterprise" => "IoT Enterprise",
        "IoTEnterpriseS" => "IoT Enterprise LTSC", "Education" => "Education", "EducationN" => "Education N",
        "ServerStandard" => "Server Standard", "ServerDatacenter" => "Server Datacenter",
        _ => return None,
    })
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
        // LPDDR is soldered (some firmware still calls it SO-DIMM)
        replaceable: if kind.starts_with("LP") { "no" } else if socketed { "yes" } else { "" }.into(),
        modules: (socketed && !kind.starts_with("LP")).then_some(modules.len() as u32),
        module_gb: if socketed && !kind.starts_with("LP") { first.and_then(|m| num(&m["Capacity"])).map(|c| (c / 1024.0 / 1024.0 / 1024.0).round()) } else { None },
        form_factor: if kind.starts_with("LP") { "" } else { match form { 12 => "SO-DIMM", 8 => "DIMM", _ => "" } }.into(),
    });

    for gpu in list(&j["gpus"]) {
        let name = util::clean(&text(&gpu["Name"]));
        let lower = name.to_lowercase();
        if name.is_empty() || ["basic display", "basic render", "remote", "virtual", "parsec", "mirage", "citrix", "displaylink", "idd "].iter().any(|s| lower.contains(s)) {
            continue;
        }
        let kind = gpu_kind(&lower);
        // the driver's registry size; else AdapterRAM, which cannot say more than 4 GB (so 4 GB there means "4 or more")
        let vram = num(&gpu["Vram"]).filter(|b| *b > 0.0)
            .or_else(|| num(&gpu["AdapterRAM"]).filter(|b| *b > 256.0 * 1024.0 * 1024.0 && *b < 4.0 * 1024.0 * 1024.0 * 1024.0 - 2.0 * 1024.0 * 1024.0))
            .map(|b| (b / 1024.0 / 1024.0 / 1024.0).round());
        // "Intel Arc 140V GPU (16GB)": the size in brackets is the computer's memory class, not the card's
        let name = name.split(" (").next().unwrap_or(&name).trim().to_string();
        f.gpus.push(Gpu { name, kind: kind.into(), vram_gb: if kind == "dedicated" { vram } else { None } });
    }

    for disk in list(&j["disks"]) {
        let bus = text(&disk["BusType"]);
        // removable or virtual drives (by name, or by number if Windows wrote the number): USB, SD, iSCSI, virtual
        if ["USB", "SD", "iSCSI", "Virtual", "FileBackedVirtual", "7", "9", "12", "14", "15"].contains(&bus.as_str()) {
            continue;
        }
        let Some(bytes) = num(&disk["Size"]) else { continue };
        let media = text(&disk["MediaType"]);
        let kind = match media.as_str() { "SSD" | "4" => "SSD", "HDD" | "3" => "HDD", "SCM" => "SSD", _ if bus == "NVMe" || bus == "17" => "SSD", _ if bus == "MMC" || bus == "13" => "eMMC", _ => "" };
        let model_lower = text(&disk["Model"]).to_lowercase();
        let connector = match bus.as_str() {
            "NVMe" | "17" => "NVMe", "SATA" | "11" => "SATA", "SAS" | "10" => "SAS",
            // behind Intel's storage controller (RST/VMD) a single drive shows as RAID; its name usually says NVMe
            "RAID" | "8" if model_lower.contains("nvme") || ["mzv", "pm9", "kxg", "kbg", "pc sn", "sn5", "sn7", "bc7", "bc9", "pc801", "pc711", "pc611"].iter().any(|p| model_lower.contains(p)) => "NVMe",
            _ => "",
        };
        // no media type: a drive that spins is a hard drive, one that does not (0 rpm) an SSD
        let kind = match (kind, num(&disk["Spindle"])) { ("", Some(rpm)) if rpm == 0.0 => "SSD", ("", Some(rpm)) if rpm > 0.0 && rpm < 1e9 => "HDD", (k, _) => k };
        let replaceable = util::pc_disk_replaceable(kind, connector, if bus == "19" { "UFS" } else { &bus }, &text(&disk["Model"]));
        let form_factor = util::pc_disk_form(kind, connector, replaceable, &f.chassis).into();
        // wear: percent of the SSD's rated life used (Windows leaves it 0 or empty when the drive does not say)
        let wear_pct = num(&disk["Wear"]).filter(|w| *w > 0.0 && *w <= 255.0);
        let hours = num(&disk["Hours"]).filter(|h| *h > 0.0 && *h < 1_000_000.0).map(|h| h as u32);
        f.disks.push(Disk { gb: util::disk_gb(bytes), kind: kind.into(), connector: connector.into(), replaceable: replaceable.into(), form_factor,
                            health: util::disk_health(&text(&disk["Health"])).into(), wear_pct, hours });
    }

    for nic in list(&j["nics"]) {
        let media = text(&nic["Media"]).to_lowercase();
        let kind = if media.contains("802.11") || media.contains("wireless") { "wifi" } else if media.contains("802.3") { "eth" } else { continue };
        if text(&nic["Pnp"]).to_uppercase().starts_with("USB\\") {
            continue; // USB adapters and docks move between computers: not this computer's card
        }
        let model = util::clean(&text(&nic["Description"]));
        let mut card = Nic { kind: kind.into(), mac: text(&nic["MacAddress"]), ..Nic::default() };
        if kind == "wifi" {
            let wlan = text(&j["wlan"]);
            let by_name = util::wifi_gen_from_name(&model);
            let six_ghz = by_name == "6E" || wlan.contains("6 GHz") || wlan.contains("6GHz");
            let by_standards = util::wifi_gen_from_standards(&wlan, six_ghz);
            card.wifi_gen = if by_standards.is_empty() { by_name } else { by_standards };
            card.bt_ver = num(&j["lmp"]).map(|n| util::bt_from_lmp(n as i64)).unwrap_or_default();
        }
        if kind == "eth" {
            // the fastest of the speeds the driver offers ("1.0 Gbps Full Duplex"...), else the current link speed
            card.speed_mbps = util::max_ethernet_mbps(&text(&nic["Modes"]))
                .or_else(|| num(&nic["Speed"]).filter(|b| *b > 0.0).map(|b| (b / 1_000_000.0).round() as u32))
                .or_else(|| util::ethernet_mbps_from_name(&model)); // an unplugged adapter whose driver lists nothing
        }
        card.model = model;
        f.nics.push(card);
    }

    // "Microsoft Windows 11 Pro" + "23H2" -> version "11 23H2", edition "Pro"
    let caption = text(&j["os"]["Caption"]);
    let after = caption.split("Windows").nth(1).unwrap_or("").trim().to_string();
    let mut words = after.split_whitespace();
    let mut major = words.next().unwrap_or("").to_string();
    // the build settles 10 or 11 (Windows 11 starts at build 22000)
    if let Some(build) = num(&j["os"]["Build"]) {
        if major == "10" || major == "11" {
            major = if build >= 22000.0 { "11" } else { "10" }.into();
        }
    }
    // the edition from its fixed code (the description is in the system's language), else from the description
    let edition = edition_name(&text(&j["os"]["EditionID"])).map(String::from).unwrap_or_else(|| words.collect::<Vec<_>>().join(" "));
    let release = [text(&j["os"]["DisplayVersion"]), text(&j["os"]["ReleaseId"])].into_iter().find(|r| !r.is_empty()).unwrap_or_default();
    if !major.is_empty() {
        f.os = Some(Os { family: "win".into(), version: format!("{major} {release}").trim().to_string(), edition, ..Os::default() });
    }

    // BitLocker on the system drive: 1 on, 3 encrypting, 5 suspended (still encrypted), 6 locked / 2 off, 4 decrypting
    f.encryption = match num(&j["bitlocker"]).map(|v| v as i64) { Some(1) | Some(3) | Some(5) | Some(6) => "yes", Some(2) | Some(4) | Some(0) => "no", _ => "" }.into();
    f.domain_joined = match &cs["PartOfDomain"] { Value::Bool(true) => "yes", Value::Bool(false) => "no", _ => "" }.into();

    if let (Some(full), Some(design)) = (num(&j["battery"]["Full"]), num(&j["battery"]["Design"])) {
        if design > 0.0 && full > 0.0 {
            f.battery = Some(Battery { health_pct: (full / design * 100.0).round().min(100.0) });
        }
    } else if f.chassis == "laptop" || j["battery"]["Present"] == Value::Bool(true) {
        // only worth suggesting administrator rights when the app did not have them
        let admin = j["battery"]["Admin"] == Value::Bool(true);
        f.warnings.push(if admin { "battery_unknown" } else { "battery_needs_admin" }.into());
    }
    f.battery_cycles = num(&j["battery"]["Cycles"]).filter(|c| *c > 0.0 && *c < 100_000.0).map(|c| c as u32);
    if let (Some(w), Some(h)) = (num(&j["screen"]["Width"]), num(&j["screen"]["Height"])) {
        f.screen_inches = util::inches_from_cm(w, h);
    }
    if f.serial.is_empty() || util::is_placeholder(&f.serial) {
        f.serial.clear();
    }
    f
}
