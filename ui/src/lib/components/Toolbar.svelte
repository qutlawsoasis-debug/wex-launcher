<!--
  Apple HIG Toolbar Component
  Specification: tasks/apple/hig/toolbars.md
  Rules:
  - Three distinct functional areas: leading, center, trailing
  - View title (under 15 characters, concise)
  - Window control actions and search placement
  - Concentric corner radii with child buttons
  - Frameless window drag-region support
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './icons/Icon.svelte';
  import { t } from '../i18n';

  interface Props {
    title?: string;
    showWindowControls?: boolean;
    isMaximized?: boolean;
    onminimize?: () => void;
    onmaximize?: () => void;
    onclose?: () => void;
    leading?: Snippet;
    center?: Snippet;
    trailing?: Snippet;
  }

  let {
    title,
    showWindowControls = true,
    isMaximized = false,
    onminimize,
    onmaximize,
    onclose,
    leading,
    center,
    trailing,
  }: Props = $props();

  let hasLeading = $derived(!!leading);
  let hasTrailing = $derived(!!trailing || showWindowControls);
</script>

<header class="apple-toolbar" data-tauri-drag-region>
  {#if hasLeading}
    <div class="toolbar-leading" data-tauri-drag-region>
      {@render leading?.()}
    </div>
  {/if}

  <div
    class="toolbar-center"
    class:standalone={!hasLeading && !hasTrailing}
    data-tauri-drag-region
  >
    {#if center}
      {@render center()}
    {:else if title}
      {#key title}
        <h1 class="toolbar-title" data-tauri-drag-region>{title}</h1>
      {/key}
    {/if}
  </div>

  {#if hasTrailing}
    <div class="toolbar-trailing" data-tauri-drag-region>
      {@render trailing?.()}

      {#if showWindowControls}
        <div class="window-controls">
          <button
            type="button"
            class="window-btn minimize"
            title={t('toolbar.minimize')}
            onclick={onminimize}
            aria-label={t('toolbar.minimize')}
          >
            <Icon name="minimize" size={12} color="var(--color-secondary-label)" />
          </button>

          <button
            type="button"
            class="window-btn maximize"
            title={isMaximized ? t('toolbar.restore') : t('toolbar.maximize')}
            onclick={onmaximize}
            aria-label={isMaximized ? t('toolbar.restore') : t('toolbar.maximize')}
          >
            <Icon name={isMaximized ? 'restore' : 'maximize'} size={11} color="var(--color-secondary-label)" />
          </button>

          <button
            type="button"
            class="window-btn close"
            title={t('toolbar.close')}
            onclick={onclose}
            aria-label={t('toolbar.close')}
          >
            <Icon name="close" size={13} color="var(--color-secondary-label)" />
          </button>
        </div>
      {/if}
    </div>
  {/if}
</header>

<style>
  .apple-toolbar {
    height: 52px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--glass-regular-bg);
    backdrop-filter: blur(20px) saturate(180%);
    border-bottom: 1px solid var(--glass-border);
    user-select: none;
    z-index: 20;
    gap: 16px;
    box-sizing: border-box;
  }

  .toolbar-leading {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }

  .toolbar-center {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 2;
    min-width: 0;
  }

  .toolbar-center.standalone {
    flex: 1;
    justify-content: center;
    width: 100%;
  }

  .toolbar-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    letter-spacing: var(--tracking-headline);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin: 0;
    animation: titleFadeIn var(--motion-duration-short) var(--motion-curve-spring-system) forwards;
  }

  @keyframes titleFadeIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  :global([data-effects="off"]) .toolbar-title,
  :global([data-reduce-motion="true"]) .toolbar-title {
    animation: none;
    transform: none !important;
  }

  .toolbar-center.standalone .toolbar-title {
    overflow: visible;
  }

  .toolbar-trailing {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
    -webkit-app-region: no-drag;
  }

  .window-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
    transition:
      background-color var(--motion-duration-short) ease,
      transform var(--motion-duration-press) var(--motion-curve-press);
  }

  .window-btn:active {
    transform: scale(var(--motion-press-scale, 0.97));
  }

  .window-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .window-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 1px;
  }

  .window-btn.close:hover {
    background-color: var(--color-system-red-accessible);
    color: #ffffff;
  }

  .window-btn.close:hover :global(svg) {
    stroke: #ffffff;
  }
</style>
