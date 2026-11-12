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
    expect(t('shell.play')).toBe('Играть');
    expect(t('shell.nav_options')).toBe('Настройки');
    expect(t('shell.offline')).toBe('Не в сети');
  });

  it('switches to English immediately without restart', () => {
    i18n.setLocale('en');

    expect(t('shell.play')).toBe('Play');
    expect(t('shell.nav_options')).toBe('Settings');
    expect(t('shell.offline')).toBe('Offline');

    // Switch back to Russian
    i18n.setLocale('ru');
    expect(t('shell.play')).toBe('Играть');
  });

  it('interpolates single and multiple parameters accurately', () => {
    i18n.setLocale('ru');
    expect(t('shell.memory_gb', { gb: 6 })).toBe('6 ГБ памяти');
    expect(t('shell.of', { done: '1 МБ', total: '2 МБ' })).toBe('1 МБ из 2 МБ');

    i18n.setLocale('en');
    expect(t('shell.memory_gb', { gb: 6 })).toBe('6 GB memory');
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

  it('renders screens in authentic English when locale is set to en', async () => {
    const { mount, unmount, tick } = await import('svelte');
    const { default: SettingsPage } = await import('../pages/settings/SettingsPage.svelte');
    const { default: LaunchBar } = await import('../shell/LaunchBar.svelte');
    const { default: ServersPage } = await import('../pages/servers/ServersPage.svelte');
    const { default: BuildPage } = await import('../pages/build/BuildPage.svelte');
    const { default: WardrobePage } = await import('../pages/wardrobe/WardrobePage.svelte');
    const { default: FriendsPage } = await import('../pages/friends/FriendsPage.svelte');

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

    // 3. Servers
    const compServers = mount(ServersPage, { target });
    await tick();
    expect(target.textContent).toContain('Servers');
    expect(target.textContent).toContain('Add server');
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

    // 6. Wardrobe
    const compWardrobe = mount(WardrobePage, { target });
    await tick();
    for (const label of ['Wardrobe', 'Skins', 'Capes', 'Profile', 'Stands', 'Walks', 'Upload skin', 'Arm model']) {
      expect(target.textContent).toContain(label);
    }
    unmount(compWardrobe);
    target.innerHTML = '';

    // 7. Friends with the Rooms tab
    const compFriends = mount(FriendsPage, { target });
    await tick();
    for (const label of ['Friends', 'Rooms', 'Friends will appear once accounts are connected.']) {
      expect(target.textContent).toContain(label);
    }
    unmount(compFriends);
    target.remove();

    // Reset back to Russian default
    i18n.setLocale('ru');
  });
});
