//! Renaming this computer, for inventories where the computer's name (hostname) is its asset tag.
//!
//! Renaming needs administrator rights, so each system asks for them in its own way: macOS shows its password
//! prompt, Windows its "allow changes" prompt (and the new name applies after a restart), Linux asks through
//! polkit, or renames directly when already running as root. The name is checked first, so nothing but letters,
//! digits and dashes ever reaches a command.

use std::process::Command;

/// Whether a name can be a computer's name on every system: 1 to 15 letters, digits or dashes (Windows' limit),
/// not starting or ending with a dash.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 15 && !name.starts_with('-') && !name.ends_with('-')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// What happened after a rename.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    /// The new name only applies after restarting (Windows).
    pub restart_needed: bool,
}

pub fn rename(name: &str) -> Result<Renamed, String> {
    if !valid_name(name) {
        return Err("bad_name".into());
    }
    rename_here(name)
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(program).args(args).output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // the person closed the password or permission prompt
        if text.contains("-128") || text.contains("canceled") || text.contains("cancelled") || output.status.code() == Some(126) {
            Err("cancelled".into())
        } else {
            Err(if text.is_empty() { format!("{program} failed") } else { text })
        }
    }
}

#[cfg(target_os = "macos")]
fn rename_here(name: &str) -> Result<Renamed, String> {
    let script = format!(
        "do shell script \"/usr/sbin/scutil --set ComputerName {name} && /usr/sbin/scutil --set LocalHostName {name} && /usr/sbin/scutil --set HostName {name}\" with administrator privileges"
    );
    run("/usr/bin/osascript", &["-e", &script])?;
    Ok(Renamed { restart_needed: false })
}

#[cfg(windows)]
fn rename_here(name: &str) -> Result<Renamed, String> {
    // an elevated PowerShell renames; its exit code says whether it worked
    let inner = format!("Rename-Computer -NewName '{name}' -Force -ErrorAction Stop");
    let outer = format!(
        "$p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-Command','{}'; exit $p.ExitCode",
        inner.replace('\'', "''")
    );
    run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &outer]).map_err(|e| {
        if e.contains("canceled by the user") || e.contains("cancelado") { "cancelled".into() } else { e }
    })?;
    Ok(Renamed { restart_needed: true })
}

#[cfg(target_os = "linux")]
fn rename_here(name: &str) -> Result<Renamed, String> {
    // SAFETY: geteuid takes no arguments, cannot fail and touches no memory
    let root = unsafe { geteuid() } == 0;
    if root {
        run("hostnamectl", &["set-hostname", name])?;
    } else {
        run("pkexec", &["hostnamectl", "set-hostname", name])?;
    }
    Ok(Renamed { restart_needed: false })
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn geteuid() -> u32;
}

/// Opens the system's own page for changing the computer's name, for someone who prefers to do it by hand.
pub fn open_name_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return run("/usr/bin/open", &["x-apple.systempreferences:com.apple.Sharing-Settings.extension"]);
    #[cfg(windows)]
    return run("cmd", &["/C", "start", "", "ms-settings:about"]);
    #[cfg(target_os = "linux")]
    {
        for (program, args) in [("gnome-control-center", &["system"][..]), ("gnome-control-center", &["info-overview"][..]), ("systemsettings", &["kcm_about-distro"][..])] {
            if Command::new(program).args(args).spawn().is_ok() {
                return Ok(());
            }
        }
        Err("no_settings".into())
    }
}

#[cfg(test)]
mod tests {
    use super::valid_name;

    #[test]
    fn names() {
        assert!(valid_name("ZLAP004"));
        assert!(valid_name("INV-0001"));
        assert!(!valid_name(""));
        assert!(!valid_name("-ZLAP"));
        assert!(!valid_name("ZLAP 004"));
        assert!(!valid_name("ZLAP004\"; rm"));
        assert!(!valid_name("ABCDEFGHIJKLMNOP"));
    }
}
