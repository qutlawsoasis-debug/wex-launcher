import { describe, it, expect, beforeEach, afterEach } from 'vitest';
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
import WorkshopScreen from './WorkshopScreen.svelte';

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

  it('renders PlayScreen with authentic empty states for news and servers when without IPC data', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
      },
    });

    // Without IPC, changelog must show authentic empty state: «Новостей пока нет»
    expect(target.textContent).toContain('Новостей пока нет');
    // Without IPC, recent server must show authentic empty state: «Вы ещё не играли на серверах»
    expect(target.textContent).toContain('Вы ещё не играли на серверах');
    // Zero hardcoded fabricated numbers or mock domains
    expect(target.textContent).not.toContain('mc.wexmc.net');
    expect(target.textContent).not.toContain('Сборка 2.4');

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
    expect(target.textContent).toContain('СПИСОК ДОСТИЖЕНИЙ');

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

  it('renders ModsScreen with mod list and status toggles', () => {
    const comp = mount(ModsScreen, { target });
    expect(target.textContent).toContain('Sodium');
    expect(target.textContent).toContain('Iris Shaders');
    expect(target.textContent).toContain('Wex Client');
    unmount(comp);
  });

  it('renders SettingsScreen with RAM slider, graphics presets, appearance, and About Wex', () => {
    const comp = mount(SettingsScreen, { target });
    expect(target.textContent).toContain('Тема оформления');
    expect(target.textContent).toContain('Цветовой акцент');
    expect(target.textContent).toContain('Уменьшение движения');
    expect(target.textContent).toContain('Уменьшение прозрачности');
    expect(target.textContent).toContain('Выделение оперативной памяти');
    expect(target.textContent).toContain('Графический пресет');
    expect(target.textContent).toContain('О программе');
    expect(target.textContent).toContain('Tauri 2');
    expect(target.textContent).not.toContain('.minecraft');
    expect(target.textContent).toContain('Wex');
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
});
