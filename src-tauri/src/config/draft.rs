use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::LoadError;
use super::model::Config;
use super::path::{backup_path_for, default_config_path, temp_path_for, CONFIG_FILENAME};
use super::validate::validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigResult {
    pub filename: String,
    pub backup_created: bool,
    pub restart_required: bool,
}

/// Parse a JSON draft and run semantic validation. Does not touch disk.
pub fn parse_config_draft(json: &str) -> Result<Config, LoadError> {
    let config: Config = serde_json::from_str(json)?;
    validate(&config)?;
    Ok(config)
}

/// Validate and atomically write a config draft to the fixed default path.
pub fn save_config_draft(json: &str) -> Result<SaveConfigResult, LoadError> {
    save_config_draft_at(&default_config_path(), json)
}

/// Validate and atomically write a config draft to `path` (tests supply a temp path).
pub fn save_config_draft_at(path: &Path, json: &str) -> Result<SaveConfigResult, LoadError> {
    let config = parse_config_draft(json)?;
    let pretty = serde_json::to_string_pretty(&config).map_err(LoadError::Parse)?;

    let backup_path = backup_path_for(path);
    let temp_path = temp_path_for(path);

    let backup_created = if path.exists() {
        fs::copy(path, &backup_path)?;
        true
    } else {
        false
    };

    fs::write(&temp_path, &pretty)?;
    fs::rename(&temp_path, path)?;

    Ok(SaveConfigResult {
        filename: CONFIG_FILENAME.to_string(),
        backup_created,
        restart_required: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config;
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

    const NATIVE_DDC_JSON: &str = r#"{
        "deviceName": "device-a",
        "peers": [],
        "devices": [{ "id": "device-a", "label": "Device A" }],
        "monitors": [{
            "id": "monitor1",
            "label": "Monitor 1",
            "order": 0,
            "nativeDdc": { "displayId": "K@P:d0e5:0" },
            "inputs": {
                "device-a": {
                    "type": "displayport",
                    "nativeDdc": { "inputSourceValue": 4626 }
                }
            }
        }],
        "presets": {}
    }"#;

    fn temp_config_path(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "deskmux_save_test_{name}_{}_{n}.json",
            std::process::id()
        ))
    }

    fn cleanup_paths(base: &Path) {
        let _ = fs::remove_file(base);
        let _ = fs::remove_file(backup_path_for(base));
        let _ = fs::remove_file(temp_path_for(base));
    }

    #[test]
    fn invalid_draft_returns_validation_error_without_writing() {
        let path = temp_config_path("invalid_no_write");
        cleanup_paths(&path);

        let result = save_config_draft_at(&path, INVALID_JSON);
        assert!(matches!(result, Err(LoadError::Invalid(_))));
        assert!(!path.exists());
        assert!(!backup_path_for(&path).exists());
        assert!(!temp_path_for(&path).exists());

        cleanup_paths(&path);
    }

    #[test]
    fn invalid_draft_leaves_existing_config_untouched() {
        let path = temp_config_path("invalid_preserve");
        cleanup_paths(&path);
        fs::write(&path, VALID_JSON).expect("seed config");

        let result = save_config_draft_at(&path, INVALID_JSON);
        assert!(matches!(result, Err(LoadError::Invalid(_))));
        assert_eq!(fs::read_to_string(&path).expect("read"), VALID_JSON);
        assert!(!backup_path_for(&path).exists());
        assert!(!temp_path_for(&path).exists());

        cleanup_paths(&path);
    }

    #[test]
    fn valid_draft_without_existing_file_writes_config_without_backup() {
        let path = temp_config_path("valid_new");
        cleanup_paths(&path);

        let result = save_config_draft_at(&path, VALID_JSON).expect("save");
        assert_eq!(result.filename, CONFIG_FILENAME);
        assert!(!result.backup_created);
        assert!(result.restart_required);

        let loaded = load_config(&path).expect("reload");
        assert_eq!(loaded.device_name, "device-a");
        assert!(!backup_path_for(&path).exists());
        assert!(!temp_path_for(&path).exists());

        cleanup_paths(&path);
    }

    #[test]
    fn valid_draft_with_existing_file_creates_backup_and_replaces_target() {
        let path = temp_config_path("valid_replace");
        cleanup_paths(&path);
        let original = r#"{ "stale": true }"#;
        fs::write(&path, original).expect("seed stale config");

        save_config_draft_at(&path, VALID_JSON).expect("save");

        let backup = fs::read_to_string(backup_path_for(&path)).expect("backup exists");
        assert_eq!(backup, original);

        let loaded = load_config(&path).expect("reload");
        assert_eq!(loaded.device_name, "device-a");
        assert!(!temp_path_for(&path).exists());

        cleanup_paths(&path);
    }

    #[test]
    fn successful_save_leaves_no_temp_file() {
        let path = temp_config_path("no_tmp");
        cleanup_paths(&path);

        save_config_draft_at(&path, VALID_JSON).expect("save");
        assert!(!temp_path_for(&path).exists());

        cleanup_paths(&path);
    }

    #[test]
    fn native_ddc_u16_value_round_trips_through_save() {
        let path = temp_config_path("native_u16");
        cleanup_paths(&path);

        save_config_draft_at(&path, NATIVE_DDC_JSON).expect("save");
        let loaded = load_config(&path).expect("reload");

        assert_eq!(
            loaded.monitors[0].inputs["device-a"]
                .native_ddc
                .as_ref()
                .map(|n| n.input_source_value),
            Some(4626)
        );

        cleanup_paths(&path);
    }

    #[test]
    fn backup_failure_blocks_overwrite() {
        let path = temp_config_path("backup_fail");
        cleanup_paths(&path);
        fs::write(&path, VALID_JSON).expect("seed config");
        fs::create_dir(backup_path_for(&path)).expect("block backup path");

        let result = save_config_draft_at(&path, NATIVE_DDC_JSON);
        assert!(matches!(result, Err(LoadError::Io(_))));
        assert_eq!(
            load_config(&path)
                .expect("original still valid")
                .device_name,
            "device-a"
        );
        assert!(!temp_path_for(&path).exists());

        let _ = fs::remove_dir(backup_path_for(&path));
        cleanup_paths(&path);
    }

    #[test]
    fn parse_config_draft_does_not_write() {
        let path = temp_config_path("parse_only");
        cleanup_paths(&path);

        parse_config_draft(VALID_JSON).expect("valid draft");
        assert!(!path.exists());

        cleanup_paths(&path);
    }

    #[test]
    fn default_config_path_uses_config_filename() {
        let path = default_config_path();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(CONFIG_FILENAME)
        );
    }
}
