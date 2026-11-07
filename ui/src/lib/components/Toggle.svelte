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
    onchange?: (checked: boolean) => void;
    children?: Snippet;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    id = `toggle-${Math.random().toString(36).substring(2, 9)}`,
    name,
    label,
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
    font-family: var(--font-system);
    font-size: var(--text-body);
    color: var(--color-label);
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
    border-radius: var(--radius-pill);
    transition: transform var(--motion-duration-short) ease;
  }

  .apple-toggle:focus-visible {
    box-shadow: 0 0 0 3px var(--color-system-blue);
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
    border-radius: var(--radius-pill);
    background-color: var(--color-tertiary-system-fill);
    border: 1px solid var(--color-separator);
    transition:
      background-color var(--motion-duration-normal) var(--motion-spring-ease),
      border-color var(--motion-duration-normal) var(--motion-spring-ease);
  }

  .apple-toggle.checked .track {
    background-color: var(--color-system-green);
    border-color: var(--color-system-green);
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
      transform var(--motion-duration-normal) var(--motion-spring-ease),
      width var(--motion-duration-short) ease,
      margin-left var(--motion-duration-short) ease;
  }

  .apple-toggle.checked .thumb {
    transform: translateX(18px);
  }

  .label-text {
    font-size: var(--text-subheadline);
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
</style>
