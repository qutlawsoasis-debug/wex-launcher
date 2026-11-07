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
</script>

<header class="apple-toolbar" data-tauri-drag-region>
  <div class="toolbar-leading" data-tauri-drag-region>
    {@render leading?.()}
  </div>

  <div class="toolbar-center" data-tauri-drag-region>
    {#if center}
      {@render center()}
    {:else if title}
      <h1 class="toolbar-title" data-tauri-drag-region>{title}</h1>
    {/if}
  </div>

  <div class="toolbar-trailing" data-tauri-drag-region>
    {@render trailing?.()}

    {#if showWindowControls}
      <div class="window-controls">
        <button
          type="button"
          class="window-btn minimize"
          title="Свернуть"
          onclick={onminimize}
          aria-label="Свернуть окно"
        >
          <Icon name="minimize" size={12} color="var(--color-secondary-label)" />
        </button>

        <button
          type="button"
          class="window-btn maximize"
          title={isMaximized ? 'Восстановить' : 'Развернуть'}
          onclick={onmaximize}
          aria-label={isMaximized ? 'Восстановить окно' : 'Развернуть окно'}
        >
          <Icon name={isMaximized ? 'restore' : 'maximize'} size={11} color="var(--color-secondary-label)" />
        </button>

        <button
          type="button"
          class="window-btn close"
          title="Закрыть"
          onclick={onclose}
          aria-label="Закрыть окно"
        >
          <Icon name="close" size={13} color="var(--color-secondary-label)" />
        </button>
      </div>
    {/if}
  </div>
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

  .toolbar-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    letter-spacing: var(--tracking-headline);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    transition: background-color var(--motion-duration-short) ease;
  }

  .window-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .window-btn.close:hover {
    background-color: var(--color-system-red-accessible);
    color: #ffffff;
  }

  .window-btn.close:hover :global(svg) {
    stroke: #ffffff;
  }
</style>
