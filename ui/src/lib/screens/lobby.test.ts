import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import LobbyScreen from './LobbyScreen.svelte';
import { network } from '../design/network.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('LobbyScreen Component Tests', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    network.isOnline = true;
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
    network.isOnline = true;
  });

  // 1. Empty States
  it('renders LAN tab authentic empty state when no local worlds are found', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Локальные миры не найдены');
    expect(target.textContent).toContain('откроет одиночный мир для сети');
    unmount(comp);
  });

  it('renders Cloud tab authentic empty state when no public rooms exist', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'list_open_rooms') return Promise.resolve([]);
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 40));

    // Switch to Cloud mode
    const cloudTab = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Облачные миры')
    );
    expect(cloudTab).toBeDefined();
    cloudTab?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Нет открытых комнат');
    expect(target.textContent).toContain('Обновить');
    unmount(comp);
  });

  // 2. Loading States
  it('renders loading state while cloud rooms are being fetched', async () => {
    let finishLoading: (val: any) => void = () => {};
    const pendingPromise = new Promise((resolve) => {
      finishLoading = resolve;
    });

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'list_open_rooms') return pendingPromise;
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 40));

    // Switch to Cloud mode
    const cloudTab = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Облачные миры')
    );
    cloudTab?.click();
    await new Promise((r) => setTimeout(r, 40));

    // Verify loading spinner is present
    const spinner = target.querySelector('.cloud-status-card .loading-spinner');
    expect(spinner).not.toBeNull();

    // Finish request
    finishLoading([]);
    await new Promise((r) => setTimeout(r, 50));
    unmount(comp);
  });

  // 3. Error States
  it('renders ErrorState when cloud service returns an error', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'list_open_rooms') {
        return Promise.reject(new Error('Сервер лобби временно недоступен'));
      }
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 40));

    const cloudTab = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Облачные миры')
    );
    cloudTab?.click();
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Ошибка в лобби');
    const retryBtn = target.querySelector('.error-state-card button');
    expect(retryBtn).not.toBeNull();
    expect(retryBtn?.textContent).toContain('Повторить');

    unmount(comp);
  });

  it('renders OfflineState when network is offline', async () => {
    network.isOnline = false;
    const comp = mount(LobbyScreen, { target });
    await tick();

    expect(target.textContent).toContain('Нет подключения');
    unmount(comp);
  });

  // 4. List Populated from IPC Mock
  it('renders list of LAN worlds from list_lan_worlds IPC command', async () => {
    const mockLanWorlds = [
      { motd: 'Alex - Survival Hub', address: '192.168.1.10:54321', port: 54321 },
      { motd: 'Steve - Redstone Lab', address: '192.168.1.25:55555', port: 55555 },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve(mockLanWorlds);
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Alex - Survival Hub');
    expect(target.textContent).toContain('192.168.1.10:54321');
    expect(target.textContent).toContain('Steve - Redstone Lab');
    expect(target.textContent).toContain('192.168.1.25:55555');

    unmount(comp);
  });

  it('renders list of cloud rooms from list_open_rooms IPC command', async () => {
    const mockCloudRooms = [
      {
        id: 'ROOM-1',
        name: 'Mega SMP Season 4',
        host_name: 'Dmitry',
        game_version: '1.21.1',
        player_count: 5,
        max_players: 10,
        address: 'smp.playit.gg:25565',
        status: 'open',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'list_open_rooms') return Promise.resolve(mockCloudRooms);
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 40));

    const cloudTab = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Облачные миры')
    );
    cloudTab?.click();
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Mega SMP Season 4');
    expect(target.textContent).toContain('Организатор: Dmitry');
    expect(target.textContent).toContain('5/10');
    expect(target.textContent).toContain('Войти');

    unmount(comp);
  });

  // 5. Button Clicks Call Expected IPC Commands With Arguments
  it('calls connect_lan_world with correct address when Connect is clicked', async () => {
    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'list_lan_worlds') {
        return Promise.resolve([
          { motd: 'Alex - Creative Studio', address: '192.168.1.105:54321', port: 54321 },
        ]);
      }
      if (cmd === 'connect_lan_world') {
        return Promise.resolve(`Подключение к ${args.address} (PID 8888)`);
      }
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const connectBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Подключиться') && b.closest('.lan-worlds-section')
    );
    expect(connectBtn).toBeDefined();
    connectBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('connect_lan_world', {
      address: '192.168.1.105:54321',
    });
    expect(target.textContent).toContain('Подключение к 192.168.1.105:54321');

    unmount(comp);
  });

  it('calls publish_lan_world with motd, port, and playerName when Open to Friends is clicked', async () => {
    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'list_lan_worlds') {
        return Promise.resolve([
          { motd: 'Alex - Creative Studio', address: '192.168.1.105:54321', port: 54321 },
        ]);
      }
      if (cmd === 'publish_lan_world') {
        return Promise.resolve({
          code: '#L7K9',
          host_name: 'Player',
          world_name: args.motd,
          tunnel_address: 'tunnel.ply.gg:54321',
          tunnel_active: true,
          is_host: true,
          players: [{ name: 'Player', is_host: true, ping_ms: 0, mods_synced: true }],
          mod_mismatch_count: 0,
          total_mods: 50,
        });
      }
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const publishBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Открыть друзьям') && b.closest('.lan-worlds-section')
    );
    expect(publishBtn).toBeDefined();
    publishBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('publish_lan_world', {
      motd: 'Alex - Creative Studio',
      port: 54321,
      playerName: 'Player',
    });

    expect(target.textContent).toContain('#L7K9');
    expect(target.textContent).toContain('Туннель активен: tunnel.ply.gg:54321');

    unmount(comp);
  });

  it('calls join_lobby_room with uppercase code and playerName when Enter code is submitted', async () => {
    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'join_lobby_room') {
        return Promise.resolve({
          code: args.code,
          host_name: 'FriendHost',
          world_name: 'Friend SMP',
          tunnel_address: 'friend.ply.gg:25565',
          tunnel_active: true,
          is_host: false,
          players: [
            { name: 'FriendHost', is_host: true, ping_ms: 20, mods_synced: true },
            { name: 'Player', is_host: false, ping_ms: 25, mods_synced: true },
          ],
          mod_mismatch_count: 0,
          total_mods: 40,
        });
      }
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const codeInput = target.querySelector('.code-input') as HTMLInputElement;
    expect(codeInput).not.toBeNull();
    codeInput.value = '#b3c4';
    codeInput.dispatchEvent(new Event('input'));
    await tick();

    const joinBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Войти в комнату')
    );
    expect(joinBtn).toBeDefined();
    joinBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('join_lobby_room', {
      code: '#B3C4',
      playerName: 'Player',
    });

    expect(target.textContent).toContain('Friend SMP');
    expect(target.textContent).toContain('Покинуть');

    unmount(comp);
  });

  it('calls create_lobby_room with selected worldName and playerName when Open World for Friends is clicked', async () => {
    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'create_lobby_room') {
        return Promise.resolve({
          code: '#C8D9',
          host_name: 'Player',
          world_name: args.worldName,
          tunnel_address: 'host.ply.gg:25565',
          tunnel_active: true,
          is_host: true,
          players: [{ name: 'Player', is_host: true, ping_ms: 0, mods_synced: true }],
          mod_mismatch_count: 0,
          total_mods: 85,
        });
      }
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const openWorldBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Открыть мир для друзей') && b.closest('.host-card')
    );
    expect(openWorldBtn).toBeDefined();
    openWorldBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('create_lobby_room', {
      worldName: 'Выживание в тайге',
      playerName: 'Player',
    });

    expect(target.textContent).toContain('#C8D9');
    expect(target.textContent).toContain('Закрыть комнату');

    unmount(comp);
  });

  it('calls close_lobby_room when host closes the active room session', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_lan_worlds') return Promise.resolve([]);
      if (cmd === 'create_lobby_room') {
        return Promise.resolve({
          code: '#X1Y2',
          host_name: 'Player',
          world_name: 'Мой мир',
          tunnel_address: 'host.ply.gg:25565',
          tunnel_active: true,
          is_host: true,
          players: [{ name: 'Player', is_host: true, ping_ms: 0, mods_synced: true }],
          mod_mismatch_count: 0,
          total_mods: 10,
        });
      }
      if (cmd === 'close_lobby_room') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(LobbyScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const openWorldBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Открыть мир для друзей') && b.closest('.host-card')
    );
    expect(openWorldBtn).toBeDefined();
    openWorldBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    const closeBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Закрыть комнату')
    );
    expect(closeBtn).toBeDefined();
    closeBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('close_lobby_room', {
      code: '#X1Y2',
    });

    // Returned to initial state
    expect(target.textContent).not.toContain('Закрыть комнату');

    unmount(comp);
  });
});
