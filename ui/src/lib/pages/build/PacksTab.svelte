<!--
  Resource packs of the build (workshop, owner decision: a tab next to Mods/Worlds).
  Same list style as mods: toggle turns a pack on in options.txt.
  Adding: «Добавить ресурспак» (system file picker) or drop .zip files onto the window.
  HIG › Toggles; HIG › Lists and tables.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { onFileDrop } from '../../shell/fileDrop';
  import { pickFiles } from '../../shell/filePick';
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { formatSize } from '../../shell/format';
  import { t, tn } from '../../i18n';
  import ItemIcon from './ItemIcon.svelte';
  import { build, type PackItem } from './build.svelte';

  let over = $state(false);
  let busy = $state<string | null>(null);

  async function toggle(p: PackItem) {
    busy = p.file_name;
    try {
      await build.togglePack(p);
    } catch (e) {
      const f = formatError(e, 'workshop');
      toasts.show(f.title, f.description);
    } finally {
      busy = null;
    }
  }

  async function openFolder() {
    try {
      await call('workshop_open_packs_folder', { packType: 'resourcepack', version: build.version });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }

  async function add(paths: string[]) {
    if (!paths.some((p) => /\.zip$/i.test(p))) {
      if (paths.length) toasts.show(t('build.drop_need_zip'));
      return;
    }
    try {
      toasts.show(tn('build.packs_added', await build.addPacks(paths, 'resourcepack')));
    } catch (e) {
      const f = formatError(e, 'workshop');
      toasts.show(f.title, f.description);
    }
  }

  async function choose() {
    try {
      await add(await pickFiles({ title: t('build.add_pack'), filterName: t('build.filter_packs'), extensions: ['zip'] }));
    } catch (e) {
      toasts.show(t('build.pick_failed'), formatError(e, 'workshop').description);
    }
  }

  onMount(() => onFileDrop({ hover: (v) => (over = v), drop: (paths) => void add(paths) }));
</script>

<div class="tools">
  <span class="grow"></span>
  <button type="button" class="btn" onclick={openFolder}>{t('build.packs_folder')}</button>
  <button type="button" class="btn" onclick={choose}>{t('build.add_pack')}</button>
</div>

<div class="list" class:over>
  {#if build.packs === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else if build.packs.length}
    {#each build.packs as p (p.file_name)}
      <div class="lrow lrow-mod">
        <ItemIcon kind="pack" />
        <div>
          <b>{p.name}</b>
          <span>{[p.description, formatSize(p.size_bytes)].filter(Boolean).join(', ')}</span>
        </div>
        {#if p.is_from_pack}<span class="tag">{t('build.from_pack')}</span>{:else}<span></span>{/if}
        <button
          type="button"
          class="switch"
          role="switch"
          aria-checked={!!p.is_enabled}
          aria-label={p.name}
          disabled={busy === p.file_name}
          onclick={() => toggle(p)}
        ></button>
      </div>
    {/each}
  {:else}
    <p class="empty"><b>{t('build.packs_empty')}</b> {t('build.packs_drop_hint')}</p>
  {/if}
</div>
{#if build.packs?.length}<p class="hint">{t('build.packs_drop_also')}</p>{/if}

<style>
  .grow {
    flex: 1;
  }
  .lrow-mod {
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
  }
  .list.over {
    box-shadow: inset 0 0 0 2px var(--blue-text);
  }
  .hint {
    margin: 8px 0 0;
    font-size: var(--t-foot);
    color: var(--label-3);
  }
</style>
