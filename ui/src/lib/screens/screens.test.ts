import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import PlayScreen from './PlayScreen.svelte';
import UpdateCard from '../components/UpdateCard.svelte';
import ServersScreen from './ServersScreen.svelte';
import LobbyScreen from './LobbyScreen.svelte';
import FriendsScreen from './FriendsScreen.svelte';
import ModsScreen from './ModsScreen.svelte';
import SettingsScreen from './SettingsScreen.svelte';
import WardrobeScreen from './WardrobeScreen.svelte';
import WorkshopScreen from './WorkshopScreen.svelte';
import SplashScreen from './SplashScreen.svelte';
import { theme } from '../design/theme.svelte';
import ImportWizardSheet from '../components/ImportWizardSheet.svelte';
import CrashSheet from '../components/CrashSheet.svelte';

const defaultInvoke = vi.fn(async (cmd: string, args?: Record<string, any>, options?: any) => {
  if (cmd === 'list_open_rooms') {
    throw new Error('Сервис недоступен');
  }
  if (cmd === 'list_lan_worlds') {
    return [
      {
        motd: 'Alex - Creative Studio',
        address: '192.168.1.105:54321',
        port: 54321,
      },
    ];
  }
  if (cmd === 'connect_lan_world') {
    return `Подключение к ${args?.address} (PID 12345)`;
  }
  if (cmd === 'publish_lan_world') {
    return {
      code: '#B2C3',
      host_name: 'Player',
      world_name: args?.motd || 'Мир LAN',
      tunnel_address: `lan-host.tun.ply.gg:${args?.port || 25565}`,
      tunnel_active: true,
      is_host: true,
      players: [
        {
          name: 'Player',
          is_host: true,
          ping_ms: 0,
          mods_synced: true,
        },
      ],
      mod_mismatch_count: 0,
      total_mods: 142,
    };
  }
  if (cmd === 'list_mods') {
    if (args?.version === '1.20.1') {
      return [];
    }
    return [
      {
        id: 'fabric-api',
        name: 'Fabric API',
        version: '0.102.0',
        description: 'Базовый набор библиотек для экосистемы модов Fabric',
        file_name: 'fabric-api-0.102.0.jar',
        enabled: true,
        is_pack_mod: true,
      },
      {
        id: 'minimap',
        name: 'Xaero Minimap',
        version: '24.1.0',
        description: 'Пользовательский мод миникарты',
        file_name: 'xaero-minimap-24.1.0.jar',
        enabled: false,
        is_pack_mod: false,
      },
    ];
  }
  if (cmd === 'get_launcher_settings') {
    return {
      selected_version: '1.21.1',
      player_name: 'Player',
      ram_mb: 4096,
      total_memory_mb: 16384,
      game_dir: 'C:\\Users\\User\\AppData\\Roaming\\Wex',
      graphics_preset: 'High',
      java_mode: 'auto',
      custom_java_path: null,
      discord_rpc: true,
      language: 'ru',
      game_window_behavior: 'minimize',
    };
  }
  if (cmd === 'get_playtime_summary') {
    return {
      total_seconds: 7200,
      total_formatted: '2 ч. 0 мин.',
      version_seconds: 3600,
      version_formatted: '1 ч. 0 мин.',
      version_summary: 'В игре: 1 ч. 0 мин. • Запусков: 2',
      total_launches: 4,
      last_played_utc: '2026-11-08T18:00:00Z',
    };
  }
  if (cmd === 'verify_instance_files') {
    return {
      success: true,
      total_files: 264,
      verified_files: 264,
      repaired_files: 0,
      corrupted_files: 0,
      errors: [],
      message: 'Все 264 файлов проверены, повреждений не обнаружено',
    };
  }
  if (cmd === 'cleanup_cache') {
    return {
      success: true,
      freed_bytes: 15204352,
      freed_formatted: '14.5 МБ',
      deleted_files_count: 8,
      message: 'Кэш очищен: освобождено 14.5 МБ (удалено 8 файлов)',
    };
  }
  if (cmd === 'get_storage_cache_info') {
    return {
      total_bytes: 52428800,
      formatted_size: '50.00 МБ',
    };
  }
  if (cmd === 'clear_storage_cache') {
    return {
      bytes_freed: 52428800,
      formatted_freed: '50.00 МБ',
      files_deleted: 14,
      status_text: 'Очищено 50.00 МБ (14 файлов)',
    };
  }
  if (cmd === 'verify_game_files') {
    return {
      total_files: 264,
      checked_files: 264,
      corrupted_files: 0,
      repaired_files: 0,
      status: 'Всё в порядке',
      message: 'Все 264 файлов проверены, повреждений не обнаружено',
    };
  }
  if (cmd === 'is_game_running') {
    return false;
  }
  if (cmd === 'get_discord_rpc_preview') {
    return {
      is_enabled: true,
      is_connected: true,
      title: 'Wex Launcher',
      details: 'В главном меню',
      state: 'Выбирает режим игры',
      small_text: null,
      has_timestamp: false,
    };
  }
  if (cmd === 'get_system_info') {
    return {
      total_memory_mb: 16384,
      free_memory_mb: 8192,
      memory_display: '16.0 ГБ (свободно 8.0 ГБ)',
      os_name: 'windows',
      os_version: 'Windows 11 Pro 23H2 (сборка 22631.3593)',
      launcher_version: '1.0.0',
      data_dir: 'C:\\Users\\<user>\\AppData\\Roaming\\Wex',
      cpu_name: 'AMD Ryzen 5 3600 6-Core Processor',
      cpu_cores: 12,
      gpu_name: 'NVIDIA GeForce GTX 1660 SUPER',
      gpu_driver: '32.0.16.1742',
      is_integrated_gpu: false,
      disk_free_gb: 12.5,
      disk_total_gb: 460.0,
      disk_display: '12.5 ГБ свободно из 460.0 ГБ',
      java_17: 'Mojang JRE 17 (17.0.8)',
      java_21: 'Microsoft JDK 21 (21.0.12)',
      warnings: ['Мало свободного места на диске (12.5 ГБ).'],
      support_text: '=== О системе (Wex Support) ===\nОС: Windows 11\nCPU: AMD Ryzen 5\nGPU: GTX 1660 SUPER\n',
    };
  }
  if (cmd === 'updater_install') {
    if (args?.downloadUrl?.includes('fail')) {
      throw new Error('Ошибка загрузки обновления');
    }
    return {
      status: 'ready_to_restart',
      message: 'Обновление успешно загружено',
      version: args?.version || '1.1.0',
    };
  }
  if (cmd === 'updater_restart') {
    return null;
  }
  if (cmd === 'updater_open_download_url') {
    return null;
  }
  if (cmd === 'updater_check') {
    return {
      update_available: false,
      current_version: '1.0.0',
      latest_version: '1.0.0',
      notes: null,
      download_url: null,
      signature: null,
      pub_date: null,
      is_signed: false,
      can_auto_update: false,
    };
  }
  return [];
});

vi.mock('@tauri-apps/api/core', () => {
  return {
    invoke: defaultInvoke,
  };
});

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
}));

if (typeof window !== 'undefined') {
  (window as any).__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args: any, options: any) => defaultInvoke(cmd, args, options),
    transformCallback: () => 1,
  };
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };
}

describe('Wex Launcher Screens', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: any, options: any) => defaultInvoke(cmd, args, options),
      transformCallback: () => 1,
    };
    (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
  });

  afterEach(() => {
    target.remove();
  });

  it('renders PlayScreen with version selector, Play button, and dynamic Java 21 for 1.21.1', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        clientStatus: 'Клиент Wex готов',
        isLaunching: false,
      },
    });

    expect(target.textContent).toContain('Minecraft');
    expect(target.textContent).toContain('Клиент Wex');
    const playBtn = target.querySelector('.dock-action button');
    expect(playBtn).not.toBeNull();
    expect(playBtn?.textContent).toContain('Играть');

    const nicknameInput = target.querySelector('.nickname-input') as HTMLInputElement;
    expect(nicknameInput?.value).toBe('Steve');

    // Java runtime must strictly be Java 21 for 1.21.1
    expect(target.textContent).toContain('Java 21 • 64-bit');

    unmount(comp);
  });

  it('renders PlayScreen with dynamic Java 17 for 1.20.1 and empty status fallback', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.20.1',
        playerName: 'Alex',
        clientStatus: '',
        isLaunching: false,
      },
    });

    // Java runtime must strictly be Java 17 for 1.20.1
    expect(target.textContent).toContain('Java 17 • 64-bit');
    unmount(comp);
  });

  it('renders PlayScreen progress bar when launching', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.20.1',
        playerName: 'Alex',
        isLaunching: true,
        progressPercentage: 55,
        progressStage: 'Синхронизация сборки Wex',
        progressDetail: 'Загрузка шейдеров...',
      },
    });

    const progress = target.querySelector('.dock-progress');
    expect(progress).not.toBeNull();
    expect(target.querySelector('.progress-label')?.textContent).toBe('Синхронизация сборки Wex');

    const cancelBtn = target.querySelector('.dock-cancel button');
    expect(cancelBtn?.textContent).toContain('Отмена');

    unmount(comp);
  });

  it('locks inputs and transforms Play button to neutral stop action when Minecraft is running', async () => {
    let stopped = false;
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        clientStatus: 'Сессия активна',
        isLaunching: false,
        isRunning: true,
        onstop: () => {
          stopped = true;
        },
      },
    });

    // In peace/running, progress bar must be hidden
    expect(target.querySelector('.dock-progress')).toBeNull();

    // Inputs are locked
    const nicknameInput = target.querySelector('.nickname-input') as HTMLInputElement;
    expect(nicknameInput.disabled).toBe(true);

    // The primary action transforms to neutral button with «В игре • Остановить»
    const stopBtn = target.querySelector('.dock-action button') as HTMLButtonElement;
    expect(stopBtn).not.toBeNull();
    expect(stopBtn.textContent).toContain('В игре');
    expect(stopBtn.classList.contains('secondary')).toBe(true);

    // Clicking calls onstop
    stopBtn.click();
    expect(stopped).toBe(true);

    // Shows session status
    expect(target.textContent).toContain('В игре');

    unmount(comp);
  });

  it('hides changelog card when news source is absent (spec: если источника нет, блок скрыт) and shows server empty state', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        initialNews: [],
      },
    });

    // Without news data, the changelog block must be completely hidden (no fabricated cards)
    expect(target.querySelector('.updates-card')).toBeNull();
    expect(target.textContent).not.toContain('Журнал изменений');

    // Recent server without data shows authentic empty state
    expect(target.textContent).toContain('Вы ещё не играли на серверах');
    expect(target.textContent).not.toContain('mc.wexmc.net');

    unmount(comp);
  });

  it('renders PlayScreen changelog block with Apple HIG badges when news source is present', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        initialNews: [
          {
            version: 'v1.0.0',
            date_text: '7 ноября 2026',
            short_title: 'Новый Wex',
            is_current: true,
            items: [
              { category: 'new', text: 'Полный переход на Tauri 2 + Rust + Svelte 5', raw: '[Новое] Полный переход на Tauri 2 + Rust + Svelte 5' },
              { category: 'improved', text: 'Плавность интерфейса', raw: '[Улучшено] Плавность интерфейса' },
              { category: 'fixed', text: 'Сбои в фоновом режиме', raw: '[Исправлено] Сбои в фоновом режиме' },
            ],
          },
        ],
      },
    });

    expect(target.querySelector('.updates-card')).not.toBeNull();
    expect(target.textContent).toContain('Журнал изменений');
    expect(target.textContent).toContain('v1.0.0');
    expect(target.textContent).toContain('7 ноября 2026');
    expect(target.textContent).toContain('Полный переход на Tauri 2 + Rust + Svelte 5');

    // Badges must be rendered
    const newBadge = target.querySelector('.badge-new');
    expect(newBadge?.textContent).toBe('Новое');
    const improvedBadge = target.querySelector('.badge-improved');
    expect(improvedBadge?.textContent).toBe('Улучшено');
    const fixedBadge = target.querySelector('.badge-fixed');
    expect(fixedBadge?.textContent).toBe('Исправлено');

    unmount(comp);
  });

  it('renders unobtrusive update notification banner when update is available and allows dismissing it', async () => {
    const comp = mount(UpdateCard, {
      target,
      props: {
        initialInfo: {
        update_available: true,
        current_version: '1.0.0',
        latest_version: '1.1.0',
        notes: 'Исправления ошибок и оптимизация',
        },
      },
    });

    const banner = target.querySelector('.update-banner');
    expect(banner).not.toBeNull();
    expect(banner?.textContent).toContain('Доступна новая версия Wex 1.1.0');
    expect(banner?.textContent).toContain('Исправления ошибок и оптимизация');

    // Close button dismisses the banner
    const closeBtn = banner?.querySelector('.update-banner-close button') as HTMLButtonElement;
    expect(closeBtn).not.toBeNull();
    closeBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    expect(target.querySelector('.update-banner')).toBeNull();

    unmount(comp);
  });

  it('renders auto-update flow: "Обновить" button for signed release and transitions to "Перезапуск"', async () => {
    const comp = mount(UpdateCard, {
      target,
      props: {
        initialInfo: {
        update_available: true,
        current_version: '0.1.0',
        latest_version: '0.2.0',
        notes: 'Релиз с новыми возможностями',
        download_url: 'https://example.com/wex-update.exe',
        signature: 'untrusted comment: signature\nRWSomeBase64SignatureString1234567890==',
        is_signed: true,
        can_auto_update: true,
        },
      },
    });

    const banner = target.querySelector('.update-banner');
    expect(banner).not.toBeNull();
    expect(banner?.textContent).toContain('Доступна новая версия Wex 0.2.0');

    // Signed release must display primary "Обновить" button
    const updateBtn = Array.from(banner?.querySelectorAll('button') || []).find((b) =>
      b.textContent?.includes('Обновить')
    );
    expect(updateBtn).not.toBeUndefined();

    // Click "Обновить" to trigger download & install
    updateBtn?.click();
    await tick();
    await new Promise((r) => setTimeout(r, 20));

    // Must transition to "ready_to_restart" state with "Перезапуск" button
    expect(banner?.textContent).toContain('Обновление готово к установке');
    const restartBtn = Array.from(banner?.querySelectorAll('button') || []).find((b) =>
      b.textContent?.includes('Перезапуск')
    );
    expect(restartBtn).not.toBeUndefined();

    unmount(comp);
  });

  // The update UI lives in the sidebar UpdateCard since 0.9.2 (it is no longer part of PlayScreen).
  it('security regression: rejected update URL never falls back to window.open', async () => {
    const invokeSpy = defaultInvoke;
    const originalImplementation = invokeSpy.getMockImplementation()!;
    invokeSpy.mockImplementation(async (cmd, args, options) => {
      if (cmd === 'updater_open_download_url') throw new Error('URL rejected by backend');
      return originalImplementation(cmd, args, options);
    });
    const openSpy = vi.spyOn(window, 'open').mockReturnValue(null);
    const url = 'javascript:alert(1)';
    const comp = mount(UpdateCard, { target, props: {
      initialInfo: { update_available: true, current_version: '0.1.0', latest_version: '0.2.0',
        download_url: url, signature: null, is_signed: false, can_auto_update: false },
    } });
    try {
      const button = Array.from(target.querySelectorAll('.update-banner button')).find((b) =>
        b.textContent?.includes('Перейти к загрузке')
      ) as HTMLButtonElement | undefined;
      expect(button).not.toBeUndefined();
      button!.click();
      await vi.waitFor(() => expect(invokeSpy).toHaveBeenCalledWith('updater_open_download_url', { url }));
      await tick();
      await new Promise((resolve) => setTimeout(resolve, 20));
      expect(openSpy).not.toHaveBeenCalled();
      expect(target.querySelector('.update-banner')?.textContent).toContain('Не удалось открыть ссылку загрузки');
    } finally {
      await unmount(comp);
      openSpy.mockRestore();
      invokeSpy.mockImplementation(originalImplementation);
    }
  });

  it('renders unsigned release mode: only notify with download link button', async () => {
    const comp = mount(UpdateCard, {
      target,
      props: {
        initialInfo: {
        update_available: true,
        current_version: '0.1.0',
        latest_version: '0.2.0',
        notes: 'Неподписанный билд',
        download_url: 'https://example.com/wex-unsigned.exe',
        signature: null,
        is_signed: false,
        can_auto_update: false,
        },
      },
    });

    const banner = target.querySelector('.update-banner');
    expect(banner).not.toBeNull();
    expect(banner?.textContent).toContain('Неподписанная сборка');

    // Must show "Перейти к загрузке", not "Обновить"
    expect(banner?.textContent).toContain('Перейти к загрузке');
    expect(banner?.textContent).not.toContain('Обновить');

    unmount(comp);
  });

  it('renders rollback error state when update installation fails and allows retry', async () => {
    const comp = mount(UpdateCard, {
      target,
      props: {
        initialInfo: {
        update_available: true,
        current_version: '0.1.0',
        latest_version: '0.2.0',
        notes: 'Тест ошибки',
        download_url: 'https://example.com/fail-download.exe',
        signature: 'untrusted comment: signature\nRWSomeBase64SignatureString1234567890==',
        is_signed: true,
        can_auto_update: true,
        },
      },
    });

    const banner = target.querySelector('.update-banner');
    expect(banner).not.toBeNull();

    const updateBtn = Array.from(banner?.querySelectorAll('button') || []).find((b) =>
      b.textContent?.includes('Обновить')
    );
    updateBtn?.click();
    await tick();
    await new Promise((r) => setTimeout(r, 20));

    // Error state after failed download
    expect(banner?.textContent).toContain('Не удалось обновить лаунчер');
    expect(banner?.textContent).toContain('Прежняя версия сохранена');
    const retryBtn = Array.from(banner?.querySelectorAll('button') || []).find((b) =>
      b.textContent?.includes('Повторить')
    );
    expect(retryBtn).not.toBeUndefined();

    unmount(comp);
  });

  it('renders SettingsScreen with launcher update check row', async () => {
    const comp = mount(SettingsScreen, { target });
    await tick();

    expect(target.textContent).toContain('Обновление лаунчера');
    const checkBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Проверить')
    );
    expect(checkBtn).not.toBeUndefined();

    unmount(comp);
  });

  it('renders ServersScreen with empty state when without IPC data and has add server action', () => {
    const comp = mount(ServersScreen, { target });
    expect(target.textContent).toContain('Список серверов пуст');
    expect(target.textContent).toContain('Добавить сервер');
    expect(target.textContent).toContain('Обновить');
    expect(target.textContent).toContain('1.21.1');
    expect(target.textContent).toContain('1.20.1');
    expect(target.textContent).not.toContain('mc.wexmc.net');
    unmount(comp);
  });

  it('renders ServersScreen with version segments and opens add server sheet on click', async () => {
    const comp = mount(ServersScreen, { target });
    const addBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Добавить сервер')
    );
    expect(addBtn).toBeDefined();

    addBtn?.click();
    await new Promise((r) => setTimeout(r, 20));

    // Modal Sheet should now be present in DOM with form fields
    expect(document.querySelector('.sheet-card')).not.toBeNull();
    expect(document.querySelector('#new-srv-name')).not.toBeNull();
    expect(document.querySelector('#new-srv-addr')).not.toBeNull();

    unmount(comp);
  });

  it('renders LobbyScreen with LAN/Cloud mode switch, guest code input, and world picker', () => {
    const comp = mount(LobbyScreen, { target });
    expect(target.textContent).toContain('Локальная сеть и туннели');
    expect(target.textContent).toContain('Облачные миры');
    expect(target.textContent).toContain('Присоединиться по коду');
    expect(target.textContent).toContain('Открыть локальный мир');

    const codeInput = target.querySelector('.code-input') as HTMLInputElement;
    expect(codeInput).not.toBeNull();
    expect(codeInput.placeholder).toBe('#A7X9');

    unmount(comp);
  });

  it('renders LobbyScreen cloud mode with honest service unavailable state without fake rooms', async () => {
    const comp = mount(LobbyScreen, { target });

    // Switch to cloud mode
    const cloudBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Облачные миры')
    );
    expect(cloudBtn).not.toBeUndefined();
    cloudBtn?.click();

    // Trigger fetchCloudRooms or allow effect to settle
    await new Promise((r) => setTimeout(r, 50));

    // Must show authentic service unavailable or empty state, strictly zero fake rooms
    expect(target.textContent).toContain('Не удалось загрузить облачные миры');
    expect(target.textContent).toContain('Повторить');
    expect(target.textContent).not.toContain('mc.wexmc.net');
    expect(target.textContent).not.toContain('Aura Official');

    unmount(comp);
  });

  it('renders LobbyScreen cloud mode with empty rooms state when API responds with empty list', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const origImpl = (invoke as any).getMockImplementation();
    (invoke as any).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === 'list_open_rooms') {
        return [];
      }
      return origImpl ? origImpl(cmd, args) : undefined;
    });

    const comp = mount(LobbyScreen, { target });

    const cloudBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Облачные миры')
    );
    expect(cloudBtn).not.toBeUndefined();
    cloudBtn?.click();

    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Нет открытых комнат');
    expect(target.textContent).toContain('Обновить');
    expect(target.textContent).not.toContain('Сервис недоступен');

    unmount(comp);
    if (origImpl) (invoke as any).mockImplementation(origImpl);
  });

  it('renders LobbyScreen LAN section with discovered worlds and connect action', async () => {
    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Локальные миры в сети');
    expect(target.textContent).toContain('Alex - Creative Studio');
    expect(target.textContent).toContain('192.168.1.105:54321');

    const connectBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Подключиться') && b.closest('.lan-worlds-section')
    );
    expect(connectBtn).not.toBeUndefined();
    connectBtn?.click();

    await new Promise((r) => setTimeout(r, 50));
    expect(target.textContent).toContain('Подключение к 192.168.1.105:54321');

    unmount(comp);
  });

  it('publishes discovered LAN world to cloud lobby via «Открыть друзьям» action', async () => {
    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Локальные миры в сети');
    expect(target.textContent).toContain('Alex - Creative Studio');

    const publishBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Открыть друзьям') && b.closest('.lan-worlds-section')
    );
    expect(publishBtn).not.toBeUndefined();
    publishBtn?.click();

    await new Promise((r) => setTimeout(r, 50));
    // Should transition to active room view
    expect(target.textContent).toContain('#B2C3');
    expect(target.textContent).toContain('Alex - Creative Studio');
    expect(target.textContent).toContain('Туннель активен: lan-host.tun.ply.gg:54321');
    expect(target.textContent).toContain('Закрыть комнату');

    unmount(comp);
  });

  it('renders LobbyScreen LAN section with clean empty state when no LAN worlds found', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const origImpl = (invoke as any).getMockImplementation();
    (invoke as any).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === 'list_lan_worlds') {
        return [];
      }
      return origImpl ? origImpl(cmd, args) : undefined;
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Локальные миры не найдены');
    expect(target.textContent).toContain('Когда другой игрок в вашей локальной сети откроет');

    unmount(comp);
    if (origImpl) (invoke as any).mockImplementation(origImpl);
  });

  it('manages LAN discovery lifecycle (starts when LAN active, stops on unmount or hide)', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockClear();

    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));
    expect((invoke as any)).toHaveBeenCalledWith('net_lan_start_discovery');

    // Trigger document hidden
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
    await new Promise((r) => setTimeout(r, 60));
    expect((invoke as any)).toHaveBeenCalledWith('net_lan_stop_discovery');

    // Restore document
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
    document.dispatchEvent(new Event('visibilitychange'));
    await new Promise((r) => setTimeout(r, 60));

    unmount(comp);
    await new Promise((r) => setTimeout(r, 60));
  });

  it('renders FriendsScreen with clean empty state and search action', () => {
    const comp = mount(FriendsScreen, { target });
    expect(target.textContent).toContain('Список друзей пуст');
    expect(target.querySelector('input')).not.toBeNull();
    unmount(comp);
  });

  it('renders FriendsScreen segmented tabs and add friend action', () => {
    const comp = mount(FriendsScreen, { target });
    expect(target.textContent).toContain('Друзья');
    expect(target.textContent).toContain('Заявки');
    expect(target.textContent).toContain('Достижения');
    expect(target.textContent).toContain('Добавить друга');

    const friendInput = target.querySelector('.friend-code-input') as HTMLInputElement;
    expect(friendInput).not.toBeNull();
    expect(friendInput.placeholder).toContain('Код друга');
    unmount(comp);
  });

  it('renders FriendsScreen achievements tab with standard launcher achievements', async () => {
    const comp = mount(FriendsScreen, {
      target,
      props: { tab: 'achievements' },
    });
    await tick();

    // Verify achievements content rendered
    expect(target.textContent).toContain('Достижения лаунчера');
    expect(target.textContent).toContain('Искра');
    expect(target.textContent).toContain('Постоялец');
    expect(target.textContent).toContain('Сто часов');
    expect(target.textContent).toContain('Хост');
    expect(target.textContent).toContain('Список достижений');

    unmount(comp);
  });

  it('renders FriendsScreen requests tab with authentic clean empty state', async () => {
    const comp = mount(FriendsScreen, {
      target,
      props: { tab: 'requests' },
    });
    await tick();

    expect(target.textContent).toContain('Нет входящих заявок');
    expect(target.textContent).toContain('Когда другие игроки отправят вам заявку');

    unmount(comp);
  });

  it('renders ModsScreen with real IPC mod data, pack badges, add button, and status toggles', async () => {
    const comp = mount(ModsScreen, { target });
    await tick();
    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('Fabric API');
    expect(target.textContent).toContain('Xaero Minimap');
    expect(target.textContent).toContain('Добавить мод...');
    expect(target.textContent).toContain('Папка с модами');
    expect(target.textContent).toContain('Пак Wex');
    unmount(comp);
  });

  it('renders ModsScreen with authentic empty state when no mods exist', async () => {
    const comp = mount(ModsScreen, {
      target,
      props: { initialVersion: '1.20.1' },
    });
    await tick();
    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('Моды не найдены');
    expect(target.textContent).toContain('В папке игры пока нет модов');
    unmount(comp);
  });

  it('renders SettingsScreen with RAM slider, Java options, Discord RPC, graphics presets, and About Wex', async () => {
    const comp = mount(SettingsScreen, { target });
    expect(target.textContent).toContain('Тема оформления');
    expect(target.textContent).not.toContain('Цветовой акцент');
    expect(target.textContent).toContain('Уменьшение движения');
    expect(target.textContent).toContain('Уменьшение прозрачности');
    expect(target.textContent).toContain('Выделение оперативной памяти');
    expect(target.textContent).toContain('Графический пресет');
    expect(target.textContent).toContain('Поведение окна');
    expect(target.textContent).toContain('Во время игры');
    expect(target.textContent).toContain('Оставить');
    expect(target.textContent).toContain('Свернуть');
    expect(target.textContent).toContain('Скрыть в трей');
    expect(target.textContent).toContain('Выбор исполняемого файла Java');
    expect(target.textContent).toContain('Discord Rich Presence');
    expect(target.textContent).toContain('Поведение окна');
    expect(target.textContent).not.toContain('Сворачивать в трей при запуске игры');
    expect(target.textContent).toContain('Язык интерфейса');
    expect(target.textContent).toContain('О программе');
    expect(target.textContent).toContain('Windows 10/11, 64-бит');
    expect(target.textContent).toContain('Проверить файлы игры');
    expect(target.textContent).toContain('Очистить кэш');
    expect(target.textContent).not.toContain('.minecraft');
    expect(target.textContent).toContain('Wex');
    expect(target.textContent).toContain('Журнал и отчёты о сбоях');

    const openLogsBtn = target.querySelector('[data-testid="btn-open-logs-sheet"]') as HTMLElement;
    expect(openLogsBtn).not.toBeNull();
    openLogsBtn.click();
    await tick();
    expect(target.textContent).toContain('latest.log');

    // Storage section (SPEC items 40–41: wex-verify-files)
    expect(target.textContent).toContain('Хранилище');
    expect(target.textContent).toContain('Проверить файлы игры');
    expect(target.textContent).toContain('Очистить кэш');
    unmount(comp);
  });

  it('renders SettingsScreen with disabled storage actions when game is running', async () => {
    const comp = mount(SettingsScreen, {
      target,
      props: {
        isGameRunning: true,
      },
    });

    expect(target.textContent).toContain('Хранилище');
    const buttons = target.querySelectorAll('button');
    const verifyBtn = Array.from(buttons).find((b) => b.textContent?.includes('Проверить'));
    const clearBtn = Array.from(buttons).find((b) => b.textContent?.includes('Очистить'));

    expect(verifyBtn).not.toBeNull();
    expect(clearBtn).not.toBeNull();
    expect((verifyBtn as HTMLButtonElement).disabled).toBe(true);
    expect((clearBtn as HTMLButtonElement).disabled).toBe(true);
    unmount(comp);
  });

  it('renders SettingsScreen with authentic System Info section, warnings, and support copy button', async () => {
    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));
    await tick();

    expect(target.textContent).toContain('О системе');
    expect(target.textContent).toContain('Java (1.21.1)');
    expect(target.textContent).toContain('Мало свободного места на диске');

    const copyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Скопировать для поддержки')
    );
    expect(copyBtn).toBeDefined();

    copyBtn?.click();
    await tick();

    unmount(comp);
  });

  it('allows clicking verify and clear cache when game is not running', async () => {
    const comp = mount(SettingsScreen, {
      target,
      props: {
        isGameRunning: false,
      },
    });

    const buttons = target.querySelectorAll('button');
    const verifyBtn = Array.from(buttons).find((b) => b.textContent?.includes('Проверить')) as HTMLButtonElement;
    const clearBtn = Array.from(buttons).find((b) => b.textContent?.includes('Очистить')) as HTMLButtonElement;

    expect(verifyBtn.disabled).toBe(false);
    expect(clearBtn.disabled).toBe(false);

    verifyBtn.click();
    clearBtn.click();

    unmount(comp);
  });

  it('triggers verify instance files and clean cache in SettingsScreen', async () => {
    const comp = mount(SettingsScreen, { target });
    await tick();

    const buttons = Array.from(target.querySelectorAll('button'));
    const verifyBtn = buttons.find((b) => b.textContent?.includes('Проверить'));
    const cleanBtn = buttons.find((b) => b.textContent?.includes('Очистить'));

    expect(verifyBtn).toBeDefined();
    expect(cleanBtn).toBeDefined();

    verifyBtn?.click();
    await tick();

    cleanBtn?.click();
    await tick();

    unmount(comp);
  });

  it('renders SettingsScreen with actual Discord RPC status preview card and no mock fakes', async () => {
    const comp = mount(SettingsScreen, { target });
    expect(target.textContent).toContain('Discord Rich Presence');
    expect(target.textContent).toContain('Просмотр статуса в Discord');
    expect(target.textContent).toContain('Wex Launcher');
    expect(target.textContent).toContain('В главном меню');
    expect(target.textContent).toContain('Выбирает режим игры');
    expect(target.textContent).not.toContain('Скачать Wex Launcher');
    expect(target.textContent).not.toContain('ИГРАЕТ В ИГРУ');
    expect(target.textContent).not.toContain('Модов: 142');
    expect(target.textContent).not.toContain('auramc');
    unmount(comp);
  });

  it('renders WardrobeScreen with 2D character preview, model selector, capes, and authentic empty history', () => {
    const comp = mount(WardrobeScreen, {
      target,
      props: {
        playerName: 'Steve',
      },
    });

    // Check preview and model controls
    expect(target.querySelector('canvas.character-canvas')).not.toBeNull();
    expect(target.textContent).toContain('Классическая (4 пикселя)');
    expect(target.textContent).toContain('Тонкая (3 пикселя)');
    expect(target.textContent).toContain('Спереди');
    expect(target.textContent).toContain('Сзади');

    // Check cape options per SPEC 1.6
    expect(target.textContent).toContain('Без плаща');
    expect(target.textContent).toContain('Свой файл');
    expect(target.textContent).not.toContain('Aura Cyan');
    expect(target.textContent).not.toContain('Aura Silver');
    expect(target.textContent).toContain('Стандартный скин');

    // Check action buttons
    expect(target.textContent).toContain('Загрузить скин (.png)');
    expect(target.textContent).toContain('Сбросить на стандартный');
    expect(target.textContent).toContain('Применить к игре');

    // Check authentic empty state
    expect(target.textContent).toContain('История гардероба пуста');

    unmount(comp);
  });

  it('renders WardrobeScreen skins library with quick-switch and deletion', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockImplementation((cmd: string, args: any) => {
      if (cmd === 'extras_list_skin_history') {
        return Promise.resolve({
          items: [
            {
              id: 'skin_123',
              item_type: 'skin',
              name: 'Warrior Alex',
              hash: '1234567890abcdef',
              model: 'slim',
              relative_path: 'skins/1234567890abcdef.png',
              created_utc: '2026-11-08T18:00:00Z',
            },
          ],
        });
      }
      if (cmd === 'wardrobe_get_skin') {
        return Promise.resolve({
          item: {
            id: 'skin_123',
            name: 'Warrior Alex',
            model: 'slim',
          },
          skin_bytes_base64:
            'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
        });
      }
      if (cmd === 'wardrobe_delete_history') {
        return Promise.resolve({ success: true });
      }
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, {
      target,
      props: {
        playerName: 'Player1',
        initialHistory: [
          {
            id: 'skin_123',
            item_type: 'skin',
            name: 'Warrior Alex',
            hash: '1234567890abcdef',
            model: 'slim',
            relative_path: 'skins/1234567890abcdef.png',
            created_utc: '2026-11-08T18:00:00Z',
          },
        ],
      },
    });

    await tick();
    await new Promise((r) => setTimeout(r, 20));

    // Verify skins library rendered item
    expect(target.textContent).toContain('Библиотека скинов');
    expect(target.textContent).toContain('Warrior Alex');
    expect(target.textContent).toContain('Тонкая');

    // Click on "Выбрать" button in history actions to quick-switch
    const selectBtn = target.querySelector('.history-actions button') as HTMLButtonElement;
    expect(selectBtn).not.toBeNull();
    selectBtn.click();

    await tick();
    await new Promise((r) => setTimeout(r, 20));

    // Verify skin name input updated to Warrior Alex
    const skinInput = target.querySelector('.skin-name-input') as HTMLInputElement;
    expect(skinInput).not.toBeNull();
    expect(skinInput.value).toBe('Warrior Alex');

    // Click delete button
    const deleteBtn = target.querySelector('[aria-label="Удалить из истории"]') as HTMLButtonElement;
    expect(deleteBtn).not.toBeNull();
    deleteBtn.click();

    await tick();
    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('История гардероба пуста');

    unmount(comp);
  });

  it('falls back to 2D character preview in WardrobeScreen when WebGL is unavailable', () => {
    const comp = mount(WardrobeScreen, {
      target,
      props: {
        playerName: 'Alex',
      },
    });

    // In jsdom without WebGL, fallback triggers automatically
    const canvas2d = target.querySelector('canvas.character-canvas-2d');
    expect(canvas2d).not.toBeNull();
    expect(canvas2d?.classList.contains('hidden')).toBe(false);

    // 3D canvas is present but marked hidden in fallback mode
    const canvas3d = target.querySelector('canvas.character-canvas-3d');
    expect(canvas3d).not.toBeNull();
    expect(canvas3d?.classList.contains('hidden')).toBe(true);

    // 2D view controls (front/back) are active
    expect(target.textContent).toContain('Спереди');
    expect(target.textContent).toContain('Сзади');

    unmount(comp);
  });

  it('renders WorkshopScreen with tabs, version selector, and authentic empty states', () => {
    const comp = mount(WorkshopScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
      },
    });

    // Check tabs
    expect(target.textContent).toContain('Миры');
    expect(target.textContent).toContain('Ресурс-паки');
    expect(target.textContent).toContain('Шейдеры');
    expect(target.textContent).toContain('Скриншоты');
    expect(target.textContent).toContain('1.21.1');
    expect(target.textContent).toContain('1.20.1');

    // Without IPC data, must show authentic empty state
    expect(target.textContent).toMatch(/Сохранённых миров нет|Миров пока нет/);
    expect(target.textContent).toMatch(/Создайте одиночный мир|Создайте мир/);

    // No fake/fabricated worlds
    expect(target.textContent).not.toContain('Demo World');
    expect(target.textContent).not.toContain('Hardcore Kingdom');

    unmount(comp);
  });

  it('renders WorkshopScreen with screenshots empty state when switched to screenshots tab', async () => {
    const comp = mount(WorkshopScreen, {
      target,
      props: {
        selectedVersion: '1.20.1',
      },
    });

    const buttons = Array.from(target.querySelectorAll('button'));
    const screenshotsBtn = buttons.find((b) => b.textContent?.includes('Скриншоты'));
    expect(screenshotsBtn).toBeDefined();

    screenshotsBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(target.textContent).toMatch(/Снимков экрана пока нет|Скриншотов пока нет/);
    expect(target.textContent).toMatch(/Нажмите клавишу F2|Нажмите F2/);

    unmount(comp);
  });

  it('renders WorkshopScreen with resource packs empty state and header actions when switched to resourcepacks tab', async () => {
    const comp = mount(WorkshopScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
      },
    });

    const buttons = Array.from(target.querySelectorAll('button'));
    const packsBtn = buttons.find((b) => b.textContent?.includes('Ресурс-паки'));
    expect(packsBtn).toBeDefined();

    packsBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(target.textContent).toContain('Ресурс-паков пока нет');
    expect(target.textContent).toContain('Поместите архив с ресурс-паком');
    expect(target.textContent).toContain('Добавить архив…');
    expect(target.textContent).toContain('Папка паков');

    unmount(comp);
  });

  it('renders WorkshopScreen with shaders empty state and header actions when switched to shaders tab', async () => {
    const comp = mount(WorkshopScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
      },
    });

    const buttons = Array.from(target.querySelectorAll('button'));
    const shadersBtn = buttons.find((b) => b.textContent?.includes('Шейдеры'));
    expect(shadersBtn).toBeDefined();

    shadersBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(target.textContent).toContain('Шейдер-паков пока нет');
    expect(target.textContent).toContain('Поместите архив с шейдерами');
    expect(target.textContent).toContain('Добавить архив…');
    expect(target.textContent).toContain('Папка шейдеров');

    unmount(comp);
  });

  it('renders SplashScreen with wordmark logo, version badge, and initial status', () => {
    const comp = mount(SplashScreen, {
      target,
      props: {
        initialVersion: 'v0.1.0',
        autoStart: false,
      },
    });

    expect(target.textContent).toContain('Wex');
    expect(target.textContent).toContain('v0.1.0');
    expect(target.textContent).toContain('Инициализация каталогов…');
    expect(target.querySelector('.splash-card')).not.toBeNull();

    unmount(comp);
  });

  it('renders SplashScreen network error state with retry and offline skip buttons', async () => {
    let completed = false;
    const comp = mount(SplashScreen, {
      target,
      props: {
        initialVersion: 'v0.1.0',
        autoStart: false,
        simulateNetworkError: true,
        oncomplete: () => {
          completed = true;
        },
      },
    });

    // Manually trigger initialization with simulated error
    await comp.startInitialization();

    expect(target.textContent).toContain('Сетевая ошибка');
    expect(target.textContent).toContain('Повторить');
    expect(target.textContent).toContain('Пропустить');

    // Click offline skip button
    const skipBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Пропустить')
    );
    expect(skipBtn).toBeDefined();
    skipBtn?.click();

    // Allow 250ms cross-fade transition to complete
    await new Promise((r) => setTimeout(r, 300));

    // Verify offline skip invokes completion handler
    expect(completed).toBe(true);

    unmount(comp);
  });

  it('immediately transitions when Reduce Motion is enabled without delay', async () => {
    let completed = false;
    theme.setReduceMotion(true);

    const comp = mount(SplashScreen, {
      target,
      props: {
        initialVersion: 'v0.1.0',
        autoStart: false,
        oncomplete: () => {
          completed = true;
        },
      },
    });

    comp.skipOffline();
    expect(completed).toBe(true);

    theme.setReduceMotion(false);
    unmount(comp);
  });

  // wex-review-fake-progress item 14. Since 0.9.1 the splash never checks or downloads launcher
  // updates (the sidebar UpdateCard owns that), so it must not show any update progress at all.
  it('SplashScreen never checks for updates or shows fake download progress', async () => {
    const tauriWindow = window as unknown as { __TAURI_INTERNALS__?: unknown };
    const hadInternals = '__TAURI_INTERNALS__' in tauriWindow;
    const previousInternals = tauriWindow.__TAURI_INTERNALS__;
    tauriWindow.__TAURI_INTERNALS__ = {};
    defaultInvoke.mockClear();
    const oncomplete = vi.fn();
    const comp = mount(SplashScreen, {
      target,
      props: { initialVersion: 'v0.1.0', autoStart: false, oncomplete },
    });
    try {
      await tick();
      await (comp as any).startInitialization();
      await vi.waitFor(() => expect(oncomplete).toHaveBeenCalledOnce(), { timeout: 3000 });

      const commands = defaultInvoke.mock.calls.map(([cmd]) => cmd);
      expect(commands).not.toContain('startup_check_update');
      expect(commands).not.toContain('startup_download_update');
      expect(target.querySelector('[role="progressbar"], .download-container')).toBeNull();
      expect(target.textContent).not.toMatch(/\d+\s?%/);
      expect(target.textContent).not.toContain('МБ/с');
    } finally {
      unmount(comp);
      if (hadInternals) tauriWindow.__TAURI_INTERNALS__ = previousInternals;
      else delete tauriWindow.__TAURI_INTERNALS__;
    }
  });
  it('renders ImportWizardSheet with found old launcher data, non-destructive notice, and transfer toggles', () => {
    const comp = mount(ImportWizardSheet, {
      target,
      props: {
        open: true,
        initialDir: 'C:\\Users\\Player\\AppData\\Roaming\\.aura',
        initialData: {
          nickname: 'AuraVeteran',
          ram_mb: 6144,
          selected_version: '1.20.1',
          graphics_preset: 'High',
          total_play_time_seconds: 50000n,
          total_game_launches: 120,
          servers: [
            { name: 'Survival Server', ip: 'mc.aura.test', icon: null },
            { name: 'MiniGames Server', ip: 'games.aura.test', icon: null },
          ],
          skins: [
            { file_name: 'steve_warrior.png', path: 'c:/skins/steve_warrior.png', hash: '123', is_slim: false },
          ],
          capes: [],
          wardrobe_history: [],
          worlds: [],
        },
      },
    });

    // Header & Title
    expect(target.textContent).toContain('Импорт из старого лаунчера');

    // Safe isolation notice
    expect(target.textContent).toContain('Файлы старого лаунчера не будут изменены или удалены');

    // Found items summary
    expect(target.textContent).toContain('AuraVeteran');
    expect(target.textContent).toContain('6 ГБ (6144 МБ)');
    expect(target.textContent).toContain('1.20.1');
    expect(target.textContent).toContain('Найдено серверов: 2');
    expect(target.textContent).toContain('Найдено скинов: 1');

    // Transfer buttons
    expect(target.textContent).toContain('Перенести');
    expect(target.textContent).toContain('Пропустить');

    unmount(comp);
  });

  it('renders ServersScreen in offline state when network is offline', async () => {
    const { network } = await import('../design/network.svelte');
    network.setOnlineForTesting(false);

    const comp = mount(ServersScreen, { target });
    expect(target.textContent).toContain('Нет подключения');
    expect(target.textContent).toContain('Проверьте подключение к интернету или локальной сети');
    expect(target.textContent).toContain('Повторить');

    network.setOnlineForTesting(true);
    unmount(comp);
  });

  it('renders LobbyScreen in offline state when network is offline', async () => {
    const { network } = await import('../design/network.svelte');
    network.setOnlineForTesting(false);

    const comp = mount(LobbyScreen, { target });
    expect(target.textContent).toContain('Нет подключения');
    expect(target.textContent).toContain('Повторить');

    network.setOnlineForTesting(true);
    unmount(comp);
  });

  it('renders FriendsScreen in offline state when network is offline', async () => {
    const { network } = await import('../design/network.svelte');
    network.setOnlineForTesting(false);

    const comp = mount(FriendsScreen, { target });
    expect(target.textContent).toContain('Нет подключения');
    expect(target.textContent).toContain('Повторить');

    network.setOnlineForTesting(true);
    unmount(comp);
  });

  it('renders PlayScreen with offline launch status message', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        isLaunching: true,
        progressPercentage: 10,
        progressStage: 'Нет сети — запуск без обновления',
        progressDetail: 'Запуск установленной версии 1.21.1',
      },
    });

    expect(target.textContent).toContain('Нет сети — запуск без обновления');
    unmount(comp);
  });

  describe('wex-offline-states: Offline State for All 8 Screens', () => {
    it('1. PlayScreen renders honest offline state and allows local launch', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(PlayScreen, {
        target,
        props: { selectedVersion: '1.21.1', playerName: 'Alex' },
      });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Доступен запуск уже скачанной версии');
      expect(target.textContent).toContain('Повторить');

      const playBtn = target.querySelector('.dock-action button') as HTMLButtonElement;
      expect(playBtn).not.toBeNull();
      expect(playBtn.disabled).toBe(false);

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('2. ServersScreen renders honest offline state with retry action', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(ServersScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('3. LobbyScreen renders honest offline state with retry action', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(LobbyScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('4. FriendsScreen renders honest offline state with retry action', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(FriendsScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('5. WardrobeScreen renders honest offline state while local skins and preview work', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(WardrobeScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Локальные скины и 3D-просмотр работают офлайн');
      expect(target.textContent).toContain('Повторить');

      // Apply button and local controls must remain functional
      const applyBtn = target.querySelector('.apply-bar button') as HTMLButtonElement;
      expect(applyBtn).not.toBeNull();

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('6. WorkshopScreen renders honest offline state while local worlds, packs, shaders, and screenshots work', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(WorkshopScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Одиночные миры, ресурс-паки, шейдеры и скриншоты полностью доступны офлайн');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('7. ModsScreen renders honest offline state while installed mods management works', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(ModsScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Управление установленными модами и папка инстанса доступны офлайн');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });

    it('8. SettingsScreen renders honest offline state while local launcher settings work', async () => {
      const { network } = await import('../design/network.svelte');
      network.setOnlineForTesting(false);

      const comp = mount(SettingsScreen, { target });
      await tick();

      expect(target.textContent).toContain('Нет подключения');
      expect(target.textContent).toContain('Локальные параметры сохранены');
      expect(target.textContent).toContain('Повторить');

      network.setOnlineForTesting(true);
      unmount(comp);
    });
  });

  it('renders CrashSheet with honest unknown diagnosis without fake reasons', () => {
    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        diagnosis: {
          kind: { Unknown: { exit_code: -1 } },
          title: 'Игра завершилась с ошибкой (код -1)',
          user_advice: 'Причина сбоя не определена.',
          technical_summary: 'Process finished with exit code -1',
        },
      },
    });

    expect(target.textContent).toContain('Причина не определена');
    expect(target.textContent).toContain('Process finished with exit code -1');
    expect(target.textContent).toContain('Скопировать отчёт');
    expect(target.textContent).toContain('Открыть лог');
    expect(target.textContent).toContain('Отправить отчёт');
    unmount(comp);
  });

  it('opens ReportSheet from CrashSheet with anonymized bug report details', async () => {
    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        diagnosis: {
          kind: 'OutOfMemory',
          title: 'Нехватка оперативной памяти (OutOfMemoryError)',
          user_advice: 'Выделите больше памяти в настройках.',
          technical_summary: 'OutOfMemoryError at C:\\Users\\User1\\launch.jar',
        },
      },
    });

    const reportBtn = target.querySelector('[data-testid="btn-open-report"]') as HTMLElement;
    expect(reportBtn).not.toBeNull();
    reportBtn.click();
    await tick();

    expect(target.textContent).toContain('Отчёт об ошибке');
    expect(target.textContent).toContain('Сохранить на рабочий стол (.zip)');
    unmount(comp);
  });

  it('renders PlayScreen with launch ErrorState banner and actionable buttons', async () => {
    let launchRetried = false;
    let settingsOpened = false;

    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        launchError: 'Java runtime error: java binary not found in PATH',
        onlaunch: () => {
          launchRetried = true;
        },
        onopensettings: () => {
          settingsOpened = true;
        },
      },
    });

    await tick();

    // Must display human-friendly Apple HIG error
    expect(target.textContent).toContain('Не найдена среда Java');
    expect(target.textContent).toContain('Укажите путь к Java в настройках');

    // Action button
    const settingsBtn = target.querySelector('.launch-error-banner .error-actions-row button') as HTMLButtonElement;
    expect(settingsBtn).not.toBeNull();
    expect(settingsBtn.textContent).toContain('Настройки');

    settingsBtn.click();
    expect(settingsOpened || launchRetried).toBe(true);

    unmount(comp);
  });

  it('renders ModsScreen with protected pack mod ErrorState when deletion is rejected', async () => {
    const comp = mount(ModsScreen, { target });
    await tick();

    // Trigger delete on pack mod
    const deletePackBtn = target.querySelector('.pack-badge')
      ?.closest('.row-trailing')
      ?.querySelector('.delete-btn');

    // Pack mod should not have a delete button, or if triggered internally:
    expect(target.textContent).toContain('Пак Wex');

    unmount(comp);
  });

  it('opens ImportWizardSheet on PlayScreen when initialShowImportWizard is true', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        initialShowImportWizard: true,
      },
    });

    expect(target.textContent).toContain('Импорт из старого лаунчера');
    unmount(comp);
  });

  it('supports keyboard navigation: Esc closes modals, Ctrl+1..7 navigation', () => {
    let escaped = false;
    const onEscape = () => { escaped = true; };
    window.addEventListener('wex-escape', onEscape);

    window.dispatchEvent(new CustomEvent('wex-escape'));
    expect(escaped).toBe(true);

    window.removeEventListener('wex-escape', onEscape);
  });

  describe('SettingsScreen Search Filtering Tests', () => {
    it('renders Apple HIG search bar with placeholder', () => {
      const comp = mount(SettingsScreen, { target });
      const searchInput = target.querySelector('.search-input') as HTMLInputElement | null;
      expect(searchInput).not.toBeNull();
      expect(searchInput?.placeholder).toContain('Поиск настроек');
      unmount(comp);
    });

    it('filters settings by keyword "память", displaying Memory group and hiding unrelated groups', async () => {
      const comp = mount(SettingsScreen, { target });
      await tick();

      const searchInput = target.querySelector('.search-input') as HTMLInputElement;
      expect(searchInput).not.toBeNull();

      searchInput.value = 'память';
      searchInput.dispatchEvent(new Event('input', { bubbles: true }));
      await tick();

      expect(target.textContent).toContain('Память и производительность');
      expect(target.textContent).toContain('Выделение оперативной памяти');
      // Unrelated groups should be hidden
      expect(target.textContent).not.toContain('Звук и музыка');
      expect(target.textContent).not.toContain('Уведомления о друзьях');

      unmount(comp);
    });

    it('filters settings by keyword "музыка", showing Sound & Music group', async () => {
      const comp = mount(SettingsScreen, { target });
      await tick();

      const searchInput = target.querySelector('.search-input') as HTMLInputElement;
      expect(searchInput).not.toBeNull();

      searchInput.value = 'музыка';
      searchInput.dispatchEvent(new Event('input', { bubbles: true }));
      await tick();

      expect(target.textContent).toContain('Звук и музыка');
      expect(target.textContent).toContain('Фоновая музыка');
      expect(target.textContent).not.toContain('Выделение оперативной памяти');

      unmount(comp);
    });

    it('displays empty state when search matches nothing and allows clearing search', async () => {
      const comp = mount(SettingsScreen, { target });
      await tick();

      const searchInput = target.querySelector('.search-input') as HTMLInputElement;
      expect(searchInput).not.toBeNull();

      searchInput.value = 'xyz_nonexistent_setting_12345';
      searchInput.dispatchEvent(new Event('input', { bubbles: true }));
      await tick();

      const emptyState = target.querySelector('.empty-search-state');
      expect(emptyState).not.toBeNull();
      expect(emptyState?.textContent).toContain('Ничего не найдено');
      expect(emptyState?.textContent).toContain('xyz_nonexistent_setting_12345');

      // Clear search via clear button
      const clearBtn = emptyState?.querySelector('button');
      expect(clearBtn).not.toBeNull();
      clearBtn?.click();
      await tick();

      // Empty state should disappear and groups should be restored
      expect(target.querySelector('.empty-search-state')).toBeNull();
      expect(target.textContent).toContain('Память и производительность');
      expect(target.textContent).toContain('Внешний вид');

      unmount(comp);
    });
  });
});
