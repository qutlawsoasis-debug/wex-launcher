/**
 * Tauri 2.0 Mock IPC for Playwright UI Smoke Tests
 * Supports both 'full' (populated) and 'empty' states.
 */

export interface MockIpcOptions {
  mode: 'full' | 'empty';
  background?: string;
  theme?: 'light' | 'dark';
}

export function setupMockIpc(options: MockIpcOptions = { mode: 'full' }) {
  const mode = options.mode;

  const callbacks = new Map<number, (data: any) => void>();
  let nextId = 1;
  const listeners = new Map<string, number[]>();

  const fullMods = [
    {
      id: 'fabric-api',
      name: 'Fabric API',
      version: '0.102.0',
      description: 'Базовые библиотеки Fabric',
      file_name: 'fabric-api-0.102.0.jar',
      enabled: true,
      is_pack_mod: true,
    },
    {
      id: 'sodium',
      name: 'Sodium',
      version: '0.5.8',
      description: 'Оптимизация рендеринга',
      file_name: 'sodium-0.5.8.jar',
      enabled: true,
      is_pack_mod: true,
    },
    {
      id: 'iris',
      name: 'Iris Shaders',
      version: '1.7.0',
      description: 'Поддержка шейдеров',
      file_name: 'iris-1.7.0.jar',
      enabled: true,
      is_pack_mod: true,
    },
    {
      id: 'minimap',
      name: 'Xaero Minimap',
      version: '24.1.0',
      description: 'Миникарта',
      file_name: 'xaero-minimap-24.1.0.jar',
      enabled: false,
      is_pack_mod: false,
    },
  ];

  const fullServers = [
    {
      id: 'srv-1',
      name: 'Wex Official',
      address: 'play.wex.lan:25565',
      version: '1.21.1',
      ping_ms: 18,
      players_online: 42,
      players_max: 100,
      is_official: true,
      motd: 'Официальный сервер Wex Launcher',
    },
    {
      id: 'srv-2',
      name: 'Survival SMP',
      address: 'mc.example.com',
      version: '1.21.1',
      ping_ms: 55,
      players_online: 12,
      players_max: 50,
      is_official: false,
      motd: 'Классическое выживание',
    },
  ];

  const fullSkins = [
    {
      id: 'skin-1',
      name: 'Hero Steve',
      model: 'classic',
      created_utc: '2026-11-08T10:00:00Z',
      preview_base64: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAAD0lEQVQImWNg+M+ABzBEAABQkA/969vNiwAAAABJRU5ErkJggg==',
    },
    {
      id: 'skin-2',
      name: 'Wizard Alex',
      model: 'slim',
      created_utc: '2026-11-08T11:00:00Z',
      preview_base64: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAAD0lEQVQImWNg+M+ABzBEAABQkA/969vNiwAAAABJRU5ErkJggg==',
    },
  ];

  const fullWorlds = [
    {
      folder_name: 'Survival-1',
      display_name: 'Моё выживание',
      version: '1.21.1',
      last_played_utc: '2026-11-08T17:00:00Z',
      size_bytes: 145000000,
      game_mode: 'survival',
      cheats_enabled: false,
      icon_base64: null,
    },
  ];

  const fullRooms = [
    {
      code: 'CLD1',
      name: 'Комната Alex',
      host_name: 'Alex',
      player_count: 3,
      max_players: 8,
      ping_ms: 24,
      motd: 'Присоединяйтесь!',
      is_locked: false,
      mods_count: 35,
    },
  ];

  const fullFriends = [
    {
      id: 'f1',
      nick: 'Steve',
      status: 'Online',
      last_seen: null,
      game_version: '1.21.1',
      world_name: 'Survival',
      lobby_code: null,
    },
    {
      id: 'f2',
      nick: 'Notch',
      status: 'Offline',
      last_seen: 1731000000000,
      game_version: null,
      world_name: null,
      lobby_code: null,
    },
  ];

  const fullAchievements = [
    { id: 'first_launch', title: 'Первый запуск', description: 'Запустите Wex впервые', target: 1 },
    { id: 'five_worlds', title: 'Создатель миров', description: 'Создайте 5 миров', target: 5 },
  ];

  const fullProgress = [
    { id: 'first_launch', current: 1, unlocked: true, unlocked_at: '2026-11-08T12:00:00Z' },
    { id: 'five_worlds', current: 2, unlocked: false, unlocked_at: null },
  ];

  const mockData: Record<string, any> = {
    // Network & Profile
    shell_check_network: { is_online: true, latency_ms: 22, status_text: 'Онлайн' },
    get_network_status: { is_online: true, latency_ms: 22, status_text: 'Онлайн' },
    shell_get_profile: {
      nickname: 'Alex',
      initials: 'AL',
      is_online: true,
      skin_head_url: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAAD0lEQVQImWNg+M+ABzBEAABQkA/969vNiwAAAABJRU5ErkJggg==',
      total_playtime_seconds: 7200,
      total_playtime_formatted: '2 ч. 0 мин.',
      total_launches: 4,
    },
    is_game_running: false,
    get_system_info: {
      os: 'Windows 11 Pro 64-bit',
      cpu: 'AMD Ryzen 7 7800X3D',
      ram_total: 16384,
      ram_free: 8192,
    },
    get_launcher_settings: {
      selected_version: '1.21.1',
      player_name: 'Alex',
      ram_mb: 4096,
      total_memory_mb: 16384,
      game_dir: 'C:\\Games\\Wex',
      graphics_preset: 'High',
      java_mode: 'auto',
      custom_java_path: null,
      discord_rpc: true,
      language: 'ru',
    },
    save_launcher_settings: true,
    get_playtime_summary: mode === 'full'
      ? {
          total_seconds: 7200,
          total_formatted: '2 ч. 0 мин.',
          version_seconds: 3600,
          version_formatted: '1 ч. 0 мин.',
          version_summary: 'В игре: 1 ч. 0 мин. • Запусков: 2',
          total_launches: 4,
          last_played_utc: '2026-11-08T18:00:00Z',
        }
      : {
          total_seconds: 0,
          total_formatted: '0 ч.',
          version_seconds: 0,
          version_formatted: '0 ч.',
          version_summary: 'В игре: 0 ч. • Запусков: 0',
          total_launches: 0,
          last_played_utc: null,
        },
    get_discord_rpc_preview: {
      is_enabled: true,
      is_connected: true,
      title: 'Wex Launcher',
      details: 'В главном меню',
      state: 'Выбирает режим',
      small_text: null,
      has_timestamp: false,
    },

    // Servers
    list_servers: mode === 'full' ? fullServers : [],
    ping_server: { latency_ms: 24, players_online: 15, players_max: 50, motd: 'Сервер доступен' },
    add_server: (args: any) => ({
      id: `srv-${Date.now()}`,
      name: args?.name || 'New Server',
      address: args?.address || 'play.example.com',
      version: args?.version || '1.21.1',
      is_official: false,
      ping_ms: 25,
      players_online: 5,
      players_max: 50,
      motd: 'Смоук-тестовый сервер',
    }),
    edit_server: true,
    remove_server: true,

    // Launch & Game Session
    launch_game: async (args: any) => {
      if (args && args.server) {
        return `Игра запущена. Подключение к ${args.server}`;
      }
      // Brief delay to allow test assertions during progress state
      await new Promise((r) => setTimeout(r, 600));
      return 'Minecraft 1.21.1 запущен';
    },
    shell_stop_game: async () => true,
    cancel_launch: async () => true,

    // Mods
    list_mods: mode === 'full' ? fullMods : [],
    toggle_mod: (args: any) => ({ ...fullMods[0], enabled: args.enabled }),
    delete_custom_mod: true,
    open_mods_dir: true,

    // Lobby
    list_open_rooms: mode === 'full' ? fullRooms : [],
    list_lan_worlds: mode === 'full' ? [{ motd: 'Мир LAN', address: '192.168.1.100:54321', port: 54321 }] : [],
    connect_lan_world: 'Подключение к миру LAN...',
    publish_lan_world: {
      code: 'LAN-1',
      host_name: 'Alex',
      world_name: 'Мир LAN',
      tunnel_address: 'lan.wex.lan:25565',
      port: 25565,
    },
    create_lobby_room: (args: any) => ({
      code: 'NEW1',
      host_name: args?.playerName || 'Player',
      world_name: args?.worldName || 'Мой мир',
      is_host: true,
      tunnel_active: true,
      tunnel_address: 'tunnel.wex.lan:25565',
      players: [
        { id: 'p1', name: args?.playerName || 'Player', is_host: true, ping_ms: 5, mods_synced: true },
      ],
      mod_mismatch_count: 0,
      total_mods: 42,
    }),
    join_lobby_room: (args: any) => ({
      code: args?.code || 'JOIN1',
      host_name: 'HostAlex',
      world_name: 'Земли приключений',
      is_host: false,
      tunnel_active: true,
      tunnel_address: 'tunnel.wex.lan:25565',
      players: [
        { id: 'p0', name: 'HostAlex', is_host: true, ping_ms: 22, mods_synced: true },
        { id: 'p1', name: args?.playerName || 'Player', is_host: false, ping_ms: 35, mods_synced: true },
      ],
      mod_mismatch_count: 0,
      total_mods: 42,
    }),
    get_lobby_status: {
      code: 'NEW1',
      host_name: 'Player',
      world_name: 'Мой мир',
      is_host: true,
      tunnel_active: true,
      tunnel_address: 'tunnel.wex.lan:25565',
      players: [
        { id: 'p1', name: 'Player', is_host: true, ping_ms: 5, mods_synced: true },
      ],
      mod_mismatch_count: 0,
      total_mods: 42,
    },
    close_lobby_room: true,
    kick_lobby_player: true,

    // Friends & Social
    social_sync: {
      friends: mode === 'full' ? fullFriends : [],
      incoming_requests: mode === 'full' ? [{ id: 'req-1', from_id: 'u2', from_nick: 'FriendBob', created_at: 1731000000000 }] : [],
      incoming_invites: [],
    },
    social_list_achievements: {
      achievements: mode === 'full' ? fullAchievements : [],
      progress: mode === 'full' ? fullProgress : [],
    },
    social_send_friend_request: true,
    social_accept_friend_request: true,
    social_decline_friend_request: true,
    social_send_lobby_invite: true,
    social_report_achievement_progress: { unlocked: false, current: 1, target: 5 },

    // Wardrobe
    extras_list_skin_history: mode === 'full' ? fullSkins : [],
    wardrobe_get_state: mode === 'full' ? { active_skin: { name: 'Hero Steve', model: 'classic' }, active_cape: null } : null,
    wardrobe_apply: true,
    wardrobe_get_skin: { item_id: 'skin-1', name: 'Hero Steve', model: 'classic', skin_base64: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAAD0lEQVQImWNg+M+ABzBEAABQkA/969vNiwAAAABJRU5ErkJggg==' },
    wardrobe_delete_history: true,

    // Workshop
    workshop_list_worlds: mode === 'full' ? fullWorlds : [],
    workshop_list_resource_packs: mode === 'full' ? [{ file_name: 'Faithful-64x.zip', title: 'Faithful 64x', description: 'High resolution', enabled: true, pack_format: 34, icon_base64: null }] : [],
    workshop_list_shader_packs: mode === 'full' ? [{ file_name: 'Complementary.zip', title: 'Complementary Shaders', active: true }] : [],
    workshop_list_screenshots: mode === 'full' ? [{ file_name: '2026-11-08_15.30.00.png', file_path: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAACXBIWXMAAA7EAAAOxAGVKw4bAAAAD0lEQVQImWNg+M+ABzBEAABQkA/969vNiwAAAABJRU5ErkJggg==', width: 1920, height: 1080, created_utc: '2026-11-08T15:30:00Z', size_bytes: 2400000 }] : [],
    workshop_list_world_backups: mode === 'full' ? [{ file_name: 'Survival-1_2026-11-08.zip', world_name: 'Survival-1', created_utc: '2026-11-08T16:00:00Z', size_bytes: 120000000 }] : [],
    workshop_create_world_backup: true,
    workshop_restore_world_backup: true,
    workshop_delete_world: true,
    workshop_toggle_resource_pack: true,
    workshop_set_active_shader_pack: true,
    workshop_open_packs_folder: true,
    workshop_open_path: true,
    workshop_delete_pack_file: true,
    workshop_add_pack_file: 'Faithful-64x.zip',
    workshop_copy_screenshot_image: true,

    // Import & Crash
    extras_detect_old_launcher: null, // do not auto-open import wizard on every page visit
    extras_import_old_launcher: { profiles: ['Default'], accounts: ['Alex'], last_version: '1.21.1' },
    extras_apply_imported_data: { success: true, imported_worlds: 1, imported_skins: 2, message: 'Импорт завершён' },
    crash_open_log_file: 'C:/Games/Wex/instances/1.21.1/logs/latest.log',
    crash_disable_mod: 'optifabric-1.14.0.jar.disabled',
    report_copy_text: 'Report text copied',
    report_save_zip: { success: true, path: 'C:/Games/Wex/report.zip' },
  };

  (window as any).__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main' },
    },
    callbacks,
    transformCallback: (cb: any, once = false) => {
      const id = nextId++;
      callbacks.set(id, (data: any) => {
        if (once) callbacks.delete(id);
        return cb && cb(data);
      });
      return id;
    },
    unregisterCallback: (id: number) => callbacks.delete(id),
    invoke: async (cmd: string, args: any) => {
      if (cmd.startsWith('plugin:event|listen')) {
        const ev = args.event;
        if (!listeners.has(ev)) listeners.set(ev, []);
        listeners.get(ev)!.push(args.handler);
        return args.handler;
      }
      if (cmd.startsWith('plugin:event|emit')) {
        const ev = args.event;
        const handlers = listeners.get(ev) || [];
        for (const h of handlers) {
          const cb = callbacks.get(h);
          if (cb) cb({ event: ev, id: h, payload: args.payload });
        }
        return null;
      }
      if (cmd.startsWith('plugin:event|unlisten')) return null;

      if (cmd in mockData) {
        const val = mockData[cmd];
        return typeof val === 'function' ? val(args) : val;
      }
      return null;
    },
  };
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {};

  (window as any).__TRIGGER_CRASH__ = () => {
    const handlers = listeners.get('game-crash') || [];
    for (const h of handlers) {
      const cb = callbacks.get(h);
      if (cb) {
        cb({
          event: 'game-crash',
          id: h,
          payload: {
            error_type: 'ModConflict',
            title: 'Конфликт модов Fabric',
            summary: 'Обнаружен несовместимый мод',
            recommendation: 'Удалите конфликтующий мод',
            blamed_mod: 'optifabric-1.14.0.jar',
            log_path: 'C:/Games/Wex/instances/1.21.1/logs/latest.log',
            exit_code: 1,
          },
        });
      }
    }
  };
}
