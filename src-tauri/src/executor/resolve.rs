use std::collections::HashMap;

use super::model::{ExecutorError, ResolutionError};
use crate::config::Config;

pub(super) struct ResolvedCommand {
    pub monitor_id: String,
    pub device_id: String,
    pub command: String,
}

pub(super) enum ResolvedEntry {
    Ready(ResolvedCommand),
    Failed {
        monitor_id: String,
        device_id: String,
        error: ResolutionError,
    },
}

/// Looks up `preset_name` and resolves every monitorId -> deviceId entry in its layout to a
/// command. Entries are ordered by the monitor's `order` field (unresolvable monitors sort last)
/// so execution is deterministic. Pure: no I/O, no process spawning.
pub(super) fn resolve_preset(
    config: &Config,
    preset_name: &str,
) -> Result<Vec<ResolvedEntry>, ExecutorError> {
    let preset = config
        .presets
        .get(preset_name)
        .ok_or_else(|| ExecutorError::PresetNotFound {
            preset_name: preset_name.to_string(),
        })?;

    let monitors_by_id = config
        .monitors
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect::<HashMap<_, _>>();

    let mut entries: Vec<(&String, &String)> = preset.layout.iter().collect();
    entries.sort_by(|(a_id, _), (b_id, _)| {
        let a_order = monitors_by_id
            .get(a_id.as_str())
            .map_or(u32::MAX, |m| m.order);
        let b_order = monitors_by_id
            .get(b_id.as_str())
            .map_or(u32::MAX, |m| m.order);
        a_order.cmp(&b_order).then_with(|| a_id.cmp(b_id))
    });

    let resolved = entries
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
        .collect();

    Ok(resolved)
}
