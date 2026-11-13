import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), start: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock('../../shell/launch.svelte', () => ({ launch: { busy: false, start: mocks.start } }));

import { i18n } from '../../i18n';
import { builds } from '../../shell/builds.svelte';
import { cloudWorlds, FAST_POLL_MS, SLOW_POLL_MS, type CloudWorld } from './cloud.svelte';
import CloudTab from './CloudTab.svelte';

function world(over: Partial<CloudWorld> = {}): CloudWorld {
  return {
    id: 'w1',
    name: 'Дача',
    owner: 'Steve',
    seed: null,
    allowedPlayers: ['Alex'],
    state: 'sleeping',
    port: null,
    address: null,
    onlinePlayers: [],
    maxPlayers: 8,
    isOwner: true,
    stageText: null,
    progress: 0,
    gameVersion: '1.21.1',
    lastPlayed: 1,
    ...over,
  };
}

let worlds: CloudWorld[];
let available: boolean;
let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;

const btn = (label: string) => [...target.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label);
const calls = (cmd: string) => mocks.invoke.mock.calls.filter((c) => c[0] === cmd);

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  i18n.setLocale('ru');
  mocks.invoke.mockReset();
  mocks.start.mockReset();
  cloudWorlds.status = null;
  cloudWorlds.list = null;
  cloudWorlds.error = '';
  cloudWorlds.busy = {};
  worlds = [];
  available = true;
  builds.list = [
    { id: '1.21.1', name: '1.21.1' },
    { id: '1.20.1', name: '1.20.1' },
  ] as unknown as typeof builds.list;
  builds.selectedId = '1.21.1';
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, any>) => {
    switch (cmd) {
      case 'cloud_status':
        return available
          ? { available: true, reason: null, owner: 'Steve', host: '10.0.0.7' }
          : { available: false, reason: 'Не задан ключ облачного сервера (WEX_CLOUD_SECRET)', owner: 'Steve', host: null };
      case 'cloud_list':
        return { owner: 'Steve', maxActive: 3, activeCount: worlds.filter((w) => w.state !== 'sleeping').length, worlds };
      case 'cloud_create':
        worlds = [...worlds, world({ id: 'new', name: args!.name, gameVersion: args!.version, state: 'starting' })];
        return { worldId: 'new', state: 'starting', port: 25620, address: '10.0.0.7:25620' };
      case 'cloud_start':
        worlds = worlds.map((w) => (w.id === args!.id ? { ...w, state: 'starting', progress: 40, stageText: 'Загрузка мира' } : w));
        return { worldId: args!.id, state: 'starting', port: 25601, address: '10.0.0.7:25601' };
      case 'cloud_stop':
        worlds = worlds.map((w) => (w.id === args!.id ? { ...w, state: 'sleeping', port: null, address: null } : w));
        return null;
      case 'cloud_share':
        return args!.add ? ['Alex', args!.player] : [];
      case 'cloud_delete':
        worlds = worlds.filter((w) => w.id !== args!.id);
        return null;
      case 'get_launcher_settings':
        return { selected_version: '1.21.1' };
      default:
        return null;
    }
  });
  target = document.createElement('div');
  target.className = 'wx';
  document.body.appendChild(target);
});

afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  target.remove();
});

describe('cloud worlds tab', () => {
  it('without a secret says why and never asks for the list', async () => {
    available = false;
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('Облачные миры недоступны.'));
    expect(target.textContent).toContain('WEX_CLOUD_SECRET');
    expect(calls('cloud_list')).toHaveLength(0);
  });

  it('with no worlds of mine offers to create one; name, version and seed reach core', async () => {
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('Новый облачный мир'));
    expect(btn('Создать мир')!.disabled).toBe(true);
    const name = target.querySelector<HTMLInputElement>('#cloud-name')!;
    name.value = 'Дача';
    name.dispatchEvent(new Event('input'));
    const seed = target.querySelector<HTMLInputElement>('#cloud-seed')!;
    seed.value = ' 42 ';
    seed.dispatchEvent(new Event('input'));
    btn('1.20.1')!.click();
    flushSync();
    btn('Создать мир')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_create', { name: 'Дача', version: '1.20.1', seed: '42' }));
    await vi.waitFor(() => expect(target.textContent).toContain('Мои миры'));
    expect(target.textContent).toContain('Запускается');
    // The owner is never sent from the UI.
    expect(calls('cloud_create')[0][1]).not.toHaveProperty('owner');
  });

  it('empty seed is sent as null', async () => {
    await cloudWorlds.load();
    await cloudWorlds.create('Дача', '1.21.1', '   ');
    expect(mocks.invoke).toHaveBeenCalledWith('cloud_create', { name: 'Дача', version: '1.21.1', seed: null });
  });

  it('start shows the stage and progress, then Join launches the world version at its address', async () => {
    worlds = [world({ gameVersion: '1.20.1' })];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(btn('Запустить')).toBeTruthy());
    btn('Запустить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_start', { id: 'w1' }));
    await cloudWorlds.refresh();
    flushSync();
    expect(target.textContent).toContain('Загрузка мира');
    expect(target.querySelector('[role="progressbar"]')).toBeTruthy();
    worlds = [world({ gameVersion: '1.20.1', state: 'running', port: 25601, address: '10.0.0.7:25601', onlinePlayers: ['Alex'] })];
    await cloudWorlds.refresh();
    flushSync();
    expect(target.textContent).toContain('В сети · игроков 1 из 8');
    expect(target.textContent).toContain('10.0.0.7:25601');
    btn('Зайти')!.click();
    await vi.waitFor(() => expect(mocks.start).toHaveBeenCalledWith('10.0.0.7:25601'));
    expect(builds.selectedId).toBe('1.20.1');
  });

  it('stop asks core and the world goes back to sleep', async () => {
    worlds = [world({ state: 'running', port: 25601, address: '10.0.0.7:25601' })];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(btn('Остановить')).toBeTruthy());
    btn('Остановить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_stop', { id: 'w1' }));
    await vi.waitFor(() => expect(target.textContent).toContain('Спит'));
  });

  it('share: validates the nickname, gives and takes back access', async () => {
    worlds = [world()];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(btn('Доступ (1)')).toBeTruthy());
    btn('Доступ (1)')!.click();
    flushSync();
    const input = target.querySelector<HTMLInputElement>('#cloud-share-w1')!;
    input.value = 'Ал';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(btn('Открыть доступ')!.disabled).toBe(true);
    expect(target.textContent).toContain('3–16 латинских букв');
    input.value = 'Bob_1';
    input.dispatchEvent(new Event('input'));
    flushSync();
    btn('Открыть доступ')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_share', { id: 'w1', player: 'Bob_1', add: true }));
    await vi.waitFor(() => expect(target.textContent).toContain('Bob_1'));
    btn('Убрать')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_share', { id: 'w1', player: 'Alex', add: false }));
  });

  it('delete asks first and is not offered while the world runs', async () => {
    worlds = [world({ state: 'running', port: 1, address: 'h:1' })];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(btn('Удалить мир')).toBeTruthy());
    expect(btn('Удалить мир')!.disabled).toBe(true);
    worlds = [world()];
    await cloudWorlds.refresh();
    flushSync();
    btn('Удалить мир')!.click();
    flushSync();
    expect(calls('cloud_delete')).toHaveLength(0);
    expect(target.textContent).toContain('Это нельзя отменить');
    btn('Удалить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cloud_delete', { id: 'w1' }));
    await vi.waitFor(() => expect(target.textContent).toContain('Новый облачный мир'));
  });

  it('shared worlds show the owner and no owner actions', async () => {
    worlds = [world({ id: 's1', owner: 'Alex', isOwner: false, name: 'Замок' })];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('владелец Alex'));
    expect(target.textContent).toContain('Миры друзей');
    expect(btn('Удалить мир')).toBeUndefined();
    expect(target.textContent).not.toContain('Доступ (');
  });

  it('errors from core are shown as they are', async () => {
    worlds = [world()];
    comp = mount(CloudTab, { target });
    await vi.waitFor(() => expect(btn('Запустить')).toBeTruthy());
    mocks.invoke.mockImplementationOnce(async () => {
      throw 'Достигнут лимит активных миров (3)';
    });
    btn('Запустить')!.click();
    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')!.textContent).toContain('Достигнут лимит'));
    await vi.waitFor(() => expect(btn('Запустить')).toBeTruthy());
  });

  it('polls fast only while a world is starting or stopping', async () => {
    worlds = [world()];
    await cloudWorlds.load();
    const t0 = Date.now();
    mocks.invoke.mockClear();
    await cloudWorlds.tick(t0 + FAST_POLL_MS);
    expect(calls('cloud_list')).toHaveLength(0);
    await cloudWorlds.tick(t0 + SLOW_POLL_MS + 10);
    expect(calls('cloud_list')).toHaveLength(1);
    worlds = [world({ state: 'starting' })];
    await cloudWorlds.refresh();
    mocks.invoke.mockClear();
    await cloudWorlds.tick(Date.now() + FAST_POLL_MS);
    expect(calls('cloud_list')).toHaveLength(1);
  });
});
