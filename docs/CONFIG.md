# Configuration

DeskMux reads `deskmux.config.json` from the project root at startup. Copy the template and edit it for your hardware:

```bash
cp deskmux.config.example.json deskmux.config.json
```

`deskmux.config.json` is gitignored — it describes *your* desk, so it stays out of the repo.

## Schema

| Field                      | Type    | Description                                                                    |
|----------------------------|---------|--------------------------------------------------------------------------------|
| `deviceName`               | string  | This machine's id (must match one of the `devices[].id`).                      |
| `apiPort`                  | number  | Port this machine's local HTTP API listens on. Default `3737`.                 |
| `apiLanAccess`             | boolean | When `true`, bind the API on all interfaces (`0.0.0.0`) so LAN peers can reach it. Default `false` (loopback only, `127.0.0.1`). **Security:** enabling LAN access lets other machines on your network trigger any preset defined in your config — only enable on trusted networks; authentication is not implemented yet. |
| `peers[]`                  | array   | Every other machine on the LAN.                                                |
| `peers[].name`             | string | Peer machine name.                                                            |
| `peers[].host`             | string | Peer LAN IP.                                                                   |
| `peers[].port`             | number | Port the peer's DeskMux API listens on (default `3737`).                       |
| `devices[]`                | array  | All machines in the setup — add or remove entries freely.                     |
| `devices[].id`             | string | Stable machine key, used as the input key on monitors and the value in layouts. |
| `devices[].label`          | string | Human-readable machine name shown in the UI.                                   |
| `monitors[]`               | array  | Your monitors — any number, add/remove/reorder.                              |
| `monitors[].id`            | string | Stable key referenced by presets.                                             |
| `monitors[].label`         | string | Human-readable name shown in the UI.                                          |
| `monitors[].order`         | number | Display order in the UI (lower = first).                                       |
| `monitors[].controlledBy`  | string | Optional. Device id that runs input-switch commands for this monitor. Defaults to this config's `deviceName` at apply/validation time (not a serde default — see below). |
| `monitors[].nativeDdc`     | object | Optional. `{ displayId }` — this monitor's identity for native DDC/CI control (Windows only for now). Required if any of this monitor's `inputs[].nativeDdc` is set. See [Native DDC/CI input switching](#native-ddcci-input-switching-nativeddc) below. |
| `monitors[].inputs`        | object | Map of **device id** → `{ type, command?, nativeDdc? }`. Required for locally owned monitors; omit for remote-owned stubs on a coordinator. |
| `inputs.<deviceId>.type`   | string | Connector type, informational (`hdmi`, `displayport`, `usb-c`).               |
| `inputs.<deviceId>.command`| string | Shell command that selects this input on this monitor. Optional if `nativeDdc` is set instead — but at least one of the two is required. |
| `inputs.<deviceId>.nativeDdc` | object | Optional. `{ inputSourceValue }` — the VCP input-source value (code `0x60`) that selects this device's input via native DDC/CI. Requires the parent monitor's `nativeDdc.displayId`. |
| `presets`                  | object | Map of preset name → `{ label, layout }`.                                    |
| `presets.<name>.layout`    | object | Map of `monitorId` → `deviceId`.                                              |
| `hotkeys`                  | object | Optional. Map of **preset name** → global shortcut string (desktop only).      |

**Key idea:** input keys are device ids, not fixed strings. Two machines or six, three monitors or one — you describe your setup and DeskMux adapts. A monitor only needs to declare the inputs it physically has; presets can only route a monitor to a machine it declares.

## Global hotkeys (`hotkeys`)

Optional map of preset name → shortcut string. When DeskMux runs on desktop (Windows/macOS/Linux), pressing a configured shortcut applies that preset for real (not dry-run), using the same orchestration path as the dashboard and API.

```json
"hotkeys": {
  "all_windows": "Ctrl+Alt+1",
  "split_win_mac": "Ctrl+Alt+2"
}
```

**Rules:**

- Each key must match a `presets` entry.
- Shortcut strings use [Tauri global-shortcut syntax](https://v2.tauri.app/plugin/global-shortcut/) — e.g. `Ctrl+Alt+1`, `CmdOrCtrl+Shift+2` (cross-platform alias for Command on macOS / Control elsewhere).
- Duplicate shortcuts across presets are rejected at validation.
- Invalid shortcut strings are rejected at validation.
- If another app already owns a shortcut at runtime, DeskMux logs a warning and skips that binding; the app still starts.
- On macOS, global shortcuts may require accessibility permission.
- When config fails to load, no hotkeys are registered.

**System tray:** DeskMux also shows a tray icon with **Show DeskMux**, preset apply items (when config is loaded), and **Quit**. Tray preset apply uses the same real coordinated apply as hotkeys.

## Monitor ownership (`controlledBy`)

Each monitor has an **owner** — the machine that runs the shell command to switch its input. That owner is `monitors[].controlledBy`, which defaults to this config's `deviceName` when omitted.

Because serde field defaults cannot read the parent config, the default is applied in validation/planning code via `Monitor::controlled_by(&config.device_name)`, not at JSON parse time.

| `controlledBy`                         | Who runs the command | Config requirements on this machine |
|----------------------------------------|----------------------|-------------------------------------|
| Omitted or equals `deviceName`         | This machine         | `inputs` required; preset layout targets must have a matching input entry |
| Another `devices[].id`                 | That peer            | Stub allowed: `id`, `label`, `order`, and `controlledBy` only — no `inputs` |

**Coordinator rule:** the coordinating machine must list every monitor it wants to switch in its own `monitors[]`, including monitors physically attached elsewhere. For remote-owned monitors, add a **stub** entry so the coordinator knows which peer owns the command:

```json
{
  "id": "monitor3",
  "label": "Mac-side Monitor",
  "order": 2,
  "controlledBy": "mac-mini"
}
```

The owning peer's config carries the real `inputs` and runs the command when the coordinator fans out a preset apply.

**Validation:**

- Locally owned monitors (`controlledBy == deviceName`): require `inputs`; each preset layout entry targeting a device must have a matching input command on that monitor.
- Remote-owned stubs (`controlledBy != deviceName`): `inputs` may be omitted; preset validation only checks that the layout's target `deviceId` exists in `devices[]`. The owning peer validates and executes its local command.
- `peers[].name` must match a `devices[].id` and must not equal `deviceName`.

## Native DDC/CI input switching (`nativeDdc`)

**Status: foundation only, input switching, Windows only.** An alternative to the shell `command` for switching a monitor's input, talking to the monitor directly over DDC/CI via the Windows Monitor Configuration API instead of shelling out to a tool. Opt-in per input — existing shell-only configs are unaffected, and shell commands remain a permanent fallback for displays that don't support this (see [Limitations](../README.md#limitations)).

```json
{
  "id": "monitor1",
  "label": "Left Monitor",
  "order": 0,
  "nativeDdc": { "displayId": "K@P:d0e5:0" },
  "inputs": {
    "windows-pc": {
      "type": "displayport",
      "command": "C:\\Tools\\ControlMyMonitor.exe /SetValue \"\\\\.\\DISPLAY1\\Monitor0\" 60 4626",
      "nativeDdc": { "inputSourceValue": 4626 }
    },
    "mac-mini": {
      "type": "hdmi",
      "command": "betterdisplaycli set --name=\"LG HDR 4K\" --inputSource=hdmi1",
      "nativeDdc": { "inputSourceValue": 4623 }
    }
  }
}
```

**`monitors[].nativeDdc.displayId`** identifies the physical monitor, derived from its EDID (manufacturer id + product code, correlated with a per-connection identifier) rather than enumeration order or the Windows device name (`\\.\DISPLAY1`) — both of those can silently reassign across reboots, sleep/wake, or just plugging monitors in a different order. DeskMux logs detected displays and their `displayId` at startup (Windows only) so you can copy the right value into your config instead of guessing.

**Known limitation:** the API DeskMux uses doesn't expose a true per-unit EDID serial number, only manufacturer + product code + a connection-derived identifier. **Two identical monitor models on the same machine can end up with the same `displayId`** and be indistinguishable to DeskMux. If you own two of the exact same monitor, native DDC input switching may not reliably tell them apart yet — use the shell `command` for those monitors instead until per-unit serial matching is added.

**`inputs.<deviceId>.nativeDdc.inputSourceValue`** is the monitor-specific VCP input-source value (code `0x60`) that selects this device's input — the same number you'd read off the monitor for a shell-based `command`, just structured instead of embedded in a command string. DeskMux doesn't guess this value any more than it guesses shell commands.

**Values are often larger than 255.** Real monitors report input-source codes like `4626` (DisplayPort) and `4623` (HDMI) — not small sequential integers. Discover yours by reading VCP `0x60` on the target display (DeskMux startup logs list `displayId` values on Windows; reading the current input value in-app is a planned follow-up). Do not copy example numbers unless you've confirmed them on your hardware.

**Some monitors do not support input switching over DDC**, or may allow reads but reject writes. DeskMux reports a failed result in those cases — the same as a shell command that exits non-zero — and never silently falls back to the shell `command`.

**This is input-source switching only.** There's deliberately no field for an arbitrary VCP code — brightness, contrast, volume, and power are separate future capabilities with their own config fields when they're built, not something you can reach through `nativeDdc` today.

**Platform behavior:** if an input sets `nativeDdc` but this build can't run it (non-Windows, for now), DeskMux falls back to that input's `command` if one is set, or reports a clear resolution error if not. A native operation that runs but fails (display not found, monitor rejects the write) is reported as a failed result, the same as a shell command that exits non-zero — it never silently falls back to the shell command.

## Peer coordination

When a coordinator calls `POST /apply-preset` without `localOnly`, DeskMux:

1. Plans the preset layout against **this** config's monitor list.
2. Runs commands for monitors where `controlledBy == deviceName`.
3. Calls each required peer's API with `{ "preset": "...", "dryRun": ..., "localOnly": true }` so peers only act on monitors they own.

**Missing monitors:**

| Mode | Behavior |
|------|----------|
| Coordinator (`localOnly: false`) | A preset layout entry whose `monitorId` is missing from this config returns a structured `planningErrors` entry (`unknownMonitor`) — not a silent skip. |
| Peer / local-only (`localOnly: true`) | Layout entries for monitors missing from this config or owned by another device are skipped. Peers may have partial configs. |

Peer HTTP calls still run during dry-run (with `dryRun: true` on the peer) so you can preview coordinated applies end-to-end without executing commands.

## Local HTTP API

DeskMux serves a small HTTP API on this machine (default `http://127.0.0.1:3737`):

- `GET /health` — liveness; works even when config failed to load. When config is missing or invalid, the response includes `configError` with a human-readable load/validation message (no stack traces or shell commands).
- `GET /status` — device name, presets, monitors (no shell commands). Returns **503** with `{ "error": "config not loaded", "configError": "..." }` when config did not load.
- `GET /events` — recent activity (see below). Always returns 200, even when config failed to load.
- `POST /apply-preset` — apply a named preset (see below). Returns the same **503** shape when config did not load.

### `GET /health`

**Response** (200):

| Field           | When config loaded | When config missing/invalid |
|-----------------|--------------------|-----------------------------|
| `status`        | `"ok"`             | `"ok"`                      |
| `configLoaded`  | `true`             | `false`                     |
| `configError`   | omitted            | human-readable load/validation message |

Example when config failed:

```json
{
  "status": "ok",
  "configLoaded": false,
  "configError": "failed to read config file: ..."
}
```

### `GET /events`

**Response** (200):

| Field    | Description |
|----------|-------------|
| `events` | Up to 50 most recent events, newest first |

Each event: `{ timestampMs, kind, message, preset?, source?, monitorId? }`. `kind` is `"info" \| "success" \| "error"`; `source` (when present) is `"api" \| "tray" \| "hotkey"` — which trigger caused the preset apply. Recorded on config load/failure, preset apply start/finish, and per-monitor native-DDC results. Messages never include shell commands or raw VCP values — only preset/monitor names and outcome summaries.

### `POST /apply-preset`

**Request** (`application/json`):

| Field        | Type    | Default | Description |
|--------------|---------|---------|-------------|
| `preset`     | string  | —       | Preset name (required) |
| `dryRun`     | boolean | `false` | Resolve and report commands without executing |
| `localOnly`  | boolean | `false` | When `true`, only apply monitors owned by this machine — no peer fan-out. Peers always receive `localOnly: true` when called by a coordinator. |

**Response** (200 on success):

| Field              | Description |
|--------------------|-------------|
| `preset`           | Preset name applied |
| `dryRun`           | Whether this was a dry run |
| `localOnly`        | Whether peer fan-out was skipped |
| `planningErrors`   | Structured planning failures (e.g. unknown monitor on coordinator) |
| `localResults`     | Per-monitor results for commands run on this machine |
| `peerResults`      | Per-peer outcomes when coordinating (HTTP errors, nested `localResults` on success) |

`lastAppliedPreset` in `GET /status` updates only on a **non-dry-run full success**: no planning errors, all local monitor outcomes successful, and all peer calls successful. Dry-run and any failure leave it unchanged. A coordinated apply with no local entries still updates when all peer work succeeds.

| Setting        | Default   | Bind address                          |
|----------------|-----------|---------------------------------------|
| `apiPort`      | `3737`    | Port on this machine                  |
| `apiLanAccess` | `false`   | `127.0.0.1` (local processes only)    |
| `apiLanAccess` | `true`    | `0.0.0.0` (reachable from the LAN)    |

With `apiLanAccess: false`, only local clients can call the API. With `apiLanAccess: true`, any machine on your network that can reach the port may trigger configured presets by name. There is no auth yet — treat LAN access as an explicit opt-in on trusted networks only.

Peer entries (`peers[].host` / `peers[].port`) tell DeskMux where to call on *other* machines; they do not change where this machine binds its own API.

## The important part: the `command` field

DeskMux does **not** know your monitors' DDC/CI input values — they vary by make and model. You supply the command that works on your setup. DeskMux just runs it. Use dry-run mode first to confirm the commands before letting them fire.

### Windows examples

Using [ControlMyMonitor](https://www.nirsoft.net/utils/control_my_monitor.html) (NirSoft). Input Select is VCP code `60`; the value for each input is monitor-specific — read it off the tool once, then paste it in.

```
"command": "C:\\Tools\\ControlMyMonitor.exe /SetValue \"\\\\.\\DISPLAY1\\Monitor0\" 60 15"
```

(Here `15` is the DisplayPort value on *that* monitor — yours may differ. `17` is often HDMI. Confirm with ControlMyMonitor's GUI.)

### macOS examples

Using [BetterDisplay](https://github.com/waydabber/BetterDisplay) CLI or [Lunar](https://lunar.fyi/)'s CLI:

```
"command": "betterdisplaycli set --name=\"LG HDR 4K\" --inputSource=hdmi1"
```

or with Lunar:

```
"command": "lunar displays \"LG HDR 4K\" input HDMI1"
```

### Linux (if you build for it)

Using `ddcutil`:

```
"command": "ddcutil --display 1 setvcp 60 0x0f"
```

## Finding your input values

1. Install the backend for your platform.
2. Note each monitor's current input value while it's on a known source.
3. Switch inputs manually and note the value again — that difference is what you put in each `command`.
4. Put the commands in the config, run DeskMux in **dry-run**, and confirm the printed commands look right before switching dry-run off.
