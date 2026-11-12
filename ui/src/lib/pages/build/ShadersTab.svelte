<!--
  Shader packs of the build (workshop tab). Only one can be active, so the list
  is a single choice with a checkmark, «Без шейдеров» first.
  HIG › Lists and tables: a checkmark marks the selected row in a single-choice list.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { onFileDrop } from '../../shell/fileDrop';
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { formatSize } from '../../shell/format';
  import { t, tn } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import { build } from './build.svelte';

  let over = $state(false);
  let busy = $state(false);

  const active = $derived((build.shaders ?? []).find((s) => s.is_active)?.file_name ?? null);

  async function choose(fileName: string | null) {
    if (busy || fileName === active) return;
    busy = true;
    try {
      await build.setShader(fileName);
    } catch (e) {
      const f = formatError(e, 'workshop');
      toasts.show(f.title, f.description);
    } finally {
      busy = false;
    }
  }

  async function openFolder() {
    try {
      await call('workshop_open_packs_folder', { packType: 'shaderpack', version: build.version });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }

  onMount(() =>
    onFileDrop({
      hover: (v) => (over = v),
      drop: async (paths) => {
        if (!paths.some((p) => /\.zip$/i.test(p))) {
          toasts.show(t('build.drop_need_zip'));
          return;
        }
        try {
          toasts.show(tn('build.shaders_added', await build.addPacks(paths, 'shaderpack')));
        } catch (e) {
          const f = formatError(e, 'workshop');
          toasts.show(f.title, f.description);
        }
      },
    })
  );
</script>

<div class="tools">
  <span class="grow"></span>
  <button type="button" class="btn" onclick={openFolder}>{t('build.shaders_folder')}</button>
</div>

<div class="list" class:over role="radiogroup" aria-label={t('build.tab_shaders')}>
  {#if build.shaders === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else}
    <button type="button" class="lrow lrow-pick" role="radio" aria-checked={active === null} onclick={() => choose(null)}>
      <span class="none" aria-hidden="true"></span>
      <div><b>{t('build.shaders_off')}</b><span>{t('build.shaders_off_hint')}</span></div>
      <svg class="check" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3L13 4.5" /></svg>
    </button>
    {#each build.shaders as s (s.file_name)}
      <button type="button" class="lrow lrow-pick" role="radio" aria-checked={s.is_active === true} onclick={() => choose(s.file_name)}>
        <Monogram name={s.name} shape="square" tone={(s.name.length % 5) + 1} />
        <div>
          <b>{s.name}{#if s.is_from_pack}<span class="tag in">{t('build.from_pack')}</span>{/if}</b>
          <span>{[s.description, formatSize(s.size_bytes)].filter(Boolean).join(', ')}</span>
        </div>
        <svg class="check" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3L13 4.5" /></svg>
      </button>
    {/each}
    {#if !build.shaders.length}
      <p class="empty">{t('build.shaders_drop_hint')}</p>
    {/if}
  {/if}
</div>
{#if build.shaders?.length}<p class="hint">{t('build.shaders_drop_hint')}</p>{/if}

<style>
  .grow {
    flex: 1;
  }
  .lrow-pick {
    width: 100%;
    text-align: left;
    grid-template-columns: 40px minmax(0, 1fr) 20px;
  }
  .none {
    width: 40px;
    height: 40px;
    border-radius: var(--r-s);
    box-shadow: inset 0 0 0 1.5px var(--separator);
  }
  .check {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: var(--blue-text);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0;
  }
  [aria-checked='true'] .check {
    opacity: 1;
  }
  .tag.in {
    margin-left: 8px;
    vertical-align: 1px;
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
