/**
 * Command palette (reference: CommandPalette). Searches actions, builds, servers,
 * friends, mods of the selected build and sections; arrows and Enter run them.
 * Every item comes from real data: list_servers, list_mods, friendsLive, builds.
 */

import { builds } from './builds.svelte';
import { launch } from './launch.svelte';
import { router, buildRoute, SECTIONS, type Section } from './router.svelte';
import { toasts } from './toast.svelte';
import { call } from './ipc';
import { formatError } from '../design/errors';
import { friendsLive } from '../services/friendsLive';
import { buildIntent } from '../pages/build/build.svelte';
import { t, tn } from '../i18n';

export interface CommandItem {
  group: string;
  title: string;
  sub?: string;
  kbd?: string;
  run: () => void;
}

interface ServerEntry {
  name: string;
  address: string;
}

interface ModEntry {
  name: string;
  version: string;
  enabled: boolean;
}

const SECTION_LABEL: Record<Section, string> = {
  home: 'shell.nav_home',
  servers: 'shell.nav_servers',
  friends: 'shell.nav_friends',
  wardrobe: 'shell.nav_wardrobe',
  options: 'shell.nav_options',
};

export const PALETTE_LIMIT = 12;

class Palette {
  open = $state(false);
  servers = $state<ServerEntry[]>([]);
  mods = $state<ModEntry[]>([]);
  private returnFocus: HTMLElement | null = null;

  show(): void {
    if (this.open) return;
    this.returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    this.open = true;
    void this.refresh();
  }

  hide(): void {
    if (!this.open) return;
    this.open = false;
    const el = this.returnFocus;
    this.returnFocus = null;
    if (el?.isConnected) queueMicrotask(() => el.focus());
  }

  /** Servers and the selected build's mods are fetched when the palette opens. */
  async refresh(): Promise<void> {
    const version = builds.selectedId;
    const [servers, mods] = await Promise.all([
      call<ServerEntry[]>('list_servers', { version: null }).catch(() => [] as ServerEntry[]),
      call<ModEntry[]>('list_mods', { version }).catch(() => [] as ModEntry[]),
    ]);
    this.servers = servers;
    this.mods = mods;
  }

  items(): CommandItem[] {
    const sel = builds.selectedId;
    const other = builds.list.find((b) => b.id !== sel);
    const g = {
      actions: t('cmd.group_actions'),
      builds: t('cmd.group_builds'),
      servers: t('cmd.group_servers'),
      friends: t('cmd.group_friends'),
      mods: t('cmd.group_mods'),
      sections: t('cmd.group_sections'),
    };
    const list: CommandItem[] = [];

    list.push(
      launch.phase === 'running'
        ? { group: g.actions, title: t('shell.stop_label'), kbd: t('cmd.kbd_enter'), run: () => void launch.stop() }
        : launch.phase === 'launching'
          ? { group: g.actions, title: t('shell.cancel_label'), kbd: t('cmd.kbd_enter'), run: () => void launch.cancel() }
          : { group: g.actions, title: t('shell.play_label', { ver: sel }), kbd: t('cmd.kbd_enter'), run: () => void launch.start() }
    );
    if (other && !launch.busy) {
      list.push({ group: g.actions, title: t('cmd.switch_build', { ver: other.id }), run: () => void selectBuild(other.id) });
    }
    list.push(
      { group: g.actions, title: t('cmd.open_build_folder'), run: () => void openBuildFolder(sel) },
      { group: g.actions, title: t('cmd.verify_build'), run: () => void verifyBuild(sel) }
    );

    for (const b of builds.list) {
      list.push({
        group: g.builds,
        title: t('shell.build_name', { ver: b.id }),
        sub: b.mods !== null ? tn('shell.mods', b.mods) : undefined,
        run: () => openBuild(b.id, null, ''),
      });
    }
    for (const s of this.servers) {
      list.push({ group: g.servers, title: s.name, sub: s.address, run: () => void launch.start(s.address) });
    }
    for (const f of friendsLive.friends) {
      list.push({ group: g.friends, title: f.nick, sub: t(f.status === 'offline' ? 'shell.offline' : 'shell.online'), run: () => router.go('friends') });
    }
    for (const m of this.mods) {
      list.push({
        group: g.mods,
        title: m.name,
        sub: [m.version, m.enabled ? t('cmd.mod_on') : t('cmd.mod_off')].filter(Boolean).join(', '),
        run: () => openBuild(sel, 'mods', m.name),
      });
    }
    for (const id of SECTIONS) {
      list.push({ group: g.sections, title: t(SECTION_LABEL[id]), run: () => router.go(id) });
    }
    return list;
  }

  /** Items matching the query (title, detail and group), at most PALETTE_LIMIT. */
  search(query: string): CommandItem[] {
    const q = query.trim().toLowerCase();
    return this.items()
      .filter((x) => !q || `${x.title} ${x.sub ?? ''} ${x.group}`.toLowerCase().includes(q))
      .slice(0, PALETTE_LIMIT);
  }
}

export const palette = new Palette();

export async function selectBuild(id: string): Promise<void> {
  if (launch.busy || id === builds.selectedId) return;
  try {
    await builds.select(id);
    toasts.show(t('shell.build_selected', { ver: id }));
  } catch (e) {
    toasts.show(t('shell.select_failed'), formatError(e, 'settings').description);
  }
}

function openBuild(id: string, tab: 'mods' | null, modQuery: string): void {
  buildIntent.tab = tab;
  buildIntent.modQuery = modQuery;
  const route = buildRoute(id);
  if (router.route === route) router.reopen();
  else router.go(route);
}

async function openBuildFolder(id: string): Promise<void> {
  try {
    const root = await call<string>('get_data_dir');
    const sep = root.includes('\\') ? '\\' : '/';
    await call('workshop_open_path', { path: [root.replace(/[\\/]+$/, ''), 'instances', id].join(sep) });
  } catch (e) {
    toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
  }
}

async function verifyBuild(id: string): Promise<void> {
  if (launch.busy) {
    toasts.show(t('cmd.verify_busy'));
    return;
  }
  toasts.show(t('build.verify_running'));
  try {
    const r = await call<{ status_message: string }>('verify_game_files', { version: id });
    toasts.show(t('build.verify_done'), r.status_message);
  } catch (e) {
    const f = formatError(e, 'install');
    toasts.show(f.title, f.description);
  }
}
