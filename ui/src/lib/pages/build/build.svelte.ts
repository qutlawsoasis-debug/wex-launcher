/**
 * Data for one build page, loaded per version from the backend:
 *   mods          list_mods / toggle_mod / add_custom_mod (dropped .jar)
 *   worlds        workshop_list_worlds
 *   resource packs, shader packs   workshop_list_resource_packs / workshop_list_shader_packs
 *   screenshots   workshop_list_screenshots
 *   backups       workshop_list_world_backups / create / restore
 *   data folder   get_data_dir (the build lives in <data>/instances/<version>)
 */

import { call } from '../../shell/ipc';

export type BuildTab = 'mods' | 'worlds' | 'packs' | 'shaders' | 'shots' | 'backups' | 'settings';
export const BUILD_TABS: readonly BuildTab[] = ['mods', 'worlds', 'packs', 'shaders', 'shots', 'backups', 'settings'];

export interface ModEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  file_name: string;
  enabled: boolean;
  is_pack_mod: boolean;
  /** The mod's own icon from its jar (a PNG data URL), or null. */
  icon?: string | null;
}

export type GameMode = 'Survival' | 'Creative' | 'Adventure' | 'Spectator' | 'Unknown' | number;

export interface WorldItem {
  folder_name: string;
  folder_path: string;
  display_name: string;
  game_mode: GameMode;
  last_played: number;
  /** In-game ticks; 24000 per Minecraft day. */
  day_time?: number;
  size_bytes: number;
}

export interface PackItem {
  name: string;
  file_name: string;
  file_path: string;
  size_bytes: number;
  description: string | null;
  is_enabled?: boolean;
  is_active?: boolean;
  is_from_pack: boolean;
}

export interface ShotItem {
  file_name: string;
  file_path: string;
  created_at: string | null;
  size_bytes: number;
}

export interface BackupItem {
  file_name: string;
  file_path: string;
  created_at: string | null;
  size_bytes: number;
}

/** Where the next build page should open (set by the command palette). Consumed once. */
export const buildIntent = $state<{ tab: BuildTab | null; modQuery: string }>({ tab: null, modQuery: '' });

class BuildData {
  version = $state('');
  mods = $state<ModEntry[] | null>(null);
  worlds = $state<WorldItem[] | null>(null);
  packs = $state<PackItem[] | null>(null);
  shaders = $state<PackItem[] | null>(null);
  shots = $state<ShotItem[] | null>(null);
  backups = $state<BackupItem[] | null>(null);
  backupWorld = $state<string | null>(null);
  dataDir = $state<string | null>(null);

  /** Switch to a version; drops data that belonged to the previous one. */
  open(version: string): void {
    if (version === this.version) return;
    this.version = version;
    this.mods = null;
    this.worlds = null;
    this.packs = null;
    this.shaders = null;
    this.shots = null;
    this.backups = null;
    this.backupWorld = null;
  }

  /** <data>/instances/<version>, built from the data root the backend reports. */
  instanceDir(): string | null {
    if (!this.dataDir) return null;
    const sep = this.dataDir.includes('\\') ? '\\' : '/';
    return [this.dataDir.replace(/[\\/]+$/, ''), 'instances', this.version].join(sep);
  }

  async loadBasics(): Promise<void> {
    await Promise.all([this.loadMods(), this.loadWorlds(), this.loadDataDir()]);
  }

  async loadDataDir(): Promise<void> {
    try {
      this.dataDir = await call<string>('get_data_dir');
    } catch {
      this.dataDir = null;
    }
  }

  async loadMods(): Promise<void> {
    const v = this.version;
    const list = await call<ModEntry[]>('list_mods', { version: v }).catch(() => []);
    if (v === this.version) this.mods = list;
  }

  async loadWorlds(): Promise<void> {
    const v = this.version;
    const list = await call<WorldItem[]>('workshop_list_worlds', { version: v }).catch(() => []);
    if (v === this.version) {
      this.worlds = [...list].sort((a, b) => Number(b.last_played) - Number(a.last_played));
      if (!this.backupWorld && this.worlds[0]) this.backupWorld = this.worlds[0].folder_name;
    }
  }

  async loadPacks(): Promise<void> {
    const v = this.version;
    const list = await call<PackItem[]>('workshop_list_resource_packs', { version: v }).catch(() => []);
    if (v === this.version) this.packs = list;
  }

  async loadShaders(): Promise<void> {
    const v = this.version;
    const list = await call<PackItem[]>('workshop_list_shader_packs', { version: v }).catch(() => []);
    if (v === this.version) this.shaders = list;
  }

  async loadShots(): Promise<void> {
    const v = this.version;
    const list = await call<ShotItem[]>('workshop_list_screenshots', { version: v }).catch(() => []);
    if (v === this.version) this.shots = list;
  }

  async loadBackups(): Promise<void> {
    const v = this.version;
    const world = this.worlds?.find((w) => w.folder_name === this.backupWorld);
    if (!world) {
      this.backups = [];
      return;
    }
    const list = await call<BackupItem[]>('workshop_list_world_backups', {
      version: v,
      worldFolderName: world.folder_name,
      displayName: world.display_name,
    }).catch(() => []);
    if (v === this.version && world.folder_name === this.backupWorld) this.backups = list;
  }

  async loadTab(tab: BuildTab): Promise<void> {
    if (tab === 'packs') await this.loadPacks();
    else if (tab === 'shaders') await this.loadShaders();
    else if (tab === 'shots') await this.loadShots();
    else if (tab === 'backups') {
      if (!this.worlds) await this.loadWorlds();
      await this.loadBackups();
    }
  }

  async toggleMod(mod: ModEntry): Promise<void> {
    const updated = await call<ModEntry>('toggle_mod', { version: this.version, fileName: mod.file_name, enabled: !mod.enabled });
    this.mods = (this.mods ?? []).map((m) => (m.file_name === mod.file_name ? { ...m, ...updated } : m));
  }

  async addMods(paths: string[]): Promise<number> {
    let added = 0;
    for (const p of paths.filter((x) => /\.jar$/i.test(x))) {
      const mod = await call<ModEntry>('add_custom_mod', { version: this.version, sourcePath: p });
      this.mods = [mod, ...(this.mods ?? []).filter((m) => m.file_name !== mod.file_name)];
      added++;
    }
    return added;
  }

  async togglePack(pack: PackItem): Promise<void> {
    const enabled = !pack.is_enabled;
    await call('workshop_toggle_resource_pack', { fileName: pack.file_name, enabled, version: this.version });
    this.packs = (this.packs ?? []).map((p) => (p.file_name === pack.file_name ? { ...p, is_enabled: enabled } : p));
  }

  async setShader(fileName: string | null): Promise<void> {
    await call('workshop_set_active_shader_pack', { shaderFileName: fileName, version: this.version });
    this.shaders = (this.shaders ?? []).map((s) => ({ ...s, is_active: s.file_name === fileName }));
  }

  async addPacks(paths: string[], packType: 'resourcepack' | 'shaderpack'): Promise<number> {
    let added = 0;
    for (const p of paths.filter((x) => /\.zip$/i.test(x))) {
      await call<string>('workshop_add_pack_file', { sourcePath: p, packType, version: this.version });
      added++;
    }
    if (added) await (packType === 'shaderpack' ? this.loadShaders() : this.loadPacks());
    return added;
  }

  async createBackup(): Promise<BackupItem | null> {
    const world = this.worlds?.find((w) => w.folder_name === this.backupWorld);
    if (!world) return null;
    const backup = await call<BackupItem>('workshop_create_world_backup', { worldFolder: world.folder_path, version: this.version });
    await this.loadBackups();
    return backup;
  }

  async restoreBackup(backup: BackupItem): Promise<void> {
    const world = this.worlds?.find((w) => w.folder_name === this.backupWorld);
    if (!world) return;
    await call('workshop_restore_world_backup', { backupPath: backup.file_path, targetWorldFolder: world.folder_path, version: this.version });
  }
}

export const build = new BuildData();
