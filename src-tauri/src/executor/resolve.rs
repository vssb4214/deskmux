use std::collections::HashMap;

use super::model::{ExecutorError, ResolutionError};
use crate::config::Config;

#[derive(Debug, PartialEq)]
pub(super) struct ResolvedCommand {
    pub monitor_id: String,
    pub device_id: String,
    pub command: String,
}

#[derive(Debug, PartialEq)]
pub(super) enum ResolvedEntry {
    Ready(ResolvedCommand),
    Failed {
        monitor_id: String,
        device_id: String,
        error: ResolutionError,
    },
}

/// A preset layout entry to resolve: `(monitorId, deviceId)`.
pub type LayoutEntry = (String, String);

/// Resolves only the supplied layout entries to commands. Entries are ordered by the
/// monitor's `order` field (unknown monitors sort last). Pure: no I/O.
pub(super) fn resolve_layout_entries(
    config: &Config,
    entries: &[LayoutEntry],
) -> Vec<ResolvedEntry> {
    let monitors_by_id: HashMap<&str, _> =
        config.monitors.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut sorted: Vec<&LayoutEntry> = entries.iter().collect();
    sorted.sort_by(|(a_id, _), (b_id, _)| {
        let a_order = monitors_by_id
            .get(a_id.as_str())
            .map_or(u32::MAX, |m| m.order);
        let b_order = monitors_by_id
            .get(b_id.as_str())
            .map_or(u32::MAX, |m| m.order);
        a_order.cmp(&b_order).then_with(|| a_id.cmp(b_id))
    });

    sorted
        .into_iter()
        .map(
            |(monitor_id, device_id)| match monitors_by_id.get(monitor_id.as_str()) {
                None => ResolvedEntry::Failed {
                    monitor_id: monitor_id.clone(),
                    device_id: device_id.clone(),
                    error: ResolutionError::UnknownMonitor {
                        monitor_id: monitor_id.clone(),
                    },
                },
                Some(monitor) => match monitor.inputs.get(device_id.as_str()) {
                    None => ResolvedEntry::Failed {
                        monitor_id: monitor_id.clone(),
                        device_id: device_id.clone(),
                        error: ResolutionError::UnknownDevice {
                            monitor_id: monitor_id.clone(),
                            device_id: device_id.clone(),
                        },
                    },
                    Some(input) => ResolvedEntry::Ready(ResolvedCommand {
                        monitor_id: monitor_id.clone(),
                        device_id: device_id.clone(),
                        command: input.command.clone(),
                    }),
                },
            },
        )
        .collect()
}

/// Returns the layout entries for `preset_name`.
pub(super) fn preset_layout_entries(
    config: &Config,
    preset_name: &str,
) -> Result<Vec<LayoutEntry>, ExecutorError> {
    let preset = config
        .presets
        .get(preset_name)
        .ok_or_else(|| ExecutorError::PresetNotFound {
            preset_name: preset_name.to_string(),
        })?;

    Ok(preset
        .layout
        .iter()
        .map(|(monitor_id, device_id)| (monitor_id.clone(), device_id.clone()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_config() -> Config {
        let json = r#"{
            "deviceName": "device-a",
            "peers": [],
            "devices": [
                { "id": "device-a", "label": "Device A" },
                { "id": "device-b", "label": "Device B" }
            ],
            "monitors": [
                {
                    "id": "monitor2",
                    "label": "Monitor 2",
                    "order": 1,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-monitor2-a" },
                        "device-b": { "type": "displayport", "command": "cmd-monitor2-b" }
                    }
                },
                {
                    "id": "monitor1",
                    "label": "Monitor 1",
                    "order": 0,
                    "inputs": {
                        "device-a": { "type": "hdmi", "command": "cmd-monitor1-a" }
                    }
                }
            ],
            "presets": {
                "valid_preset": {
                    "label": "Valid",
                    "layout": { "monitor1": "device-a", "monitor2": "device-b" }
                },
                "unknown_monitor_preset": {
                    "label": "Unknown monitor",
                    "layout": { "ghost-monitor": "device-a" }
                },
                "unknown_device_preset": {
                    "label": "Unknown device",
                    "layout": { "monitor1": "device-b" }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    #[test]
    fn resolves_entries_in_monitor_order() {
        let config = fixture_config();

        let entries = preset_layout_entries(&config, "valid_preset").expect("should resolve");
        let resolved = resolve_layout_entries(&config, &entries);

        assert_eq!(
            resolved,
            vec![
                ResolvedEntry::Ready(ResolvedCommand {
                    monitor_id: "monitor1".to_string(),
                    device_id: "device-a".to_string(),
                    command: "cmd-monitor1-a".to_string(),
                }),
                ResolvedEntry::Ready(ResolvedCommand {
                    monitor_id: "monitor2".to_string(),
                    device_id: "device-b".to_string(),
                    command: "cmd-monitor2-b".to_string(),
                }),
            ]
        );
    }

    #[test]
    fn resolve_layout_entries_only_resolves_supplied_entries() {
        let config = fixture_config();
        let entries = vec![("monitor1".to_string(), "device-a".to_string())];

        let resolved = resolve_layout_entries(&config, &entries);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Ready(ResolvedCommand {
                monitor_id: "monitor1".to_string(),
                device_id: "device-a".to_string(),
                command: "cmd-monitor1-a".to_string(),
            })]
        );
    }

    #[test]
    fn filtered_resolve_does_not_touch_unlisted_monitors() {
        let config = fixture_config();
        let entries = vec![("monitor1".to_string(), "device-a".to_string())];

        let resolved = resolve_layout_entries(&config, &entries);

        assert!(!resolved.iter().any(|entry| match entry {
            ResolvedEntry::Failed { monitor_id, .. }
            | ResolvedEntry::Ready(ResolvedCommand { monitor_id, .. }) => {
                monitor_id == "ghost-monitor"
            }
        }));
    }

    #[test]
    fn unknown_monitor_in_layout_is_a_resolution_error() {
        let config = fixture_config();

        let entries =
            preset_layout_entries(&config, "unknown_monitor_preset").expect("should resolve");
        let resolved = resolve_layout_entries(&config, &entries);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Failed {
                monitor_id: "ghost-monitor".to_string(),
                device_id: "device-a".to_string(),
                error: ResolutionError::UnknownMonitor {
                    monitor_id: "ghost-monitor".to_string(),
                },
            }]
        );
    }

    #[test]
    fn device_with_no_input_is_a_resolution_error() {
        let config = fixture_config();

        let entries =
            preset_layout_entries(&config, "unknown_device_preset").expect("should resolve");
        let resolved = resolve_layout_entries(&config, &entries);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Failed {
                monitor_id: "monitor1".to_string(),
                device_id: "device-b".to_string(),
                error: ResolutionError::UnknownDevice {
                    monitor_id: "monitor1".to_string(),
                    device_id: "device-b".to_string(),
                },
            }]
        );
    }

    #[test]
    fn unknown_preset_name_is_an_executor_error() {
        let config = fixture_config();

        let result = preset_layout_entries(&config, "does-not-exist");

        assert_eq!(
            result.unwrap_err(),
            ExecutorError::PresetNotFound {
                preset_name: "does-not-exist".to_string(),
            }
        );
    }
}
