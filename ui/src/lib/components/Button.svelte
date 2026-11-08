<!--
  Apple HIG Button Component
  Specification: tasks/apple/hig/buttons.md
  Rules:
  - Prominent style (primary) uses system accent color (systemBlue)
  - Destructive style uses system red
  - Hit region >= 28x28 pt (macOS), accessible target
  - Smooth press state feedback scale(0.97)
  - Loading activity indicator state
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'destructive' | 'ghost';
    size?: 'small' | 'regular' | 'large';
    title?: string;
    disabled?: boolean;
    loading?: boolean;
    fullWidth?: boolean;
    type?: 'button' | 'submit' | 'reset';
    onclick?: (event: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'regular',
    title,
    disabled = false,
    loading = false,
    fullWidth = false,
    type = 'button',
    onclick,
    children,
  }: Props = $props();
</script>

<button
  {type}
  {title}
  class="apple-button {variant} {size}"
  class:loading
  class:full-width={fullWidth}
  disabled={disabled || loading}
  {onclick}
  aria-busy={loading}
>
  {#if loading}
    <span class="spinner" aria-hidden="true"></span>
  {/if}
  <span class="content" class:hidden-content={loading}>
    {@render children?.()}
  </span>
</button>

<style>
  .apple-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: none;
    cursor: pointer;
    font-family: var(--font-system);
    font-weight: 600;
    white-space: nowrap;
    text-align: center;
    user-select: none;
    transition:
      transform var(--motion-duration-short) var(--motion-spring-ease),
      background-color var(--motion-duration-short) ease,
      opacity var(--motion-duration-short) ease,
      box-shadow var(--motion-duration-short) ease;
    outline: none;
  }

  .apple-button:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.3);
  }

  /* Sizes */
  .apple-button.small {
    height: 28px;
    padding: 0 12px;
    font-size: var(--text-footnote);
    line-height: var(--line-footnote);
    border-radius: var(--radius-sm);
    min-width: 56px;
  }

  .apple-button.regular {
    height: 36px;
    padding: 0 16px;
    font-size: var(--text-subheadline);
    line-height: var(--line-subheadline);
    border-radius: var(--radius-md);
    min-width: 72px;
  }

  .apple-button.large {
    height: 48px;
    padding: 0 24px;
    font-size: var(--text-headline);
    line-height: var(--line-headline);
    border-radius: var(--radius-lg);
    min-width: 120px;
    letter-spacing: -0.2px;
  }

  .apple-button.full-width {
    width: 100%;
  }

  /* Variants */
  .apple-button.primary {
    background-color: var(--color-system-blue);
    color: var(--color-system-blue-on-accent);
    box-shadow: 0 2px 8px rgba(0, 145, 255, 0.28);
  }

  .apple-button.primary:hover:not(:disabled) {
    background-color: #1a9dff;
    box-shadow: 0 4px 14px rgba(0, 145, 255, 0.38);
  }

  .apple-button.primary:active:not(:disabled) {
    transform: scale(var(--motion-press-scale));
    background-color: #0077e6;
  }

  .apple-button.secondary {
    background-color: var(--color-system-fill);
    color: var(--color-label);
    border: 1px solid var(--glass-border);
  }

  .apple-button.secondary:hover:not(:disabled) {
    background-color: var(--color-secondary-system-fill);
  }

  .apple-button.secondary:active:not(:disabled) {
    transform: scale(var(--motion-press-scale));
    background-color: var(--color-tertiary-system-fill);
  }

  .apple-button.destructive {
    background-color: var(--color-system-red-accessible);
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(255, 69, 58, 0.25);
  }

  .apple-button.destructive:hover:not(:disabled) {
    background-color: #ff5449;
    box-shadow: 0 4px 14px rgba(255, 69, 58, 0.35);
  }

  .apple-button.destructive:active:not(:disabled) {
    transform: scale(var(--motion-press-scale));
    background-color: #e03228;
  }

  .apple-button.ghost {
    background-color: transparent;
    color: var(--color-system-blue);
  }

  .apple-button.ghost:hover:not(:disabled) {
    background-color: var(--color-quaternary-system-fill);
  }

  .apple-button.ghost:active:not(:disabled) {
    transform: scale(var(--motion-press-scale));
  }

  /* Disabled state */
  .apple-button:disabled {
    opacity: 0.42;
    cursor: not-allowed;
    transform: none !important;
    box-shadow: none !important;
  }

  /* Loading spinner */
  .spinner {
    position: absolute;
    width: 16px;
    height: 16px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .apple-button.large .spinner {
    width: 20px;
    height: 20px;
    border-width: 2.5px;
  }

  .hidden-content {
    visibility: hidden;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .apple-button {
      transition: none !important;
    }
    .apple-button:active:not(:disabled) {
      transform: none !important;
    }
    .spinner {
      animation: none !important;
      opacity: 0.85;
    }
  }

  :global([data-reduce-motion="true"]) .apple-button {
    transition: none !important;
  }

  :global([data-reduce-motion="true"]) .apple-button:active:not(:disabled) {
    transform: none !important;
  }

  :global([data-reduce-motion="true"]) .spinner {
    animation: none !important;
    opacity: 0.85;
  }
</style>
