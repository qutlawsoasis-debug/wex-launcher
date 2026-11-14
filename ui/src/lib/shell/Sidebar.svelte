<!--
  Sidebar (reference: .side). Sections, the build library and the profile.
  HIG › Sidebars: a translucent material, the selected row is a fill, icons and
  labels together, items keep their order. HIG › Materials: solid under Reduce
  Transparency (wex.css).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { router, buildRoute, buildIdOf, SECTIONS, type Route } from './router.svelte';
  import { builds } from './builds.svelte';
  import { indicator } from './indicator';
  import { NAV_ICONS } from './icons';
  import { t, tn } from '../i18n';
  import BuildCover from './BuildCover.svelte';
  import Monogram from './Monogram.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import { accounts } from './accounts.svelte';
  import { openSignIn } from './accountUi.svelte';
  import { onMount } from 'svelte';
  import { friendsLive } from '../services/friendsLive';

  interface Props {
    online: boolean;
    /** Shown above the profile (the update card). */
    footer?: Snippet;
  }

  let { online, footer }: Props = $props();

  const LABEL = {
    home: 'shell.nav_home',
    servers: 'shell.nav_servers',
    friends: 'shell.nav_friends',
    wardrobe: 'shell.nav_wardrobe',
    options: 'shell.nav_options',
  } as const;

  function open(e: MouseEvent, route: Route) {
    e.preventDefault();
    router.go(route);
  }

  const currentBuild = $derived(buildIdOf(router.route));

  let menuOpen = $state(false);
  let meBtn = $state<HTMLButtonElement | null>(null);
  /** No account yet: the profile is the way to sign in. */
  const signedOut = $derived(accounts.loaded && !accounts.list.length);

  function onMe() {
    if (signedOut) openSignIn();
    else menuOpen = !menuOpen;
  }

  /** Friends online, shown next to «Друзья» only when core reports any (reference: .count). */
  let friendsOnline = $state(friendsLive.friends.filter((f) => f.status !== 'offline').length);
  onMount(() => friendsLive.onData((d) => (friendsOnline = (d.friends ?? []).filter((f) => f.status !== 'offline').length)));
</script>

<aside class="side">
  <div class="brand"><span class="brand-mark" aria-hidden="true"><i></i><i></i></span>Wex</div>

  <nav class="nav" aria-label={t('shell.nav_label')} use:indicator={{ selector: '[aria-current="page"]', key: router.route }}>
    {#each SECTIONS as id (id)}
      <a href="#{id}" aria-current={router.route === id ? 'page' : undefined} onclick={(e) => open(e, id)}>
        <svg viewBox="0 0 20 20" aria-hidden="true">{@html NAV_ICONS[id]}</svg>
        <span>{t(LABEL[id])}</span>
        {#if id === 'friends' && friendsOnline > 0}<span class="count" aria-label={t('shell.friends_online_count', { n: friendsOnline })}>{friendsOnline}</span>{/if}
      </a>
    {/each}
  </nav>

  {#if builds.list.length}
    <div class="side-h" id="side-builds">{t('shell.builds')}</div>
    <nav class="lib" aria-labelledby="side-builds" use:indicator={{ selector: '[aria-current="page"]', key: router.route }}>
      {#each builds.list as b (b.id)}
        <a
          href="#{buildRoute(b.id)}"
          aria-current={currentBuild === b.id ? 'page' : undefined}
          onclick={(e) => open(e, buildRoute(b.id))}
        >
          <BuildCover id={b.id} />
          <div class="lib-t">
            <b>{t('shell.build_name', { ver: b.id })}</b>
            {#if b.mods !== null}<span>{tn('shell.mods', b.mods)}</span>{/if}
          </div>
          {#if b.id === builds.selectedId}
            <span class="sel" role="img" aria-label={t('shell.selected_for_launch')}></span>
          {/if}
        </a>
      {/each}
    </nav>
  {/if}

  <div class="foot">
    {@render footer?.()}
    <div class="me-wrap">
      <AccountMenu bind:open={menuOpen} anchor={meBtn} />
      <button
        type="button"
        class="me"
        bind:this={meBtn}
        aria-haspopup={signedOut ? 'dialog' : 'menu'}
        aria-expanded={signedOut ? undefined : menuOpen}
        title={t('shell.profile_label')}
        onclick={onMe}
      >
        {#if signedOut}
          <Monogram name="?" tone={2} />
          <div>
            <b>{t('acc.sign_in')}</b>
            <span class="off">{t('acc.sign_in_d')}</span>
          </div>
        {:else}
          <Monogram name={builds.playerName} tone={2} />
          <div>
            <b>{builds.playerName}</b>
            <span class:off={!online}>{online ? t('shell.online') : t('shell.offline')}</span>
          </div>
        {/if}
        <svg class="chev" viewBox="0 0 16 16" aria-hidden="true"><path d="M5 10l3-3 3 3" /></svg>
      </button>
    </div>
  </div>
</aside>

<style>
  .side {
    grid-row: 1 / 3;
    background: var(--side);
    backdrop-filter: blur(30px) saturate(160%);
    -webkit-backdrop-filter: blur(30px) saturate(160%);
    display: flex;
    flex-direction: column;
    padding: 20px 12px 12px;
    gap: 4px;
    min-height: 0;
    overflow-y: auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 8px 16px;
    font-weight: var(--w-display);
    font-size: var(--t-t3);
    letter-spacing: -0.01em;
  }
  .brand-mark {
    display: flex;
    gap: 4px;
    align-items: flex-end;
    height: 24px;
  }
  .brand-mark i {
    width: 8px;
    border-radius: 3px;
    display: block;
  }
  .brand-mark i:first-child {
    height: 24px;
    background: var(--cover-1211);
  }
  .brand-mark i:last-child {
    height: 16px;
    background: var(--cover-1201);
  }
  .nav,
  .lib {
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .nav a,
  .lib a {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    height: var(--row-nav);
    padding: 0 12px;
    border-radius: var(--r-s);
    color: var(--label-2);
    font-weight: var(--w-medium);
    text-decoration: none;
    transition:
      background-color var(--dur-micro),
      color var(--dur-micro);
  }
  .nav a:hover,
  .lib a:hover {
    background: var(--fill-3);
    color: var(--label);
  }
  .nav a[aria-current='page'],
  .lib a[aria-current='page'] {
    color: var(--label);
    background: transparent;
  }
  .nav :global(> .ind),
  .lib :global(> .ind) {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 0;
    pointer-events: none;
    border-radius: var(--r-s);
    background: var(--fill);
    transition:
      transform var(--dur-ind) var(--spring),
      opacity var(--dur-short);
  }
  .nav svg {
    width: 20px;
    height: 20px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .count {
    margin-left: auto;
    font-size: var(--t-foot);
    color: var(--label-3);
  }
  .side-h {
    /* 24 px line: the reference reserves it for the new-build button, which core does not offer. */
    line-height: 24px;
    padding: 20px 12px 8px;
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    color: var(--label-3);
  }
  .lib a {
    height: var(--row-lib);
    padding: 0 8px;
  }
  .lib-t {
    min-width: 0;
  }
  .lib b {
    display: block;
    font-weight: var(--w-semibold);
    color: var(--label);
    font-size: var(--t-sub);
  }
  .lib-t span {
    display: block;
    font-size: var(--t-cap);
    color: var(--label-3);
  }
  .sel {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--blue-text);
    flex: none;
  }
  .foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 12px;
  }
  .me-wrap {
    position: relative;
  }
  .me {
    width: 100%;
    border: 0;
    background: none;
    font: inherit;
    text-align: left;
    cursor: default;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: var(--r-m);
    color: inherit;
    text-decoration: none;
    transition: background-color var(--dur-micro);
  }
  .me:hover,
  .me[aria-expanded='true'] {
    background: var(--fill-3);
  }
  .me > div {
    flex: 1;
    min-width: 0;
  }
  .chev {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: var(--label-3);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .me b {
    display: block;
    font-weight: var(--w-semibold);
  }
  .me div > span {
    display: block;
    font-size: var(--t-cap);
    color: var(--green);
  }
  .me div > span.off {
    color: var(--label-3);
  }
</style>
