# DeskMux

A cross-platform display-profile switcher for multi-machine, multi-monitor desks.

DeskMux gives you one dashboard to flip your monitors between computers — all screens on your Windows PC, all on your Mac, or any split you define — by driving each monitor's input over DDC/CI. It's config-driven, so it works with whatever monitors you own instead of hardcoding values for mine.

> **Status: MVP / work in progress.** Monitor input switching and preset orchestration work today. Keyboard/mouse sharing and native DDC are on the roadmap, not in the box yet. See [Limitations](#limitations).

## Why

If you run two machines into the same set of monitors, switching "who's driving which screen" means walking through each monitor's OSD menu by hand. DeskMux collapses that into one click by sending the input-select command to each monitor for you, and lets you save the layouts you actually use as presets.

## Features

- **Preset switching** — define layouts once (All PC, All Mac, splits, custom) and apply them with one click.
- **Config-driven** — monitors, inputs, and switch commands live in `deskmux.config.json`. Nothing about your hardware is baked into the code.
- **Dry-run mode** — see exactly which commands would fire before anything touches your monitors.
- **Local API** — `/health`, `/status`, and preset application over HTTP, so the two machines can coordinate over your LAN.
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

1. Start DeskMux on each machine. It loads `deskmux.config.json` and serves the dashboard on port `3737`.
2. Toggle **Dry run** if you want to preview commands first.
3. Click a preset. DeskMux resolves the layout, runs the configured command for each monitor, and logs the result.

## Limitations

- **No USB peripheral switching.** True zero-latency keyboard/mouse handoff needs hardware (a real KVM). DeskMux switches *displays*; software peripheral sharing is a planned integration, not a current feature.
- **Backend-dependent.** DeskMux orchestrates commands from tools like ControlMyMonitor / ddcutil / BetterDisplay / Lunar. If your monitor doesn't expose input select over DDC/CI, DeskMux can't switch it.
- **Research-grade.** This is an MVP. Expect rough edges.

## Roadmap

- [ ] Native DDC/CI support (drop the external backend dependency)
- [ ] System tray + global hotkeys
- [ ] Peer auto-discovery on the LAN
- [ ] Deskflow integration for software keyboard/mouse sharing
- [ ] Hardware USB-switch integration for real peripheral handoff

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Commits follow [Conventional Commits](docs/COMMIT_CONVENTION.md).

## License

[MIT](LICENSE)
