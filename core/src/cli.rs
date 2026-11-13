//! Command-line mode, for scripts and remote sessions:
//!
//!   inventario-registro --server inventario.empresa.es --email it@empresa.es
//!
//! Exit codes: 0 done, 2 wrong options, 3 certificate not trusted, 4 sign-in refused, 5 any other problem.

use crate::collect;
use crate::facts::Facts;
use crate::net::{self, Client, Problem};
use crate::prefs::Prefs;
use clap::Parser;
use serde_json::{json, Value};
use std::io::{BufRead, IsTerminal, Write};

#[derive(Parser, Debug)]
#[command(name = "inventario-registro", version = crate::VERSION,
          about = "Adds this computer to the inventory, or updates it if its serial number is already there.",
          after_help = "Exit codes: 0 done, 2 wrong options, 3 certificate not trusted, 4 sign-in refused, 5 other problem.")]
pub struct Args {
    /// Address of the inventory, for example inventario.empresa.es
    #[arg(long, short = 's')]
    pub server: Option<String>,
    /// Email of an inventory account with the technician or administrator role
    #[arg(long, short = 'e')]
    pub email: Option<String>,
    /// Read the password from this environment variable instead of asking for it
    #[arg(long, value_name = "VARIABLE", conflicts_with = "password_stdin")]
    pub password_env: Option<String>,
    /// Read the password from the first line of standard input
    #[arg(long)]
    pub password_stdin: bool,
    /// Asset tag for a new computer (default: the next free one)
    #[arg(long)]
    pub tag: Option<String>,
    /// Device type for a new computer, by name or id (default: one that fits the chassis)
    #[arg(long = "type", value_name = "TYPE")]
    pub kind: Option<String>,
    /// Location for a new computer, by name or id (default: the storage location)
    #[arg(long)]
    pub location: Option<String>,
    /// Serial number to use when the computer does not report one
    #[arg(long)]
    pub serial: Option<String>,
    /// Trust the server's own certificate (or certificate authority) with this SHA-256 fingerprint
    #[arg(long, value_name = "SHA256")]
    pub trust: Option<String>,
    /// Only show the details that would be sent; nothing is sent
    #[arg(long)]
    pub dry_run: bool,
    /// Print the result as JSON
    #[arg(long)]
    pub json: bool,
}

struct Fail(i32, String);

fn usage(message: impl Into<String>) -> Fail { Fail(2, message.into()) }

/// Plain-English text for the inventory's error keys.
pub fn describe(code: &str) -> &'static str {
    match code {
        "login_err" => "wrong email or password",
        "login_off" => "this account is deactivated",
        "login_locked" => "too many failed attempts; wait a few minutes and try again",
        "err_must_change" => "this account must change its password on the inventory website first",
        "err_forbidden" => "this account can only read the inventory; a technician or administrator account is needed",
        "err_auth" => "the session expired; sign in again",
        "err_dup" => "another device already uses that asset tag or its identification number",
        "err_required" => "the inventory is missing something it needs (a serial number, a device type or a location)",
        "maintenance" => "the inventory is being restored or updated; try again in a few minutes",
        "not_inventory" => "that address does not look like an inventory",
        "err_invalid" => "the server did not accept the request",
        _ => "the server refused the request",
    }
}

fn exit_code(problem: &Problem) -> i32 {
    match problem {
        Problem::Untrusted { .. } => 3,
        Problem::Server { code, .. } if ["login_err", "login_off", "login_locked", "err_must_change", "err_forbidden", "err_auth"].contains(&code.as_str()) => 4,
        Problem::BadAddress { .. } => 2,
        _ => 5,
    }
}

fn fail(problem: Problem) -> Fail {
    let text = match &problem {
        Problem::Server { code, .. } => describe(code).to_string(),
        other => other.to_string(),
    };
    Fail(exit_code(&problem), text)
}

fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

fn ask(question: &str) -> String {
    eprint!("{question}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    line.trim().to_string()
}

/// A name in any of the inventory's languages, or the id.
fn find_named(list: &Value, wanted: &str, what: &str) -> Result<String, Fail> {
    let wanted = wanted.trim().to_lowercase();
    let items = list.as_array().cloned().unwrap_or_default();
    for item in &items {
        let id = item["id"].as_str().unwrap_or("").to_string();
        let names: Vec<String> = match &item["nombre"] {
            Value::Object(map) => map.values().filter_map(|v| v.as_str()).map(|s| s.to_lowercase()).collect(),
            Value::String(s) => vec![s.to_lowercase()],
            _ => vec![],
        };
        if id == wanted || names.contains(&wanted) {
            return Ok(id);
        }
    }
    let choices: Vec<String> = items.iter().filter_map(|i| i["nombre"]["en"].as_str().or(i["nombre"].as_str()).map(String::from)).collect();
    Err(usage(format!("there is no {what} called \"{wanted}\"; choose one of: {}", choices.join(", "))))
}

fn print_facts(f: &Facts) {
    let line = |label: &str, value: String| if !value.trim().is_empty() { println!("  {label:<14}{value}") };
    line("Serial", f.serial.clone());
    line("Name", f.hostname.clone());
    line("Model", format!("{} {}", f.manufacturer, f.model));
    if let Some(cpu) = &f.cpu { line("Processor", cpu.name.clone()); }
    if let Some(ram) = &f.ram { line("Memory", ram.gb.map(|g| format!("{g} GB {}", ram.kind)).unwrap_or_default()); }
    line("Drives", f.disks.iter().map(|d| format!("{} GB {} {}", d.gb, d.kind, d.connector).trim().to_string()).collect::<Vec<_>>().join(", "));
    line("Graphics", f.gpus.iter().map(|g| g.name.clone()).collect::<Vec<_>>().join(", "));
    if let Some(os) = &f.os {
        let family = match os.family.as_str() { "mac" => "macOS", "win" => "Windows", _ => "Linux" };
        let name = if os.distro.is_empty() { family } else { &os.distro };
        line("System", format!("{name} {} {}", os.version, os.edition));
    }
    if let Some(inches) = f.screen_inches { line("Screen", format!("{inches}\"")); }
    if let Some(b) = &f.battery { line("Battery", format!("{}%", b.health_pct)); }
    line("Encryption", f.encryption.clone());
    line("Network", f.nics.iter().map(|n| {
        let kind = if n.kind == "wifi" { format!("Wi-Fi {}", n.wifi_gen).trim().to_string() } else { "Ethernet".into() };
        let bt = if n.bt_ver.is_empty() { String::new() } else { format!("Bluetooth {}", n.bt_ver) };
        [kind, n.model.clone(), bt].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
    }).collect::<Vec<_>>().join(", "));
    for w in &f.warnings {
        eprintln!("note: {}", warning_text(w));
    }
}

pub fn warning_text(key: &str) -> &'static str {
    match key {
        "serial_missing" => "this computer does not report a serial number; give one with --serial",
        "serial_needs_admin" => "run as root (sudo) to read the serial number and memory details",
        "battery_needs_admin" => "battery health can only be read when running as administrator",
        "windows_tools_failed" => "Windows did not answer the hardware questions (PowerShell)",
        _ => "some details could not be read",
    }
}

/// Runs the command line with these arguments (the program name first) and returns the exit code.
pub fn run<I, T>(args: I) -> i32
where I: IntoIterator<Item = T>, T: Into<std::ffi::OsString> + Clone {
    let args = match Args::try_parse_from(args) {
        Ok(a) => a,
        Err(e) => {
            let code = if e.use_stderr() { 2 } else { 0 };
            let _ = e.print();
            return code;
        }
    };
    let json = args.json;
    match register(args) {
        Ok(()) => 0,
        Err(Fail(code, message)) => {
            if json {
                println!("{}", json!({ "ok": false, "exit": code, "error": message }));
            } else {
                eprintln!("error: {message}");
            }
            code
        }
    }
}

fn register(args: Args) -> Result<(), Fail> {
    let mut facts = collect::collect();
    if let Some(serial) = args.serial.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        facts.serial = serial.to_string();
        facts.warnings.retain(|w| w != "serial_missing");
    }
    if args.dry_run {
        if args.json { println!("{}", serde_json::to_string_pretty(&facts).unwrap()); } else { print_facts(&facts); }
        return Ok(());
    }
    if facts.serial.is_empty() {
        return Err(usage(warning_text("serial_missing")));
    }

    let mut prefs = Prefs::load();
    let server = match args.server.clone().or_else(|| interactive().then(|| ask("Inventory address: "))) {
        Some(s) if !s.trim().is_empty() => net::normalize_url(&s).map_err(fail)?,
        _ => return Err(usage("give the inventory's address with --server")),
    };
    let email = match args.email.clone().or_else(|| interactive().then(|| ask("Email: "))) {
        Some(e) if !e.trim().is_empty() => e.trim().to_string(),
        _ => return Err(usage("give the account's email with --email")),
    };

    // the certificate is checked before the password is even asked for
    let mut client = Client::new(&server, prefs.pinned(&server)).map_err(fail)?;
    match client.check() {
        Ok(()) => {}
        Err(Problem::Untrusted { offer }) => {
            let what = if offer.ca.is_empty() { "certificate" } else { "certificate authority" };
            let fp = &offer.fingerprint;
            match &args.trust {
                Some(given) if offer.matches(given) => {}
                Some(_) => return Err(Fail(3, format!("the fingerprint given with --trust does not match the server's {what} ({fp})"))),
                None => {
                    eprintln!("The server uses its own certificate, which this computer does not trust.");
                    eprintln!("SHA-256 fingerprint of its {what}: {fp}");
                    eprintln!("Only trust it if this is your inventory server; whoever installed it can confirm the fingerprint.");
                    if !interactive() || !matches!(ask("Trust it for this server? [y/N] ").to_lowercase().as_str(), "y" | "yes" | "s" | "si" | "sí") {
                        return Err(Fail(3, format!("certificate not trusted; to trust it, add --trust {fp}")));
                    }
                }
            }
            client = Client::new(&server, Some(offer.clone())).map_err(fail)?;
            client.check().map_err(fail)?;
            prefs.trusted.insert(server.clone(), offer);
        }
        Err(other) => return Err(fail(other)),
    }

    let password = if let Some(var) = &args.password_env {
        std::env::var(var).map_err(|_| usage(format!("the environment variable {var} is not set")))?
    } else if args.password_stdin {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).map_err(|e| usage(e.to_string()))?;
        line.trim_end_matches(['\r', '\n']).to_string()
    } else if interactive() {
        rpassword::prompt_password("Password: ").map_err(|e| usage(e.to_string()))?
    } else {
        return Err(usage("no password: use --password-env or --password-stdin"));
    };

    let user = client.login(&email, &password).map_err(fail)?;
    drop(password);
    prefs.server = server.clone();
    prefs.email = email.clone();
    prefs.save();

    let options = client.options(&facts.serial).map_err(fail)?;
    let existing = options["existing"].clone();
    let mut body = json!({ "facts": facts });
    if existing.is_null() {
        if let Some(tag) = &args.tag { body["tag"] = json!(tag.trim()); }
        if let Some(kind) = &args.kind { body["typeId"] = json!(find_named(&options["types"], kind, "device type")?); }
        if let Some(loc) = &args.location { body["loc"] = json!(find_named(&options["locations"], loc, "location")?); }
    } else if args.tag.is_some() || args.kind.is_some() || args.location.is_some() {
        eprintln!("note: this computer is already in the inventory as {}; --tag, --type and --location are ignored",
                  existing["tag"].as_str().unwrap_or("?"));
    }

    let result = client.report(&body).map_err(fail)?;
    let device = &result["device"];
    let created = result["created"].as_bool().unwrap_or(false);
    let id = device["id"].as_str().map(String::from).or_else(|| device["id"].as_i64().map(|n| n.to_string())).unwrap_or_default();
    if args.json {
        println!("{}", json!({ "ok": true, "created": created, "tag": device["tag"], "id": id, "url": client.device_url(&id), "by": user.nombre }));
    } else {
        print_facts(&facts);
        println!("{} {} ({})", if created { "Added to the inventory as" } else { "Updated in the inventory:" },
                 device["tag"].as_str().unwrap_or("?"), client.device_url(&id));
    }
    Ok(())
}
