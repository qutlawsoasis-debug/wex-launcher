import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { builds } from './builds.svelte';
import { launch } from './launch.svelte';
import { router } from './router.svelte';
import { toasts } from './toast.svelte';
import { palette, PALETTE_LIMIT } from './commands.svelte';
import { handleHotkey } from './hotkeys';
import { buildIntent } from '../pages/build/build.svelte';
import { i18n } from '../i18n';
import CommandPalette from './CommandPalette.svelte';

let settings: Record<string, unknown>;

beforeEach(async () => {
  mocks.invoke.mockReset();
  settings = { selected_version: '1.21.1', player_name: 'Player', ram_mb: 6144 };
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'get_launcher_settings':
        return { ...settings };
      case 'save_launcher_settings':
        settings = { ...(args!.newSettings as Record<string, unknown>) };
        return null;
      case 'get_installed_versions':
        return [
          { id: '1.21.1', name: '1.21.1', is_installed: true, required_java: 21 },
          { id: '1.20.1', name: '1.20.1', is_installed: true, required_java: 17 },
        ];
      case 'list_mods':
        return [
          { name: 'Sodium', version: '0.6.5', enabled: true },
          { name: 'Iris Shaders', version: '1.8.1', enabled: false },
        ];
      case 'list_servers':
        return [{ name: 'Aura Official', address: 'mc.auramc.net' }];
      case 'start_game':
        return new Promise(() => {});
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  router.reset();
  toasts.dismiss();
  launch.onStatus({ state: 'idle', message: '' });
  palette.hide();
  await builds.load();
});

let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  target.remove();
  launch.onStatus({ state: 'cancelled', message: '' });
});

const ctx = { overlayOpen: false, closeOverlay: () => false };
function key(k: string, init: KeyboardEventInit = {}) {
  return new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init });
}

async function openPalette() {
  comp = mount(CommandPalette, { target });
  palette.show();
  await vi.waitFor(() => expect(palette.servers.length).toBe(1));
  flushSync();
  await tick();
}

describe('command palette', () => {
  it('lists actions, builds, servers, mods and sections from real data', async () => {
    await openPalette();
    palette.mods = palette.mods.slice(0, 1);
    flushSync();
    const groups = [...target.querySelectorAll('.cmd-g')].map((g) => g.textContent);
    expect(groups).toEqual(['Действия', 'Сборки', 'Серверы', 'Моды', 'Разделы']);
    const rows = [...target.querySelectorAll('.cmd-row b')].map((b) => b.textContent);
    expect(rows[0]).toBe('Играть в Wex 1.21.1');
    expect(rows).toContain('Сменить сборку на Wex 1.20.1');
    expect(rows).toContain('Aura Official');
    expect(rows.length).toBeLessThanOrEqual(PALETTE_LIMIT);
    // No sample friends: friendsLive has none.
    expect(groups).not.toContain('Друзья');
    expect(document.activeElement).toBe(target.querySelector('.cmd-in input'));
  });

  it('filters as you type and explains an empty result', async () => {
    await openPalette();
    const input = target.querySelector<HTMLInputElement>('.cmd-in input')!;
    input.value = 'sod';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    const rows = [...target.querySelectorAll('.cmd-row b')].map((b) => b.textContent);
    expect(rows).toEqual(['Sodium']);
    expect(target.querySelector('.cmd-row span')?.textContent).toBe('0.6.5, включён');
    input.value = 'zzzz';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector('.cmd-empty')?.textContent).toContain('Ничего не нашлось');
  });

  it('arrows move the selection, Enter runs it and closes the palette', async () => {
    await openPalette();
    const input = target.querySelector<HTMLInputElement>('.cmd-in input')!;
    input.value = 'aura';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector('.cmd-row[aria-selected="true"] b')?.textContent).toBe('Aura Official');
    expect(input.getAttribute('aria-activedescendant')).toBe('cmd-0');
    input.dispatchEvent(key('Enter'));
    flushSync();
    expect(palette.open).toBe(false);
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ server: 'mc.auramc.net' })));
  });

  it('a mod opens the build page on the Mods tab with the name searched', async () => {
    await openPalette();
    const input = target.querySelector<HTMLInputElement>('.cmd-in input')!;
    input.value = 'iris';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    input.dispatchEvent(key('Enter'));
    expect(router.route).toBe('build-1.21.1');
    expect(buildIntent).toEqual({ tab: 'mods', modQuery: 'Iris Shaders' });
    buildIntent.tab = null;
    buildIntent.modQuery = '';
  });

  it('Esc closes and gives focus back; clicking the backdrop closes too', async () => {
    const opener = document.createElement('button');
    document.body.appendChild(opener);
    opener.focus();
    await openPalette();
    target.querySelector<HTMLInputElement>('.cmd-in input')!.dispatchEvent(key('Escape'));
    flushSync();
    expect(palette.open).toBe(false);
    await tick();
    await Promise.resolve();
    expect(document.activeElement).toBe(opener);
    palette.show();
    flushSync();
    (target.querySelector('.cmd-bg') as HTMLElement).click();
    expect(palette.open).toBe(false);
    opener.remove();
  });
});

describe('hotkeys', () => {
  it('Ctrl K opens and closes the palette', () => {
    expect(handleHotkey(key('k', { ctrlKey: true }), ctx)).toBe(true);
    expect(palette.open).toBe(true);
    expect(handleHotkey(key('k', { ctrlKey: true }), ctx)).toBe(true);
    expect(palette.open).toBe(false);
  });

  it('Ctrl 2 selects Wex 1.20.1 and saves it; Ctrl 1 goes back', async () => {
    handleHotkey(key('2', { ctrlKey: true }), ctx);
    await vi.waitFor(() => expect(settings.selected_version).toBe('1.20.1'));
    expect(toasts.current?.title).toBe('Для запуска выбрана Wex 1.20.1');
    handleHotkey(key('1', { ctrlKey: true }), ctx);
    await vi.waitFor(() => expect(settings.selected_version).toBe('1.21.1'));
  });

  it('Alt ← and Alt → move through history', () => {
    router.go('servers');
    router.go('options');
    handleHotkey(key('ArrowLeft', { altKey: true }), ctx);
    expect(router.route).toBe('servers');
    handleHotkey(key('ArrowRight', { altKey: true }), ctx);
    expect(router.route).toBe('options');
  });

  it('Enter plays the selected build only when nothing else has focus', async () => {
    const field = document.createElement('input');
    document.body.appendChild(field);
    field.focus();
    expect(handleHotkey(key('Enter'), ctx)).toBe(false);
    const btn = document.createElement('button');
    document.body.appendChild(btn);
    btn.focus();
    expect(handleHotkey(key('Enter'), ctx)).toBe(false);
    btn.blur();
    field.blur();
    expect(handleHotkey(key('Enter'), ctx)).toBe(true);
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ version: '1.21.1' })));
    field.remove();
    btn.remove();
  });

  it('while a sheet is open only Esc and Ctrl K work', () => {
    const closeOverlay = vi.fn(() => true);
    const sheet = { overlayOpen: true, closeOverlay };
    expect(handleHotkey(key('2', { ctrlKey: true }), sheet)).toBe(false);
    expect(handleHotkey(key('Enter'), sheet)).toBe(false);
    expect(handleHotkey(key('Escape'), sheet)).toBe(true);
    expect(closeOverlay).toHaveBeenCalledOnce();
  });
});
