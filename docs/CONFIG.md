# Configuration

DeskMux reads `deskmux.config.json` from the **process working directory** at startup and when saving from the desktop app. The filename is always `deskmux.config.json`; load and save use the same resolved path.

Copy the template next to where DeskMux runs:

```bash
cp deskmux.config.example.json deskmux.config.json
```

**Where that file lives depends on how you start DeskMux:**

| How you run | Typical working directory | Config path |
|-------------|-------------------------|-------------|
| `npm run tauri dev` | `src-tauri/` | `src-tauri/deskmux.config.json` |
| Installed app / `deskmux.exe` | Directory containing the executable | `./deskmux.config.json` beside the binary |

On startup DeskMux logs the exact path it tried, for example:

```text
deskmux: loading config from C:\path\to\src-tauri\deskmux.config.json
```

`/health` and dashboard config errors include the same path when load fails. There is no alternate search path and no UI/API argument to pick a different file.

`deskmux.config.json` is gitignored — it describes *your* desk, so it stays out of the repo.

Sibling files `deskmux.config.json.bak` and `deskmux.config.json.tmp` may appear briefly during save from the desktop app; they are gitignored too.

## Saving from the desktop app (Tauri IPC)

Config drafting and save are **desktop-only** — they use Tauri IPC, not the HTTP API. There is no `POST /config/validate` or config-write HTTP endpoint; LAN peers and browser-only dashboard viewers cannot validate or save config over HTTP.

From the DeskMux window, the **Config draft** card lets you paste or edit JSON, **Validate**, then **Save**. This is temporary plumbing before the full first-run wizard.

| IPC command | Args | Result |
|-------------|------|--------|
| `validate_config_draft` | `{ json: string }` | `null` on success; structured `LoadError` on failure |
| `save_config_draft` | `{ json: string }` | `{ filename, backupCreated, restartRequired }` on success |

**Save behavior:**

1. Parse JSON and run the same `validate()` used at startup — invalid drafts return errors; the file on disk is untouched.
2. If `deskmux.config.json` already exists, copy it to `deskmux.config.json.bak` first. If the backup fails, the save aborts and the original file is not overwritten.
3. Write pretty-printed JSON to `deskmux.config.json.tmp`, then rename into `deskmux.config.json` (atomic on Windows and POSIX).
4. Return `restartRequired: true`. DeskMux does **not** hot-reload config — restart the app for the new file to take effect.

The save path is fixed (`deskmux.config.json` in the app working directory — see table above). IPC commands do not accept a path argument.


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

**This config shape is input-source switching only.** There's deliberately no field for an arbitrary VCP code. Brightness, contrast, and volume are available as live desktop controls where the monitor supports them, not as preset/config values. Power control is intentionally separate future work.

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
- `GET /native-ddc/displays` — native DDC display discovery (see below). Works without a loaded config.
- `GET /native-ddc/displays/{displayId}/input-source` — read the current VCP `0x60` input-source value (see below). Works without a loaded config.
- `GET /native-ddc/displays/{displayId}/controls` — read live brightness, contrast, and volume controls for one display (see below). Works without a loaded config.
- `POST /apply-preset` — apply a named preset (see below). Returns the same **503** shape when config did not load.

Setup-time and hardware writes (config save, native DDC test switches, live native DDC control writes) are **not** on this HTTP API — they're Tauri IPC commands, invokable only from the bundled desktop app, never from a plain HTTP request. See [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md) for native DDC commands and [FIRST_RUN_SETUP.md](./FIRST_RUN_SETUP.md) for config save.

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

### `GET /native-ddc/displays`

Read-only monitor discovery for filling in `nativeDdc` config values in-app instead of reading
startup logs (see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md)). Works with no config
loaded — first run is when discovery matters most.

**Response** (200):

| Field             | Description |
|-------------------|-------------|
| `nativeAvailable` | Whether native DDC works on this platform (Windows only today). `false` means `displays` is empty; configure shell commands instead. |
| `displays`        | `[{ "displayId": "..." }]` — copy into `monitors[].nativeDdc.displayId` |

### `GET /native-ddc/displays/{displayId}/input-source`

Reads the current VCP `0x60` input-source value for one display (percent-encode the
`displayId`). To identify each input's value: switch the monitor to an input physically, read,
note the value, repeat. The reply's `maximum` is a single monitor-reported number — not a list
of supported values.

Reads retry once internally with a refreshed enumeration (some monitors fail transiently after
hotplug). Discovery never writes to the monitor.

**Response** (200): `{ "current": 4626, "maximum": 4626 }`

**Errors** carry a stable `code` alongside the message:

| Status | `code` | Meaning |
|--------|--------|---------|
| 404 | `displayNotFound` | `displayId` not in the current enumeration |
| 500 | `vcpReadFailed` / `enumerationFailed` | DDC read failed even after refresh / enumeration failed |
| 501 | `nativeUnavailable` | Not a Windows build |

### `GET /native-ddc/displays/{displayId}/controls`

Reads live continuous native DDC controls for one display: brightness (`0x10`), contrast (`0x12`),
and volume (`0x62`). This is for immediate adjustment only; it does not add config or preset
fields. Unsupported controls are returned individually as unavailable so one missing feature does
not hide the others.

**Response** (200):

```json
{
  "displayId": "K@P:d0e5:0",
  "controls": {
    "brightness": { "available": true, "current": 70, "maximum": 100 },
    "contrast": { "available": true, "current": 50, "maximum": 100 },
    "volume": { "available": false, "error": "vcpReadFailed" }
  }
}
```

Writes use the desktop-only `set_native_ddc_control` IPC command:

```js
invoke('set_native_ddc_control', { displayId, feature: 'brightness', value: 70 })
```

Only `brightness`, `contrast`, and `volume` are accepted feature names. DeskMux reads the monitor's
current value and maximum before writing, rejects values above the monitor-reported maximum, and
does not accept arbitrary VCP codes. Power control (`0xD6`) is intentionally not included here.

### `probe_input` (Tauri IPC command, not HTTP)

Setup-time test switch for one native DDC input value (VCP `0x60`). Deliberately **not** an HTTP
endpoint — see [NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md#security-and-safety) for why.
Invoked from the dashboard as `invoke('probe_input', { displayId, value })`.

**Arguments:** `displayId: string`, `value: number` (`u16`).

**Only accepts a `value` that a prior `GET .../input-source` read has already returned as that
display's current input this session** — enforced inside the command, not just by what the UI
offers. There is no way to probe a value that hasn't been read first; see
[NATIVE_DDC_DISCOVERY.md](./NATIVE_DDC_DISCOVERY.md#security-and-safety) for the reasoning.

**Response on success:** `{ "accepted": true, "displayId": "K@P:d0e5:0", "value": 4626, "current": 4626 }` — `current` is a best-effort read-back after the write and may be omitted if it fails.

**Errors** reject the promise with `{ error, code }`:

| `code` | Meaning |
|--------|---------|
| `valueNotObserved` | `value` has not been read as this display's current input this session |
| `displayNotFound` | `displayId` not in current enumeration |
| `vcpWriteFailed` | Native DDC write attempt failed |
| `nativeUnavailable` | Not a Windows build |

The dashboard's "Test this input" control (setup checklist) auto-reverts to the pre-probe value
after a short countdown unless confirmed — see `src/lib/probe.js`.

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
