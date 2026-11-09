/**
 * Wex Launcher — Friends Live Polling & Notification Supervisor (Svelte 5)
 *
 * Rules:
 * - Real-time friend status «в сети / в игре на сервере X» without full reload
 * - Polling Lobby API: every 20–30s when window is visible, every 2 min when hidden
 * - Immediate refresh on transition to visible if >= 20s elapsed
 * - Windows / in-app notifications: «Друг зашёл в игру» and «Вас пригласили»
 * - «Присоединиться» action button connecting to lobby (same as wex://join)
 * - Zero duplicated lobby/friends code; relies on existing social_sync / join_lobby_room
 */

import { i18n } from '../i18n';

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

export interface FriendNotificationAction {
  label: string;
  onclick: () => void;
}

export interface FriendLiveNotification {
  type: 'friend_playing' | 'invite_received';
  title: string;
  description: string;
  action?: FriendNotificationAction;
}

export interface FriendsLiveOptions {
  visibleIntervalMs?: number; // default: 25_000 (25s)
  hiddenIntervalMs?: number;  // default: 120_000 (2m)
  immediateRefreshThresholdMs?: number; // default: 20_000 (20s)
  onNotification?: (notification: FriendLiveNotification) => void;
  onJoinLobby?: (code: string) => void;
  syncFn?: () => Promise<{
    friends?: FriendItem[];
    incoming_requests?: FriendRequestItem[];
    incoming_invites?: LobbyInviteItem[];
  }>;
}

export class FriendsLiveManager {
  friends = $state<FriendItem[]>([]);
  incomingRequests = $state<FriendRequestItem[]>([]);
  incomingInvites = $state<LobbyInviteItem[]>([]);
  isSyncing = $state<boolean>(false);
  lastSyncTime = $state<number>(0);
  activePollIntervalMs = $state<number>(25_000);

  private timer: any = null;
  private isPolling = false;
  private knownFriendStatuses = new Map<string, string>();
  private knownInviteIds = new Set<string>();
  private hasInitializedBaseline = false;

  public visibleIntervalMs: number;
  public hiddenIntervalMs: number;
  public immediateRefreshThresholdMs: number;
  public onNotification?: (notification: FriendLiveNotification) => void;
  public onJoinLobby?: (code: string) => void;
  private customSyncFn?: FriendsLiveOptions['syncFn'];
  private boundVisibilityHandler: (() => void) | null = null;

  constructor(options: FriendsLiveOptions = {}) {
    this.visibleIntervalMs = options.visibleIntervalMs ?? 25_000;
    this.hiddenIntervalMs = options.hiddenIntervalMs ?? 120_000;
    this.immediateRefreshThresholdMs = options.immediateRefreshThresholdMs ?? 20_000;
    this.onNotification = options.onNotification;
    this.onJoinLobby = options.onJoinLobby;
    this.customSyncFn = options.syncFn;
    this.activePollIntervalMs = this.visibleIntervalMs;
  }

  public isWindowVisible(): boolean {
    if (typeof document !== 'undefined' && typeof document.visibilityState !== 'undefined') {
      return document.visibilityState === 'visible';
    }
    return true;
  }

  public startPolling() {
    if (this.isPolling) return;
    this.isPolling = true;

    // Attach visibility change listener
    if (typeof document !== 'undefined') {
      this.boundVisibilityHandler = () => this.handleVisibilityChange();
      document.addEventListener('visibilitychange', this.boundVisibilityHandler);
    }

    this.rescheduleTimer();
    // Immediate initial sync
    this.sync().catch(() => {});
  }

  public stopPolling() {
    this.isPolling = false;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    if (typeof document !== 'undefined' && this.boundVisibilityHandler) {
      document.removeEventListener('visibilitychange', this.boundVisibilityHandler);
      this.boundVisibilityHandler = null;
    }
  }

  public handleVisibilityChange() {
    const isVisible = this.isWindowVisible();
    const newInterval = isVisible ? this.visibleIntervalMs : this.hiddenIntervalMs;
    this.activePollIntervalMs = newInterval;

    if (isVisible) {
      const now = Date.now();
      const elapsed = now - this.lastSyncTime;
      if (elapsed >= this.immediateRefreshThresholdMs) {
        this.sync().catch(() => {});
      }
    }

    this.rescheduleTimer();
  }

  private rescheduleTimer() {
    if (!this.isPolling) return;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }

    const interval = this.isWindowVisible() ? this.visibleIntervalMs : this.hiddenIntervalMs;
    this.activePollIntervalMs = interval;

    this.timer = setTimeout(async () => {
      if (!this.isPolling) return;
      try {
        await this.sync();
      } finally {
        this.rescheduleTimer();
      }
    }, interval);
  }

  public async sync(): Promise<void> {
    if (this.isSyncing) return;
    this.isSyncing = true;

    try {
      let result: {
        friends?: FriendItem[];
        incoming_requests?: FriendRequestItem[];
        incoming_invites?: LobbyInviteItem[];
      };

      if (this.customSyncFn) {
        result = await this.customSyncFn();
      } else {
        const isTauri = typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
        if (isTauri) {
          const { invoke } = await import('@tauri-apps/api/core');
          result = await invoke('social_sync', {});
        } else {
          // In test/browser mock environment retain current lists
          result = {
            friends: this.friends,
            incoming_requests: this.incomingRequests,
            incoming_invites: this.incomingInvites,
          };
        }
      }

      this.processSyncResult(result);
      this.lastSyncTime = Date.now();
    } catch {
      // Offline fallback: retain local snapshot
    } finally {
      this.isSyncing = false;
    }
  }

  public processSyncResult(result: {
    friends?: FriendItem[];
    incoming_requests?: FriendRequestItem[];
    incoming_invites?: LobbyInviteItem[];
  }) {
    const newFriends = result.friends ?? [];
    const newRequests = result.incoming_requests ?? [];
    const newInvites = result.incoming_invites ?? [];

    if (!this.hasInitializedBaseline) {
      // First baseline: populate sets without spamming notifications
      for (const f of newFriends) {
        this.knownFriendStatuses.set(f.id, f.status);
      }
      for (const inv of newInvites) {
        this.knownInviteIds.add(inv.id);
      }
      this.hasInitializedBaseline = true;
    } else {
      // Detect friends transitioning to "playing"
      for (const f of newFriends) {
        const prevStatus = this.knownFriendStatuses.get(f.id);
        if (prevStatus && prevStatus !== 'playing' && f.status === 'playing') {
          this.notifyFriendPlaying(f);
        }
        this.knownFriendStatuses.set(f.id, f.status);
      }

      // Detect incoming lobby invites
      for (const inv of newInvites) {
        if (!this.knownInviteIds.has(inv.id)) {
          this.knownInviteIds.add(inv.id);
          this.notifyInviteReceived(inv);
        }
      }
    }

    this.friends = newFriends;
    this.incomingRequests = newRequests;
    this.incomingInvites = newInvites;
  }

  private notifyFriendPlaying(friend: FriendItem) {
    const isEn = i18n.locale === 'en';
    const serverDetail = friend.world_name
      ? (isEn ? `is playing on server ${friend.world_name}` : `играет на сервере ${friend.world_name}`)
      : friend.game_version
        ? (isEn ? `is playing ${friend.game_version}` : `в игре: ${friend.game_version}`)
        : (isEn ? 'is playing' : 'в игре');

    const title = isEn ? 'Friend joined game' : 'Друг зашёл в игру';
    const description = `${friend.nick} ${serverDetail}`;

    let action: FriendNotificationAction | undefined;
    if (friend.lobby_code) {
      action = {
        label: isEn ? 'Join' : 'Присоединиться',
        onclick: () => this.handleJoin(friend.lobby_code!),
      };
    }

    this.emitNotification({
      type: 'friend_playing',
      title,
      description,
      action,
    });
  }

  private notifyInviteReceived(invite: LobbyInviteItem) {
    const isEn = i18n.locale === 'en';
    const title = isEn ? 'You were invited' : 'Вас пригласили';
    const description = isEn
      ? `${invite.from_nick} invited you to play${invite.host_name ? ` (${invite.host_name})` : ''}`
      : `${invite.from_nick} зовёт вас в игру${invite.host_name ? ` (${invite.host_name})` : ''}`;

    const action: FriendNotificationAction = {
      label: isEn ? 'Join' : 'Присоединиться',
      onclick: () => this.handleJoin(invite.lobby_code),
    };

    this.emitNotification({
      type: 'invite_received',
      title,
      description,
      action,
    });
  }

  private emitNotification(notif: FriendLiveNotification) {
    // 1. In-app toast callback
    this.onNotification?.(notif);

    // 2. Windows Desktop Notification (WebView2 / browser Notification API)
    if (typeof window !== 'undefined' && 'Notification' in window) {
      if (Notification.permission === 'granted') {
        try {
          const n = new Notification(notif.title, {
            body: notif.description,
            icon: '/icons/icon.ico',
          });
          n.onclick = () => {
            if (typeof window.focus === 'function') {
              window.focus();
            }
            notif.action?.onclick();
          };
        } catch {
          // Headless or permission error ignored
        }
      } else if (Notification.permission === 'default') {
        Notification.requestPermission().catch(() => {});
      }
    }
  }

  public handleJoin(code: string) {
    const cleanCode = code.replace(/^#/, '').trim().toUpperCase();
    if (!cleanCode) return;
    this.onJoinLobby?.(cleanCode);
  }

  public removeRequest(requestId: string) {
    this.incomingRequests = this.incomingRequests.filter((r) => r.id !== requestId);
  }

  public removeInvite(inviteId: string) {
    this.incomingInvites = this.incomingInvites.filter((i) => i.id !== inviteId);
  }

  public addFriend(friend: FriendItem) {
    if (!this.friends.some((f) => f.id === friend.id)) {
      this.friends = [...this.friends, friend];
      this.knownFriendStatuses.set(friend.id, friend.status);
    }
  }
}

export const friendsLive = new FriendsLiveManager();
