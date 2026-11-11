//! What is sent to the inventory, in the shape its API expects (see docs/agent-api.md in INVENTARIO-IT).
//! Text fields are empty when unknown; tri-state fields are "yes", "no" or "" (unknown).

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub serial: String,
    pub hostname: String,
    pub manufacturer: String,
    pub model: String,
    /// "laptop", "desktop", "all_in_one" or "unknown"
    pub chassis: String,
    pub cpu: Option<Cpu>,
    pub ram: Option<Ram>,
    pub disks: Vec<Disk>,
    pub gpus: Vec<Gpu>,
    pub screen_inches: Option<f64>,
    pub battery: Option<Battery>,
    pub os: Option<Os>,
    pub encryption: String,
    pub domain_joined: String,
    pub nics: Vec<Nic>,
    pub agent: Agent,
    /// Things that could not be read, as message keys for the person using the app (not used by the server).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub name: String,
    pub cores: Option<u32>,
    pub max_ghz: Option<f64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ram {
    pub gb: Option<f64>,
    #[serde(rename = "type")]
    pub kind: String,
    pub mts: Option<f64>,
    pub replaceable: String,
    pub modules: Option<u32>,
    pub module_gb: Option<f64>,
    pub form_factor: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub gb: f64,
    /// "SSD", "HDD" or "eMMC"
    #[serde(rename = "type")]
    pub kind: String,
    /// "NVMe", "SATA"...
    pub connector: String,
    pub replaceable: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    /// "integrated", "dedicated" or ""
    pub kind: String,
    pub vram_gb: Option<f64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub health_pct: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Os {
    /// "mac", "win" or "linux"
    pub family: String,
    pub version: String,
    pub edition: String,
    pub distro: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Nic {
    /// "wifi" or "eth"
    #[serde(rename = "type")]
    pub kind: String,
    pub model: String,
    pub mac: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub version: String,
    pub platform: String,
}
