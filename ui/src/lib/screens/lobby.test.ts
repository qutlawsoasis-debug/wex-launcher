import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import { network } from '../design/network.svelte';
import LobbyScreen, { type LobbyRoom, type OnlineFriend } from './LobbyScreen.svelte';

const eventHandlers = vi.hoisted(() => new Map<string, (event: any) => void>());
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (name: string, callback: any) => {
  eventHandlers.set(name, callback); return () => eventHandlers.delete(name);
}) }));
// Mock Tauri IPC invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('LobbyScreen - Friends Lobby & P2P Multiplayer (wex-lobby-polish)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    // Mock navigator.clipboard
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  it('(1) creates room in one click from selected world with default name, displays large room code and copy button', async () => {
    const mockCreatedRoom: LobbyRoom = {
      code: '#A7X9',
      host_name: 'Alex',
      world_name: 'Выживание в тайге',
      tunnel_address: 'tunnel.playit.gg:25565',
      tunnel_active: true,
      tunnel_error: null,
      is_host: true,
      players: [
        {
          name: 'Alex',
          is_host: true,
          ping_ms: 0,
          mods_synced: true,
        },
      ],
      mod_mismatch_count: 0,
      total_mods: 142,
    };

    invokeMock.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'list_lan_worlds') return [{ address: '127.0.0.1:54321', motd: 'Выживание в тайге', clean_motd: 'Выживание в тайге', port: 54321 }];
      if (cmd === 'publish_lan_world') {
        expect(args.playerName).toBe('Alex');
        expect(args.motd).toBe('Выживание в тайге');
        expect(args.port).toBe(54321);
        return mockCreatedRoom;
      }
      if (cmd === 'social_sync') {
        return { friends: [] };
      }
      return null;
    });

    const comp = mount(LobbyScreen, {
      target,
      props: {
        playerName: 'Alex',
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    // Verify 1-click button «Создать комнату» is present
    const createBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Создать комнату')
    );
    expect(createBtn).toBeDefined();

    // Click «Создать комнату»
    createBtn?.click();
    await new Promise((r) => setTimeout(r, 50));

    // Verify active room rendered
    const activeRoomCard = target.querySelector('[data-testid="active-room-card"]');
    expect(activeRoomCard).not.toBeNull();

    // Verify room code is rendered prominently (large font)
    const codeDisplay = target.querySelector('[data-testid="room-code-display"]');
    expect(codeDisplay?.textContent).toContain('#A7X9');

    // Verify Copy button works
    const copyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Скопировать')
    );
    expect(copyBtn).toBeDefined();

    copyBtn?.click();
    await new Promise((r) => setTimeout(r, 20));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('#A7X9');

    unmount(comp);
  });

  it('(1b) opens online friends list and sends invite via social_send_lobby_invite', async () => {
    const mockOnlineFriends: OnlineFriend[] = [
      { id: 'u_charlie', nick: 'Charlie', status: 'online' },
      { id: 'u_diana', nick: 'Diana', status: 'playing', world_name: 'Hardcore' },
    ];

    const hostRoom: LobbyRoom = {
      code: '#A7X9',
      host_name: 'Alex',
      world_name: 'Мир Alex',
      tunnel_address: 'tunnel.playit.gg:25565',
      tunnel_active: true,
      tunnel_error: null,
      is_host: true,
      players: [
        {
          name: 'Alex',
          is_host: true,
          ping_ms: 0,
          mods_synced: true,
        },
      ],
      mod_mismatch_count: 0,
      total_mods: 142,
    };

    let sentInvitePayload: any = null;

    invokeMock.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'social_sync') {
        return {
          friends: [
            { id: 'u_charlie', nick: 'Charlie', status: 'online' },
            { id: 'u_diana', nick: 'Diana', status: 'playing', world_name: 'Hardcore' },
          ],
        };
      }
      if (cmd === 'social_send_lobby_invite') {
        sentInvitePayload = args;
        return null;
      }
      return null;
    });

    const comp = mount(LobbyScreen, {
      target,
      props: {
        playerName: 'Alex',
        initialRoom: hostRoom,
      },
    });

    await new Promise((r) => setTimeout(r, 30));

    // Click «Пригласить друга»
    const inviteBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Пригласить друга')
    );
    expect(inviteBtn).toBeDefined();
    inviteBtn?.click();
    await new Promise((r) => setTimeout(r, 30));

    // Verify online friends popover rendered
    const popover = target.querySelector('[data-testid="invite-friends-popover"]');
    expect(popover).not.toBeNull();
    expect(target.textContent).toContain('Charlie');
    expect(target.textContent).toContain('Diana');

    // Click «Пригласить» for Charlie
    const charlieInviteBtn = target.querySelector('[data-testid="send-invite-btn-u_charlie"] button') as HTMLButtonElement;
    expect(charlieInviteBtn).not.toBeNull();

    charlieInviteBtn?.click();
    await new Promise((r) => setTimeout(r, 30));

    // Verify social_send_lobby_invite called with room code and friend ID
    expect(sentInvitePayload).not.toBeNull();
    expect(sentInvitePayload.lobbyCode).toBe('#A7X9');
    expect(sentInvitePayload.targetUserId).toBe('u_charlie');

    // Verify invited badge displayed for Charlie
    const invitedBadge = target.querySelector('[data-testid="invited-badge-u_charlie"]');
    expect(invitedBadge?.textContent).toBe('Приглашён');

    unmount(comp);
  });

  it('(2) join stages only advance on actual backend events and launch success', async () => {
    let finishPreparation!: (room: LobbyRoom) => void;
    let finishLaunch!: () => void;
    const room: LobbyRoom = { code: '#A7X9', host_name: 'Host', world_name: 'World', is_host: false,
      tunnel_active: true, tunnel_address: 'test.ply.gg:25565', players: [], mod_mismatch_count: 0, total_mods: 2 };
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'prepare_lobby_join') return new Promise(r => { finishPreparation = r; });
      if (cmd === 'launch_game') {
        eventHandlers.get('launch-status')?.({ payload: { state: 'preparing', message: 'Подключаемся к миру' } });
        return new Promise<void>(r => { finishLaunch = r; });
      }
      return Promise.resolve(null);
    });
    const comp = mount(LobbyScreen, { target, props: { playerName: 'Guest' } });
    await tick();
    const input = target.querySelector('[data-testid="guest-code-input"]') as HTMLInputElement;
    input.value = 'A7X9'; input.dispatchEvent(new Event('input')); await tick();
    const joining = comp.handleJoinRoom();
    await vi.waitFor(() => expect(finishPreparation).toBeTypeOf('function'));
    expect(target.querySelector('[data-testid="step-downloading-mods"]')).toBeNull();
    eventHandlers.get('lobby-join-progress')?.({ payload: { code: 'A7X9', stage: 'checking_mods', message: 'Сверяем моды' } });
    await tick();
    expect(target.querySelector('[data-testid="step-checking-mods"]')?.textContent).toBe('Сверяем моды');
    eventHandlers.get('lobby-join-progress')?.({ payload: { code: 'A7X9', stage: 'downloading_mods', message: 'Подготовка файлов' } });
    eventHandlers.get('install-progress')?.({ payload: { percentage: 35, stage: 'Pack', detail: 'Проверено 7 из 20' } });
    await tick();
    expect(target.querySelector('[data-testid="step-downloading-mods"]')?.textContent).toBe('Проверено 7 из 20');
    expect(target.querySelector('[data-testid="active-room-card"]')).toBeNull();
    finishPreparation(room);
    await vi.waitFor(() => expect(finishLaunch).toBeTypeOf('function'));
    await tick();
    expect(target.querySelector('[data-testid="step-connecting"]')?.textContent).toBe('Подключаемся к миру');
    expect(target.querySelector('[data-testid="active-room-card"]')).toBeNull();
    finishLaunch(); await joining; await tick();
    expect(target.querySelector('[data-testid="active-room-card"]')).not.toBeNull();
    expect(invokeMock).toHaveBeenCalledWith('launch_game', { version: '1.21.1', nickname: 'Guest', server: 'test.ply.gg:25565' });
    for (const name of ['lobby-join-progress', 'install-progress', 'launch-status']) expect(eventHandlers.has(name)).toBe(false);
    unmount(comp);
  });

  it('(3) displays plain diagnostic reason and Retry button when Playit tunnel fails to raise, and recovers on retry', async () => {
    const mockTunnelDownRoom: LobbyRoom = {
      code: '#FAIL1',
      host_name: 'Alex',
      world_name: 'Мир Alex',
      tunnel_address: null,
      tunnel_active: false,
      tunnel_error: 'Не удалось поднять Playit-туннель: служба туннелей временно недоступна',
      is_host: true,
      players: [
        { name: 'Alex', is_host: true, ping_ms: 0, mods_synced: true },
      ],
      mod_mismatch_count: 0,
      total_mods: 142,
    };

    let retryCalled = false;
    invokeMock.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'retry_lobby_tunnel') {
        retryCalled = true;
        return {
          ...mockTunnelDownRoom,
          tunnel_address: 'tunnel.playit.gg:25565',
          tunnel_active: true,
          tunnel_error: null,
        };
      }
      return null;
    });

    const comp = mount(LobbyScreen, {
      target,
      props: {
        playerName: 'Alex',
        initialRoom: mockTunnelDownRoom,
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    // Verify tunnel error card is visible
    const errorCard = target.querySelector('[data-testid="tunnel-error-card"]');
    expect(errorCard).not.toBeNull();

    // Verify plain diagnostic reason is displayed
    const reasonText = target.querySelector('[data-testid="tunnel-error-reason"]');
    expect(reasonText?.textContent).toBe('Не удалось поднять Playit-туннель: служба туннелей временно недоступна');

    // Verify Retry button is present
    const retryBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Повторить')
    );
    expect(retryBtn).toBeDefined();

    // Click «Повторить»
    retryBtn?.click();
    await new Promise((r) => setTimeout(r, 30));

    expect(retryCalled).toBe(true);

    // Verify tunnel recovered and active pill is now visible
    expect(target.querySelector('[data-testid="tunnel-error-card"]')).toBeNull();
    const activePill = target.querySelector('[data-testid="tunnel-active-pill"]');
    expect(activePill?.textContent).toContain('Туннель активен: tunnel.playit.gg:25565');

    unmount(comp);
  });

  it('(4) host closing room evicts guest with alert message «Комната закрыта организатором»', async () => {
    const guestRoom: LobbyRoom = {
      code: '#HOST1',
      host_name: 'Charlie',
      world_name: 'Мир Charlie',
      tunnel_address: 'charlie.playit.gg:25565',
      tunnel_active: true,
      tunnel_error: null,
      is_host: false,
      players: [
        { name: 'Charlie', is_host: true, ping_ms: 0, mods_synced: true },
        { name: 'Alex', is_host: false, ping_ms: 18, mods_synced: true },
      ],
      mod_mismatch_count: 0,
      total_mods: 142,
    };

    let statusQueried = false;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_lobby_status') {
        statusQueried = true;
        // Host has closed the room -> API returns 404 or closed error
        throw 'Комната закрыта организатором';
      }
      return null;
    });

    const comp = mount(LobbyScreen, {
      target,
      props: {
        playerName: 'Alex',
        initialRoom: guestRoom,
      },
    });

    await new Promise((r) => setTimeout(r, 30));
    expect(target.querySelector('[data-testid="active-room-card"]')).not.toBeNull();

    // Trigger polling check
    await new Promise((r) => setTimeout(r, 2600));

    // Verify guest was evicted from room session
    expect(target.querySelector('[data-testid="active-room-card"]')).toBeNull();

    // Verify alert message is clearly displayed
    const alert = target.querySelector('[data-testid="room-closed-alert"]');
    expect(alert).not.toBeNull();
    expect(alert?.textContent).toContain('Комната закрыта организатором');

    unmount(comp);
  });

  it('(4b) host closes room cleanly and returns to lobby main view', async () => {
    const hostRoom: LobbyRoom = {
      code: '#ROOM1',
      host_name: 'Alex',
      world_name: 'Мир Alex',
      tunnel_address: 'tunnel.playit.gg:25565',
      tunnel_active: true,
      tunnel_error: null,
      is_host: true,
      players: [{ name: 'Alex', is_host: true, ping_ms: 0, mods_synced: true }],
      mod_mismatch_count: 0,
      total_mods: 142,
    };

    let closedCode: string | null = null;
    invokeMock.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'close_lobby_room') {
        closedCode = args.code;
        return null;
      }
      return null;
    });

    const comp = mount(LobbyScreen, {
      target,
      props: {
        playerName: 'Alex',
        initialRoom: hostRoom,
      },
    });

    await new Promise((r) => setTimeout(r, 20));

    const closeBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Закрыть комнату')
    );
    expect(closeBtn).toBeDefined();

    closeBtn?.click();
    await new Promise((r) => setTimeout(r, 30));

    expect(closedCode).toBe('#ROOM1');
    expect(target.querySelector('[data-testid="active-room-card"]')).toBeNull();

    unmount(comp);
  });
  it('review_08_never_fakes_mod_download_or_connects_a_mismatched_room', async () => {
    invokeMock.mockImplementation(async (cmd: string) => cmd === 'prepare_lobby_join' ? ({
      code: '#ABC123', host_name: 'Host', world_name: 'World', is_host: false,
      tunnel_active: true, tunnel_address: '127.0.0.1:25565', players: [],
      mod_mismatch_count: 2, total_mods: 4,
    }) : null);
    const comp = mount(LobbyScreen, { target, props: { playerName: 'Guest' } });
    await tick();
    const input = target.querySelector('[data-testid="guest-code-input"]') as HTMLInputElement;
    input.value = 'ABC123'; input.dispatchEvent(new Event('input')); await tick();
    await comp.handleJoinRoom(); await tick();
    expect(target.querySelector('[data-testid="active-room-card"]')).toBeNull();
    expect(invokeMock).not.toHaveBeenCalledWith('launch_game', expect.anything());
    expect(target.textContent).not.toContain('Все моды совпадают');
    unmount(comp);
  });

});


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

    expect(target.textContent).toMatch(/Ошибка в (комнате|лобби)/);
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
      version: '1.21.1', nickname: 'Player',
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

  });
