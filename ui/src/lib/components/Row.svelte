<!--
  Apple HIG Row Component
  Specification: tasks/apple/hig/lists-and-tables.md
  Rules:
  - Clean text hierarchy: title (bold/medium), subtitle (secondaryLabel)
  - Leading icon or thumbnail
  - Trailing accessory (chevron, badge, control, or secondary value)
  - Subtle hover highlight and active state
  - 1px indented separator line
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './icons/Icon.svelte';

  interface Props {
    title: string;
    subtitle?: string;
    trailingText?: string;
    showChevron?: boolean;
    clickable?: boolean;
    disabled?: boolean;
    index?: number;
    role?: string;
    ariaChecked?: boolean;
    ariaPressed?: boolean;
    ariaLabel?: string;
    onclick?: () => void;
    onkeydown?: (e: KeyboardEvent) => void;
    leading?: Snippet;
    trailing?: Snippet;
    children?: Snippet;
  }

  let {
    title,
    subtitle,
    trailingText,
    showChevron = false,
    clickable = false,
    disabled = false,
    index,
    role,
    ariaChecked,
    ariaPressed,
    ariaLabel,
    onclick,
    onkeydown,
    leading,
    trailing,
    children,
  }: Props = $props();

  const isInteractive = $derived(clickable || !!onclick);
  const cascadeDelayMs = $derived(index !== undefined ? Math.min(index, 7) * 25 : 0);
  const isCascade = $derived(index !== undefined);

  function handleKeyDown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === 'Enter' || e.key === ' ') {
      if (e.key === ' ') e.preventDefault();
      onclick?.();
    }
    onkeydown?.(e);
  }
</script>

{#if isInteractive}
  <button
    type="button"
    class="apple-row clickable"
    class:disabled
    class:cascade-in={isCascade}
    style:--cascade-delay="{cascadeDelayMs}ms"
    disabled={disabled}
    onclick={onclick}
    onkeydown={handleKeyDown}
    {role}
    aria-checked={ariaChecked}
    aria-pressed={ariaPressed}
    aria-label={ariaLabel}
  >
    {#if leading}
      <div class="row-leading">
        {@render leading()}
      </div>
    {/if}

    <div class="row-content">
      <div class="row-title-area">
        <span class="row-title">{title}</span>
        {#if subtitle}
          <span class="row-subtitle">{subtitle}</span>
        {/if}
        {@render children?.()}
      </div>

      {#if trailing || trailingText || showChevron}
        <div class="row-trailing">
          {#if trailing}
            {@render trailing()}
          {:else if trailingText}
            <span class="trailing-text">{trailingText}</span>
          {/if}

          {#if showChevron}
            <span class="chevron">
              <Icon name="chevron-right" size={14} color="var(--color-tertiary-label)" />
            </span>
          {/if}
        </div>
      {/if}
    </div>
  </button>
{:else}
  <div
    class="apple-row"
    class:disabled
    class:cascade-in={isCascade}
    style:--cascade-delay="{cascadeDelayMs}ms"
  >
    {#if leading}
      <div class="row-leading">
        {@render leading()}
      </div>
    {/if}

    <div class="row-content">
      <div class="row-title-area">
        <span class="row-title">{title}</span>
        {#if subtitle}
          <span class="row-subtitle">{subtitle}</span>
        {/if}
        {@render children?.()}
      </div>

      {#if trailing || trailingText || showChevron}
        <div class="row-trailing">
          {#if trailing}
            {@render trailing()}
          {:else if trailingText}
            <span class="trailing-text">{trailingText}</span>
          {/if}

          {#if showChevron}
            <span class="chevron">
              <Icon name="chevron-right" size={14} color="var(--color-tertiary-label)" />
            </span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .apple-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    min-height: 44px;
    background-color: transparent;
    font-family: var(--font-system);
    user-select: none;
    transition:
      background-color var(--motion-duration-hover) ease,
      transform var(--motion-duration-press) var(--motion-curve-press);
    border: none;
    border-bottom: 1px solid var(--color-separator);
    width: 100%;
    text-align: left;
    content-visibility: auto;
    contain-intrinsic-size: 52px;
    outline: none;
  }

  @keyframes rowCascadeIn {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .apple-row.cascade-in {
    animation: rowCascadeIn var(--motion-duration-normal, 240ms) var(--motion-curve-spring-gentle, cubic-bezier(0.16, 1, 0.3, 1)) both;
    animation-delay: var(--cascade-delay, 0ms);
  }

  .apple-row:last-child {
    border-bottom: none;
  }

  .apple-row.clickable {
    cursor: pointer;
  }

  .apple-row.clickable:hover:not(.disabled) {
    background-color: var(--color-quaternary-system-fill);
  }

  .apple-row.clickable:active:not(.disabled) {
    background-color: var(--color-tertiary-system-fill);
    transform: scale(var(--motion-press-scale, 0.97));
  }

  .apple-row.clickable:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: -2px;
    z-index: 1;
  }

  .apple-row.disabled {
    opacity: 0.42;
    cursor: not-allowed;
  }

  .row-leading {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .row-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-width: 0;
    gap: 12px;
  }

  .row-title-area {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .row-title {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-subtitle {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-weight: 500;
    line-height: var(--line-footnote);
  }

  .row-trailing {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .trailing-text {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
  }

  .chevron {
    display: flex;
    align-items: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .apple-row {
      transition: none !important;
    }
    .apple-row.cascade-in {
      animation: none !important;
    }
  }

  :global([data-reduce-motion="true"]) .apple-row {
    transition: none !important;
  }

  :global([data-reduce-motion="true"]) .apple-row.cascade-in {
    animation: none !important;
  }
</style>
