import { describe, it, expect, beforeEach, vi } from 'vitest';
import { ru } from './locales/ru';
import { en } from './locales/en';
import { i18n, t } from './i18n.svelte';

describe('i18n Localization Integrity & Parity', () => {
  it('has identical keys in Russian and English dictionaries', () => {
    const ruKeys = Object.keys(ru).sort();
    const enKeys = Object.keys(en).sort();

    expect(ruKeys).toEqual(enKeys);
    expect(ruKeys.length).toBeGreaterThan(50);
  });

  it('contains no empty or whitespace-only translation strings', () => {
    for (const [key, val] of Object.entries(ru)) {
      expect(val.trim().length, `ru[${key}] must not be empty`).toBeGreaterThan(0);
    }
    for (const [key, val] of Object.entries(en)) {
      expect(val.trim().length, `en[${key}] must not be empty`).toBeGreaterThan(0);
    }
  });

  it('has identical interpolation placeholder tokens in both locales', () => {
    const extractPlaceholders = (str: string) => {
      const matches = str.match(/\{([a-zA-Z0-9_]+)\}/g) || [];
      return matches.sort();
    };

    for (const key of Object.keys(ru) as (keyof typeof ru)[]) {
      const ruPlaceholders = extractPlaceholders(ru[key]);
      const enPlaceholders = extractPlaceholders(en[key]);
      expect(
        enPlaceholders,
        `Placeholder mismatch for key: "${key}" (RU: ${ruPlaceholders.join(',')} vs EN: ${enPlaceholders.join(',')})`
      ).toEqual(ruPlaceholders);
    }
  });
});

describe('i18n Reactive Manager & Translation Helper', () => {
  beforeEach(() => {
    i18n.setLocale('ru');
  });

  it('returns Russian translation by default', () => {
    expect(t('sidebar.play')).toBe('Играть');
    expect(t('sidebar.settings')).toBe('Настройки');
    expect(t('offline.title')).toBe('Нет подключения');
  });

  it('switches to English immediately without restart', () => {
    i18n.setLocale('en');

    expect(t('sidebar.play')).toBe('Play');
    expect(t('sidebar.settings')).toBe('Settings');
    expect(t('offline.title')).toBe('No Connection');

    // Switch back to Russian
    i18n.setLocale('ru');
    expect(t('sidebar.play')).toBe('Играть');
  });

  it('interpolates single and multiple parameters accurately', () => {
    i18n.setLocale('ru');
    expect(t('servers.ping_ms', { ms: 42 })).toBe('42 мс');
    expect(t('play.update_description', { version: '1.2.0' })).toBe(
      'Доступна новая версия 1.2.0. Нажмите для обновления.'
    );

    i18n.setLocale('en');
    expect(t('servers.ping_ms', { ms: 42 })).toBe('42 ms');
    expect(t('play.update_description', { version: '1.2.0' })).toBe(
      'Version 1.2.0 is available.'
    );
  });

  it('falls back to key string when translation key is not defined', () => {
    expect(t('unknown.non_existent.key')).toBe('unknown.non_existent.key');
  });

  it('updates documentElement lang attribute on locale change', () => {
    if (typeof document !== 'undefined') {
      i18n.setLocale('en');
      expect(document.documentElement.lang).toBe('en');
      i18n.setLocale('ru');
      expect(document.documentElement.lang).toBe('ru');
    }
  });

  it('reactively updates rendered Svelte component when locale changes without restart', async () => {
    const { mount, unmount, tick } = await import('svelte');
    const { default: OfflineState } = await import('../components/OfflineState.svelte');

    const target = document.createElement('div');
    document.body.appendChild(target);

    i18n.setLocale('ru');
    const comp = mount(OfflineState, { target });
    await tick();

    expect(target.textContent).toContain('Нет подключения');
    expect(target.textContent).toContain('Повторить');

    // Switch to English dynamically
    i18n.setLocale('en');
    await tick();

    expect(target.textContent).toContain('No Connection');
    expect(target.textContent).toContain('Try Again');

    // Switch back to Russian dynamically
    i18n.setLocale('ru');
    await tick();

    expect(target.textContent).toContain('Нет подключения');
    expect(target.textContent).toContain('Повторить');

    unmount(comp);
    target.remove();
  });

  it('renders screens in authentic English when locale is set to en', async () => {
    const { mount, unmount, tick } = await import('svelte');
    const { default: SettingsPage } = await import('../pages/settings/SettingsPage.svelte');
    const { default: LaunchBar } = await import('../shell/LaunchBar.svelte');
    const { default: ServersScreen } = await import('../screens/ServersScreen.svelte');
    const { default: BuildPage } = await import('../pages/build/BuildPage.svelte');
    const { default: WardrobeScreen } = await import('../screens/WardrobeScreen.svelte');
    const { default: LobbyScreen } = await import('../screens/LobbyScreen.svelte');
    const { default: FriendsScreen } = await import('../screens/FriendsScreen.svelte');

    const target = document.createElement('div');
    document.body.appendChild(target);

    i18n.setLocale('en');

    // 1. Settings page
    const compSettings = mount(SettingsPage, { target });
    await tick();
    for (const label of ['Game', 'Graphics', 'Appearance', 'Launcher', 'Data', 'Keys', 'Memory for the game', 'Reset settings']) {
      expect(target.textContent).toContain(label);
    }
    unmount(compSettings);
    target.innerHTML = '';

    // 2. Launch bar
    const compPlay = mount(LaunchBar, { target });
    await tick();
    expect(target.textContent).toContain('Play');
    expect(target.querySelector('.play')?.getAttribute('aria-label')).toContain('Play Wex');
    unmount(compPlay);
    target.innerHTML = '';

    // 3. ServersScreen
    const compServers = mount(ServersScreen, { target });
    await tick();
    expect(target.textContent).toContain('Add Server');
    expect(target.textContent).toContain('No Servers Found');
    unmount(compServers);
    target.innerHTML = '';

    // 4. Build page: tabs for mods, worlds and the workshop
    const compBuild = mount(BuildPage, { target, props: { version: '1.21.1' } });
    await tick();
    for (const label of ['Mods', 'Worlds', 'Resource packs', 'Shaders', 'Screenshots', 'Backups', 'Build settings']) {
      expect(target.textContent).toContain(label);
    }
    expect(target.textContent).toContain('Play');
    unmount(compBuild);
    target.innerHTML = '';

    // 6. WardrobeScreen
    const compWardrobe = mount(WardrobeScreen, { target });
    await tick();
    expect(target.textContent).toContain('Character Model');
    expect(target.textContent).toContain('Classic (4 px)');
    expect(target.textContent).toContain('Slim (3 px)');
    expect(target.textContent).toContain('No Cape');
    expect(target.textContent).toContain('Apply to Game');
    unmount(compWardrobe);
    target.innerHTML = '';

    // 7. LobbyScreen
    const compLobby = mount(LobbyScreen, { target });
    await tick();
    expect(target.textContent).toContain('LAN & Tunnels');
    expect(target.textContent).toContain('Cloud Worlds');
    expect(target.textContent).toContain('Join by Code');
    expect(target.textContent).toContain('Open LAN World');
    unmount(compLobby);
    target.innerHTML = '';

    // 8. FriendsScreen
    const compFriends = mount(FriendsScreen, { target });
    await tick();
    expect(target.textContent).toContain('Friends');
    expect(target.textContent).toContain('Requests');
    expect(target.textContent).toContain('Achievements');
    expect(target.textContent).toContain('Add Friend');
    expect(target.textContent).toContain('No Friends Yet');
    unmount(compFriends);
    target.remove();

    // Reset back to Russian default
    i18n.setLocale('ru');
  });
});
