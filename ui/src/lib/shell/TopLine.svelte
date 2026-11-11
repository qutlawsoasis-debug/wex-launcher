<!--
  Top line (reference: .topline): back/forward, room for search, window buttons.
  The empty area is the window drag region (frameless window, decorations: false).
  HIG › Toolbars: navigation on the leading side, window actions trailing;
  disabled back/forward when there is nowhere to go.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { router } from './router.svelte';
  import { call } from './ipc';
  import { t } from '../i18n';

  interface Props {
    /** Centre slot: the search field arrives with the command palette. */
    center?: Snippet;
  }

  let { center }: Props = $props();
  let maximized = $state(false);

  async function minimize() {
    try {
      await call('shell_minimize_window');
    } catch {}
  }

  async function toggleMaximize() {
    try {
      maximized = await call<boolean>('shell_toggle_maximize_window');
    } catch {}
  }

  async function close() {
    try {
      await call('shell_close_window');
    } catch {}
  }
</script>

<div class="topline" data-tauri-drag-region>
  <div class="crumbs">
    <button type="button" aria-label={t('shell.back')} title={t('shell.back')} disabled={!router.canBack} onclick={() => router.goBack()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
    </button>
    <button type="button" aria-label={t('shell.forward')} title={t('shell.forward')} disabled={!router.canForward} onclick={() => router.goForward()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3l5 5-5 5" /></svg>
    </button>
  </div>
  {@render center?.()}
  <div class="winctl" role="group" aria-label={t('shell.window')}>
    <button type="button" aria-label={t('app.minimize_window')} title={t('app.minimize_window')} onclick={minimize}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 6h8" /></svg>
    </button>
    <button
      type="button"
      aria-label={maximized ? t('app.restore_window') : t('app.maximize_window')}
      title={maximized ? t('app.restore_window') : t('app.maximize_window')}
      onclick={toggleMaximize}
    >
      {#if maximized}
        <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2.5" y="4" width="5.5" height="5.5" /><path d="M4.5 4V2.5H9.5V7.5H8" /></svg>
      {:else}
        <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" /></svg>
      {/if}
    </button>
    <button type="button" class="close" aria-label={t('app.close_window')} title={t('app.close_window')} onclick={close}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" /></svg>
    </button>
  </div>
</div>

<style>
  .topline {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--base);
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 var(--gutter);
  }
  .crumbs {
    display: flex;
    gap: 4px;
  }
  .crumbs button {
    width: 32px;
    height: 32px;
    border-radius: var(--r-s);
    color: var(--label-2);
    display: grid;
    place-items: center;
    transition: background-color var(--dur-micro);
  }
  .crumbs button:hover:not([disabled]) {
    background: var(--fill-2);
    color: var(--label);
  }
  .crumbs button[disabled] {
    color: var(--label-3);
    cursor: default;
  }
  .crumbs svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  .winctl {
    margin-left: auto;
    display: flex;
    gap: 4px;
  }
  .winctl button {
    width: 36px;
    height: 28px;
    border-radius: var(--r-xs);
    color: var(--label-3);
    display: grid;
    place-items: center;
    transition:
      background-color var(--dur-micro),
      color var(--dur-micro);
  }
  .winctl button:hover {
    background: var(--fill-2);
    color: var(--label);
  }
  .winctl button.close:hover {
    background: var(--red);
    color: var(--blue-ink);
  }
  .winctl svg {
    width: 11px;
    height: 11px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }
  button {
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }
</style>
