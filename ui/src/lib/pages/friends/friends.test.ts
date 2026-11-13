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

import { builds } from '../../shell/builds.svelte';
import { launch } from '../../shell/launch.svelte';
import { toasts } from '../../shell/toast.svelte';
import { friendsLive } from '../../services/friendsLive';
import { i18n } from '../../i18n';
import { rooms } from './rooms.svelte';
import { friendsIntent } from './friendsIntent.svelte';
import FriendsPage from './FriendsPage.svelte';
import ServersPage from '../servers/ServersPage.svelte';

let servers: Array<{ id?: string; name: string; address: string }>;
const room = {
  code: 'AB12',
  host_name: 'Host',
  world_name: 'Северная база',
  tunnel_address: 'tun.example:25565',
  tunnel_active: true,
  tunnel_error: null,
  is_host: true,
  players: [
    { name: 'Host', is_host: true, ping_ms: 0, mods_synced: true },
    { name: 'Guest', is_host: false, ping_ms: 30, mods_synced: true },
  ],
  mod_mismatch_count: 0,
  total_mods: 130,
};

beforeEach(async () => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  mocks.invoke.mockReset();
  mocks.handlers.clear();
  servers = [
    { id: 'aura-main', name: 'Aura Official', address: 'mc.auramc.net' },
    { name: 'My server', address: 'my.example.net' },
  ];
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, any>) => {
    switch (cmd) {
      case 'get_launcher_settings':
        return { selected_version: '1.21.1', player_name: 'Player' };
      case 'get_installed_versions':
        return [{ id: '1.21.1', name: '1.21.1', is_installed: true }];
      case 'list_servers':
        return servers;
      case 'ping_server_details':
        return args!.address === 'mc.auramc.net' ? { success: true, latency_ms: 23, online: 214, max_players: 500, motd: '§6Выживание\n§rи PvP' } : { success: false };
      case 'add_server':
        servers = [...servers, { name: args!.name, address: args!.address }];
        return servers.at(-1);
      case 'edit_server':
        servers = servers.map((s) => (s.address === args!.oldAddress ? { ...s, name: args!.name, address: args!.newAddress } : s));
        return null;
      case 'remove_server':
        servers = servers.filter((s) => s.address !== args!.address);
        return null;
      case 'list_lan_worlds':
        return [{ motd: 'Мой мир', address: '192.168.1.5:51234', port: 51234 }];
      case 'list_open_rooms':
        return [];
      case 'publish_lan_world':
        return { ...room };
      case 'prepare_lobby_join':
        return { ...room, is_host: false };
      case 'get_lobby_status':
        return rooms.active;
      case 'start_game':
        return new Promise(() => {});
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  toasts.dismiss();
  launch.onStatus({ state: 'idle', message: '' });
  rooms.active = null;
  rooms.error = '';
  friendsLive.friends = [];
  friendsLive.requests = [];
  await builds.load();
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
  launch.onStatus({ state: 'cancelled', message: '' });
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('servers page', () => {
  it('lists servers from core with live players and ping; no sample servers', async () => {
    comp = mount(ServersPage, { target });
    await vi.waitFor(() => expect(target.querySelectorAll('.lrow-srv').length).toBe(2));
    await vi.waitFor(() => expect(target.querySelector('.lrow-srv .meter')?.textContent).toContain('214 из 500, пинг 23 мс'));
    expect(target.querySelectorAll('.lrow-srv')[1].textContent).toContain('Не отвечает');
    // Description from the server (colour codes removed), then the address.
    expect(target.querySelector('.lrow-srv > div > span')?.textContent).toBe('Выживание и PvP, mc.auramc.net');
    expect(target.textContent).not.toMatch(/Aura Creative|Aura Minigames|примеры/);
    expect(target.querySelector('.lrow-srv .btn')?.classList.contains('blue')).toBe(true);
    (target.querySelector('.lrow-srv .btn') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ server: 'mc.auramc.net' })));
  });

  it('adds, edits (camelCase arguments) and removes on a second press', async () => {
    comp = mount(ServersPage, { target });
    await vi.waitFor(() => expect(target.querySelectorAll('.lrow-srv').length).toBe(2));
    [...target.querySelectorAll<HTMLButtonElement>('.btn')].find((b) => b.textContent === 'Добавить сервер')!.click();
    flushSync();
    const [nameIn, addrIn] = target.querySelectorAll<HTMLInputElement>('.editor input');
    nameIn.value = 'New';
    nameIn.dispatchEvent(new Event('input', { bubbles: true }));
    addrIn.value = 'bad address!';
    addrIn.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector<HTMLButtonElement>('.editor .btn.blue')!.disabled).toBe(true);
    addrIn.value = 'new.example.net:25566';
    addrIn.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    target.querySelector<HTMLButtonElement>('.editor .btn.blue')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('add_server', { version: '1.21.1', name: 'New', address: 'new.example.net:25566' }));
    await vi.waitFor(() => expect(target.querySelectorAll('.lrow-srv').length).toBe(3));

    target.querySelectorAll<HTMLButtonElement>('.lrow-srv .mini')[1].click();
    flushSync();
    const name2 = target.querySelector<HTMLInputElement>('.editor input')!;
    name2.value = 'Renamed';
    name2.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    target.querySelector<HTMLButtonElement>('.editor .btn.blue')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('edit_server', { version: '1.21.1', oldAddress: 'my.example.net', name: 'Renamed', newAddress: 'my.example.net' })
    );

    await vi.waitFor(() => expect(target.querySelectorAll('.lrow-srv .mini').length).toBe(3));
    target.querySelectorAll<HTMLButtonElement>('.lrow-srv .mini')[1].click();
    flushSync();
    const del = target.querySelector<HTMLButtonElement>('.editor .btn.danger')!;
    del.click();
    flushSync();
    expect(del.textContent).toBe('Нажмите ещё раз');
    expect(mocks.invoke).not.toHaveBeenCalledWith('remove_server', expect.anything());
    del.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('remove_server', { version: '1.21.1', address: 'my.example.net' }));
  });
});

describe('friends page', () => {
  it('without accounts: honest empty state, no own code and no «Добавить друга»', async () => {
    comp = mount(FriendsPage, { target });
    await tick();
    expect([...target.querySelectorAll('[role="tab"]')].map((b) => b.textContent)).toEqual(['Друзья', 'Комнаты', 'Свой сервер']);
    expect(target.textContent).toContain('Друзья появятся, когда подключим аккаунты.');
    expect(target.textContent).not.toMatch(/WEX-|Добавить друга|kotya/);
  });

  it('shows friends and requests from core with filters and answers requests', async () => {
    friendsLive.friends = [
      { id: '1', nick: 'kotya', status: 'playing', game_version: '1.20.1', lobby_code: 'ZZ99' },
      { id: '2', nick: 'Lisa', status: 'online' },
      { id: '3', nick: 'Max', status: 'offline' },
    ];
    friendsLive.requests = [{ id: 'r1', from_id: 'u9', from_nick: 'Snow' }];
    comp = mount(FriendsPage, { target });
    await tick();
    expect(target.querySelectorAll('.panel .prow').length).toBe(4);
    [...target.querySelectorAll<HTMLButtonElement>('.seg button')].find((b) => b.textContent === 'В игре')!.click();
    flushSync();
    expect([...target.querySelectorAll('.panel')[1].querySelectorAll('.prow b')].map((b) => b.textContent?.replace('1.20.1', ''))).toEqual(['kotya']);
    [...target.querySelectorAll<HTMLButtonElement>('.mini')].find((b) => b.textContent === 'Принять')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('social_accept_friend_request', { requestId: 'r1' }));
  });

  it('opens on the Rooms tab when asked (invites and join links)', async () => {
    friendsIntent.tab = 'rooms';
    comp = mount(FriendsPage, { target });
    await tick();
    expect(target.querySelector('[role="tab"][aria-selected="true"]')?.textContent).toBe('Комнаты');
    expect(friendsIntent.tab).toBe('friends');
  });
});

describe('rooms tab', () => {
  async function openRooms() {
    friendsIntent.tab = 'rooms';
    comp = mount(FriendsPage, { target });
    await vi.waitFor(() => expect(target.textContent).toContain('Мой мир'));
  }

  it('finds LAN worlds and opens one to friends; the room shows code, tunnel and players', async () => {
    await openRooms();
    expect(mocks.invoke).toHaveBeenCalledWith('net_lan_start_discovery', undefined);
    [...target.querySelectorAll<HTMLButtonElement>('.mini')].find((b) => b.textContent === 'Открыть друзьям')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('publish_lan_world', { motd: 'Мой мир', port: 51234, playerName: 'Player', version: '1.21.1' }));
    await vi.waitFor(() => expect(target.textContent).toContain('Северная база'));
    expect(target.textContent).toContain('Код AB12');
    expect(target.textContent).toContain('tun.example:25565');
    const kick = [...target.querySelectorAll<HTMLButtonElement>('.mini')].find((b) => b.textContent === 'Исключить')!;
    kick.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('kick_lobby_player', { code: 'AB12', playerName: 'Guest' }));
    [...target.querySelectorAll<HTMLButtonElement>('.btn')].find((b) => b.textContent === 'Закрыть комнату')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('close_lobby_room', { code: 'AB12' }));
  });

  it('joins by code: prepares the room with core, then starts the game on the tunnel', async () => {
    await openRooms();
    const input = target.querySelector<HTMLInputElement>('#room-code')!;
    input.value = '#ab12';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    [...target.querySelectorAll<HTMLButtonElement>('.join .btn')].find((b) => b.textContent === 'Войти')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('prepare_lobby_join', { code: 'AB12', playerName: 'Player', version: '1.21.1', repair: false }));
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ server: 'tun.example:25565' })));
  });

  it('different mods: explains and offers to fix and join', async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'prepare_lobby_join') return { ...room, is_host: false, mod_mismatch_count: 3 };
      if (cmd === 'list_lan_worlds') return [{ motd: 'Мой мир', address: '192.168.1.5:51234', port: 51234 }];
      if (cmd === 'list_open_rooms') return [];
      return null;
    });
    await openRooms();
    await rooms.join('AB12');
    flushSync();
    expect(target.textContent).toContain('Исправить и войти');
    expect(mocks.invoke).not.toHaveBeenCalledWith('start_game', expect.anything());
  });
});
