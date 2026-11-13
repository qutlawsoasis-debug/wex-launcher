/**
 * Wex Launcher — Real-time Friends & Live Presence Poller
 *
 * Requirements (wex-friends-notify):
 * - Real-time friends presence updates without reload.
 * - Dynamic intervals:
 *   - Window visible & game idle: 20–30s (default: 25s = 25,000ms).
 *   - Window hidden / minimized: 2 minutes (120,000ms).
 *   - While game is running: 2 minutes (120,000ms).
 * - Real Windows notifications via tauri-plugin-notification when window is not in focus
 *   (blurred, hidden/minimized, in tray, or during active gameplay):
 *   - «<ник> зашёл в игру» (with world/server name if present)
 *   - «<ник> приглашает в лобби» (with action to join)
 * - Click on notification unminimizes/focuses window and joins room (same as wex://join).
 * - When in focus: displays in-app Toast instead of OS notification.
 * - Setting «Уведомления о друзьях» (enabled by default).
 * - Anti-spam cooldown: at most one notification per friend in 5 minutes (300,000ms).
 */

import type { UserPresenceDto } from '../ipc/generated/models';
import type { LobbyRoom } from '../pages/friends/lobbyTypes';

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

/**
 * Friends sign-in, as core reports it (commands/friends.rs FriendsConnectionDto).
 * Core signs in with the launcher's active account; the UI never sees a token.
 *   signed_out      no account (reason no_account) or the session ended (session_expired)
 *   connecting      sign-in in progress
 *   signed_in       friends work
 *   unreachable     the friends server cannot be used now (network, timeout, server_error,
 *                   no_friends_api, bad_response); «Повторить» tries again
 *   unusable        the server refused this account (nick_rejected); detail = its words
 *   not_configured  friends are switched off (WEX_FRIENDS_API_URL=off)
 */
export type FriendsConnectionState = 'signed_out' | 'connecting' | 'signed_in' | 'unreachable' | 'unusable' | 'not_configured';

export interface FriendsConnection {
  state: FriendsConnectionState;
  reason?: string | null;
  detail?: string | null;
  account_id?: string | null;
  account_kind?: 'microsoft' | 'offline' | null;
  nick?: string | null;
  friend_code?: string | null;
  server?: string | null;
}

/** Window event the accounts store fires when the active account changes. */
export const ACTIVE_ACCOUNT_EVENT = 'wex-active-account';

const STATES: readonly FriendsConnectionState[] = ['signed_out', 'connecting', 'signed_in', 'unreachable', 'unusable', 'not_configured'];

function isConnection(v: unknown): v is FriendsConnection {
  return !!v && typeof v === 'object' && STATES.includes((v as FriendsConnection).state);
}

/** Core sends OnlineStatusDto as "Online" | "Playing" | "InLobby" | "Offline"; the UI uses snake case. */
export function normalizeStatus(s: unknown): FriendItem['status'] {
  const v = String(s ?? '').toLowerCase().replace(/[^a-z]/g, '');
  if (v === 'playing') return 'playing';
  if (v === 'inlobby' || v === 'lobby') return 'in_lobby';
  if (v === 'online') return 'online';
  return 'offline';
}

export type ConnectFn = () => Promise<FriendsConnection>;
export type StatusFn = () => Promise<FriendsConnection>;

export interface FriendsLiveNotification {
  id: string;
  friendId: string;
  type: 'friend_playing' | 'invite';
  title: string;
  description: string;
  lobbyCode?: string;
  /** Invite whose room code the server hands out only on accept. */
  inviteId?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const INTERVAL_VISIBLE = 25_000; // 25s (within 20–30s requirement)
export const INTERVAL_HIDDEN = 120_000; // 2 min when window is hidden
export const INTERVAL_IN_GAME = 120_000; // 2 min while game is running
export const FRIEND_NOTIFICATION_COOLDOWN_MS = 5 * 60 * 1000; // 5 min anti-spam cooldown

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
 * Decides whether to send a Windows OS notification vs an in-app Toast.
 * When window is NOT in focus (blurred, minimized/hidden, in tray, or game is running):
 * returns true -> send native Windows notification.
 * When in focus, visible, and idle:
 * returns false -> send in-app Toast.
 */
export function shouldSendWindowsNotification(options: {
  isFocused: boolean;
  isVisible: boolean;
  isGameRunning: boolean;
  isInTray?: boolean;
}): boolean {
  if (options.isGameRunning || !options.isVisible || !options.isFocused || options.isInTray) {
    return true;
  }
  return false;
}

/**
 * Checks if notification for friend can be sent given the 5-minute anti-spam cooldown.
 */
export function checkFriendNotificationCooldown(
  lastNotificationTime: number | undefined,
  currentTime: number,
  cooldownMs = FRIEND_NOTIFICATION_COOLDOWN_MS
): boolean {
  if (lastNotificationTime === undefined) return true;
  return currentTime - lastNotificationTime >= cooldownMs;
}

/**
 * Parses lobby code from a wex://join URL or hash route.
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
export type JoinFn = (code: string) => Promise<void | LobbyRoom>;
export type WindowsNotifyFn = (notif: FriendsLiveNotification) => Promise<void> | void;

export class FriendsLiveService {
  public friends: FriendItem[] = [];
  public requests: FriendRequestItem[] = [];
  public invites: LobbyInviteItem[] = [];

  public notificationsEnabled: boolean = true;
  private isVisible: boolean = true;
  private isFocused: boolean = true;
  private isGameRunning: boolean = false;
  private isInTray: boolean = false;
  private isPaused: boolean = false;
  private isStarted: boolean = false;

  private timer: any = null;
  private previousFriendsMap: Map<string, FriendItem> = new Map();
  private seenInviteIds: Set<string> = new Set();
  private friendCooldowns: Map<string, number> = new Map();
  private initialSyncDone: boolean = false;

  private dataListeners: Set<(data: SyncResponse) => void> = new Set();
  private toastListeners: Set<(n: FriendsLiveNotification) => void> = new Set();
  private windowsNotificationListeners: Set<(n: FriendsLiveNotification) => void> = new Set();
  private joinListeners: Set<(code: string, room?: LobbyRoom) => void> = new Set();
  private errorListeners = new Set<(error: unknown, context: 'friends' | 'lobby') => void>();
  private playerName = '';
  private gameVersion = '';
  private runningGameVersion: string | null = null;
  private lobbyRoom: LobbyRoom | null = null;
  private presenceQueue: Promise<void> = Promise.resolve();
  private lastPublishedPresence = '';
  public lastPresenceError: unknown = null;
  private notificationActionListener: { unregister: () => void | Promise<void> } | null = null;
  private notificationActionReady: Promise<void> | null = null;
  private lifecycle = 0;

  private customSyncFn?: SyncFn;
  private customJoinFn?: JoinFn;
  private customWindowsNotifyFn?: WindowsNotifyFn;
  private customConnectFn?: ConnectFn;
  private customStatusFn?: StatusFn;

  /** Friends sign-in state. Managed services start as «connecting». */
  public connection: FriendsConnection;
  private connectionListeners = new Set<(c: FriendsConnection) => void>();
  private connecting: Promise<FriendsConnection> | null = null;
  private connectAgain = false;

  constructor(options?: {
    syncFn?: SyncFn;
    joinFn?: JoinFn;
    windowsNotifyFn?: WindowsNotifyFn;
    connectFn?: ConnectFn;
    statusFn?: StatusFn;
  }) {
    this.customSyncFn = options?.syncFn;
    this.customJoinFn = options?.joinFn;
    this.customWindowsNotifyFn = options?.windowsNotifyFn;
    this.customConnectFn = options?.connectFn;
    this.customStatusFn = options?.statusFn;
    // A custom sync without a custom connect is the old test seam: sign-in is not modelled.
    this.connection = { state: this.managed ? 'connecting' : 'signed_in' };

    // Load saved notification preference if available
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('wex_friend_notifications');
      if (saved !== null) {
        this.notificationsEnabled = saved === 'true';
      }
    }
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

  public get focused(): boolean {
    return this.isFocused;
  }

  public get inGame(): boolean {
    return this.isGameRunning;
  }

  public get inTray(): boolean {
    return this.isInTray;
  }

  /**
   * Sets whether friend notifications are enabled (default: true).
   */
  public setNotificationsEnabled(enabled: boolean): void {
    this.notificationsEnabled = enabled;
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem('wex_friend_notifications', String(enabled));
      } catch {}
    }
  }

  /**
   * Starts the polling lifecycle and registers focus/visibility listeners.
   */
  public start(): void {
    if (this.isStarted) return;
    this.isStarted = true;
    this.isPaused = false;

    if (typeof document !== 'undefined') {
      this.isVisible = document.visibilityState === 'visible';
      this.isFocused = document.hasFocus ? document.hasFocus() : true;
      document.addEventListener('visibilitychange', this.onVisibilityChange);
    }

    if (typeof window !== 'undefined') {
      window.addEventListener('focus', this.onFocus);
      window.addEventListener('blur', this.onBlur);
      window.addEventListener('beforeunload', this.onBeforeUnload);
      window.addEventListener(ACTIVE_ACCOUNT_EVENT, this.onAccountChange);
    }

    // Setup action listener for tauri-plugin-notification
    this.notificationActionReady = this.setupTauriNotificationActionListener(++this.lifecycle);
    void this.publishPresence();

    void this.pollNow();
    this.scheduleNext();
  }

  /**
   * Completely stops polling and cleans up timers and listeners.
   */
  public stop(): void {
    if (this.isStarted) void this.publishPresence('Offline');
    this.isStarted = false;
    ++this.lifecycle;
    const actionListener = this.notificationActionListener;
    this.notificationActionListener = null;
    if (actionListener) void Promise.resolve(actionListener.unregister()).catch(error => this.reportError(error));
    this.isPaused = false;
    this.clearTimer();

    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', this.onVisibilityChange);
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('focus', this.onFocus);
      window.removeEventListener('blur', this.onBlur);
      window.removeEventListener('beforeunload', this.onBeforeUnload);
      window.removeEventListener(ACTIVE_ACCOUNT_EVENT, this.onAccountChange);
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
   * Updates window visibility state (visible vs hidden/minimized).
   */
  public setVisibility(visible: boolean): void {
    if (this.isVisible === visible) return;
    this.isVisible = visible;
    if (this.isStarted && !this.isPaused) {
      this.reschedule();
    }
  }

  /**
   * Updates window focus state (in focus vs blurred/background).
   */
  public setFocused(focused: boolean): void {
    this.isFocused = focused;
  }

  /**
   * Updates tray state (hidden to tray vs on screen).
   */
  public setInTray(inTray: boolean): void {
    this.isInTray = inTray;
    if (inTray) {
      this.setVisibility(false);
      this.setFocused(false);
    }
  }

  /**
   * Updates game running state (idle vs Minecraft active).
   */
  public setGameRunning(running: boolean): void {
    if (this.isGameRunning === running) return;
    this.isGameRunning = running;
    this.runningGameVersion = running ? this.gameVersion : null;
    if (this.isStarted) void this.publishPresence();
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
      if (this.managed && this.connection.state !== 'signed_in') {
        // Sign in first (start, after «connecting», or the server was unreachable).
        if (!this.autoConnects()) return null;
        const c = await this.connect({ quiet: this.connection.state === 'unreachable' });
        if (c.state !== 'signed_in') return null;
      }
      if (this.isStarted) void this.publishPresence();
      const data = await this.executeSync();
      this.processSyncResult(data);
      if (this.managed) await this.refreshConnection();
      return data;
    } catch {
      return null;
    }
  }

  // --------------------------------------------------------------------------
  // Friends sign-in (with the launcher's active account, by core)
  // --------------------------------------------------------------------------

  /** Real sign-in through core, unless a test replaced only the sync. */
  private get managed(): boolean {
    return !this.customSyncFn || !!this.customConnectFn;
  }

  /** States that heal on their own; the others wait for an account or for «Повторить». */
  private autoConnects(): boolean {
    const { state, reason } = this.connection;
    if (state === 'connecting' || state === 'unreachable') return true;
    return state === 'signed_out' && reason !== 'no_account';
  }

  public onConnection(listener: (c: FriendsConnection) => void): () => void {
    this.connectionListeners.add(listener);
    return () => this.connectionListeners.delete(listener);
  }

  private setConnection(c: FriendsConnection): void {
    if (JSON.stringify(c) === JSON.stringify(this.connection)) return;
    this.connection = c;
    for (const listener of this.connectionListeners) listener(c);
  }

  /**
   * Sign in to friends with the active account. Calls made while one runs wait
   * for it and run once more after it (an account switch in the middle).
   * `quiet` keeps the current message on screen (automatic retries).
   */
  public connect(options: { quiet?: boolean } = {}): Promise<FriendsConnection> {
    if (this.connecting) {
      this.connectAgain = true;
      return this.connecting;
    }
    if (!options.quiet && this.connection.state !== 'connecting') this.setConnection({ ...this.connection, state: 'connecting', reason: null, detail: null });
    this.connecting = (async () => {
      let c: FriendsConnection;
      do {
        this.connectAgain = false;
        c = await this.executeConnect();
        this.setConnection(c);
      } while (this.connectAgain);
      this.connecting = null;
      return c;
    })();
    return this.connecting;
  }

  /** «Повторить»: sign in again now, then fetch the lists. */
  public async retry(): Promise<void> {
    await this.connect();
    if (this.connection.state === 'signed_in') await this.pollNow();
  }

  private async executeConnect(): Promise<FriendsConnection> {
    try {
      let c: unknown;
      if (this.customConnectFn) c = await this.customConnectFn();
      else if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core');
        c = await invoke('social_connect');
      } else {
        return { state: 'not_configured', reason: 'no_backend' };
      }
      if (isConnection(c)) return c;
      return { ...this.connection, state: 'unreachable', reason: 'bad_response', detail: null };
    } catch (error) {
      return { ...this.connection, state: 'unreachable', reason: 'app_error', detail: error instanceof Error ? error.message : String(error) };
    }
  }

  /** After a sync: core may have seen the server fail or the session end. */
  private async refreshConnection(): Promise<void> {
    try {
      let c: unknown;
      if (this.customStatusFn) c = await this.customStatusFn();
      else if (this.customConnectFn) return;
      else if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core');
        c = await invoke('social_connection_status');
      } else return;
      if (isConnection(c)) this.setConnection(c);
    } catch {}
  }

  private onAccountChange = () => {
    // Another player: the old friends are not theirs.
    this.friends = [];
    this.requests = [];
    this.invites = [];
    this.previousFriendsMap.clear();
    this.seenInviteIds.clear();
    this.initialSyncDone = false;
    for (const listener of this.dataListeners) listener({ friends: [], incoming_requests: [], incoming_invites: [] });
    if (this.managed) void this.retry();
  };

  /** Accept an invite whose room code the server gives only on accept, then join. */
  public async acceptInvite(inviteId: string): Promise<void> {
    if (typeof window === 'undefined' || !(window as any).__TAURI_INTERNALS__) return;
    const { invoke } = await import('@tauri-apps/api/core');
    const code = await invoke<string | null>('social_respond_lobby_invite', { inviteId, accept: true });
    this.invites = this.invites.filter((i) => i.id !== inviteId);
    if (!code) throw new Error('Сервер друзей не прислал код комнаты для этого приглашения.');
    await this.handleJoin(code);
  }

  /**
   * Handles user clicking "Присоединиться" or executing wex://join.
   */
  public async handleJoin(code: string): Promise<void> {
    const cleanCode = code.trim().replace(/^#/, '').toUpperCase();
    if (!cleanCode) return;

    // Raise and focus window if minimized or in tray
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      try {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = getCurrentWebviewWindow();
        await win.unminimize();
        await win.show();
        await win.setFocus();
      } catch {}
    }

    if (this.customJoinFn) {
      const room = await this.customJoinFn(cleanCode);
      for (const listener of this.joinListeners) listener(cleanCode, room || undefined);
      return;
    }

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      const { invoke } = await import('@tauri-apps/api/core');
      const room = await invoke<LobbyRoom>('join_lobby_room', { code: cleanCode, playerName: this.playerName });
      this.lobbyRoom = room;
      if (this.isStarted) void this.publishPresence();
      for (const listener of this.joinListeners) listener(cleanCode, room);
    }
  }

  public setSessionContext(context: { playerName: string; gameVersion: string; lobby: LobbyRoom | null }): void {
    const changed = this.playerName !== context.playerName || this.gameVersion !== context.gameVersion || this.lobbyRoom !== context.lobby;
    this.playerName = context.playerName;
    this.gameVersion = context.gameVersion;
    this.lobbyRoom = context.lobby;
    if (changed && this.isStarted) void this.publishPresence();
  }

  public onError(listener: (error: unknown, context: 'friends' | 'lobby') => void): () => void {
    this.errorListeners.add(listener);
    return () => this.errorListeners.delete(listener);
  }

  private reportError(error: unknown, context: 'friends' | 'lobby' = 'friends'): void {
    for (const listener of this.errorListeners) listener(error, context);
  }

  private publishPresence(status?: 'Offline'): Promise<void> {
    const presence: UserPresenceDto = {
      status: status || (this.isGameRunning ? 'Playing' : this.lobbyRoom ? 'InLobby' : 'Online'),
      game_version: !status ? this.runningGameVersion : null,
      world_name: !status ? this.lobbyRoom?.world_name || null : null,
      lobby_code: !status ? this.lobbyRoom?.code || null : null,
    };
    const key = JSON.stringify([this.playerName, presence]);
    this.presenceQueue = this.presenceQueue.then(async () => {
      if (key === this.lastPublishedPresence) return;
      if (typeof window === 'undefined' || !(window as any).__TAURI_INTERNALS__) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('social_set_presence', { presence });
        this.lastPublishedPresence = key;
        this.lastPresenceError = null;
      } catch (error) {
        if (!this.lastPresenceError) this.reportError(error);
        this.lastPresenceError = error;
      }
    });
    return this.presenceQueue;
  }

  public onData(listener: (data: SyncResponse) => void): () => void {
    this.dataListeners.add(listener);
    return () => this.dataListeners.delete(listener);
  }

  /**
   * Listens for in-app Toast notifications (displayed when window is in focus).
   */
  public onNotification(listener: (n: FriendsLiveNotification) => void): () => void {
    this.toastListeners.add(listener);
    return () => this.toastListeners.delete(listener);
  }

  /**
   * Listens for Windows OS native notifications (sent when window is not in focus).
   */
  public onWindowsNotification(listener: (n: FriendsLiveNotification) => void): () => void {
    this.windowsNotificationListeners.add(listener);
    return () => this.windowsNotificationListeners.delete(listener);
  }

  public onJoin(listener: (code: string, room?: LobbyRoom) => void): () => void {
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

  private onFocus = () => {
    this.setFocused(true);
  };

  private onBlur = () => {
    this.setFocused(false);
  };

  private onBeforeUnload = () => {
    this.stop();
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

    return {
      friends: this.friends,
      incoming_requests: this.requests,
      incoming_invites: this.invites,
    };
  }

  private processSyncResult(answer: SyncResponse | null | undefined): void {
    const raw: SyncResponse = answer ?? { friends: [], incoming_requests: [], incoming_invites: [] };
    const newFriends = (raw.friends || []).map((f) => ({ ...f, status: normalizeStatus(f.status) }));
    const data: SyncResponse = { ...raw, friends: newFriends };
    const newInvites = data.incoming_invites || [];
    const newRequests = data.incoming_requests || [];

    if (this.initialSyncDone && this.notificationsEnabled) {
      const now = Date.now();

      // 1. Check for friends entering game ("online"/"offline" -> "playing")
      for (const friend of newFriends) {
        const prev = this.previousFriendsMap.get(friend.id);
        if (prev && prev.status !== 'playing' && friend.status === 'playing') {
          // Check anti-spam cooldown (max 1 notification per friend per 5 minutes)
          const lastSent = this.friendCooldowns.get(friend.id);
          if (checkFriendNotificationCooldown(lastSent, now)) {
            this.friendCooldowns.set(friend.id, now);

            const serverInfo = friend.world_name
              ? ` на сервере «${friend.world_name}»`
              : friend.game_version
                ? ` (${friend.game_version})`
                : '';

            const notif: FriendsLiveNotification = {
              id: `notif-playing-${friend.id}-${now}`,
              friendId: friend.id,
              type: 'friend_playing',
              title: `${friend.nick} зашёл в игру`,
              description: `${friend.nick} играет${serverInfo}`,
            };
            this.dispatchNotification(notif);
          }
        }
      }

      // 2. Check for new incoming lobby invites
      for (const invite of newInvites) {
        if (!this.seenInviteIds.has(invite.id)) {
          this.seenInviteIds.add(invite.id);

          // Check anti-spam cooldown for sender
          const lastSent = this.friendCooldowns.get(invite.from_id);
          if (checkFriendNotificationCooldown(lastSent, now)) {
            this.friendCooldowns.set(invite.from_id, now);

            const code = invite.lobby_code || '';
            const notif: FriendsLiveNotification = {
              id: `notif-invite-${invite.id}`,
              friendId: invite.from_id,
              type: 'invite',
              title: `${invite.from_nick} приглашает в лобби`,
              description: code ? `${invite.from_nick} зовёт вас в игру в лобби ${code}` : `${invite.from_nick} зовёт вас в игру`,
              lobbyCode: code || undefined,
              inviteId: code ? undefined : invite.id,
              action: {
                label: 'Присоединиться',
                onClick: () => {
                  const run = code ? this.handleJoin(code) : this.acceptInvite(invite.id);
                  void run.catch(error => this.reportError(error, 'lobby'));
                },
              },
            };
            this.dispatchNotification(notif);
          }
        }
      }
    } else {
      // Record initial baseline
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

  /**
   * Routes notification to either Windows OS Notification or In-app Toast
   * based on whether the window is currently in focus.
   */
  private dispatchNotification(notif: FriendsLiveNotification): void {
    if (!this.notificationsEnabled) return;

    const useWindowsNotification = shouldSendWindowsNotification({
      isFocused: this.isFocused,
      isVisible: this.isVisible,
      isGameRunning: this.isGameRunning,
      isInTray: this.isInTray,
    });

    if (useWindowsNotification) {
      // Not in focus (blurred, hidden/minimized, in tray, or game running) -> Windows Notification
      void this.deliverWindowsNotification(notif);
    } else {
      // In focus -> in-app Liquid Glass Toast
      for (const listener of this.toastListeners) {
        listener(notif);
      }
    }
  }

  private async deliverWindowsNotification(notif: FriendsLiveNotification): Promise<void> {
    for (const listener of this.windowsNotificationListeners) {
      listener(notif);
    }

    if (this.customWindowsNotifyFn) {
      await this.customWindowsNotifyFn(notif);
      return;
    }

    // 1. Deliver through tauri-plugin-notification
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      try {
        await this.notificationActionReady;
        const { isPermissionGranted, requestPermission, sendNotification } = await import('@tauri-apps/plugin-notification');
        let hasPerm = await isPermissionGranted();
        if (!hasPerm) {
          const perm = await requestPermission();
          hasPerm = perm === 'granted';
        }
        if (hasPerm) {
          sendNotification({
            title: notif.title,
            body: notif.description,
            extra: notif.lobbyCode ? { lobbyCode: notif.lobbyCode } : notif.inviteId ? { inviteId: notif.inviteId } : undefined,
          });
          return;
        }
      } catch {
        // fallback to Web Notification
      }
    }

    // 2. Deliver through standard Web Notification API (Windows Action Center in WebView2)
    if (typeof window !== 'undefined' && 'Notification' in window) {
      try {
        if (Notification.permission === 'granted') {
          const winNotif = new Notification(notif.title, {
            body: notif.description,
          });
          winNotif.onclick = () => {
            void this.handleNotificationClick(notif).catch(error => this.reportError(error, 'lobby'));
          };
        } else if (Notification.permission !== 'denied') {
          Notification.requestPermission().then((perm) => {
            if (perm === 'granted') {
              const winNotif = new Notification(notif.title, {
                body: notif.description,
              });
              winNotif.onclick = () => {
                void this.handleNotificationClick(notif).catch(error => this.reportError(error, 'lobby'));
              };
            }
          });
        }
      } catch {}
    }
  }

  /**
   * Handles user clicking a Windows notification or in-app toast action:
   * unminimizes and focuses launcher window, and joins lobby if lobbyCode is attached.
   */
  public async handleNotificationClick(notif: FriendsLiveNotification): Promise<void> {
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      try {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = getCurrentWebviewWindow();
        await win.unminimize();
        await win.show();
        await win.setFocus();
      } catch {}
    }

    if (notif.lobbyCode) {
      await this.handleJoin(notif.lobbyCode);
    } else if (notif.inviteId) {
      await this.acceptInvite(notif.inviteId);
    } else if (notif.action) {
      notif.action.onClick();
    }
  }

  private async setupTauriNotificationActionListener(lifecycle: number): Promise<void> {
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      try {
        const { onAction } = await import('@tauri-apps/plugin-notification');
        const listener = await onAction((action) => {
            if (!this.isStarted || lifecycle !== this.lifecycle) return;
            const lobbyCode = action.notification?.extra?.lobbyCode;
            const inviteId = action.notification?.extra?.inviteId;
            if (typeof lobbyCode === 'string') {
              void this.handleJoin(lobbyCode).catch(error => this.reportError(error, 'lobby'));
            } else if (typeof inviteId === 'string') {
              void this.acceptInvite(inviteId).catch(error => this.reportError(error, 'lobby'));
            }
        });
        if (!this.isStarted || lifecycle !== this.lifecycle) await listener.unregister();
        else this.notificationActionListener = listener;
      } catch (error) { this.reportError(error); }
    }
  }
}

// Global shared singleton instance for the UI
export const friendsLive = new FriendsLiveService();
