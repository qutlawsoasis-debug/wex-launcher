import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), drop: null as null | ((e: { payload: unknown }) => void) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (cb: (e: { payload: unknown }) => void) => {
      mocks.drop = cb;
      return () => (mocks.drop = null);
    },
  }),
}));

import { builds } from '../../shell/builds.svelte';
import { launch } from '../../shell/launch.svelte';
import { toasts } from '../../shell/toast.svelte';
import { i18n } from '../../i18n';
import { build } from './build.svelte';
import BuildPage from './BuildPage.svelte';

let settings: Record<string, unknown>;
let mods: Array<Record<string, unknown>>;
let shaders: Array<Record<string, unknown>>;

beforeEach(async () => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  mocks.invoke.mockReset();
  settings = { selected_version: '1.21.1', player_name: 'Player', ram_mb: 6144, total_memory_mb: 16384, java_mode: 'auto' };
  mods = [
    { id: 'fabric', name: 'Fabric API', version: '0.110.0', description: '', file_name: 'fabric.jar', enabled: true, is_pack_mod: true },
    { id: 'menu', name: 'Mod Menu', version: '11.0.3', description: 'Меню модов', file_name: 'menu.jar', enabled: false, is_pack_mod: false },
  ];
  shaders = [{ name: 'BSL', file_name: 'bsl.zip', file_path: 'x', size_bytes: 1048576, description: null, is_active: false, is_from_pack: false }];
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
        return mods;
      case 'toggle_mod':
        return { ...mods.find((m) => m.file_name === args!.fileName), enabled: args!.enabled };
      case 'add_custom_mod':
        return { id: 'new', name: 'New Mod', version: '1.0', description: '', file_name: 'new.jar', enabled: true, is_pack_mod: false };
      case 'workshop_list_worlds':
        return args!.version === '1.20.1'
          ? [{ folder_name: 'w', folder_path: 'C:/Wex/instances/1.20.1/saves/w', display_name: 'Северная база', game_mode: 'Creative', last_played: Date.now(), day_time: 214 * 24000, size_bytes: 1 }]
          : [];
      case 'get_data_dir':
        return 'C:\\Users\\P\\AppData\\Roaming\\Wex';
      case 'workshop_list_shader_packs':
        return shaders;
      case 'workshop_list_resource_packs':
        return [];
      case 'workshop_list_screenshots':
        return [];
      case 'workshop_list_world_backups':
        return [{ file_name: 'w-1.zip', file_path: 'C:/b/w-1.zip', created_at: null, size_bytes: 2048 }];
      case 'verify_game_files':
        return { status_message: 'Все файлы в порядке' };
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  toasts.dismiss();
  launch.onStatus({ state: 'idle', message: '' });
  await builds.load();
  build.open('');
});

let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  target = document.createElement('div');
  target.className = 'wx';
  document.body.appendChild(target);
});
afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  target.remove();
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

async function open(version = '1.20.1') {
  comp = mount(BuildPage, { target, props: { version } });
  await vi.waitFor(() => expect(target.querySelectorAll('.lrow').length).toBeGreaterThan(0));
  await tick();
}

function tab(id: string) {
  (target.querySelector(`#btab-${id}`) as HTMLButtonElement).click();
  flushSync();
}

describe('build page', () => {
  it('header and facts only state what core knows', async () => {
    await open();
    expect(target.querySelector('.build-head h1')?.textContent).toBe('Wex 1.20.1');
    expect(target.querySelector('.build-head p')?.textContent).toBe('Fabric, Java 17');
    expect(target.querySelector('.build-head')?.textContent).toContain('Выбрать для запуска');
    await vi.waitFor(() => expect(target.querySelectorAll('.fact').length).toBe(3));
    const facts = [...target.querySelectorAll('.fact span')].map((s) => s.textContent);
    expect(facts).toEqual(['мода', 'мир', 'памяти']);
    expect(target.textContent).not.toContain('последний запуск');
  });

  it('has the workshop as tabs next to Mods and Worlds', async () => {
    await open();
    const tabs = [...target.querySelectorAll('[role="tab"]')].map((b) => b.textContent);
    expect(tabs).toEqual(['Моды', 'Миры', 'Ресурспаки', 'Шейдеры', 'Скриншоты', 'Бэкапы', 'Настройки сборки']);
    const first = target.querySelector<HTMLButtonElement>('#btab-mods')!;
    first.parentElement!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    flushSync();
    expect(target.querySelector('#btab-worlds')?.getAttribute('aria-selected')).toBe('true');
  });

  it('selecting the build saves it', async () => {
    await open();
    [...target.querySelectorAll<HTMLButtonElement>('.build-head .btn')].find((b) => b.textContent?.includes('Выбрать'))!.click();
    await vi.waitFor(() => expect(settings.selected_version).toBe('1.20.1'));
  });
});

describe('mods tab', () => {
  it('lists mods with the pack tag, filters and toggles through core', async () => {
    await open();
    const rows = target.querySelectorAll('.lrow');
    expect(rows.length).toBe(2);
    expect(rows[0].querySelector('.tag')?.textContent).toBe('Из сборки Aura');
    expect(rows[1].textContent).toContain('11.0.3, Меню модов');
    // No invented update tags or author.
    expect(target.textContent).not.toMatch(/С обновлением|Есть \d/);
    [...target.querySelectorAll<HTMLButtonElement>('.seg button')].find((b) => b.textContent === 'Включённые')!.click();
    flushSync();
    expect(target.querySelectorAll('.lrow').length).toBe(1);
    [...target.querySelectorAll<HTMLButtonElement>('.seg button')].find((b) => b.textContent === 'Все')!.click();
    flushSync();
    const input = target.querySelector<HTMLInputElement>('.search input')!;
    input.value = 'меню';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(target.querySelectorAll('.lrow').length).toBe(1);
    target.querySelector<HTMLButtonElement>('.lrow .switch')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('toggle_mod', { version: '1.20.1', fileName: 'menu.jar', enabled: true }));
    await vi.waitFor(() => expect(target.querySelector('.lrow .switch')?.getAttribute('aria-checked')).toBe('true'));
  });

  it('adds a mod from a .jar dropped onto the window', async () => {
    await open();
    await vi.waitFor(() => expect(mocks.drop).not.toBeNull());
    mocks.drop!({ payload: { type: 'drop', paths: ['C:/Downloads/new.jar', 'C:/Downloads/readme.txt'], position: { x: 0, y: 0 } } });
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('add_custom_mod', { version: '1.20.1', sourcePath: 'C:/Downloads/new.jar' }));
    expect(mocks.invoke).not.toHaveBeenCalledWith('add_custom_mod', expect.objectContaining({ sourcePath: 'C:/Downloads/readme.txt' }));
    await vi.waitFor(() => expect(toasts.current?.title).toBe('Добавлен 1 мод'));
  });
});

describe('worlds tab', () => {
  it('shows mode, in-game days and when it was played; no play-into-world button', async () => {
    await open();
    tab('worlds');
    const row = target.querySelector('.lrow')!;
    expect(row.textContent).toContain('Северная база');
    expect(row.textContent).toContain('Творческий режим, 214 дней в игре, последний вход сегодня');
    expect(row.textContent).not.toContain('Играть');
  });

  it('empty build explains how to add a world and opens saves', async () => {
    await open('1.21.1');
    tab('worlds');
    await vi.waitFor(() => expect(target.textContent).toContain('В этой сборке ещё нет миров.'));
    [...target.querySelectorAll<HTMLButtonElement>('.empty .mini')].find((b) => b.textContent === 'Открыть saves')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('workshop_open_path', { path: 'C:\\Users\\P\\AppData\\Roaming\\Wex\\instances\\1.21.1\\saves' })
    );
  });
});

describe('workshop tabs', () => {
  it('shaders are a single choice and the choice is saved', async () => {
    await open();
    tab('shaders');
    await vi.waitFor(() => expect(target.querySelectorAll('[role="radio"]').length).toBe(2));
    const radios = target.querySelectorAll<HTMLButtonElement>('[role="radio"]');
    expect(radios[0].getAttribute('aria-checked')).toBe('true');
    radios[1].click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('workshop_set_active_shader_pack', { shaderFileName: 'bsl.zip', version: '1.20.1' }));
    await vi.waitFor(() => expect(target.querySelectorAll('[role="radio"]')[1].getAttribute('aria-checked')).toBe('true'));
  });

  it('restoring a backup asks twice before replacing the world', async () => {
    await open();
    tab('backups');
    await vi.waitFor(() => expect(target.querySelector('.lrow-backup')).not.toBeNull());
    const btn = target.querySelector<HTMLButtonElement>('.lrow-backup .mini')!;
    btn.click();
    flushSync();
    expect(btn.textContent?.trim()).toBe('Нажмите ещё раз');
    expect(mocks.invoke).not.toHaveBeenCalledWith('workshop_restore_world_backup', expect.anything());
    btn.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('workshop_restore_world_backup', {
        backupPath: 'C:/b/w-1.zip',
        targetWorldFolder: 'C:/Wex/instances/1.20.1/saves/w',
        version: '1.20.1',
      })
    );
  });
});

describe('build settings tab', () => {
  it('memory is saved to the shared config and says it is shared', async () => {
    {
      await open();
      tab('settings');
      await vi.waitFor(() => expect(target.querySelector<HTMLInputElement>('input[type="range"]')?.disabled).toBe(false));
      expect(target.textContent).toContain('Одна настройка для обеих сборок.');
      const range = target.querySelector<HTMLInputElement>('input[type="range"]')!;
      range.value = '8';
      range.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await vi.waitFor(() => expect(settings.ram_mb).toBe(8192), { timeout: 2000 });
    expect(settings.player_name).toBe('Player');
  });

  it('verify files calls verify_game_files and reports the result', async () => {
    await open();
    tab('settings');
    [...target.querySelectorAll<HTMLButtonElement>('.frow .btn')].find((b) => b.textContent === 'Проверить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('verify_game_files', { version: '1.20.1' }));
    await vi.waitFor(() => expect(toasts.current?.detail).toBe('Все файлы в порядке'));
  });
});
