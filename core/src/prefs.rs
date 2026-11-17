//! Remembers the last server and email, and the certificates a person chose to trust, in the user's settings
//! folder (for example ~/Library/Application Support/inventario-registro/prefs.json). Passwords are never saved.

use crate::net::Trust;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The inventory address offered until someone uses another one. A build can set its own with the
/// INVENTARIO_DEFAULT_SERVER environment variable.
pub const DEFAULT_SERVER: &str = match option_env!("INVENTARIO_DEFAULT_SERVER") {
    Some(server) => server,
    None => "inventario.zoopa.local",
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub server: String,
    pub email: String,
    /// server address -> the certificate (or certificate authority) a person chose to trust for it
    pub trusted: BTreeMap<String, Trust>,
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("inventario-registro").join("prefs.json"))
}

impl Prefs {
    pub fn load() -> Prefs {
        path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default());
        }
    }

    /// The last address used, or the company's own when none was used yet.
    pub fn server_or_default(&self) -> String {
        if self.server.trim().is_empty() { DEFAULT_SERVER.to_string() } else { self.server.clone() }
    }

    pub fn pinned(&self, server: &str) -> Option<Trust> {
        self.trusted.get(server).cloned()
    }
}
