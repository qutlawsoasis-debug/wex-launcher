import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  calculatePollingInterval,
  shouldSendWindowsNotification,
  checkFriendNotificationCooldown,
  parseWexJoinUri,
  FriendsLiveService,
  INTERVAL_VISIBLE,
  INTERVAL_HIDDEN,
  INTERVAL_IN_GAME,
  FRIEND_NOTIFICATION_COOLDOWN_MS,
  type SyncResponse,
  type FriendItem,
  type LobbyInviteItem,
  type FriendsLiveNotification,
} from './friendsLive';

describe('friendsLive: Interval calculation & Deep Links', () => {
  it('calculates 20–30s interval (25,000ms) when window is visible and game is idle', () => {
    const interval = calculatePollingInterval({
      isVisible: true,
      isGameRunning: false,
    });
    expect(interval).toBe(INTERVAL_VISIBLE);
    expect(interval).toBeGreaterThanOrEqual(20_000);
    expect(interval).toBeLessThanOrEqual(30_000);
  });

  it('calculates 2 minute interval (120,000ms) when window is hidden/minimized', () => {
    const interval = calculatePollingInterval({
      isVisible: false,
      isGameRunning: false,
    });
    expect(interval).toBe(INTERVAL_HIDDEN);
    expect(interval).toBe(120_000);
  });

  it('calculates 2 minute interval (120,000ms) while Minecraft game is active', () => {
    const intervalVisible = calculatePollingInterval({
      isVisible: true,
      isGameRunning: true,
    });
    expect(intervalVisible).toBe(INTERVAL_IN_GAME);
    expect(intervalVisible).toBe(120_000);

    const intervalHidden = calculatePollingInterval({
      isVisible: false,
      isGameRunning: true,
    });
    expect(intervalHidden).toBe(INTERVAL_IN_GAME);
  });

  it('parses wex://join deep links in various standard formats', () => {
    expect(parseWexJoinUri('wex://join?code=ROOM123')).toBe('ROOM123');
    expect(parseWexJoinUri('wex://join/A7X9')).toBe('A7X9');
    expect(parseWexJoinUri('#wex://join?code=SURVIVAL_42')).toBe('SURVIVAL_42');
    expect(parseWexJoinUri('#join=ALPHA')).toBe('ALPHA');
    expect(parseWexJoinUri('https://example.com')).toBeNull();
    expect(parseWexJoinUri('')).toBeNull();
  });
});

describe('friendsLive: Notification Routing Logic (shouldSendWindowsNotification)', () => {
  it('routes to in-app Toast when window is focused, visible, not in tray, and game idle', () => {
    const useWindows = shouldSendWindowsNotification({
      isFocused: true,
      isVisible: true,
      isGameRunning: false,
      isInTray: false,
    });
    expect(useWindows).toBe(false);
  });

  it('routes to Windows Notification when window is blurred (not focused)', () => {
    const useWindows = shouldSendWindowsNotification({
      isFocused: false,
      isVisible: true,
      isGameRunning: false,
      isInTray: false,
    });
    expect(useWindows).toBe(true);
  });

  it('routes to Windows Notification when window is hidden or minimized', () => {
    const useWindows = shouldSendWindowsNotification({
      isFocused: false,
      isVisible: false,
      isGameRunning: false,
      isInTray: false,
    });
    expect(useWindows).toBe(true);
  });

  it('routes to Windows Notification when launcher is minimized to tray', () => {
    const useWindows = shouldSendWindowsNotification({
      isFocused: false,
      isVisible: false,
      isGameRunning: false,
      isInTray: true,
    });
    expect(useWindows).toBe(true);
  });

  it('routes to Windows Notification during gameplay regardless of window state', () => {
    const useWindowsDuringGame = shouldSendWindowsNotification({
      isFocused: true,
      isVisible: true,
      isGameRunning: true,
      isInTray: false,
    });
    expect(useWindowsDuringGame).toBe(true);
  });
});

describe('friendsLive: Anti-Spam Cooldown Utility (checkFriendNotificationCooldown)', () => {
  it('allows notification if friend was never notified before', () => {
    expect(checkFriendNotificationCooldown(undefined, 1000)).toBe(true);
  });

  it('suppresses notification if less than 5 minutes (300,000ms) elapsed', () => {
    const lastSent = 1_000_000;
    const nowWithinCooldown = lastSent + 299_999;
    expect(checkFriendNotificationCooldown(lastSent, nowWithinCooldown)).toBe(false);
  });

  it('allows notification after 5 minutes (300,000ms) or more elapsed', () => {
    const lastSent = 1_000_000;
    const nowExactCooldown = lastSent + 300_000;
    const nowPastCooldown = lastSent + 300_001;
    expect(checkFriendNotificationCooldown(lastSent, nowExactCooldown)).toBe(true);
    expect(checkFriendNotificationCooldown(lastSent, nowPastCooldown)).toBe(true);
  });
});

describe('friendsLive: Polling lifecycle, Intervals, and Pause/Resume', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('starts with visible interval and triggers polling on schedule', async () => {
    let pollCount = 0;
    const mockSync = vi.fn().mockImplementation(async () => {
      pollCount++;
      return {
        friends: [],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.start();

    expect(service.running).toBe(true);
    expect(service.paused).toBe(false);
    expect(service.activeInterval).toBe(INTERVAL_VISIBLE);
    expect(pollCount).toBe(1); // Immediate first poll

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(3);

    service.stop();
    expect(service.running).toBe(false);
  });

  it('switches interval to 2 minutes when window is hidden and back to 25s when visible', async () => {
    let pollCount = 0;
    const mockSync = vi.fn().mockImplementation(async () => {
      pollCount++;
      return {
        friends: [],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.start();
    expect(pollCount).toBe(1);

    service.setVisibility(false);
    expect(service.visible).toBe(false);
    expect(service.activeInterval).toBe(INTERVAL_HIDDEN);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(1);

    await vi.advanceTimersByTimeAsync(INTERVAL_HIDDEN - INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    service.setVisibility(true);
    expect(service.visible).toBe(true);
    expect(service.activeInterval).toBe(INTERVAL_VISIBLE);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(3);

    service.stop();
  });

  it('switches interval to 2 minutes during active gameplay', async () => {
    let pollCount = 0;
    const mockSync = vi.fn().mockImplementation(async () => {
      pollCount++;
      return {
        friends: [],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.start();
    expect(pollCount).toBe(1);

    service.setGameRunning(true);
    expect(service.inGame).toBe(true);
    expect(service.activeInterval).toBe(INTERVAL_IN_GAME);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(1);

    await vi.advanceTimersByTimeAsync(INTERVAL_IN_GAME - INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    service.setGameRunning(false);
    expect(service.inGame).toBe(false);
    expect(service.activeInterval).toBe(INTERVAL_VISIBLE);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(3);

    service.stop();
  });

  it('pauses and resumes polling cleanly', async () => {
    let pollCount = 0;
    const mockSync = vi.fn().mockImplementation(async () => {
      pollCount++;
      return {
        friends: [],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.start();
    expect(pollCount).toBe(1);

    service.pause();
    expect(service.paused).toBe(true);
    expect(service.running).toBe(false);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE * 4);
    expect(pollCount).toBe(1);

    const res = await service.pollNow();
    expect(res).toBeNull();
    expect(pollCount).toBe(1);

    service.resume();
    expect(service.paused).toBe(false);
    expect(service.running).toBe(true);
    expect(pollCount).toBe(2);

    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(3);

    service.stop();
  });
});

describe('friendsLive: Real-time Notifications & Live Status Updates', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('detects when a friend enters a game and emits notification with server details', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      if (round === 1) {
        return {
          friends: [
            {
              id: 'u1',
              nick: 'Alice',
              status: 'online',
              game_version: '1.20.1',
              world_name: null,
            },
          ],
          incoming_requests: [],
          incoming_invites: [],
        } satisfies SyncResponse;
      } else {
        return {
          friends: [
            {
              id: 'u1',
              nick: 'Alice',
              status: 'playing',
              game_version: '1.20.1',
              world_name: 'Survival SMP',
            },
          ],
          incoming_requests: [],
          incoming_invites: [],
        } satisfies SyncResponse;
      }
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    const notifications: FriendsLiveNotification[] = [];
    service.onNotification((n) => notifications.push(n));

    // First round records baseline
    await service.pollNow();
    expect(notifications.length).toBe(0);
    expect(service.friends[0].status).toBe('online');

    // Second round: Alice enters game on 'Survival SMP'
    round = 2;
    await service.pollNow();

    expect(notifications.length).toBe(1);
    expect(notifications[0].type).toBe('friend_playing');
    expect(notifications[0].title).toBe('Alice зашёл в игру');
    expect(notifications[0].description).toContain('Alice играет на сервере «Survival SMP»');
    expect(service.friends[0].status).toBe('playing');
  });

  it('detects incoming lobby invitation and generates notification with Join action', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      if (round === 1) {
        return {
          friends: [],
          incoming_requests: [],
          incoming_invites: [],
        } satisfies SyncResponse;
      } else {
        return {
          friends: [],
          incoming_requests: [],
          incoming_invites: [
            {
              id: 'inv_101',
              lobby_code: 'ROOM_99',
              from_id: 'u2',
              from_nick: 'Bob',
            },
          ],
        } satisfies SyncResponse;
      }
    });

    const joinedCodes: string[] = [];
    const mockJoin = vi.fn().mockImplementation(async (code: string) => {
      joinedCodes.push(code);
    });

    const service = new FriendsLiveService({
      syncFn: mockSync,
      joinFn: mockJoin,
    });

    const notifications: FriendsLiveNotification[] = [];
    service.onNotification((n) => notifications.push(n));

    await service.pollNow();
    expect(notifications.length).toBe(0);

    // Second round brings invite
    round = 2;
    await service.pollNow();

    expect(notifications.length).toBe(1);
    const inviteNotif = notifications[0];
    expect(inviteNotif.type).toBe('invite');
    expect(inviteNotif.title).toBe('Bob приглашает в лобби');
    expect(inviteNotif.description).toContain('Bob зовёт вас в игру в лобби ROOM_99');
    expect(inviteNotif.action?.label).toBe('Присоединиться');

    // Click "Присоединиться" button
    await inviteNotif.action?.onClick();
    expect(mockJoin).toHaveBeenCalledWith('ROOM_99');
    expect(joinedCodes).toEqual(['ROOM_99']);
  });

  it('notifies listeners when data updates without full application reload', async () => {
    const receivedData: SyncResponse[] = [];
    const mockSync = vi.fn().mockResolvedValue({
      friends: [
        {
          id: 'u3',
          nick: 'Charlie',
          status: 'online',
        },
      ],
      incoming_requests: [],
      incoming_invites: [],
    } satisfies SyncResponse);

    const service = new FriendsLiveService({ syncFn: mockSync });
    const unsubscribe = service.onData((data) => receivedData.push(data));

    await service.pollNow();
    expect(receivedData.length).toBe(1);
    expect(receivedData[0].friends[0].nick).toBe('Charlie');

    unsubscribe();
    await service.pollNow();
    expect(receivedData.length).toBe(1);
  });
});

describe('friendsLive: Windows OS vs In-App Toast Routing', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('delivers in-app Toast when window is focused and visible', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: [
          {
            id: 'u1',
            nick: 'Alice',
            status: round === 1 ? 'online' : 'playing',
          },
        ],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.setFocused(true);
    service.setVisibility(true);
    service.setGameRunning(false);

    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    await service.pollNow();
    round = 2;
    await service.pollNow();

    expect(toasts.length).toBe(1);
    expect(windowsNotifs.length).toBe(0);
    expect(toasts[0].title).toBe('Alice зашёл в игру');
  });

  it('delivers Windows OS notification when window is blurred (not focused)', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: [
          {
            id: 'u1',
            nick: 'Alice',
            status: round === 1 ? 'online' : 'playing',
          },
        ],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.setFocused(false); // Window is in background/blurred
    service.setVisibility(true);
    service.setGameRunning(false);

    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    await service.pollNow();
    round = 2;
    await service.pollNow();

    expect(toasts.length).toBe(0);
    expect(windowsNotifs.length).toBe(1);
    expect(windowsNotifs[0].title).toBe('Alice зашёл в игру');
  });

  it('delivers Windows OS notification when window is minimized or hidden', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: [
          {
            id: 'u1',
            nick: 'Alice',
            status: round === 1 ? 'online' : 'playing',
          },
        ],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.setVisibility(false); // Window minimized/hidden

    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    await service.pollNow();
    round = 2;
    await service.pollNow();

    expect(toasts.length).toBe(0);
    expect(windowsNotifs.length).toBe(1);
  });

  it('delivers Windows OS notification when launcher is sent to system tray', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: [],
        incoming_requests: [],
        incoming_invites:
          round === 1
            ? []
            : [
                {
                  id: 'inv_tray_1',
                  lobby_code: 'TRAY_LOBBY',
                  from_id: 'u_tray',
                  from_nick: 'Dave',
                },
              ],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.setInTray(true);

    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    await service.pollNow();
    round = 2;
    await service.pollNow();

    expect(toasts.length).toBe(0);
    expect(windowsNotifs.length).toBe(1);
    expect(windowsNotifs[0].title).toBe('Dave приглашает в лобби');
  });

  it('delivers Windows OS notification during active gameplay', async () => {
    let round = 1;
    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: [
          {
            id: 'u1',
            nick: 'Alice',
            status: round === 1 ? 'online' : 'playing',
          },
        ],
        incoming_requests: [],
        incoming_invites: [],
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    service.setFocused(true);
    service.setVisibility(true);
    service.setGameRunning(true); // Minecraft is running

    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    await service.pollNow();
    round = 2;
    await service.pollNow();

    expect(toasts.length).toBe(0);
    expect(windowsNotifs.length).toBe(1);
    expect(windowsNotifs[0].title).toBe('Alice зашёл в игру');
  });
});

describe('friendsLive: 5-Minute Anti-Spam Notification Cooldown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('suppresses duplicate notifications for the same friend within 5 minutes (300,000ms)', async () => {
    let currentFriends: FriendItem[] = [
      { id: 'u1', nick: 'Alice', status: 'online' },
      { id: 'u2', nick: 'Bob', status: 'online' },
    ];

    const mockSync = vi.fn().mockImplementation(async () => ({
      friends: currentFriends,
      incoming_requests: [],
      incoming_invites: [],
    }));

    const service = new FriendsLiveService({ syncFn: mockSync });
    const notifications: FriendsLiveNotification[] = [];
    service.onNotification((n) => notifications.push(n));

    // Initial baseline poll
    await service.pollNow();
    expect(notifications.length).toBe(0);

    // 1st transition: Alice enters game
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing', world_name: 'SMP' },
      { id: 'u2', nick: 'Bob', status: 'online' },
    ];
    await service.pollNow();
    expect(notifications.length).toBe(1);
    expect(notifications[0].title).toBe('Alice зашёл в игру');

    // Alice leaves game 1 minute later
    vi.advanceTimersByTime(60_000);
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'online' },
      { id: 'u2', nick: 'Bob', status: 'online' },
    ];
    await service.pollNow();
    expect(notifications.length).toBe(1); // No new notification for leaving

    // Alice re-enters game 1 minute later (2 min elapsed, < 5 min cooldown)
    vi.advanceTimersByTime(60_000);
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing', world_name: 'Minigames' },
      { id: 'u2', nick: 'Bob', status: 'online' },
    ];
    await service.pollNow();
    // Notification for Alice MUST be suppressed by anti-spam cooldown
    expect(notifications.length).toBe(1);

    // Meanwhile, Bob enters game 1 minute later (3 min elapsed from Alice's first notif)
    vi.advanceTimersByTime(60_000);
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing', world_name: 'Minigames' },
      { id: 'u2', nick: 'Bob', status: 'playing', world_name: 'PvP Arena' },
    ];
    await service.pollNow();
    // Bob has his own cooldown and MUST be notified
    expect(notifications.length).toBe(2);
    expect(notifications[1].title).toBe('Bob зашёл в игру');

    // Advance beyond 5 minutes since Alice's first notification (total > 300,000ms)
    vi.advanceTimersByTime(4 * 60_000); // Now 7 minutes since Alice's first notification
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'online' },
      { id: 'u2', nick: 'Bob', status: 'playing' },
    ];
    await service.pollNow();

    // Alice re-enters game after cooldown expired
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing', world_name: 'Hardcore' },
      { id: 'u2', nick: 'Bob', status: 'playing' },
    ];
    await service.pollNow();
    // Now Alice's notification goes through!
    expect(notifications.length).toBe(3);
    expect(notifications[2].title).toBe('Alice зашёл в игру');
  });
});

describe('friendsLive: Settings Toggle & Notification Delivery Suppression', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('suppresses both Toast and Windows notifications when notificationsEnabled is false', async () => {
    let currentFriends: FriendItem[] = [
      { id: 'u1', nick: 'Alice', status: 'online' },
      { id: 'u3', nick: 'Charlie', status: 'online' },
    ];
    let currentInvites: LobbyInviteItem[] = [];

    const mockSync = vi.fn().mockImplementation(async () => {
      return {
        friends: currentFriends,
        incoming_requests: [],
        incoming_invites: currentInvites,
      } satisfies SyncResponse;
    });

    const service = new FriendsLiveService({ syncFn: mockSync });
    const toasts: FriendsLiveNotification[] = [];
    const windowsNotifs: FriendsLiveNotification[] = [];
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => windowsNotifs.push(n));

    // Disable friend notifications in settings
    service.setNotificationsEnabled(false);
    expect(service.notificationsEnabled).toBe(false);

    // Initial baseline poll
    await service.pollNow();

    // Alice joins game and Bob invites while notifications are disabled
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing' },
      { id: 'u3', nick: 'Charlie', status: 'online' },
    ];
    currentInvites = [
      {
        id: 'inv_disabled_1',
        lobby_code: 'ROOM_DISABLED',
        from_id: 'u2',
        from_nick: 'Bob',
      },
    ];
    await service.pollNow();

    // Both should be completely silent
    expect(toasts.length).toBe(0);
    expect(windowsNotifs.length).toBe(0);

    // Re-enable friend notifications
    service.setNotificationsEnabled(true);
    expect(service.notificationsEnabled).toBe(true);

    // New event: friend Charlie joins game
    currentFriends = [
      { id: 'u1', nick: 'Alice', status: 'playing' },
      { id: 'u3', nick: 'Charlie', status: 'playing' },
    ];
    await service.pollNow();

    expect(toasts.length).toBe(1);
    expect(toasts[0].title).toBe('Charlie зашёл в игру');
  });
});

describe('friendsLive: Notification Action & Windows Notification Click Handling', () => {
  it('unminimizes window and triggers join action on notification click', async () => {
    const joinedCodes: string[] = [];
    const mockJoin = vi.fn().mockImplementation(async (code: string) => {
      joinedCodes.push(code);
    });

    const service = new FriendsLiveService({ joinFn: mockJoin });

    const inviteNotif: FriendsLiveNotification = {
      id: 'inv-test-1',
      friendId: 'u4',
      type: 'invite',
      title: 'Bob приглашает в лобби',
      description: 'Bob зовёт вас в игру в лобби ROOM_CLI',
      lobbyCode: 'ROOM_CLI',
    };

    let listenerReceived = '';
    service.onJoin((code) => {
      listenerReceived = code;
    });

    await service.handleNotificationClick(inviteNotif);

    expect(mockJoin).toHaveBeenCalledWith('ROOM_CLI');
    expect(joinedCodes).toEqual(['ROOM_CLI']);
    expect(listenerReceived).toBe('ROOM_CLI');
  });
});
