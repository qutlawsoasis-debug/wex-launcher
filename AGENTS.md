# Rules for agents — Wex

Wex is the new launcher for the Aura client and Aura-Pack. It replaces the old Aura-Launcher (C#/WPF), which was a mess. Nothing is copied from the old code: it is a reference for behaviour only.

## Stack

- `core/` — Rust library crate `wex-core`: networking, pack sync, Minecraft + Fabric + Java install, launch, accounts, config. Pure Rust, no UI, fully unit-tested, usable from a CLI (`wex-cli`) and from the app.
- `app/` — Tauri 2 shell (`src-tauri/`) that exposes `wex-core` to the UI through typed commands and events.
- `ui/` — Svelte 5 + TypeScript + Vite front end, including the design system in `ui/src/lib/design/`.
- Windows 10/11 first. Do not break the macOS or Linux builds on purpose.

## Hard rules

1. This repository contains code only. Never commit reports, screenshots, logs, zips, downloaded jars or scratch scripts. Reports go to `C:\Users\magne\Documents\AuraDev\.agent-state\agy\`.
2. No git checkout/restore/stash/reset/clean on anything you did not create. No force push, no history rewrite. Work on a branch from fresh `origin/main` and never merge into main yourself.
3. Never touch the user's real game or launcher data: `%APPDATA%\.minecraft`, the old Aura-Launcher data folders, or any running game. Tests use temporary directories. Wex keeps its own data root (default `%APPDATA%\Wex`), and every Minecraft instance gets its own game directory.
4. Both versions always: Minecraft 1.20.1 and 1.21.1 install, sync and launch together.
5. Downloads are checked with SHA-1/SHA-256 before use, can resume, run in parallel with a limit, and never block the UI thread.
6. Tests run with the build: `cargo test` in `core/`, `npm test` and `npm run check` in `ui/`. A task is not done until they pass.
7. Debug and QA paths are opt-in only. A normal launch never runs demos or test data.
8. No secrets in the repository. Client IDs or API keys that must stay private come from environment variables or the user's config.

## Apple is the specification

Before working on the interface, read `C:\Users\Public\aura-knowledge\00-Index.md` and the relevant card; use only cards with status verified/confirmed.

Every visible element follows Apple documentation. Before building any button, toggle, slider, list, sheet, tab bar, sidebar, search field, material, animation or transition, read its page first:

- Human Interface Guidelines: https://developer.apple.com/design/human-interface-guidelines/<page>
- Liquid Glass: https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass
- Machine-readable pages: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/<page>.json

The report for each element must name the page and the rule applied. An element with no citation is rejected. If Apple does not cover something, say so explicitly instead of inventing a value.

Colours come only from Apple system colours (the same palette as aura-client `common/ui/AppleColors.java`). Controls are neutral by default. There is one accent, systemBlue, on the primary action; systemGreen is for toggles and systemRed for destructive actions. Selection is a fill, never a coloured outline or glow. Respect Reduce Motion and Reduce Transparency.

## Running things

Every test run of the game or the app has a 5-minute timeout. If the log stops moving for 2 minutes, kill only your own process, read the log and fix the cause. Close everything you started.
