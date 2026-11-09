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
  import { t } from '../i18n';

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
    cancelLabel = t('common.cancel'),
    doneLabel = t('common.done'),
    showCancel = true,
    showDone = true,
    maxWidth = '540px',
    onclose,
    ondone,
    children,
  }: Props = $props();

  let cardEl: HTMLElement | undefined = $state();
  let previouslyFocusedEl: HTMLElement | null = null;

  function close() {
    open = false;
    onclose?.();
    const elToFocus = previouslyFocusedEl;
    if (elToFocus && typeof elToFocus.focus === 'function') {
      elToFocus.focus();
      setTimeout(() => {
        if (elToFocus && (document.activeElement === document.body || !document.activeElement)) {
          elToFocus.focus();
        }
      }, 10);
    }
  }

  function handleDone() {
    ondone?.();
    close();
  }

  $effect(() => {
    if (open) {
      if (typeof document !== 'undefined') {
        previouslyFocusedEl = document.activeElement as HTMLElement | null;
        setTimeout(() => {
          if (!cardEl) return;
          const focusables = cardEl.querySelectorAll<HTMLElement>(
            'input:not([disabled]), button:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
          );
          if (focusables.length > 0) {
            focusables[0].focus();
          } else {
            cardEl.focus();
          }
        }, 15);
      }
    }
  });

  function handleCardKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      close();
      return;
    }

    if (e.key === 'Tab' && cardEl) {
      const focusables = Array.from(
        cardEl.querySelectorAll<HTMLElement>(
          'input:not([disabled]), button:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
      ).filter((el) => el.offsetParent !== null || el.offsetWidth > 0 || el.offsetHeight > 0);

      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }

      const firstEl = focusables[0];
      const lastEl = focusables[focusables.length - 1];

      if (e.shiftKey) {
        if (document.activeElement === firstEl || !cardEl.contains(document.activeElement)) {
          e.preventDefault();
          lastEl.focus();
        }
      } else {
        if (document.activeElement === lastEl || !cardEl.contains(document.activeElement)) {
          e.preventDefault();
          firstEl.focus();
        }
      }
    }
  }

  function handleWindowKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeyDown} />

{#if open}
  <div
    class="sheet-scrim"
    onclick={close}
    role="presentation"
    tabindex="-1"
  >
    <div
      bind:this={cardEl}
      class="sheet-card"
      style="max-width: {maxWidth};"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleCardKeyDown}
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
    animation: scrimFadeIn var(--motion-duration-screen-enter) ease forwards;
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
    animation: sheetSlideUp var(--motion-duration-sheet-enter) var(--motion-curve-spring-system) forwards;
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

  @keyframes scrimFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes sheetSlideUp {
    from {
      transform: translateY(48px) scale(0.98);
      opacity: 0;
    }
    to {
      transform: translateY(0) scale(1);
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sheet-scrim {
      animation: scrimFadeIn var(--motion-duration-short) ease forwards !important;
    }
    .sheet-card {
      animation: scrimFadeIn var(--motion-duration-short) ease forwards !important;
      transform: none !important;
    }
  }

  :global([data-effects="off"]) .sheet-card,
  :global([data-reduce-motion="true"]) .sheet-card {
    animation: scrimFadeIn var(--motion-duration-short) ease forwards !important;
    transform: none !important;
  }

  :global([data-effects="off"]) .sheet-scrim,
  :global([data-reduce-motion="true"]) .sheet-scrim {
    animation: scrimFadeIn var(--motion-duration-short) ease forwards !important;
  }
</style>
