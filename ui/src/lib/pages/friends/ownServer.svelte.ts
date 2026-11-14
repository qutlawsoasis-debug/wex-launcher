/**
 * «Свой сервер» (a Friends tab): a dedicated Fabric server for the company of
 * friends, run by Wex on this PC. Real commands only (app/src-tauri/src/commands/own_server.rs):
 *   own_server_list / create / update / accept_eula / install (+ own-server-progress)
 *   own_server_start / stop / console (polled while the tab is open) / command / whitelist
 *   own_server_delete / add_to_servers
 * Friends connect by the LAN address, or by the internet address the host typed
 * after forwarding the port. Nothing here invents an address.
 */

import { call, on } from '../../shell/ipc';
import { t } from '../../i18n';

export type GameMode = 'survival' | 'creative' | 'adventure';
export type Difficulty = 'peaceful' | 'easy' | 'normal' | 'hard';
export type ServerStatus = 'stopped' | 'starting' | 'running' | 'stopping' | 'crashed';

export interface ServerSettings {
  name: string;
  maxPlayers: number;
  gamemode: GameMode;
  difficulty: Difficulty;
  whitelist: boolean;
  onlineMode: boolean;
  pvp: boolean;
  port: number;
  memoryMb: number;
  publicAddress: string | null;
  bindIp?: string | null;
}

export interface InstallInfo {
  minecraft: string;
  fabricLoader: string;
  packVersion: string | null;
  modSource: 'manifest' | 'fabricModJson' | 'noPackMods';
  mods: string[];
}

export interface OwnServer {
  id: string;
  version: string;
  createdAt: string;
  settings: ServerSettings;
  eulaAccepted: boolean;
  installed: InstallInfo | null;
  installing: boolean;
  status: ServerStatus;
  players: string[];
  whitelist: string[];
  lanAddress: string | null;
  localAddress: string;
  publicAddress: string | null;
  problem: string | null;
}

export interface LogLine {
  seq: number;
  text: string;
}

interface Snapshot {
  status: ServerStatus;
  players: string[];
  lines: LogLine[];
  nextSeq: number;
  problem: string | null;
}

interface Progress {
  id: string;
  stage: string;
  current: number;
  total: number;
}

export const VERSIONS = ['1.20.1', '1.21.1'] as const;
export const EULA_URL = 'https://aka.ms/MinecraftEULA';
const LOG_KEEP = 500;

export function defaultSettings(): ServerSettings {
  return {
    name: t('own.default_name'),
    maxPlayers: 8,
    gamemode: 'survival',
    difficulty: 'normal',
    whitelist: false,
    onlineMode: true,
    pvp: true,
    port: 25565,
    memoryMb: 3072,
    publicAddress: null,
  };
}

function message(e: unknown, fallback: string): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') return (e as { message: string }).message;
  return fallback;
}

class OwnServers {
  list = $state<OwnServer[] | null>(null);
  selectedId = $state<string | null>(null);
  busy = $state<string | null>(null);
  error = $state('');
  progress = $state<Progress | null>(null);
  log = $state<LogLine[]>([]);
  private since = 0;
  private logFor: string | null = null;
  private unlisten: (() => void) | null = null;

  get selected(): OwnServer | null {
    return this.list?.find((s) => s.id === this.selectedId) ?? null;
  }

  private put(view: OwnServer): void {
    const list = this.list ?? [];
    const i = list.findIndex((s) => s.id === view.id);
    this.list = i < 0 ? [...list, view] : list.map((s) => (s.id === view.id ? view : s));
  }

  listen(): () => void {
    this.unlisten?.();
    this.unlisten = on<Progress>('own-server-progress', (p) => (this.progress = p));
    return () => {
      this.unlisten?.();
      this.unlisten = null;
    };
  }

  async refresh(): Promise<void> {
    try {
      this.list = (await call<OwnServer[]>('own_server_list')) ?? [];
      if (!this.selected) this.selectedId = this.list[0]?.id ?? null;
    } catch (e) {
      this.list = [];
      this.error = message(e, t('own.err_generic'));
    }
  }

  select(id: string): void {
    this.selectedId = id;
    this.resetLog();
  }

  private resetLog(): void {
    this.log = [];
    this.since = 0;
    this.logFor = this.selectedId;
  }

  /** Runs a command; the returned view replaces the stored one. */
  private async act(kind: string, cmd: string, args: Record<string, unknown>): Promise<OwnServer | null> {
    this.busy = kind;
    this.error = '';
    try {
      const view = await call<OwnServer>(cmd, args);
      if (view) this.put(view);
      return view;
    } catch (e) {
      this.error = message(e, t('own.err_generic'));
      return null;
    } finally {
      this.busy = null;
    }
  }

  async create(version: string, settings: ServerSettings): Promise<boolean> {
    const view = await this.act('create', 'own_server_create', { version, settings });
    if (view) this.select(view.id);
    return !!view;
  }

  save(settings: ServerSettings) {
    return this.selectedId ? this.act('save', 'own_server_update', { id: this.selectedId, settings }) : null;
  }

  acceptEula() {
    return this.selectedId ? this.act('eula', 'own_server_accept_eula', { id: this.selectedId }) : null;
  }

  async install() {
    if (!this.selectedId) return null;
    this.progress = null;
    const id = this.selectedId;
    const list = this.list ?? [];
    this.list = list.map((s) => (s.id === id ? { ...s, installing: true } : s));
    const view = await this.act('install', 'own_server_install', { id });
    this.progress = null;
    if (!view) await this.refresh();
    return view;
  }

  async start() {
    if (!this.selectedId) return null;
    this.resetLog();
    return this.act('start', 'own_server_start', { id: this.selectedId });
  }

  stop() {
    return this.selectedId ? this.act('stop', 'own_server_stop', { id: this.selectedId }) : null;
  }

  async remove(): Promise<boolean> {
    const id = this.selectedId;
    if (!id) return false;
    this.busy = 'delete';
    this.error = '';
    try {
      await call('own_server_delete', { id });
      this.list = (this.list ?? []).filter((s) => s.id !== id);
      this.selectedId = this.list[0]?.id ?? null;
      this.resetLog();
      return true;
    } catch (e) {
      this.error = message(e, t('own.err_generic'));
      return false;
    } finally {
      this.busy = null;
    }
  }

  async addToServers(): Promise<boolean> {
    if (!this.selectedId) return false;
    this.error = '';
    try {
      await call('own_server_add_to_servers', { id: this.selectedId });
      return true;
    } catch (e) {
      this.error = message(e, t('own.err_generic'));
      return false;
    }
  }

  async send(command: string): Promise<boolean> {
    if (!this.selectedId || !command.trim()) return false;
    try {
      await call('own_server_command', { id: this.selectedId, command });
      await this.poll();
      return true;
    } catch (e) {
      this.error = message(e, t('own.err_generic'));
      return false;
    }
  }

  async whitelist(name: string, add: boolean): Promise<boolean> {
    if (!this.selectedId) return false;
    try {
      await call('own_server_whitelist', { id: this.selectedId, name, add });
      return true;
    } catch (e) {
      this.error = message(e, t('own.err_generic'));
      return false;
    }
  }

  /** Status, players and new log lines of the selected server. */
  async poll(): Promise<void> {
    const s = this.selected;
    if (!s) return;
    if (this.logFor !== s.id) this.resetLog();
    try {
      const snap = await call<Snapshot | null>('own_server_console', { id: s.id, since: this.since });
      if (!snap) return;
      this.since = snap.nextSeq;
      if (snap.lines.length) this.log = [...this.log, ...snap.lines].slice(-LOG_KEEP);
      const changed = snap.status !== s.status || snap.problem !== s.problem || snap.players.join() !== s.players.join();
      if (changed) {
        this.put({ ...s, status: snap.status, players: snap.players, problem: snap.problem });
        // Whitelist edits land in whitelist.json; pick them up with the next list.
        if (snap.status === 'stopped' || snap.status === 'crashed') void this.refresh();
      }
    } catch {
      /* polling is best effort */
    }
  }
}

export const ownServers = new OwnServers();

export function statusLabel(s: ServerStatus): string {
  return t(`own.status_${s}`);
}

/** The address friends should use, best first. */
export function friendAddress(s: OwnServer): string | null {
  return s.publicAddress ?? s.lanAddress;
}
