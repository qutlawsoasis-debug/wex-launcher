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
      opacity var(--dur-micro) ease;
  }

  .apple-button:active:not(:disabled) {
    transition-duration: var(--dur-micro);
    transition-timing-function: var(--ease);
  }

  /* Focus: the one global ring in app.css (2 px accent outline, offset outward;
     aura-knowledge «Кольцо фокуса Aura»). No second halo here. */

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

  /* Variants. States from aura-knowledge «Состояния кнопки Aura» (HIG › Buttons):
     Normal/Destructive rest on systemFill, hover goes to secondarySystemFill,
     pressed to tertiarySystemFill; Primary rests on the accent, does not change on
     hover and moves 20 % toward secondarySystemFill when pressed; Disabled is
     quaternarySystemFill with quaternaryLabel. Text: label, Destructive systemRed,
     Primary onAccent. No coloured shadows: selection and emphasis are fills. */
  .apple-button.primary {
    background-color: var(--blue);
    color: var(--blue-ink);
  }

  .apple-button.primary:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: color-mix(in srgb, var(--blue) 80%, var(--fill-2));
  }

  .apple-button.secondary,
  .apple-button.destructive {
    background-color: var(--fill);
    color: var(--label);
  }

  .apple-button.destructive {
    color: var(--red);
  }

  .apple-button.secondary:hover:not(:disabled),
  .apple-button.destructive:hover:not(:disabled) {
    background-color: var(--fill-2);
  }

  .apple-button.secondary:active:not(:disabled),
  .apple-button.destructive:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: var(--fill-3);
  }

  /* Ghost: a borderless button, neutral like every non-primary control
     (AGENTS.md: controls are neutral by default). */
  .apple-button.ghost {
    background-color: transparent;
    color: var(--label);
  }

  .apple-button.ghost:hover:not(:disabled) {
    background-color: var(--fill-3);
  }

  .apple-button.ghost:active:not(:disabled) {
    transform: scale(var(--press-scale));
    background-color: var(--fill-2);
  }

  /* Glass: over imagery (HIG › Materials; Liquid Glass takes its colour from what
     is behind it, «Цвет на стекле»). White text over the cover, like the hero buttons. */
  .apple-button.glass {
    background-color: var(--hero-btn);
    color: var(--cover-ink);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
  }

  .apple-button.glass:hover:not(:disabled) {
    background-color: var(--hero-btn-hi);
  }

  .apple-button.glass:active:not(:disabled) {
    transform: scale(var(--press-scale));
  }

  /* Reduce Transparency: the glass becomes an opaque surface (HIG › Accessibility,
     Materials). The global rule in wex.css already removes the blur. */
  :global(:root[data-reduce-transparency='true']) .apple-button.glass {
    background-color: var(--card);
    color: var(--label);
  }
  @media (prefers-reduced-transparency: reduce) {
    .apple-button.glass {
      background-color: var(--card);
      color: var(--label);
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
  }

  /* Disabled state */
  .apple-button:disabled {
    background-color: var(--fill-4);
    color: var(--label-4);
    cursor: not-allowed;
    transform: none !important;
  }

  .apple-button.ghost:disabled {
    background-color: transparent;
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
