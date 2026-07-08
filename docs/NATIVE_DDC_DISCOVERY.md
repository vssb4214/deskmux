# Native DDC input discovery — technical design

**Status:** read path and setup probe-write are implemented — display enumeration and VCP 0x60
reads are live via `GET /native-ddc/displays` and `GET /native-ddc/displays/{displayId}/input-source`,
surfaced in the dashboard's "Monitor discovery" card and the guided setup checklist's "Test this
input" control. The probe (test-switch) write is **not** an HTTP endpoint — it's the Tauri
`probe_input` IPC command, invokable only from the bundled webview, and it only accepts a value
that a read has already returned as that exact display's current input this session (enforced
server-side, not just by the UI). See "Security and safety" below for why. Capabilities-string
parsing and the full onboarding wizard are still pending. This document captures what
real-hardware validation proved and how DeskMux exposes discovery without a separate diagnostic
session.

## Live controls

DeskMux also exposes live native DDC controls for brightness (`0x10`), contrast (`0x12`), and
volume (`0x62`) on displays that support them. Reads are available over HTTP:

```text
GET /native-ddc/displays/{displayId}/controls
```

The response reports each control independently. A monitor may support brightness but not volume;
unsupported controls are shown as unavailable rather than treated as a whole-request failure when
possible. These controls are live adjustments only in this PR: they are not config fields, preset
steps, or unattended automation.

Writes are desktop-app-only through the Tauri `set_native_ddc_control` IPC command. The command
accepts only named features (`brightness`, `contrast`, `volume`), reads the feature first, treats
the monitor-reported maximum as the semantic ceiling for these continuous controls, and rejects
values outside `0..=maximum`. It does not accept arbitrary VCP codes.

Power mode (`0xD6`) is intentionally not included. A DDC power-off can leave a monitor in a state
that may not wake over DDC, so power needs a separate guarded design with stronger confirmation
and honest wake-back copy.

## Problem

Native DDC input switching works on supported Windows displays, but users must currently supply two opaque values per monitor:

| Field | Example | How it was found (today) |
|-------|---------|--------------------------|
| `monitors[].nativeDdc.displayId` | `K@P:d0e5:0` | Startup log from `list_native_display_ids()` |
| `inputs.<device>.nativeDdc.inputSourceValue` | `4626` (desktop), `4623` (MacBook) | Temporary VCP 0x60 read/write diagnostic |

Values like `4626` exceed `u8` and are monitor-specific — they cannot be guessed from input type names (`hdmi`, `displayport`).

## Goals

1. **Read** the current VCP `0x60` input-source value for a selected display.
2. **List supported values** when the monitor exposes them via DDC (not all do).
3. **Classify write capability** — some monitors read fine but reject `SetVCPFeature` for input select.
4. **Stay stable across hotplug** — refresh physical-monitor handles when reads fail intermittently.
5. **Feed first-run onboarding** — discovery output becomes structured data the setup wizard can write into `deskmux.config.json`.

Non-goals for the first implementation: brightness/contrast, arbitrary VCP codes, macOS backend.

## Real-hardware observations

- **Write path:** `SetVCPFeature(handle, 0x60, value)` with `value: u16` successfully switched a real panel between inputs when given the correct codes.
- **Read path:** `GetVCPFeatureAndVCPFeatureReply` returns `(current, maximum)` for VCP `0x60`; current value changes when the user switches input via the monitor's physical menu.
- **Intermittent reads:** Some displays (e.g. `KJL:0e25:2`) occasionally fail `GetVCPFeatureAndVCPFeatureReply` until handles are re-enumerated — likely stale `HMONITOR` / physical-monitor handles after sleep, hotplug, or driver refresh.
- **Read-but-no-write:** Some panels may report current input but ignore or reject input-select writes; DeskMux must surface this as a failed apply, not fall back to shell silently.

## Proposed architecture

### 1. Extend the native seam (Windows)

Add read capability to `NativeDdcController` (behind the existing mockable trait):

```rust
fn get_vcp_feature(&self, display_id: &str, vcp_code: u8) -> io::Result<(u32, u32)>;
// Returns (current_value, maximum_value)
```

Implementation reuses the enumeration + `GetVCPFeatureAndVCPFeatureReply` path already proven in diagnostics, but lives in `windows_ddc.rs` as production API — not startup scratch code.

**Retry policy (handle refresh):**

1. Attempt read on cached enumeration match.
2. On failure, re-run `list_displays()` + physical-monitor open (full refresh).
3. Retry once; if still failing, return structured error (`displayNotFound`, `vcpReadFailed`, `staleHandle`).

Do not retry writes automatically — repeated input switching while the user is away is undesirable.

### 2. Discovery service (Rust, no UI yet)

New module `executor::discovery` (or `api::discovery`) with pure functions:

| Function | Input | Output |
|----------|-------|--------|
| `list_displays()` | — | `Vec<{ displayId, label? }>` (wraps existing enumeration) |
| `read_input_source(display_id)` | display ID | `{ current, maximum }` |
| `probe_input(display_id, value)` | display ID + a value previously returned by a read | `{ accepted: bool, current: Option<u32> }` — never called from preset apply |

`probe_input` is for the setup checklist's "Test this input" control, gated by the observed-value
check below and by the dashboard's revert-on-timeout UX — never silent, never a value the user
just typed.

### 3. HTTP API (reads only)

```
GET /native-ddc/displays
GET /native-ddc/displays/{displayId}/input-source
```

Responses are JSON, camelCase, no shell commands, no secrets. **The probe write is deliberately
not here** — see "Security and safety."

### 4. Error taxonomy

| Code | Meaning | User-facing hint |
|------|---------|------------------|
| `displayNotFound` | `displayId` not in current enumeration | Re-plug monitor; check startup log |
| `vcpReadFailed` | DDC read returned failure | Monitor may not support DDC on this port |
| `vcpWriteFailed` | SetVCPFeature failed | Try shell fallback; some panels read but reject input writes |
| `staleHandle` | Read failed, succeeded after refresh | Transient; retry once in UI |

### 5. Onboarding integration (later)

Discovery API feeds a first-run wizard (see `docs/FIRST_RUN_SETUP.md`):

1. Enumerate displays → user picks physical monitors.
2. For each monitor, read current VCP `0x60` while user switches inputs manually → capture value per machine.
3. Optional: show supported range (`maximum`) when useful.
4. User names devices/inputs → generate `deskmux.config.json`.
5. Dry-run preset → user confirms → optional single real apply per monitor.

## Security and safety

- Discovery **read** endpoints bind to loopback by default (same as existing API) — any local
  process can enumerate displays and read the current input, which is non-destructive.
- The probe **write** is IPC-only (`probe_input` Tauri command), not HTTP. Any local process can
  reach the loopback API, but only the bundled webview can invoke a Tauri command — the same
  reasoning already applied to config-save. Testing a probe also only makes sense when you're
  physically watching the monitor, which the IPC restriction matches correctly rather than
  incidentally.
- **Value bounding is observed-values-only, not a numeric range.** Real hardware readings
  (`current=4626, maximum=4626` and `current=3, maximum=3` on this project's own validated
  monitors) show `maximum` mirrors `current` rather than reporting a stable ceiling — a
  `value ≤ maximum` check would be close to meaningless on this hardware. Instead, `probe_input`
  only accepts a `value` that a read has already returned as the exact `current` value for that
  `display_id` this session (`AppState.observed_input_values`, shared between the HTTP read
  handler and the IPC probe command). This is enforced inside the command itself, not just by
  what the UI offers, so it can't be bypassed by invoking the command directly with an untested
  value. There is deliberately no escape hatch for "a value I already know from documentation" —
  physically switching the monitor to it once and reading makes it observed, which is the normal
  flow one step earlier.
- Never log or return shell `command` strings in discovery responses.
- The dashboard's "Test this input" control auto-reverts to the pre-probe value after a short
  countdown unless the user confirms the change is correct — see `src/lib/probe.js`. This is
  what actually prevents a stranded no-signal display; the exposure and value-bounding rules
  above only make the write itself safer, not recoverable on their own.
- No automatic probe on startup, and probe is never called from preset apply.

## Testing strategy

| Layer | Approach |
|-------|----------|
| Unit | Mock `NativeDdcController` returns canned `(current, max)` |
| Resolver | Unchanged — still maps config `inputSourceValue: u16` → `BackendAction` |
| Integration | HTTP handler tests (reads) and direct `probe_input_gated` tests (probe) with mock discovery seam |
| Frontend | `startProbeRevertTimer`'s confirm/revert/timeout state machine is unit-tested with a fake scheduler — no real timers |
| Hardware | Manual checklist on Windows with at least one DDC-capable external panel |

## Implementation order

1. ~~`get_vcp_feature` on trait + Windows impl + retry refresh.~~ Done.
2. ~~`GET /native-ddc/displays` and `GET .../input-source`.~~ Done.
3. ~~Dashboard “Discovery” panel (read-only).~~ Done ("Monitor discovery" card).
4. ~~Probe-write for explicit setup-time test switches.~~ Done — Tauri `probe_input` IPC command, observed-values-gated, with revert-on-timeout in the setup checklist.
5. First-run wizard consumes discovery API (see [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md)).

## Related docs

- [CONFIG.md](./CONFIG.md) — `nativeDdc` schema
- [ROADMAP.md](./ROADMAP.md) — Phase 2 native control
- [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md) — end-user onboarding plan
