/**
 * «Облачные миры» (a Friends tab): friends' worlds that run 24/7 on the Aura
 * world manager, not on this PC. Real commands only (app/src-tauri/src/commands/cloud.rs):
 *   cloud_status (no network: is the address/secret set, which account owns)
 *   cloud_list / create / start / stop / share / delete
 * The owner is the active account; core reads it, the UI never sends it.
 * Polling: every 15 s while the tab is open, every 1.5 s while a world is
 * starting or stopping (same cadence as the old launcher).
 */

import { call } from '../../shell/ipc';
import { t } from '../../i18n';

export type CloudState = 'running' | 'starting' | 'stopping' | 'sleeping';

export interface CloudWorld {
  id: string;
  name: string;
  owner: string;
  seed: string | null;
  allowedPlayers: string[];
  state: CloudState;
  port: number | null;
  address: string | null;
  onlinePlayers: string[];
  maxPlayers: number;
  isOwner: boolean;
  stageText: string | null;
  progress: number;
  gameVersion: string;
  lastPlayed: number;
}

export interface CloudList {
  owner: string;
  maxActive: number;
  activeCount: number;
  worlds: CloudWorld[];
}

export interface CloudStatus {
  available: boolean;
  reason: string | null;
  owner: string | null;
  host: string | null;
}

interface StartView {
  worldId: string | null;
  state: CloudState;
  port: number | null;
  address: string | null;
}

export const CLOUD_VERSIONS = ['1.20.1', '1.21.1'] as const;
export const FAST_POLL_MS = 1500;
export const SLOW_POLL_MS = 15000;
export const PLAYER_RE = /^[A-Za-z0-9_]{3,16}$/;
export const MAX_NAME = 48;
export const MAX_SEED = 64;

function message(e: unknown, fallback: string): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') return (e as { message: string }).message;
  return fallback;
}

export function inTransit(w: CloudWorld): boolean {
  return w.state === 'starting' || w.state === 'stopping';
}

class CloudWorlds {
  status = $state<CloudStatus | null>(null);
  list = $state<CloudList | null>(null);
  /** World id → the action in flight («create» while creating). */
  busy = $state<Record<string, string>>({});
  error = $state('');
  private lastFetch = 0;

  get mine(): CloudWorld[] {
    return this.list?.worlds.filter((w) => w.isOwner) ?? [];
  }

  get shared(): CloudWorld[] {
    return this.list?.worlds.filter((w) => !w.isOwner) ?? [];
  }

  get transit(): boolean {
    return (this.list?.worlds ?? []).some(inTransit);
  }

  private setBusy(id: string, kind: string | null): void {
    const next = { ...this.busy };
    if (kind) next[id] = kind;
    else delete next[id];
    this.busy = next;
  }

  private patch(id: string, over: Partial<CloudWorld>): void {
    if (!this.list) return;
    this.list = { ...this.list, worlds: this.list.worlds.map((w) => (w.id === id ? { ...w, ...over } : w)) };
  }

  /** Status first (cheap); the list only when the feature is available. */
  async load(): Promise<void> {
    try {
      this.status = await call<CloudStatus>('cloud_status');
    } catch (e) {
      this.status = { available: false, reason: message(e, t('cloud.err_generic')), owner: null, host: null };
    }
    if (this.status?.available) await this.refresh();
  }

  async refresh(): Promise<void> {
    if (!this.status?.available) return;
    this.lastFetch = Date.now();
    try {
      this.list = await call<CloudList>('cloud_list');
      this.error = '';
    } catch (e) {
      this.error = message(e, t('cloud.err_list'));
      if (!this.list) this.list = { owner: this.status.owner ?? '', maxActive: 0, activeCount: 0, worlds: [] };
    }
  }

  /** Called every FAST_POLL_MS; fetches fast while a world is in transit, slowly otherwise. */
  async tick(now = Date.now()): Promise<void> {
    const due = this.transit ? FAST_POLL_MS : SLOW_POLL_MS;
    if (now - this.lastFetch >= due) await this.refresh();
  }

  private async act<T>(id: string, kind: string, run: () => Promise<T>): Promise<T | null> {
    this.setBusy(id, kind);
    this.error = '';
    try {
      return await run();
    } catch (e) {
      this.error = message(e, t('cloud.err_generic'));
      return null;
    } finally {
      this.setBusy(id, null);
    }
  }

  async create(name: string, version: string, seed: string): Promise<boolean> {
    const r = await this.act('create', 'create', () =>
      call<StartView>('cloud_create', { name: name.trim(), version, seed: seed.trim() || null })
    );
    if (!r) return false;
    await this.refresh();
    if (r.worldId && r.state !== 'running') this.patch(r.worldId, { state: r.state });
    return true;
  }

  async start(w: CloudWorld): Promise<StartView | null> {
    this.patch(w.id, { state: 'starting', progress: 0, stageText: null });
    const r = await this.act(w.id, 'start', () => call<StartView>('cloud_start', { id: w.id }));
    if (!r) {
      this.patch(w.id, { state: 'sleeping' });
      return null;
    }
    this.patch(w.id, { state: r.state, port: r.port, address: r.address, progress: r.state === 'running' ? 100 : 0 });
    this.lastFetch = 0;
    return r;
  }

  async stop(w: CloudWorld): Promise<boolean> {
    const prev = w.state;
    this.patch(w.id, { state: 'stopping', stageText: null });
    const ok = (await this.act(w.id, 'stop', () => call('cloud_stop', { id: w.id }).then(() => true))) ?? false;
    if (!ok) this.patch(w.id, { state: prev });
    this.lastFetch = 0;
    await this.refresh();
    return ok;
  }

  async share(w: CloudWorld, player: string, add: boolean): Promise<boolean> {
    const allowed = await this.act(w.id, 'share', () => call<string[]>('cloud_share', { id: w.id, player: player.trim(), add }));
    if (!allowed) return false;
    this.patch(w.id, { allowedPlayers: allowed });
    return true;
  }

  async remove(w: CloudWorld): Promise<boolean> {
    const ok = (await this.act(w.id, 'delete', () => call('cloud_delete', { id: w.id }).then(() => true))) ?? false;
    if (ok && this.list) this.list = { ...this.list, worlds: this.list.worlds.filter((x) => x.id !== w.id) };
    return ok;
  }
}

export const cloudWorlds = new CloudWorlds();

/** One line under the world's name: state, version and players (never colour alone). */
export function stateLabel(w: CloudWorld): string {
  return t(`cloud.state_${w.state}`);
}
