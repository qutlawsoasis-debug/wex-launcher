/**
 * Data for the home page, all from the backend:
 *   worlds per build   workshop_list_worlds(version)
 *   news               overview_get_news (the launcher changelog)
 *   play together      list_servers (the player's own servers) + ping_server_details,
 *                      list_lan_worlds (worlds opened to LAN nearby)
 *   friends online     friendsLive (social_sync), signed in by core with the active
 *                      account; friendsLive.connection says when it cannot be
 */

import { call } from '../../shell/ipc';
import { friendsLive, type FriendItem, type FriendsConnection } from '../../services/friendsLive';

export interface NewsItem {
  version: string;
  date: string;
  title: string;
  text: string;
}

interface ReleaseNote {
  version: string;
  date_text: string;
  short_title: string;
  items: Array<{ text: string }>;
}

export interface PlayServer {
  name: string;
  address: string;
}

export interface ServerPing {
  ok: boolean;
  online?: number;
  max?: number;
  ms?: number;
}

export interface LanWorld {
  motd: string;
  address: string;
  port: number;
}

/** How many saved servers Home shows; the rest are on the Servers page. */
export const HOME_SERVERS = 3;

class Home {
  worlds = $state<Record<string, number>>({});
  news = $state<NewsItem[]>([]);
  servers = $state<PlayServer[]>([]);
  /** Total saved servers (Home shows the first HOME_SERVERS). */
  serverCount = $state(0);
  pings = $state<Record<string, ServerPing>>({});
  lan = $state<LanWorld[]>([]);
  friends = $state<FriendItem[]>([]);
  /** True once friendsLive has delivered at least one sync with friends data. */
  friendsKnown = $state(false);
  /** Friends sign-in with the active account (signed in, connecting, unreachable…). */
  connection = $state<FriendsConnection>({ state: 'connecting' });

  private stopFriends: (() => void) | null = null;

  async load(buildIds: string[]): Promise<void> {
    await Promise.all([this.loadWorlds(buildIds), this.loadNews(), this.loadTogether()]);
  }

  async loadWorlds(buildIds: string[]): Promise<void> {
    const entries = await Promise.all(
      buildIds.map(async (id) => {
        try {
          const list = await call<unknown[]>('workshop_list_worlds', { version: id });
          return [id, list.length] as const;
        } catch {
          return null;
        }
      })
    );
    const next: Record<string, number> = {};
    for (const e of entries) if (e) next[e[0]] = e[1];
    this.worlds = next;
  }

  async loadNews(): Promise<void> {
    try {
      const notes = await call<ReleaseNote[]>('overview_get_news');
      this.news = notes.slice(0, 2).map((n) => ({
        version: n.version,
        date: n.date_text,
        title: n.short_title,
        text: n.items.map((i) => i.text).filter(Boolean).slice(0, 2).join(' '),
      }));
    } catch {
      this.news = [];
    }
  }

  async loadTogether(): Promise<void> {
    const [servers, lan] = await Promise.all([
      call<PlayServer[]>('list_servers', { version: null }).catch(() => [] as PlayServer[]),
      call<LanWorld[]>('list_lan_worlds').catch(() => [] as LanWorld[]),
    ]);
    this.serverCount = servers?.length ?? 0;
    this.servers = (servers ?? []).slice(0, HOME_SERVERS).map((s) => ({ name: s.name, address: s.address }));
    this.lan = lan ?? [];
    await this.refreshPings();
  }

  async refreshPings(): Promise<void> {
    await Promise.all(
      this.servers.map(async (s) => {
        let p: ServerPing = { ok: false };
        try {
          const r = await call<{ success: boolean; latency_ms?: number | null; online?: number | null; max_players?: number | null }>('ping_server_details', {
            address: s.address,
            timeoutMs: null,
          });
          if (r.success) p = { ok: true, online: r.online ?? undefined, max: r.max_players ?? undefined, ms: r.latency_ms ?? undefined };
        } catch {}
        this.pings = { ...this.pings, [s.address]: p };
      })
    );
  }

  watchFriends(): () => void {
    this.stopFriends?.();
    this.friends = [...friendsLive.friends];
    this.friendsKnown = friendsLive.friends.length > 0;
    this.connection = { ...friendsLive.connection };
    const stopData = friendsLive.onData((data) => {
      this.friends = [...(data.friends ?? [])];
      this.friendsKnown = this.friends.length > 0;
    });
    const stopConn = friendsLive.onConnection((c) => (this.connection = { ...c }));
    this.stopFriends = () => {
      stopData();
      stopConn();
    };
    return () => {
      this.stopFriends?.();
      this.stopFriends = null;
    };
  }

  get friendsOnline(): FriendItem[] {
    return this.friends.filter((f) => f.status !== 'offline');
  }
}

export const home = new Home();
