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
    /// How many full charge cycles the battery has gone through, when the system says
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery_cycles: Option<u32>,
    pub os: Option<Os>,
    pub encryption: String,
    pub domain_joined: String,
    /// Macs only (None elsewhere): OpenCore Legacy Patcher's version ("2.1.0") when the Mac starts up through it to
    /// run a macOS Apple no longer supports on it; "yes" when it starts through OpenCore but the version is unknown;
    /// "" when it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oclp: Option<String>,
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
    /// "SATA III", "NVMe PCIe 3.0" when the generation is known; "SATA", "NVMe"... otherwise
    pub connector: String,
    pub replaceable: String,
    /// "2.5\"", "3.5\"", "M.2", "Apple (propietario)" when it can be told; "" otherwise
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub form_factor: String,
    /// What the drive says about itself (SMART), read only: "ok", "warning", "failing", or "" when not said
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub health: String,
    /// SSDs: how much of its rated life is used, in percent (can pass 100)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wear_pct: Option<f64>,
    /// Hours the drive has been powered on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<u32>,
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
    /// Wi-Fi only: "4", "5", "6", "6E" or "7"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub wifi_gen: String,
    /// Wi-Fi only: the Bluetooth version of the same chip, "5.3"
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bt_ver: String,
    /// Ethernet only: the fastest speed the port supports, in Mbit/s (1000 for Gigabit Ethernet)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_mbps: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub version: String,
    pub platform: String,
}
