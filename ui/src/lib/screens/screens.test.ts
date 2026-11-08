import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import PlayScreen from './PlayScreen.svelte';
import Mockup1 from './mockups/Mockup1.svelte';
import Mockup2 from './mockups/Mockup2.svelte';
import Mockup3 from './mockups/Mockup3.svelte';
import ServersScreen from './ServersScreen.svelte';
import LobbyScreen from './LobbyScreen.svelte';
import FriendsScreen from './FriendsScreen.svelte';
import ModsScreen from './ModsScreen.svelte';
import SettingsScreen from './SettingsScreen.svelte';
import WardrobeScreen from './WardrobeScreen.svelte';
import WorkshopScreen from './WorkshopScreen.svelte';
import ImportWizardSheet from '../components/ImportWizardSheet.svelte';

vi.mock('@tauri-apps/api/core', () => {
  return {
    invoke: vi.fn(async (cmd: string, args?: Record<string, any>) => {
      if (cmd === 'list_open_rooms') {
        throw new Error('Сервис недоступен');
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
        };
      }
      return [];
    }),
  };
});

describe('Wex Launcher Screens', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
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

  it('renders Mockup 2 (Editorial) with showcase banner, changelog, and servers', () => {
    const comp = mount(Mockup2, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
      },
    });

    expect(target.textContent).toContain('Wex 1.21.1');
    expect(target.textContent).toContain('Новое в сборке');
    expect(target.textContent).toContain('Журнал изменений');
    expect(target.textContent).toContain('Быстрый вход');
    expect(target.querySelector('.strip-right button')?.textContent).toContain('Играть');

    unmount(comp);
  });

  it('renders Mockup 3 (Minimalist) with player capsule and quiet feed', () => {
    const comp = mount(Mockup3, {
      target,
      props: {
        selectedVersion: '1.20.1',
        playerName: 'Steve',
      },
    });

    expect(target.textContent).toContain('Minecraft');
    expect(target.textContent).toContain('Клиент Wex • Fabric');
    expect(target.textContent).toContain('Последние изменения');
    expect(target.textContent).toContain('Java 17 • 64-bit');
    expect(target.querySelector('.launch-btn-wrap button')?.textContent).toContain('Играть');

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
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        initialNews: [],
        initialUpdateInfo: {
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

  it('renders ServersScreen with empty state when without IPC data and has add server action', () => {
    const comp = mount(ServersScreen, { target });
    expect(target.textContent).toContain('Список серверов пуст');
    expect(target.textContent).toContain('Добавить сервер');
    expect(target.textContent).not.toContain('mc.wexmc.net');
    unmount(comp);
  });

  it('renders LobbyScreen with LAN/Cloud mode switch, guest code input, and world picker', () => {
    const comp = mount(LobbyScreen, { target });
    expect(target.textContent).toContain('LAN и Туннели');
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
    expect(target.textContent).toContain('Сервис недоступен');
    expect(target.textContent).toContain('Повторить попытку');
    expect(target.textContent).not.toContain('mc.wexmc.net');
    expect(target.textContent).not.toContain('Aura Official');

    unmount(comp);
  });

  it('renders LobbyScreen cloud mode with empty rooms state when API responds with empty list', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockResolvedValueOnce([]);

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
    expect(target.textContent).toContain('В папке инстанса пока нет модов');
    unmount(comp);
  });

  it('renders SettingsScreen with RAM slider, Java options, Discord RPC, graphics presets, and About Wex', () => {
    const comp = mount(SettingsScreen, { target });
    expect(target.textContent).toContain('Тема оформления');
    expect(target.textContent).toContain('Цветовой акцент');
    expect(target.textContent).toContain('Уменьшение движения');
    expect(target.textContent).toContain('Уменьшение прозрачности');
    expect(target.textContent).toContain('Выделение оперативной памяти');
    expect(target.textContent).toContain('Графический пресет');
    expect(target.textContent).toContain('Выбор исполняемого файла Java');
    expect(target.textContent).toContain('Discord Rich Presence');
    expect(target.textContent).toContain('Язык интерфейса');
    expect(target.textContent).toContain('О программе');
    expect(target.textContent).toContain('Tauri 2');
    expect(target.textContent).not.toContain('.minecraft');
    expect(target.textContent).toContain('Wex');
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
    expect(target.textContent).toContain('Классическая (4 px)');
    expect(target.textContent).toContain('Тонкая (3 px)');
    expect(target.textContent).toContain('Спереди');
    expect(target.textContent).toContain('Сзади');

    // Check cape options per SPEC 1.6
    expect(target.textContent).toContain('Без плаща');
    expect(target.textContent).toContain('Aura Cyan');
    expect(target.textContent).toContain('Aura Silver');
    expect(target.textContent).toContain('Пользовательский');

    // Check action buttons
    expect(target.textContent).toContain('Загрузить скин (PNG)');
    expect(target.textContent).toContain('Сбросить на стандартный');
    expect(target.textContent).toContain('Применить к игре');

    // Check authentic empty state
    expect(target.textContent).toContain('История гардероба пуста');

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

    // Without IPC data, must show authentic empty state: «Сохранённых миров нет»
    expect(target.textContent).toContain('Сохранённых миров нет');
    expect(target.textContent).toContain('Создайте одиночный мир в игре Minecraft 1.21.1');

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

    expect(target.textContent).toContain('Снимков экрана пока нет');
    expect(target.textContent).toContain('Нажмите клавишу F2');

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
    expect(target.textContent).toContain('Поместите ZIP-архив с ресурс-паком');
    expect(target.textContent).toContain('Добавить ZIP…');
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
    expect(target.textContent).toContain('Поместите ZIP-архив с шейдерами');
    expect(target.textContent).toContain('Добавить ZIP…');
    expect(target.textContent).toContain('Папка шейдеров');

    unmount(comp);
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
    expect(target.textContent).toContain('2 сервер(-ов) найдено');
    expect(target.textContent).toContain('1 скин(-ов) найдено');

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
});
