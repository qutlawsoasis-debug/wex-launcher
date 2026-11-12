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
    variant?: 'primary' | 'secondary' | 'destructive' | 'ghost' | 'glass';
    size?: 'small' | 'regular' | 'large';
    title?: string;
    disabled?: boolean;
    loading?: boolean;
    fullWidth?: boolean;
    type?: 'button' | 'submit' | 'reset';
    role?: string;
    ariaLabel?: string;
    ariaChecked?: boolean;
    ariaPressed?: boolean;
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
    role,
    ariaLabel,
    ariaChecked,
    ariaPressed,
    onclick,
    children,
  }: Props = $props();
</script>

<button
  {type}
  {title}
  {role}
  aria-label={ariaLabel || title}
  aria-checked={ariaChecked}
  aria-pressed={ariaPressed}
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
    font-family: var(--font);
    font-weight: 600;
    white-space: nowrap;
    text-align: center;
    user-select: none;
    transition:
      transform var(--dur-micro) var(--ease),
      background-color var(--dur-micro) ease,
      opacity var(--dur-micro) ease,
      box-shadow var(--dur-micro) ease;
    outline: none;
  }

  .apple-button:active:not(:disabled) {
    transition-duration: var(--dur-micro);
    transition-timing-function: var(--ease);
  }

  .apple-button:focus-visible {
    outline: 2px solid var(--blue);
    outline-offset: 2px;
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.3);
  }

  /* Sizes */
  .apple-button.small {
    height: 28px;
    padding: 0 12px;
    font-size: var(--t-foot);
    line-height: 1.35;
    border-radius: var(--r-s);
    min-width: 56px;
  }

  .apple-button.regular {
    height: 36px;
    padding: 0 16px;
    font-size: var(--t-sub);
    line-height: 1.35;
    border-radius: var(--r-m);
    min-width: 72px;
  }

  .apple-button.large {
    height: 48px;
    padding: 0 24px;
    font-size: var(--t-body);
    line-height: 1.35;
    border-radius: var(--r-l);
    min-width: 120px;
    letter-spacing: -0.2px;
  }

  .apple-button.full-width {
    width: 100%;
  }

  /* Variants */
  .apple-button.primary {
    background-color: var(--blue-text);
    color: var(--blue-ink);
    box-shadow: 0 2px 8px rgba(0, 145, 255, 0.28);
  }

  .apple-button.primary:hover:not(:disabled) {
    background-color: #1a9dff;
    box-shadow: 0 4px 14px rgba(0, 145, 255, 0.38);
  }

  .apple-button.primary:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: #0077e6;
  }

  .apple-button.secondary {
    background-color: var(--fill);
    color: var(--label);
    border: 1px solid var(--edge);
  }

  .apple-button.secondary:hover:not(:disabled) {
    background-color: var(--fill-2);
  }

  .apple-button.secondary:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: var(--fill-3);
  }

  .apple-button.destructive {
    background-color: var(--red);
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(255, 69, 58, 0.25);
  }

  .apple-button.destructive:hover:not(:disabled) {
    background-color: #ff5449;
    box-shadow: 0 4px 14px rgba(255, 69, 58, 0.35);
  }

  .apple-button.destructive:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: #e03228;
  }

  .apple-button.ghost {
    background-color: transparent;
    color: var(--blue-text);
  }

  .apple-button.ghost:hover:not(:disabled) {
    background-color: var(--fill-3);
  }

  .apple-button.ghost:active:not(:disabled) {
    transform: scale(var(--press-scale));
  }

  .apple-button.glass {
    background-color: rgba(118, 118, 128, 0.24);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }

  .apple-button.glass:hover:not(:disabled) {
    background-color: rgba(118, 118, 128, 0.36);
    border-color: rgba(255, 255, 255, 0.28);
    color: #ffffff;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .apple-button.glass:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: rgba(118, 118, 128, 0.44);
  }

  .apple-button.glass :global(svg) {
    color: rgba(255, 255, 255, 0.90);
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

  :global([data-effects="off"]) .apple-button,
  :global([data-reduce-motion="true"]) .apple-button {
    transition: background-color var(--dur-micro) ease, opacity var(--dur-micro) ease !important;
  }

  :global([data-effects="off"]) .apple-button:active:not(:disabled),
  :global([data-reduce-motion="true"]) .apple-button:active:not(:disabled) {
    transform: none !important;
  }

  :global([data-reduce-motion="true"]) .spinner {
    animation: none !important;
    opacity: 0.85;
  }
</style>
