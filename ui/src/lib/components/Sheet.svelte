<!--
  Apple HIG Sheet Component
  Specification: tasks/apple/hig/sheets.md
  Rules:
  - Modal card view with rounded corners floating over parent window
  - Parent window is dimmed with translucent scrim
  - Cancel on leading edge, Done on trailing edge, Title in center
  - Grabber at top edge
  - Escape key dismisses sheet
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Button from './Button.svelte';

  interface Props {
    open?: boolean;
    title?: string;
    cancelLabel?: string;
    doneLabel?: string;
    showCancel?: boolean;
    showDone?: boolean;
    maxWidth?: string;
    onclose?: () => void;
    ondone?: () => void;
    children?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    cancelLabel = 'Отмена',
    doneLabel = 'Готово',
    showCancel = true,
    showDone = true,
    maxWidth = '540px',
    onclose,
    ondone,
    children,
  }: Props = $props();

  function close() {
    open = false;
    onclose?.();
  }

  function handleDone() {
    ondone?.();
    close();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
  <div
    class="sheet-scrim"
    onclick={close}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') close();
    }}
    role="presentation"
    tabindex="-1"
  >
    <div
      class="sheet-card"
      style="max-width: {maxWidth};"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label={title || 'Sheet'}
      tabindex="-1"
    >
      <div class="grabber-area">
        <div class="grabber"></div>
      </div>

      <div class="sheet-header">
        <div class="header-action leading">
          {#if showCancel}
            <Button variant="ghost" size="small" onclick={close}>
              {cancelLabel}
            </Button>
          {/if}
        </div>

        {#if title}
          <div class="header-title">{title}</div>
        {/if}

        <div class="header-action trailing">
          {#if showDone}
            <Button variant="primary" size="small" onclick={handleDone}>
              {doneLabel}
            </Button>
          {/if}
        </div>
      </div>

      <div class="sheet-body">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  .sheet-scrim {
    position: fixed;
    inset: 0;
    background-color: var(--color-dim-scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 1000;
    backdrop-filter: blur(8px);
    animation: fadeIn var(--motion-duration-short) ease forwards;
  }

  .sheet-card {
    position: relative;
    width: 100%;
    background-color: var(--color-secondary-system-background);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sheet);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45), 0 4px 12px rgba(0, 0, 0, 0.2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideUp var(--motion-duration-sheet) var(--motion-spring-ease) forwards;
    outline: none;
  }

  .grabber-area {
    display: flex;
    justify-content: center;
    padding-top: 8px;
  }

  .grabber {
    width: 36px;
    height: 5px;
    border-radius: var(--radius-pill);
    background-color: var(--color-tertiary-system-fill);
  }

  .sheet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-separator);
  }

  .header-action {
    display: flex;
    align-items: center;
    min-width: 60px;
  }

  .header-action.trailing {
    justify-content: flex-end;
  }

  .header-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    text-align: center;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sheet-body {
    padding: 20px;
    max-height: 70vh;
    overflow-y: auto;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes slideUp {
    from {
      transform: translateY(30px) scale(0.97);
      opacity: 0;
    }
    to {
      transform: translateY(0) scale(1);
      opacity: 1;
    }
  }
</style>
