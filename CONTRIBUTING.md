# Contributing

This is a small project. A few guidelines keep things consistent.

## Setup

```bash
git clone https://github.com/vssb4214/deskmux.git
cd deskmux
npm install
cp deskmux.config.example.json deskmux.config.json   # edit for your hardware
npm run tauri dev
```

Requires Node.js 20+ and a stable Rust toolchain.

## Branching

- Branch off `main`. Name branches by intent: `feat/preset-executor`, `fix/config-validation`, `docs/config-examples`.
- Keep each branch to one change.

## Commits

Follow [Conventional Commits](docs/COMMIT_CONVENTION.md). One logical change per commit. Squash `wip`/typo-fix commits before opening a PR.

## Before opening a PR

Run the checks CI runs:

```bash
npm run lint
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Pull requests

- Describe what changed and why. Link related issues.
- Note which platform(s) you tested (Windows / macOS).
- If you touched the config schema, update `deskmux.config.example.json` and `docs/CONFIG.md` in the same PR.

## Scope

DeskMux's MVP is monitor input switching and preset orchestration. Peripheral streaming and native DDC are roadmap items, not yet in scope — open an issue before starting work on either.
