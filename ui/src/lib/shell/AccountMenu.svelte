<!--
  Account menu, opened from the profile at the bottom of the sidebar.
  HIG › Menus (pull-down menus): a short list of commands related to the button
  that opened it; a checkmark shows the current choice (the active account);
  separators group related items; an ellipsis marks an item that needs more
  input (sheet). HIG › Managing accounts: switching and adding accounts are easy
  to find; signing out lives in Settings with its warning.
  Escape, a click outside, or picking an item closes the menu; focus returns
  to the profile button.
-->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { accounts } from './accounts.svelte';
  import { accountUi, openSignIn } from './accountUi.svelte';
  import { router } from './router.svelte';
  import { toasts } from './toast.svelte';
  import { formatError } from '../design/errors';
  import { t } from '../i18n';
  import Monogram from './Monogram.svelte';

  interface Props {
    open: boolean;
    /** The button that opened the menu: focus goes back to it. */
    anchor: HTMLElement | null;
  }

  let { open = $bindable(), anchor }: Props = $props();
  let menu = $state<HTMLDivElement | null>(null);

  function close(refocus = true) {
    open = false;
    if (refocus) anchor?.focus();
  }

  $effect(() => {
    if (open) {
      void accounts.load();
      void tick().then(() => menu?.querySelector<HTMLElement>('[role^="menuitem"]')?.focus());
    }
  });

  onMount(() => {
    const down = (e: PointerEvent) => {
      if (!open) return;
      const el = e.target as Node;
      if (menu?.contains(el) || anchor?.contains(el)) return;
      close(false);
    };
    window.addEventListener('pointerdown', down, true);
    return () => window.removeEventListener('pointerdown', down, true);
  });

  function onKey(e: KeyboardEvent) {
    const items = [...(menu?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const n = items.length;
      items[(i + (e.key === 'ArrowDown' ? 1 : n - 1)) % n]?.focus();
    }
  }

  async function use(id: string) {
    close();
    if (id === accounts.activeId) return;
    try {
      await accounts.switchTo(id);
      toasts.show(t('acc.switched', { name: accounts.active?.name ?? '' }));
    } catch (e) {
      toasts.show(t('acc.switch_failed'), formatError(e, 'auth').description);
    }
  }

  function add() {
    close();
    openSignIn();
  }

  function manage() {
    close();
    accountUi.settingsSection = 'accounts';
    router.go('options');
  }

  function wardrobe() {
    close();
    router.go('wardrobe');
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="menu" role="menu" aria-label={t('acc.menu_label')} bind:this={menu} onkeydown={onKey}>
    {#each accounts.list as a (a.id)}
      <button type="button" role="menuitemradio" aria-checked={a.id === accounts.activeId} onclick={() => use(a.id)}>
        <Monogram name={a.name} tone={2} />
        <span class="t">
          <b>{a.name}</b>
          <span>{a.needs_sign_in ? t('acc.needs_sign_in') : a.kind === 'microsoft' ? t('acc.kind_microsoft') : t('acc.kind_offline')}</span>
        </span>
        <svg class="check" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>
      </button>
    {/each}
    {#if accounts.list.length}<div class="sep" role="separator"></div>{/if}
    <button type="button" role="menuitem" class="cmd" onclick={add}>{t('acc.menu_add')}</button>
    <button type="button" role="menuitem" class="cmd" onclick={manage}>{t('acc.menu_manage')}</button>
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" class="cmd" onclick={wardrobe}>{t('shell.nav_wardrobe')}</button>
  </div>
{/if}

<style>
  .menu {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: calc(100% + 6px);
    z-index: 30;
    display: flex;
    flex-direction: column;
    padding: 5px;
    border-radius: var(--r-m);
    background: var(--card);
    border: 1px solid var(--edge);
    box-shadow: var(--shade);
    animation: menu-in var(--dur-short) var(--ease);
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .menu {
      animation: none;
    }
  }
  :global(html[data-reduce-motion="true"]) .menu {
    animation: none;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--r-s);
    background: none;
    color: var(--label);
    font: inherit;
    text-align: left;
    cursor: default;
  }
  button:hover,
  button:focus-visible {
    background: var(--fill-2);
    outline: none;
  }
  .t {
    flex: 1;
    min-width: 0;
  }
  .t b {
    display: block;
    font-weight: var(--w-semibold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .t span {
    display: block;
    font-size: var(--t-cap);
    color: var(--label-2);
  }
  .check {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: var(--label);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    visibility: hidden;
  }
  [aria-checked='true'] .check {
    visibility: visible;
  }
  .cmd {
    min-height: 30px;
  }
  .sep {
    height: 1px;
    margin: 4px 8px;
    background: var(--separator);
  }
</style>
