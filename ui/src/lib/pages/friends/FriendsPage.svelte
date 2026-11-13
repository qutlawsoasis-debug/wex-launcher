<!--
  Friends (reference: V.friends) with Rooms as a second tab (owner decision:
  Lobby lives in Friends) and «Свой сервер» as the third (a dedicated server for
  the group). Friends come from friendsLive (social_sync); until
  accounts are connected the list is empty and says so, and nothing that needs an
  account (own code, «Добавить друга») is shown.
  HIG › Lists and tables; Segmented controls (All / In game / Online).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { indicator } from '../../shell/indicator';
  import { toasts } from '../../shell/toast.svelte';
  import { call } from '../../shell/ipc';
  import { formatError } from '../../design/errors';
  import { friendsLive, type FriendItem, type FriendRequestItem } from '../../services/friendsLive';
  import { t } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import RoomsTab from './RoomsTab.svelte';
  import ServerTab from './ServerTab.svelte';
  import { friendsIntent } from './friendsIntent.svelte';

  let tab = $state<'friends' | 'rooms' | 'server'>(friendsIntent.tab);
  friendsIntent.tab = 'friends';
  let filter = $state<'all' | 'game' | 'online'>('all');
  let friends = $state<FriendItem[]>([...friendsLive.friends]);
  let requests = $state<FriendRequestItem[]>([...friendsLive.requests]);

  onMount(() =>
    friendsLive.onData((d) => {
      friends = [...(d.friends ?? [])];
      requests = [...(d.incoming_requests ?? [])];
    })
  );

  const shown = $derived(
    friends.filter((f) => filter === 'all' || (filter === 'game' && f.status === 'playing') || (filter === 'online' && f.status !== 'offline'))
  );

  function status(f: FriendItem): string {
    if (f.status === 'playing') return f.world_name ? t('home.friend_playing_in', { world: f.world_name }) : t('home.friend_playing');
    if (f.status === 'in_lobby') return t('home.friend_in_lobby');
    if (f.status === 'online') return t('home.friend_online');
    return t('shell.offline');
  }

  async function answer(r: FriendRequestItem, accept: boolean) {
    try {
      await call(accept ? 'social_accept_friend_request' : 'social_decline_friend_request', { requestId: r.id });
      requests = requests.filter((x) => x.id !== r.id);
      void friendsLive.pollNow();
    } catch (e) {
      const f = formatError(e, 'friends');
      toasts.show(f.title, f.description);
    }
  }

  async function join(f: FriendItem) {
    if (!f.lobby_code) return;
    try {
      await friendsLive.handleJoin(f.lobby_code);
    } catch (e) {
      const err = formatError(e, 'lobby');
      toasts.show(err.title, err.description);
    }
  }
</script>

<div class="page-title">
  <h1>{t('shell.nav_friends')}</h1>
  <p>{t('friends2.subtitle')}</p>
</div>

<div>
  <div class="tabs" role="tablist" aria-label={t('shell.nav_friends')} use:indicator={{ selector: '[aria-selected="true"]', key: tab, kind: 'underline' }}>
    <button type="button" role="tab" aria-selected={tab === 'friends'} onclick={() => (tab = 'friends')}>{t('friends2.tab_friends')}</button>
    <button type="button" role="tab" aria-selected={tab === 'rooms'} onclick={() => (tab = 'rooms')}>{t('friends2.tab_rooms')}</button>
    <button type="button" role="tab" aria-selected={tab === 'server'} onclick={() => (tab = 'server')}>{t('friends2.tab_server')}</button>
  </div>
  <div class="body" role="tabpanel">
    {#if tab === 'rooms'}
      <RoomsTab />
    {:else if tab === 'server'}
      <ServerTab />
    {:else}
      {#if requests.length}
        <section class="requests">
          <div class="h-row"><h2>{t('friends2.requests', { count: requests.length })}</h2></div>
          <div class="panel">
            {#each requests as r (r.id)}
              <div class="prow">
                <Monogram name={r.from_nick} />
                <div><b>{r.from_nick}</b><span>{t('friends2.request_d')}</span></div>
                <div class="acts">
                  <button type="button" class="mini blue" onclick={() => answer(r, true)}>{t('friends2.accept')}</button>
                  <button type="button" class="mini" onclick={() => answer(r, false)}>{t('friends2.decline')}</button>
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}
      {#if friends.length}
        <div class="tools">
          <div class="seg" role="group" aria-label={t('friends2.filter')} use:indicator={{ selector: '[aria-pressed="true"]', key: filter }}>
            <button type="button" aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>{t('build.filter_all')}</button>
            <button type="button" aria-pressed={filter === 'game'} onclick={() => (filter = 'game')}>{t('friends2.in_game')}</button>
            <button type="button" aria-pressed={filter === 'online'} onclick={() => (filter = 'online')}>{t('shell.online')}</button>
          </div>
        </div>
        <div class="panel">
          {#each shown as f (f.id)}
            <div class="prow" class:off={f.status === 'offline'}>
              <Monogram name={f.nick} />
              <div>
                <b>{f.nick}{#if f.game_version}<span class="vtag" style:--c="var(--cover-{f.game_version.replace(/\D/g, '')}, var(--fill))">{f.game_version}</span>{/if}</b>
                <span>{#if f.status === 'playing'}<span class="dot" aria-hidden="true"></span>{/if}{status(f)}</span>
              </div>
              {#if f.lobby_code}
                <button type="button" class="mini blue" onclick={() => join(f)}>{t('home.join')}</button>
              {:else}
                <span></span>
              {/if}
            </div>
          {:else}
            <p class="empty">{t('friends2.none_filtered')}</p>
          {/each}
        </div>
      {:else}
        <div class="panel"><p class="empty"><b>{t('home.friends_need_accounts')}</b> {t('friends2.rooms_hint')}</p></div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .page-title h1 {
    font-size: var(--t-t1);
    font-weight: var(--w-display);
    letter-spacing: -0.02em;
  }
  .page-title p {
    margin: 4px 0 0;
    color: var(--label-2);
  }
  .body {
    margin-top: 16px;
  }
  .requests {
    margin-bottom: 28px;
  }
  .prow {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 8px;
    border-radius: var(--r-m);
    transition: background-color var(--dur-micro);
  }
  .prow:hover {
    background: var(--fill-3);
  }
  .prow > div {
    min-width: 0;
  }
  .prow b {
    display: block;
    font-weight: var(--w-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .prow > div > span {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .prow.off {
    opacity: 0.5;
  }
  .acts {
    display: flex;
    gap: 8px;
  }
  .vtag {
    display: inline-block;
    vertical-align: 1px;
    margin-left: 4px;
    padding: 0 8px;
    border-radius: var(--r-pill);
    font-size: 11px;
    font-weight: var(--w-bold);
    line-height: 18px;
    color: var(--cover-ink);
    background: var(--c);
  }
</style>
