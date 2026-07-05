use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub device_name: String,
    /// Port this machine's DeskMux API listens on. Default 3737.
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    /// When true, bind the API on all interfaces so LAN peers can reach it.
    /// Default false — loopback only. Remote preset triggering is an attack surface.
    #[serde(default)]
    pub api_lan_access: bool,
    pub peers: Vec<Peer>,
    pub devices: Vec<Device>,
    pub monitors: Vec<Monitor>,
    pub presets: HashMap<String, Preset>,
}

fn default_api_port() -> u16 {
    3737
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Peer {
    pub name: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Device {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Monitor {
    pub id: String,
    pub label: String,
    pub order: u32,
    /// Device id of the machine that runs DDC for this monitor. When omitted, defaults to
    /// this config's `deviceName` (see `Monitor::controlled_by`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlled_by: Option<String>,
    /// Keyed by device id. Required on the owning machine; remote stubs may omit inputs.
    #[serde(default)]
    pub inputs: HashMap<String, Input>,
}

impl Monitor {
    /// Effective command owner for this monitor. Not a serde default — `deviceName` lives on
    /// the parent [`Config`], so callers pass it explicitly after load.
    pub fn controlled_by<'a>(&'a self, device_name: &'a str) -> &'a str {
        self.controlled_by.as_deref().unwrap_or(device_name)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Input {
    #[serde(rename = "type")]
    pub kind: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Preset {
    pub label: String,
    /// Maps monitorId -> deviceId.
    pub layout: HashMap<String, String>,
}
