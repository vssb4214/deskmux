use std::collections::HashMap;

use crate::config::{Config, ConfigError};

/// Parses a global shortcut string using the same rules as runtime registration.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn parse_shortcut(shortcut: &str) -> Result<(), String> {
    use std::str::FromStr;

    tauri_plugin_global_shortcut::Shortcut::from_str(shortcut)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn parse_shortcut(_shortcut: &str) -> Result<(), String> {
    Ok(())
}

pub fn validate_hotkeys(config: &Config, errors: &mut Vec<ConfigError>) {
    if config.hotkeys.is_empty() {
        return;
    }

    let mut shortcut_owners: HashMap<String, String> = HashMap::new();

    for (preset_name, shortcut) in &config.hotkeys {
        if !config.presets.contains_key(preset_name) {
            errors.push(ConfigError::UnknownHotkeyPreset {
                preset_name: preset_name.clone(),
            });
            continue;
        }

        if parse_shortcut(shortcut).is_err() {
            errors.push(ConfigError::InvalidHotkeyShortcut {
                preset_name: preset_name.clone(),
                shortcut: shortcut.clone(),
            });
            continue;
        }

        let normalized = normalize_shortcut(shortcut);
        if let Some(existing) = shortcut_owners.get(&normalized) {
            errors.push(ConfigError::DuplicateHotkey {
                shortcut: shortcut.clone(),
                preset_a: existing.clone(),
                preset_b: preset_name.clone(),
            });
        } else {
            shortcut_owners.insert(normalized, preset_name.clone());
        }
    }
}

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ConfigError};

    fn base_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [{ "id": "device-a", "label": "Device A" }],
            "monitors": [{
                "id": "monitor1",
                "label": "Monitor 1",
                "order": 0,
                "inputs": { "device-a": { "type": "hdmi", "command": "cmd" } }
            }],
            "presets": {
                "all_a": { "label": "All A", "layout": { "monitor1": "device-a" } }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config")
    }

    #[test]
    fn valid_hotkeys_pass_validation() {
        let mut config = base_config();
        config
            .hotkeys
            .insert("all_a".to_string(), "Ctrl+Alt+1".to_string());

        let mut errors = Vec::new();
        validate_hotkeys(&config, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_hotkey_preset_fails() {
        let mut config = base_config();
        config
            .hotkeys
            .insert("missing".to_string(), "Ctrl+Alt+1".to_string());

        let mut errors = Vec::new();
        validate_hotkeys(&config, &mut errors);
        assert_eq!(
            errors,
            vec![ConfigError::UnknownHotkeyPreset {
                preset_name: "missing".to_string(),
            }]
        );
    }

    #[test]
    fn duplicate_hotkey_fails() {
        let mut config = base_config();
        config.presets.insert(
            "all_b".to_string(),
            serde_json::from_str(r#"{ "label": "All B", "layout": { "monitor1": "device-a" } }"#)
                .expect("preset"),
        );
        config
            .hotkeys
            .insert("all_a".to_string(), "Ctrl+Alt+1".to_string());
        config
            .hotkeys
            .insert("all_b".to_string(), "ctrl+alt+1".to_string());

        let mut errors = Vec::new();
        validate_hotkeys(&config, &mut errors);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ConfigError::DuplicateHotkey { .. })));
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn invalid_hotkey_shortcut_fails() {
        let mut config = base_config();
        config
            .hotkeys
            .insert("all_a".to_string(), "not a real shortcut".to_string());

        let mut errors = Vec::new();
        validate_hotkeys(&config, &mut errors);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ConfigError::InvalidHotkeyShortcut { .. })));
    }
}
