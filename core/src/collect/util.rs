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

/// Whole gigabytes as shown on a box: 15.6 GiB of usable memory -> 16.
pub fn ram_gb_from_bytes(bytes: f64) -> f64 {
    let gib = bytes / 1024.0 / 1024.0 / 1024.0;
    if gib <= 0.0 {
        return 0.0;
    }
    let up = (gib / 2.0).ceil() * 2.0; // memory is sold in even sizes
    if up - gib > 1.5 { gib.round() } else { up }
}

/// Drive sizes as manufacturers count them (1 GB = 1,000,000,000 bytes), rounded.
pub fn disk_gb(bytes: f64) -> f64 {
    (bytes / 1e9).round()
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

/// The hostname, without the domain.
pub fn hostname() -> String {
    run("hostname", &[]).map(|h| h.trim().split('.').next().unwrap_or("").to_string()).unwrap_or_default()
}
