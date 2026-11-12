<!--
  Worlds of the build (reference: Миры tab). Each world can be shown in its
  folder. «Играть» in a world is not offered: core cannot start straight into
  a singleplayer world. Empty state explains how to add a world.
  HIG › Lists and tables; HIG › Writing for the empty state.
-->
<script lang="ts">
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { formatAgo } from '../../shell/format';
  import { t, tn } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import { build, type WorldItem } from './build.svelte';

  const MODE: Record<string, string> = {
    Survival: 'build.mode_survival',
    Creative: 'build.mode_creative',
    Adventure: 'build.mode_adventure',
    Spectator: 'build.mode_spectator',
    '0': 'build.mode_survival',
    '1': 'build.mode_creative',
    '2': 'build.mode_adventure',
    '3': 'build.mode_spectator',
  };

  function detail(w: WorldItem): string {
    const parts: string[] = [];
    const mode = MODE[String(w.game_mode)];
    if (mode) parts.push(t(mode));
    const days = Math.floor(Number(w.day_time ?? 0) / 24000);
    if (days > 0) parts.push(tn('build.world_days', days));
    const when = formatAgo(w.last_played);
    if (when) parts.push(t('build.world_last', { date: when }));
    return parts.join(', ');
  }

  async function open(path: string | null) {
    if (!path) return;
    try {
      await call('workshop_open_path', { path });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }

  const savesDir = $derived.by(() => {
    const dir = build.instanceDir();
    return dir ? dir + (dir.includes('\\') ? '\\' : '/') + 'saves' : null;
  });
</script>

<div class="list">
  {#if build.worlds === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else if build.worlds.length}
    {#each build.worlds as w (w.folder_name)}
      <div class="lrow lrow-world">
        <Monogram name={w.display_name} shape="square" tone={3} />
        <div>
          <b>{w.display_name}</b>
          <span>{detail(w)}</span>
        </div>
        <button type="button" class="mini" onclick={() => open(w.folder_path)}>{t('build.show_in_folder')}</button>
      </div>
    {/each}
  {:else}
    <div class="empty">
      <b>{t('build.worlds_empty')}</b>
      {t('build.worlds_empty_hint')}
      {#if savesDir}<button type="button" class="mini" onclick={() => open(savesDir)}>{t('build.open_saves')}</button>{/if}
    </div>
  {/if}
</div>

<style>
  .lrow-world {
    grid-template-columns: 40px minmax(0, 1fr) auto;
  }
  .empty .mini {
    margin-left: 8px;
  }
</style>
