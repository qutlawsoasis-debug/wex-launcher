/**
 * Data for the home page, all from the backend:
 *   worlds per build   workshop_list_worlds(version)
 *   news               overview_get_news (the launcher changelog)
 *   Aura server        list_servers → the mc.auramc.net entry, ping_server_details for players and ping
 *   friends online     friendsLive (social_sync); empty until accounts are connected
 */

import { call } from '../../shell/ipc';
import { friendsLive, type FriendItem } from '../../services/friendsLive';

export interface NewsItem {
  version: string;
  date: string;
  title: string;
  text: string;
}

export interface AuraServer {
  name: string;
  address: string;
}

export interface Ping {
  ok: boolean;
  online?: number;
  max?: number;
  ms?: number;
}

interface ServerEntry {
  id?: string;
  name: string;
  address: string;
}

interface ReleaseNote {
  version: string;
  date_text: string;
  short_title: string;
  items: Array<{ text: string }>;
}

interface PingDetails {
  success: boolean;
  latency_ms?: number | null;
  online?: number | null;
  max_players?: number | null;
}

/** The official server: the backend seeds it as "aura-main" at mc.auramc.net. */
export function findAuraServer(list: ServerEntry[]): AuraServer | null {
  const s = list.find((x) => x.id === 'aura-main') ?? list.find((x) => /(^|\.)mc\.auramc\.net(:\d+)?$/i.test(x.address.trim()));
  return s ? { name: s.name, address: s.address } : null;
}

class Home {
  worlds = $state<Record<string, number>>({});
  news = $state<NewsItem[]>([]);
  aura = $state<AuraServer | null>(null);
  ping = $state<Ping | null>(null);
  friends = $state<FriendItem[]>([]);
  /** True once friendsLive has delivered at least one sync with friends data. */
  friendsKnown = $state(false);

  private stopFriends: (() => void) | null = null;

  async load(buildIds: string[]): Promise<void> {
    await Promise.all([this.loadWorlds(buildIds), this.loadNews(), this.loadAura()]);
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

  async loadAura(): Promise<void> {
    try {
      this.aura = findAuraServer(await call<ServerEntry[]>('list_servers', { version: null }));
    } catch {
      this.aura = null;
    }
    await this.refreshPing();
  }

  async refreshPing(): Promise<void> {
    if (!this.aura) {
      this.ping = null;
      return;
    }
    try {
      const p = await call<PingDetails>('ping_server_details', { address: this.aura.address, timeoutMs: null });
      this.ping = p.success
        ? { ok: true, online: p.online ?? undefined, max: p.max_players ?? undefined, ms: p.latency_ms ?? undefined }
        : { ok: false };
    } catch {
      this.ping = { ok: false };
    }
  }

  watchFriends(): () => void {
    this.stopFriends?.();
    this.friends = [...friendsLive.friends];
    this.friendsKnown = friendsLive.friends.length > 0;
    this.stopFriends = friendsLive.onData((data) => {
      this.friends = [...(data.friends ?? [])];
      this.friendsKnown = this.friends.length > 0;
    });
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
