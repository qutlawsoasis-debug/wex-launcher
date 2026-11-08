# IPC Wex: зарегистрированные команды, DTO и события

Снимок: **08.11.2026, main afb981a**. Источник доступных вызовов — [`tauri::generate_handler!`](../app/src-tauri/src/lib.rs), а не только константы `wex-ipc`. Ниже перечислены **все 101 зарегистрированная команда**, включая 4 обработчика серверов в lib.rs. Новые функции в файле commands ещё не доступны UI, пока не добавлены в этот реестр. Схема потоков и владение состоянием: [ARCHITECTURE.md](ARCHITECTURE.md).

## Формат вызова и ответа

`invoke<T>(command, args)` возвращает Promise: Rust `Ok(T)` resolve-ится непосредственно в T, `Err(E)` reject-ится сериализованным E. Возврата `{ Ok: ... } | { Err: ... }` на границе Tauri нет, хотя такой alias присутствует в generated/error.ts. `()` соответствует JSON null. `Option<T>` допускает null; optional top-level аргументы помечены `?`.

В таблицах используются Rust-имена типов: String = JSON string, bool = boolean, Vec<T> = массив, Option<T> = T|null, целые/float = JSON number. Не передавайте JS bigint в JSON invoke; для u64/usize учитывайте предел точных JS integers. Generated TS может представлять u64 как bigint — это декларация генератора, не переключение serde_json на native BigInt.

**Имена top-level аргументов — camelCase по умолчанию Tauri** (`new_config` → `newConfig`, `target_friend_code` → `targetFriendCode`). Вложенные serde DTO сохраняют snake_case: `{ request: { target_friend_code: "...", auth: ... } }`. В текущих обработчиках нет rename_all override. Правило подтверждено [официальной документацией Tauri](https://v2.tauri.app/develop/calling-rust/#passing-arguments). State, AppHandle и Window внедряет Tauri; UI их не отправляет.

`request?: ...` у друзей — альтернатива плоским аргументам, не обёртка для любого вызова. При наличии request он имеет приоритет; где указано, auth затем берётся из FriendsState. Для request-DTO все поля и enum-варианты приведены в приложении.

## Ошибки

Колонка «Ошибка» содержит реальный тип rejection. `String` не имеет обязательного машинного кода: текст может быть локализованным, ошибкой IO/HTTP или Display от IpcError. Не разбирайте его как объект IpcError. `IpcError` — `{ code: String, message: String, technical: Option<String> }`; показывайте message, technical используйте для диагностики. `—` означает обычный, не Result, ответ. Ошибки десериализации входных аргументов/отсутствие команды/ошибки транспорта Tauri возможны до входа в любой обработчик.

Зарегистрированные structured-error группы: друзья/достижения и низкоуровневые net_lobby/net_tunnel. `INVALID_ARGUMENT`, `NOT_FOUND`, `INTERNAL_ERROR`, `TIMEOUT`; IO-коды `IO_NOT_FOUND`, `IO_PERMISSION_DENIED`, `IO_ALREADY_EXISTS`, `IO_TIMED_OUT`, `IO_ERROR`; JSON → `SERIALIZATION_ERROR`.

| Конвертер shared IpcError | Коды |
|---|---|
| LobbyError | NET_LOBBY_NETWORK_ERROR; HTTP 404 → NET_LOBBY_NOT_FOUND; 401/403 → NET_LOBBY_UNAUTHORIZED; прочее HTTP → NET_LOBBY_API_ERROR; NET_LOBBY_JSON_ERROR; NET_LOBBY_URL_ERROR |
| FriendError | SOCIAL_NETWORK_ERROR, SOCIAL_UNAUTHORIZED, SOCIAL_NOT_FOUND, SOCIAL_API_ERROR, SOCIAL_JSON_ERROR |
| AchievementError | ACHIEVEMENT_IO_ERROR, ACHIEVEMENT_SERIALIZE_ERROR |
| WorkshopError | WORKSHOP_NOT_FOUND, WORKSHOP_IO_ERROR, WORKSHOP_ZIP_ERROR, WORKSHOP_INVALID_LEVEL_DAT |
| UpdateError | UPDATE_NETWORK_ERROR, UPDATE_INVALID_MANIFEST, UPDATE_PLATFORM_NOT_FOUND, UPDATE_SIGNATURE_FAILED |
| SkinError | SKIN_IMAGE_ERROR, SKIN_INVALID_PNG, SKIN_UNSUPPORTED_DIMENSIONS, SKIN_FILE_TOO_LARGE, IO_ERROR, SERIALIZATION_ERROR |

Конвертеры определены в [`error.rs`](../core/wex-ipc/src/error.rs); наличие кода в этой таблице не означает, что конкретный String-handler его сохраняет. Generated ERROR_CODES сейчас не перечисляет SKIN_*; свойство code в IpcError остаётся String. Успешный best-effort вызов также не доказывает, что все побочные операции удались: подавление ошибок отмечено явно.

## Реестр команд

### launch (9)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`get_config`](../app/src-tauri/src/commands/launch.rs#L228) | `{}` | `LauncherConfig` | `String` | Узкий shared LauncherConfig: ram_gb, не ram_mb; чтение core config. |
| [`save_config`](../app/src-tauri/src/commands/launch.rs#L234) | `{ config: LauncherConfig }` | `()` | `String` | Сохраняет узкий shared LauncherConfig; set_config принимает newConfig. Ошибки core превращаются в строку. |
| [`get_installed_versions`](../app/src-tauri/src/commands/launch.rs#L247) | `{}` | `Vec<InstalledVersion>` | `String` | Shared InstalledVersion без required_java; обе версии 1.20.1/1.21.1. |
| [`start_game`](../app/src-tauri/src/commands/launch.rs#L259) | `{ version: String, nickname: String, jvmArgs?: Vec<String> }` | `LaunchGameResponse` | `String` | Ожидает подготовку/запуск; duplicate-start, offline-uninstalled и ошибки установки/Java/процесса отклоняют Promise; после запуска включает наблюдение за завершением. |
| [`cancel_launch`](../app/src-tauri/src/commands/launch.rs#L480) | `{}` | `()` | `String` | Запрашивает отмену и посылает cancelled; ошибка core.cancel_launch подавлена, Ok не доказывает остановку процесса. |
| [`is_game_running`](../app/src-tauri/src/commands/launch.rs#L500) | `{}` | `bool` | `String` | Флаг PlayState, не независимая проверка процесса; сейчас всегда Ok. |
| [`kill_game`](../app/src-tauri/src/commands/launch.rs#L506) | `{ version?: String }` | `()` | `String` | Версия по аргументу либо running_version; ошибка core.kill_game подавлена, флаги сбрасываются; сейчас всегда Ok. |
| [`launch_game`](../app/src-tauri/src/commands/launch.rs#L531) | `{ version: String, nickname: String }` | `String` | `String` | Адаптер start_game без jvmArgs; возвращает только message, не LaunchGameResponse. |
| [`set_config`](../app/src-tauri/src/commands/launch.rs#L543) | `{ newConfig: LauncherConfig }` | `()` | `String` | Сохраняет узкий shared LauncherConfig; set_config принимает newConfig. Ошибки core превращаются в строку. |

### settings (9)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`get_launcher_settings`](../app/src-tauri/src/commands/settings.rs#L239) | `{}` | `LauncherSettingsDto` | `String` | Полный LauncherSettingsDto; load_or_default, telemetry/Java detection; сейчас всегда Ok. Не shared LauncherConfig. |
| [`save_launcher_settings`](../app/src-tauri/src/commands/settings.rs#L246) | `{ newSettings: LauncherSettingsDto }` | `()` | `String` | Полный DTO; RAM минимум 1024 MB, запись config; disk failure → String; apply_preset/core.save_config best-effort. game_dir из DTO не переназначает WexPaths. |
| [`reset_settings`](../app/src-tauri/src/commands/settings.rs#L254) | `{}` | `LauncherSettingsDto` | `String` | Пишет CoreConfig::default, возвращает полный DTO; disk failure → String, sync core best-effort. |
| [`open_game_data_dir`](../app/src-tauri/src/commands/settings.rs#L259) | `{}` | `()` | `String` | Открывает корень Wex в системном проводнике; запуск внешней программы может вернуть String. |
| [`get_data_dir`](../app/src-tauri/src/commands/settings.rs#L289) | `{}` | `String` | `—` | Строка корня WexPaths; не Result. |
| [`get_system_info`](../app/src-tauri/src/commands/settings.rs#L294) | `{}` | `SystemInfo` | `String` | App-local SystemInfo (MB/ОС/версия/путь); сейчас всегда Ok. |
| [`get_launcher_config`](../app/src-tauri/src/commands/settings.rs#L308) | `{}` | `LauncherSettingsDto` | `String` | Полный LauncherSettingsDto; load_or_default, telemetry/Java detection; сейчас всегда Ok. Не shared LauncherConfig. |
| [`save_launcher_config`](../app/src-tauri/src/commands/settings.rs#L315) | `{ newConfig: LauncherSettingsDto }` | `()` | `String` | Полный DTO; RAM минимум 1024 MB, запись config; disk failure → String; apply_preset/core.save_config best-effort. game_dir из DTO не переназначает WexPaths. |
| [`open_data_dir`](../app/src-tauri/src/commands/settings.rs#L323) | `{}` | `()` | `String` | Открывает корень Wex в системном проводнике; запуск внешней программы может вернуть String. |

### network (1)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`get_network_status`](../app/src-tauri/src/commands/network.rs#L156) | `{}` | `NetworkStatusPayload` | `String` | Текущий общий supervisor-флаг; сейчас всегда Ok. Полезная нагрузка только is_online, не Shell NetworkStatusDto. |

### shell (7)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`shell_check_network`](../app/src-tauri/src/commands/shell.rs#L113) | `{}` | `NetworkStatusDto` | `String` | Отдельная активная TCP-проба в spawn_blocking; offline — успешный DTO, ошибка JoinHandle → String. |
| [`shell_get_profile`](../app/src-tauri/src/commands/shell.rs#L137) | `{ nickname: String }` | `ShellProfileDto` | `String` | Имя/initials/статус и путь/URL головы; сбой сетевого worker превращается в offline, сейчас всегда Ok. |
| [`shell_get_launch_state`](../app/src-tauri/src/commands/shell.rs#L159) | `{}` | `ShellLaunchStateDto` | `String` | Снимок флагов и running_version, client_status; сейчас всегда Ok. |
| [`shell_stop_game`](../app/src-tauri/src/commands/shell.rs#L184) | `{ version?: String }` | `()` | `String` | Версия по аргументу либо running_version; ошибка core.kill_game подавлена, флаги сбрасываются; сейчас всегда Ok. |
| [`shell_minimize_window`](../app/src-tauri/src/commands/shell.rs#L210) | `{}` | `()` | `String` | Операция над текущим injected Window; ошибки window API → String. |
| [`shell_toggle_maximize_window`](../app/src-tauri/src/commands/shell.rs#L216) | `{}` | `bool` | `String` | Переключает maximized, возвращает новое bool; ошибки window API → String. |
| [`shell_close_window`](../app/src-tauri/src/commands/shell.rs#L229) | `{}` | `()` | `String` | Операция над текущим injected Window; ошибки window API → String. |

### servers (4)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`list_servers`](../app/src-tauri/src/lib.rs#L53) | `{}` | `Vec<ServerEntryModel>` | `String` | Копия in-memory AppState.servers; строковая ошибка poisoned mutex. Не загрузка списка из экземпляра. |
| [`ping_server`](../app/src-tauri/src/lib.rs#L62) | `{ address: String }` | `ServerPingResult` | `String` | SLP ping 2500/2500 ms, protocol 763; сеть/DNS/таймаут/протокол → String. |
| [`add_server`](../app/src-tauri/src/lib.rs#L78) | `{ name: String, address: String }` | `ServerEntryModel` | `String` | Обновляет список в памяти; ошибки mutex возвращаются, сохранение root servers.dat best-effort, ошибки записи подавлены. |
| [`remove_server`](../app/src-tauri/src/lib.rs#L136) | `{ address: String }` | `()` | `String` | Обновляет список в памяти; ошибки mutex возвращаются, сохранение root servers.dat best-effort, ошибки записи подавлены. |

### mods (5)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`list_mods`](../app/src-tauri/src/commands/mods.rs#L387) | `{ version: String }` | `Vec<ModEntry>` | `String` | Читает mods версии и metadata JAR; IO → String, отсутствующая metadata использует filename fallback. |
| [`toggle_mod`](../app/src-tauri/src/commands/mods.rs#L395) | `{ version: String, fileName: String, enabled: bool }` | `ModEntry` | `String` | Включает/выключает через .jar/.jar.disabled; missing/already-enabled/already-disabled и IO → String. |
| [`add_custom_mod`](../app/src-tauri/src/commands/mods.rs#L405) | `{ version: String, sourcePath: String }` | `ModEntry` | `String` | Копирует sourcePath; только .jar; missing/invalid filename/IO → String. |
| [`delete_custom_mod`](../app/src-tauri/src/commands/mods.rs#L414) | `{ version: String, fileName: String }` | `()` | `String` | Удаляет пользовательский JAR; invalid name/missing/protected pack/IO → String. |
| [`open_mods_dir`](../app/src-tauri/src/commands/mods.rs#L423) | `{ version: String }` | `()` | `String` | Создаёт/открывает mods; IO или ошибка запуска проводника → String. |

### overview (7)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`social_get_news`](../app/src-tauri/src/commands/overview.rs#L248) | `{ request?: SocialGetNewsRequest }` | `SocialGetNewsResponse` | `String` | Возвращает объект {versions}, cached/bundled news; request.cache_dir может переопределить кеш; сейчас всегда Ok. |
| [`overview_get_news`](../app/src-tauri/src/commands/overview.rs#L257) | `{}` | `Vec<ReleaseNoteVersionDto>` | `String` | Те же новости без request, возвращает непосредственно массив версий; сейчас всегда Ok. |
| [`social_check_launcher_update`](../app/src-tauri/src/commands/overview.rs#L264) | `{ request?: SocialCheckLauncherUpdateRequest }` | `UpdateCheckResultDto` | `String` | HTTP/JSON/update errors → String при заданном URL; без URL возвращает update_available=false, не выполняя удалённую проверку; overview использует current_version=1.0.0. |
| [`overview_check_update`](../app/src-tauri/src/commands/overview.rs#L279) | `{ manifestUrl?: String }` | `UpdateCheckResultDto` | `String` | HTTP/JSON/update errors → String при заданном URL; без URL возвращает update_available=false, не выполняя удалённую проверку; overview использует current_version=1.0.0. |
| [`extras_detect_old_launcher`](../app/src-tauri/src/commands/overview.rs#L292) | `{}` | `Option<String>` | `String` | Определяет существующий старый каталог; отсутствие → null, сейчас всегда Ok. Сам перенос не выполняет. |
| [`extras_import_old_launcher`](../app/src-tauri/src/commands/overview.rs#L297) | `{ request: ExtrasImportOldLauncherRequest }` | `OldLauncherDataDto` | `String` | Читает явный old_launcher_dir; ошибки чтения/импорта → String, не применяет выбранные настройки. |
| [`extras_apply_import`](../app/src-tauri/src/commands/overview.rs#L304) | `{ request: ApplyImportRequest }` | `ApplyImportResponse` | `String` | Выборочный перенос в Wex; config failure → String; результат сообщает числа реально импортированных серверов/скинов. Входные flags с serde(default). |

### friends (11)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`social_login`](../app/src-tauri/src/commands/friends.rs#L63) | `{ request?: SocialLoginRequest, auth?: UserAuthDto }` | `FriendProfileDto` | `IpcError` | request.auth > auth; без auth INVALID_ARGUMENT; profile HTTP failure даёт локальный fallback, auth хранится в FriendsState. |
| [`social_get_friends`](../app/src-tauri/src/commands/friends.rs#L101) | `{ request?: SocialGetFriendsRequest, auth?: UserAuthDto }` | `SocialGetFriendsResponse` | `IpcError` | request.auth > auth > state auth; remote failure/no auth возвращает локальный кеш, сейчас нет ветки Err. sync обновляет friends/requests/invites. |
| [`social_send_friend_request`](../app/src-tauri/src/commands/friends.rs#L131) | `{ request?: SocialSendFriendRequestRequest, auth?: UserAuthDto, targetFriendCode?: String }` | `()` | `IpcError` | request либо auth/state + targetFriendCode; missing/empty → INVALID_ARGUMENT; API ошибки → SOCIAL_*; успех null. |
| [`social_accept_friend_request`](../app/src-tauri/src/commands/friends.rs#L165) | `{ request?: SocialAcceptFriendRequestRequest, auth?: UserAuthDto, requestId?: String }` | `()` | `IpcError` | request либо auth/state + requestId; missing → INVALID_ARGUMENT, API → SOCIAL_*; меняет локальный кеш. |
| [`social_decline_friend_request`](../app/src-tauri/src/commands/friends.rs#L212) | `{ request?: SocialDeclineFriendRequestRequest, auth?: UserAuthDto, requestId?: String }` | `()` | `IpcError` | request либо auth/state + requestId; missing → INVALID_ARGUMENT, API → SOCIAL_*; меняет локальный кеш. |
| [`social_sync`](../app/src-tauri/src/commands/friends.rs#L244) | `{ request?: SocialSyncRequest, auth?: UserAuthDto }` | `SyncResponseDto` | `IpcError` | request.auth > auth > state auth; remote failure/no auth возвращает локальный кеш, сейчас нет ветки Err. sync обновляет friends/requests/invites. |
| [`social_send_lobby_invite`](../app/src-tauri/src/commands/friends.rs#L281) | `{ request?: SocialSendLobbyInviteRequest, auth?: UserAuthDto, targetUserId?: String, lobbyCode?: String, hostName?: String }` | `()` | `IpcError` | request либо auth/state + targetUserId/lobbyCode; missing/empty → INVALID_ARGUMENT. hostName игнорируется; ошибка remote send подавлена. |
| [`social_respond_lobby_invite`](../app/src-tauri/src/commands/friends.rs#L316) | `{ inviteId: String, accept: bool, auth?: UserAuthDto }` | `Option<String>` | `IpcError` | auth либо state; missing → INVALID_ARGUMENT, API → SOCIAL_*; удаляет invite из кеша, возвращает code|null, не входит в лобби автоматически. |
| [`social_set_presence`](../app/src-tauri/src/commands/friends.rs#L341) | `{ request?: SocialSetPresenceRequest, auth?: UserAuthDto, presence?: UserPresenceDto }` | `()` | `IpcError` | request либо auth/state + presence (по умолчанию все поля null); missing auth → INVALID_ARGUMENT; remote sync error подавлена. |
| [`social_list_achievements`](../app/src-tauri/src/commands/friends.rs#L376) | `{ request?: SocialListAchievementsRequest, dataDir?: String }` | `SocialListAchievementsResponse` | `IpcError` | request.data_dir > dataDir > root; читает локальные definitions/progress; ACHIEVEMENT_IO_ERROR/SERIALIZE_ERROR. |
| [`social_report_achievement_progress`](../app/src-tauri/src/commands/friends.rs#L423) | `{ request?: SocialReportAchievementProgressRequest, dataDir?: String, achievementId?: String, increment?: u32 }` | `SocialReportAchievementProgressResponse` | `IpcError` | request либо dataDir/root + achievementId + increment(default 1); INVALID_ARGUMENT/NOT_FOUND/ACHIEVEMENT_*; при unlock посылает social-achievement-toast. |

### lobby (20)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`net_lobby_create`](../app/src-tauri/src/commands/lobby.rs#L226) | `{ request: NetLobbyCreateRequest }` | `LobbyCreateResponseDto` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_join`](../app/src-tauri/src/commands/lobby.rs#L250) | `{ request: NetLobbyJoinRequest }` | `LobbyJoinResponseDto` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_open`](../app/src-tauri/src/commands/lobby.rs#L274) | `{ request: NetLobbyOpenRequest }` | `bool` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_status`](../app/src-tauri/src/commands/lobby.rs#L292) | `{ request: NetLobbyStatusRequest }` | `LobbyStatusResponseDto` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_heartbeat`](../app/src-tauri/src/commands/lobby.rs#L305) | `{ request: NetLobbyHeartbeatRequest }` | `bool` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_close`](../app/src-tauri/src/commands/lobby.rs#L318) | `{ request: NetLobbyCloseRequest }` | `bool` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_leave`](../app/src-tauri/src/commands/lobby.rs#L341) | `{ request: NetLobbyLeaveRequest }` | `bool` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_kick`](../app/src-tauri/src/commands/lobby.rs#L356) | `{ request: NetLobbyKickRequest }` | `bool` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_tunnel_config`](../app/src-tauri/src/commands/lobby.rs#L369) | `{}` | `TunnelConfigResponseDto` | `IpcError` | Typed LobbyApiClient: NET_LOBBY_NETWORK_ERROR/API_ERROR/NOT_FOUND/UNAUTHORIZED/JSON_ERROR/URL_ERROR. Request-поля snake_case. close останавливает heartbeat/tunnel и очищает session; leave очищает session. |
| [`net_lobby_list_open_rooms`](../app/src-tauri/src/commands/lobby.rs#L381) | `{ request: NetListOpenRoomsRequest }` | `NetListOpenRoomsResponse` | `IpcError` | Legacy cloud backend: INTERNAL_ERROR при создании клиента, NET_LOBBY_API_ERROR при сети/HTTP, NET_LOBBY_JSON_ERROR при неверном JSON; не возвращает фиктивный пустой список вместо ошибки. |
| [`net_tunnel_ensure_binary`](../app/src-tauri/src/commands/lobby.rs#L389) | `{ request: NetTunnelEnsureBinaryRequest }` | `NetTunnelEnsureBinaryResponse` | `IpcError` | Скачивает/проверяет playit в destination_path; INTERNAL_ERROR при сбое; verified=true только после успешной проверки. |
| [`net_tunnel_start`](../app/src-tauri/src/commands/lobby.rs#L406) | `{ request: NetTunnelStartRequest }` | `NetTunnelStartResponse` | `IpcError` | Запускает supervisor; request.secret_key интерпретируется как путь к файлу ключа, не inline secret; INTERNAL_ERROR при запуске. Сейчас результат pid=0 — placeholder, не OS PID. |
| [`net_tunnel_stop`](../app/src-tauri/src/commands/lobby.rs#L422) | `{}` | `()` | `IpcError` | Останавливает managed supervisor при наличии; сейчас всегда Ok. |
| [`net_tunnel_status`](../app/src-tauri/src/commands/lobby.rs#L431) | `{}` | `TunnelStatusDto` | `IpcError` | Снимок current_tunnel_status; сейчас всегда Ok. Не подписывается на NetEventBus, статус не гарантированно свежий. |
| [`create_lobby_room`](../app/src-tauri/src/commands/lobby.rs#L441) | `{ worldName: String, playerName?: String }` | `LobbyRoomModel` | `String` | High-level создание session/heartbeat через LobbyApiClient; ошибки сети/API форматируются в String; playerName по умолчанию Player. |
| [`join_lobby_room`](../app/src-tauri/src/commands/lobby.rs#L525) | `{ code: String, playerName?: String }` | `LobbyRoomModel` | `String` | High-level join; пустой/некорректный code и join failure → String; сохраняет session; playerName по умолчанию Player. |
| [`get_lobby_status`](../app/src-tauri/src/commands/lobby.rs#L608) | `{ code: String, playerName?: String }` | `Option<LobbyRoomModel>` | `String` | High-level статус; remote error → Ok(null); kicked → String; playerName/session fallback. Не равнозначен typed net_lobby_status. |
| [`close_lobby_room`](../app/src-tauri/src/commands/lobby.rs#L684) | `{ code: String }` | `()` | `String` | High-level close; завершает session/heartbeat/tunnel; API failure → String. |
| [`kick_lobby_player`](../app/src-tauri/src/commands/lobby.rs#L716) | `{ code: String, playerName: String }` | `bool` | `String` | Требует host session; не host → String, API failure → String; успешный bool. |
| [`list_open_rooms`](../app/src-tauri/src/commands/lobby.rs#L741) | `{ playerName?: String }` | `Vec<NetOpenRoomDto>` | `String` | High-level cloud list; fetch_open_rooms IpcError сужается до e.message (String); playerName optional. |

### wardrobe (8)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`extras_validate_skin`](../app/src-tauri/src/commands/wardrobe.rs#L72) | `{ request: ExtrasValidateSkinRequest }` | `SkinValidationResultDto` | `String` | Base64 failure → String; PNG/dimensions validation возвращается SkinValidationResultDto, в том числе valid=false. |
| [`extras_convert_skin`](../app/src-tauri/src/commands/wardrobe.rs#L84) | `{ request: ExtrasConvertSkinRequest }` | `ExtrasConvertSkinResponse` | `String` | Base64/PNG/размер/encode failure → String; legacy 64×32 → 64×64, результат PNG base64. |
| [`extras_save_skin`](../app/src-tauri/src/commands/wardrobe.rs#L104) | `{ request: ExtrasSaveSkinRequest }` | `WardrobeItemDto` | `String` | Base64/skin/IO/history failure → String; явные data_dir/name/model; результат запись истории. |
| [`extras_list_skin_history`](../app/src-tauri/src/commands/wardrobe.rs#L118) | `{ request: ExtrasListSkinHistoryRequest }` | `ExtrasListSkinHistoryResponse` | `String` | История из request.data_dir; ошибка чтения/JSON → String. |
| [`wardrobe_delete_history`](../app/src-tauri/src/commands/wardrobe.rs#L130) | `{ request: WardrobeDeleteHistoryRequest }` | `WardrobeDeleteHistoryResponse` | `String` | Удаляет id из указанной истории; IO/JSON → String; результат DTO с success. |
| [`extras_generate_csl_config`](../app/src-tauri/src/commands/wardrobe.rs#L142) | `{ request: ExtrasGenerateCslConfigRequest }` | `()` | `String` | Генерирует CSL JSON в явном game_dir; JSON/запись → String. |
| [`wardrobe_get_state`](../app/src-tauri/src/commands/wardrobe.rs#L184) | `{}` | `WardrobeStateDto` | `String` | Читает root wardrobe.json; missing/invalid JSON → default WardrobeStateDto, сейчас всегда Ok. |
| [`wardrobe_apply`](../app/src-tauri/src/commands/wardrobe.rs#L207) | `{ request: WardrobeApplyRequest }` | `WardrobeApplyResponse` | `String` | Применяет skin/cape и CSL к обоим инстансам, сохраняет wardrobe.json; base64/JSON/запись → String; часть файловых операций best-effort. |

### workshop (12)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`workshop_list_worlds`](../app/src-tauri/src/commands/workshop.rs#L23) | `{ version?: String }` | `Vec<WorldItemDto>` | `String` | Миры/NBT → String при ошибке; workshop возвращает массив, social — {worlds}; social непустой request.game_dir имеет приоритет над version. |
| [`social_list_worlds`](../app/src-tauri/src/commands/workshop.rs#L34) | `{ request?: SocialListWorldsRequest, version?: String }` | `SocialListWorldsResponse` | `String` | Миры/NBT → String при ошибке; workshop возвращает массив, social — {worlds}; social непустой request.game_dir имеет приоритет над version. |
| [`workshop_create_world_backup`](../app/src-tauri/src/commands/workshop.rs#L51) | `{ worldFolder: String, version?: String }` | `WorldBackupItemDto` | `String` | worldFolder — путь к исходному миру; ZIP кладётся в instance(version)/backups; missing/IO/ZIP → String. |
| [`social_create_world_backup`](../app/src-tauri/src/commands/workshop.rs#L84) | `{ request: SocialCreateWorldBackupRequest }` | `WorldBackupItemDto` | `String` | Явные request.world_dir/backups_dir; missing/IO/ZIP → String; не использует WexPaths для назначения. |
| [`workshop_list_world_backups`](../app/src-tauri/src/commands/workshop.rs#L115) | `{ version?: String, worldFolderName?: String, displayName?: String }` | `Vec<WorldBackupItemDto>` | `String` | instance(version)/backups, optional worldFolderName/displayName; IO/ZIP metadata errors → String. |
| [`social_list_world_backups`](../app/src-tauri/src/commands/workshop.rs#L132) | `{ request: SocialListWorldBackupsRequest }` | `SocialListWorldBackupsResponse` | `String` | Явный request.backups_dir; возвращает {backups}; IO → String. |
| [`workshop_restore_world_backup`](../app/src-tauri/src/commands/workshop.rs#L146) | `{ backupPath: String, targetWorldFolder?: String, version?: String }` | `String` | `String` | Явный backupPath; targetWorldFolder либо saves/<stem до _>; возвращает путь; missing/IO/ZIP → String. |
| [`social_restore_world_backup`](../app/src-tauri/src/commands/workshop.rs#L172) | `{ request: SocialRestoreWorldBackupRequest }` | `SocialRestoreWorldBackupResponse` | `String` | request.backup_path + saves_dir; target из stem до _; возвращает {restored_world_dir}; missing/IO/ZIP → String. |
| [`workshop_delete_world`](../app/src-tauri/src/commands/workshop.rs#L193) | `{ worldFolder: String }` | `()` | `String` | worldFolder — явный путь, не id; permanent delete; missing/IO → String. |
| [`workshop_list_screenshots`](../app/src-tauri/src/commands/workshop.rs#L203) | `{ version?: String }` | `Vec<ScreenshotItemDto>` | `String` | Список PNG metadata; IO → String. workshop — массив; social — {screenshots}, непустой request.screenshots_dir имеет приоритет. |
| [`social_list_screenshots`](../app/src-tauri/src/commands/workshop.rs#L215) | `{ request?: SocialListScreenshotsRequest, version?: String }` | `SocialListScreenshotsResponse` | `String` | Список PNG metadata; IO → String. workshop — массив; social — {screenshots}, непустой request.screenshots_dir имеет приоритет. |
| [`workshop_open_path`](../app/src-tauri/src/commands/workshop.rs#L235) | `{ path: String }` | `()` | `String` | Открывает/reveal явный путь через системную программу; missing/OS launch → String. |

### workshop_packs (8)

| Команда / источник | Аргументы invoke | Успешный результат | Ошибка | Поведение и ограничения |
|---|---|---|---|---|
| [`workshop_list_resource_packs`](../app/src-tauri/src/commands/workshop_packs.rs#L339) | `{ version?: String }` | `Vec<ResourcePackItemDto>` | `String` | Перечисляет файлы/каталоги и metadata/icon; отсутствующая папка → пустой массив, read_dir/IO → String; неизвестная metadata допускается. |
| [`workshop_toggle_resource_pack`](../app/src-tauri/src/commands/workshop_packs.rs#L422) | `{ fileName: String, enabled: bool, version?: String }` | `()` | `String` | Меняет options.txt resourcePacks; fileName включая имя ZIP; IO/разбор → String. |
| [`workshop_set_resource_packs_order`](../app/src-tauri/src/commands/workshop_packs.rs#L435) | `{ packNames: Vec<String>, version?: String }` | `()` | `String` | Перезаписывает порядок packNames в options.txt; IO → String. |
| [`workshop_list_shader_packs`](../app/src-tauri/src/commands/workshop_packs.rs#L496) | `{ version?: String }` | `Vec<ShaderPackItemDto>` | `String` | Перечисляет файлы/каталоги и metadata/icon; отсутствующая папка → пустой массив, read_dir/IO → String; неизвестная metadata допускается. |
| [`workshop_set_active_shader_pack`](../app/src-tauri/src/commands/workshop_packs.rs#L573) | `{ shaderFileName?: String, version?: String }` | `()` | `String` | Меняет Iris properties; shaderFileName=null отключает шейдеры; IO → String. |
| [`workshop_add_pack_file`](../app/src-tauri/src/commands/workshop_packs.rs#L615) | `{ sourcePath: String, packType: String, version?: String }` | `String` | `String` | Копирует sourcePath; packType=shaderpack → shaderpacks, остальные значения → resourcepacks; missing/name/IO → String, возвращает filename. Само расширение ZIP строго не проверяется. |
| [`workshop_delete_pack_file`](../app/src-tauri/src/commands/workshop_packs.rs#L668) | `{ fileName: String, packType: String, version?: String }` | `()` | `String` | Protected official pack/missing/IO → String; удаляет файл/каталог; disable/update options после удаления best-effort. |
| [`workshop_open_packs_folder`](../app/src-tauri/src/commands/workshop_packs.rs#L679) | `{ packType: String, version?: String }` | `()` | `String` | packType=shaderpack либо resourcepacks fallback; создание папки best-effort, ошибка reveal → String. |

## Имена shared-контракта без Tauri-обработчика

В commands.rs объявлено 60 CMD-констант; **20 имён сейчас не зарегистрированы**. Нельзя вызывать их только потому, что они присутствуют в generated COMMANDS. Типы request/response остаются общим контрактом для библиотек/будущих адаптеров.

Типы ниже описывают общие модели/trait, **не рабочую форму invoke**. Для каждого имени транспорт сейчас отвергнет вызов как незарегистрированную команду; runtime-ошибки будущего адаптера не определены. У core-методов trait использует IpcError. Для остальных shared-константа и DTO не задают обязательное соответствие request/result — указана доступная связанная модель.

| Константа | Незарегистрированное имя | Вход shared-контракта | Результат / связанная модель |
|---|---|---|---|
| `CMD_CORE_GET_CONFIG` | `core_get_config` | `()` | `LauncherConfig` (trait) |
| `CMD_CORE_SAVE_CONFIG` | `core_save_config` | `LauncherConfig` | `()` (trait) |
| `CMD_CORE_GET_INSTALLED_VERSIONS` | `core_get_installed_versions` | `()` | `Vec<InstalledVersion>` (trait) |
| `CMD_CORE_INSTALL_INSTANCE` | `core_install_instance` | `InstallInstanceRequest` | `InstallInstanceResponse` (trait) |
| `CMD_CORE_LAUNCH_GAME` | `core_launch_game` | `LaunchGameRequest` | `LaunchGameResponse` (trait) |
| `CMD_CORE_CANCEL_LAUNCH` | `core_cancel_launch` | `()` | `()` (trait) |
| `CMD_CORE_IS_RUNNING` | `core_is_running` | `IsRunningRequest`; trait принимает instance_id | Trait возвращает bool; отдельно объявлен IsRunningResponse, не используемый этим trait |
| `CMD_CORE_KILL_GAME` | `core_kill_game` | `KillGameRequest`; trait принимает instance_id | `()` (trait) |
| `CMD_NET_PING_SERVER` | `net_ping_server` | `NetPingRequest` | `ServerPingResponseDto` |
| `CMD_NET_RESOLVE_ADDRESS` | `net_resolve_address` | `NetResolveRequest` | `NetResolveResponse` |
| `CMD_NET_LOAD_SERVERS` | `net_load_servers` | `NetLoadServersRequest` | `NetLoadServersResponse` |
| `CMD_NET_SAVE_SERVERS` | `net_save_servers` | `NetSaveServersRequest` | Отдельный response DTO не объявлен |
| `CMD_NET_MERGE_SERVERS` | `net_merge_servers` | `NetMergeServersRequest` | `NetMergeServersResponse` |
| `CMD_NET_COMPARE_MANIFESTS` | `net_compare_manifests` | `NetCompareManifestsRequest` | `NetCompareManifestsResponse` |
| `CMD_NET_COMPUTE_MANIFEST_HASH` | `net_compute_manifest_hash` | `NetComputeManifestHashRequest` | `NetComputeManifestHashResponse` |
| `CMD_EXTRAS_ANALYZE_CRASH` | `extras_analyze_crash` | `ExtrasAnalyzeCrashRequest` | `CrashDiagnosisDto` |
| `CMD_EXTRAS_DISCORD_SET_ACTIVITY` | `extras_discord_set_activity` | `ExtrasDiscordSetActivityRequest` | Отдельный response DTO не объявлен |
| `CMD_EXTRAS_DISCORD_CLEAR_ACTIVITY` | `extras_discord_clear_activity` | Отдельный request DTO не объявлен | Отдельный response DTO не объявлен |
| `CMD_EXTRAS_GET_PRESETS` | `extras_get_presets` | Отдельный request DTO не объявлен | `ExtrasGetPresetsResponse` |
| `CMD_EXTRAS_APPLY_PRESET` | `extras_apply_preset` | `ExtrasApplyPresetRequest` | `PresetApplyResultDto` |

**Различия одноимённых DTO:** launch использует `wex_ipc::models::LauncherConfig` (ram_gb) и InstalledVersion без required_java. `get_launcher_config` использует app-local LauncherSettingsDto (ram_mb, Java/Discord/language). Одноимённый LauncherConfig в app/ipc.rs — другой исторический тип; он не является wire-типом get_config. Не заменяйте один другим. Установочный trait `LauncherCore::install_instance` есть, отдельного зарегистрированного core_install_instance нет. В этом снимке нет `startup.rs` и команды startup: загрузка приложения описана в ARCHITECTURE.md.

## События Rust → UI

`listen<Payload>(name, e => ...)` получает Tauri envelope, данные лежат в `e.payload`. App emit посылает голый DTO, **не** `{event,payload}` из shared IpcEvent. Подписку сохраняют и вызывают unlisten при размонтировании. Подписывайтесь до запуска операции: это broadcast без replay/подтверждения доставки; часть emit-ошибок намеренно игнорируется.

| Событие | Payload | Реальный producer / статус |
|---|---|---|
| `install-progress` | InstallProgressPayload | TauriEventHandler в launch.rs: StageStarted/Progress |
| `launch-status` | LaunchStatusPayload | launch.rs: preparing/installing/running/cancelled/idle/error; тип допускает и launching, отдельный такой emit сейчас не найден |
| `game-log` | GameLogPayload | launch.rs: GameOutput и Log; реальные levels включают DEBUG, INFO, WARN, ERROR |
| `network-status` | app commands/network.rs::NetworkStatusPayload `{is_online: bool}` | NetworkState supervisor каждые 5 s, emit только при смене bool; этого события нет в shared IpcEvent/generated EVENTS |
| `social-achievement-toast` | AchievementUnlockedEventDto | friends.rs::report_achievement_progress_impl, только при разблокировке и наличии AppHandle |
| `net-tunnel-status` | NetTunnelStatusPayload | Объявлено shared; NetEventBus существует, но его подписчик/мост в app.emit не подключён |
| `net-ping-progress` | NetPingProgressPayload | Объявлено shared; app emit producer не найден, ping_server возвращает результат команды |
| `net-lobby-status` | NetLobbyStatusPayload | Объявлено shared; LobbyApiClient/heartbeat используют NetEventBus, моста Tauri нет |
| `social-friend-presence` | SocialFriendPresencePayload | Объявлено shared; producer app.emit отсутствует, UI должен получать текущий список/синхронизацию командой |
| `social-lobby-invite` | LobbyInviteItemDto | Объявлено shared; app.emit отсутствует, входящие приглашения приходят в social_sync.incoming_invites |
| `launcher-update-available` | UpdateCheckResultDto | Объявлено shared; app.emit отсутствует, overview возвращает результат invoke |

Payload shared-полей ниже. `install-progress.speed_bytes_per_sec` текущий адаптер устанавливает в 0. Поля total_bytes/downloaded_bytes заполняются raw total/current из Event::Progress, чья единица зависит от этапа; не обещайте пользователю скорость/ETA в байтах только по имени поля. Supervisor и EventHandler могут сообщать финальный статус независимо, поэтому не предполагайте единственный финальный event на запуск.

Shared `IpcEvent` — tagged enum serde `{ "event": "game-log", "payload": {...} }` для явной сериализации enum. Это отдельное представление, не автоматический формат app.emit. Остальные enums в models.rs не имеют rename_all: сохраняют имена Rust (`"Online"`, `"Offline"` и т.п.), data-варианты externally tagged (`{"Version":{"player_version":"...","host_version":"..."}}`). Не предполагайте lowercase по комментариям UI.

## Примеры вызовов

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LaunchGameResponse, InstallProgressPayload, FriendProfileDto, UserAuthDto } from './lib/ipc/generated';

const unlisten = await listen<InstallProgressPayload>('install-progress', ({ payload }) => {
  console.log(payload.stage, payload.percentage);
});
const launch = await invoke<LaunchGameResponse>('start_game', {
  version: '1.21.1', nickname: 'Player', jvmArgs: [],
});
// Call unlisten() on component teardown; handle rejection as String here.

async function login(auth: UserAuthDto) {
  return invoke<FriendProfileDto>('social_login', { request: { auth } });
}
await invoke('toggle_mod', { version: '1.20.1', fileName: 'example.jar', enabled: false });
```

Не отправляйте `{request: ...}` команде start_game; не используйте незарегистрированный `COMMANDS.CORE_LAUNCH_GAME`. Для social request поля внутри auth/request сохраняют snake_case. Временные тестовые каталоги используйте вместо пользовательских .minecraft/Wex.

## Генерация и границы проверки типов

Из корня репозитория:

```powershell
cargo run --manifest-path core/Cargo.toml -p wex-ipc --bin gen_ts -j 2
cargo test --manifest-path core/Cargo.toml -p wex-ipc test_typescript_bindings_drift_detection -j 2
```

gen_ts записывает `ui/src/lib/ipc/generated/`; drift-test сравнивает с generate_all_bindings, нормализуя CRLF/LF. Это проверяет декларации wex-ipc, **не** реестр generate_handler!, app-local DTO, совместимость аргументов, наличие event producers или фактическую wire-форму вызова. Нельзя считать этот тест проверкой всех 101 Tauri-команд. Для документационной правки генератор запускать не требуется.

## Поля DTO

Ниже схемы shared commands/models/events и app-local wire DTO. Имена полей — serde JSON, типы — Rust. `Option<T>` можно передать null/опустить; обязательные Vec/числа/boolean нужно передавать, кроме полей с явным `#[serde(default)]`. Для ApplyImportRequest такие flags default=false, списки default=[]. Ссылки ведут в исходник, при изменении типов эта таблица требует синхронизации.

### core/wex-ipc/src/commands.rs

| Тип | Поля |
|---|---|
| [`InstallInstanceRequest`](../core/wex-ipc/src/commands.rs#L20) | `version: String; force_reinstall: bool` |
| [`InstallInstanceResponse`](../core/wex-ipc/src/commands.rs#L26) | `success: bool; version: String; message: String` |
| [`LaunchGameRequest`](../core/wex-ipc/src/commands.rs#L33) | `version: String; nickname: String; jvm_args: Option<Vec<String>>` |
| [`LaunchGameResponse`](../core/wex-ipc/src/commands.rs#L40) | `success: bool; pid: Option<u32>; message: String` |
| [`IsRunningRequest`](../core/wex-ipc/src/commands.rs#L47) | `instance_id: String` |
| [`IsRunningResponse`](../core/wex-ipc/src/commands.rs#L52) | `is_running: bool; pid: Option<u32>` |
| [`KillGameRequest`](../core/wex-ipc/src/commands.rs#L58) | `instance_id: String` |
| [`NetPingRequest`](../core/wex-ipc/src/commands.rs#L89) | `address: String; port: Option<u16>; timeout_ms: Option<u64>` |
| [`NetResolveRequest`](../core/wex-ipc/src/commands.rs#L96) | `host: String; default_port: Option<u16>` |
| [`NetResolveResponse`](../core/wex-ipc/src/commands.rs#L102) | `host: String; port: u16` |
| [`NetLoadServersRequest`](../core/wex-ipc/src/commands.rs#L108) | `path: String` |
| [`NetLoadServersResponse`](../core/wex-ipc/src/commands.rs#L113) | `servers: Vec<PackServerEntryDto>` |
| [`NetSaveServersRequest`](../core/wex-ipc/src/commands.rs#L118) | `path: String; servers: Vec<PackServerEntryDto>` |
| [`NetMergeServersRequest`](../core/wex-ipc/src/commands.rs#L124) | `user_servers_path: String; pack_servers: Vec<PackServerEntryDto>` |
| [`NetMergeServersResponse`](../core/wex-ipc/src/commands.rs#L130) | `merged_count: usize` |
| [`NetLobbyCreateRequest`](../core/wex-ipc/src/commands.rs#L135) | `host_name: String; manifest: Option<Vec<ModManifestEntryDto>>` |
| [`NetLobbyJoinRequest`](../core/wex-ipc/src/commands.rs#L141) | `code: String; player_name: String; manifest: Option<Vec<ModManifestEntryDto>>` |
| [`NetLobbyOpenRequest`](../core/wex-ipc/src/commands.rs#L148) | `code: String; host_token: String; tunnel_address: String; port: u16` |
| [`NetLobbyStatusRequest`](../core/wex-ipc/src/commands.rs#L156) | `code: String; player_name: Option<String>` |
| [`NetLobbyHeartbeatRequest`](../core/wex-ipc/src/commands.rs#L162) | `code: String; host_token: String` |
| [`NetLobbyCloseRequest`](../core/wex-ipc/src/commands.rs#L168) | `code: String; host_token: String` |
| [`NetLobbyLeaveRequest`](../core/wex-ipc/src/commands.rs#L174) | `code: String; player_name: String` |
| [`NetLobbyKickRequest`](../core/wex-ipc/src/commands.rs#L180) | `code: String; host_token: String; player: String` |
| [`NetListOpenRoomsRequest`](../core/wex-ipc/src/commands.rs#L187) | `player_name: Option<String>` |
| [`NetListOpenRoomsResponse`](../core/wex-ipc/src/commands.rs#L192) | `rooms: Vec<NetOpenRoomDto>` |
| [`NetCompareManifestsRequest`](../core/wex-ipc/src/commands.rs#L197) | `client_manifest: Vec<ModManifestEntryDto>; host_manifest: Vec<ModManifestEntryDto>` |
| [`NetCompareManifestsResponse`](../core/wex-ipc/src/commands.rs#L203) | `mismatches: Vec<ModMismatchItemDto>; manifest_hash: String` |
| [`NetComputeManifestHashRequest`](../core/wex-ipc/src/commands.rs#L209) | `manifest: Vec<ModManifestEntryDto>` |
| [`NetComputeManifestHashResponse`](../core/wex-ipc/src/commands.rs#L214) | `hash: String` |
| [`NetTunnelStartRequest`](../core/wex-ipc/src/commands.rs#L219) | `binary_path: String; secret_key: String` |
| [`NetTunnelStartResponse`](../core/wex-ipc/src/commands.rs#L225) | `pid: u32` |
| [`NetTunnelEnsureBinaryRequest`](../core/wex-ipc/src/commands.rs#L230) | `destination_path: String` |
| [`NetTunnelEnsureBinaryResponse`](../core/wex-ipc/src/commands.rs#L235) | `path: String; verified: bool` |
| [`ExtrasAnalyzeCrashRequest`](../core/wex-ipc/src/commands.rs#L260) | `exit_code: Option<i32>; log_text: Option<String>; crash_report_text: Option<String>` |
| [`ExtrasValidateSkinRequest`](../core/wex-ipc/src/commands.rs#L267) | `skin_bytes_base64: String` |
| [`ExtrasConvertSkinRequest`](../core/wex-ipc/src/commands.rs#L272) | `skin_bytes_base64: String` |
| [`ExtrasConvertSkinResponse`](../core/wex-ipc/src/commands.rs#L277) | `converted_bytes_base64: String` |
| [`ExtrasSaveSkinRequest`](../core/wex-ipc/src/commands.rs#L282) | `history_dir: String; skin_name: String; skin_bytes_base64: String; model: SkinModelDto` |
| [`ExtrasListSkinHistoryRequest`](../core/wex-ipc/src/commands.rs#L290) | `history_dir: String` |
| [`ExtrasListSkinHistoryResponse`](../core/wex-ipc/src/commands.rs#L295) | `items: Vec<WardrobeItemDto>` |
| [`ExtrasGenerateCslConfigRequest`](../core/wex-ipc/src/commands.rs#L300) | `output_path: String; csl_api_root: String` |
| [`WardrobeApplyRequest`](../core/wex-ipc/src/commands.rs#L306) | `nickname: String; skin_name: String; skin_base64: Option<String>; model: SkinModelDto; cape_id: String; cape_name: String; cape_base64: Option<String>` |
| [`WardrobeApplyResponse`](../core/wex-ipc/src/commands.rs#L317) | `success: bool; message: String` |
| [`WardrobeDeleteHistoryRequest`](../core/wex-ipc/src/commands.rs#L323) | `history_dir: String; item_id: String` |
| [`WardrobeDeleteHistoryResponse`](../core/wex-ipc/src/commands.rs#L329) | `success: bool` |
| [`ExtrasDiscordSetActivityRequest`](../core/wex-ipc/src/commands.rs#L334) | `activity: DiscordActivityDto` |
| [`ExtrasImportOldLauncherRequest`](../core/wex-ipc/src/commands.rs#L339) | `old_launcher_dir: String` |
| [`ExtrasGetPresetsResponse`](../core/wex-ipc/src/commands.rs#L344) | `presets: Vec<GraphicsPresetDto>` |
| [`ExtrasApplyPresetRequest`](../core/wex-ipc/src/commands.rs#L349) | `options_txt_path: String; shaderpacks_dir: String; tier: PresetTierDto` |
| [`SocialLoginRequest`](../core/wex-ipc/src/commands.rs#L378) | `auth: UserAuthDto` |
| [`SocialGetFriendsRequest`](../core/wex-ipc/src/commands.rs#L383) | `auth: UserAuthDto` |
| [`SocialGetFriendsResponse`](../core/wex-ipc/src/commands.rs#L388) | `friends: Vec<FriendItemDto>` |
| [`SocialSendFriendRequestRequest`](../core/wex-ipc/src/commands.rs#L393) | `auth: UserAuthDto; target_friend_code: String` |
| [`SocialAcceptFriendRequestRequest`](../core/wex-ipc/src/commands.rs#L399) | `auth: UserAuthDto; request_id: String` |
| [`SocialDeclineFriendRequestRequest`](../core/wex-ipc/src/commands.rs#L405) | `auth: UserAuthDto; request_id: String` |
| [`SocialSyncRequest`](../core/wex-ipc/src/commands.rs#L411) | `auth: UserAuthDto` |
| [`SocialSendLobbyInviteRequest`](../core/wex-ipc/src/commands.rs#L416) | `auth: UserAuthDto; target_user_id: String; lobby_code: String; host_name: Option<String>` |
| [`SocialSetPresenceRequest`](../core/wex-ipc/src/commands.rs#L424) | `auth: UserAuthDto; presence: UserPresenceDto` |
| [`SocialListAchievementsRequest`](../core/wex-ipc/src/commands.rs#L430) | `data_dir: String` |
| [`SocialListAchievementsResponse`](../core/wex-ipc/src/commands.rs#L435) | `achievements: Vec<AchievementDefinitionDto>; progress: Vec<AchievementProgressDto>` |
| [`SocialReportAchievementProgressRequest`](../core/wex-ipc/src/commands.rs#L441) | `data_dir: String; achievement_id: String; increment: u32` |
| [`SocialReportAchievementProgressResponse`](../core/wex-ipc/src/commands.rs#L448) | `unlocked: bool; current: f64; target: u32` |
| [`SocialListWorldsRequest`](../core/wex-ipc/src/commands.rs#L455) | `game_dir: String` |
| [`SocialListWorldsResponse`](../core/wex-ipc/src/commands.rs#L460) | `worlds: Vec<WorldItemDto>` |
| [`SocialCreateWorldBackupRequest`](../core/wex-ipc/src/commands.rs#L465) | `world_dir: String; backups_dir: String` |
| [`SocialListWorldBackupsRequest`](../core/wex-ipc/src/commands.rs#L471) | `backups_dir: String` |
| [`SocialListWorldBackupsResponse`](../core/wex-ipc/src/commands.rs#L476) | `backups: Vec<WorldBackupItemDto>` |
| [`SocialRestoreWorldBackupRequest`](../core/wex-ipc/src/commands.rs#L481) | `backup_path: String; saves_dir: String` |
| [`SocialRestoreWorldBackupResponse`](../core/wex-ipc/src/commands.rs#L487) | `restored_world_dir: String` |
| [`SocialListScreenshotsRequest`](../core/wex-ipc/src/commands.rs#L492) | `screenshots_dir: String` |
| [`SocialListScreenshotsResponse`](../core/wex-ipc/src/commands.rs#L497) | `screenshots: Vec<ScreenshotItemDto>` |
| [`SocialGetNewsRequest`](../core/wex-ipc/src/commands.rs#L502) | `cache_dir: String; remote_url: Option<String>; force_refresh: bool` |
| [`SocialGetNewsResponse`](../core/wex-ipc/src/commands.rs#L509) | `versions: Vec<ReleaseNoteVersionDto>` |
| [`SocialCheckLauncherUpdateRequest`](../core/wex-ipc/src/commands.rs#L514) | `manifest_url: String; current_version: String; target_platform: Option<String>` |

### core/wex-ipc/src/models.rs

| Тип | Поля |
|---|---|
| [`LauncherConfig`](../core/wex-ipc/src/models.rs#L11) | `selected_version: String; player_name: String; ram_gb: u32; reduce_motion: bool; reduce_transparency: bool; appearance: String; accent: String` |
| [`InstalledVersion`](../core/wex-ipc/src/models.rs#L37) | `id: String; name: String; is_installed: bool` |
| [`PlayerSampleDto`](../core/wex-ipc/src/models.rs#L48) | `name: String; id: String` |
| [`ServerPingResponseDto`](../core/wex-ipc/src/models.rs#L54) | `host: String; port: u16; latency_ms: u64; version_name: String; protocol: i32; players_online: u32; players_max: u32; sample_players: Vec<PlayerSampleDto>; description: String; favicon: Option<String>` |
| [`PackServerEntryDto`](../core/wex-ipc/src/models.rs#L94) | `id: Option<String>; name: String; ip: String; icon: Option<String>` |
| [`LobbyCreateResponseDto`](../core/wex-ipc/src/models.rs#L124) | `code: String; host_token: String; status: String; created_at: i64` |
| [`LobbyJoinResponseDto`](../core/wex-ipc/src/models.rs#L143) | `success: bool; code: String; status: String; tunnel_address: Option<String>; player_count: u32` |
| [`LobbyStatusResponseDto`](../core/wex-ipc/src/models.rs#L164) | `code: String; status: String; tunnel_address: Option<String>; player_count: u32; last_heartbeat: i64; players: Option<Vec<String>>; host_name: Option<String>; kicked: bool` |
| [`TunnelConfigResponseDto`](../core/wex-ipc/src/models.rs#L191) | `secret: Option<String>; public_address: Option<String>; public_port: Option<u16>` |
| [`NetOpenRoomDto`](../core/wex-ipc/src/models.rs#L198) | `id: String; name: String; host_name: String; game_version: String; player_count: u32; max_players: u32; address: Option<String>; status: String` |
| [`ModManifestEntryDto`](../core/wex-ipc/src/models.rs#L220) | `id: String; name: String; version: String; enabled: bool` |
| [`ModMismatchItemDto`](../core/wex-ipc/src/models.rs#L278) | `mod_id: String; mod_name: String; mismatch_type: ModMismatchTypeDto; type_ru: String; is_fixable: bool; player_version: Option<String>; host_version: Option<String>; display_description: String` |
| [`CrashDiagnosisDto`](../core/wex-ipc/src/models.rs#L398) | `kind: CrashKindDto; title: String; user_advice: String; technical_summary: String` |
| [`SkinValidationResultDto`](../core/wex-ipc/src/models.rs#L441) | `is_valid: bool; error: Option<String>; width: u32; height: u32` |
| [`WardrobeItemDto`](../core/wex-ipc/src/models.rs#L460) | `id: String; item_type: String; name: String; hash: String; model: String; relative_path: String; created_utc: String` |
| [`WardrobeStateDto`](../core/wex-ipc/src/models.rs#L485) | `nickname: String; skin_name: String; skin_base64: Option<String>; model: SkinModelDto; cape_id: String; cape_name: String; cape_base64: Option<String>` |
| [`ImportedServerDto`](../core/wex-ipc/src/models.rs#L569) | `name: String; ip: String; icon: Option<String>` |
| [`ImportedSkinDto`](../core/wex-ipc/src/models.rs#L576) | `file_name: String; path: String; hash: String; is_slim: bool` |
| [`ImportedCapeDto`](../core/wex-ipc/src/models.rs#L584) | `file_name: String; path: String; hash: String` |
| [`ImportedWardrobeItemDto`](../core/wex-ipc/src/models.rs#L591) | `id: String; item_type: String; name: String; hash: String; model: String` |
| [`OldLauncherDataDto`](../core/wex-ipc/src/models.rs#L600) | `nickname: String; ram_mb: u32; selected_version: String; graphics_preset: Option<String>; total_play_time_seconds: u64; total_game_launches: u32; servers: Vec<ImportedServerDto>; skins: Vec<ImportedSkinDto>; capes: Vec<ImportedCapeDto>; wardrobe_history: Vec<ImportedWardrobeItemDto>` |
| [`GraphicsPresetDto`](../core/wex-ipc/src/models.rs#L696) | `key: String; name: String; hardware: String; render_distance: u32; simulation_distance: u32; graphics_mode: String; particles: String; clouds: String; entity_distance_scaling: f64; biome_blend_radius: u32; mipmap_levels: u32; ao: u32; ram_mb: u32; jvm_args: Vec<String>; shader_pack: String; shader_packs_fallback: Vec<String>; shader_options: HashMap<String, String>; resource_packs: Vec<String>` |
| [`PresetApplyResultDto`](../core/wex-ipc/src/models.rs#L743) | `preset_key: String; chosen_shader: String; final_ram_mb: u32; jvm_args: Vec<String>; written_options: HashMap<String, String>; warning: Option<String>` |
| [`UserAuthDto`](../core/wex-ipc/src/models.rs#L770) | `user_id: String; user_token: String; friend_code: String; nick: String` |
| [`FriendProfileDto`](../core/wex-ipc/src/models.rs#L830) | `id: String; nick: String; friend_code: Option<String>; play_time_seconds: u64; total_launches: u32; achievements_count: usize; skin_hash: Option<String>; skin_model: Option<String>` |
| [`FriendItemDto`](../core/wex-ipc/src/models.rs#L857) | `id: String; nick: String; status: OnlineStatusDto; last_seen: Option<i64>; game_version: Option<String>; world_name: Option<String>; lobby_code: Option<String>` |
| [`UserPresenceDto`](../core/wex-ipc/src/models.rs#L882) | `status: Option<OnlineStatusDto>; game_version: Option<String>; world_name: Option<String>; lobby_code: Option<String>` |
| [`FriendRequestItemDto`](../core/wex-ipc/src/models.rs#L912) | `id: String; from_id: String; from_nick: String; created_at: Option<i64>` |
| [`LobbyInviteItemDto`](../core/wex-ipc/src/models.rs#L931) | `id: String; lobby_code: String; from_id: String; from_nick: String; host_name: Option<String>; created_at: Option<i64>` |
| [`SyncResponseDto`](../core/wex-ipc/src/models.rs#L954) | `friends: Vec<FriendItemDto>; incoming_requests: Vec<FriendRequestItemDto>; incoming_invites: Vec<LobbyInviteItemDto>` |
| [`AchievementDefinitionDto`](../core/wex-ipc/src/models.rs#L971) | `id: String; title: String; description: String; target: u32` |
| [`AchievementProgressDto`](../core/wex-ipc/src/models.rs#L990) | `id: String; current: f64; unlocked: bool; unlocked_at: Option<String>` |
| [`AchievementUnlockedEventDto`](../core/wex-ipc/src/models.rs#L1009) | `id: String; title: String; description: String; unlocked_at: String` |
| [`WorldItemDto`](../core/wex-ipc/src/models.rs#L1049) | `folder_name: String; folder_path: String; display_name: String; game_mode: GameModeDto; last_played: u64; day_time: u64; size_bytes: u64; icon_path: Option<String>` |
| [`WorldBackupItemDto`](../core/wex-ipc/src/models.rs#L1076) | `file_name: String; file_path: String; created_at: Option<String>; size_bytes: u64` |
| [`ScreenshotItemDto`](../core/wex-ipc/src/models.rs#L1095) | `file_name: String; file_path: String; created_at: Option<String>; size_bytes: u64` |
| [`ReleaseNoteItemDto`](../core/wex-ipc/src/models.rs#L1133) | `category: NoteCategoryDto; text: String; raw: String` |
| [`ReleaseNoteVersionDto`](../core/wex-ipc/src/models.rs#L1150) | `version: String; internal_version: String; date_text: String; short_title: String; is_current: bool; items: Vec<ReleaseNoteItemDto>` |
| [`UpdateCheckResultDto`](../core/wex-ipc/src/models.rs#L1173) | `update_available: bool; current_version: String; latest_version: Option<String>; notes: Option<String>; download_url: Option<String>; signature: Option<String>; pub_date: Option<String>` |

#### ModMismatchTypeDto

```rust
pub enum ModMismatchTypeDto {
    Disabled,
    Missing,
    Extra,
    Version {
        player_version: String,
        host_version: String,
    },
}
```

#### TunnelStatusDto

```rust
pub enum TunnelStatusDto {
    Inactive,
    Downloading { progress: f32 },
    Starting,
    Running { address: String, port: u16 },
    Stopped,
    Error(String),
}
```

#### CrashKindDto

```rust
pub enum CrashKindDto {
    OutOfMemory,
    MissingFabricDependency {
        mod_name: String,
        required_dep: String,
    },
    IncompatibleMod {
        mod_name: String,
    },
    MixinConflict {
        mod_names: Vec<String>,
        target_class: Option<String>,
    },
    InvalidJavaVersion {
        required_java: String,
        current_java: Option<String>,
    },
    CorruptedFile {
        details: String,
    },
    GraphicsDriverOrShader {
        details: String,
    },
    Unknown {
        exit_code: Option<i32>,
    },
}
```

#### SkinModelDto

```rust
pub enum SkinModelDto {
    Classic,
    Slim,
}
```

#### DiscordActivityDto

```rust
pub enum DiscordActivityDto {
    #[default]
    InLauncher,
    Playing {
        version: String,
        world_name: Option<String>,
        mod_count: Option<usize>,
        start_timestamp: Option<i64>,
    },
    InLobby {
        lobby_code: Option<String>,
        player_count: usize,
        host_name: Option<String>,
    },
}
```

#### PresetTierDto

```rust
pub enum PresetTierDto {
    Low,
    Medium,
    High,
    Ultra,
}
```

#### OnlineStatusDto

```rust
pub enum OnlineStatusDto {
    Online,
    Playing,
    InLobby,
    Offline,
}
```

#### GameModeDto

```rust
pub enum GameModeDto {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
    Unknown = -1,
}
```

#### NoteCategoryDto

```rust
pub enum NoteCategoryDto {
    New,
    Improved,
    Fixed,
    General,
}
```

### core/wex-ipc/src/events.rs

| Тип | Поля |
|---|---|
| [`InstallProgressPayload`](../core/wex-ipc/src/events.rs#L20) | `stage: String; percentage: f32; detail: String; speed_bytes_per_sec: u64; total_bytes: u64; downloaded_bytes: u64` |
| [`LaunchStatusPayload`](../core/wex-ipc/src/events.rs#L32) | `state: String; message: String; exit_code: Option<i32>` |
| [`GameLogPayload`](../core/wex-ipc/src/events.rs#L42) | `line: String; level: String` |
| [`NetTunnelStatusPayload`](../core/wex-ipc/src/events.rs#L50) | `status: TunnelStatusDto; address: Option<String>` |
| [`NetPingProgressPayload`](../core/wex-ipc/src/events.rs#L58) | `host: String; port: u16; latency_ms: Option<u64>; status: String` |
| [`NetLobbyStatusPayload`](../core/wex-ipc/src/events.rs#L68) | `code: String; status: String; player_count: u32; tunnel_address: Option<String>` |
| [`SocialFriendPresencePayload`](../core/wex-ipc/src/events.rs#L78) | `user_id: String; presence: UserPresenceDto` |

### core/wex-ipc/src/error.rs

| Тип | Поля |
|---|---|
| [`IpcError`](../core/wex-ipc/src/error.rs#L14) | `code: String; message: String; technical: Option<String>` |

### app/src-tauri/src/commands/settings.rs

| Тип | Поля |
|---|---|
| [`LauncherSettingsDto`](../app/src-tauri/src/commands/settings.rs#L45) | `selected_version: String; player_name: String; ram_mb: u32; total_memory_mb: u32; game_dir: String; graphics_preset: String; java_mode: String; custom_java_path: Option<String>; detected_java_17: Option<String>; detected_java_21: Option<String>; discord_rpc: bool; language: String; appearance: String; accent: String; reduce_motion: bool; reduce_transparency: bool` |

### app/src-tauri/src/commands/mods.rs

| Тип | Поля |
|---|---|
| [`ModEntry`](../app/src-tauri/src/commands/mods.rs#L12) | `id: String; name: String; version: String; description: String; file_name: String; enabled: bool; is_pack_mod: bool; icon: Option<String>` |

### app/src-tauri/src/commands/network.rs

| Тип | Поля |
|---|---|
| [`NetworkStatusPayload`](../app/src-tauri/src/commands/network.rs#L12) | `is_online: bool` |

### app/src-tauri/src/commands/shell.rs

| Тип | Поля |
|---|---|
| [`NetworkStatusDto`](../app/src-tauri/src/commands/shell.rs#L17) | `is_online: bool; latency_ms: Option<u64>; status_text: String` |
| [`ShellProfileDto`](../app/src-tauri/src/commands/shell.rs#L25) | `nickname: String; initials: String; is_online: bool; has_custom_skin: bool; skin_head_url: Option<String>` |
| [`ShellLaunchStateDto`](../app/src-tauri/src/commands/shell.rs#L35) | `is_running: bool; is_launching: bool; running_version: Option<String>; client_status: String` |

### app/src-tauri/src/commands/overview.rs

| Тип | Поля |
|---|---|
| [`ApplyImportRequest`](../app/src-tauri/src/commands/overview.rs#L22) | `old_launcher_dir: String; import_nickname: bool; nickname: Option<String>; import_ram: bool; ram_mb: Option<u32>; import_version: bool; selected_version: Option<String>; import_servers: bool; servers: Vec<ImportedServerDto>; import_skins: bool; skins: Vec<ImportedSkinDto>` |
| [`ApplyImportResponse`](../app/src-tauri/src/commands/overview.rs#L44) | `success: bool; imported_nickname: Option<String>; imported_ram_gb: Option<u32>; imported_servers_count: usize; imported_skins_count: usize; message: String` |

### app/src-tauri/src/commands/workshop_packs.rs

| Тип | Поля |
|---|---|
| [`ResourcePackItemDto`](../app/src-tauri/src/commands/workshop_packs.rs#L15) | `name: String; file_name: String; file_path: String; size_bytes: u64; description: Option<String>; pack_format: Option<u32>; icon_data_url: Option<String>; is_enabled: bool; is_from_pack: bool` |
| [`ShaderPackItemDto`](../app/src-tauri/src/commands/workshop_packs.rs#L28) | `name: String; file_name: String; file_path: String; size_bytes: u64; description: Option<String>; icon_data_url: Option<String>; is_active: bool; is_from_pack: bool` |

### app/src-tauri/src/ipc.rs

| Тип | Поля |
|---|---|
| [`SystemInfo`](../app/src-tauri/src/ipc.rs#L64) | `total_memory_mb: u32; os_name: String; launcher_version: String; data_dir: String` |
| [`ServerEntryModel`](../app/src-tauri/src/ipc.rs#L82) | `id: String; name: String; address: String; icon: Option<String>; ping_ms: Option<u64>; online: Option<u32>; max_players: Option<u32>; motd: Option<String>; version: Option<String>` |
| [`ServerPingResult`](../app/src-tauri/src/ipc.rs#L96) | `latency_ms: u64; online: u32; max_players: u32; motd: String; version: String; favicon: Option<String>` |
| [`LobbyPlayerModel`](../app/src-tauri/src/ipc.rs#L107) | `name: String; is_host: bool; ping_ms: u64; mods_synced: bool` |
| [`LobbyRoomModel`](../app/src-tauri/src/ipc.rs#L116) | `code: String; host_name: String; world_name: String; tunnel_address: Option<String>; tunnel_active: bool; is_host: bool; players: Vec<LobbyPlayerModel>; mod_mismatch_count: usize; total_mods: usize` |
