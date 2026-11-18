//! Small helpers shared by the readers of every system.

use serde_json::Value;

/// Runs a program and returns what it printed, or None if it is missing or failed.
/// On Windows it never opens a console window.
pub fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut command = std::process::Command::new(program);
    command.args(args).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command.output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Reads a text file, trimmed; None if it is missing or unreadable (for example, needs root).
pub fn read(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Collapses spaces and removes trademark marks.
pub fn clean(text: &str) -> String {
    let text = text.replace("(R)", "").replace("(r)", "").replace("(TM)", "").replace("(tm)", "").replace('\u{0}', "");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Values firmware uses when the real one was never filled in.
pub fn is_placeholder(value: &str) -> bool {
    let v = value.trim().to_lowercase();
    v.is_empty()
        || v.chars().all(|c| c == '0' || c == ' ' || c == '-')
        || [
            "to be filled by o.e.m.", "system serial number", "default string", "not specified", "none", "n/a", "na",
            "unknown", "chassis serial number", "o.e.m.", "oem", "123456789", "system product name", "system manufacturer",
            "invalid", "not applicable",
        ]
        .contains(&v.as_str())
}

/// "a4-c3-f0-12-34-56" or "a4c3f0123456" -> "A4:C3:F0:12:34:56"; "" if it is not a MAC address.
pub fn normalize_mac(mac: &str) -> String {
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_uppercase();
    if hex.len() != 12 {
        return String::new();
    }
    hex.as_bytes().chunks(2).map(|p| String::from_utf8_lossy(p).into_owned()).collect::<Vec<_>>().join(":")
}

/// Shorter, consistent manufacturer names ("LENOVO" -> "Lenovo", "Dell Inc." -> "Dell").
pub fn manufacturer(name: &str) -> String {
    let n = clean(name);
    let lower = n.to_lowercase();
    let known = [
        ("lenovo", "Lenovo"), ("dell", "Dell"), ("hewlett-packard", "HP"), ("hp", "HP"), ("apple", "Apple"), ("asustek", "ASUS"),
        ("asus", "ASUS"), ("acer", "Acer"), ("microsoft", "Microsoft"), ("samsung", "Samsung"), ("toshiba", "Toshiba"),
        ("dynabook", "Dynabook"), ("fujitsu", "Fujitsu"), ("msi", "MSI"), ("micro-star", "MSI"), ("gigabyte", "Gigabyte"),
        ("intel", "Intel"), ("huawei", "Huawei"), ("lg electronics", "LG"), ("framework", "Framework"), ("raspberry pi", "Raspberry Pi"),
        ("vmware", "VMware"), ("qemu", "QEMU"), ("innotek", "VirtualBox"), ("parallels", "Parallels"),
    ];
    for (key, value) in known {
        if lower == key || lower.starts_with(&format!("{key} ")) || lower.starts_with(&format!("{key},")) || lower.starts_with(&format!("{key}.")) {
            return value.to_string();
        }
    }
    n
}

/// "Intel(R) Core(TM) i7-1365U CPU @ 2.60GHz", "13th Gen Intel Core i5-1345U", "AMD Ryzen 7 PRO 7840U w/ Radeon 780M Graphics"
/// -> "Intel Core i7-1365U", "Intel Core i5-1345U", "AMD Ryzen 7 PRO 7840U"
pub fn cpu_name(name: &str) -> String {
    let mut n = clean(name);
    if let Some(i) = n.find(" @ ") {
        n.truncate(i);
    }
    for cut in [" w/ ", " with Radeon", " with Radeon"] {
        if let Some(i) = n.find(cut) {
            n.truncate(i);
        }
    }
    let mut n = n.replace(" CPU", "").replace(" Processor", "").replace(" processor", "");
    // "AMD Ryzen 7 5800X 8-Core" -> "AMD Ryzen 7 5800X"
    if let Some((before, last)) = n.rsplit_once(' ') {
        let lower = last.to_lowercase();
        if lower.ends_with("-core") && lower[..lower.len() - 5].chars().all(|c| c.is_ascii_alphanumeric()) {
            n = before.to_string();
        }
    }
    let n = n.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches("th Gen ").trim_start_matches("st Gen ")
        .trim_start_matches("nd Gen ").trim_start_matches("rd Gen ").to_string();
    clean(&n)
}

/// Memory as sold, from what the system says is usable (a little less: firmware and graphics keep some):
/// 15.6 GiB -> 16, 7.7 -> 8, 2.9 -> 3, 11.8 -> 12; the smallest size sold that holds it.
pub fn ram_gb_from_bytes(bytes: f64) -> f64 {
    let gib = bytes / 1024.0 / 1024.0 / 1024.0;
    if gib <= 0.0 {
        return 0.0;
    }
    const SOLD: [f64; 22] = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 18.0, 20.0, 24.0, 32.0, 36.0, 40.0, 48.0, 64.0, 96.0, 128.0, 192.0, 256.0, 384.0, 512.0];
    SOLD.iter().copied().find(|s| *s >= gib - 0.05).unwrap_or(gib.round())
}

/// Drive size as sold, in GB (1 GB = 1,000,000,000 bytes, as makers count): the exact size moves to the nearest size
/// drives are sold in when it is close (a "1 TB" drive of 1,024 GB -> 1000; 256.06 -> 256). Apple's SSDs keep less
/// for themselves than others, so theirs are matched against Apple's sizes (500.3 GB -> the "512 GB" model).
pub fn disk_gb(bytes: f64) -> f64 {
    disk_gb_sold(bytes, false)
}

pub fn disk_gb_sold(bytes: f64, apple_ssd: bool) -> f64 {
    let gb = bytes / 1e9;
    const SOLD: [f64; 40] = [8.0, 16.0, 32.0, 60.0, 64.0, 80.0, 120.0, 128.0, 160.0, 180.0, 240.0, 250.0, 256.0, 320.0, 400.0, 480.0, 500.0, 512.0,
                             640.0, 750.0, 960.0, 1000.0, 1200.0, 1500.0, 1920.0, 2000.0, 3000.0, 3840.0, 4000.0, 5000.0, 6000.0, 8000.0, 10000.0,
                             12000.0, 14000.0, 16000.0, 18000.0, 20000.0, 22000.0, 24000.0];
    const APPLE: [f64; 8] = [64.0, 128.0, 256.0, 512.0, 1000.0, 2000.0, 4000.0, 8000.0];
    let (sizes, margin): (&[f64], f64) = if apple_ssd { (&APPLE, 0.06) } else { (&SOLD, 0.03) };
    let nearest = sizes.iter().copied().min_by(|a, b| (a - gb).abs().total_cmp(&(b - gb).abs())).unwrap_or(gb);
    if (nearest - gb).abs() <= nearest * margin { nearest } else { gb.round() }
}

/// The first number in a text like "18 GB", "2,6 GHz" or "1536 MB", with the unit after it.
pub fn number_and_unit(text: &str) -> Option<(f64, String)> {
    let t = text.trim().replace(',', ".");
    let start = t.find(|c: char| c.is_ascii_digit())?;
    let rest = &t[start..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
    let value: f64 = rest[..end].parse().ok()?;
    Some((value, rest[end..].trim().to_string()))
}

/// SMBIOS chassis type codes -> "laptop", "all_in_one", "desktop" or "".
pub fn chassis_from_smbios(code: i64) -> &'static str {
    match code {
        8 | 9 | 10 | 11 | 12 | 14 | 18 | 21 | 30 | 31 | 32 => "laptop",
        13 => "all_in_one",
        3 | 4 | 5 | 6 | 7 | 15 | 16 | 24 | 35 | 36 => "desktop",
        _ => "",
    }
}

/// A JSON value that may be one object or a list of them, as a list (PowerShell writes single items as objects).
pub fn list(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(items) => items.iter().collect(),
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

/// A JSON field as text, whether it was written as text or as a number.
pub fn text(value: &Value) -> String {
    match value {
        Value::String(s) => clean(s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// A JSON field as a number, whether it was written as a number or as text.
pub fn num(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Screen diagonal in inches from its width and height in whole centimetres (as EDID and Windows report them).
/// Whole centimetres are not precise enough (a 14" panel reports 31 x 17 cm, 13.9"), so the result is moved to the
/// nearest size screens are sold in when one is close.
pub fn inches_from_cm(width_cm: f64, height_cm: f64) -> Option<f64> {
    if width_cm < 10.0 || height_cm < 5.0 || width_cm > 300.0 {
        return None; // 0 or nonsense: not a real panel size
    }
    let inches = (width_cm * width_cm + height_cm * height_cm).sqrt() / 2.54;
    const SOLD: [f64; 26] = [10.1, 11.6, 12.0, 12.3, 12.5, 13.0, 13.3, 13.4, 13.5, 13.6, 14.0, 14.2, 14.5, 15.0, 15.3, 15.4, 15.6,
                             16.0, 16.2, 17.0, 17.3, 18.0, 21.5, 23.8, 24.0, 27.0];
    let nearest = SOLD.iter().copied().min_by(|a, b| (a - inches).abs().total_cmp(&(b - inches).abs())).unwrap();
    Some(if (nearest - inches).abs() <= 0.25 { nearest } else { (inches * 10.0).round() / 10.0 })
}

/// Wi-Fi generation from the standards a card supports, however the system writes them ("802.11 a/b/g/n/ac/ax",
/// "802.11ax 802.11ac"): "7", "6E" (Wi-Fi 6 with the 6 GHz band), "6", "5", "4" or "".
pub fn wifi_gen_from_standards(standards: &str, six_ghz: bool) -> String {
    // only what follows "802.11": "ax" (Windows, one per standard) or "a/b/g/n/ac/ax" (macOS)
    let lower = standards.to_lowercase();
    let tokens: Vec<String> = lower.split("802.11").skip(1)
        .flat_map(|part| part.trim_start().chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '/').collect::<String>()
            .split('/').map(String::from).collect::<Vec<_>>())
        .collect();
    let has = |t: &str| tokens.iter().any(|x| x == t);
    if has("be") { "7" } else if has("ax") && six_ghz { "6E" } else if has("ax") { "6" } else if has("ac") { "5" } else if has("n") { "4" } else { "" }.into()
}

/// Wi-Fi generation from a card's name ("Intel(R) Wi-Fi 6E AX211 160MHz", "Killer Wi-Fi 7 BE1750"), or "".
pub fn wifi_gen_from_name(name: &str) -> String {
    let n = format!(" {} ", name.to_lowercase().replace(['(', ')', '-'], " "));
    if n.contains("wi fi 7") || n.contains(" be2") || n.contains(" be1") { "7" }
    else if n.contains("wi fi 6e") || n.contains(" 6e ") || n.contains(" ax21") || n.contains(" ax411") { "6E" }
    else if n.contains("wi fi 6") || n.contains(" ax2") || n.contains(" ax1") || n.contains("802.11ax") { "6" }
    else if n.contains("wi fi 5") || n.contains(" ac ") || n.contains("802.11ac") { "5" }
    else { "" }.into()
}

/// Bluetooth version from the controller's LMP version number (Bluetooth Core Specification "Assigned Numbers").
pub fn bt_from_lmp(lmp: i64) -> String {
    match lmp { 6 => "4.0", 7 => "4.1", 8 => "4.2", 9 => "5.0", 10 => "5.1", 11 => "5.2", 12 => "5.3", 13 => "5.4", 14 => "6.0", _ => "" }.into()
}

/// The hostname, without the domain.
pub fn hostname() -> String {
    run("hostname", &[]).map(|h| h.trim().split('.').next().unwrap_or("").to_string()).unwrap_or_default()
}

/// Whether a PC's (Windows or Linux) internal drive can be swapped: soldered eMMC and UFS storage cannot; SATA and
/// SAS drives, and NVMe drives in an M.2 slot (nearly all of them), can. NVMe drives soldered to the board usually
/// say "BGA" in their model name.
pub fn pc_disk_replaceable(kind: &str, connector: &str, bus: &str, model: &str) -> &'static str {
    let model = model.to_uppercase();
    if kind == "eMMC" || bus.eq_ignore_ascii_case("ufs") || model.contains("UFS") || model.contains("EMMC") {
        "no"
    } else if connector == "NVMe" && model.contains("BGA") {
        "no"
    } else if ["SATA", "SAS", "NVMe"].contains(&connector) {
        "yes"
    } else {
        ""
    }
}

/// Whether a Mac's internal drive can be swapped, from its model identifier ("MacBookPro14,1"). Apple silicon and
/// T2 Macs (2018 on) have their storage soldered or paired to the chip; so do the 12-inch MacBook and the 2016-2017
/// MacBook Pro with Touch Bar. Older Macs have a SATA drive, or an SSD on a removable (Apple-specific) card.
pub fn mac_disk_replaceable(identifier: &str, apple_silicon: bool) -> &'static str {
    if apple_silicon {
        return "no";
    }
    let family: String = identifier.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let mut numbers = identifier[family.len()..].split(',').map(|n| n.trim().parse::<u32>().ok());
    let (Some(major), minor) = (numbers.next().flatten(), numbers.next().flatten()) else { return "" };
    let soldered = match family.as_str() {
        "MacBookPro" => major >= 15 || ((major == 13 || major == 14) && minor != Some(1)),
        "MacBookAir" => major >= 8,
        "MacBook" => major >= 8,
        "iMac" => major >= 20,
        "iMacPro" => true,
        "Macmini" => major >= 8,
        "MacPro" => major >= 7,
        _ => return "",
    };
    if soldered { "no" } else { "yes" }
}

/// The SATA generation from a link speed as each system writes it ("6 Gigabit", "6.0 Gb/s", "3.0 Gbps").
pub fn sata_connector(speed: &str) -> &'static str {
    let number: String = speed.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    match number.parse::<f64>().ok() {
        Some(v) if v >= 5.0 => "SATA III",
        Some(v) if v >= 2.5 => "SATA II",
        Some(v) if v > 0.0 => "SATA I",
        _ => "SATA",
    }
}

/// The PCI Express generation from a link speed in GT/s: "5.0 GT/s" -> "2.0", "8.0 GT/s PCIe" -> "3.0"; "" if unknown.
pub fn pcie_generation(link_speed: &str) -> &'static str {
    let number: String = link_speed.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    match number.parse::<f64>().ok() {
        Some(v) if v >= 60.0 => "6.0", Some(v) if v >= 30.0 => "5.0", Some(v) if v >= 15.0 => "4.0",
        Some(v) if v >= 7.0 => "3.0", Some(v) if v >= 4.0 => "2.0", Some(v) if v >= 2.0 => "1.0",
        _ => "",
    }
}

/// The PCI Express generation of an NVMe drive from its link speed in GT/s ("8.0 GT/s PCIe" -> NVMe PCIe 3.0).
pub fn nvme_connector(link_speed: &str) -> String {
    match pcie_generation(link_speed) {
        "" => "NVMe".into(),
        generation => format!("NVMe PCIe {generation}"),
    }
}

/// The fastest Ethernet speed (Mbit/s) in a list of link modes, whatever the system's wording:
/// "1000baseT/Full 2500baseT/Full" (Linux), "10baseT/UTP 100baseTX 1000baseT 10Gbase-T" (macOS),
/// "100 Mbps Full Duplex;1.0 Gbps Full Duplex;2.5 Gbps Full Duplex" (Windows).
pub fn max_ethernet_mbps(modes: &str) -> Option<u32> {
    let text = modes.to_lowercase();
    let words: Vec<&str> = text.split(|c: char| c.is_whitespace() || matches!(c, '/' | ',' | ';' | '(' | ')' | '<' | '>')).filter(|w| !w.is_empty()).collect();
    let lead = |w: &str| -> Option<f64> { w.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>().parse().ok() };
    let mut best: Option<f64> = None;
    for (i, word) in words.iter().enumerate() {
        let speed = if let Some(n) = word.find("gbase") { lead(&word[..n]).map(|v| v * 1000.0) }
            else if let Some(n) = word.find("base") { lead(&word[..n]) }
            else if *word == "gbps" || *word == "gbit" { words.get(i.wrapping_sub(1)).and_then(|w| lead(w)).map(|v| v * 1000.0) }
            else if *word == "mbps" || *word == "mbit" { words.get(i.wrapping_sub(1)).and_then(|w| lead(w)) }
            else { None };
        if let Some(v) = speed.filter(|v| *v >= 10.0) {
            best = Some(best.map_or(v, |b: f64| b.max(v)));
        }
    }
    best.map(|v| v.round() as u32)
}

/// A PC's (Windows or Linux) drive form factor, where the kind and connection settle it: NVMe drives that can be
/// swapped sit in an M.2 slot; hard drives are 2.5" in laptops and 3.5" in desktops. A SATA SSD can be 2.5", M.2 or
/// mSATA, which the system does not say, so it is left out.
pub fn pc_disk_form(kind: &str, connector: &str, replaceable: &str, chassis: &str) -> &'static str {
    if replaceable == "no" || kind == "eMMC" {
        ""
    } else if connector.starts_with("NVMe") {
        "M.2"
    } else if kind == "HDD" && chassis == "laptop" {
        "2.5\""
    } else if kind == "HDD" && chassis == "desktop" {
        "3.5\""
    } else {
        ""
    }
}

/// A Mac's drive form factor, from its model: hard drives are 3.5" in the Mac Pro, in iMacs up to 2011 and in the
/// 27-inch iMac, and 2.5" elsewhere; SATA SSDs are 2.5" (except the MacBook Air's own card); NVMe and PCIe SSDs that
/// can be swapped are on Apple's own card. Soldered storage (Apple silicon, T2) has none.
pub fn mac_disk_form(identifier: &str, kind: &str, connector: &str, replaceable: &str, screen_inches: Option<f64>) -> &'static str {
    if replaceable != "yes" {
        return "";
    }
    let family: String = identifier.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let major: u32 = identifier[family.len()..].split(',').next().and_then(|n| n.trim().parse().ok()).unwrap_or(0);
    if connector.starts_with("NVMe") || connector.starts_with("PCIe") {
        return "Apple (propietario)"; // Apple's own SSD card
    }
    match (kind, family.as_str()) {
        ("HDD", "MacPro") => "3.5\"",
        ("HDD", "iMac") if major <= 12 || screen_inches.is_some_and(|s| s >= 26.0) => "3.5\"",
        ("HDD", _) => "2.5\"",
        ("SSD", "MacBookAir") => "Apple (propietario)",
        ("SSD", _) => "2.5\"",
        _ => "",
    }
}

/// An Ethernet port's speed from its name, when nothing else says it: "Realtek USB GbE Family Controller" and
/// "Intel Ethernet Connection I219-V Gigabit" -> 1000, "Realtek 2.5GbE" and "Intel Ethernet Controller I225-V 2.5G"
/// -> 2500, "10GbE" -> 10000, "Fast Ethernet" and "10/100" -> 100.
pub fn ethernet_mbps_from_name(name: &str) -> Option<u32> {
    let n = name.to_lowercase().replace(['-', '_'], " ");
    // the faster names first, and 2.5 before 5: "2.5GbE" also contains "5GbE"
    for (marks, mbps) in [(&["10gbe", "10 gbe", "10gbase", "10 gigabit", "10g "][..], 10000), (&["2.5gbe", "2.5 gbe", "2.5g", "2.5 gigabit"][..], 2500),
                          (&["5gbe", "5 gbe", "5g "][..], 5000), (&["gbe", "gigabit", "1000base", "10/100/1000"][..], 1000),
                          (&["fast ethernet", "10/100"][..], 100)] {
        if marks.iter().any(|m| format!("{n} ").contains(m)) {
            return Some(mbps);
        }
    }
    None
}

/// A drive's health as each system words it: macOS "Verified" / "Failing", Windows "Healthy" / "Warning" /
/// "Unhealthy" (or 0 / 1 / 2) -> "ok", "warning", "failing"; "" when not said ("Not Supported", "Unknown").
pub fn disk_health(status: &str) -> &'static str {
    match status.trim().to_lowercase().as_str() {
        "verified" | "healthy" | "ok" | "passed" | "0" => "ok",
        "warning" | "1" => "warning",
        "failing" | "unhealthy" | "failed" | "2" => "failing",
        _ => "",
    }
}

/// Runs a program like `run`, but gives up after a time limit (a failing drive can make health tools hang).
/// The output is read while the program runs, so a long one cannot fill the pipe and stall it.
pub fn run_timeout(program: &str, args: &[&str], seconds: u64) -> Option<String> {
    use std::io::Read;
    let mut child = std::process::Command::new(program).args(args)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
        .spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || { let mut out = Vec::new(); let _ = stdout.read_to_end(&mut out); out });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed().as_secs() >= seconds => { let _ = child.kill(); let _ = child.wait(); return None; }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(_) => { let _ = child.kill(); return None; }
        }
    }
    let out = reader.join().ok()?;
    Some(String::from_utf8_lossy(&out).into_owned()) // smartctl uses its exit code for findings, so the output counts whatever it is
}
