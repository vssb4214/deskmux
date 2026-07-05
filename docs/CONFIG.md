# Configuration

DeskMux reads `deskmux.config.json` from the project root at startup. Copy the template and edit it for your hardware:

```bash
cp deskmux.config.example.json deskmux.config.json
```

`deskmux.config.json` is gitignored — it describes *your* desk, so it stays out of the repo.

## Schema

| Field                      | Type   | Description                                                                    |
|----------------------------|--------|--------------------------------------------------------------------------------|
| `deviceName`               | string | This machine's id (must match one of the `devices[].id`).                      |
| `peers[]`                  | array  | Every other machine on the LAN.                                                |
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
| `monitors[].inputs`        | object | Map of **device id** → `{ type, command }`. Only list the machines this monitor can actually receive. |
| `inputs.<deviceId>.type`   | string | Connector type, informational (`hdmi`, `displayport`, `usb-c`).               |
| `inputs.<deviceId>.command`| string | Shell command that selects this input on this monitor.                         |
| `presets`                  | object | Map of preset name → `{ label, layout }`.                                    |
| `presets.<name>.layout`    | object | Map of `monitorId` → `deviceId`.                                              |

**Key idea:** input keys are device ids, not fixed strings. Two machines or six, three monitors or one — you describe your setup and DeskMux adapts. A monitor only needs to declare the inputs it physically has; presets can only route a monitor to a machine it declares.

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
