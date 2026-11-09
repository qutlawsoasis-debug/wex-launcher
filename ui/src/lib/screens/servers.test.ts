import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import ServersScreen, { diagnosePingError, type ServerEntry } from './ServersScreen.svelte';
import { network } from '../design/network.svelte';

describe('ServersScreen: Error Diagnosis (diagnosePingError)', () => {
  it('diagnoses no internet when system is offline or error mentions network unreachable', () => {
    // When network is reported offline
    const diagOfflineSystem = diagnosePingError('connection error', false);
    expect(diagOfflineSystem.reason).toBe('no_internet');
    expect(diagOfflineSystem.message).toBe('Отсутствует подключение к интернету');

    // When error string indicates unreachable network
    const diagNetUnreach = diagnosePingError('Network unreachable (os error 10051)', true);
    expect(diagNetUnreach.reason).toBe('no_internet');
    expect(diagNetUnreach.message).toBe('Отсутствует подключение к интернету');

    const diagEnet = diagnosePingError('ENETUNREACH: socket error', true);
    expect(diagEnet.reason).toBe('no_internet');
  });

  it('diagnoses invalid address when DNS lookup fails or host is not found', () => {
    const diagDns = diagnosePingError('failed to lookup address information: Name or service not known', true);
    expect(diagDns.reason).toBe('invalid_address');
    expect(diagDns.message).toBe('Неверный адрес сервера');

    const diagNoSuchHost = diagnosePingError('No such host is known. (os error 11001)', true);
    expect(diagNoSuchHost.reason).toBe('invalid_address');
    expect(diagNoSuchHost.message).toBe('Неверный адрес сервера');

    const diagInvalid = diagnosePingError('invalid_address: invalid ip format', true);
    expect(diagInvalid.reason).toBe('invalid_address');
  });

  it('diagnoses offline when server refused connection, timed out, or is shut down', () => {
    const diagRefused = diagnosePingError('Connection refused (os error 10061)', true);
    expect(diagRefused.reason).toBe('offline');
    expect(diagRefused.message).toBe('Сервер выключен или недоступен');

    const diagTimeout = diagnosePingError('Connection timed out (os error 10060)', true);
    expect(diagTimeout.reason).toBe('offline');
    expect(diagTimeout.message).toBe('Сервер выключен или недоступен');

    const diagGeneral = diagnosePingError('Generic socket failure', true);
    expect(diagGeneral.reason).toBe('offline');
    expect(diagGeneral.message).toBe('Сервер выключен или недоступен');
  });
});

describe('ServersScreen: Component Rendering & Features', () => {
  let target: HTMLDivElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    network.isOnline = true;
  });

  afterEach(() => {
    target.remove();
    vi.restoreAllMocks();
  });

  it('renders toolbar with version segmented control, refresh, import, and add server buttons', () => {
    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1', playerName: 'Alex' },
    });

    expect(target.textContent).toContain('1.21.1');
    expect(target.textContent).toContain('1.20.1');
    expect(target.textContent).toContain('Обновить');
    expect(target.textContent).toContain('Импорт');
    expect(target.textContent).toContain('Добавить сервер');

    unmount(comp);
  });

  it('pins friend hosted server at the very top of the list when friend is in game', async () => {
    const friendServer: ServerEntry = {
      id: 'friend-srv-u_bob',
      name: 'Мир: Bob',
      address: '127.0.0.1:25565',
      motd: 'Хост: Bob • Bob Survival SMP',
      version: '1.21.1',
      isFriendServer: true,
      friendNick: 'Bob',
      worldName: 'Bob Survival SMP',
    };

    const comp = mount(ServersScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
        mockFriendServers: [friendServer],
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('Сервер друга');
    expect(target.textContent).toContain('Мир: Bob');
    expect(target.textContent).toContain('Bob Survival SMP');

    // Pinned badge should be visible
    const pinnedBadge = target.querySelector('.pinned-badge');
    expect(pinnedBadge).not.toBeNull();
    expect(pinnedBadge?.textContent).toContain('Сервер друга');

    unmount(comp);
  });

  it('launches server with quickPlay when clicking Play button', async () => {
    let launchedServer: any = null;
    const friendServer: ServerEntry = {
      id: 'friend-srv-u_charlie',
      name: 'Мир: Charlie',
      address: '127.0.0.1:25565',
      motd: 'Хост: Charlie • Hardcore',
      isFriendServer: true,
      friendNick: 'Charlie',
      worldName: 'Hardcore',
    };

    const comp = mount(ServersScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
        mockFriendServers: [friendServer],
        onlaunch: (srv) => {
          launchedServer = srv;
        },
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    const playBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Играть')
    );
    expect(playBtn).toBeDefined();

    playBtn?.click();
    await new Promise((r) => setTimeout(r, 20));

    expect(launchedServer).not.toBeNull();
    expect(launchedServer?.friendNick).toBe('Charlie');
    expect(launchedServer?.address).toBe('127.0.0.1:25565');

    unmount(comp);
  });

  it('displays accurate offline reason badge without fabricating online numbers and shows Retry button', async () => {
    const offlineServer: ServerEntry = {
      id: 'srv-offline-test',
      name: 'Unreachable Server',
      address: '127.0.0.1:49999',
      pingError: true,
      errorReason: 'offline',
      errorMessage: 'Сервер выключен или недоступен',
      ping_ms: null,
      online: null,
      max_players: null,
    };

    const comp = mount(ServersScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
        mockFriendServers: [offlineServer],
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    // Verify row rendered with exact error message
    expect(target.textContent).toContain('Unreachable Server');
    expect(target.textContent).toContain('Сервер выключен или недоступен');
    expect(target.textContent).toContain('Выключен');
    expect(target.textContent).toContain('Повторить');

    // No fabricated player counts
    const onlineCounter = target.querySelector('.online-counter');
    expect(onlineCounter).toBeNull();

    unmount(comp);
  });
});
