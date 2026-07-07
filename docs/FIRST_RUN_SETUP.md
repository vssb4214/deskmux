# First-run setup experience

**Status:** guided checklist shipped in the dashboard; full stepper wizard and tray entry still planned.

Goal: someone new can go from zero to a working `deskmux.config.json` without hand-editing JSON.

## Using the dashboard checklist today

When DeskMux starts without a valid config, open the desktop app and follow **Set up DeskMux** (expanded automatically). After configuration loads, setup tools collapse behind **Run setup again**.

1. **Name this computer** — readable label and derived config id (for example “Gaming PC” → `gaming_pc`).
2. **Detect monitors** — uses `GET /native-ddc/displays` on Windows; name each display (for example “Left monitor”).
3. **Capture input values** — switch each monitor physically, click **Read current input**, optionally label the capture (for example “Desktop”).
4. **Name your preset** — defaults to “All {computer name}”; generates a readable preset id (for example `all_gaming_pc`).
5. **Generate config draft** — builds starter JSON using your names, not generic `monitor1` placeholders.
6. **Validate and save** — open **Advanced options**, review JSON if needed, then validate/save via desktop IPC.
7. **Restart DeskMux** — quit and reopen so the new config loads.
8. **Test a preset** — apply with dry-run first.

Generated configs use your labels for `devices[]`, `monitors[]`, and `presets`. Manual JSON editing remains available under **Advanced options**.

Restart is required after save; there is no hot reload.

## Current pain (manual path)

Today a new user must:

1. Copy `deskmux.config.example.json` → `deskmux.config.json`
2. Replace placeholder shell commands or native DDC fields
3. Guess or externally discover `displayId` and `inputSourceValue` (see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md))
4. Restart DeskMux and hope validation passes

That is too much for "install and use."

## Target experience (v1 wizard)

A **first-run flow** inside DeskMux (dashboard or dedicated window) that runs once when no valid config exists, or when the user chooses **Setup wizard** from the tray.

### Step 1 — Welcome and machine identity

- Prompt: "What is this computer called in your desk setup?"
- Maps to `deviceName` and a matching `devices[]` entry.
- Default API port `3737`, loopback-only unless user opts into LAN access (with short security note).

### Step 2 — Detect monitors (Windows native path)

- Call discovery API: list `displayId` values for all DDC-capable displays (reuse `list_native_display_ids()` / future `GET /native-ddc/displays`).
- Show a checklist of detected displays with friendly labels ("Display 1 — K@P:d0e5:0").
- User selects which physical monitors DeskMux should manage on **this** machine.
- For monitors controlled by another PC, offer "skip — configured on peer" (remote stub, no `inputs` locally).

### Step 3 — Name computers and inputs

- User lists all machines at the desk (Windows PC, Mac mini, etc.) → `devices[]`.
- For each selected monitor, user maps which devices can connect to it (HDMI, DisplayPort, USB-C labels are cosmetic `type` fields).
- Optional: add `peers[]` entries for machines that run their own DeskMux instance.

### Step 4 — Discover input source values (native DDC)

Per monitor × device pair where native DDC is desired:

1. Show: "Switch this monitor to **{device label}** using its physical input button."
2. User confirms → DeskMux reads VCP `0x60` current value via discovery API.
3. Store as `inputSourceValue` (u16 — may be > 255).
4. Repeat for each input.

Optional setup-time test switch (no saved config required):

- `POST /native-ddc/displays/{displayId}/probe-input` with `{ "value": <u16> }`
- Performs one explicit native DDC VCP `0x60` write attempt.
- Response reports write acceptance; optional read-back may include `current`.

For monitors without native DDC support, fall back to:

- "Paste or pick a shell command" (ControlMyMonitor, ddcutil, etc.) — same as today.

Show a clear warning when read succeeds but probe-write fails (monitor may not support DDC input switching).

### Step 5 — Build presets

- Simple layout builder: for each preset name, pick which device each monitor should show.
- Seed common presets: "All {this machine}", "Split left/right" if two monitors.

### Step 6 — Review and generate config

- Show a read-only summary (no shell commands in event log style — redact if needed).
- Write `deskmux.config.json` next to the executable / project root (same path as today).
- Offer **Download JSON** as backup before save.

### Step 7 — Safe test apply

- Run coordinated **dry-run** preset through existing API.
- Show resolved actions (native DDC shows display + value, not raw shell strings in UI if sensitive).
- Single **Test on this monitor** button with explicit confirmation — one real apply, not a loop.
- Success → "Setup complete"; failure → link to recent events + CONFIG.md troubleshooting.

## Technical dependencies

| Capability | Status |
|------------|--------|
| Config load/validate | Done |
| Native DDC apply (u16) | Done |
| Event history API | Done |
| Discovery read API | Done — `GET /native-ddc/displays`, `GET /native-ddc/displays/{id}/input-source` (see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md)) |
| Discovery probe-write API | Done — `POST /native-ddc/displays/{id}/probe-input` (setup-time VCP `0x60` test switch; one write attempt per request) |
| Discovery dashboard panel (read-only) | Done — "Monitor discovery" card |
| Config draft validate/save (Tauri IPC) | Done — minimal "Config draft" dashboard card; full wizard still planned |
| Guided setup checklist (dashboard) | Done — status bar + checklist + draft generation from captured readings |
| Dashboard wizard UI | Not started — tray "Run setup wizard" and dedicated stepper still planned |

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

1. ~~Discovery HTTP API (read + probe)~~ — done (`GET /native-ddc/displays`, `GET .../input-source`, `POST .../probe-input`)
2. ~~Config draft validate/save via Tauri IPC~~ — done, with a minimal dashboard "Config draft" card
3. ~~Guided setup checklist in dashboard~~ — done
4. Wizard shell / tray entry (stepper UI, vanilla JS)

## Success criteria

A new user on Windows with one DDC-capable monitor can:

- Install DeskMux
- Complete the wizard in < 10 minutes
- Dry-run a preset and see correct native DDC values
- Apply once and observe the monitor switch

Without editing JSON by hand or running diagnostic builds.
