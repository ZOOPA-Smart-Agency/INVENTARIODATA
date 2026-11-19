//! "Registro de equipos": a window that reads this computer's details and adds it to the inventory.
//! Started with arguments (for example `--server ...` or `--help`) it works as the command-line tool instead.
//! It does nothing in the background: it runs when someone opens it and stops when closed. On opening it checks
//! for a newer published version and offers it (see update.rs); it never installs one unasked.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use inventario_core::facts::Facts;
use inventario_core::net::{self, Client, Problem, User};
use inventario_core::prefs::Prefs;
use inventario_core::{collect, VERSION};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

mod update;

/// The server this window is talking to, once its certificate is trusted, and later signed in.
#[derive(Default)]
struct Session(Mutex<Option<Client>>);

fn current(session: &State<Session>) -> Result<Client, Problem> {
    session.0.lock().unwrap().clone().ok_or(Problem::Server { status: 401, code: "err_auth".into() })
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(work).await.expect("background work failed")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Start {
    server: String,
    email: String,
    version: &'static str,
    platform: String,
}

#[tauri::command]
fn start() -> Start {
    let prefs = Prefs::load();
    Start { server: prefs.server_or_default(), email: prefs.email, version: VERSION, platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH) }
}

#[tauri::command]
async fn collect_facts() -> Facts {
    blocking(collect::collect).await
}

/// Checks the address answers and its certificate is trusted. Nothing private is sent.
#[tauri::command]
async fn connect(server: String, session: State<'_, Session>) -> Result<String, Problem> {
    let base = net::normalize_url(&server)?;
    let pinned = Prefs::load().pinned(&base);
    let client = Client::new(&base, pinned)?;
    let client = blocking(move || client.check().map(|_| client)).await?;
    *session.0.lock().unwrap() = Some(client);
    Ok(base)
}

/// A person confirmed the fingerprint shown for this server: ask the server again, and if it still offers the same
/// certificate (or certificate authority), remember it and connect with it.
#[tauri::command]
async fn trust(server: String, fingerprint: String, session: State<'_, Session>) -> Result<String, Problem> {
    let base = net::normalize_url(&server)?;
    let address = base.clone();
    let client = blocking(move || -> Result<Client, Problem> {
        let offer = match Client::new(&address, None)?.check() {
            Err(Problem::Untrusted { offer }) if offer.matches(&fingerprint) => offer,
            Err(Problem::Untrusted { offer }) => return Err(Problem::Untrusted { offer }), // it changed: show the new one
            Err(other) => return Err(other),
            Ok(()) => Default::default(), // trusted by the computer after all
        };
        let client = Client::new(&address, Some(offer.clone()))?;
        client.check()?;
        if !offer.fingerprint.is_empty() {
            let mut prefs = Prefs::load();
            prefs.trusted.insert(address, offer);
            prefs.save();
        }
        Ok(client)
    }).await?;
    *session.0.lock().unwrap() = Some(client);
    Ok(base)
}

#[tauri::command]
async fn sign_in(email: String, password: String, session: State<'_, Session>) -> Result<User, Problem> {
    let mut client = current(&session)?;
    let email = email.trim().to_string();
    let (client, user) = blocking(move || {
        let user = client.login(&email, &password);
        (client, user.map(|u| (u, email)))
    }).await;
    let (user, email) = user?;
    let mut prefs = Prefs::load();
    prefs.server = client.base.clone();
    prefs.email = email;
    prefs.save();
    *session.0.lock().unwrap() = Some(client);
    Ok(user)
}

#[tauri::command]
async fn options(serial: String, hostname: Option<String>, session: State<'_, Session>) -> Result<Value, Problem> {
    let client = current(&session)?;
    blocking(move || client.options(&serial, hostname.as_deref().unwrap_or(""))).await
}

/// Renames this computer to its asset tag (the system asks for administrator rights).
#[tauri::command]
async fn rename_computer(name: String) -> Result<inventario_core::rename::Renamed, String> {
    blocking(move || inventario_core::rename::rename(&name)).await
}

/// Opens the system's page for changing the computer's name.
#[tauri::command]
fn open_name_settings() -> Result<(), String> {
    inventario_core::rename::open_name_settings()
}

#[tauri::command]
async fn send(body: Value, session: State<'_, Session>) -> Result<Value, Problem> {
    let client = current(&session)?;
    blocking(move || {
        let mut result = client.report(&body)?;
        let id = match &result["device"]["id"] { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), _ => String::new() };
        result["url"] = json!(client.device_url(&id));
        Ok(result)
    }).await
}

/// Opens a page of the inventory this window is connected to (and nothing else) in the browser.
#[tauri::command]
fn open_inventory(url: String, app: AppHandle, session: State<Session>) -> Result<(), Problem> {
    let base = current(&session)?.base;
    if url != base && !url.starts_with(&format!("{base}/")) {
        return Err(Problem::BadAddress { detail: "not an inventory address".into() });
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| Problem::Network { detail: e.to_string() })
}

/// Opens the page of the latest release (for copies that cannot install an update themselves).
#[tauri::command]
fn open_release_page(app: AppHandle) -> Result<(), String> {
    app.opener().open_url(update::RELEASE_PAGE, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn sign_out(session: State<Session>) {
    *session.0.lock().unwrap() = None;
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// On Windows the window app has no console of its own; when started from one, write to it.
#[cfg(windows)]
fn use_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS); }
}

fn main() {
    // macOS adds "-psn_..." when an app is opened from the Finder on old versions; that is not a command line
    let args: Vec<std::ffi::OsString> = std::env::args_os().filter(|a| !a.to_string_lossy().starts_with("-psn_")).collect();
    if args.len() > 1 {
        #[cfg(windows)]
        use_parent_console();
        std::process::exit(inventario_core::cli::run(args));
    }

    update::clean_up();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Session::default())
        .manage(update::Pending::default())
        .invoke_handler(tauri::generate_handler![start, collect_facts, connect, trust, sign_in, options, rename_computer, open_name_settings, send, open_inventory,
            sign_out, quit, open_release_page, update::check_update, update::install_update])
        .run(tauri::generate_context!())
        .expect("could not open the window");
}
