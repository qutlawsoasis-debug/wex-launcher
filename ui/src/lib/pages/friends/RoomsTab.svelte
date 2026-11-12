<!--
  Rooms tab (the former Lobby screen). Play together through a room: open your
  world (opened to LAN in Minecraft) to friends, join by a code, see open rooms.
  In a room: code to share, tunnel state with Retry, players (host can remove),
  invite friends online, leave/close.
  HIG › Text fields (room code), Buttons (one blue action), Lists and tables,
  Progress indicators (join steps from core events).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createIdleInterval } from '../../design/idleManager';
  import { network } from '../../design/network.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { toasts } from '../../shell/toast.svelte';
  import { t } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import { friendsLive } from '../../services/friendsLive';
  import { rooms } from './rooms.svelte';

  let code = $state('');

  const friendsOnline = $derived(rooms.active ? friendsLive.friends.filter((f) => f.status !== 'offline') : []);

  // LAN discovery runs only while this tab is open and the window is visible.
  onMount(() => {
    void rooms.startDiscovery();
    void rooms.refreshLan();
    void rooms.refreshOpen();
    const stopLan = createIdleInterval(() => void rooms.refreshLan(), 3000);
    const onVis = () => (document.hidden ? void rooms.stopDiscovery() : void rooms.startDiscovery());
    document.addEventListener('visibilitychange', onVis);
    return () => {
      stopLan();
      document.removeEventListener('visibilitychange', onVis);
      void rooms.stopDiscovery();
    };
  });

  $effect(() => {
    if (!rooms.active) return;
    return createIdleInterval(() => void rooms.poll(), 2500);
  });

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toasts.show(t('lobby.copied'));
    } catch {
      toasts.show(t('rooms.copy_failed'));
    }
  }

  async function connectLan(address: string) {
    const msg = await rooms.connectLan(address);
    if (msg) toasts.show(msg);
  }

  const STEP: Record<string, string> = {
    checking_mods: 'lobby.step_check_mods',
    downloading_mods: 'rooms.step_mods',
    connecting: 'lobby.step_connecting',
  };
</script>

{#if rooms.closedNotice}
  <div class="notice" role="status">
    <span>{rooms.closedNotice}</span>
    <button type="button" class="mini" onclick={() => (rooms.closedNotice = '')}>{t('rooms.ok')}</button>
  </div>
{/if}

{#if rooms.active}
  {@const room = rooms.active}
  <section class="tile room">
    <div class="room-head">
      <Monogram name={room.world_name} shape="large" tone={3} />
      <div class="room-t">
        <b>{room.world_name}</b>
        <span>{t('lobby.host_label', { host: room.host_name })}</span>
      </div>
      <button type="button" class="btn" onclick={() => copy(room.code)}>{t('rooms.code', { code: room.code })}</button>
    </div>
    <div class="tunnel" class:bad={!room.tunnel_active}>
      {#if room.tunnel_active && room.tunnel_address}
        <span class="live"><span class="dot" aria-hidden="true"></span>{t('lobby.tunnel_active', { address: room.tunnel_address })}</span>
        <button type="button" class="mini" onclick={() => copy(room.tunnel_address ?? '')}>{t('rooms.copy_address')}</button>
      {:else if room.tunnel_error}
        <span>{room.tunnel_error}</span>
        <button type="button" class="mini" disabled={rooms.retrying} onclick={() => rooms.retryTunnel()}>{t('lobby.retry_button')}</button>
      {:else}
        <span>{t('lobby.tunnel_starting')}</span>
      {/if}
    </div>
  </section>

  <section>
    <div class="h-row"><h2>{t('lobby.players_in_room', { count: room.players.length })}</h2></div>
    <div class="panel">
      {#each room.players as p (p.name)}
        <div class="prow">
          <Monogram name={p.name} />
          <div>
            <b>{p.name}</b>
            <span>{p.is_host ? t('rooms.host') : t('lobby.player_role_guest')}{p.ping_ms ? `, ${t('home.ping_ms', { ms: p.ping_ms })}` : ''}{p.mods_synced ? '' : `, ${t('lobby.sync_badge_mismatch')}`}</span>
          </div>
          {#if room.is_host && !p.is_host}
            <button type="button" class="mini danger" onclick={() => rooms.kick(p.name)}>{t('lobby.kick_player')}</button>
          {:else}
            <span></span>
          {/if}
        </div>
      {/each}
    </div>
  </section>

  {#if room.is_host && friendsOnline.length}
    <section>
      <div class="h-row"><h2>{t('lobby.invite_friend_header')}</h2></div>
      <div class="panel">
        {#each friendsOnline as f (f.id)}
          <div class="prow">
            <Monogram name={f.nick} />
            <div><b>{f.nick}</b><span>{t('home.friend_online')}</span></div>
            {#if rooms.invited.includes(f.id)}
              <span class="tag">{t('lobby.invited_badge')}</span>
            {:else}
              <button type="button" class="mini" onclick={() => rooms.invite(f.id)}>{t('rooms.invite')}</button>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <div>
    <button type="button" class="btn danger" onclick={() => rooms.leave()}>{room.is_host ? t('lobby.close_room') : t('lobby.leave_room')}</button>
  </div>
{:else}
  <section>
    <div class="h-row"><h2>{t('lobby.title_connect')}</h2></div>
    <div class="tile join">
      <label class="sr" for="room-code">{t('lobby.room_code_input_aria')}</label>
      <input
        id="room-code"
        class="field code-in"
        placeholder={t('lobby.code_placeholder')}
        bind:value={code}
        maxlength="12"
        autocomplete="off"
        spellcheck="false"
        onkeydown={(e) => e.key === 'Enter' && rooms.join(code)}
      />
      <button type="button" class="btn blue" disabled={!code.trim() || rooms.joining || launch.busy || !network.isOnline} onclick={() => rooms.join(code)}>{t('lobby.enter_btn')}</button>
      <p class="d">{t('lobby.room_code_friend_hint')}</p>
      {#if rooms.joining}
        <p class="d" aria-live="polite">{STEP[rooms.joinStep] ? t(STEP[rooms.joinStep]) + ' — ' : ''}{rooms.joinText}{rooms.joinPercent !== null ? ` ${Math.round(rooms.joinPercent)}%` : ''}</p>
      {/if}
      {#if rooms.modsDiffer}
        <button type="button" class="btn" disabled={rooms.joining} onclick={() => rooms.join(code, true)}>{t('lobby.fix_and_join')}</button>
      {/if}
    </div>
  </section>

  <section>
    <div class="h-row">
      <h2>{t('rooms.open_world')}</h2>
      <button type="button" onclick={() => rooms.refreshLan()}>{t('lobby.refresh')}</button>
    </div>
    <div class="list">
      {#if rooms.lan.length}
        {#each rooms.lan as w (w.address)}
          <div class="lrow lrow-lan">
            <Monogram name={w.motd} shape="square" tone={3} />
            <div><b>{w.motd}</b><span>{t('lobby.lan_world_address', { address: w.address })}</span></div>
            <button type="button" class="mini" onclick={() => connectLan(w.address)}>{t('lobby.connect_btn')}</button>
            <button type="button" class="mini blue" disabled={rooms.publishing !== null || !network.isOnline} onclick={() => rooms.publish(w)}>{t('lobby.btn_open_world_friends')}</button>
          </div>
        {/each}
      {:else}
        <p class="empty"><b>{t('lobby.no_lan_worlds_title')}.</b> {t('rooms.open_world_hint')}</p>
      {/if}
    </div>
  </section>

  <section>
    <div class="h-row">
      <h2>{t('rooms.open_rooms')}</h2>
      <button type="button" onclick={() => rooms.refreshOpen()}>{t('lobby.refresh')}</button>
    </div>
    <div class="list">
      {#if rooms.open === null}
        <p class="empty" aria-busy="true">{t('build.loading')}</p>
      {:else if rooms.openError}
        <p class="empty"><b>{t('lobby.cloud_error_title')}.</b> {rooms.openError}</p>
      {:else if rooms.open.length}
        {#each rooms.open as r (r.id)}
          <div class="lrow lrow-open">
            <Monogram name={r.name} shape="square" tone={4} />
            <div><b>{r.name}</b><span>{t('rooms.open_room_d', { host: r.host_name, version: r.game_version, players: r.player_count, max: r.max_players })}</span></div>
            <button type="button" class="mini" disabled={rooms.joining || launch.busy} onclick={() => rooms.join(r.id)}>{t('lobby.enter_btn')}</button>
          </div>
        {/each}
      {:else}
        <p class="empty"><b>{t('lobby.no_open_rooms')}.</b> {t('rooms.no_open_hint')}</p>
      {/if}
    </div>
  </section>
{/if}

{#if rooms.error}
  <p class="err" role="alert">{rooms.error}</p>
{/if}

<style>
  section + section,
  section + div {
    margin-top: 28px;
  }
  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 16px;
    margin-bottom: 16px;
    border-radius: var(--r-m);
    background: var(--fill-3);
  }
  .room {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .room:hover {
    transform: none;
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .room-head {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
  }
  .room-t b {
    display: block;
    font-size: var(--t-t3);
    font-weight: var(--w-bold);
  }
  .room-t span,
  .tunnel {
    color: var(--label-2);
    font-size: var(--t-sub);
  }
  .tunnel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--separator);
  }
  .tunnel.bad span {
    color: var(--red);
  }
  .join {
    padding: 16px;
    display: grid;
    grid-template-columns: minmax(0, 260px) auto;
    gap: 8px 12px;
    align-items: center;
    justify-content: start;
  }
  .join:hover {
    transform: none;
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .join .d {
    grid-column: 1 / -1;
    margin: 0;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .code-in {
    font-family: var(--font-mono);
    font-size: var(--t-t3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .lrow-lan {
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
    gap: 8px 16px;
  }
  .lrow-open {
    grid-template-columns: 40px minmax(0, 1fr) auto;
  }
  .prow {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 8px;
    border-radius: var(--r-m);
  }
  .prow > div {
    min-width: 0;
  }
  .prow b {
    display: block;
    font-weight: var(--w-semibold);
  }
  .prow > div > span {
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .mini.danger,
  .btn.danger {
    color: var(--red);
  }
  .err {
    margin: 16px 0 0;
    color: var(--red);
    font-size: var(--t-foot);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
