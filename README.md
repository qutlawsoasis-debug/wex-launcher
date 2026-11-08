# Wex

Wex is a Windows-first launcher for Aura Client and Aura-Pack. The launcher is built from a Rust core and networking libraries, a Tauri 2 desktop shell, and a Svelte 5 + TypeScript UI. The supported game targets are Minecraft 1.20.1 and 1.21.1 with Fabric.

## Repository map

- `core/wex-core` — configuration and Wex data paths, checksummed/resumable downloads, Mojang Minecraft assets, Fabric, Mojang Java runtimes, Aura-Pack synchronization, and game launch lifecycle.
- `core/wex-net` — Minecraft server ping and `servers.dat`, Lobby API, mod manifest comparison, and playit tunnel process management.
- `core/wex-extras` — crash diagnosis, Discord Rich Presence, old-launcher data import, skin helpers, and graphics presets.
- `core/wex-social` — friends, achievements, workshop/world helpers, release notes, and launcher update checks.
- `core/wex-ipc` — serializable command, model, event, and error contracts, `LauncherCore` trait, and TypeScript binding generator.
- `core/wex-cli` — command-line sync, install, and launch entry points.
- `app/src-tauri` — Tauri application setup, command handlers, state, and event forwarding.
- `ui` — Svelte screens, components, design system, tests, and generated IPC types.
- `docs` — product specification and IPC notes.

The core workspace is `core/Cargo.toml`; the desktop application workspace is the root `Cargo.toml`. They are separate Cargo workspaces and both need checking.

## Requirements

- Rust toolchain with Cargo
- Node.js and npm
- Tauri 2 platform prerequisites for building the desktop application (on Windows, WebView2 and the Microsoft C++ build tools)
- Java is installed by Wex for game instances; it is not needed to run the Rust or UI checks

## Checks

On Windows PowerShell, run the repository check script from the repository root:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\check.ps1
```

It runs formatting checks, Clippy with warnings denied, Rust tests for both workspaces, then `npm ci`, UI tests, and Svelte/TypeScript checks. Rust and Vitest use at most two workers. The script lowers its process priority to `BelowNormal`, prints a PASS/FAIL result for each check, and exits non-zero if any check fails.

The equivalent individual commands are:

```powershell
Push-Location .\core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
Pop-Location

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2

Push-Location .\ui
npm ci
npm test -- --maxWorkers=2
npm run check
Pop-Location
```

## Build and run

Install UI dependencies and build the frontend:

```powershell
Push-Location .\ui
npm ci
npm run build
Pop-Location
```

Run the desktop app in development mode (requires Tauri platform prerequisites):

```powershell
Push-Location .\ui
npm ci
Pop-Location
cargo tauri dev
```

Build a release desktop bundle with `cargo tauri build`. The Rust CLI can also be run from the core workspace:

```powershell
Push-Location .\core
cargo run -p wex-cli -- sync 1.21.1
cargo run -p wex-cli -- install 1.21.1
cargo run -p wex-cli -- launch 1.21.1 --nick Player
Pop-Location
```

Wex stores its own data under `%APPDATA%\Wex` by default (or a platform data directory when `APPDATA` is unavailable). Its configuration, instances, libraries, assets, downloaded Java runtime, and logs live beneath that root. It does not use the old launcher's data directory; import helpers read old data only when explicitly given a source path.

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for module responsibilities and the current command/event flow between the UI, Tauri, and Rust core.
