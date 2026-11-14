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

It runs formatting checks, Clippy with warnings denied, Rust tests for both workspaces, then `npm ci`, UI tests (including Vitest and Axe-core accessibility audits), and Svelte/TypeScript checks. Rust and Vitest use at most two workers. Rust checks disable incremental compilation and debug symbols to limit temporary disk use. The npm commands target the `ui` workspace package with npm workspace traversal disabled because its lockfile is stored in `ui/`; package lifecycle scripts run in the foreground. The script lowers its process priority to `BelowNormal`, prints a PASS/FAIL result for each check, and exits non-zero if any check fails.

The equivalent individual commands are:

```powershell
Push-Location .\core
cargo fmt --all -- --check
cargo clippy --workspace -j 2 -- -D warnings
cargo test --workspace -j 2
Pop-Location

cargo fmt --all -- --check
cargo clippy --workspace -j 2 -- -D warnings
cargo test --workspace -j 2

npm --prefix ui --workspaces=false ci --foreground-scripts
npm --prefix ui --workspaces=false test -- --maxWorkers=2
npm --prefix ui --workspaces=false run check
```

### Smoke and accessibility tests

- **Accessibility audit (axe-core & keyboard navigation):**
  Unit and component accessibility tests run automatically in Vitest (`ui/src/lib/a11y.test.ts`), verifying zero serious/critical axe violations across all 8 screens in both dark and light themes, modal Escape handling, SegmentedControl arrow navigation, and Toggle keyboard interaction:
  ```powershell
  npm --prefix ui --workspaces=false test -- src/lib/a11y.test.ts
  ```
- **Keyboard navigation E2E (Playwright):**
  Verifies visible focus outlines, Tab / Shift+Tab traversal, and keyboard navigation flows:
  ```powershell
  npx --prefix ui playwright test e2e/keyboard.spec.ts
  ```
- **Smoke tests (Playwright):**
  End-to-end smoke tests cover navigation across all 8 screens, onboarding walkthrough, theme and wallpaper switching, settings adjustments, and offline states:
  ```powershell
  npm --prefix ui --workspaces=false run smoke
  ```
  Or step-by-step:
  ```powershell
  npm --prefix ui --workspaces=false run build
  npx --prefix ui playwright test
  ```

## Key subsystems and features

- **Быстрый запуск (Fast Launch & JVM Prewarm):**
  - **Порог активации:** Выключен по умолчанию (`default_fast_launch()`), включается в Настройки › Лаунчер › «Готовить игру заранее». Управляется настройкой `fast_launch`.
  - **Прогрев JVM:** Предварительный запуск легковесного зонда (`-Xms16m -Xmx32m -version`) для прогрева дискового кэша ОС (`jvm.dll`, системные библиотеки).
  - **Фоновый процесс и аргументы:** Фоновый экземпляр игры запускается со скрытым окном и параметрами:
    `-Dwex.prewarm.signal=<path>`, `-Dwex.prewarm.parentPid=<pid>`, `-Dwex.qa.background=true`, `-Daura.client.hiddenWindow=true`.
  - **Сигнал готовности (Signal Handshake):** При нажатии кнопки «Играть» лаунчер записывает `READY\n` в файл `prewarm.signal` (`activate_prewarm`), что приводит к мгновенному отображению уже инициализированного окна игры (sub-second launch). Если запуск нажат во время инициализации прогрева, лаунчер отображает статус «Игра догружается…», записывает сигнал и дожидается готовности. При отмене или закрытии лаунчера фоновый процесс завершается, а сигнальный файл удаляется.
- **Друзья в реальном времени и уведомления (Live Friends & Notifications):**
  - **Опрос без перезагрузки:** Фоновый поллер (`friendsLive.ts`) периодически опрашивает состояние друзей и приглашений без перерисовки всего интерфейса.
  - **Интервалы обновления:** 25 секунд при активном и видимом окне; 120 секунд при свернутом окне или во время запущенной игры.
  - **Двойной канал уведомлений:** Нативные уведомления Windows через `tauri-plugin-notification`, когда лаунчер не в фокусе (свернут, в трее, перекрыт другими окнами или идет игра); всплывающие Toast-уведомления внутри интерфейса, когда окно активно.
  - **События и переход:** События «<ник> зашёл в игру» (с указанием сервера/мира) и «<ник> приглашает в лобби». Клик по уведомлению разворачивает окно лаунчера и подключается к лобби (аналогично диплинку `wex://join`).
  - **Защита от спама:** Ограничение не более одного уведомления от одного друга в течение 5 минут (`300 000` мс). Настройка в интерфейсе: `friend_notifications` («Уведомления о друзьях», включена по умолчанию).
- **Лобби и мультиплеер (Lobby System):**
  - **Создание комнат:** Настройка публичных, приватных или доступных только друзьям комнат с выбором мира, модпака и описания. Управление хостом (кик игроков, закрытие комнаты).
  - **Пошаговое подключение:** Вертикальный степпер в интерфейсе: сверка манифеста модов (`net_compare_manifests`), докачка недостающих модов, поднятие туннеля и прямой запуск с аргументом `--quickPlayMultiplayer <tunnel_ip:port>`.
  - **Управление туннелем Playit.gg:** Автоматическая загрузка бинарника туннеля, супервизия процесса (`net_tunnel_start`, `net_tunnel_status`, `net_tunnel_stop`), кнопка повторного подключения при сбое (`retry_lobby_tunnel`).
  - **Локальная сеть (LAN):** Поиск и публикация миров через UDP multicast (`224.0.2.60:4445`) с форматом анонса `[MOTD]...[/MOTD][AD]...[/AD]`.
- **Офлайн-режим (Honest Offline Mode):**
  - **Обработка на всех экранах:** Корректные пустые и деградированные состояния на всех 8 экранах лаунчера при отсутствии интернета (баннеры с кнопкой повторной проверки `network.check()`).
  - **Автоматическое переподключение:** Супервизор сети каждые 5 секунд проверяет DNS TCP endpoints. При восстановлении связи экраны автоматически подгружают актуальные данные без перезапуска.
  - **Офлайн-запуск:** Возможность запуска уже установленных версий клиента без сетевого соединения (PlayScreen).
- **Проверка и восстановление файлов (Integrity Verification & Repair):**
  - **Проверка целостности:** Команды `verify_game_files` и `verify_instance_files` сверяют SHA-1 хэши клиентского JAR, библиотек Vanilla/Fabric и ассетов Mojang с официальными индексами.
  - **Восстановление:** Автоматическая докачка только поврежденных или отсутствующих файлов с трансляцией прогресса через событие `storage-verify-progress`. Запуск проверки блокируется, если игра запущена.
  - **Очистка кэша:** `cleanup_cache` удаляет временные загрузки и устаревшие ассеты, не затрагивая конфигурацию и миры игрока.
- **Раздел «О системе» (System Information & Telemetry):**
  - **Диагностика оборудования:** Сбор данных в `wex_core::system_info` (модель и ядра CPU; дискретная графика Intel Arc, Nvidia, AMD Radeon против встроенной; общий и свободный объем RAM; версия и сборка Windows; свободное место на диске с предупреждениями при < 5 ГБ и < 20 ГБ; обнаруженные среды Java в Wex и PATH).
  - **Защита приватности:** Автоматическая санитизация путей (`sanitize_user_path`), заменяющая `C:\Users\<username>` на обезличенный `C:\Users\<user>`.
  - **Интерфейс:** Группа 7 в «Настройках» с предупреждающими плашками и кнопкой «Скопировать для поддержки», копирующей диагностический отчет в буфер обмена.
- **Безопасное самообновление (Secure Self-Updater):**
  - **Криптографическая проверка:** Проверка цифровой подписи обновлений алгоритмом Ed25519 (`minisign-verify`).
  - **Защита ключа:** В релизных сборках доверенный публичный ключ извлекается исключительно из конфигурации сборки (`resolve_trusted_public_key`). Переменные окружения (`WEX_UPDATER_PUBKEY`) в релизе строго игнорируются для защиты от подмены.
  - **Фолбэк без ключа:** Если публичный ключ не настроен или сборка обновления не подписана, автоматическая установка блокируется. Лаунчер переходит в режим уведомления с возможностью открыть прямую ссылку на скачивание (`updater_open_download_url`).
  - **Блокировка во время игры:** Установка обновления невозможна при запущенном процессе клиента.
- **Окно и системный трей (Window State & Tray):**
  - **Сохранение геометрии:** Координаты и размеры окна (`WindowStateDto`) сохраняются и валидируются относительно активных мониторов (`sanitize_window_position`), исключая появление окна за пределами видимой области экрана.
  - **Системный трей:** Опция «Сворачивать в трей» (`minimize_to_tray`, включена по умолчанию в Windows). Иконка в трее с меню («Показать», «Остановить игру», «Выйти») и восстановление по клику.
  - **Поведение окна при запуске игры (`GameWindowBehavior`):**
    - `hide` («Скрыть в трей»): окно полностью скрывается в трей на время игры и автоматически восстанавливается после закрытия.
    - `minimize` («Свернуть»): сворачивание на панель задач.
    - `keep` («Не закрывать»): лаунчер остается на экране.

## Build and run

Install UI dependencies and build the frontend:

```powershell
npm --prefix ui --workspaces=false ci --foreground-scripts
npm --prefix ui --workspaces=false run build
```

Run the desktop app in development mode (requires Tauri platform prerequisites):

```powershell
npm --prefix ui --workspaces=false ci --foreground-scripts
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
