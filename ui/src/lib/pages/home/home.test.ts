import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { builds } from '../../shell/builds.svelte';
import { launch } from '../../shell/launch.svelte';
import { router } from '../../shell/router.svelte';
import { toasts } from '../../shell/toast.svelte';
import { i18n } from '../../i18n';
import { home } from './home.svelte';
import { friendsLive } from '../../services/friendsLive';
import HomePage from './HomePage.svelte';

let settings: Record<string, unknown>;
let installed: Record<string, boolean>;
let ping: Record<string, unknown>;
let servers: Array<{ id?: string; name: string; address: string }>;
let lan: Array<{ motd: string; address: string; port: number }>;

beforeEach(async () => {
  mocks.invoke.mockReset();
  settings = { selected_version: '1.21.1', player_name: 'Player', ram_mb: 6144 };
  installed = { '1.21.1': true, '1.20.1': false };
  ping = { success: true, latency_ms: 23, online: 3, max_players: 10 };
  servers = [
    { id: 's1', name: 'Сервер нашей компании', address: 'play.example.org' },
    { id: 's2', name: 'Мир Миши', address: '192.168.1.20:25565' },
  ];
  lan = [];
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'get_launcher_settings':
        return { ...settings };
      case 'save_launcher_settings':
        settings = { ...(args!.newSettings as Record<string, unknown>) };
        return null;
      case 'get_installed_versions':
        return [
          { id: '1.21.1', name: '1.21.1', is_installed: installed['1.21.1'] },
          { id: '1.20.1', name: '1.20.1', is_installed: installed['1.20.1'] },
        ];
      case 'list_mods':
        return new Array(args!.version === '1.21.1' ? 132 : 130).fill({});
      case 'workshop_list_worlds':
        return args!.version === '1.20.1' ? [{}, {}, {}] : [];
      case 'overview_get_news':
        return [
          { version: '0.9.2', date_text: '10 ноября', short_title: 'Что нового в 0.9.2', items: [{ text: 'Обновление без окна установщика.' }] },
          { version: '0.9.1', date_text: '1 ноября', short_title: 'Исправления', items: [] },
          { version: '0.9.0', date_text: '20 октября', short_title: 'Старое', items: [] },
        ];
      case 'list_servers':
        return servers;
      case 'list_lan_worlds':
        return lan;
      case 'ping_server_details':
        return args!.address === 'play.example.org' ? ping : { success: false };
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
  launch.prewarm = 'idle';
  home.servers = [];
  home.serverCount = 0;
  home.pings = {};
  home.lan = [];
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
});

async function render() {
  comp = mount(HomePage, { target });
  await vi.waitFor(() => expect(target.querySelector('.pane.open')).not.toBeNull());
  await vi.waitFor(() => expect(target.querySelectorAll('.news-card').length).toBe(2));
  await tick();
}

describe('home page', () => {
  it('open pane: real mod count, Java, world count and readiness from core — no invented facts', async () => {
    await render();
    const open = target.querySelector('.pane.open')!;
    expect(open.querySelector('h1')?.textContent).toBe('Wex 1.21.1');
    expect(open.querySelector('p')?.textContent).toBe('132 мода на Fabric. Миров пока нет.');
    expect(open.querySelector('.ready')?.textContent).toBe('Игра скачана, можно играть');
    // Nothing core does not report: loader version, last launch, download size.
    expect(open.textContent).not.toMatch(/0\.19|Последний запуск|МБ/);
    const closed = target.querySelector('.pane:not(.open)')!;
    expect(closed.getAttribute('role')).toBe('button');
    expect(closed.textContent).toContain('130 модов');
  });

  it('readiness says the game will download when it is not installed, and fast launch when prewarmed', async () => {
    await render();
    (target.querySelector('.pane:not(.open)') as HTMLElement).click();
    await vi.waitFor(() => expect(builds.selectedId).toBe('1.20.1'));
    flushSync();
    await vi.waitFor(() => expect(settings.selected_version).toBe('1.20.1'));
    await vi.waitFor(() => expect(toasts.current?.title).toBe('Для запуска выбрана Wex 1.20.1'));
    flushSync();
    const open = target.querySelector('.pane.open')!;
    expect(open.querySelector('p')?.textContent).toBe('130 модов на Fabric. 3 мира.');
    expect(open.querySelector('.ready')?.classList.contains('upd')).toBe(true);
    expect(open.querySelector('.ready')?.textContent).toBe('Игра ещё не скачана — нажмите «Играть», чтобы скачать');
    launch.prewarm = 'ready';
    flushSync();
    expect(target.querySelector('.pane.open .ready')?.textContent).toBe('Готово к быстрому запуску');
  });

  it('Play in the pane starts the selected build', async () => {
    await render();
    (target.querySelector('.pane.open .pane-play') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('start_game', { version: '1.21.1', nickname: 'Player', jvmArgs: null, server: null })
    );
    launch.onStatus({ state: 'cancelled', message: '' });
  });

  it('no public server block: Wex is for playing with friends', async () => {
    await render();
    expect(target.textContent).not.toContain('Сервер Aura');
    expect(target.textContent).not.toMatch(/auramc|Aura Official/);
  });

  it('Играть вместе: the player\'s own servers with live ping; Join connects to the address', async () => {
    await render();
    const panel = target.querySelector('.together')!;
    await vi.waitFor(() => expect(panel.textContent).toContain('3 из 10, пинг 23 мс'));
    const rows = [...panel.querySelectorAll('.prow')];
    expect(rows.map((r) => r.querySelector('b')?.textContent)).toEqual(['Сервер нашей компании', 'Мир Миши']);
    expect(rows[1].textContent).toContain('Не отвечает');
    // Play is the one prominent action: Join here is a secondary button.
    expect(rows[0].querySelector('button')?.classList.contains('blue')).toBe(false);
    (rows[0].querySelector('button') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ server: 'play.example.org' }))
    );
    launch.onStatus({ state: 'cancelled', message: '' });
  });

  it('Играть вместе: LAN worlds nearby, at most three saved servers with a link to all', async () => {
    servers = Array.from({ length: 5 }, (_, i) => ({ name: `S${i}`, address: `s${i}.example.org` }));
    lan = [{ motd: 'Мир Кати', address: '192.168.1.30', port: 51234 }];
    await render();
    const panel = target.querySelector('.together')!;
    await vi.waitFor(() => expect(panel.textContent).toContain('Мир Кати'));
    expect(panel.querySelectorAll('.prow').length).toBe(4);
    expect(target.textContent).toContain('Все серверы (5)');
    const lanRow = [...panel.querySelectorAll('.prow')].at(-1)!;
    (lanRow.querySelector('button') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('start_game', expect.objectContaining({ server: '192.168.1.30:51234' }))
    );
    launch.onStatus({ state: 'cancelled', message: '' });
  });

  it('Играть вместе: honest empty state with the ways to start', async () => {
    servers = [];
    await render();
    const panel = target.querySelector('.together')!;
    await vi.waitFor(() => expect(panel.textContent).toContain('Пока не с кем.'));
    expect(panel.querySelectorAll('.prow').length).toBe(0);
    expect(panel.textContent).toContain('Пока не с кем.');
    const [rooms, add] = [...panel.querySelectorAll('.together-actions button')] as HTMLButtonElement[];
    expect(add.textContent).toBe('Добавить сервер');
    rooms.click();
    expect(router.route).toBe('friends');
  });

  it('news are the two latest changelog entries', async () => {
    await render();
    const cards = [...target.querySelectorAll('.news-card')];
    expect(cards.map((c) => c.querySelector('b')?.textContent)).toEqual(['Что нового в 0.9.2', 'Исправления']);
    expect(cards[0].querySelector('time')?.textContent).toBe('10 ноября');
    expect(cards[0].textContent).toContain('Обновление без окна установщика.');
  });

  it('friends: the real sign-in state, no sample friends', async () => {
    friendsLive.connection = { state: 'signed_out', reason: 'no_account' };
    await render();
    const panel = target.querySelector('.panel:not(.together)')!;
    expect(panel.querySelectorAll('.prow').length).toBe(0);
    expect(panel.textContent).toContain('Войдите в аккаунт, чтобы видеть друзей.');
    expect(panel.textContent).not.toContain('Друзья появятся');
    home.connection = { state: 'unreachable', reason: 'server_error', server: 'lobby-api.vercel.app' };
    flushSync();
    expect(panel.textContent).toContain('Сервер друзей недоступен.');
    expect(panel.textContent).toContain('lobby-api.vercel.app сейчас не работает.');
    expect([...panel.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Повторить']);
    home.connection = { state: 'signed_in', friend_code: 'K7Q2' };
    flushSync();
    expect(panel.textContent).toContain('Друзей пока нет.');
    home.friends = [
      { id: '1', nick: 'kotya', status: 'playing', game_version: '1.20.1', lobby_code: 'AB12' },
      { id: '2', nick: 'off', status: 'offline' },
    ];
    home.friendsKnown = true;
    flushSync();
    const rows = panel.querySelectorAll('.prow');
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain('kotya');
    expect(rows[0].querySelector('.vtag')?.textContent).toBe('1.20.1');
    expect(rows[0].querySelector('.mini.blue')?.textContent).toBe('Зайти');
  });
});
