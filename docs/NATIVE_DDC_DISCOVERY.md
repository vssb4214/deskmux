# Native DDC input discovery — technical design

**Status:** read path and setup probe-write are implemented — display enumeration, VCP 0x60 reads,
and setup-time test writes are live via
`GET /native-ddc/displays` and `GET /native-ddc/displays/{displayId}/input-source`, surfaced in
the dashboard's "Monitor discovery" card, plus `POST /native-ddc/displays/{displayId}/probe-input`.
Capabilities-string parsing and the onboarding wizard are still pending. This
document captures what real-hardware validation proved and how DeskMux exposes discovery
without a separate diagnostic session.

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
| `probe_input_write(display_id, value)` | display ID + candidate value | `{ accepted: bool, rawReturnCode }` — **manual / explicit only**, not called from preset apply |

`probe_input_write` is for onboarding “test this input” buttons, gated behind user confirmation and dry-run style UX — never silent.

### 3. HTTP API

```
GET /native-ddc/displays
GET /native-ddc/displays/{displayId}/input-source
POST /native-ddc/displays/{displayId}/probe-input   { "value": 4626 }  // explicit test only
```

Responses are JSON, camelCase, no shell commands, no secrets.

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

- Discovery endpoints bind to loopback by default (same as existing API).
- Never log or return shell `command` strings in discovery responses.
- Probe-write requires explicit user action; rate-limit in UI (one probe per button click).
- No automatic probe on startup.

## Testing strategy

| Layer | Approach |
|-------|----------|
| Unit | Mock `NativeDdcController` returns canned `(current, max)` |
| Resolver | Unchanged — still maps config `inputSourceValue: u16` → `BackendAction` |
| Integration | HTTP handler tests with mock discovery seam |
| Hardware | Manual checklist on Windows with at least one DDC-capable external panel |

## Implementation order

1. ~~`get_vcp_feature` on trait + Windows impl + retry refresh.~~ Done.
2. ~~`GET /native-ddc/displays` and `GET .../input-source`.~~ Done.
3. ~~Dashboard “Discovery” panel (read-only).~~ Done ("Monitor discovery" card).
4. ~~Probe-write endpoint for explicit setup-time test switches.~~ Done — `POST /native-ddc/displays/{id}/probe-input`.
5. First-run wizard consumes discovery API (see [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md)).

## Related docs

- [CONFIG.md](./CONFIG.md) — `nativeDdc` schema
- [ROADMAP.md](./ROADMAP.md) — Phase 2 native control
- [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md) — end-user onboarding plan
