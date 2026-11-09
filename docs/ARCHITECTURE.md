# Architecture and data flow

This describes the implementation at main **4e1396c (2026-11-09)**. There are **199 registered Tauri commands** across core lifecycle, fast launch, network/lobby, social/friends, storage and file verification, system diagnostics, self-updater, window/tray, and UI adapters. [IPC.md](IPC.md) documents command contracts, argument/result/error shapes, and event producers. A generated TypeScript declaration or a UI listener is not evidence that a command or event producer exists.

## Boundaries and workspaces

| Layer | Responsibility | Important implementation boundary |
|---|---|---|
| `ui/` | Svelte 5 screens, theme/network state, components, user actions and event subscriptions | Calls `@tauri-apps/api/core::invoke` and `event::listen`; browser dev lacks the Tauri backend. Screen-specific fallback/empty states do not establish native backend behavior. |
| `app/src-tauri` | Native shell, command registry, state ownership, feature orchestration and event adaptation | Root Cargo workspace. `src/lib.rs::run` registers managed states and all commands. Command handlers contain substantial business logic and app-local DTOs; they are not all thin adapters to one core trait. |
| `core/wex-ipc` | Shared serde request/result/model/event types, `IpcError`, `LauncherCore`, mock and `ts-rs` generation | Contains 60 command constants, only 40 of whose names are registered. App-local handlers and DTOs extend and sometimes differ from this contract. |
| `core/wex-core` | Wex paths/config, downloads/hashes, Minecraft/Fabric/Java/pack preparation and game process management | `WexLauncherCore` in `ipc_adapter.rs` implements `LauncherCore`; owns running processes and cancellation tokens. |
| `core/wex-net` | SLP/DNS ping, NBT server lists, Lobby API, heartbeat, manifest comparison and Playit supervision | Internal `NetEventBus` is distinct from the Tauri event bus. App has no subscriber forwarding it to the UI. |
| `core/wex-social` | Friends service, local achievements, world/backups/screenshots, news and update metadata | Friend state and caches are held by the app; local workshop operations also accept explicit paths. |
| `core/wex-extras` | Crash diagnosis, Discord RPC, explicit old-launcher import, skin/CSL helpers and graphics presets | Used directly by feature handlers and the launch lifecycle. |
| `core/wex-cli` | CLI sync/install/launch using core libraries | Separate entry point; does not use the Tauri command registry. |

`core/Cargo.toml` is the library/CLI workspace. Root `Cargo.toml` is the desktop app workspace. Cargo commands from one workspace do not automatically test the other. Supported game versions remain **1.20.1 / Java 17** and **1.21.1 / Java 21**, with separate instance directories.

```mermaid
flowchart LR
    UI[Svelte screens] -->|invoke args JSON| Registry[Tauri generate_handler]
    Registry --> Launch[launch/settings handlers]
    Registry --> Social[friends/overview/workshop handlers]
    Registry --> Net[lobby/network/shell handlers]
    Registry --> Files[mods/wardrobe/workshop_packs handlers]
    Launch --> Trait[LauncherCore trait]
    Trait --> Core[WexLauncherCore]
    Core --> Install[Minecraft/Fabric/Java/pack]
    Core --> Java[Managed Java process]
    Social --> Libraries[wex-social / wex-extras]
    Net --> NetLib[wex-net]
    Files --> Disk[Wex instance files]
    Libraries --> Disk
    Install --> Disk
    Core -->|EventHandler callbacks| Bridge[TauriEventHandler]
    Bridge -->|install-progress / launch-status / game-log| UI
    Social -->|achievement unlock only| UI
    Net -->|network-status on transition| UI
    NetLib --> NetBus[Internal NetEventBus]
```

There is deliberately no NetEventBus → UI arrow: the app has not wired that bridge. Likewise, event names declared in `wex-ipc` for friend presence, lobby invites and launcher update availability have no `app.emit` producer in this snapshot.

## Startup and managed state

The Rust entry point `app/src-tauri/src/main.rs` calls `wex_app_lib::run`. Tauri setup in `lib.rs`:

1. Creates `WexPaths::new(None)` and attempts `ensure_dirs`.
2. Creates `NetworkState`, starts its background connectivity supervisor (TCP DNS endpoint probes every five seconds), and registers it with Tauri.
3. Creates `StartupState` for startup diagnostics, directory verification, and preflight update checks (`commands/startup.rs`).
4. Creates `PlayState::new_real`: a `WexLauncherCore` with `TauriEventHandler`, flags/current version, paths, running process handle, background prewarm instance and `is_prewarming` state, and a Discord client connection started in the background. If `fast_launch` is enabled in config and system RAM meets the >= 16 GB threshold, background prewarming is triggered.
5. Registers `FriendsState`, legacy `AppState`, and `LobbyState` with Lobby API clients, session/heartbeat/tunnel storage, and a `NetEventBus`.
6. Registers `TrayState` for system tray management and `DeepLinkState` for `wex://` protocol handling.
7. Registers the 199 command functions, configures the system tray (`setup_tray`), registers deep link handlers, and starts Tauri's event loop.

| Managed state | Ownership / lifetime |
|---|---|
| `PlayState` | `Arc<dyn LauncherCore>`, paths, atomic `is_launching`/`is_running`, async mutex `running_version`, Discord client, shared `NetworkState`, `prewarmed: Arc<Mutex<Option<PrewarmedInstance>>>`, `is_prewarming: Arc<AtomicBool>`, and `active_process: Arc<Mutex<Option<wex_core::GameProcess>>>`. Core owns OS process handles and cancellation tokens. |
| `NetworkState` | Shared online atomic and optional test override; supervisor runs every five seconds, probes TCP DNS endpoints, and emits `network-status` transitions. |
| `StartupState` | Directory verification and initial update check orchestration (`commands/startup.rs`). |
| `FriendsState` | `FriendApiClient`, optional auth, and in-memory friends/requests/invites caches behind async mutexes; API defaults to `https://api.wexmc.net`. |
| `LobbyState` | `LobbyApiClient`, active session/host token, heartbeat handle, `PlayitSupervisor`, `current_tunnel_status`, tools path, and legacy cloud endpoint. It owns the internal `NetEventBus`. |
| `TrayState` | System tray menu state (`stop_item` MenuItem) and window restoration hooks. |
| `DeepLinkState` | Pending deep link URI queue (`wex://join/...`) with thread-safe mutex and UI event trigger. |
| Legacy `AppState` | In-memory server list and a separate optional active_lobby model. |

The server list currently starts from entries hardcoded in `AppState::default`; `list_servers` clones that in-memory list rather than reading each instance's servers.dat. `add_server`/`remove_server` attempt root `Wex/servers.dat` persistence but suppress persistence errors. These are current boundaries, not guarantees of full per-instance server-list synchronization.

## Launch, cancellation and process completion

```mermaid
sequenceDiagram
    participant UI as Svelte
    participant App as launch handler / PlayState
    participant Net as NetworkState
    participant Core as LauncherCore / WexLauncherCore
    participant Services as Metadata/download services
    participant JVM as Java process
    UI->>App: start_game(version, nickname, jvmArgs?)
    App->>App: Duplicate-start guard and graphics preset
    App->>Net: Read connectivity
    App->>Core: get_installed_versions when offline
    alt Offline and client not marked installed
        App-->>UI: Reject String + launch-status error
    else Allowed to attempt launch
        App-->>UI: launch-status preparing
        App->>Core: launch_game(LaunchGameRequest)
        Core->>Services: prepare_and_launch as required
        Core-->>UI: EventHandler → progress/status/log events
        Core->>JVM: Spawn with version-specific game directory
        Core-->>App: LaunchGameResponse
        App-->>UI: Resolve response; running state
        App->>App: Background process-state monitor
        JVM-->>Core: Output / exit
        Core-->>UI: EventHandler → game-log / launch-status
        App->>App: Clear state, restore Discord, analyze logs
        App-->>UI: Final launch-status
    end
```

`launch_game(version, nickname)` is a compatibility wrapper that returns only `LaunchGameResponse.message`; `start_game` returns the full response. Both use `LauncherCore::launch_game`, not a registered `core_launch_game` command. The command awaits preparation/start, not the entire game session. Process completion is observed separately and may result in both core and app-level final status events.

The offline preflight checks `get_installed_versions`. In the current core adapter, `is_installed` is based on the client JAR's presence. In offline mode, the launcher permits starting installed instances without remote metadata or network sync requests.

`cancel_launch` requests core cancellation, clears the launching flag, cleans up any prewarmed instance, and emits `cancelled`. `kill_game` and `shell_stop_game` choose an explicit version or `running_version`, request termination, and clear flags. These handlers currently suppress core errors. A successful invoke is not proof that the OS process has exited; consult lifecycle events/process state. Crash diagnosis uses the relevant instance logs, emits `game-crash` with diagnostic details, and creates the user-facing final message.

### Fast launch and prewarm lifecycle

When `fast_launch` is enabled in configuration (defaulting to enabled only when total system RAM is >= 16 GB / 15,360 MB via `default_fast_launch`):

1. **JVM probe (`prewarm_jvm`):** Executes `java -Xms16m -Xmx32m -version` to populate the operating system's disk cache for `jvm.dll` and runtime libraries.
2. **Hidden background instance:** If the instance passes fast-path verification, `prepare_and_prewarm` spawns the game client in the background with a hidden GLFW window and prewarm JVM flags:
   `-Dwex.prewarm.signal=<instance_dir>/prewarm.signal`, `-Dwex.prewarm.parentPid=<launcher_pid>`, `-Dwex.qa.background=true`, `-Daura.client.hiddenWindow=true`.
3. **Instant launch takeover (`activate_prewarm`):** When the player clicks «Играть», the launcher writes `READY\n` to `prewarm.signal`. The waiting game client unhides its GLFW window immediately (sub-second launch time).
4. **Concurrent launch handling:** If the player launches while prewarming is in progress (`is_prewarming == true`), the launcher emits `launch-status` ("Игра догружается…"), writes `READY\n` in advance so the client unhides as soon as its main menu renders, and waits up to 30 seconds for process readiness.
5. **Prewarm cleanup (`cleanup_prewarm`):** On `cancel_launch`, `kill_game`, configuration change, or app exit, the prewarmed process is cleanly killed and `prewarm.signal` is deleted.

## Feature flows

| Feature | Data path and important behavior |
|---|---|
| Fast launch / Prewarm | Background JVM probe and hidden process execution gated by >= 16 GB RAM threshold; `activate_prewarm` writes `READY\n` to `prewarm.signal` for instant sub-second launch takeover. Automatic teardown on cancel, exit, or config modification. |
| Live friends / Notifications | Adaptive poller in `friendsLive.ts` (25s when window is visible/idle, 120s when hidden/in-game). Emits native Windows notifications (`tauri-plugin-notification`) when app is blurred/hidden/playing; emits in-app toasts when app is focused. Enforces a 5-minute per-friend anti-spam cooldown. Controlled by `friend_notifications` setting. |
| Lobby / Multiplayer | Room creation (public/friends/invite-only), host controls (kick, close), vertical step-by-step join flow (`net_compare_manifests`, mod download, playit tunnel supervision `net_tunnel_*`, retry `retry_lobby_tunnel`, launch with `--quickPlayMultiplayer`). UDP multicast LAN discovery (224.0.2.60:4445). |
| Honest offline mode | Degraded empty/status states across all 8 screens with retry buttons (`network.check()`). NetworkState supervisor runs every 5s; automatic reload when connectivity is restored (`network-status`). Allows launching installed game instances without network calls. |
| File verification & repair | `verify_game_files` (`commands/storage.rs`) and `verify_instance_files` (`commands/settings.rs`) verify SHA-1 hashes of client JAR, libraries, and assets against remote index; repairs corrupt files with `storage-verify-progress`. Blocked while game is running. |
| System info («О системе») | Hardware telemetry in `wex_core::system_info`: CPU cores, discrete vs integrated GPU detection (properly classifying Intel Arc A-series, Nvidia, AMD), RAM, Windows version/build, disk free space warning banners (<5 GB / <20 GB), Java runtimes, path sanitization (`C:\Users\<user>`), and "Скопировать для поддержки" clipboard action. |
| Secure self-updater | Minisign Ed25519 cryptographic signature verification. In release builds, public key is resolved strictly from compile-time build configuration (`resolve_trusted_public_key`), ignoring environment variables. Unsigned or keyless updates fall back to notify-only mode with manual download link (`updater_open_download_url`). Blocked while game is running. |
| Window state & Tray | Multi-monitor bounds sanitization (`sanitize_window_position`) prevents off-screen rendering. System tray menu with "Показать", "Остановить игру", "Выйти". Configurable `minimize_to_tray` and launch window behavior (`GameWindowBehavior`: `hide` in tray, `minimize`, `keep` open). |
| Settings | Full app-local LauncherSettingsDto ↔ `wex_core::config::LauncherConfig` on disk; metadata includes system RAM and Java paths. Save clamps RAM to at least 1024 MB, writes config and attempts preset/core synchronization. Those latter errors are suppressed. Reading game_dir reports WexPaths.root; submitting a different game_dir does not re-root WexPaths in this handler. |
| Narrow core config | `get_config`/`save_config`/`set_config` use shared LauncherConfig with ram_gb and fewer fields. It is not the full settings DTO with ram_mb. Core conversion is lossy relative to full settings; use the intended endpoint. |
| Network / shell | `get_network_status` reads the five-second supervisor flag. `shell_check_network` performs an independent active TCP probe via spawn_blocking and returns latency/status_text. `shell_get_profile` checks local CSL/general skin files and may return a remote avatar URL. These are distinct APIs, not interchangeable payloads. |
| Friends / presence | request auth takes precedence over flat auth and stored auth where supported. Login stores auth and can return an offline profile; get_friends/sync can return cached state on service failure. Mutating request/response operations map some API errors to structured SOCIAL_* errors. send_lobby_invite and set_presence currently suppress remote failures. |
| Achievements | Definitions plus `achievements.json` in selected data_dir/root. Progress changes may emit social-achievement-toast on unlock; listings return definitions and progress separately. No Tauri event is emitted just because a UI component subscribes to it. |
| Mods | Read JAR metadata, rename .jar ↔ .jar.disabled, copy custom JARs, reject removal of pack-owned mods. Uses version-specific mods plus installed-mod state. File/validation failures are String rejections. |
| Worlds / backups / screenshots | `workshop_*` convenience commands use selected version (default 1.21.1) where applicable, while `social_*` compatibility requests may supply explicit directories. Backups are ZIPs; restore derives a name from the archive stem when no target is supplied. Destructive world deletion accepts a path, not a world id. `workshop_open_path` reveals/opens through the OS. |
| Resource packs / shaders | App-local ResourcePackItemDto/ShaderPackItemDto; reads ZIP metadata/icons, options.txt resourcePacks order/enabled entries and Iris shader properties. `pack_type == "shaderpack"` selects shaderpacks; other values fall back to resourcepacks. Pack-owned files cannot be deleted; post-delete disable operations are best-effort. |
| Wardrobe | Shared skin/wardrobe DTOs + wex-extras validation/conversion/history and app-level wardrobe.json/CSL installation. Apply targets both supported instance versions. Some filesystem work is best-effort; success is not an atomic multi-instance transaction. |
| News / updates / import | News loads cache/bundled notes. Update checks fetch only an explicit manifest URL. Old-launcher detection/read/apply are separate explicit operations; apply can move selected nickname/RAM/version/servers/skins into Wex. Startup detection does not imply automatic application of an import. |

## Events and schema limits

Eleven frontend event names have active app-side producers: `install-progress`, `launch-status`, `game-log`, `game-crash`, `network-status`, `social-achievement-toast`, `storage-verify-progress`, `updater-progress`, `startup-status`, `onboarding-status`, and `deep-link-triggered` (`deep-link://url`). [IPC.md](IPC.md) lists each payload structure. Shared definitions also declare six unbridged/unproduced names; [IPC.md](IPC.md) lists each one. Network service responses and social_sync remain available to fetch current authoritative state.

The app emits bare serde DTO payloads. The shared `IpcEvent` tagged enum `{event,payload}` is a separate representation, not the Tauri listener envelope. Numeric progress fields can represent counts per stage; the current launch adapter reports speed as zero. Events have no replay, delivery acknowledgement or exactly-once completion guarantee. UI should subscribe before starting work, unlisten on teardown, and reconcile with command results/state.

Errors are also not uniform: most registered handlers return String, friends and low-level lobby/tunnel return IpcError, and get_data_dir returns a plain String. The generated IpcResult tagged union does not describe how a Tauri Result Promise resolves/rejects. These distinctions and exact top-level camelCase versus nested snake_case keys are documented in [IPC.md](IPC.md).

The unified error layer adds `From<WexError> for IpcError` in core: install/launch now preserve error classes for Java, integrity/download failures, network, cancellation and process exit inside the core adapter. The app launch handler still converts errors to String or substitutes crash diagnosis, so this does not make every Tauri rejection structured. Generated ERROR_CODES is a presentation catalog and does not include every converter-produced code. In the UI, `formatError(unknown, domain)` in `ui/src/lib/design/errors.ts` extracts a code or infers one from text, then returns title/description/actions/technical for ErrorState. Screens supply action callbacks; displaying an error state does not automatically retry a mutating operation.

Generated TypeScript covers `wex-ipc` declarations, not all app-local types or registry names. Drift tests compare those generated declarations, not frontend call arguments against every registered handler. ModEntry, LauncherSettingsDto, shell/network DTOs, ApplyImportRequest/Response and resource/shader pack DTOs are maintained in app source and currently need explicit UI synchronization.

## Storage, concurrency and side effects

`WexPaths` chooses an explicit custom root first, then `WEX_DATA_DIR`, then `%APPDATA%\Wex` on Windows, then the platform data directory/Wex, and finally `.wex`. Startup uses `new(None)`, so `WEX_DATA_DIR` is the available override for an isolated native QA run. It owns config.json, instances, libraries, assets, versions, runtime and logs. `instances/1.20.1` and `instances/1.21.1` have independent game data/natives; shared downloads/runtime live under the Wex root. Features add tools, news, wardrobe/history, backups and local metadata. Tests should supply temporary roots; normal launcher operations must not target the user's .minecraft or old launcher directories. Explicit import reads a specified old directory, so callers must distinguish it from writing Wex data.

Async Tauri handlers and Tokio cover HTTP and lifecycle tasks; shell active connectivity explicitly uses spawn_blocking. Many handlers still perform std::fs, ZIP/image processing or OS launch operations synchronously inside async functions. The current code does not establish that all file/CPU work is moved to a blocking pool; do not claim that it does. Async mutexes protect friends/lobby/runtime handles; std mutexes remain in legacy AppState. Downloads use bounded parallelism in core and hash verification; UI never receives process handles or state mutexes.

Many actions are multi-step and contain best-effort writes or ignored errors, as identified in the command inventory. There is no cross-feature transaction spanning config, filesystem, remote social service and running game. Frontend success handling should use returned counts/state rather than assuming all downstream work was atomic.

## Documentation and validation boundaries

When adding a native API, update the registry, actual Rust handler/DTO, UI invocation, relevant shared/generated types, and [IPC.md](IPC.md). An event requires a producer/bridge as well as a declared name and UI listener. Confirm the actual serde format and casing, including enum variants.

For code changes, the repository checks cover both Rust workspaces and UI test/check (see [README.md](../README.md)):
- `scripts/check.ps1` runs the full 9-step suite (Rust format, Clippy with warnings denied, and tests for `core` and `app`, followed by `npm ci`, Vitest UI tests, and Svelte/TypeScript checks).
- Component accessibility audits run via Vitest (`npm --prefix ui --workspaces=false test -- src/lib/a11y.test.ts`), executing axe-core sweeps across dark and light themes.
- E2E keyboard navigation tests run via Playwright (`npx --prefix ui playwright test e2e/keyboard.spec.ts`).
- E2E smoke tests across all 8 screens and flows run via Playwright (`npm --prefix ui --workspaces=false run smoke`).

This source audit reflects all 199 registered Tauri commands in `app/src-tauri/src/lib.rs`, active event producers, and feature orchestration contracts at `4e1396c`.
