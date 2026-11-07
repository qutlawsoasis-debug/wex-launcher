import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import PlayScreen from './PlayScreen.svelte';
import ServersScreen from './ServersScreen.svelte';
import FriendsScreen from './FriendsScreen.svelte';
import ModsScreen from './ModsScreen.svelte';
import SettingsScreen from './SettingsScreen.svelte';

describe('Wex Launcher Screens', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders PlayScreen with version selector and Play button', () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        isLaunching: false,
      },
    });

    expect(target.querySelector('.hero-title')?.textContent).toBe('Aura Client');
    const playBtn = target.querySelector('.action-buttons button');
    expect(playBtn).not.toBeNull();
    expect(playBtn?.textContent).toContain('Играть');

    const nicknameInput = target.querySelector('#nickname-input') as HTMLInputElement;
    expect(nicknameInput?.value).toBe('Steve');

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
        progressStage: 'Синхронизация Aura-Pack',
        progressDetail: 'Загрузка шейдеров...',
      },
    });

    const progress = target.querySelector('.progress-section');
    expect(progress).not.toBeNull();
    expect(target.querySelector('.progress-label')?.textContent).toBe('Синхронизация Aura-Pack');

    const cancelBtn = target.querySelector('.action-buttons button');
    expect(cancelBtn?.textContent).toContain('Отмена');

    unmount(comp);
  });

  it('renders ServersScreen with official and custom servers', () => {
    const comp = mount(ServersScreen, { target });
    const rows = target.querySelectorAll('.apple-row');
    expect(rows.length).toBeGreaterThan(0);
    expect(target.textContent).toContain('Aura Official');
    unmount(comp);
  });

  it('renders FriendsScreen with player statuses', () => {
    const comp = mount(FriendsScreen, { target });
    expect(target.textContent).toContain('Alex_Pro');
    expect(target.textContent).toContain('Друзья онлайн');
    unmount(comp);
  });

  it('renders ModsScreen with mod list and status toggles', () => {
    const comp = mount(ModsScreen, { target });
    expect(target.textContent).toContain('Sodium');
    expect(target.textContent).toContain('Iris Shaders');
    expect(target.textContent).toContain('Aura Client');
    unmount(comp);
  });

  it('renders SettingsScreen with appearance options and RAM slider', () => {
    const comp = mount(SettingsScreen, { target });
    expect(target.textContent).toContain('Тема оформления');
    expect(target.textContent).toContain('Цветовой акцент');
    expect(target.textContent).toContain('Reduce Motion');
    expect(target.textContent).toContain('Reduce Transparency');
    unmount(comp);
  });
});
