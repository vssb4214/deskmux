# First-run setup experience — plan

**Status:** planned (not implemented). Goal: someone new can go from zero to a working `deskmux.config.json` without hand-editing JSON or running a diagnostic build.

## Current pain

Today a new user must:

1. Copy `deskmux.config.example.json` → `deskmux.config.json`
2. Replace placeholder shell commands or native DDC fields
3. Guess or externally discover `displayId` and `inputSourceValue` (see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md))
4. Restart DeskMux and hope validation passes

That is too much for “install and use.”

## Target experience (v1 wizard)

A **first-run flow** inside DeskMux (dashboard or dedicated window) that runs once when no valid config exists, or when the user chooses **Setup wizard** from the tray.

### Step 1 — Welcome and machine identity

- Prompt: “What is this computer called in your desk setup?”
- Maps to `deviceName` and a matching `devices[]` entry.
- Default API port `3737`, loopback-only unless user opts into LAN access (with short security note).

### Step 2 — Detect monitors (Windows native path)

- Call discovery API: list `displayId` values for all DDC-capable displays (reuse `list_native_display_ids()` / future `GET /native-ddc/displays`).
- Show a checklist of detected displays with friendly labels (“Display 1 — K@P:d0e5:0”).
- User selects which physical monitors DeskMux should manage on **this** machine.
- For monitors controlled by another PC, offer “skip — configured on peer” (remote stub, no `inputs` locally).

### Step 3 — Name computers and inputs

- User lists all machines at the desk (Windows PC, Mac mini, etc.) → `devices[]`.
- For each selected monitor, user maps which devices can connect to it (HDMI, DisplayPort, USB-C labels are cosmetic `type` fields).
- Optional: add `peers[]` entries for machines that run their own DeskMux instance.

### Step 4 — Discover input source values (native DDC)

Per monitor × device pair where native DDC is desired:

1. Show: “Switch this monitor to **{device label}** using its physical input button.”
2. User confirms → DeskMux reads VCP `0x60` current value via discovery API.
3. Store as `inputSourceValue` (u16 — may be > 255).
4. Repeat for each input.

For monitors without native DDC support, fall back to:

- “Paste or pick a shell command” (ControlMyMonitor, ddcutil, etc.) — same as today.

Show a clear warning when read succeeds but probe-write fails (monitor may not support DDC input switching).

### Step 5 — Build presets

- Simple layout builder: for each preset name, pick which device each monitor should show.
- Seed common presets: “All {this machine}”, “Split left/right” if two monitors.

### Step 6 — Review and generate config

- Show a read-only summary (no shell commands in event log style — redact if needed).
- Write `deskmux.config.json` next to the executable / project root (same path as today).
- Offer **Download JSON** as backup before save.

### Step 7 — Safe test apply

- Run coordinated **dry-run** preset through existing API.
- Show resolved actions (native DDC shows display + value, not raw shell strings in UI if sensitive).
- Single **Test on this monitor** button with explicit confirmation — one real apply, not a loop.
- Success → “Setup complete”; failure → link to recent events + CONFIG.md troubleshooting.

## Technical dependencies

| Capability | Status |
|------------|--------|
| Config load/validate | Done |
| Native DDC apply (u16) | Done |
| Event history API | Done |
| Discovery read API | Done — `GET /native-ddc/displays`, `GET /native-ddc/displays/{id}/input-source` (see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md)) |
| Discovery dashboard panel (read-only) | Done — "Monitor discovery" card |
| Config draft validate/save (Tauri IPC) | Done — minimal "Config draft" dashboard card; full wizard still planned |
| Dashboard wizard UI | Not started |

## Config write safety

- Atomic write: `deskmux.config.json.tmp` → rename to `deskmux.config.json`.
- Validate before save; show errors inline in the dashboard card.
- If a config file already exists, copy to `deskmux.config.json.bak` before overwrite (abort if backup fails).
- Desktop-only Tauri IPC (`validate_config_draft`, `save_config_draft`) — no HTTP validate or write endpoints.
- Restart DeskMux after save; no hot-reload in the current implementation.
- Do not commit generated config to git.

## Out of scope for v1 wizard

- Full config editor (add/remove monitors later) — separate Phase 1 dashboard item in [ROADMAP.md](./ROADMAP.md)
- macOS native DDC discovery
- Peer auto-discovery (mDNS)
- Import from third-party tools

## Suggested implementation order

1. ~~Discovery HTTP API (read-only)~~ — done, with a read-only dashboard discovery panel
2. `POST /config/validate` — validate a draft config without saving
3. Wizard shell in dashboard (stepper UI, vanilla JS)
4. File save via Tauri IPC command (not an HTTP endpoint — see Config write safety)
5. Tray entry “Run setup wizard” when config missing

## Success criteria

A new user on Windows with one DDC-capable monitor can:

- Install DeskMux
- Complete the wizard in < 10 minutes
- Dry-run a preset and see correct native DDC values
- Apply once and observe the monitor switch

Without editing JSON by hand or running diagnostic builds.
