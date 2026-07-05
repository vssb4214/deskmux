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
    /// Optional global hotkeys mapping preset name → shortcut string (desktop only).
    #[serde(default)]
    pub hotkeys: HashMap<String, String>,
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
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub label: String,
    pub order: u32,
    /// Device id of the machine that runs DDC for this monitor. When omitted, defaults to
    /// this config's `deviceName` (see `Monitor::controlled_by`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlled_by: Option<String>,
    /// This monitor's identity for native DDC/CI control (Windows only for now). Required if
    /// any of this monitor's `inputs` set their own `nativeDdc`. See docs/CONFIG.md for how
    /// `displayId` is derived and its known limitation with identical monitor models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_ddc: Option<MonitorNativeDdc>,
    /// Keyed by device id. Required on the owning machine; remote stubs may omit inputs.
    #[serde(default)]
    pub inputs: HashMap<String, Input>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonitorNativeDdc {
    /// EDID-derived identity for this physical display (see docs/CONFIG.md).
    pub display_id: String,
}

impl Monitor {
    /// Effective command owner for this monitor. Not a serde default — `deviceName` lives on
    /// the parent [`Config`], so callers pass it explicitly after load.
    pub fn controlled_by<'a>(&'a self, device_name: &'a str) -> &'a str {
        self.controlled_by.as_deref().unwrap_or(device_name)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    #[serde(rename = "type")]
    pub kind: String,
    /// Shell command that selects this input. Optional if `nativeDdc` is set instead — but at
    /// least one of the two is required (validated). Existing configs always set this, so
    /// nothing changes for them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Native DDC/CI input-source parameters (Windows only for now). Requires this input's
    /// monitor to also set `nativeDdc.displayId`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_ddc: Option<InputNativeDdc>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputNativeDdc {
    /// The VCP input-source (code 0x60) value that selects this device's input on this
    /// monitor. Read it off the monitor, same as shell command values — DeskMux doesn't guess.
    /// Input-source is the only capability this exposes for now; there is deliberately no raw
    /// VCP code field here. Values are monitor-specific and often exceed 255 on real hardware.
    pub input_source_value: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Preset {
    pub label: String,
    /// Maps monitorId -> deviceId.
    pub layout: HashMap<String, String>,
}
