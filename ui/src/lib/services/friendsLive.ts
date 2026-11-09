/**
 * Wex Launcher — Real-time Friends & Live Presence Poller
 *
 * Rules & Requirements:
 * - Polls Lobby/Social API without full app reload.
 * - Interval rules:
 *   - Window visible & game idle: 20–30s (default: 25s = 25,000ms).
 *   - Window hidden / minimized: 2 minutes (120,000ms).
 *   - While game is running: 2 minutes (120,000ms).
 * - Detects live friend state changes:
 *   - "online" -> "playing" (on server X or world): triggers Windows notification "Друг зашёл в игру".
 *   - New incoming invite: triggers Windows notification "Вас пригласили" with "Присоединиться" button.
 * - Button "Присоединиться" navigates to lobby and joins room (same target as wex://join).
 * - Supports pause/resume on demand (e.g. offline state or user pause).
 */

export interface FriendItem {
  id: string;
  nick: string;
  status: 'online' | 'playing' | 'in_lobby' | 'offline';
  last_seen?: number | null;
  game_version?: string | null;
  world_name?: string | null;
  lobby_code?: string | null;
}

export interface FriendRequestItem {
  id: string;
  from_id: string;
  from_nick: string;
  created_at?: number | null;
}

export interface LobbyInviteItem {
  id: string;
  lobby_code: string;
  from_id: string;
  from_nick: string;
  host_name?: string | null;
  created_at?: number | null;
}

export interface SyncResponse {
  friends: FriendItem[];
  incoming_requests: FriendRequestItem[];
  incoming_invites: LobbyInviteItem[];
}

export interface FriendsLiveNotification {
  id: string;
  type: 'friend_playing' | 'invite';
  title: string;
  description: string;
  lobbyCode?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const INTERVAL_VISIBLE = 25_000; // 25s (within 20–30s requirement)
export const INTERVAL_HIDDEN = 120_000; // 2 min when window is hidden
export const INTERVAL_IN_GAME = 120_000; // 2 min while game is running

/**
 * Calculates current polling interval based on visibility and in-game state.
 */
export function calculatePollingInterval(options: {
  isVisible: boolean;
  isGameRunning: boolean;
}): number {
  if (options.isGameRunning) {
    return INTERVAL_IN_GAME;
  }
  if (!options.isVisible) {
    return INTERVAL_HIDDEN;
  }
  return INTERVAL_VISIBLE;
}

/**
 * Parses lobby code from a wex://join URL or hash route.
 * Examples:
 * - wex://join?code=ROOM123
 * - wex://join/ROOM123
 * - #wex://join?code=ROOM123
 * - #join=ROOM123
 */
export function parseWexJoinUri(uri: string): string | null {
  if (!uri) return null;
  const clean = uri.trim();
  const codeParam = clean.match(/[?&]code=([A-Za-z0-9_-]+)/i);
  if (codeParam) return codeParam[1];
  const slashMatch = clean.match(/wex:\/\/join\/([A-Za-z0-9_-]+)/i);
  if (slashMatch) return slashMatch[1];
  const hashJoin = clean.match(/[#&]join=([A-Za-z0-9_-]+)/i);
  if (hashJoin) return hashJoin[1];
  return null;
}

export type SyncFn = () => Promise<SyncResponse>;
export type JoinFn = (code: string) => Promise<void>;

export class FriendsLiveService {
  public friends: FriendItem[] = [];
  public requests: FriendRequestItem[] = [];
  public invites: LobbyInviteItem[] = [];

  private isVisible: boolean = true;
  private isGameRunning: boolean = false;
  private isPaused: boolean = false;
  private isStarted: boolean = false;

  private timer: any = null;
  private previousFriendsMap: Map<string, FriendItem> = new Map();
  private seenInviteIds: Set<string> = new Set();
  private initialSyncDone: boolean = false;

  private dataListeners: Set<(data: SyncResponse) => void> = new Set();
  private notificationListeners: Set<(n: FriendsLiveNotification) => void> = new Set();
  private joinListeners: Set<(code: string) => void> = new Set();

  private customSyncFn?: SyncFn;
  private customJoinFn?: JoinFn;

  constructor(options?: { syncFn?: SyncFn; joinFn?: JoinFn }) {
    this.customSyncFn = options?.syncFn;
    this.customJoinFn = options?.joinFn;
  }

  public get activeInterval(): number {
    return calculatePollingInterval({
      isVisible: this.isVisible,
      isGameRunning: this.isGameRunning,
    });
  }

  public get running(): boolean {
    return this.isStarted && !this.isPaused;
  }

  public get paused(): boolean {
    return this.isPaused;
  }

  public get visible(): boolean {
    return this.isVisible;
  }

  public get inGame(): boolean {
    return this.isGameRunning;
  }

  /**
   * Starts the polling lifecycle.
   */
  public start(): void {
    if (this.isStarted) return;
    this.isStarted = true;
    this.isPaused = false;

    // Detect browser visibility if available
    if (typeof document !== 'undefined') {
      this.isVisible = document.visibilityState === 'visible';
      document.addEventListener('visibilitychange', this.onVisibilityChange);
    }

    void this.pollNow();
    this.scheduleNext();
  }

  /**
   * Completely stops polling and cleans up timers and listeners.
   */
  public stop(): void {
    this.isStarted = false;
    this.isPaused = false;
    this.clearTimer();

    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', this.onVisibilityChange);
    }
  }

  /**
   * Pauses polling without destroying existing friends state.
   */
  public pause(): void {
    this.isPaused = true;
    this.clearTimer();
  }

  /**
   * Resumes polling after pause.
   */
  public resume(): void {
    if (!this.isStarted) {
      this.start();
      return;
    }
    this.isPaused = false;
    void this.pollNow();
    this.scheduleNext();
  }

  /**
   * Updates window visibility state (visible vs hidden/minimized) and adjusts polling interval.
   */
  public setVisibility(visible: boolean): void {
    if (this.isVisible === visible) return;
    this.isVisible = visible;
    if (this.isStarted && !this.isPaused) {
      this.reschedule();
    }
  }

  /**
   * Updates game running state (idle vs Minecraft active) and adjusts polling interval.
   */
  public setGameRunning(running: boolean): void {
    if (this.isGameRunning === running) return;
    this.isGameRunning = running;
    if (this.isStarted && !this.isPaused) {
      this.reschedule();
    }
  }

  /**
   * Immediately executes a sync round and updates state.
   */
  public async pollNow(): Promise<SyncResponse | null> {
    if (this.isPaused && this.isStarted) return null;

    try {
      const data = await this.executeSync();
      this.processSyncResult(data);
      return data;
    } catch {
      return null;
    }
  }

  /**
   * Handles user clicking "Присоединиться" or executing wex://join.
   */
  public async handleJoin(code: string): Promise<void> {
    const cleanCode = code.trim().replace(/^#/, '');
    if (!cleanCode) return;

    for (const listener of this.joinListeners) {
      listener(cleanCode);
    }

    if (this.customJoinFn) {
      await this.customJoinFn(cleanCode);
      return;
    }

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('join_lobby_room', { code: cleanCode, playerName: 'Player' });
      } catch {
        // Handled by UI error toasts
      }
    }
  }

  public onData(listener: (data: SyncResponse) => void): () => void {
    this.dataListeners.add(listener);
    return () => this.dataListeners.delete(listener);
  }

  public onNotification(listener: (n: FriendsLiveNotification) => void): () => void {
    this.notificationListeners.add(listener);
    return () => this.notificationListeners.delete(listener);
  }

  public onJoin(listener: (code: string) => void): () => void {
    this.joinListeners.add(listener);
    return () => this.joinListeners.delete(listener);
  }

  // --------------------------------------------------------------------------
  // Internal Helpers
  // --------------------------------------------------------------------------

  private onVisibilityChange = () => {
    if (typeof document !== 'undefined') {
      this.setVisibility(document.visibilityState === 'visible');
    }
  };

  private clearTimer(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private scheduleNext(): void {
    this.clearTimer();
    if (!this.isStarted || this.isPaused) return;

    const interval = this.activeInterval;
    this.timer = setTimeout(async () => {
      await this.pollNow();
      this.scheduleNext();
    }, interval);
  }

  private reschedule(): void {
    this.scheduleNext();
  }

  private async executeSync(): Promise<SyncResponse> {
    if (this.customSyncFn) {
      return await this.customSyncFn();
    }

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SyncResponse>('social_sync', {});
    }

    // Default empty in unit test environment if not mocked
    return {
      friends: this.friends,
      incoming_requests: this.requests,
      incoming_invites: this.invites,
    };
  }

  private processSyncResult(data: SyncResponse): void {
    const newFriends = data.friends || [];
    const newInvites = data.incoming_invites || [];
    const newRequests = data.incoming_requests || [];

    if (this.initialSyncDone) {
      // 1. Check for friends entering game ("online"/"offline" -> "playing")
      for (const friend of newFriends) {
        const prev = this.previousFriendsMap.get(friend.id);
        if (prev && prev.status !== 'playing' && friend.status === 'playing') {
          const serverInfo = friend.world_name
            ? ` на сервере «${friend.world_name}»`
            : friend.game_version
              ? ` (${friend.game_version})`
              : '';

          const notif: FriendsLiveNotification = {
            id: `notif-playing-${friend.id}-${Date.now()}`,
            type: 'friend_playing',
            title: 'Друг зашёл в игру',
            description: `${friend.nick} зашёл в игру${serverInfo}`,
          };
          this.emitNotification(notif);
        }
      }

      // 2. Check for new incoming lobby invites
      for (const invite of newInvites) {
        if (!this.seenInviteIds.has(invite.id)) {
          this.seenInviteIds.add(invite.id);
          const notif: FriendsLiveNotification = {
            id: `notif-invite-${invite.id}`,
            type: 'invite',
            title: 'Вас пригласили',
            description: `${invite.from_nick} зовёт вас в игру в лобби ${invite.lobby_code}`,
            lobbyCode: invite.lobby_code,
            action: {
              label: 'Присоединиться',
              onClick: () => {
                void this.handleJoin(invite.lobby_code);
              },
            },
          };
          this.emitNotification(notif);
        }
      }
    } else {
      // Record initial state without spamming notifications on first load
      for (const inv of newInvites) {
        this.seenInviteIds.add(inv.id);
      }
      this.initialSyncDone = true;
    }

    // Update state & maps
    this.friends = newFriends;
    this.requests = newRequests;
    this.invites = newInvites;

    this.previousFriendsMap.clear();
    for (const f of newFriends) {
      this.previousFriendsMap.set(f.id, { ...f });
    }

    for (const listener of this.dataListeners) {
      listener(data);
    }
  }

  private emitNotification(notif: FriendsLiveNotification): void {
    // 1. Deliver to in-app listeners (App.svelte / Toast.svelte)
    for (const listener of this.notificationListeners) {
      listener(notif);
    }

    // 2. Deliver to Windows Notification Action Center via Web Notification API
    if (typeof window !== 'undefined' && 'Notification' in window) {
      try {
        if (Notification.permission === 'granted') {
          const winNotif = new Notification(notif.title, {
            body: notif.description,
          });
          if (notif.action) {
            winNotif.onclick = () => {
              if (typeof window !== 'undefined' && window.focus) {
                window.focus();
              }
              notif.action?.onClick();
            };
          }
        } else if (Notification.permission !== 'denied') {
          Notification.requestPermission().then((perm) => {
            if (perm === 'granted') {
              const winNotif = new Notification(notif.title, {
                body: notif.description,
              });
              if (notif.action) {
                winNotif.onclick = () => {
                  if (typeof window !== 'undefined' && window.focus) {
                    window.focus();
                  }
                  notif.action?.onClick();
                };
              }
            }
          });
        }
      } catch {
        // Ignore environments where Notification constructor is unavailable
      }
    }
  }
}

// Global shared singleton instance for the UI
export const friendsLive = new FriendsLiveService();
