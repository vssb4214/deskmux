use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub device_name: String,
    pub peers: Vec<Peer>,
    pub devices: Vec<Device>,
    pub monitors: Vec<Monitor>,
    pub presets: HashMap<String, Preset>,
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
    /// Keyed by device id. A monitor only declares the inputs it physically has.
    pub inputs: HashMap<String, Input>,
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
