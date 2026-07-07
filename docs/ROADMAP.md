# Roadmap

DeskMux is built in phases so that every phase is a genuinely usable tool, not a half-finished everything. The ordering is deliberate: ship the part that works reliably today, make it standalone, and only then take on coordinated keyboard/mouse handoff.

**Long-term vision:** DeskMux is intended to become an **all-in-one desktop control suite** — native monitor control, coordinated keyboard/mouse handoff, and smart input switching — not a permanent wrapper around ControlMyMonitor, ddcutil, BetterDisplay, or Lunar.

## Phase 1 — Monitor switching foundation

Core switching, API, coordination, and dashboard apply/status are done. Config editing in the UI and richer logs remain.

- [x] Config data model (machines, monitors, inputs, presets — all dynamic, any count)
- [x] Config loader with validation and human-readable errors
- [x] Preset executor: resolve a preset's layout to per-monitor commands, run them, capture stdout/stderr
- [x] Dry-run mode: resolve and report commands without executing
- [x] Local HTTP API (`/health`, `/status`, `POST /apply-preset`)
- [x] LAN peer coordination (`controlledBy`, coordinated apply, peer fan-out)
- [x] Dashboard UI: load status, apply presets (dry-run on by default), structured apply results
- [x] System tray + global hotkeys (apply presets from tray menu or configured shortcuts)
- [ ] Dashboard UI: add / remove / reorder machines and monitors; edit presets in UI
- [x] Live logs / richer execution history in the dashboard (`GET /events`, 50-entry ring buffer, Recent Events card)

Phase 1 uses external monitor-control tools (ControlMyMonitor, ddcutil, BetterDisplay, Lunar, and similar) as **temporary backend adapters** via configured shell commands, so DeskMux is useful immediately. **This is not the end goal.** The long-term goal is for DeskMux to provide native monitor control directly and become an all-in-one suite rather than a wrapper.

## Phase 2 — Native monitor control (make it standalone)

Phase 2 replaces the *requirement* for external monitor tools on supported displays. Nothing here is implemented yet.

- [ ] **Native DDC/CI monitor control built into DeskMux.** Talk to monitors directly from Rust so supported monitors no longer need ControlMyMonitor, ddcutil, BetterDisplay, or Lunar installed.
  - **Crate choice (checked 2026-07):** prefer calling `windows-rs` (`windows::Win32::Devices::Display` — `GetPhysicalMonitorsFromHMONITOR`, `GetVCPFeatureAndVCPFeatureReply`, `SetVCPFeature`) directly rather than adding `ddc-hi`/`ddc-winapi`. `windows` is already a transitive dependency via Tauri and is actively maintained; `ddc-hi` (last published 2021) and `ddc-winapi` (still alpha as of mid-2024) are stale for the platform we'd ship first. Re-check freshness before implementing, but don't default back to the `ddc` family without a reason.
  - Keep the shell-command backend as an **optional fallback / escape hatch** for quirky displays (Apple Silicon HDMI ports that are DDC-blind, some LG panels, laptop panels, etc.).
- [ ] **Monitor controls beyond input switching** — for monitors that expose the relevant VCP codes:
  - input source (already the Phase 1 focus)
  - brightness
  - contrast
  - volume / audio output (where supported)
  - power / standby (where supported)
  - other DDC/CI VCP controls the display actually exposes
  - Not every control is available on every monitor; DeskMux will surface what the hardware supports.
- [ ] **Native DDC discovery and resilience** (follow-ups from real-hardware validation):
  - ~~In-app read of the current VCP `0x60` input-source value per display.~~ Done — `GET /native-ddc/displays[/{id}/input-source]` + dashboard "Monitor discovery" card.
  - Surface supported input-source values where the monitor exposes them via DDC (capabilities-string parsing; the VCP reply's `maximum` is a single number, not a list).
  - ~~Retry/refresh strategy for intermittent `GetVCPFeatureAndVCPFeatureReply` failures — possible stale physical-monitor handles after hotplug.~~ Done — discovery reads retry once with refreshed enumeration.
  - Onboarding flow so users can discover `displayId` and `inputSourceValue` without a separate diagnostic session (see [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md)).
  - Technical design: [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md)
- [ ] Peer auto-discovery on the LAN (mDNS/similar), so peers don't have to be hand-configured with IPs

## Phase 3 — Keyboard, mouse, and smart handoff

Keyboard and mouse sharing across machines is a **first-class future goal** for DeskMux — not an afterthought bolted onto preset switching. None of this is implemented yet.

- [ ] **Built-in keyboard/mouse sharing and focus handoff.** Seamless pointer-driven handoff between machines over a low-latency LAN transport (prefer wired Ethernet; support WiFi where needed).
  - Example: Windows on the left screen, Mac on the right — when the pointer leaves the Windows region and enters the Mac region, DeskMux shifts keyboard/mouse focus to the Mac and can coordinate the matching monitor-input preset so the physical desk follows the user automatically.
  - The keyboard and mouse follow the active machine; monitor inputs and peripheral ownership move together.
  - Eventually: built-in input capture and injection where feasible on each OS (not implemented yet; macOS login-screen injection is not possible).
  - Software sharing is not zero-latency; true physical USB handoff still requires a hardware switch (see below).
- [ ] **Smart focus/input handoff.** Presets and focus changes triggered by pointer crossing display regions, hotkeys, tray actions, or explicit focus changes — so monitor inputs and keyboard/mouse ownership stay in sync.
- [ ] **Deskflow / Synergy-style integration as an optional bridge.** Integrating [Deskflow](https://github.com/deskflow/deskflow) (or similar) may be an early path or fallback while native DeskMux input sharing matures. The long-term vision is built-in DeskMux control, not permanent dependence on a third-party sharing daemon.
- [ ] **Latency-optimized transport.** Lean LAN protocol — UDP with small packets, minimal serialization, no cloud relay — as the place DeskMux can realistically improve on incumbents.
- [ ] **Hardware USB-switch integration.** For true, zero-latency handoff of the *physical* peripherals, drive a hardware USB switch and pair its toggle with a monitor preset. This is the only way to move the actual USB device between machines (see below).

## Explicitly not planned

- **A direct USB-cable link between the two computers.** USB is host-to-device: both computers are hosts, so a plain USB or USB-C cable between them cannot carry peripheral sharing — there's no device end and the link won't function. This isn't a matter of clever software; the protocol doesn't allow it. Peripheral sharing goes over the network (Phase 3) or through a hardware switch. A USB "bridge/transfer" cable (with a chip presenting as a device to both hosts) exists for file transfer but is not a path for low-latency keyboard/mouse sharing.
