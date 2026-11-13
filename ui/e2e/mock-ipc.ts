/**
 * Tauri 2 IPC stand-in for Playwright (runs in the page before the app).
 * Answers the commands the new interface uses and emits the same events core
 * would (launch-status, install-progress). 'full' has data, 'empty' has none.
 * Test-only: never shipped.
 */

export interface MockIpcOptions {
  mode: 'full' | 'empty';
}

export function setupMockIpc(options: MockIpcOptions = { mode: 'full' }) {
  const full = options.mode === 'full';
  const callbacks = new Map<number, (data: unknown) => void>();
  const listeners = new Map<string, number[]>();
  let nextId = 1;
  const calls: Array<{ cmd: string; args: unknown }> = [];

  const settings: Record<string, unknown> = {
    selected_version: '1.21.1',
    player_name: 'Player',
    ram_mb: 6144,
    total_memory_mb: 16384,
    graphics_preset: 'Medium',
    appearance: 'auto',
    language: 'ru',
    reduce_motion: false,
    reduce_transparency: false,
    discord_rpc: true,
    fast_start_jvm: false,
    friend_notifications: true,
    minimize_to_tray: true,
    game_window_behavior: 'minimize',
  };
  const mods = (version: string) =>
    full
      ? [
          { id: 'fabric-api', name: 'Fabric API', version: '0.110.0', description: '', file_name: 'fabric-api.jar', enabled: true, is_pack_mod: true },
          { id: 'sodium', name: 'Sodium', version: '0.6.5', description: 'Оптимизация', file_name: 'sodium.jar', enabled: true, is_pack_mod: true },
          { id: 'menu', name: 'Mod Menu', version: '11.0.3', description: '', file_name: 'menu.jar', enabled: false, is_pack_mod: false },
        ].concat(version === '1.20.1' ? [] : [{ id: 'iris', name: 'Iris Shaders', version: '1.8.1', description: '', file_name: 'iris.jar', enabled: true, is_pack_mod: true }])
      : [];
  let servers = full
    ? [
        { id: 's1', name: 'Сервер нашей компании', address: 'play.example.org' },
        { id: 's2', name: 'Мир Миши', address: '192.168.1.20:25565' },
      ]
    : [];

  const emit = (event: string, payload: unknown) => {
    for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id, payload });
  };

  let running = false;
  const handlers: Record<string, (args: Record<string, any>) => unknown> = {
    'plugin:event|listen': (a) => {
      listeners.set(a.event, [...(listeners.get(a.event) ?? []), a.handler]);
      return a.handler;
    },
    'plugin:event|unlisten': () => null,
    get_launcher_settings: () => ({ ...settings }),
    save_launcher_settings: (a) => {
      Object.assign(settings, a.newSettings);
      return null;
    },
    reset_settings: () => ({ ...settings }),
    get_installed_versions: () => [
      { id: '1.21.1', name: 'Minecraft 1.21.1', is_installed: true },
      { id: '1.20.1', name: 'Minecraft 1.20.1', is_installed: full },
    ],
    list_mods: (a) => mods(a.version),
    toggle_mod: (a) => ({ ...mods(a.version).find((m) => m.file_name === a.fileName), enabled: a.enabled }),
    workshop_list_worlds: (a) =>
      full && a.version === '1.20.1'
        ? [{ folder_name: 'w', folder_path: 'C:/Wex/instances/1.20.1/saves/w', display_name: 'Северная база', game_mode: 'Creative', last_played: Date.now(), day_time: 24000 * 30, size_bytes: 1 }]
        : [],
    workshop_list_resource_packs: () => [],
    workshop_list_shader_packs: () => [],
    workshop_list_screenshots: () => [],
    get_data_dir: () => 'C:\\Users\\Player\\AppData\\Roaming\\Wex',
    get_storage_cache_info: () => ({ total_bytes: full ? 1288490188 : 0, formatted_size: full ? '1,2 ГБ' : '0 Б' }),
    startup_get_info: () => ({ app_version: '0.9.2' }),
    extras_detect_old_launcher: () => null,
    shell_get_launch_state: () => ({ is_running: running, is_launching: false, running_version: null }),
    get_prewarm_status: () => ({ is_prewarming: false, is_ready: full, progress: 0, stage: '' }),
    get_network_status: () => ({ is_online: true }),
    overview_get_news: () =>
      full ? [{ version: '0.9.2', date_text: '10 ноября', short_title: 'Что нового в 0.9.2', items: [{ text: 'Обновление без окна установщика.' }] }] : [],
    list_servers: () => servers,
    ping_server_details: (a) => (a.address === 'play.example.org' ? { success: true, latency_ms: 23, online: 3, max_players: 10, motd: 'Свои' } : { success: false }),
    add_server: (a) => {
      servers = [...servers, { id: a.address, name: a.name, address: a.address }];
      return servers[servers.length - 1];
    },
    remove_server: (a) => {
      servers = servers.filter((s) => s.address !== a.address);
      return null;
    },
    social_sync: () => ({ friends: [], incoming_requests: [], incoming_invites: [] }),
    social_list_achievements: () => ({
      achievements: [{ id: 'first', title: 'Первый запуск', description: 'Запустите игру', target: 1 }],
      progress: [{ id: 'first', current: 1, unlocked: true, unlocked_at: null }],
    }),
    wardrobe_get_state: () => ({ nickname: 'Player', skin_name: '', skin_base64: null, model: 'Classic', cape_id: 'none', cape_name: '', cape_base64: null }),
    wardrobe_apply: () => ({ success: true, message: 'ok' }),
    extras_list_skin_history: () => ({ items: [] }),
    own_server_list: () =>
      full
        ? [
            {
              id: 'srv-0000aaaa',
              version: '1.21.1',
              createdAt: '2026-11-13T00:00:00Z',
              settings: { name: 'Дача', maxPlayers: 8, gamemode: 'survival', difficulty: 'normal', whitelist: true, onlineMode: true, pvp: true, port: 25565, memoryMb: 3072, publicAddress: null },
              eulaAccepted: false,
              installed: { minecraft: '1.21.1', fabricLoader: '0.19.5', packVersion: '1', modSource: 'fabricModJson', mods: ['a.jar'] },
              installing: false,
              status: 'stopped',
              players: [],
              whitelist: ['Steve'],
              lanAddress: '192.168.1.5',
              localAddress: 'localhost',
              publicAddress: null,
              problem: null,
            },
          ]
        : [],
    own_server_console: () => null,
    list_lan_worlds: () => [],
    list_open_rooms: () => [],
    get_pending_deep_link: () => null,
    start_game: (a) => {
      // Same order of events core sends: preparing → installing with progress → running.
      setTimeout(() => emit('launch-status', { state: 'preparing', message: 'Готовим запуск' }), 50);
      setTimeout(() => {
        emit('launch-status', { state: 'installing', message: 'Скачиваем ресурсы игры' });
        emit('install-progress', { stage: 'Скачиваем ресурсы игры', percentage: 47, downloaded_bytes: 182 * 1048576, total_bytes: 388 * 1048576 });
      }, 150);
      setTimeout(() => {
        running = true;
        emit('launch-status', { state: 'running', message: 'Minecraft запущен' });
      }, 400);
      return { message: `started ${a.version}` };
    },
    shell_stop_game: () => {
      running = false;
      setTimeout(() => emit('launch-status', { state: 'idle', message: 'Игра остановлена' }), 50);
      return null;
    },
    cancel_launch: () => {
      setTimeout(() => emit('launch-status', { state: 'cancelled', message: 'Запуск отменён' }), 50);
      return null;
    },
  };

  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    transformCallback(cb: (data: unknown) => void) {
      const id = nextId++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback(id: number) {
      callbacks.delete(id);
    },
    async invoke(cmd: string, args: Record<string, any> = {}) {
      calls.push({ cmd, args });
      const h = handlers[cmd];
      return h ? h(args) : null;
    },
  };
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  w.__ipcCalls = calls;
  w.__emit = emit;
  try {
    localStorage.setItem('wex_onboarding_completed', 'true');
  } catch {}
}
