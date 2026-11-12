/**
 * The two Wex builds (Minecraft 1.21.1 and 1.20.1) and the launcher settings the
 * shell needs. Everything comes from the backend: get_installed_versions,
 * list_mods, get_launcher_settings; selecting a build saves selected_version.
 */

import { call } from './ipc';

export interface Build {
  /** Minecraft version id, e.g. "1.21.1". */
  id: string;
  /** Java major version the build runs on; null when core does not report it. */
  java: number | null;
  installed: boolean;
  /** Mod count from list_mods; null until known or if the folder can't be read. */
  mods: number | null;
}

interface InstalledVersion {
  id: string;
  name: string;
  is_installed: boolean;
  /** Not sent by the current core; read when present. */
  required_java?: number;
}

export type LauncherSettings = Record<string, unknown> & {
  selected_version?: string;
  player_name?: string;
  ram_mb?: number;
};

/** CSS variable holding the build's cover colour (wex.css: --cover-1211, --cover-1201). */
export function coverVar(id: string): string {
  return `var(--cover-${id.replace(/\D/g, '')})`;
}

class Builds {
  list = $state<Build[]>([]);
  selectedId = $state('1.21.1');
  playerName = $state('');
  ramMb = $state<number | null>(null);
  /** The full launcher settings as last read or saved. */
  settings = $state<LauncherSettings>({});
  loaded = $state(false);
  error = $state<unknown>(null);

  get selected(): Build | undefined {
    return this.list.find((b) => b.id === this.selectedId) ?? this.list[0];
  }

  async load(): Promise<void> {
    try {
      const [settings, versions] = await Promise.all([
        call<LauncherSettings>('get_launcher_settings').catch(() => ({}) as LauncherSettings),
        call<InstalledVersion[]>('get_installed_versions'),
      ]);
      this.applySettings(settings);
      this.list = versions.map((v) => ({ id: v.id, java: typeof v.required_java === 'number' ? v.required_java : null, installed: v.is_installed, mods: null }));
      if (!this.list.some((b) => b.id === this.selectedId) && this.list[0]) {
        this.selectedId = this.list[0].id;
      }
      this.error = null;
      await Promise.all(this.list.map((b) => this.loadMods(b.id)));
    } catch (e) {
      this.error = e;
    } finally {
      this.loaded = true;
    }
  }

  applySettings(settings: LauncherSettings): void {
    this.settings = settings;
    if (typeof settings.selected_version === 'string' && settings.selected_version) {
      this.selectedId = settings.selected_version;
    }
    if (typeof settings.player_name === 'string' && settings.player_name) {
      this.playerName = settings.player_name;
    }
    if (typeof settings.ram_mb === 'number' && settings.ram_mb > 0) {
      this.ramMb = settings.ram_mb;
    }
  }

  async loadMods(id: string): Promise<void> {
    try {
      const mods = await call<unknown[]>('list_mods', { version: id });
      this.list = this.list.map((b) => (b.id === id ? { ...b, mods: mods.length } : b));
    } catch {
      // Mod count stays unknown; the sidebar then shows no count rather than a guess.
    }
  }

  /**
   * Save some fields of the launcher settings through core without touching the
   * others (get, merge, save). Returns the saved settings.
   */
  async saveSettings(patch: LauncherSettings): Promise<LauncherSettings> {
    const current = await call<LauncherSettings>('get_launcher_settings');
    const next = { ...current, ...patch };
    await call('save_launcher_settings', { newSettings: next });
    this.applySettings(next);
    return next;
  }

  /** Choose the build Play starts; saved to the launcher config. */
  async select(id: string): Promise<boolean> {
    if (id === this.selectedId || !this.list.some((b) => b.id === id)) return false;
    const previous = this.selectedId;
    this.selectedId = id;
    try {
      const settings = await call<LauncherSettings>('get_launcher_settings');
      await call('save_launcher_settings', { newSettings: { ...settings, selected_version: id } });
      return true;
    } catch (e) {
      this.selectedId = previous;
      throw e;
    }
  }
}

export const builds = new Builds();
