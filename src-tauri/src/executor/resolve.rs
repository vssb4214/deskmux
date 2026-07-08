use std::collections::HashMap;

use super::backend::BackendAction;
use super::model::{ExecutorError, ResolutionError};
use super::NativeDdcFeature;
use crate::config::{Config, Input, Monitor};

/// Decides which backend action an input resolves to. Native DDC is only chosen when
/// `native_available` (the platform can actually run it) and both the monitor's `nativeDdc`
/// and this input's `nativeDdc` are set; otherwise falls back to the shell `command` if one is
/// configured. `native_available` is a parameter rather than a hard-coded platform check
/// specifically so this decision is testable on any OS (see executor::mod's default).
fn select_action(
    monitor: &Monitor,
    input: &Input,
    native_available: bool,
) -> Option<BackendAction> {
    if native_available {
        if let (Some(monitor_native), Some(input_native)) = (&monitor.native_ddc, &input.native_ddc)
        {
            return Some(BackendAction::NativeDdc {
                display_id: monitor_native.display_id.clone(),
                feature: NativeDdcFeature::InputSource,
                value: input_native.input_source_value,
            });
        }
    }
    input.command.clone().map(BackendAction::Shell)
}

#[derive(Debug, PartialEq)]
pub(super) struct ResolvedCommand {
    pub monitor_id: String,
    pub device_id: String,
    pub action: BackendAction,
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
    native_available: bool,
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
                    Some(input) => match select_action(monitor, input, native_available) {
                        Some(action) => ResolvedEntry::Ready(ResolvedCommand {
                            monitor_id: monitor_id.clone(),
                            device_id: device_id.clone(),
                            action,
                        }),
                        None => ResolvedEntry::Failed {
                            monitor_id: monitor_id.clone(),
                            device_id: device_id.clone(),
                            error: ResolutionError::NoBackendAvailable {
                                monitor_id: monitor_id.clone(),
                                device_id: device_id.clone(),
                            },
                        },
                    },
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
                },
                {
                    "id": "monitor3",
                    "label": "Monitor 3",
                    "order": 2,
                    "nativeDdc": { "displayId": "DEL4176:0" },
                    "inputs": {
                        "device-a": {
                            "type": "displayport",
                            "command": "cmd-monitor3-a",
                            "nativeDdc": { "inputSourceValue": 15 }
                        },
                        "device-b": {
                            "type": "hdmi",
                            "nativeDdc": { "inputSourceValue": 17 }
                        }
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
                },
                "native_preset": {
                    "label": "Native",
                    "layout": { "monitor3": "device-a" }
                },
                "native_only_preset": {
                    "label": "Native only",
                    "layout": { "monitor3": "device-b" }
                }
            }
        }"#;
        serde_json::from_str(json).expect("fixture config should parse")
    }

    #[test]
    fn resolves_entries_in_monitor_order() {
        let config = fixture_config();

        let entries = preset_layout_entries(&config, "valid_preset").expect("should resolve");
        let resolved = resolve_layout_entries(&config, &entries, false);

        assert_eq!(
            resolved,
            vec![
                ResolvedEntry::Ready(ResolvedCommand {
                    monitor_id: "monitor1".to_string(),
                    device_id: "device-a".to_string(),
                    action: BackendAction::Shell("cmd-monitor1-a".to_string()),
                }),
                ResolvedEntry::Ready(ResolvedCommand {
                    monitor_id: "monitor2".to_string(),
                    device_id: "device-b".to_string(),
                    action: BackendAction::Shell("cmd-monitor2-b".to_string()),
                }),
            ]
        );
    }

    #[test]
    fn resolve_layout_entries_only_resolves_supplied_entries() {
        let config = fixture_config();
        let entries = vec![("monitor1".to_string(), "device-a".to_string())];

        let resolved = resolve_layout_entries(&config, &entries, false);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Ready(ResolvedCommand {
                monitor_id: "monitor1".to_string(),
                device_id: "device-a".to_string(),
                action: BackendAction::Shell("cmd-monitor1-a".to_string()),
            })]
        );
    }

    #[test]
    fn filtered_resolve_does_not_touch_unlisted_monitors() {
        let config = fixture_config();
        let entries = vec![("monitor1".to_string(), "device-a".to_string())];

        let resolved = resolve_layout_entries(&config, &entries, false);

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
        let resolved = resolve_layout_entries(&config, &entries, false);

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
        let resolved = resolve_layout_entries(&config, &entries, false);

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

    /// Shell-only inputs (no `nativeDdc`) must behave identically to before native DDC existed,
    /// regardless of `native_available` — there's nothing for selection to choose between.
    #[test]
    fn shell_only_input_resolves_to_shell_regardless_of_native_available() {
        let config = fixture_config();
        let entries = preset_layout_entries(&config, "valid_preset").expect("should resolve");

        for native_available in [false, true] {
            let resolved = resolve_layout_entries(&config, &entries, native_available);
            assert!(!resolved.is_empty());
            assert!(resolved.iter().all(|entry| matches!(
                entry,
                ResolvedEntry::Ready(ResolvedCommand {
                    action: BackendAction::Shell(_),
                    ..
                })
            )));
        }
    }

    /// Real monitors report VCP 0x60 input-source values well above 255 (e.g. 4626, 4623).
    /// This must survive config parse → resolution → `BackendAction::NativeDdc` intact.
    #[test]
    fn native_ddc_input_preserves_large_input_source_value() {
        let json = r#"{
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
            "presets": {
                "desktop": { "label": "Desktop", "layout": { "monitor1": "device-a" } }
            }
        }"#;
        let config: Config =
            serde_json::from_str(json).expect("large inputSourceValue should parse");
        let entries = preset_layout_entries(&config, "desktop").expect("should resolve");
        let resolved = resolve_layout_entries(&config, &entries, true);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Ready(ResolvedCommand {
                monitor_id: "monitor1".to_string(),
                device_id: "device-a".to_string(),
                action: BackendAction::NativeDdc {
                    display_id: "K@P:d0e5:0".to_string(),
                    feature: NativeDdcFeature::InputSource,
                    value: 4626,
                },
            })]
        );
    }

    #[test]
    fn native_ddc_input_selects_native_when_available() {
        let config = fixture_config();
        let entries = preset_layout_entries(&config, "native_preset").expect("should resolve");

        let resolved = resolve_layout_entries(&config, &entries, true);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Ready(ResolvedCommand {
                monitor_id: "monitor3".to_string(),
                device_id: "device-a".to_string(),
                action: BackendAction::NativeDdc {
                    display_id: "DEL4176:0".to_string(),
                    feature: NativeDdcFeature::InputSource,
                    value: 15,
                },
            })]
        );
    }

    #[test]
    fn native_ddc_input_falls_back_to_shell_when_native_unavailable() {
        let config = fixture_config();
        let entries = preset_layout_entries(&config, "native_preset").expect("should resolve");

        let resolved = resolve_layout_entries(&config, &entries, false);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Ready(ResolvedCommand {
                monitor_id: "monitor3".to_string(),
                device_id: "device-a".to_string(),
                action: BackendAction::Shell("cmd-monitor3-a".to_string()),
            })]
        );
    }

    #[test]
    fn native_only_input_errors_when_native_unavailable() {
        let config = fixture_config();
        let entries = preset_layout_entries(&config, "native_only_preset").expect("should resolve");

        let resolved = resolve_layout_entries(&config, &entries, false);

        assert_eq!(
            resolved,
            vec![ResolvedEntry::Failed {
                monitor_id: "monitor3".to_string(),
                device_id: "device-b".to_string(),
                error: ResolutionError::NoBackendAvailable {
                    monitor_id: "monitor3".to_string(),
                    device_id: "device-b".to_string(),
                },
            }]
        );
    }
}
