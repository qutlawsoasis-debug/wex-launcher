/**
 * Data for the home page, all from the backend:
 *   worlds per build   workshop_list_worlds(version)
 *   news               overview_get_news (the launcher changelog)
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

interface ReleaseNote {
  version: string;
  date_text: string;
  short_title: string;
  items: Array<{ text: string }>;
}

class Home {
  worlds = $state<Record<string, number>>({});
  news = $state<NewsItem[]>([]);
  friends = $state<FriendItem[]>([]);
  /** True once friendsLive has delivered at least one sync with friends data. */
  friendsKnown = $state(false);

  private stopFriends: (() => void) | null = null;

  async load(buildIds: string[]): Promise<void> {
    await Promise.all([this.loadWorlds(buildIds), this.loadNews()]);
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
