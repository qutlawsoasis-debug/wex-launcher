<!--
  Apple HIG Popover Component
  Specification: tasks/apple/hig/popovers.md
  Rules:
  - Expose a small amount of information or functionality
  - Closes when clicking outside or pressing Escape
  - Smooth scale-in and fade transition
  - Compact, uncluttered card with subtle shadow
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open?: boolean;
    title?: string;
    width?: string;
    placement?: 'bottom-start' | 'bottom-end' | 'bottom' | 'top';
    onclose?: () => void;
    trigger?: Snippet;
    children?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    width = '260px',
    placement = 'bottom-end',
    onclose,
    trigger,
    children,
  }: Props = $props();

  let containerEl: HTMLElement | undefined = $state();

  function toggle() {
    open = !open;
    if (!open) onclose?.();
  }

  function close() {
    if (open) {
      open = false;
      onclose?.();
    }
  }

  function handleOutsideClick(e: MouseEvent) {
    if (open && containerEl && !containerEl.contains(e.target as Node)) {
      close();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onclick={handleOutsideClick} onkeydown={handleKeyDown} />

<div class="apple-popover-container" bind:this={containerEl}>
  {#if trigger}
    <div
      class="popover-trigger"
      onclick={toggle}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          toggle();
        }
      }}
      role="button"
      tabindex="0"
      aria-haspopup="dialog"
      aria-expanded={open}
    >
      {@render trigger()}
    </div>
  {/if}

  {#if open}
    <div
      class="popover-card {placement}"
      style="width: {width};"
      role="dialog"
      aria-modal="false"
      aria-label={title || 'Popover'}
    >
      {#if title}
        <div class="popover-header">
          <span class="popover-title">{title}</span>
        </div>
      {/if}

      <div class="popover-content">
        {@render children?.()}
      </div>
    </div>
  {/if}
</div>

<style>
  .apple-popover-container {
    position: relative;
    display: inline-block;
  }

  .popover-trigger {
    cursor: pointer;
    display: inline-flex;
  }

  .popover-card {
    position: absolute;
    z-index: 500;
    background-color: var(--card-bg);
    backdrop-filter: blur(24px) saturate(190%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--glass-shadow);
    padding: 12px;
    animation: popoverIn var(--motion-duration-short) var(--motion-spring-ease) forwards;
  }

  .popover-card.bottom-end {
    top: calc(100% + 8px);
    right: 0;
  }

  .popover-card.bottom-start {
    top: calc(100% + 8px);
    left: 0;
  }

  .popover-card.bottom {
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }

  .popover-card.top {
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }

  .popover-header {
    padding-bottom: 8px;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--color-separator);
  }

  .popover-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .popover-content {
    font-size: var(--text-footnote);
  }

  @keyframes popoverIn {
    from {
      opacity: 0;
      scale: 0.94;
    }
    to {
      opacity: 1;
      scale: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .popover-card {
      animation: none !important;
      scale: 1 !important;
    }
  }

  :global([data-effects="off"]) .popover-card,
  :global([data-reduce-motion="true"]) .popover-card {
    animation: none !important;
    scale: 1 !important;
  }
</style>
