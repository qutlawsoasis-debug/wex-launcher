<!--
  Mods of the build (reference: Моды tab). Search, All / Enabled, toggles.
  HIG › Toggles: systemGreen when on. HIG › Search fields: filters as you type.
  Adding: «Добавить мод» opens the system file picker (tauri-plugin-dialog);
  dropping .jar files onto the window works too.
  Not shown because core has no data: mod author, available updates, dependencies.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { indicator } from '../../shell/indicator';
  import { onFileDrop } from '../../shell/fileDrop';
  import { pickFiles } from '../../shell/filePick';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { t, tn } from '../../i18n';
  import ItemIcon from './ItemIcon.svelte';
  import { build, buildIntent, type ModEntry } from './build.svelte';

  let q = $state(buildIntent.modQuery);
  buildIntent.modQuery = '';
  let filter = $state<'all' | 'on'>('all');
  let over = $state(false);
  let busy = $state<string | null>(null);

  const shown = $derived.by(() => {
    const needle = q.trim().toLowerCase();
    return (build.mods ?? []).filter(
      (m) =>
        (filter === 'all' || m.enabled) &&
        (!needle || `${m.name} ${m.description} ${m.file_name}`.toLowerCase().includes(needle))
    );
  });

  async function toggle(m: ModEntry) {
    busy = m.file_name;
    try {
      await build.toggleMod(m);
    } catch (e) {
      const f = formatError(e, 'mods');
      toasts.show(f.title, f.description);
    } finally {
      busy = null;
    }
  }

  async function add(paths: string[]) {
    const jars = paths.filter((p) => /\.jar$/i.test(p));
    if (!jars.length) {
      if (paths.length) toasts.show(t('build.drop_need_jar'));
      return;
    }
    try {
      toasts.show(tn('build.mods_added', await build.addMods(jars)));
    } catch (e) {
      const f = formatError(e, 'mods');
      toasts.show(f.title, f.description);
    }
  }

  /** «Добавить мод»: the system open panel, .jar only, several at once. */
  async function choose() {
    try {
      await add(await pickFiles({ title: t('build.add_mod'), filterName: t('build.filter_mods'), extensions: ['jar'] }));
    } catch (e) {
      toasts.show(t('build.pick_failed'), formatError(e, 'mods').description);
    }
  }

  onMount(() => onFileDrop({ hover: (v) => (over = v), drop: (paths) => void add(paths) }));
</script>

<div class="tools">
  <label class="search">
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.5" /><path d="M10.5 10.5L14 14" /></svg>
    <input type="search" placeholder={t('build.mods_search')} aria-label={t('build.mods_search')} bind:value={q} />
  </label>
  <div class="seg" role="group" aria-label={t('build.mods_filter')} use:indicator={{ selector: '[aria-pressed="true"]', key: filter }}>
    <button type="button" aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>{t('build.filter_all')}</button>
    <button type="button" aria-pressed={filter === 'on'} onclick={() => (filter = 'on')}>{t('build.filter_on')}</button>
  </div>
  <button type="button" class="btn" onclick={choose}>{t('build.add_mod')}</button>
</div>

<div class="list" class:over>
  {#if build.mods === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else if shown.length}
    {#each shown as m (m.file_name)}
      <div class="lrow lrow-mod">
        <ItemIcon kind="mod" src={m.icon} />
        <div>
          <b>{m.name}</b>
          <span>{[m.version, m.description].filter(Boolean).join(', ')}</span>
        </div>
        {#if m.is_pack_mod}<span class="tag">{t('build.from_pack')}</span>{:else}<span></span>{/if}
        <button
          type="button"
          class="switch"
          role="switch"
          aria-checked={m.enabled}
          aria-label={m.name}
          disabled={busy === m.file_name}
          onclick={() => toggle(m)}
        ></button>
      </div>
    {/each}
  {:else if build.mods.length}
    <p class="empty"><b>{t('build.mods_none_found')}</b> {t('build.mods_none_found_hint')}</p>
  {:else}
    <p class="empty"><b>{t('build.mods_empty')}</b> {t('build.mods_drop_hint')}</p>
  {/if}
</div>
{#if build.mods?.length}<p class="hint">{t('build.mods_drop_also')}</p>{/if}

<style>
  .lrow-mod {
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
  }
  .list {
    transition: box-shadow var(--dur-micro);
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
