//! Replacing a program file with its new version while it runs, for the portable Windows app.
//!
//! Windows does not let a running program's file be overwritten or deleted, but it can be renamed. So the new
//! version is written next to it first, the running file is renamed out of the way (`.old`), and the new one takes
//! its name; the old file is deleted the next time the program starts. If any step fails, the original is put back.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// A rename, tried a few times: antivirus software often holds a newly written program for a moment to scan it.
fn rename(from: &Path, to: &Path) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match fs::rename(from, to) {
            Err(e) if tries < 10 && e.kind() != io::ErrorKind::NotFound => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            result => return result,
        }
    }
}

/// Where the replaced version waits to be deleted.
pub fn old_path(exe: &Path) -> PathBuf {
    with_suffix(exe, ".old")
}

/// Puts `bytes` in place of the program at `exe`. The running program keeps working; the new version is what starts
/// next. Fails, leaving everything as it was, when the folder cannot be written to.
pub fn replace_exe(exe: &Path, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty update"));
    }
    let new = with_suffix(exe, ".new");
    let old = old_path(exe);
    let _ = fs::remove_file(&new);
    fs::write(&new, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(exe).map(|m| m.permissions().mode()).unwrap_or(0o755);
        fs::set_permissions(&new, fs::Permissions::from_mode(mode))?;
    }
    // a still-running copy from an earlier update may hold the last .old; then this one waits under its own name
    let old = match fs::remove_file(&old) {
        Ok(()) => old,
        Err(e) if e.kind() == io::ErrorKind::NotFound => old,
        Err(_) => with_suffix(exe, &format!(".{}.old", std::process::id())),
    };
    if let Err(e) = rename(exe, &old) {
        let _ = fs::remove_file(&new);
        return Err(e);
    }
    if let Err(e) = rename(&new, exe) {
        let _ = rename(&old, exe); // put the original back
        let _ = fs::remove_file(&new);
        return Err(e);
    }
    Ok(())
}

/// Deletes what earlier updates replaced (`<exe>.old`, `<exe>.<n>.old`). The old copy may still be closing, so
/// this is tried a few times; anything left is tried again on the next start.
pub fn remove_old(exe: &Path) {
    let (Some(dir), Some(name)) = (exe.parent(), exe.file_name().and_then(|n| n.to_str())) else { return };
    for _ in 0..20 {
        let left: Vec<PathBuf> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path())
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&format!("{name}.")) && n.ends_with(".old")))
            .filter(|p| fs::remove_file(p).is_err())
            .collect();
        if left.is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("selfupdate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn replaces_and_cleans_up() {
        let d = dir("ok");
        let exe = d.join("app-portable.exe");
        fs::write(&exe, b"old version").unwrap();
        replace_exe(&exe, b"new version").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"new version");
        assert_eq!(fs::read(old_path(&exe)).unwrap(), b"old version");
        assert!(!with_suffix(&exe, ".new").exists());
        // a second update while the first .old is still there
        replace_exe(&exe, b"newer").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"newer");
        fs::write(d.join("other.exe.old"), b"not ours").unwrap();
        remove_old(&exe);
        assert!(!old_path(&exe).exists());
        assert!(d.join("other.exe.old").exists(), "only this program's leftovers are deleted");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn leaves_everything_when_it_fails() {
        let d = dir("fail");
        let exe = d.join("missing.exe"); // renaming it fails
        assert!(replace_exe(&exe, b"new").is_err());
        assert!(!with_suffix(&exe, ".new").exists());
        assert!(replace_exe(&d.join("x.exe"), b"").is_err());
        fs::remove_dir_all(&d).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn read_only_folder() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("ro");
        let exe = d.join("app");
        fs::write(&exe, b"old").unwrap();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o555)).unwrap();
        let result = replace_exe(&exe, b"new");
        fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"old");
        fs::remove_dir_all(&d).unwrap();
    }
}
