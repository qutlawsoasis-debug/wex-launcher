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
  friendId: string;
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
export type JoinFn = (code: string) => Promise<void>;
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
  private joinListeners: Set<(code: string) => void> = new Set();

  private customSyncFn?: SyncFn;
  private customJoinFn?: JoinFn;
  private customWindowsNotifyFn?: WindowsNotifyFn;

  constructor(options?: {
    syncFn?: SyncFn;
    joinFn?: JoinFn;
    windowsNotifyFn?: WindowsNotifyFn;
  }) {
    this.customSyncFn = options?.syncFn;
    this.customJoinFn = options?.joinFn;
    this.customWindowsNotifyFn = options?.windowsNotifyFn;

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
    }

    // Setup action listener for tauri-plugin-notification
    this.setupTauriNotificationActionListener();

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
    if (typeof window !== 'undefined') {
      window.removeEventListener('focus', this.onFocus);
      window.removeEventListener('blur', this.onBlur);
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
      } catch {}
    }
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

  private onFocus = () => {
    this.setFocused(true);
  };

  private onBlur = () => {
    this.setFocused(false);
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

  private processSyncResult(data: SyncResponse): void {
    const newFriends = data.friends || [];
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

            const notif: FriendsLiveNotification = {
              id: `notif-invite-${invite.id}`,
              friendId: invite.from_id,
              type: 'invite',
              title: `${invite.from_nick} приглашает в лобби`,
              description: `${invite.from_nick} зовёт вас в игру в лобби ${invite.lobby_code}`,
              lobbyCode: invite.lobby_code,
              action: {
                label: 'Присоединиться',
                onClick: () => {
                  void this.handleJoin(invite.lobby_code);
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
        const pluginPkg = '@tauri-apps/plugin-notification';
        const { isPermissionGranted, requestPermission, sendNotification } = await import(
          /* @vite-ignore */ pluginPkg
        );
        let hasPerm = await isPermissionGranted();
        if (!hasPerm) {
          const perm = await requestPermission();
          hasPerm = perm === 'granted';
        }
        if (hasPerm) {
          sendNotification({
            title: notif.title,
            body: notif.description,
            extra: notif.lobbyCode ? { lobbyCode: notif.lobbyCode } : undefined,
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
            void this.handleNotificationClick(notif);
          };
        } else if (Notification.permission !== 'denied') {
          Notification.requestPermission().then((perm) => {
            if (perm === 'granted') {
              const winNotif = new Notification(notif.title, {
                body: notif.description,
              });
              winNotif.onclick = () => {
                void this.handleNotificationClick(notif);
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
    } else if (notif.action) {
      notif.action.onClick();
    }
  }

  private setupTauriNotificationActionListener(): void {
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      const pluginPkg = '@tauri-apps/plugin-notification';
      import(/* @vite-ignore */ pluginPkg)
        .then(({ onAction }) => {
          onAction((action: any) => {
            const lobbyCode = action.notification?.extra?.lobbyCode;
            if (lobbyCode) {
              void this.handleJoin(lobbyCode);
            }
          });
        })
        .catch(() => {});
    }
  }
}

// Global shared singleton instance for the UI
export const friendsLive = new FriendsLiveService();
