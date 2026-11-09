import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  calculatePollingInterval,
  parseWexJoinUri,
  FriendsLiveService,
  INTERVAL_VISIBLE,
  INTERVAL_HIDDEN,
  INTERVAL_IN_GAME,
  type SyncResponse,
  type FriendItem,
  type LobbyInviteItem,
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
    // Both when window is visible or hidden, in-game rate is 2 minutes
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

    // Advance 25 seconds
    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    // Advance another 25 seconds
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

    // Hide window: should switch to 120s interval
    service.setVisibility(false);
    expect(service.visible).toBe(false);
    expect(service.activeInterval).toBe(INTERVAL_HIDDEN);

    // 25s passes: no new poll should occur
    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(1);

    // Advance to 120s total: poll occurs
    await vi.advanceTimersByTimeAsync(INTERVAL_HIDDEN - INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    // Restore visibility: should switch back to 25s
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

    // Game starts
    service.setGameRunning(true);
    expect(service.inGame).toBe(true);
    expect(service.activeInterval).toBe(INTERVAL_IN_GAME);

    // 25s passes: should NOT poll yet
    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE);
    expect(pollCount).toBe(1);

    // 120s total passes: polls
    await vi.advanceTimersByTimeAsync(INTERVAL_IN_GAME - INTERVAL_VISIBLE);
    expect(pollCount).toBe(2);

    // Game exits
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

    // Pause polling (e.g. offline)
    service.pause();
    expect(service.paused).toBe(true);
    expect(service.running).toBe(false);

    // Time advances while paused: no polling occurs
    await vi.advanceTimersByTimeAsync(INTERVAL_VISIBLE * 4);
    expect(pollCount).toBe(1);

    // Manual pollNow while paused returns null
    const res = await service.pollNow();
    expect(res).toBeNull();
    expect(pollCount).toBe(1);

    // Resume polling: executes immediate poll and schedules next
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
    const notifications: any[] = [];
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
    expect(notifications[0].title).toBe('Друг зашёл в игру');
    expect(notifications[0].description).toContain('Alice зашёл в игру на сервере «Survival SMP»');
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

    const notifications: any[] = [];
    service.onNotification((n) => notifications.push(n));

    await service.pollNow();
    expect(notifications.length).toBe(0);

    // Second round brings invite
    round = 2;
    await service.pollNow();

    expect(notifications.length).toBe(1);
    const inviteNotif = notifications[0];
    expect(inviteNotif.type).toBe('invite');
    expect(inviteNotif.title).toBe('Вас пригласили');
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
    expect(receivedData.length).toBe(1); // No new events after unsubscribe
  });
});
