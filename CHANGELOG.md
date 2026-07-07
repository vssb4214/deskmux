# Changelog

All notable changes to DeskMux are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- **Native DDC probe (test-switch) write moved from HTTP to a Tauri IPC command.** `POST /native-ddc/displays/{id}/probe-input` shipped in 0.2.0 as a plain endpoint on the loopback API — any local process able to reach the port could trigger a monitor write. It's removed; the only way to test-switch an input now is the `probe_input` Tauri command, invokable solely from the bundled desktop webview, matching the design already used for config save.
- **Probe writes are now gated to previously observed values**, not bounded only by the `u16` type. Real-hardware readings on this project's own validated monitors showed the VCP reply's `maximum` field mirrors `current` rather than reporting a stable ceiling, making a numeric bound close to meaningless — a probe can now only replay a value a read has already returned as that exact display's current input this session, enforced server-side regardless of what calls the command.
- **Added revert-on-timeout to the setup checklist's "Test this input" control.** A test switch auto-reverts to the pre-probe value after a short countdown unless explicitly confirmed, so a bad or unexpected switch can't strand a display with no signal and no way back short of the monitor's own physical buttons.

## [0.1.0] - 2026-07-05

First tagged release. DeskMux is config-driven monitor preset switching for multi-machine, multi-monitor desks — this version covers the full Phase 1 foundation plus the first Phase 2 milestone (native Windows DDC).

### Added

- **Config system** — a dynamic data model (any number of devices, monitors, presets), a JSON loader with human-readable validation errors that report every problem at once, monitor ownership (`controlledBy`) for coordinated multi-machine setups, and optional global hotkeys.
- **Preset executor with dry-run** — resolves a preset's layout to per-monitor commands and runs them sequentially. Dry-run reports exactly what would execute without touching hardware. A failure on one monitor is a structured per-monitor result, not a single opaque error that aborts the rest.
- **Local HTTP API + LAN peer coordination** — `GET /health`, `GET /status`, `POST /apply-preset`. A coordinator machine fans a preset apply out to peers over HTTP; each peer only touches the monitors it owns.
- **Dashboard** — a minimal web UI to view status and apply presets, with dry-run on by default. Config load/validation failures surface as readable banners instead of silent or opaque failures.
- **System tray + global hotkeys** — apply presets from a tray menu or a configured keyboard shortcut on desktop platforms.
- **Monitor-control backend abstraction** — an internal `Backend` trait separates *deciding* what a preset should do from *how* it gets done. The shell-command approach (shelling out to tools like ControlMyMonitor, ddcutil, BetterDisplay, or Lunar) is the first implementation behind that trait, not a special case baked into the executor.
- **Native Windows DDC/CI input switching** — a second backend that talks to compatible monitors directly through the Windows Monitor Configuration API, so those displays no longer need an external tool for input switching. Falls back to the shell-command backend automatically when native control isn't available on the current platform or for a given input.

### Scope and limitations

- Native DDC/CI is **Windows-only** in this release. macOS and Linux, and any Windows display that doesn't cooperate with native control, use the shell-command backend — that path is a permanent part of the design, not a shim being phased out.
- Native display identity is derived from EDID manufacturer/product data, not a true per-unit serial number — two identical monitor models on the same machine may not be reliably distinguished yet.
- The local API has no authentication. LAN access is an explicit opt-in intended for trusted networks only.
- Keyboard/mouse sharing and smart focus handoff are not implemented in this release — they remain roadmap items (see `docs/ROADMAP.md`).
- This is an MVP. Expect rough edges.
