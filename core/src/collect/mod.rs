//! Reads this computer's details. Each operating system has two parts: running the system's own tools
//! (only compiled on that system) and interpreting what they print (always compiled, so it can be tested
//! anywhere with real sample outputs, see core/tests).

pub mod linux;
pub mod macos;
pub mod util;
pub mod windows;

use crate::facts::{Agent, Facts};

/// Everything this computer reveals about itself, ready to send.
pub fn collect() -> Facts {
    let mut facts = platform();
    facts.agent = Agent { version: crate::VERSION.into(), platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH) };
    tidy(&mut facts);
    facts
}

#[cfg(target_os = "macos")]
fn platform() -> Facts {
    macos::collect()
}

#[cfg(target_os = "windows")]
fn platform() -> Facts {
    windows::collect()
}

#[cfg(target_os = "linux")]
fn platform() -> Facts {
    linux::collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn platform() -> Facts {
    Facts { warnings: vec!["unsupported_system".into()], ..Facts::default() }
}

/// Final clean-up shared by every system: trimmed text, MAC addresses as AA:BB:CC:DD:EE:FF, no duplicates.
pub fn tidy(f: &mut Facts) {
    for text in [&mut f.serial, &mut f.hostname, &mut f.manufacturer, &mut f.model] {
        *text = util::clean(text);
    }
    if util::is_placeholder(&f.serial) {
        f.serial.clear();
    }
    if f.serial.is_empty() && !f.warnings.iter().any(|w| w.starts_with("serial")) {
        f.warnings.push("serial_missing".into());
    }
    let mut seen = Vec::new();
    f.nics.retain_mut(|n| {
        n.mac = util::normalize_mac(&n.mac);
        let keep = !n.mac.is_empty() && n.mac != "00:00:00:00:00:00" && !seen.contains(&n.mac);
        seen.push(n.mac.clone());
        keep
    });
    f.disks.retain(|d| d.gb >= 1.0);
    f.gpus.dedup_by(|a, b| a.name == b.name);
}
