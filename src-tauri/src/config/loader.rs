use std::fs;
use std::path::Path;

use super::error::LoadError;
use super::model::Config;
use super::validate::validate;

pub fn load_config(path: &Path) -> Result<Config, LoadError> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&contents)?;
    validate(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    const VALID_JSON: &str = r#"{
        "deviceName": "device-a",
        "peers": [],
        "devices": [
            { "id": "device-a", "label": "Device A" }
        ],
        "monitors": [
            {
                "id": "monitor1",
                "label": "Monitor 1",
                "order": 0,
                "inputs": {
                    "device-a": { "type": "hdmi", "command": "cmd-a" }
                }
            }
        ],
        "presets": {
            "all_a": { "label": "All A", "layout": { "monitor1": "device-a" } }
        }
    }"#;

    const INVALID_JSON: &str = r#"{
        "deviceName": "ghost",
        "peers": [],
        "devices": [
            { "id": "device-a", "label": "Device A" }
        ],
        "monitors": [],
        "presets": {}
    }"#;

    fn temp_config_path(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "deskmux_test_{name}_{}_{n}.json",
            std::process::id()
        ))
    }

    fn write_and_load(name: &str, contents: &str) -> Result<Config, LoadError> {
        let path = temp_config_path(name);
        fs::write(&path, contents).expect("write temp config");
        let result = load_config(&path);
        let _ = fs::remove_file(&path);
        result
    }

    #[test]
    fn loads_valid_config() {
        let config = write_and_load("valid", VALID_JSON).expect("should load");
        assert_eq!(config.device_name, "device-a");
    }

    #[test]
    fn missing_file_is_io_error() {
        let path = temp_config_path("missing");
        let result = load_config(&path);
        assert!(matches!(result, Err(LoadError::Io(_))));
    }

    #[test]
    fn malformed_json_is_parse_error() {
        let result = write_and_load("malformed", "{ not valid json");
        assert!(matches!(result, Err(LoadError::Parse(_))));
    }

    #[test]
    fn semantically_invalid_config_is_invalid_error() {
        let result = write_and_load("invalid", INVALID_JSON);
        assert!(matches!(result, Err(LoadError::Invalid(_))));
    }

    #[test]
    fn loads_config_with_native_ddc_fields() {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [
                { "id": "device-a", "label": "Device A" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "Monitor 1",
                    "order": 0,
                    "nativeDdc": { "displayId": "DEL4176:0" },
                    "inputs": {
                        "device-a": {
                            "type": "displayport",
                            "nativeDdc": { "inputSourceValue": 15 }
                        }
                    }
                }
            ],
            "presets": {
                "all_a": { "label": "All A", "layout": { "monitor1": "device-a" } }
            }
        }"#;

        let config = write_and_load("native-ddc", json).expect("should load");

        let monitor = &config.monitors[0];
        assert_eq!(
            monitor.native_ddc.as_ref().map(|n| n.display_id.as_str()),
            Some("DEL4176:0")
        );
        let input = &monitor.inputs["device-a"];
        assert_eq!(input.command, None);
        assert_eq!(
            input.native_ddc.as_ref().map(|n| n.input_source_value),
            Some(15)
        );
    }

    /// Guards the input-source-only boundary: a raw VCP code has no field to attach to, and
    /// `deny_unknown_fields` makes that a hard parse error rather than a silently ignored one.
    #[test]
    fn native_ddc_rejects_unknown_fields() {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [
                { "id": "device-a", "label": "Device A" }
            ],
            "monitors": [
                {
                    "id": "monitor1",
                    "label": "Monitor 1",
                    "order": 0,
                    "nativeDdc": { "displayId": "DEL4176:0" },
                    "inputs": {
                        "device-a": {
                            "type": "displayport",
                            "nativeDdc": { "inputSourceValue": 15, "vcpCode": 98 }
                        }
                    }
                }
            ],
            "presets": {}
        }"#;

        let result = write_and_load("native-ddc-unknown-field", json);

        assert!(matches!(result, Err(LoadError::Parse(_))));
    }
}
