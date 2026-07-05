# DeskMux

A cross-platform display-profile switcher for multi-machine, multi-monitor desks.

DeskMux gives you one dashboard to flip your monitors between computers — all screens on your Windows PC, all on your Mac, or any split you define — by driving each monitor's input over DDC/CI. It's config-driven, so it works with whatever monitors you own instead of hardcoding values for mine.

> **Status: MVP / work in progress.** Monitor input switching and preset orchestration work today. Keyboard/mouse sharing and native DDC are on the roadmap, not in the box yet. See [Limitations](#limitations).

## Why

If you run two machines into the same set of monitors, switching "who's driving which screen" means walking through each monitor's OSD menu by hand. DeskMux collapses that into one click by sending the input-select command to each monitor for you, and lets you save the layouts you actually use as presets.

## Features

- **Preset switching** — define layouts once (All PC, All Mac, splits, custom) and apply them with one click.
- **Config-driven** — monitors, inputs, and switch commands live in `deskmux.config.json`. Nothing about your hardware is baked into the code.
- **Dry-run mode** — enabled by default in the dashboard; preview resolved commands before anything runs. Uncheck to execute.
- **Local API + LAN coordination** — `/health`, `/status`, and coordinated preset apply over HTTP (`controlledBy`, peer fan-out). No auth yet; LAN bind is opt-in.
- **Execution logs** — every command's stdout/stderr surfaces in the UI so failures are obvious.

## Requirements

- Windows 10/11 or macOS 13+
- A monitor-control backend for your platform (e.g. ControlMyMonitor or `ddcutil` on Windows/Linux; BetterDisplay or Lunar on macOS). DeskMux calls these; it doesn't replace them yet.
- Node.js 20+ and Rust (stable) if building from source.

## Install

_Coming with the first tagged release._ Until then, build from source:

```bash
git clone https://github.com/vssb4214/deskmux.git
cd deskmux
npm install
npm run tauri dev
```

## Configure

Copy the example config and edit it for your setup:

```bash
cp deskmux.config.example.json deskmux.config.json
```

Each monitor declares its inputs and the shell command that selects each input. DeskMux doesn't guess DDC values — you supply the command that works for your monitor and backend. See [`docs/CONFIG.md`](docs/CONFIG.md) for the full schema and worked examples for Windows and macOS.

## Usage

1. Start DeskMux on each machine. It loads `deskmux.config.json` and serves the local HTTP API (default port `3737`; the dashboard discovers the URL automatically).
2. **Dry run** is checked by default — apply a preset to preview resolved commands without executing them.
3. Uncheck **Dry run — preview commands only** when you are ready to switch monitor inputs, then apply a preset.
4. Results (local, peer, and planning errors) appear in the dashboard; `lastAppliedPreset` updates only after a non-dry-run full success.

Config is still edited by hand in `deskmux.config.json` — in-app config editing is not implemented yet.

## Limitations

- **Peripheral sharing isn't here yet.** DeskMux switches *displays* today. Sharing one keyboard/mouse across machines is a planned, separate phase (see [Roadmap](#roadmap)) — and worth being precise about what's physically possible:
  - *Software sharing* (control another machine's pointer over the network, Synergy/Deskflow-style) is real software and is on the roadmap.
  - *True USB handoff* — making your physically-plugged-in keyboard belong to a different computer — cannot be done in software by anyone. USB is host-to-device; two computers are both hosts, so a bare USB (or USB-C) cable between them does nothing. Re-routing the physical device requires a hardware USB switch/KVM. This is a law of the protocol, not a DeskMux shortcoming.
- **Currently backend-dependent (changing soon).** Today DeskMux shells out to tools like ControlMyMonitor / ddcutil / BetterDisplay / Lunar. Native DDC/CI is the next milestone, which will make monitor switching work with no external tool installed. Either way, if a monitor doesn't expose input-select over DDC/CI (some laptop panels, some LG models, the DDC-blind Apple Silicon HDMI port), it can't be switched.
- **Research-grade.** This is an MVP. Expect rough edges.

## Roadmap

Phased so every step is a usable tool on its own. Full detail in [`docs/ROADMAP.md`](docs/ROADMAP.md).

**Now — monitor switching foundation**
- [x] Config model, loader, validation
- [x] Preset executor + dry-run
- [x] Local HTTP API + LAN peer coordination
- [x] Dashboard apply/status UI (dry-run on by default)
- [ ] Dashboard config editing (machines, monitors, presets in UI)
- [ ] Live logs / richer execution history

**Next — make it standalone**
- [ ] Native DDC/CI support (no external monitor tool required), with the shell-command path kept as an escape hatch for quirky displays
- [ ] System tray + global hotkeys
- [ ] Peer auto-discovery on the LAN

**Later — peripheral sharing (its own major phase)**
- [ ] Integrate software keyboard/mouse sharing (Deskflow integration first; a from-scratch low-latency implementation is a stretch goal, not a v1)
- [ ] Transport-aware, latency-optimized LAN path (prefer Ethernet over WiFi; lean UDP protocol)
- [ ] Hardware USB-switch integration for true peripheral handoff

Not planned: a direct USB-cable link between the two computers — physically impossible (see Limitations).

## Prior art & how DeskMux differs

Software monitor-switching over DDC/CI is well-trodden. The closest project is [display-switch](https://github.com/haimgel/display-switch) — a mature, cross-platform Rust tool that switches monitor inputs via DDC/CI. Others include [display-input-switcher](https://github.com/3urobeat/display-input-switcher) (hotkey scripts) and various DIY setups wiring up ControlMyMonitor / ddcutil / BetterDisplay / Lunar by hand. For keyboard/mouse sharing, the established tools are [Deskflow](https://github.com/deskflow/deskflow) (and its ancestors Synergy / Barrier).

DeskMux isn't trying to reinvent DDC switching — that part isn't novel. The intended difference is in the packaging and the layouts:

- **Arbitrary layouts, not one-way follow.** Most existing switchers move *all* monitors together when you press a USB switch. DeskMux is preset-driven: monitor 1 on machine A while monitor 2 is on machine B, any split, any number of machines and monitors, applied from a dashboard.
- **A dashboard for apply/status today** — preset apply, dry-run, and structured results are in the UI. Adding/removing machines and monitors still means editing `deskmux.config.json` for now.
- **Aiming to be dependency-free** once native DDC lands, so there's nothing else to install.

The peripheral-sharing goal is deliberately honest: the near-term path is integrating existing software sharing rather than claiming a from-scratch engine, and the design leans on the LAN transport (Ethernet-preferred, lean protocol) as the place to actually improve latency. See [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Commits follow [Conventional Commits](docs/COMMIT_CONVENTION.md).

## License

[MIT](LICENSE)
