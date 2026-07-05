# Roadmap

DeskMux is built in phases so that every phase is a genuinely usable tool, not a half-finished everything. The ordering is deliberate: ship the part that works reliably today, make it standalone, and only then take on the much larger peripheral-sharing problem.

## Phase 1 — Monitor switching foundation

Core switching, API, coordination, and dashboard apply/status are done. Config editing in the UI and richer logs remain.

- [x] Config data model (machines, monitors, inputs, presets — all dynamic, any count)
- [x] Config loader with validation and human-readable errors
- [x] Preset executor: resolve a preset's layout to per-monitor commands, run them, capture stdout/stderr
- [x] Dry-run mode: resolve and report commands without executing
- [x] Local HTTP API (`/health`, `/status`, `POST /apply-preset`)
- [x] LAN peer coordination (`controlledBy`, coordinated apply, peer fan-out)
- [x] Dashboard UI: load status, apply presets (dry-run on by default), structured apply results
- [ ] Dashboard UI: add / remove / reorder machines and monitors; edit presets in UI
- [ ] Live logs / richer execution history in the dashboard

This phase depends on an external monitor-control tool (ControlMyMonitor / ddcutil / BetterDisplay / Lunar) via configured shell commands.

## Phase 2 — Make it standalone

- [ ] **Native DDC/CI support.** Talk to monitors directly from Rust (e.g. the `ddc` / `ddc-hi` crates) so no external tool is required. This is the headline improvement — dependency-free monitor switching.
  - Keep the shell-command path as a per-monitor escape hatch. Some displays misbehave over native DDC (the Apple Silicon HDMI port is DDC-blind; some LG panels don't expose input select), and a raw-command fallback keeps those working.
- [ ] System tray + global hotkeys (switch presets without opening the window)
- [ ] Peer auto-discovery on the LAN (mDNS/similar), so peers don't have to be hand-configured with IPs

## Phase 3 — Peripheral sharing (a separate, major undertaking)

Sharing one keyboard and mouse across machines is not a feature bolted onto a monitor switcher — it's effectively a second product. Approached honestly:

- [ ] **Software keyboard/mouse sharing.** Control another machine's pointer/keyboard over the network (the Synergy/Deskflow model: capture input events on the active machine, send over LAN, inject on the target).
  - **Preferred first path: integrate [Deskflow](https://github.com/deskflow/deskflow)** rather than reimplement input capture/injection from scratch. DeskMux orchestrates the handoff (a preset switches monitors *and* tells the sharing layer which machine has focus). This is the realistic route to a working feature.
  - A from-scratch implementation is a **stretch goal**, not a v1. The hard part is the per-OS input layer (low-level capture/injection differs substantially between Windows and macOS, and macOS gates injection behind accessibility permissions and blocks it entirely at the login screen). This is where Deskflow has spent years.
- [ ] **Latency-optimized transport.** Where DeskMux can realistically beat incumbents is the network path:
  - Prefer wired Ethernet over WiFi (lower, more consistent latency and jitter); support both.
  - Lean LAN protocol — UDP with small packets, minimal serialization, no cloud relay.
- [ ] **Hardware USB-switch integration.** For true, zero-latency handoff of the *physical* peripherals, drive a hardware USB switch and pair its toggle with a monitor preset. This is the only way to move the actual USB device between machines (see below).

## Explicitly not planned

- **A direct USB-cable link between the two computers.** USB is host-to-device: both computers are hosts, so a plain USB or USB-C cable between them cannot carry peripheral sharing — there's no device end and the link won't function. This isn't a matter of clever software; the protocol doesn't allow it. Peripheral sharing goes over the network (Phase 3) or through a hardware switch. A USB "bridge/transfer" cable (with a chip presenting as a device to both hosts) exists for file transfer but is not a path for low-latency keyboard/mouse sharing.
