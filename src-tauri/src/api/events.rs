use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const MAX_EVENTS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplySource {
    Api,
    Tray,
    Hotkey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskMuxEvent {
    /// Milliseconds since Unix epoch.
    pub timestamp_ms: u64,
    pub kind: EventKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ApplySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct EventLog {
    events: VecDeque<DeskMuxEvent>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: DeskMuxEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Newest events first, capped at `limit`.
    pub fn recent(&self, limit: usize) -> Vec<DeskMuxEvent> {
        let take = limit.min(self.events.len());
        self.events.iter().rev().take(take).cloned().collect()
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn record_config_loaded(events: &Mutex<EventLog>, device_name: &str) {
    let mut log = events.lock().expect("event log lock poisoned");
    log.push(DeskMuxEvent {
        timestamp_ms: now_ms(),
        kind: EventKind::Success,
        message: format!("Config loaded for device '{device_name}'"),
        preset: None,
        source: None,
        monitor_id: None,
    });
}

pub fn record_config_error(events: &Mutex<EventLog>, detail: &str) {
    let mut log = events.lock().expect("event log lock poisoned");
    log.push(DeskMuxEvent {
        timestamp_ms: now_ms(),
        kind: EventKind::Error,
        message: format!("Config failed to load: {detail}"),
        preset: None,
        source: None,
        monitor_id: None,
    });
}

pub fn record_apply_started(
    events: &Mutex<EventLog>,
    preset: &str,
    dry_run: bool,
    source: ApplySource,
) {
    let mut log = events.lock().expect("event log lock poisoned");
    let mode = if dry_run { "dry-run" } else { "apply" };
    log.push(DeskMuxEvent {
        timestamp_ms: now_ms(),
        kind: EventKind::Info,
        message: format!("Preset '{preset}' {mode} started"),
        preset: Some(preset.to_string()),
        source: Some(source),
        monitor_id: None,
    });
}

pub fn record_apply_finished(
    events: &Mutex<EventLog>,
    preset: &str,
    dry_run: bool,
    source: ApplySource,
    full_success: bool,
    failed_monitors: &[String],
) {
    let mut log = events.lock().expect("event log lock poisoned");
    let kind = if full_success {
        EventKind::Success
    } else {
        EventKind::Error
    };
    let mode = if dry_run { "dry-run" } else { "apply" };
    let message = if full_success {
        format!("Preset '{preset}' {mode} completed successfully")
    } else if failed_monitors.is_empty() {
        format!("Preset '{preset}' {mode} completed with errors")
    } else {
        format!(
            "Preset '{preset}' {mode} failed on monitor(s): {}",
            failed_monitors.join(", ")
        )
    };
    log.push(DeskMuxEvent {
        timestamp_ms: now_ms(),
        kind,
        message,
        preset: Some(preset.to_string()),
        source: Some(source),
        monitor_id: None,
    });
}

pub fn record_native_ddc_result(
    events: &Mutex<EventLog>,
    monitor_id: &str,
    success: bool,
    preset: Option<&str>,
    source: Option<ApplySource>,
) {
    let mut log = events.lock().expect("event log lock poisoned");
    let (kind, verb) = if success {
        (EventKind::Success, "succeeded")
    } else {
        (EventKind::Error, "failed")
    };
    log.push(DeskMuxEvent {
        timestamp_ms: now_ms(),
        kind,
        message: format!("Native DDC input switch {verb} on monitor '{monitor_id}'"),
        preset: preset.map(str::to_string),
        source,
        monitor_id: Some(monitor_id.to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_drops_oldest_when_full() {
        let mut log = EventLog::new();
        for i in 0..MAX_EVENTS + 5 {
            log.push(DeskMuxEvent {
                timestamp_ms: i as u64,
                kind: EventKind::Info,
                message: format!("event {i}"),
                preset: None,
                source: None,
                monitor_id: None,
            });
        }
        assert_eq!(log.events.len(), MAX_EVENTS);
        assert_eq!(log.events.front().unwrap().message, "event 5");
        assert_eq!(log.events.back().unwrap().message, "event 54");
    }

    #[test]
    fn recent_returns_newest_first() {
        let mut log = EventLog::new();
        for i in 0..3 {
            log.push(DeskMuxEvent {
                timestamp_ms: i,
                kind: EventKind::Info,
                message: format!("e{i}"),
                preset: None,
                source: None,
                monitor_id: None,
            });
        }
        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "e2");
        assert_eq!(recent[1].message, "e1");
    }
}
