<!--
  Home (reference: V.home). The build pair, «Играть вместе» (the player's own
  servers with live ping and LAN worlds nearby, all from core), launcher news from
  the changelog, friends online.
  HIG › Buttons: Play stays the one prominent action; Join in the rows is a
  secondary button. HIG › Lists and tables: rows of name, status and one action.
  Owner decision (2026-11-13): no «Сервер Aura» block. Wex is for playing with
  friends (LAN, a friend's room, the company's own server), not one public server.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { builds } from '../../shell/builds.svelte';
  import { router } from '../../shell/router.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { createIdleInterval } from '../../design/idleManager';
  import { friendsIntent } from '../friends/friendsIntent.svelte';
  import { friendsLive, type FriendItem } from '../../services/friendsLive';
  import { describeConnection } from '../friends/friendsState';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { t, tn } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import BuildPair from './BuildPair.svelte';
  import { home, type ServerPing } from './home.svelte';

  onMount(() => {
    void home.load(builds.list.map((b) => b.id));
    const stopFriends = home.watchFriends();
    const stopPing = createIdleInterval(() => void home.refreshPings(), 30_000);
    return () => {
      stopFriends();
      stopPing();
    };
  });

  /** «3 из 10, пинг 23 мс» — only what the server answered. */
  function pingText(p: ServerPing | undefined): string {
    if (!p) return t('servers2.pinging');
    if (!p.ok) return t('home.server_no_answer');
    const parts: string[] = [];
    if (p.online !== undefined && p.max !== undefined) parts.push(t('servers2.players', { on: p.online, max: p.max }));
    else if (p.online !== undefined) parts.push(tn('home.players_now', p.online));
    if (p.ms !== undefined) parts.push(t('home.ping_ms', { ms: p.ms }));
    return parts.join(', ');
  }

  function openRooms() {
    friendsIntent.tab = 'rooms';
    router.go('friends');
  }

  function friendStatus(f: FriendItem): string {
    if (f.status === 'playing') return f.world_name ? t('home.friend_playing_in', { world: f.world_name }) : t('home.friend_playing');
    if (f.status === 'in_lobby') return t('home.friend_in_lobby');
    return t('home.friend_online');
  }

  const friendsNotice = $derived(describeConnection(home.connection));
  let retrying = $state(false);
  async function retryFriends() {
    retrying = true;
    try {
      await friendsLive.retry();
    } finally {
      retrying = false;
    }
  }

  async function joinFriend(f: FriendItem) {
    if (!f.lobby_code) return;
    try {
      await friendsLive.handleJoin(f.lobby_code);
    } catch (e) {
      const err = formatError(e, 'lobby');
      toasts.show(err.title, err.description);
    }
  }
</script>

<BuildPair />

<div class="grid-3">
  <div class="col">
    <section>
      <div class="h-row">
        <h2>{t('home.together')}</h2>
        {#if home.serverCount > home.servers.length}
          <a
            href="#servers"
            onclick={(e) => {
              e.preventDefault();
              router.go('servers');
            }}>{t('home.all_servers', { n: home.serverCount })}</a
          >
        {/if}
      </div>
      <div class="panel together">
        {#each home.servers as s (s.address)}
          {@const p = home.pings[s.address]}
          <div class="prow">
            <Monogram name={s.name} />
            <div>
              <b>{s.name}</b>
              <span>{#if p?.ok}<span class="dot" aria-hidden="true"></span>{/if}{pingText(p)}</span>
            </div>
            <button type="button" class="mini" disabled={launch.busy} onclick={() => launch.start(s.address)}>{t('home.join')}</button>
          </div>
        {/each}
        {#each home.lan as w (w.address + ':' + w.port)}
          <div class="prow">
            <Monogram name={w.motd || 'LAN'} />
            <div>
              <b>{w.motd || t('home.lan_world')}</b>
              <span><span class="dot" aria-hidden="true"></span>{t('home.lan_nearby')}</span>
            </div>
            <button type="button" class="mini" disabled={launch.busy} onclick={() => launch.start(`${w.address}:${w.port}`)}>{t('home.join')}</button>
          </div>
        {/each}
        {#if !home.servers.length && !home.lan.length}
          <p class="empty"><b>{t('home.together_empty_title')}</b> {t('home.together_empty_text')}</p>
        {/if}
        <div class="together-actions">
          <button type="button" class="mini" onclick={openRooms}>{t('home.rooms')}</button>
          <button type="button" class="mini" onclick={() => router.go('servers')}>{home.servers.length ? t('home.servers') : t('servers2.add')}</button>
        </div>
      </div>
    </section>
    {#if home.news.length}
      <section>
        <div class="h-row"><h2>{t('home.news')}</h2></div>
        <div class="news">
          {#each home.news as n (n.version)}
            <article class="tile news-card">
              <time>{n.date}</time>
              <b>{n.title}</b>
              {#if n.text}<span>{n.text}</span>{/if}
            </article>
          {/each}
        </div>
      </section>
    {/if}
  </div>

  <section>
    <div class="h-row">
      <h2>{t('home.friends_online')}</h2>
      <a
        href="#friends"
        onclick={(e) => {
          e.preventDefault();
          router.go('friends');
        }}>{t('home.all_friends')}</a
      >
    </div>
    <div class="panel">
      {#if home.friendsOnline.length}
        {#each home.friendsOnline as f (f.id)}
          <div class="prow">
            <Monogram name={f.nick} />
            <div>
              <b>{f.nick}{#if f.game_version}<span class="vtag" style:--c="var(--cover-{f.game_version.replace(/\D/g, '')}, var(--fill))">{f.game_version}</span>{/if}</b>
              <span>{#if f.status === 'playing'}<span class="dot" aria-hidden="true"></span>{/if}{friendStatus(f)}</span>
            </div>
            {#if f.lobby_code}
              <button type="button" class="mini blue" onclick={() => joinFriend(f)}>{t('home.join')}</button>
            {:else}
              <span></span>
            {/if}
          </div>
        {/each}
      {:else if home.friendsKnown}
        <p class="empty">{t('home.friends_none_online')}</p>
      {:else if friendsNotice}
        <div role="status" aria-busy={friendsNotice.busy}>
          <p class="empty"><b>{friendsNotice.title}</b> {friendsNotice.text}</p>
          {#if friendsNotice.retry}
            <div class="friends-acts">
              <button type="button" class="mini" disabled={retrying} onclick={retryFriends}>{retrying ? t('friends2.retrying') : t('friends2.retry')}</button>
            </div>
          {/if}
        </div>
      {:else}
        <p class="empty">{t('friends2.none_yet_home')}</p>
      {/if}
    </div>
  </section>
</div>

<style>
  .grid-3 {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: 20px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 28px;
    min-width: 0;
  }
  .friends-acts {
    display: flex;
    gap: 8px;
    padding: 0 8px 8px;
  }
  .together-actions {
    display: flex;
    gap: 8px;
    padding: 8px;
  }
  .news {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }
  .news-card {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .news-card time {
    font-size: var(--t-foot);
    color: var(--label-3);
  }
  .news-card b {
    font-size: var(--t-body);
    font-weight: var(--w-semibold);
  }
  .news-card span {
    font-size: var(--t-sub);
    color: var(--label-2);
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  @media (max-width: 1100px) {
    .grid-3 {
      grid-template-columns: 1fr;
    }
  }
</style>
