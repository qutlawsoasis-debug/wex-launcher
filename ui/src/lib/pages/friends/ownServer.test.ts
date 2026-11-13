import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), handlers: new Map<string, (e: { payload: unknown }) => void>() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, h: (e: { payload: unknown }) => void) => {
    mocks.handlers.set(name, h);
    return () => mocks.handlers.delete(name);
  }),
}));

import { i18n } from '../../i18n';
import { ownServers, defaultSettings, friendAddress, type OwnServer } from './ownServer.svelte';
import ServerTab from './ServerTab.svelte';

function view(over: Partial<OwnServer> = {}): OwnServer {
  return {
    id: 'srv-0000aaaa',
    version: '1.21.1',
    createdAt: '2026-11-13T00:00:00Z',
    settings: { ...defaultSettings(), name: 'Дача', publicAddress: null },
    eulaAccepted: false,
    installed: null,
    installing: false,
    status: 'stopped',
    players: [],
    whitelist: [],
    lanAddress: '192.168.1.5',
    localAddress: 'localhost',
    publicAddress: null,
    problem: null,
    ...over,
  };
}

const installed = { minecraft: '1.21.1', fabricLoader: '0.19.5', packVersion: '1', modSource: 'fabricModJson' as const, mods: ['a.jar', 'b.jar'] };
let server: OwnServer | null;
let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;

const btn = (label: string) => [...target.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label);

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  i18n.setLocale('ru');
  mocks.invoke.mockReset();
  mocks.handlers.clear();
  ownServers.list = null;
  ownServers.selectedId = null;
  ownServers.error = '';
  ownServers.log = [];
  server = null;
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, any>) => {
    switch (cmd) {
      case 'own_server_list':
        return server ? [server] : [];
      case 'own_server_create':
        server = view({ version: args!.version, settings: args!.settings });
        return server;
      case 'own_server_install':
        server = { ...server!, installed };
        return server;
      case 'own_server_accept_eula':
        server = { ...server!, eulaAccepted: true };
        return server;
      case 'own_server_start':
        server = { ...server!, status: 'starting' };
        return server;
      case 'own_server_stop':
        server = { ...server!, status: 'stopped', players: [] };
        return server;
      case 'own_server_update':
        server = { ...server!, settings: args!.settings };
        return server;
      case 'own_server_console':
        return server?.status === 'running' || server?.status === 'starting'
          ? { status: 'running', players: ['Steve'], lines: [{ seq: 1, text: '[t] Done (5s)!' }], nextSeq: 1, problem: null }
          : null;
      case 'own_server_delete':
        server = null;
        return null;
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

describe('own server tab', () => {
  it('with no servers offers to create one; version choice reaches core', async () => {
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('Новый сервер'));
    btn('1.20.1')!.click();
    flushSync();
    btn('Создать сервер')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('own_server_create', { version: '1.20.1', settings: expect.objectContaining({ name: 'Сервер друзей', maxPlayers: 8 }) })
    );
    await vi.waitFor(() => expect(target.textContent).toContain('Minecraft 1.20.1, остановлен'));
  });

  it('install → EULA only by the explicit switch → start → players and console → stop', async () => {
    server = view();
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(btn('Установить')).toBeTruthy());
    expect(mocks.invoke).not.toHaveBeenCalledWith('own_server_accept_eula', expect.anything());
    btn('Установить')!.click();
    await vi.waitFor(() => expect(target.textContent).toContain('Моды сборки на сервере: 2'));
    // Not accepted: Start is disabled.
    expect(btn('Запустить')!.disabled).toBe(true);
    target.querySelector<HTMLButtonElement>('[role="switch"][aria-labelledby="own-eula-l"]')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('own_server_accept_eula', { id: 'srv-0000aaaa' }));
    await vi.waitFor(() => expect(btn('Запустить')!.disabled).toBe(false));
    btn('Запустить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('own_server_start', { id: 'srv-0000aaaa' }));
    await ownServers.poll();
    flushSync();
    expect(target.textContent).toContain('Steve');
    expect(target.querySelector('[role="log"]')!.textContent).toContain('Done (5s)!');
    btn('Остановить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('own_server_stop', { id: 'srv-0000aaaa' }));
  });

  it('shows the LAN address to copy, a port-forwarding hint and adds itself to Servers', async () => {
    server = view({ settings: { ...defaultSettings(), name: 'Дача', port: 25570 }, lanAddress: '192.168.1.5:25570' });
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('192.168.1.5:25570'));
    expect(target.textContent).toContain('Пробросьте TCP-порт 25570 на роутере на 192.168.1.5');
    expect(target.textContent).toContain('нужен VPS или хостинг');
    expect(btn('Скопировать для друзей')).toBeTruthy();
    btn('Добавить себе в Серверы')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('own_server_add_to_servers', { id: 'srv-0000aaaa' }));
  });

  it('settings: toggles and segmented controls are saved together', async () => {
    server = view({ installed, eulaAccepted: true });
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(btn('Сохранить')).toBeTruthy());
    expect(btn('Сохранить')!.disabled).toBe(true);
    btn('Творчество')!.click();
    target.querySelector<HTMLButtonElement>('[aria-labelledby="own-wl-l"]')!.click();
    flushSync();
    btn('Сохранить')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('own_server_update', { id: 'srv-0000aaaa', settings: expect.objectContaining({ gamemode: 'creative', whitelist: true }) })
    );
    await vi.waitFor(() => expect(target.textContent).toContain('Белый список меняется, пока сервер запущен.'));
  });

  it('delete asks first, then removes the server and its world', async () => {
    server = view({ installed });
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(btn('Удалить сервер')).toBeTruthy());
    btn('Удалить сервер')!.click();
    flushSync();
    expect(mocks.invoke).not.toHaveBeenCalledWith('own_server_delete', expect.anything());
    expect(target.textContent).toContain('Это нельзя отменить');
    btn('Удалить')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('own_server_delete', { id: 'srv-0000aaaa' }));
    await tick();
    await vi.waitFor(() => expect(target.textContent).toContain('Создать сервер'));
  });

  it('errors from core are shown as they are', async () => {
    server = view({ installed, eulaAccepted: true });
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'own_server_list') return [server];
      if (cmd === 'own_server_start') throw 'Порт 25565 уже занят. Остановите другой сервер или выберите другой порт.';
      return null;
    });
    comp = mount(ServerTab, { target });
    await vi.waitFor(() => expect(btn('Запустить')).toBeTruthy());
    btn('Запустить')!.click();
    await vi.waitFor(() => expect(target.querySelector('[role="alert"]')!.textContent).toContain('Порт 25565 уже занят'));
  });

  it('friend address prefers the typed public address', () => {
    expect(friendAddress(view())).toBe('192.168.1.5');
    expect(friendAddress(view({ publicAddress: 'play.example.com' }))).toBe('play.example.com');
  });
});
