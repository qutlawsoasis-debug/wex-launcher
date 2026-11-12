<!--
  Home (reference: V.home). The build pair, the Aura server with live players
  and ping, launcher news from the changelog, friends online.
  Owner decision: the reference's «Продолжить» is «Сервер Aura» until core
  remembers the last server.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { builds } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { router } from '../../shell/router.svelte';
  import { createIdleInterval } from '../../design/idleManager';
  import { friendsLive, type FriendItem } from '../../services/friendsLive';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { t, tn } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import BuildPair from './BuildPair.svelte';
  import { home } from './home.svelte';

  onMount(() => {
    void home.load(builds.list.map((b) => b.id));
    const stopFriends = home.watchFriends();
    const stopPing = createIdleInterval(() => void home.refreshPing(), 30_000);
    return () => {
      stopFriends();
      stopPing();
    };
  });

  /** "214 игроков сейчас, пинг 23 мс" — each part only if the server reported it. */
  const liveText = $derived.by(() => {
    const p = home.ping;
    if (!p?.ok) return '';
    const parts: string[] = [];
    if (p.online !== undefined) parts.push(tn('home.players_now', p.online));
    if (p.ms !== undefined) parts.push(t('home.ping_ms', { ms: p.ms }));
    return parts.join(', ');
  });

  function friendStatus(f: FriendItem): string {
    if (f.status === 'playing') return f.world_name ? t('home.friend_playing_in', { world: f.world_name }) : t('home.friend_playing');
    if (f.status === 'in_lobby') return t('home.friend_in_lobby');
    return t('home.friend_online');
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
    {#if home.aura}
      <section>
        <div class="h-row"><h2>{t('home.aura_server')}</h2></div>
        <div class="tile continue">
          <Monogram name={home.aura.name} shape="large" tone={4} />
          <div class="continue-t">
            <b>{home.aura.name}</b>
            {#if home.ping?.ok}
              <span class="live">
                <span class="dot" aria-hidden="true"></span>
                <span>{liveText}</span>
              </span>
            {:else if home.ping}
              <span>{t('home.server_no_answer')}</span>
            {/if}
          </div>
          <button type="button" class="btn blue" disabled={launch.busy} onclick={() => home.aura && launch.start(home.aura.address)}>{t('home.join')}</button>
        </div>
      </section>
    {/if}
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
      {:else}
        <p class="empty">{t('home.friends_need_accounts')}</p>
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
  .continue {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    align-items: center;
    gap: 20px;
    padding: 16px;
  }
  .continue-t {
    min-width: 0;
  }
  .continue b {
    font-size: var(--t-t3);
    font-weight: var(--w-bold);
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .continue-t > span {
    display: flex;
    width: fit-content;
    color: var(--label-2);
    font-size: var(--t-sub);
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
