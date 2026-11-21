/**
 * Launch state for the launch bar, driven only by backend events:
 *   launch-status   idle | preparing | installing | launching | running | cancelled | error
 *   install-progress stage, percentage, downloaded/total bytes
 *   prewarm-status  idle | prewarming | ready (fast launch)
 * Actions call start_game, cancel_launch and shell_stop_game. Nothing is simulated.
 */

import { accounts } from './accounts.svelte';
import { openSignIn } from './accountUi.svelte';
import { call, on } from './ipc';
import { uiLog } from './uiLog';
import { builds, type Build } from './builds.svelte';
import { toasts } from './toast.svelte';
import { formatError } from '../design/errors';
import { t } from '../i18n';

export type PrewarmState = 'idle' | 'prewarming' | 'ready';
export type LaunchPhase = 'idle' | 'launching' | 'running';

interface LaunchStatusPayload {
  state: string;
  message: string;
  exit_code?: number | null;
}

interface InstallProgressPayload {
  stage: string;
  percentage: number;
  detail?: string;
  speed_bytes_per_sec?: number;
  total_bytes?: number;
  downloaded_bytes?: number;
}

interface PrewarmPayload {
  state: string;
  /** Build the state is about; older backends omit it. */
  version?: string;
  progress?: number;
}

interface LaunchStateDto {
  is_running: boolean;
  is_launching: boolean;
  running_version?: string | null;
}

class Launch {
  phase = $state<LaunchPhase>('idle');
  /** Current stage text from the backend. */
  stage = $state('');
  /** 0–100, or null while the backend has not reported progress for this stage. */
  percent = $state<number | null>(null);
  downloaded = $state(0);
  total = $state(0);
  /** Version that is launching or running (may differ from the selected build). */
  version = $state<string | null>(null);
  server = $state<string | null>(null);
  startedAt = $state<number | null>(null);
  /** Increments each time the game starts; the bar plays its green line on change. */
  startCount = $state(0);
  /** Fast-launch state per build: readiness must never be shown for the wrong one. */
  private prewarmMap = $state<Record<string, PrewarmState>>({});
  private prewarmEvents = 0;

  /** Fast-launch state of the selected build. */
  get prewarm(): PrewarmState {
    return this.prewarmMap[builds.selectedId] ?? 'idle';
  }
  set prewarm(v: PrewarmState) {
    this.prewarmMap[builds.selectedId] = v;
  }

  prewarmFor(version: string): PrewarmState {
    return this.prewarmMap[version] ?? 'idle';
  }

  private stops: Array<() => void> = [];

  get busy(): boolean {
    return this.phase !== 'idle';
  }

  attach(): () => void {
    this.detach();
    this.stops = [
      on<LaunchStatusPayload>('launch-status', (p) => this.onStatus(p)),
      on<InstallProgressPayload>('install-progress', (p) => this.onProgress(p)),
      on<PrewarmPayload>('prewarm-status', (p) => this.onPrewarm(p)),
    ];
    void this.restore();
    return () => this.detach();
  }

  detach(): void {
    for (const stop of this.stops) stop();
    this.stops = [];
  }

  /** Pick up a game that was already running or launching when the window opened. */
  async restore(): Promise<void> {
    try {
      const s = await call<LaunchStateDto>('shell_get_launch_state');
      if (s.is_running) this.enterRunning(s.running_version ?? builds.selectedId, false);
      else if (s.is_launching) this.enterLaunching(s.running_version ?? builds.selectedId, null);
    } catch {}
    await this.refreshPrewarm();
  }

  /** Ask the backend about the selected build; an event that arrives meanwhile wins. */
  async refreshPrewarm(): Promise<void> {
    const version = builds.selectedId;
    const seen = this.prewarmEvents;
    try {
      const p = await call<{ is_prewarming: boolean; is_ready: boolean }>('get_prewarm_status', { version });
      if (seen !== this.prewarmEvents) return;
      this.prewarmMap[version] = p.is_ready ? 'ready' : p.is_prewarming ? 'prewarming' : 'idle';
    } catch {}
  }

  async start(server: string | null = null): Promise<void> {
    if (this.busy) return;
    // Nobody to play as yet: sign in first (Play stays the way into the game).
    if (accounts.loaded && !accounts.error && !accounts.list.length) {
      openSignIn();
      return;
    }
    const version = builds.selectedId;
    this.enterLaunching(version, server);
    uiLog('info', `Play pressed: ${version}${server ? ` -> ${server}` : ''}`);
    try {
      await call('start_game', { version, nickname: builds.playerName, jvmArgs: null, server });
      uiLog('info', `start_game ${version} returned, phase ${this.phase}`);
      if (this.phase === 'launching') this.enterRunning(version, true);
    } catch (e) {
      uiLog('error', `start_game ${version} failed: ${String(e)}`);
      if (this.phase === 'launching') this.fail(e);
    }
  }

  async cancel(): Promise<void> {
    if (this.phase !== 'launching') return;
    try {
      await call('cancel_launch');
    } catch (e) {
      const f = formatError(e, 'launch');
      toasts.show(f.title, f.description);
    }
  }

  async stop(): Promise<void> {
    if (this.phase !== 'running') return;
    try {
      await call('shell_stop_game', { version: this.version });
    } catch (e) {
      const f = formatError(e, 'launch');
      toasts.show(t('launch.stop_error'), f.description);
    }
  }

  onStatus(p: LaunchStatusPayload): void {
    uiLog(p.state === 'error' ? 'error' : 'info', `launch-status ${p.state}: ${p.message}${p.exit_code != null ? ` (exit ${p.exit_code})` : ''}; phase ${this.phase}`);
    switch (p.state) {
      case 'preparing':
      case 'installing':
      case 'launching':
        if (this.phase !== 'launching') this.enterLaunching(this.version ?? builds.selectedId, this.server);
        if (p.message) this.setStage(p.message);
        break;
      case 'running':
        this.enterRunning(this.version ?? builds.selectedId, true);
        break;
      case 'cancelled':
        if (this.phase !== 'idle') toasts.show(p.message || t('shell.launch_cancelled'));
        this.reset();
        break;
      case 'error':
        if (this.phase !== 'idle') this.fail(p.message);
        break;
      case 'idle':
        if (this.phase === 'running') toasts.show(p.message || t('app.game_stopped'));
        this.reset();
        break;
    }
  }

  onProgress(p: InstallProgressPayload): void {
    // Background preparation also reports progress; the bar shows it only for a launch.
    if (this.phase !== 'launching') return;
    if (p.stage && p.stage !== this.stage) uiLog('info', `progress stage: ${p.stage}`);
    if (p.stage) this.setStage(p.stage);
    this.percent = Number.isFinite(p.percentage) ? Math.max(0, Math.min(100, p.percentage)) : null;
    this.downloaded = p.downloaded_bytes ?? 0;
    this.total = p.total_bytes ?? 0;
  }

  onPrewarm(p: PrewarmPayload): void {
    this.prewarmEvents++;
    const state: PrewarmState = p.state === 'ready' ? 'ready' : p.state === 'prewarming' ? 'prewarming' : 'idle';
    if (p.version) this.prewarmMap[p.version] = state;
    else if (state === 'idle') {
      // The backend holds one prepared game: stopping it clears every build.
      for (const k of Object.keys(this.prewarmMap)) this.prewarmMap[k] = 'idle';
    } else this.prewarm = state;
  }

  private setStage(text: string): void {
    if (text !== this.stage) {
      this.stage = text;
      this.percent = null;
      this.downloaded = 0;
      this.total = 0;
    }
  }

  private enterLaunching(version: string, server: string | null): void {
    this.phase = 'launching';
    this.version = version;
    this.server = server;
    this.stage = t('shell.launch_preparing');
    this.percent = null;
    this.downloaded = 0;
    this.total = 0;
    this.startedAt = null;
  }

  private enterRunning(version: string, fresh: boolean): void {
    if (this.phase === 'running') return;
    this.phase = 'running';
    this.version = version;
    this.startedAt = Date.now();
    if (fresh) this.startCount++;
    // A launch may have just downloaded the game and its mods: read them again.
    void builds.load();
  }

  private fail(error: unknown): void {
    const f = formatError(error, 'launch');
    toasts.show(f.title, f.description);
    this.reset();
  }

  private reset(): void {
    // After a launch attempt or a session the files may have changed (installed, mods updated).
    if (this.phase !== 'idle') void builds.load();
    this.phase = 'idle';
    this.stage = '';
    this.percent = null;
    this.downloaded = 0;
    this.total = 0;
    this.server = null;
    this.startedAt = null;
  }
}

/**
 * Whether a build can start right now and, if not quite, what Play will do:
 * fast launch prepared, game files present, or the first press downloads them.
 * Shown wherever Play is, so the button never stands without a reason.
 */
export function readiness(b: Build): { ok: boolean; text: string } {
  if (launch.prewarmFor(b.id) === 'ready') return { ok: true, text: t('home.ready_fast') };
  if (b.installed) return { ok: true, text: t('home.ready_installed') };
  return { ok: false, text: t('home.ready_first_download') };
}

export const launch = new Launch();
// Selecting a build or re-reading the list: ask about the build that is selected now.
builds.onChange(() => void launch.refreshPrewarm());
