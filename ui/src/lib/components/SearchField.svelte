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
  import { t } from '../i18n';

  interface Props {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    oninput?: (val: string) => void;
    onclear?: () => void;
  }

  let {
    value = $bindable(''),
    placeholder = t('search.placeholder_default'),
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
    <Icon name="search" size={15} color="var(--label-3)" />
  </span>

  <input
    bind:this={inputEl}
    type="search"
    aria-label={t('search.input_aria')}
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
      title={t('common.clear')}
      onclick={clear}
      aria-label={t('search.clear_aria')}
    >
      <Icon name="clear" size={11} color="var(--label-3)" />
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
    background-color: var(--fill-3);
    border: 1px solid var(--edge);
    border-radius: var(--r-s);
    transition:
      background-color var(--dur-micro) ease,
      box-shadow var(--dur-micro) ease,
      border-color var(--dur-micro) ease;
  }

  .apple-search-field:hover:not(:focus-within):not(.disabled) {
    background-color: var(--fill-3);
  }

  .apple-search-field:focus-within {
    background-color: var(--card);
    border-color: var(--blue-text);
    outline: 2px solid var(--blue);
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
    font-family: var(--font);
    font-size: var(--t-sub);
    color: var(--label);
    outline: none;
  }

  .search-input::placeholder {
    color: var(--label-3);
  }

  .clear-btn {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background-color: var(--fill-3);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    margin-left: 4px;
    outline: none;
    transition:
      background-color var(--dur-micro) ease,
      box-shadow var(--dur-micro) ease;
  }

  .clear-btn:hover {
    background-color: var(--fill-2);
  }

  .clear-btn:active {
    background-color: var(--fill);
  }

  .clear-btn:focus-visible {
    outline: 2px solid var(--blue);
    outline-offset: 1px;
  }


  :global([data-reduce-motion="true"]) .apple-search-field,
  :global([data-reduce-motion="true"]) .clear-btn {
    transition: none !important;
  }
</style>
