<!--
  Command palette (reference: .cmd). Ctrl K or the search field opens it.
  HIG › Search fields: results update as you type; HIG › Keyboard: arrows move,
  Enter runs, Esc closes and focus returns where it was. A modal dialog
  (aria-modal) with a combobox and a listbox for assistive technology.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { palette, type CommandItem } from './commands.svelte';
  import { t } from '../i18n';

  let q = $state('');
  let index = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let listEl: HTMLElement | undefined = $state();

  const results = $derived.by(() => {
    void palette.servers;
    void palette.mods;
    return palette.open ? palette.search(q) : [];
  });

  $effect(() => {
    if (palette.open) {
      q = '';
      index = 0;
      void tick().then(() => input?.focus());
    }
  });

  $effect(() => {
    void q;
    index = 0;
  });

  $effect(() => {
    void index;
    void tick().then(() => listEl?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: 'nearest' }));
  });

  function run(item: CommandItem | undefined) {
    if (!item) return;
    palette.hide();
    item.run();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      index = Math.min(results.length - 1, index + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      index = Math.max(0, index - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(results[index]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      palette.hide();
    } else if (e.key === 'Tab') {
      // Focus stays in the dialog: the input is its only control.
      e.preventDefault();
    }
  }
</script>

{#if palette.open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="cmd-bg" onclick={(e) => e.target === e.currentTarget && palette.hide()}>
    <div class="cmd" role="dialog" aria-modal="true" aria-label={t('shell.search')}>
      <label class="cmd-in">
        <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.5" /><path d="M10.5 10.5L14 14" /></svg>
        <input
          bind:this={input}
          bind:value={q}
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="cmd-list"
          aria-activedescendant={results.length ? `cmd-${index}` : undefined}
          aria-autocomplete="list"
          placeholder={t('cmd.placeholder')}
          aria-label={t('shell.search')}
          autocomplete="off"
          spellcheck="false"
          onkeydown={onKey}
        />
      </label>
      <div class="cmd-list" id="cmd-list" role="listbox" bind:this={listEl} aria-label={t('shell.search')}>
        {#if results.length}
          {#each results as item, i (i)}
            {#if i === 0 || results[i - 1].group !== item.group}
              <div class="cmd-g" role="presentation">{item.group}</div>
            {/if}
            <button
              type="button"
              class="cmd-row"
              id="cmd-{i}"
              role="option"
              tabindex="-1"
              aria-selected={i === index}
              onmousemove={() => (index = i)}
              onclick={() => run(item)}
            >
              <b>{item.title}</b>
              {#if item.sub}<span>{item.sub}</span>{/if}
              {#if item.kbd}<kbd>{item.kbd}</kbd>{/if}
            </button>
          {/each}
        {:else}
          <div class="cmd-empty">{t('cmd.empty')}</div>
        {/if}
      </div>
      <div class="cmd-foot" aria-hidden="true">
        <span><kbd>↑</kbd><kbd>↓</kbd> {t('cmd.foot_select')}</span>
        <span><kbd>{t('cmd.kbd_enter')}</kbd> {t('cmd.foot_run')}</span>
        <span><kbd>{t('cmd.kbd_esc')}</kbd> {t('cmd.foot_close')}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .cmd-bg {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: var(--veil);
  }
  .cmd {
    width: min(620px, calc(100% - 32px));
    border-radius: var(--r-l);
    background: var(--veil-strong);
    box-shadow:
      inset 0 0 0 1px var(--edge),
      0 30px 80px var(--shade);
    overflow: hidden;
    animation: cmd-in var(--dur-swap) var(--spring);
  }
  @keyframes cmd-in {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
  }
  .cmd-in {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 20px;
    border-bottom: 1px solid var(--separator);
    color: var(--label-3);
  }
  .cmd-in svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  .cmd-in input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: 0;
    font: inherit;
    font-size: var(--t-t3);
    color: var(--label);
    /* Browser default field padding; the base reset in wex.css removes it. */
    padding: 1px 2px;
  }
  /* The palette itself shows where typing goes; no extra ring inside it (reference). */
  .cmd-in input:focus-visible {
    outline: none !important;
  }
  .cmd-list {
    max-height: 380px;
    overflow: auto;
    padding: 8px;
  }
  .cmd-g {
    padding: 8px 12px 4px;
    font-size: var(--t-cap);
    font-weight: var(--w-semibold);
    color: var(--label-3);
  }
  .cmd-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-s);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      background-color var(--dur-micro),
      color var(--dur-micro);
  }
  .cmd-row b {
    font-weight: var(--w-medium);
    white-space: nowrap;
  }
  .cmd-row span {
    color: var(--label-3);
    font-size: var(--t-foot);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }
  .cmd-row kbd {
    margin-left: auto;
  }
  .cmd-row:hover {
    background: var(--fill-3);
  }
  .cmd-row[aria-selected='true'] {
    background: var(--blue);
    color: var(--blue-ink);
  }
  .cmd-row[aria-selected='true'] span {
    color: var(--blue-ink);
    opacity: 0.8;
  }
  kbd {
    font: 600 11px var(--font);
    padding: 2px 8px;
    border-radius: var(--r-xs);
    background: var(--fill-2);
    color: var(--label-2);
  }
  .cmd-empty {
    padding: 20px 12px;
    color: var(--label-2);
  }
  .cmd-foot {
    display: flex;
    gap: 16px;
    padding: 12px 20px;
    border-top: 1px solid var(--separator);
    font-size: var(--t-cap);
    color: var(--label-3);
  }
  .cmd-foot kbd {
    margin-right: 4px;
  }
</style>
