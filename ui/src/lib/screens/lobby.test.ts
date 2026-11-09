import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import LobbyScreen, { type LobbyRoom, type OnlineFriend } from './LobbyScreen.svelte';

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
      if (cmd === 'create_lobby_room') {
        expect(args.playerName).toBe('Alex');
        expect(args.worldName).toBe('Выживание в тайге');
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
        mockOnlineFriends,
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

  it('(2) guest join displays honest step-by-step progress: Сверяем моды → Докачиваем N модов → Подключаемся', async () => {
    const mockGuestRoom: LobbyRoom = {
      code: '#A7X9',
      host_name: 'Charlie',
      world_name: 'Мир Charlie',
      tunnel_address: 'charlie.playit.gg:25565',
      tunnel_active: true,
      tunnel_error: null,
      is_host: false,
      players: [
        { name: 'Charlie', is_host: true, ping_ms: 0, mods_synced: true },
        { name: 'Alex', is_host: false, ping_ms: 22, mods_synced: true },
      ],
      mod_mismatch_count: 2,
      total_mods: 142,
    };

    invokeMock.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'join_lobby_room') {
        expect(args.code).toBe('A7X9');
        await new Promise((r) => setTimeout(r, 60));
        return mockGuestRoom;
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

    const codeInput = target.querySelector('input[data-testid="guest-code-input"]') as HTMLInputElement;
    expect(codeInput).not.toBeNull();
    codeInput.value = 'A7X9';
    codeInput.dispatchEvent(new Event('input'));

    await new Promise((r) => setTimeout(r, 30));

    const joinBtn = target.querySelector('[data-testid="join-room-btn"] button') as HTMLButtonElement;
    expect(joinBtn).not.toBeNull();
    expect(joinBtn.disabled).toBe(false);

    // Trigger join
    joinBtn.click();

    // Step 1: Checking mods
    await new Promise((r) => setTimeout(r, 20));
    expect(target.querySelector('[data-testid="step-checking-mods"]')?.textContent).toContain('Сверяем моды');

    // Step 2: Downloading mods (after 60ms invoke + step switch)
    await new Promise((r) => setTimeout(r, 80));
    expect(target.querySelector('[data-testid="step-downloading-mods"]')?.textContent).toContain('Докачиваем 2');

    // Step 3 & Completion: Wait until steps complete and room connects
    await new Promise((r) => setTimeout(r, 300));

    // Active room is entered
    expect(target.querySelector('[data-testid="active-room-card"]')).not.toBeNull();
    expect(target.textContent).toContain('Мир Charlie');
    expect(target.textContent).toContain('charlie.playit.gg:25565');

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
});
