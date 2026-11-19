//! Updates: on launch the window asks GitHub for the latest published release (latest.json, made by the release
//! build). A newer version is offered, never installed unasked. Every file is checked against the signing key built
//! into the app before it is used, so only releases made by this repository's build can install.
//!
//! How a copy updates depends on how it was installed: the Mac app, the Windows installers and the Linux packages
//! through Tauri's updater; the portable Windows .exe by replacing its own file (see inventario_core::selfupdate).
//! Anything else (a binary built by hand) only gets a link to the release page.

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const RELEASE_PAGE: &str = "https://github.com/ZOOPA-Smart-Agency/INVENTARIODATA/releases/latest";

/// The update found at launch, kept until it is installed.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bundle,   // installed from a .dmg, setup.exe, .msi, .deb, .rpm or AppImage: the build marks the program inside
    Portable, // the Windows .exe that runs without installing
    Manual,
}

fn kind() -> Kind {
    match tauri::utils::platform::bundle_type() {
        Some(_) => Kind::Bundle,
        None if cfg!(windows) => Kind::Portable,
        None => Kind::Manual,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    version: String,
    notes: String,
    /// false: the app cannot install it itself; the window links to the release page instead
    can_install: bool,
    page: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    done: u64,
    total: Option<u64>,
}

/// A newer published version, if there is one. Offline, or GitHub not answering, means no update: never an error.
#[tauri::command]
pub async fn check_update<R: Runtime>(app: AppHandle<R>, pending: State<'_, Pending>) -> Result<Option<Available>, String> {
    let kind = kind();
    let mut builder = app.updater_builder().timeout(Duration::from_secs(15));
    if kind == Kind::Portable {
        // its own entry in latest.json: the plain .exe, not an installer
        builder = builder.target(format!("windows-{}-portable", std::env::consts::ARCH));
    }
    let found = match builder.build() {
        Ok(updater) => updater.check().await.ok().flatten(),
        Err(_) => None,
    };
    let Some(update) = found else { return Ok(None) };
    let available = Available {
        version: update.version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        can_install: kind != Kind::Manual,
        page: RELEASE_PAGE,
    };
    *pending.0.lock().unwrap() = Some(update);
    Ok(Some(available))
}

/// Downloads the update (reporting progress), checks its signature and installs it, then starts the new version.
/// On Windows the installer closes this window itself.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>, pending: State<'_, Pending>, progress: Channel<Progress>) -> Result<(), String> {
    let update = pending.0.lock().unwrap().clone().ok_or("no update")?;
    let mut done = 0u64;
    // download() checks the signature before returning anything
    let bytes = update.download(|chunk, total| {
        done += chunk as u64;
        let _ = progress.send(Progress { done, total });
    }, || {}).await.map_err(|e| e.to_string())?;
    match kind() {
        Kind::Portable => install_portable(&app, &bytes),
        Kind::Bundle => {
            update.install(&bytes).map_err(|e| e.to_string())?; // on Windows this does not return
            app.restart()
        }
        Kind::Manual => Err("this copy cannot update itself".into()),
    }
}

fn install_portable<R: Runtime>(app: &AppHandle<R>, bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"MZ") {
        return Err("the update is not a Windows program".into()); // signed, but not what this copy runs
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    inventario_core::selfupdate::replace_exe(&exe, bytes).map_err(|e| e.to_string())?;
    std::process::Command::new(&exe).spawn().map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

/// At start: deletes what an earlier update of the portable app replaced, in the background.
pub fn clean_up() {
    if kind() == Kind::Portable {
        if let Ok(exe) = std::env::current_exe() {
            std::thread::spawn(move || inventario_core::selfupdate::remove_old(&exe));
        }
    }
}

#[cfg(test)]
mod tests {
    //! The update check against a local stand-in for GitHub, with a key made for these tests only (its private half
    //! was deleted after signing tests-fixtures/payload.exe).
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use tauri::Manager;

    const PAYLOAD: &[u8] = include_bytes!("../tests-fixtures/payload.exe");
    const SIG: &str = include_str!("../tests-fixtures/payload.exe.sig");
    const PUBKEY: &str = include_str!("../tests-fixtures/test-only.key.pub");

    /// Serves /latest.json and /payload.exe; `tamper` changes the file after signing.
    fn server(version: &str, tamper: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let entry = serde_json::json!({"signature": SIG.trim(), "url": format!("{base}/payload.exe")});
        let platforms: serde_json::Map<String, serde_json::Value> = ["darwin-aarch64", "darwin-x86_64", "linux-x86_64", "linux-aarch64", "windows-x86_64", "windows-aarch64",
            "windows-x86_64-portable", "windows-aarch64-portable"].iter().map(|t| (t.to_string(), entry.clone())).collect();
        let manifest = serde_json::json!({"version": version, "notes": "What changed", "platforms": platforms}).to_string();
        let mut file = PAYLOAD.to_vec();
        if tamper { file.push(b'!'); }
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut request = [0u8; 2048];
                let n = stream.read(&mut request).unwrap_or(0);
                let path = String::from_utf8_lossy(&request[..n]).split_whitespace().nth(1).unwrap_or("").to_string();
                let (kind, body) = if path == "/latest.json" { ("application/json", manifest.as_bytes().to_vec()) } else { ("application/octet-stream", file.clone()) };
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(&body);
            }
        });
        base
    }

    fn app(base: &str) -> tauri::App<tauri::test::MockRuntime> {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.package_info_mut().version = "0.13.0".parse().unwrap();
        context.config_mut().plugins.0.insert("updater".into(), serde_json::json!({
            "pubkey": PUBKEY.trim(), "endpoints": [format!("{base}/latest.json")], "dangerousInsecureTransportProtocol": true,
        }));
        tauri::test::mock_builder().plugin(tauri_plugin_updater::Builder::new().build()).manage(Pending::default())
            .build(context).unwrap()
    }

    #[test]
    fn offers_a_newer_version_and_checks_its_signature() {
        let app = app(&server("0.14.0", false));
        let found = tauri::async_runtime::block_on(check_update(app.handle().clone(), app.state::<Pending>())).unwrap().expect("an update");
        assert_eq!((found.version.as_str(), found.notes.as_str()), ("0.14.0", "What changed"));
        assert_eq!(found.can_install, kind() != Kind::Manual);
        let update = app.state::<Pending>().0.lock().unwrap().clone().unwrap();
        let bytes = tauri::async_runtime::block_on(update.download(|_, _| {}, || {})).unwrap();
        assert_eq!(bytes, PAYLOAD);
    }

    #[test]
    fn refuses_a_changed_file() {
        let app = app(&server("0.14.0", true));
        tauri::async_runtime::block_on(check_update(app.handle().clone(), app.state::<Pending>())).unwrap().expect("an update");
        let update = app.state::<Pending>().0.lock().unwrap().clone().unwrap();
        assert!(tauri::async_runtime::block_on(update.download(|_, _| {}, || {})).is_err(), "a file that does not match its signature");
    }

    #[test]
    fn nothing_when_up_to_date_or_offline() {
        let app = app(&server("0.13.0", false));
        assert!(tauri::async_runtime::block_on(check_update(app.handle().clone(), app.state::<Pending>())).unwrap().is_none());
        let app = self::app("http://127.0.0.1:9"); // nothing listens there
        assert!(tauri::async_runtime::block_on(check_update(app.handle().clone(), app.state::<Pending>())).unwrap().is_none());
    }
}
