import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ServersScreen, { diagnosePingError, type ServerEntry } from './ServersScreen.svelte';
import { network } from '../design/network.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

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

describe('ServersScreen: Component Tests (IPC Integration)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  // 1. Empty State
  it('renders authentic empty state when no servers exist in servers.dat', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_servers') return Promise.resolve([]);
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Серверы не найдены');
    expect(target.textContent).toContain('Список серверов пуст');

    unmount(comp);
  });

  // 2. Server List Populated from IPC Mock
  it('renders list of servers with name, address, and ping from list_servers and ping_server', async () => {
    const mockServers = [
      {
        id: 'srv-1',
        name: 'Hypixel Network',
        address: 'mc.hypixel.net',
        motd: 'Hypixel MMO and Minigames',
        version: '1.21.1',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Steve' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'ping_server') {
        return Promise.resolve({
          latency_ms: 45,
          online: 35000,
          max_players: 50000,
          motd: 'Hypixel MMO and Minigames',
          version: '1.21.1',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1', playerName: 'Steve' },
    });
    await new Promise((r) => setTimeout(r, 80));

    expect(target.textContent).toContain('Hypixel Network');
    expect(target.textContent).toContain('mc.hypixel.net');
    expect(target.textContent).toContain('45 мс');
    expect(target.textContent).toContain('35000/50000');
    expect(target.textContent).toContain('Играть');

    unmount(comp);
  });

  // 3. Ping Error State
  it('renders offline pill indicator when ping_server fails', async () => {
    const mockServers = [
      {
        id: 'srv-offline',
        name: 'Dead Server',
        address: 'offline.invalid.lan',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'ping_server') {
        return Promise.reject(new Error('Connection timed out'));
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 80));

    expect(target.textContent).toContain('Dead Server');
    const offlinePill = target.querySelector('.ping-pill.offline');
    expect(offlinePill).not.toBeNull();
    expect(offlinePill?.textContent).toMatch(/Офлайн|Выключен/);

    unmount(comp);
  });

  // 4. Add Server Action Calling add_server IPC
  it('opens Add Server sheet and calls add_server with name, address, and active version', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve([]);
      if (cmd === 'add_server') {
        return Promise.resolve({
          id: 'srv-new',
          name: 'Survival Server',
          address: 'mc.example.com',
        });
      }
      if (cmd === 'ping_server') {
        return Promise.resolve({
          latency_ms: 25,
          online: 10,
          max_players: 50,
          motd: 'Welcome',
          version: '1.21.1',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 50));

    // Click «Добавить сервер» toolbar button
    const addBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('.toolbar-controls button')).find((b) =>
      b.textContent?.includes('Добавить сервер')
    );
    expect(addBtn).toBeDefined();
    addBtn?.click();
    await new Promise((r) => setTimeout(r, 40));

    // Fill sheet inputs
    const nameInput = target.querySelector('#new-srv-name') as HTMLInputElement;
    const addrInput = target.querySelector('#new-srv-addr') as HTMLInputElement;
    expect(nameInput).not.toBeNull();
    expect(addrInput).not.toBeNull();

    nameInput.value = 'Survival Server';
    nameInput.dispatchEvent(new Event('input', { bubbles: true }));
    addrInput.value = 'mc.example.com';
    addrInput.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();

    // Click Save in sheet
    const saveBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Сохранить')
    );
    expect(saveBtn).toBeDefined();
    saveBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('add_server', {
      version: '1.21.1',
      name: 'Survival Server',
      address: 'mc.example.com',
    });

    unmount(comp);
  });

  // 5. Remove Server Action Calling remove_server IPC
  it('calls remove_server with address and version when delete button is clicked', async () => {
    const mockServers = [
      {
        id: 'srv-to-del',
        name: 'Server To Delete',
        address: 'del.example.com',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'remove_server') return Promise.resolve();
      if (cmd === 'ping_server') return Promise.resolve({ latency_ms: 10, online: 1, max_players: 10 });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const removeBtn = target.querySelector('.remove-btn') as HTMLButtonElement;
    expect(removeBtn).not.toBeNull();
    removeBtn.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('remove_server', {
      version: '1.21.1',
      address: 'del.example.com',
    });

    unmount(comp);
  });

  // 6. Connect / Play Server Calling launch_game IPC
  it('calls launch_game with server address, version, and player name when Play button is clicked', async () => {
    const mockServers = [
      {
        id: 'srv-play',
        name: 'Play Server',
        address: 'play.wex.lan:25565',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Alex' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'launch_game') return Promise.resolve({ success: true, pid: 1234 });
      if (cmd === 'ping_server') return Promise.resolve({ latency_ms: 15, online: 2, max_players: 20 });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1', playerName: 'Alex' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const playBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('.server-metrics button')).find((b) =>
      b.textContent?.includes('Играть')
    );
    expect(playBtn).toBeDefined();
    playBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('launch_game', {
      version: '1.21.1',
      nickname: 'Alex',
      server: 'play.wex.lan:25565',
    });

    unmount(comp);
  });
});
