# Единый типизированный контракт IPC Wex (Rust ↔ UI)

Документация типизированного слоя межпроцессного взаимодействия (IPC) между ядром Wex (Rust) и графическим интерфейсом (Svelte 5 / TypeScript). Реализован в крейте `core/wex-ipc/` с генерацией определений TypeScript в `ui/src/lib/ipc/generated/`.

---

## 1. Архитектура и принципы

- **Тонкий слой Tauri (`app/src-tauri`)**: обработчики команд Tauri являются легковесными адаптерами, принимающими строго типизированные структуры запросов и возвращающими типизированные структуры ответов или единую ошибку `IpcError`.
- **Строгая типизация без ручной синхронизации**: структуры данных, запросов, ответов и событий описываются на Rust с деривацией `Serialize`, `Deserialize` и `TS` (`ts-rs`). Типы автоматически генерируются в `ui/src/lib/ipc/generated/`.
- **Защита от расхождения контрактов (Drift Detection)**: unit-тест `test_typescript_bindings_drift_detection` падает при любых изменениях Rust-контрактов, если файлы TypeScript в `ui/src/lib/ipc/generated/` не были синхронизированы.
- **Изоляция ядра запуска (`LauncherCore`)**: ядро запуска описывается абстрактным асинхронным трейтом `LauncherCore` с реализацией `MockLauncherCore` для автономной работы и тестирования UI до интеграции `wex-core` (агента a1).

---

## 2. Единая модель ошибок (`IpcError`)

Все операции IPC возвращают `IpcResult<T> = Result<T, IpcError>`.

### Структура `IpcError`
```rust
pub struct IpcError {
    /// Машиночитаемый код ошибки (например, "NET_PING_TIMEOUT", "SOCIAL_NOT_FOUND")
    pub code: String,
    /// Понятный пользователю текст на русском языке для всплывающих уведомлений (toasts) и диалогов
    pub message: String,
    /// Технические подробности (исключение, код ошибки ОС, HTTP-статус, путь к файлу) для логов
    pub technical: Option<String>,
}
```

### Стандартизированные коды ошибок

| Код ошибки | Категория | Описание / Совет пользователю |
|---|---|---|
| `NOT_FOUND` | Общая | Запрашиваемый ресурс или объект не найден |
| `INVALID_ARGUMENT` | Общая | Переданы некорректные или неполные параметры запроса |
| `INTERNAL_ERROR` | Общая | Непредвиденная внутренняя ошибка ядра или сервиса |
| `TIMEOUT` | Общая | Истекло время ожидания ответа от операции или службы |
| `IO_ERROR` | Файловая система | Ошибка дисковых операций ввода-вывода |
| `IO_NOT_FOUND` | Файловая система | Файл или каталог не найден на диске |
| `IO_PERMISSION_DENIED` | Файловая система | Доступ к файлу запрещён операционной системой |
| `IO_ALREADY_EXISTS` | Файловая система | Создаваемый файл или каталог уже существует |
| `IO_TIMED_OUT` | Файловая система | Истекло время ожидания файловой блокировки или операции |
| `SERIALIZATION_ERROR` | Данные | Ошибка сериализации или разбора формата JSON |
| `NET_LOBBY_NOT_FOUND` | Сеть / Лобби | Комната лобби не найдена или была закрыта хостом |
| `NET_LOBBY_UNAUTHORIZED` | Сеть / Лобби | Неверный секретный токен доступа к лобби |
| `NET_LOBBY_API_ERROR` | Сеть / Лобби | Ошибка API сервера лобби (4xx / 5xx) |
| `NET_LOBBY_NETWORK_ERROR`| Сеть / Лобби | Сетевой сбой или таймаут соединения с Lobby API |
| `NET_LOBBY_JSON_ERROR` | Сеть / Лобби | Некорректный формат ответа от сервера лобби |
| `NET_LOBBY_URL_ERROR` | Сеть / Лобби | Недопустимый адрес базового эндпоинта Lobby API |
| `SOCIAL_NOT_FOUND` | Социальное | Пользователь или заявка в друзья не найдена |
| `SOCIAL_UNAUTHORIZED` | Социальное | Ошибка авторизации: отсутствует или истёк токен сессии |
| `SOCIAL_API_ERROR` | Социальное | Ошибка сервера социальных функций |
| `SOCIAL_NETWORK_ERROR` | Социальное | Сетевая ошибка при синхронизации профиля и друзей |
| `SOCIAL_JSON_ERROR` | Социальное | Ошибка обработки данных профиля или друзей |
| `ACHIEVEMENT_IO_ERROR` | Достижения | Ошибка чтения/записи локального хранилища достижений |
| `ACHIEVEMENT_SERIALIZE_ERROR` | Достижения | Ошибка разбора структуры прогресса достижений |
| `WORKSHOP_NOT_FOUND` | Мастерская | Указанный каталог мира или архив резервной копии не найден |
| `WORKSHOP_IO_ERROR` | Мастерская | Ошибка файловой системы при работе со снимками или мирами |
| `WORKSHOP_ZIP_ERROR` | Мастерская | Ошибка распаковки или сжатия ZIP-архива резервной копии |
| `WORKSHOP_INVALID_LEVEL_DAT` | Мастерская | Файл level.dat повреждён или имеет неподдерживаемый NBT |
| `UPDATE_NETWORK_ERROR` | Обновления | Не удалось загрузить манифест `latest.json` |
| `UPDATE_INVALID_MANIFEST` | Обновления | Файл манифеста обновлений повреждён |
| `UPDATE_PLATFORM_NOT_FOUND`| Обновления | Сборка лаунчера для текущей ОС не найдена в манифесте |
| `UPDATE_SIGNATURE_FAILED` | Обновления | Цифровая подпись пакета обновления не прошла верификацию |

---

## 3. Таблица команд IPC (Rust ↔ UI)

### 3.1. Ядро лаунчера (`LauncherCore`)

| Имя команды | Запрос (TypeScript) | Ответ (TypeScript) | Описание |
|---|---|---|---|
| `core_get_config` | `void` | `LauncherConfig` | Получение активных настроек (память, тема, версия) |
| `core_save_config` | `LauncherConfig` | `void` | Сохранение обновлённой конфигурации лаунчера |
| `core_get_installed_versions` | `void` | `Array<InstalledVersion>` | Список доступных/установленных версий клиента |
| `core_install_instance` | `InstallInstanceRequest` | `InstallInstanceResponse` | Загрузка и проверка инстанса клиента |
| `core_launch_game` | `LaunchGameRequest` | `LaunchGameResponse` | Запуск Minecraft с аргументами JVM и ником |
| `core_cancel_launch` | `void` | `void` | Запрос немедленной отмены процедуры запуска |
| `core_is_running` | `IsRunningRequest` | `IsRunningResponse` | Проверка, запущен ли процесс игры для инстанса |
| `core_kill_game` | `KillGameRequest` | `void` | Принудительное завершение процесса клиента |

### 3.2. Сетевая подсистема (`wex-net`)

| Имя команды | Запрос (TypeScript) | Ответ (TypeScript) | Описание |
|---|---|---|---|
| `net_ping_server` | `NetPingRequest` | `ServerPingResponseDto` | Minecraft SLP пинг сервера (задержка, онлайн, MOTD, иконка) |
| `net_resolve_address` | `NetResolveRequest` | `NetResolveResponse` | Разрешение DNS SRV-записи `_minecraft._tcp` |
| `net_load_servers` | `NetLoadServersRequest`| `NetLoadServersResponse` | Чтение серверов из `servers.dat` (несжатый Big-Endian NBT) |
| `net_save_servers` | `NetSaveServersRequest`| `void` | Сохранение списка серверов в `servers.dat` |
| `net_merge_servers` | `NetMergeServersRequest`| `NetMergeServersResponse`| Слияние серверов пака с серверами пользователя без дублей |
| `net_lobby_create` | `NetLobbyCreateRequest` | `LobbyCreateResponseDto` | Создание комнаты в Lobby API |
| `net_lobby_join` | `NetLobbyJoinRequest` | `LobbyJoinResponseDto` | Подключение игрока к комнате лобби по коду |
| `net_lobby_open` | `NetLobbyOpenRequest` | `boolean` | Публикация адреса туннеля хостом лобби |
| `net_lobby_status` | `NetLobbyStatusRequest` | `LobbyStatusResponseDto` | Запрос актуального состояния комнаты и участников |
| `net_lobby_heartbeat`| `NetLobbyHeartbeatRequest`| `boolean` | Отправка регулярного сигнала жизни хоста (раз в 15 с) |
| `net_lobby_close` | `NetLobbyCloseRequest` | `boolean` | Закрытие комнаты лобби хостом |
| `net_lobby_leave` | `NetLobbyLeaveRequest` | `boolean` | Выход гостя из комнаты лобби |
| `net_lobby_kick` | `NetLobbyKickRequest` | `boolean` | Исключение игрока из лобби хостом |
| `net_lobby_tunnel_config` | `void` | `TunnelConfigResponseDto`| Получение токена и конфигурации playit-туннеля |
| `net_compare_manifests`| `NetCompareManifestsRequest`| `NetCompareManifestsResponse`| Сверка манифестов модов хоста и клиента с хешами |
| `net_compute_manifest_hash`| `NetComputeManifestHashRequest`| `NetComputeManifestHashResponse`| Детерминированный 16-символьный SHA-256 хеш модов |
| `net_tunnel_start` | `NetTunnelStartRequest`| `NetTunnelStartResponse` | Запуск `playit-agent` с привязкой к Windows Job Object |
| `net_tunnel_stop` | `void` | `void` | Остановка процесса playit-агента |
| `net_tunnel_status` | `void` | `TunnelStatusDto` | Текущий статус туннеля (Inactive, Starting, Running...) |
| `net_tunnel_ensure_binary`| `NetTunnelEnsureBinaryRequest`| `NetTunnelEnsureBinaryResponse`| Проверка наличия и SHA-256 бинарника `playit.exe` |

### 3.3. Дополнительные сервисы (`wex-extras`)

| Имя команды | Запрос (TypeScript) | Ответ (TypeScript) | Описание |
|---|---|---|---|
| `extras_analyze_crash` | `ExtrasAnalyzeCrashRequest`| `CrashDiagnosisDto` | Анализ логов и краш-дампа с советом на русском языке |
| `extras_validate_skin` | `ExtrasValidateSkinRequest`| `SkinValidationResultDto`| Проверка разрешения PNG скина (64x64 / 64x32) |
| `extras_convert_skin` | `ExtrasConvertSkinRequest`| `ExtrasConvertSkinResponse`| Конвертация старого скина 64x32 в современный 64x64 |
| `extras_save_skin` | `ExtrasSaveSkinRequest` | `WardrobeItemDto` | Сохранение скина в историю гардероба Wex |
| `extras_list_skin_history`| `ExtrasListSkinHistoryRequest`| `ExtrasListSkinHistoryResponse`| Получение истории скинов и плащей пользователя |
| `extras_generate_csl_config`| `ExtrasGenerateCslConfigRequest`| `void` | Генерация конфига для мода CustomSkinLoader |
| `extras_discord_set_activity`| `ExtrasDiscordSetActivityRequest`| `void` | Обновление Discord Rich Presence через локальный IPC-пайп |
| `extras_discord_clear_activity`| `void` | `void` | Очистка статуса Discord Rich Presence |
| `extras_import_old_launcher`| `ExtrasImportOldLauncherRequest`| `OldLauncherDataDto` | Чтение данных `%APPDATA%\.aura` старого лаунчера |
| `extras_get_presets` | `void` | `ExtrasGetPresetsResponse`| Список доступных пресетов графики (Low, Medium, High, Ultra)|
| `extras_apply_preset` | `ExtrasApplyPresetRequest`| `PresetApplyResultDto` | Применение настроек `options.txt` и шейдера из пула |

### 3.4. Социальная подсистема (`wex-social`)

| Имя команды | Запрос (TypeScript) | Ответ (TypeScript) | Описание |
|---|---|---|---|
| `social_login` | `SocialLoginRequest` | `FriendProfileDto` | Вход и загрузка социального профиля игрока |
| `social_get_friends` | `SocialGetFriendsRequest`| `SocialGetFriendsResponse`| Загрузка списка друзей с их текущими статусами |
| `social_send_friend_request`| `SocialSendFriendRequestRequest`| `void` | Отправка заявки в друзья по коду друга |
| `social_accept_friend_request`| `SocialAcceptFriendRequestRequest`| `void` | Принятие входящей заявки в друзья |
| `social_decline_friend_request`| `SocialDeclineFriendRequestRequest`| `void` | Отклонение заявки в друзья |
| `social_sync` | `SocialSyncRequest` | `SyncResponseDto` | Синхронизация списка друзей, входящих заявок и инвайтов |
| `social_send_lobby_invite`| `SocialSendLobbyInviteRequest`| `void` | Отправка приглашения другу в лобби-комнату |
| `social_set_presence` | `SocialSetPresenceRequest`| `void` | Обновление статуса («в лаунчере», «в игре 1.21.1», «в лобби») |
| `social_list_achievements`| `SocialListAchievementsRequest`| `SocialListAchievementsResponse`| Список определений достижений лаунчера и прогресса |
| `social_report_achievement_progress`| `SocialReportAchievementProgressRequest`| `SocialReportAchievementProgressResponse`| Инкремент счётчика прогресса достижения |
| `social_list_worlds` | `SocialListWorldsRequest` | `SocialListWorldsResponse`| Список одиночных миров инстанса (чтение `level.dat` NBT) |
| `social_create_world_backup`| `SocialCreateWorldBackupRequest`| `WorldBackupItemDto` | Создание архивной резервной копии мира в ZIP |
| `social_list_world_backups`| `SocialListWorldBackupsRequest`| `SocialListWorldBackupsResponse`| Список имеющихся резервных копий миров |
| `social_restore_world_backup`| `SocialRestoreWorldBackupRequest`| `SocialRestoreWorldBackupResponse`| Восстановление мира из архива ZIP |
| `social_list_screenshots`| `SocialListScreenshotsRequest`| `SocialListScreenshotsResponse`| Каталог скриншотов с метаданными и датами |
| `social_get_news` | `SocialGetNewsRequest` | `SocialGetNewsResponse` | Получение чейнджлогов и новостей релизов (кеш + сеть) |
| `social_check_launcher_update`| `SocialCheckLauncherUpdateRequest`| `UpdateCheckResultDto` | Проверка наличия новой версии Wex через `latest.json` |

---

## 4. Таблица событий IPC (Rust → UI)

События отправляются асинхронно через шину событий Tauri (`app.emit(EVENT_NAME, payload)`).

| Имя события | Полезная нагрузка (`payload`) | Источник | Назначение в UI |
|---|---|---|---|
| `install-progress` | `InstallProgressPayload` | Загрузчик инстанса | Прогресс-бар загрузки библиотек, модов, ассетов и клиента |
| `launch-status` | `LaunchStatusPayload` | Процесс запуска | Смена состояний: `preparing`, `installing`, `launching`, `running`, `cancelled`, `error` |
| `game-log` | `GameLogPayload` | Дочерний процесс | Вывод строк stdout/stderr Minecraft в консоль разработчика |
| `net-tunnel-status` | `NetTunnelStatusPayload` | Менеджер Playit | Индикатор подключения туннеля и отображение публичного адреса |
| `net-ping-progress` | `NetPingProgressPayload` | Пингер серверов | Прогресс проверки доступности серверов и пинга |
| `net-lobby-status` | `NetLobbyStatusPayload` | Сервис лобби | Уведомления об обновлении комнаты, смене хоста, кике |
| `social-achievement-toast`| `AchievementUnlockedEventDto`| Менеджер достижений| Всплывающий тост в Apple-стиле о разблокировке достижения |
| `social-friend-presence`| `SocialFriendPresencePayload` | Сервис друзей | Мгновенное обновление плашки статуса друга в интерфейсе |
| `social-lobby-invite` | `LobbyInviteItemDto` | Сервис лобби | Всплывающее приглашение в игру от друга с кнопкой «Принять» |
| `launcher-update-available`| `UpdateCheckResultDto` | Служба обновлений | Баннер или плашка предложения обновить Wex |

---

## 5. Использование в TypeScript

Все типы и константы реэкспортируются из `$lib/ipc/generated`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
    COMMANDS,
    EVENTS,
    type LauncherConfig,
    type InstallProgressPayload,
    type ServerPingResponseDto,
    type IpcError
} from '$lib/ipc/generated';

// 1. Вызов команды с типизацией
async function pingServer(ip: string): Promise<ServerPingResponseDto> {
    return await invoke<ServerPingResponseDto>(COMMANDS.NET_PING_SERVER, {
        request: { address: ip, port: null, timeoutMs: 3000 }
    });
}

// 2. Подписка на событие
const unlisten = await listen<InstallProgressPayload>(EVENTS.INSTALL_PROGRESS, (event) => {
    console.log(`Прогресс: ${event.payload.stage} - ${event.payload.percentage}%`);
});
```

---

## 6. Запуск генератора и проверка целостности

1. **Генерация типов TypeScript**:
   ```powershell
   cargo run --bin gen_ts
   ```
2. **Проверка отсутствия расхождений (Drift Test)**:
   ```powershell
   cargo test test_typescript_bindings_drift_detection
   ```
   Тест сравнивает сгенерированные определения с файлами в `ui/src/lib/ipc/generated/` и падает с диффом при любом расхождении.
