import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { FriendsLiveManager, type FriendItem, type LobbyInviteItem } from './friends-live.svelte';
import { i18n } from '../i18n';

describe('FriendsLiveManager — Real-time Status & Notifications', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    i18n.setLocale('ru');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('initializes with default intervals (25s visible, 120s hidden)', () => {
    const manager = new FriendsLiveManager();
    expect(manager.visibleIntervalMs).toBe(25000);
    expect(manager.hiddenIntervalMs).toBe(120000);
    expect(manager.immediateRefreshThresholdMs).toBe(20000);
    expect(manager.activePollIntervalMs).toBe(25000);
  });

  it('switches intervals on visibilitychange (25s when visible, 2m when hidden)', () => {
    const manager = new FriendsLiveManager();
    const isVisibleSpy = vi.spyOn(manager, 'isWindowVisible');

    // Simulate visible window
    isVisibleSpy.mockReturnValue(true);
    manager.handleVisibilityChange();
    expect(manager.activePollIntervalMs).toBe(25000);

    // Simulate minimized / hidden window
    isVisibleSpy.mockReturnValue(false);
    manager.handleVisibilityChange();
    expect(manager.activePollIntervalMs).toBe(120000);

    // Simulate returning to visible window
    isVisibleSpy.mockReturnValue(true);
    manager.handleVisibilityChange();
    expect(manager.activePollIntervalMs).toBe(25000);
  });

  it('triggers immediate refresh on return to visible when >= 20s have elapsed', async () => {
    let syncCallCount = 0;
    const manager = new FriendsLiveManager({
      syncFn: async () => {
        syncCallCount++;
        return { friends: [], incoming_requests: [], incoming_invites: [] };
      },
    });

    const isVisibleSpy = vi.spyOn(manager, 'isWindowVisible');
    isVisibleSpy.mockReturnValue(true);

    // Initial sync
    await manager.sync();
    expect(syncCallCount).toBe(1);

    // Simulate window hidden for 25s
    isVisibleSpy.mockReturnValue(false);
    manager.handleVisibilityChange();
    vi.advanceTimersByTime(25000);

    // Now window becomes visible again
    isVisibleSpy.mockReturnValue(true);
    manager.handleVisibilityChange();

    expect(syncCallCount).toBe(2);
  });

  it('does not trigger immediate refresh on return to visible if less than 20s elapsed', async () => {
    let syncCallCount = 0;
    const manager = new FriendsLiveManager({
      syncFn: async () => {
        syncCallCount++;
        return { friends: [], incoming_requests: [], incoming_invites: [] };
      },
    });

    const isVisibleSpy = vi.spyOn(manager, 'isWindowVisible');
    isVisibleSpy.mockReturnValue(true);

    await manager.sync();
    expect(syncCallCount).toBe(1);

    // Window hidden for only 5s
    isVisibleSpy.mockReturnValue(false);
    manager.handleVisibilityChange();
    vi.advanceTimersByTime(5000);

    // Window becomes visible again
    isVisibleSpy.mockReturnValue(true);
    manager.handleVisibilityChange();

    // No extra immediate sync
    expect(syncCallCount).toBe(1);
  });

  it('establishes initial baseline on first sync without spamming notifications', () => {
    const notifications: any[] = [];
    const manager = new FriendsLiveManager({
      onNotification: (n) => notifications.push(n),
    });

    const initialFriends: FriendItem[] = [
      { id: 'f-1', nick: 'Alex', status: 'playing', world_name: 'Hypixel' },
      { id: 'f-2', nick: 'Steve', status: 'online' },
    ];
    const initialInvites: LobbyInviteItem[] = [
      { id: 'inv-1', lobby_code: 'ROOM42', from_id: 'u-1', from_nick: 'Notch' },
    ];

    manager.processSyncResult({
      friends: initialFriends,
      incoming_requests: [],
      incoming_invites: initialInvites,
    });

    expect(manager.friends.length).toBe(2);
    expect(manager.incomingInvites.length).toBe(1);
    expect(notifications.length).toBe(0);
  });

  it('detects friend transitioning to playing on server and emits notification', () => {
    const notifications: any[] = [];
    const manager = new FriendsLiveManager({
      onNotification: (n) => notifications.push(n),
    });

    // 1. Initial baseline: Steve is online
    manager.processSyncResult({
      friends: [{ id: 'f-2', nick: 'Steve', status: 'online' }],
      incoming_requests: [],
      incoming_invites: [],
    });
    expect(notifications.length).toBe(0);

    // 2. Next sync: Steve enters game on Hypixel
    manager.processSyncResult({
      friends: [{ id: 'f-2', nick: 'Steve', status: 'playing', world_name: 'Hypixel' }],
      incoming_requests: [],
      incoming_invites: [],
    });

    expect(notifications.length).toBe(1);
    expect(notifications[0].type).toBe('friend_playing');
    expect(notifications[0].title).toBe('Друг зашёл в игру');
    expect(notifications[0].description).toBe('Steve играет на сервере Hypixel');
  });

  it('formats friend playing notification in English when locale is en', () => {
    i18n.setLocale('en');
    const notifications: any[] = [];
    const manager = new FriendsLiveManager({
      onNotification: (n) => notifications.push(n),
    });

    manager.processSyncResult({
      friends: [{ id: 'f-2', nick: 'Alex', status: 'offline' }],
      incoming_requests: [],
      incoming_invites: [],
    });

    manager.processSyncResult({
      friends: [{ id: 'f-2', nick: 'Alex', status: 'playing', world_name: 'Mineplex' }],
      incoming_requests: [],
      incoming_invites: [],
    });

    expect(notifications.length).toBe(1);
    expect(notifications[0].title).toBe('Friend joined game');
    expect(notifications[0].description).toBe('Alex is playing on server Mineplex');
  });

  it('detects incoming lobby invite and emits notification with Join button', () => {
    const notifications: any[] = [];
    let joinedCode = '';

    const manager = new FriendsLiveManager({
      onNotification: (n) => notifications.push(n),
      onJoinLobby: (code) => {
        joinedCode = code;
      },
    });

    // 1. Initial baseline
    manager.processSyncResult({
      friends: [],
      incoming_requests: [],
      incoming_invites: [],
    });

    // 2. New invite arrives
    manager.processSyncResult({
      friends: [],
      incoming_requests: [],
      incoming_invites: [
        {
          id: 'inv-99',
          lobby_code: 'WEX77',
          from_id: 'u-55',
          from_nick: 'Alice',
          host_name: 'Survival World',
        },
      ],
    });

    expect(notifications.length).toBe(1);
    expect(notifications[0].type).toBe('invite_received');
    expect(notifications[0].title).toBe('Вас пригласили');
    expect(notifications[0].description).toContain('Alice зовёт вас в игру');
    expect(notifications[0].action).toBeDefined();
    expect(notifications[0].action.label).toBe('Присоединиться');

    // Click join button
    notifications[0].action.onclick();
    expect(joinedCode).toBe('WEX77');
  });

  it('supports friend with lobby_code with Join action', () => {
    const notifications: any[] = [];
    let joinedCode = '';

    const manager = new FriendsLiveManager({
      onNotification: (n) => notifications.push(n),
      onJoinLobby: (code) => {
        joinedCode = code;
      },
    });

    manager.processSyncResult({
      friends: [{ id: 'f-3', nick: 'Bob', status: 'online' }],
    });

    manager.processSyncResult({
      friends: [{ id: 'f-3', nick: 'Bob', status: 'playing', lobby_code: '#B0B1' }],
    });

    expect(notifications.length).toBe(1);
    expect(notifications[0].action).toBeDefined();
    expect(notifications[0].action.label).toBe('Присоединиться');

    notifications[0].action.onclick();
    expect(joinedCode).toBe('B0B1');
  });

  it('removes requests and invites and adds friend locally', () => {
    const manager = new FriendsLiveManager();

    manager.processSyncResult({
      friends: [],
      incoming_requests: [{ id: 'req-1', from_id: 'u-1', from_nick: 'Steve' }],
      incoming_invites: [{ id: 'inv-1', lobby_code: 'CODE', from_id: 'u-2', from_nick: 'Alex' }],
    });

    expect(manager.incomingRequests.length).toBe(1);
    expect(manager.incomingInvites.length).toBe(1);

    manager.removeRequest('req-1');
    expect(manager.incomingRequests.length).toBe(0);

    manager.removeInvite('inv-1');
    expect(manager.incomingInvites.length).toBe(0);

    manager.addFriend({ id: 'u-1', nick: 'Steve', status: 'online' });
    expect(manager.friends.length).toBe(1);
    expect(manager.friends[0].nick).toBe('Steve');
  });

  it('cleans up timers and event listeners on stopPolling', () => {
    const manager = new FriendsLiveManager();
    const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener');

    manager.startPolling();
    manager.stopPolling();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('visibilitychange', expect.any(Function));
  });
});
