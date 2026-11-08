<!--
  Apple HIG Search Field Component
  Specification: tasks/apple/hig/search-fields.md
  Rules:
  - Magnifying glass icon on leading edge
  - Clear button ('x') removes text with single click
  - Helpful placeholder text
  - Escape key clears search
  - Focus ring with system accent color
-->
<script lang="ts">
  import Icon from './icons/Icon.svelte';

  interface Props {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    oninput?: (val: string) => void;
    onclear?: () => void;
  }

  let {
    value = $bindable(''),
    placeholder = 'Поиск...',
    disabled = false,
    oninput,
    onclear,
  }: Props = $props();

  let inputEl: HTMLInputElement | undefined = $state();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    value = target.value;
    oninput?.(value);
  }

  function clear() {
    value = '';
    oninput?.('');
    onclear?.();
    inputEl?.focus();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && value) {
      e.stopPropagation();
      clear();
    }
  }
</script>

<div class="apple-search-field" class:disabled class:has-value={!!value} role="search">
  <span class="search-icon">
    <Icon name="search" size={15} color="var(--color-tertiary-label)" />
  </span>

  <input
    bind:this={inputEl}
    type="search"
    aria-label="Поле поиска"
    {placeholder}
    {value}
    {disabled}
    oninput={handleInput}
    onkeydown={handleKeyDown}
    class="search-input"
    spellcheck="false"
  />

  {#if value}
    <button
      type="button"
      class="clear-btn"
      title="Очистить"
      onclick={clear}
      aria-label="Очистить поиск"
    >
      <Icon name="clear" size={11} color="var(--color-tertiary-label)" />
    </button>
  {/if}
</div>

<style>
  .apple-search-field {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 100%;
    max-width: 320px;
    height: 32px;
    padding: 0 8px 0 10px;
    background-color: var(--color-quaternary-system-fill);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    transition:
      background-color var(--motion-duration-short) ease,
      box-shadow var(--motion-duration-short) ease,
      border-color var(--motion-duration-short) ease;
  }

  .apple-search-field:hover:not(:focus-within):not(.disabled) {
    background-color: var(--color-tertiary-system-fill);
  }

  .apple-search-field:focus-within {
    background-color: var(--card-bg);
    border-color: var(--color-system-blue);
    outline: 2px solid var(--color-system-blue);
    outline-offset: 1px;
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
  }

  .apple-search-field.disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  .search-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-right: 6px;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    background: transparent;
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    color: var(--color-label);
    outline: none;
  }

  .search-input::placeholder {
    color: var(--color-tertiary-label);
  }

  .clear-btn {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background-color: var(--color-tertiary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    margin-left: 4px;
    outline: none;
    transition:
      background-color var(--motion-duration-short) ease,
      box-shadow var(--motion-duration-short) ease;
  }

  .clear-btn:hover {
    background-color: var(--color-secondary-system-fill);
  }

  .clear-btn:active {
    background-color: var(--color-system-fill);
  }

  .clear-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 1px;
  }

  @media (prefers-reduced-motion: reduce) {
    .apple-search-field,
    .clear-btn {
      transition: none !important;
    }
  }

  :global([data-reduce-motion="true"]) .apple-search-field,
  :global([data-reduce-motion="true"]) .clear-btn {
    transition: none !important;
  }
</style>
