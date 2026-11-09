<!--
  Apple HIG Segmented Control Component
  Specification: tasks/apple/hig/segmented-controls.md
  Rules:
  - Equal width segments
  - Sliding active pill background with smooth spring transition
  - High contrast on selected segment, secondaryLabel on unselected
  - Consistent control types (all text or all icons)
  - Keyboard accessible (arrow keys, space, enter)
-->
<script lang="ts">
  interface Option {
    value: string;
    label: string;
    icon?: string;
  }

  interface Props {
    options: Option[] | string[];
    value?: string;
    disabled?: boolean;
    size?: 'small' | 'regular';
    ariaLabel?: string;
    onchange?: (val: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    disabled = false,
    size = 'regular',
    ariaLabel,
    onchange,
  }: Props = $props();

  const normalizedOptions = $derived<Option[]>(
    options.map((opt) => (typeof opt === 'string' ? { value: opt, label: opt } : opt))
  );

  let activeIndex = $derived(
    Math.max(
      0,
      normalizedOptions.findIndex((opt) => opt.value === value)
    )
  );

  let buttonEls: HTMLButtonElement[] = $state([]);

  function select(optValue: string, focusIndex?: number) {
    if (disabled) return;
    value = optValue;
    onchange?.(optValue);
    if (focusIndex !== undefined && buttonEls[focusIndex]) {
      buttonEls[focusIndex].focus();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (disabled) return;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      event.preventDefault();
      const nextIndex = (activeIndex + 1) % normalizedOptions.length;
      select(normalizedOptions[nextIndex].value, nextIndex);
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      event.preventDefault();
      const prevIndex = (activeIndex - 1 + normalizedOptions.length) % normalizedOptions.length;
      select(normalizedOptions[prevIndex].value, prevIndex);
    }
  }
</script>

<div
  class="apple-segmented-control {size}"
  class:disabled
  role="tablist"
  aria-label={ariaLabel}
  aria-orientation="horizontal"
>
  <!-- Sliding active pill -->
  <div
    class="active-pill"
    style="
      width: calc((100% - 6px) / {normalizedOptions.length});
      transform: translateX(calc({activeIndex} * 100%));
    "
  ></div>

  {#each normalizedOptions as opt, i (opt.value)}
    {@const isSelected = opt.value === value || (i === 0 && !value)}
    <button
      type="button"
      bind:this={buttonEls[i]}
      role="tab"
      class="segment-button"
      class:selected={isSelected}
      aria-selected={isSelected}
      tabindex={isSelected ? 0 : -1}
      onclick={() => select(opt.value, i)}
      onkeydown={handleKeyDown}
    >
      <span class="segment-label">{opt.label}</span>
    </button>
  {/each}
</div>

<style>
  .apple-segmented-control {
    position: relative;
    display: inline-flex;
    align-items: center;
    padding: 3px;
    background-color: var(--color-tertiary-system-fill);
    border-radius: var(--radius-sm);
    border: 1px solid var(--glass-border);
    user-select: none;
    width: 100%;
    max-width: 640px;
  }

  .apple-segmented-control.small {
    height: 28px;
    border-radius: var(--radius-xs);
  }

  .apple-segmented-control.small .segment-button {
    font-size: var(--text-caption1);
    padding: 0 6px;
  }

  .apple-segmented-control.regular {
    height: 34px;
    border-radius: var(--radius-sm);
  }

  .apple-segmented-control.disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  .active-pill {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    border-radius: calc(var(--radius-sm) - 2px);
    background-color: var(--color-card-bg, #ffffff);
    box-shadow:
      0 1px 4px rgba(0, 0, 0, 0.14),
      0 0 1px rgba(0, 0, 0, 0.12);
    transition: transform var(--motion-duration-normal) var(--motion-spring-ease);
    pointer-events: none;
    z-index: 1;
  }

  :global([data-theme="dark"]) .active-pill {
    background-color: #636366;
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.4),
      0 0 1px rgba(255, 255, 255, 0.1);
  }

  .segment-button {
    position: relative;
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    cursor: pointer;
    font-family: var(--font-system);
    font-weight: 500;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 10px;
    z-index: 2;
    transition: color var(--motion-duration-short) ease;
    outline: none;
  }

  .segment-button:hover:not(.selected) {
    color: var(--color-label);
  }

  .segment-button:active {
    opacity: 0.75;
  }

  .segment-button:focus-visible {
    border-radius: calc(var(--radius-sm) - 2px);
    outline: 2px solid var(--color-system-blue);
    outline-offset: 1px;
    box-shadow: 0 0 0 2px rgba(0, 145, 255, 0.3);
  }

  .apple-segmented-control.small .segment-button:focus-visible {
    border-radius: calc(var(--radius-xs) - 2px);
  }

  .segment-button.selected {
    color: var(--color-label);
    font-weight: 600;
  }

  .segment-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (prefers-reduced-motion: reduce) {
    .active-pill,
    .segment-button {
      transition: none !important;
    }
  }

  :global([data-reduce-motion="true"]) .active-pill,
  :global([data-reduce-motion="true"]) .segment-button {
    transition: none !important;
  }
</style>
