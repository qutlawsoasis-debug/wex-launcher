<!--
  Apple HIG Toggle (Switch) Component
  Specification: tasks/apple/hig/toggles.md
  Rules:
  - Chooses between two opposing values
  - State clarity: green/accent track when ON, neutral fill when OFF
  - Circular white knob with subtle shadow
  - Smooth spring animation for knob sliding
  - Accessible label and keyboard toggle support
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    id?: string;
    name?: string;
    label?: string;
    ariaLabel?: string;
    onchange?: (checked: boolean) => void;
    children?: Snippet;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    id = `toggle-${Math.random().toString(36).substring(2, 9)}`,
    name,
    label,
    ariaLabel,
    onchange,
    children,
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (disabled) return;
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      toggle();
    }
  }
</script>

<label class="apple-toggle-wrapper" class:disabled for={id}>
  <div
    class="apple-toggle"
    class:checked
    role="switch"
    aria-label={ariaLabel || label || 'Переключатель'}
    aria-checked={checked}
    aria-disabled={disabled}
    tabindex={disabled ? -1 : 0}
    onclick={toggle}
    onkeydown={handleKeyDown}
  >
    <div class="track"></div>
    <div class="thumb"></div>
  </div>

  {#if label || children}
    <span class="label-text">
      {#if children}
        {@render children()}
      {:else}
        {label}
      {/if}
    </span>
  {/if}

  <input
    type="checkbox"
    {id}
    {name}
    bind:checked
    {disabled}
    class="sr-only"
    tabindex="-1"
    aria-hidden="true"
  />
</label>

<style>
  .apple-toggle-wrapper {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    user-select: none;
    font-family: var(--font);
    font-size: var(--t-body);
    color: var(--label);
  }

  .apple-toggle-wrapper.disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .apple-toggle {
    position: relative;
    width: 44px;
    height: 26px;
    flex-shrink: 0;
    outline: none;
    border-radius: var(--r-pill);
    transition: transform var(--dur-micro) ease;
  }

  .apple-toggle-wrapper:hover:not(.disabled) .track {
    border-color: var(--separator);
  }

  .apple-toggle:focus-visible {
    outline: 2px solid var(--blue);
    outline-offset: 2px;
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.35);
  }

  .apple-toggle:active:not([aria-disabled="true"]) .thumb {
    width: 24px;
  }

  .apple-toggle.checked:active:not([aria-disabled="true"]) .thumb {
    margin-left: -4px;
  }

  .track {
    position: absolute;
    inset: 0;
    border-radius: var(--r-pill);
    background-color: var(--fill-3);
    border: 1px solid var(--separator);
    transition:
      background-color var(--dur-short) var(--ease),
      border-color var(--dur-short) var(--ease);
  }

  .apple-toggle.checked .track {
    background-color: var(--green);
    border-color: var(--green);
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background-color: #ffffff;
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.22),
      0 0 1px rgba(0, 0, 0, 0.15);
    transition:
      transform var(--dur-short) var(--ease),
      width var(--dur-micro) ease,
      margin-left var(--dur-micro) ease;
  }

  .apple-toggle.checked .thumb {
    transform: translateX(18px);
  }

  .label-text {
    font-size: var(--t-sub);
    font-weight: 400;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }


  :global([data-reduce-motion="true"]) .apple-toggle,
  :global([data-reduce-motion="true"]) .track,
  :global([data-reduce-motion="true"]) .thumb {
    transition: none !important;
  }

  :global([data-reduce-motion="true"]) .apple-toggle:active:not([aria-disabled="true"]) .thumb,
  :global([data-reduce-motion="true"]) .apple-toggle.checked:active:not([aria-disabled="true"]) .thumb {
    width: 20px !important;
    margin-left: 0 !important;
  }
</style>
