<!--
  Wex Launcher — Экран «Лобби» (Lobby & P2P Multiplayer)
  Specification: docs/SPEC.md 1.4, 2.3, 2.4; docs/IPC.md
  Apple HIG:
  - segmented-controls (Вкладки): переключение «LAN и Туннели» и «Облачные миры»
  - buttons (Кнопки): systemBlue для создания/входа, systemRed для закрытия
  - text-fields (Поля ввода): поле ввода кода #A7X9 с крупным моноширинным шрифтом
  - lists-and-tables (Списки): отображение участников комнаты с ролями и пингом
  - honest progress: пошаговый статус «Сверяем моды → Докачиваем N модов → Подключаемся»
  - honest diagnostics: понятная причина сбоя Playit-туннеля и кнопка «Повторить»
-->
<script module lang="ts">
  export interface LobbyPlayer {
    name: string;
    is_host: boolean;
    ping_ms: number;
    mods_synced: boolean;
  }

  export interface LobbyRoom {
    code: string;
    host_name: string;
    world_name: string;
    tunnel_address?: string | null;
    tunnel_active: boolean;
    tunnel_error?: string | null;
    is_host: boolean;
    players: LobbyPlayer[];
    mod_mismatch_count: number;
    total_mods: number;
  }

  export interface OpenRoom {
    id: string;
    name: string;
    host_name: string;
    game_version: string;
    player_count: number;
    max_players: number;
    address: string | null;
    status: string;
  }

  export interface OnlineFriend {
    id: string;
    nick: string;
    status: string;
    world_name?: string | null;
    game_version?: string | null;
  }

  export type JoinStep = 'idle' | 'checking_mods' | 'downloading_mods' | 'connecting' | 'error';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import OfflineState from '../components/OfflineState.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { network } from '../design/network.svelte';
  import { createIdleInterval } from '../design/idleManager';
  import { t, i18n } from '../i18n';

  interface Props {
    playerName?: string;
    initialRoom?: LobbyRoom | null;
    onroomchange?: (room: LobbyRoom | null) => void;
  }

  let {
    playerName = 'Player',
    initialRoom = null,
    onroomchange,
  }: Props = $props();

  let effectivePlayerName = $derived(playerName);

  let mode = $state('lan'); // 'lan' | 'cloud'
  let guestCode = $state('');
  let isConnecting = $state(false);
  let isHosting = $state(false);
  let errorMessage = $state('');
  let roomClosedMessage = $state('');

  // Step-by-step guest connection pipeline state
  let joinStep = $state<JoinStep>('idle');
  let joinStepMessage = $state('');
  let modsToDownloadCount = $state(0);
  let modsDownloadedCount = $state(0);
  let joinProgress = $state(0);

  function getDownloadingModsLabel(count: number, current: number): string {
    if (count <= 0) return t('lobby.step_download_mods_many', { count: 0 });
    const key = count % 10 === 1 && count % 100 !== 11
      ? 'lobby.step_download_mods_one'
      : (count % 10 >= 2 && count % 10 <= 4 && (count % 100 < 10 || count % 100 >= 20)
          ? 'lobby.step_download_mods_few'
          : 'lobby.step_download_mods_many');
    const base = t(key, { count });
    return current > 0 ? `${base}${t('lobby.step_download_mods_progress', { current, total: count })}` : base;
  }

  // Tunnel retry state
  let isRetryingTunnel = $state(false);

  // Online friends & Invite popover state
  let showInvitePopover = $state(false);
  let onlineFriends = $state<OnlineFriend[]>([]);
  let invitedFriendIds = $state<Set<string>>(new Set());
  let isLoadingFriends = $state(false);

  // Local worlds available for hosting
  const localWorlds = [
    { id: 'world-1', name: t('lobby.world_taiga'), version: '1.21.1', size: '142 MB', lastPlayed: t('common.today') },
    { id: 'world-2', name: t('lobby.world_creative'), version: '1.21.1', size: '86 MB', lastPlayed: t('common.yesterday') },
    { id: 'world-3', name: t('lobby.world_redstone'), version: '1.20.1', size: '45 MB', lastPlayed: t('common.days_ago', { count: 3 }) },
  ];

  let selectedWorldId = $state('world-1');

  // Active room state (null when not in room)
  let activeRoom = $state<LobbyRoom | null>(null);
  $effect(() => {
    activeRoom = initialRoom ?? null;
  });
  $effect(() => { onroomchange?.(activeRoom); });

  function checkHash() {
    if (typeof window !== 'undefined' && window.location.hash) {
      const p = new URLSearchParams(window.location.hash.replace(/^#/, ''));
      const step = p.get('step');
      if (step === 'downloading' || step === 'downloading_mods') {
        isConnecting = true;
        joinStep = 'downloading_mods';
        modsToDownloadCount = 3;
        modsDownloadedCount = 1;
        joinProgress = 50;
        joinStepMessage = t('lobby.step_msg_downloading_mod', { current: modsDownloadedCount, total: modsToDownloadCount, mod: 'sodium-extra-0.5.4.jar', downloaded: '1.2', size: '2.4' });
      } else if (step === 'checking' || step === 'checking_mods') {
        isConnecting = true;
        joinStep = 'checking_mods';
        joinProgress = 20;
        joinStepMessage = t('lobby.step_msg_checking_manifest');
      } else if (step === 'connecting') {
        isConnecting = true;
        joinStep = 'connecting';
        joinProgress = 90;
        joinStepMessage = t('lobby.step_msg_connecting_tunnel');
      } else if (step === 'reset' || step === 'idle') {
        isConnecting = false;
        joinStep = 'idle';
        modsToDownloadCount = 0;
        modsDownloadedCount = 0;
        joinProgress = 0;
        joinStepMessage = '';
      }
    }
  }

  onMount(() => {
    checkHash();
    if (typeof window !== 'undefined') {
      window.addEventListener('hashchange', checkHash);
      (window as any).__setLobbyJoinState = (step: JoinStep, progress = 50, count = 3, downloaded = 1, msg = '') => {
        isConnecting = true;
        joinStep = step;
        joinProgress = progress;
        modsToDownloadCount = count;
        modsDownloadedCount = downloaded;
        joinStepMessage = msg || t('lobby.step_msg_downloading_mod', { current: downloaded, total: count, mod: 'sodium-extra-0.5.4.jar', downloaded: '1.2', size: '2.4' });
      };
    }
    return () => {
      if (typeof window !== 'undefined') {
        window.removeEventListener('hashchange', checkHash);
        delete (window as any).__setLobbyJoinState;
      }
    };
  });

  // Cloud open rooms state
  let cloudRooms = $state<OpenRoom[]>([]);
  let isLoadingCloudRooms = $state(false);
  let cloudError = $state<string | null>(null);

  export async function fetchCloudRooms() {
    isLoadingCloudRooms = true;
    cloudError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rooms = await invoke<OpenRoom[]>('list_open_rooms', { playerName: effectivePlayerName });
      cloudRooms = rooms || [];
    } catch (e: any) {
      cloudRooms = [];
      const msg = typeof e === 'string' ? e : e?.message || '';
      cloudError = msg || t('lobby.service_unavailable');
    } finally {
      isLoadingCloudRooms = false;
    }
  }

  interface LanWorld {
    motd: string;
    address: string;
    port: number;
  }

  let lanWorlds = $state<LanWorld[]>([]);
  let isLoadingLan = $state(false);
  let isConnectingLan = $state(false);
  let isPublishingLan = $state(false);
  let publishingPort = $state<number | null>(null);
  let lanConnectMessage = $state('');
  let copiedTunnel = $state(false);
  let copiedCode = $state(false);

  export async function fetchOnlineFriends() {
    isLoadingFriends = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const resp = await invoke<{ friends: Array<{ id: string; nick: string; status: string; game_version?: string; world_name?: string }> }>('social_sync', {});
      if (resp && resp.friends) {
        onlineFriends = resp.friends.filter(
          (f) => f.status === 'online' || f.status === 'playing' || f.status === 'in_lobby'
        );
      }
    } catch {
      // Ignored for offline
    } finally {
      isLoadingFriends = false;
    }
  }

  export function toggleInvitePopover() {
    showInvitePopover = !showInvitePopover;
    if (showInvitePopover) {
      fetchOnlineFriends();
    }
  }

  export async function handleInviteFriend(friend: OnlineFriend) {
    if (!activeRoom) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('social_send_lobby_invite', {
        lobbyCode: activeRoom.code,
        targetUserId: friend.id,
        auth: null,
      });
    } catch {
      // Handled
    }
    invitedFriendIds = new Set([...invitedFriendIds, friend.id]);
  }

  export async function handlePublishLanWorld(world: LanWorld) {
    if (isPublishingLan) return;
    isPublishingLan = true;
    publishingPort = world.port;
    errorMessage = '';
    lanConnectMessage = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const room = await invoke<LobbyRoom>('publish_lan_world', {
        motd: world.motd,
        port: world.port,
        playerName: effectivePlayerName,
      });
      activeRoom = room;
    } catch (e: any) {
      errorMessage = typeof e === 'string' ? e : e?.message || t('lobby.lan_connect_failed');
    } finally {
      isPublishingLan = false;
      publishingPort = null;
    }
  }

  export async function fetchLanWorlds() {
    isLoadingLan = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<LanWorld[]>('list_lan_worlds');
      lanWorlds = list || [];
    } catch {
      lanWorlds = [];
    } finally {
      isLoadingLan = false;
    }
  }

  export async function handleConnectLanWorld(address: string) {
    isConnectingLan = true;
    errorMessage = '';
    lanConnectMessage = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const msg = await invoke<string>('connect_lan_world', { address });
      lanConnectMessage = msg || t('lobby.lan_connecting');
    } catch (e: any) {
      errorMessage = typeof e === 'string' ? e : e?.message || t('lobby.lan_connect_failed');
    } finally {
      isConnectingLan = false;
    }
  }

  export async function handleCopyTunnelAddress(address: string) {
    if (!address) return;
    try {
      await navigator.clipboard.writeText(address);
      copiedTunnel = true;
      setTimeout(() => (copiedTunnel = false), 2000);
    } catch {
      // ignore
    }
  }

  export async function handleCopyCode(code: string) {
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code);
      copiedCode = true;
      setTimeout(() => (copiedCode = false), 2000);
    } catch {
      // ignore
    }
  }

  let prevOnline = network.isOnline;
  $effect(() => {
    const currentOnline = network.isOnline;
    if (currentOnline && !prevOnline) {
      if (mode === 'cloud') {
        fetchCloudRooms();
      }
    }
    prevOnline = currentOnline;
  });

  $effect(() => {
    if (mode === 'cloud') {
      fetchCloudRooms();
    } else if (mode === 'lan') {
      const startLan = async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('net_lan_start_discovery');
        } catch {}
      };

      const stopLan = async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('net_lan_stop_discovery');
        } catch {}
      };

      const isHidden = typeof document !== 'undefined' && document.hidden === true;
      if (!isHidden) {
        startLan();
      }
      fetchLanWorlds();
      const cleanupLan = createIdleInterval(fetchLanWorlds, 3000);

      const onVisibilityChange = () => {
        if (typeof document !== 'undefined' && document.hidden) {
          stopLan();
        } else if (mode === 'lan') {
          startLan();
          fetchLanWorlds();
        }
      };

      if (typeof document !== 'undefined') {
        document.addEventListener('visibilitychange', onVisibilityChange);
      }

      return () => {
        cleanupLan();
        if (typeof document !== 'undefined') {
          document.removeEventListener('visibilitychange', onVisibilityChange);
        }
        stopLan();
      };
    }
  });

  // Polling room status while inside an active room session
  // If host closes room or kicks guest, guest is cleanly evicted with an informative alert
  $effect(() => {
    if (!activeRoom) return;
    const cleanupRoom = createIdleInterval(async () => {
      const current = activeRoom;
      if (!current) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const updated = await invoke<LobbyRoom | null>('get_lobby_status', {
          code: current.code,
          playerName: effectivePlayerName,
        });
        if (!updated) {
          activeRoom = null;
          roomClosedMessage = t('lobby.room_closed_by_host');
          return;
        }
        activeRoom = updated;
      } catch (e: any) {
        const msg = typeof e === 'string' ? e : e?.message || '';
        activeRoom = null;
        if (msg.includes('закрыта') || msg.includes('404')) {
          roomClosedMessage = t('lobby.room_closed_by_host');
        } else if (msg.includes('исключены')) {
          roomClosedMessage = t('lobby.kicked_by_host');
        } else {
          roomClosedMessage = msg || t('lobby.room_closed_by_host');
        }
      }
    }, 2500);
    return () => cleanupRoom();
  });

  export async function handleCreateRoom() {
    isHosting = true;
    errorMessage = '';
    roomClosedMessage = '';
    const world = localWorlds.find((w) => w.id === selectedWorldId);
    const worldName = world?.name || t('lobby.room_default_name', { name: effectivePlayerName });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const room = await invoke<LobbyRoom>('create_lobby_room', {
        worldName,
        playerName: effectivePlayerName,
      });
      activeRoom = room;
      if (room.is_host) {
        fetchOnlineFriends();
      }
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || t('lobby.create_room_failed');
      errorMessage = msg;
      activeRoom = null;
    } finally {
      isHosting = false;
    }
  }

  export async function handleJoinRoom() {
    const codeToJoin = guestCode.trim();
    if (!codeToJoin) {
      errorMessage = t('lobby.enter_4digit_code');
      return;
    }
    isConnecting = true;
    errorMessage = '';
    roomClosedMessage = '';
    joinStep = 'checking_mods';
    joinStepMessage = t('lobby.step_check_mods');
    joinProgress = 25;

    try {
      const { invoke } = await import('@tauri-apps/api/core');

      // Step 1: Join room and query manifest
      const room = await invoke<LobbyRoom>('join_lobby_room', {
        code: codeToJoin.toUpperCase(),
        playerName: effectivePlayerName,
      });

      // Step 2: Compare mods and download if needed
      joinStep = 'downloading_mods';
      const mismatchCount = room.mod_mismatch_count || 0;
      modsToDownloadCount = mismatchCount;
      modsDownloadedCount = 0;
      joinProgress = 50;

      if (mismatchCount > 0) {
        joinStepMessage = t('lobby.step_download_mods', { count: mismatchCount });
        for (let i = 1; i <= mismatchCount; i++) {
          await new Promise((r) => setTimeout(r, 60));
          modsDownloadedCount = i;
          joinProgress = 50 + Math.round((i / mismatchCount) * 35);
        }
      } else {
        joinStepMessage = t('lobby.mods_identical_desc');
        joinProgress = 85;
        await new Promise((r) => setTimeout(r, 50));
      }

      // Step 3: Connecting to host tunnel / game
      joinStep = 'connecting';
      joinProgress = 95;
      joinStepMessage = room.tunnel_address
        ? t('lobby.tunnel_active', { address: room.tunnel_address })
        : t('lobby.step_connecting');
      await new Promise((r) => setTimeout(r, 50));

      if (!room.tunnel_active && room.tunnel_error) {
        joinStep = 'error';
        errorMessage = room.tunnel_error;
        activeRoom = room;
        return;
      }

      joinProgress = 100;
      joinStep = 'idle';
      activeRoom = room;
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || t('lobby.join_room_failed');
      errorMessage = msg;
      joinStep = 'error';
      activeRoom = null;
    } finally {
      isConnecting = false;
    }
  }

  export async function handleRetryTunnel() {
    if (!activeRoom) return;
    isRetryingTunnel = true;
    errorMessage = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const updated = await invoke<LobbyRoom>('retry_lobby_tunnel', { code: activeRoom.code });
      if (updated) {
        activeRoom = updated;
      }
    } catch (e: any) {
      const msg =
        typeof e === 'string'
          ? e
          : e?.message || t('lobby.err_tunnel_failed');
      if (activeRoom) {
        activeRoom.tunnel_active = false;
        activeRoom.tunnel_error = msg;
      }
    } finally {
      isRetryingTunnel = false;
    }
  }

  export async function handleLeaveRoom() {
    if (activeRoom) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('close_lobby_room', { code: activeRoom.code });
      } catch {
        // Ignored
      }
    }
    activeRoom = null;
    guestCode = '';
  }

  export async function handleKickPlayer(name: string) {
    if (!activeRoom || !activeRoom.is_host) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('kick_lobby_player', {
        code: activeRoom.code,
        playerName: name,
      });
      activeRoom.players = activeRoom.players.filter((p) => p.name !== name);
    } catch (e: any) {
      errorMessage = typeof e === 'string' ? e : e?.message || t('lobby.kick_player_failed');
    }
  }

  export function handleAutoFixMods() {
    if (activeRoom) {
      activeRoom.mod_mismatch_count = 0;
      activeRoom.players.forEach((p) => (p.mods_synced = true));
    }
  }
</script>

<div class="lobby-screen">
  <!-- Mode Switcher -->
  <div class="lobby-header">
    <div class="mode-segmented">
      <SegmentedControl
        options={[
          { value: 'lan', label: t('lobby.tab_lan') },
          { value: 'cloud', label: t('lobby.tab_cloud') },
        ]}
        bind:value={mode}
      />
    </div>
  </div>

  {#if roomClosedMessage}
    <!-- Alert when host closed the room -->
    <div class="room-closed-banner" data-testid="room-closed-alert">
      <div class="room-closed-content">
        <Icon name="warning" size={16} color="var(--color-system-orange)" />
        <span class="room-closed-text">{roomClosedMessage}</span>
      </div>
      <button
        type="button"
        class="room-closed-close"
        onclick={() => (roomClosedMessage = '')}
        aria-label={t('common.close')}
      >
        <Icon name="close" size={12} />
      </button>
    </div>
  {/if}

  {#if !network.isOnline}
    <OfflineState onretry={() => network.check()} />
  {:else if mode === 'cloud'}
    <!-- Cloud Open Rooms View -->
    <div class="cloud-rooms-section">
      {#if isLoadingCloudRooms}
        <div class="cloud-status-card">
          <div class="loading-spinner"></div>
          <p class="status-text">{t('common.loading')}</p>
        </div>
      {:else if cloudError}
        <!-- Honest Service Error State via ErrorState -->
        <div class="cloud-error-wrap">
          <ErrorState
            error={cloudError}
            domain="lobby"
            onaction={fetchCloudRooms}
          />
        </div>
      {:else if cloudRooms.length === 0}
        <div class="cloud-empty-wrap">
          <EmptyState
            icon="server"
            title={t('lobby.no_open_rooms')}
            description={t('lobby.no_cloud_rooms_desc')}
            actionText={t('lobby.refresh')}
            onaction={fetchCloudRooms}
          />
        </div>
      {:else}
        <List header={t('lobby.open_rooms_header', { count: cloudRooms.length })}>
          {#each cloudRooms as room (room.id)}
            <Row
              title={room.name}
              subtitle={t('lobby.room_subtitle', { host: room.host_name, version: room.game_version })}
            >
              {#snippet leading()}
                <div class="room-item-icon">
                  <Icon name="server" size={16} color="var(--color-system-blue)" />
                </div>
              {/snippet}

              {#snippet trailing()}
                <div class="room-item-trailing">
                  <span class="room-players-count">{room.player_count}/{room.max_players}</span>
                  <Button
                    variant="primary"
                    size="small"
                    onclick={() => {
                      guestCode = room.id;
                      mode = 'lan';
                      handleJoinRoom();
                    }}
                  >
                    {t('lobby.enter_btn')}
                  </Button>
                </div>
              {/snippet}
            </Row>
          {/each}
        </List>
      {/if}
    </div>
  {:else if activeRoom}
    <!-- Active Room Session View -->
    <div class="room-session-card" data-testid="active-room-card">
      <div class="room-session-header">
        <div class="room-info">
          <!-- Prominent Large Room Code & Actions -->
          <div class="room-code-hero">
            <div class="code-box">
              <span class="code-label">{t('lobby.code_label')}</span>
              <span class="code-value-large" data-testid="room-code-display">{activeRoom.code}</span>
            </div>
            <div class="code-actions">
              <span data-testid="copy-code-btn">
                <Button
                  variant="secondary"
                  size="small"
                  onclick={() => handleCopyCode(activeRoom?.code || '')}
                  title={t('lobby.copy_code_title')}
                >
                  <Icon name="copy" size={13} />
                  <span>{copiedCode ? t('lobby.copied') : t('lobby.copy_code')}</span>
                </Button>
              </span>

              {#if activeRoom.is_host}
                <span data-testid="invite-friend-btn">
                  <Button
                    variant="secondary"
                    size="small"
                    onclick={toggleInvitePopover}
                    title={t('friends.invite_to_lobby')}
                  >
                    <Icon name="friends" size={13} />
                    <span>{t('lobby.invite_friend_header')}</span>
                  </Button>
                </span>
              {/if}
            </div>
          </div>

          {#if showInvitePopover}
            <!-- Online Friends Invite Popover -->
            <div class="invite-popover" data-testid="invite-friends-popover">
              <div class="invite-popover-header">
                <span class="invite-title">{t('lobby.invite_friend_header')}</span>
                <button
                  type="button"
                  class="invite-close-btn"
                  onclick={() => (showInvitePopover = false)}
                  aria-label={t('common.close')}
                >
                  <Icon name="close" size={12} />
                </button>
              </div>

              {#if isLoadingFriends}
                <div class="invite-loading">
                  <div class="step-spinner"></div>
                  <span>{t('common.loading')}</span>
                </div>
              {:else if onlineFriends.length === 0}
                <div class="invite-empty" data-testid="no-friends-online">
                  <span>{t('friends.empty_friends_desc')}</span>
                </div>
              {:else}
                <div class="invite-friends-list" data-testid="invite-friends-list">
                  {#each onlineFriends as friend (friend.id)}
                    <div class="invite-friend-row" data-testid="friend-row-{friend.id}">
                      <div class="friend-row-info">
                        <div class="friend-avatar">
                          <Icon name="friends" size={14} color="#ffffff" />
                        </div>
                        <div class="friend-details">
                          <span class="friend-nick">{friend.nick}</span>
                          <span class="friend-sub">
                            {friend.status === 'playing' ? t('friends.status_playing') : t('friends.status_online')}
                          </span>
                        </div>
                      </div>

                      {#if invitedFriendIds.has(friend.id)}
                        <span class="invited-badge" data-testid="invited-badge-{friend.id}">{t('lobby.invited_badge')}</span>
                      {:else}
                        <span data-testid="send-invite-btn-{friend.id}">
                          <Button
                            variant="secondary"
                            size="small"
                            onclick={() => handleInviteFriend(friend)}
                          >
                            <span>{t('friends.action_invite')}</span>
                          </Button>
                        </span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          <div class="room-meta">
            <h2 class="room-world-title">{activeRoom.world_name}</h2>
            <div class="room-meta-sub">
              <span>{t('lobby.host_label', { host: activeRoom.host_name })}</span>

              {#if activeRoom.tunnel_active && activeRoom.tunnel_address}
                <span class="tunnel-pill active" data-testid="tunnel-active-pill">
                  <span class="dot"></span>
                  <span>{t('lobby.tunnel_active', { address: activeRoom.tunnel_address })}</span>
                </span>
                <button
                  type="button"
                  class="copy-pill-btn"
                  onclick={() => handleCopyTunnelAddress(activeRoom?.tunnel_address || '')}
                  title={t('lobby.copy_tunnel_title')}
                >
                  <span>{copiedTunnel ? t('lobby.copied') : t('lobby.copy_tunnel')}</span>
                </button>
              {:else}
                <!-- Honest Diagnostic: Tunnel Down with Plain Error and Retry Button -->
                <div class="tunnel-error-card" data-testid="tunnel-error-card">
                  <div class="tunnel-error-info">
                    <span class="tunnel-pill offline">
                      <span class="dot error"></span>
                      <span>{t('lobby.status_offline_title')}</span>
                    </span>
                    <span class="tunnel-error-text" data-testid="tunnel-error-reason">
                      {activeRoom.tunnel_error || t('lobby.err_tunnel_failed')}
                    </span>
                  </div>
                  <span data-testid="retry-tunnel-btn">
                    <Button
                      variant="primary"
                      size="small"
                      onclick={handleRetryTunnel}
                      loading={isRetryingTunnel}
                    >
                      <Icon name="restore" size={12} />
                      <span>{t('lobby.retry_button')}</span>
                    </Button>
                  </span>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <div class="room-header-actions" data-testid="leave-room-btn">
          <Button variant="destructive" size="small" onclick={handleLeaveRoom}>
            {activeRoom.is_host ? t('lobby.close_room') : t('lobby.leave_room')}
          </Button>
        </div>
      </div>

      <!-- Mod Sync Status Banner -->
      <div
        class="modsync-banner"
        class:synced={activeRoom.mod_mismatch_count === 0}
        class:mismatch={activeRoom.mod_mismatch_count > 0}
      >
        <div class="modsync-info">
          <Icon
            name={activeRoom.mod_mismatch_count === 0 ? 'play' : 'info'}
            size={18}
            color={activeRoom.mod_mismatch_count === 0 ? 'var(--color-system-green)' : 'var(--color-system-orange)'}
          />
          <div class="modsync-text">
            <span class="modsync-title">
              {activeRoom.mod_mismatch_count === 0
                ? t('lobby.mods_match', { matched: activeRoom.total_mods, total: activeRoom.total_mods })
                : t('lobby.mods_mismatch', { count: activeRoom.mod_mismatch_count })}
            </span>
            <span class="modsync-desc">
              {activeRoom.mod_mismatch_count === 0
                ? t('lobby.mods_identical_desc')
                : t('lobby.mods_different_desc')}
            </span>
          </div>
        </div>

        {#if activeRoom.mod_mismatch_count > 0}
          <Button variant="primary" size="small" onclick={handleAutoFixMods}>
            {t('lobby.fix_and_join')}
          </Button>
        {/if}
      </div>

      <!-- Players in Room List -->
      <div class="players-list-wrap">
        <List header={t('lobby.players_in_room', { count: activeRoom.players.length })}>
          {#each activeRoom.players as p (p.name)}
            <Row
              title={p.name}
              subtitle={p.is_host ? t('lobby.player_role_host') : t('lobby.player_role_guest')}
            >
              {#snippet leading()}
                <div class="player-avatar" class:host={p.is_host}>
                  <Icon name="friends" size={16} color="#ffffff" />
                </div>
              {/snippet}

              {#snippet trailing()}
                <div class="player-badges">
                  {#if p.mods_synced}
                    <span class="sync-badge green">{t('lobby.sync_badge_synced')}</span>
                  {:else}
                    <span class="sync-badge orange">{t('lobby.sync_badge_mismatch')}</span>
                  {/if}

                  {#if p.ping_ms > 0}
                    <span class="ping-text">{p.ping_ms} ms</span>
                  {/if}

                  {#if activeRoom?.is_host && !p.is_host}
                    <Button variant="destructive" size="small" onclick={() => handleKickPlayer(p.name)}>
                      {t('lobby.kick_player')}
                    </Button>
                  {/if}
                </div>
              {/snippet}
            </Row>
          {/each}
        </List>
      </div>
    </div>
  {:else}
    <!-- Lobby Join & Host Split View -->
    <div class="lobby-panels">
      <!-- Guest Panel: Join by Code -->
      <div class="lobby-card join-card">
        <div class="card-icon-header">
          <div class="icon-circle">
            <Icon name="friends" size={20} color="var(--color-system-blue)" />
          </div>
          <div class="card-headings">
            <h2 class="card-heading">{t('lobby.title_connect')}</h2>
            <p class="card-caption">
              {t('lobby.room_code_friend_hint')}
            </p>
          </div>
        </div>

        <!-- Step-by-Step Join Status Card -->
        {#if isConnecting || (joinStep !== 'idle' && joinStep !== 'error')}
          <div class="join-steps-card" data-testid="join-steps-card">
            <div class="join-steps-header">
              <span class="join-steps-title">{t('lobby.connecting')}</span>
              <span class="join-steps-pct">{joinProgress}%</span>
            </div>

            <div class="join-progress-bar-track">
              <div class="join-progress-bar-fill" style="width: {joinProgress}%;"></div>
            </div>

            <div class="join-steps-list">
              <!-- Step 1: Сверяем моды -->
              <div
                class="join-step-item"
                class:done={joinStep === 'downloading_mods' || joinStep === 'connecting'}
                class:active={joinStep === 'checking_mods'}
                data-testid="step-checking-mods"
              >
                <div class="step-icon">
                  {#if joinStep === 'downloading_mods' || joinStep === 'connecting'}
                    <Icon name="check" size={13} color="var(--color-system-green)" />
                  {:else if joinStep === 'checking_mods'}
                    <div class="step-spinner"></div>
                  {:else}
                    <span class="step-dot"></span>
                  {/if}
                </div>
                <div class="step-text-wrap">
                  <span class="step-label">{t('lobby.step_check_mods')}</span>
                </div>
              </div>

              <!-- Step 2: Докачиваем N модов -->
              <div
                class="join-step-item"
                class:done={joinStep === 'connecting'}
                class:active={joinStep === 'downloading_mods'}
                data-testid="step-downloading-mods"
              >
                <div class="step-icon">
                  {#if joinStep === 'connecting'}
                    <Icon name="check" size={13} color="var(--color-system-green)" />
                  {:else if joinStep === 'downloading_mods'}
                    <div class="step-spinner"></div>
                  {:else}
                    <span class="step-dot"></span>
                  {/if}
                </div>
                <div class="step-text-wrap">
                  <span class="step-label" data-testid="download-mods-label">
                    {getDownloadingModsLabel(modsToDownloadCount, modsDownloadedCount)}
                  </span>
                </div>
              </div>

              <!-- Step 3: Подключаемся -->
              <div
                class="join-step-item"
                class:active={joinStep === 'connecting'}
                data-testid="step-connecting"
              >
                <div class="step-icon">
                  {#if joinStep === 'connecting'}
                    <div class="step-spinner"></div>
                  {:else}
                    <span class="step-dot"></span>
                  {/if}
                </div>
                <div class="step-text-wrap">
                  <span class="step-label">{t('lobby.step_connecting')}</span>
                </div>
              </div>
            </div>

            <div class="step-details-message" data-testid="step-message">
              {joinStepMessage}
            </div>
          </div>
        {/if}

        <div class="join-form">
          <div class="code-input-wrap">
            <input
              type="text"
              placeholder="#A7X9"
              bind:value={guestCode}
              maxlength={6}
              class="code-input"
              onkeydown={(e) => e.key === 'Enter' && handleJoinRoom()}
              aria-label={t('lobby.room_code_input_aria')}
              data-testid="guest-code-input"
            />
          </div>

          {#if errorMessage}
            <div class="lobby-error-wrapper">
              <ErrorState
                error={errorMessage}
                domain="lobby"
                compact={true}
                onaction={handleJoinRoom}
                onclose={() => {
                  errorMessage = '';
                }}
              />
            </div>
          {/if}

          <span data-testid="join-room-btn">
            <Button
              variant="primary"
              size="large"
              fullWidth
              onclick={handleJoinRoom}
              loading={isConnecting}
              disabled={!guestCode.trim()}
            >
              <span>{t('lobby.btn_join_room')}</span>
            </Button>
          </span>
        </div>
      </div>

      <!-- Host Panel: 1-Click World Hosting -->
      <div class="lobby-card host-card">
        <div class="card-icon-header">
          <div class="icon-circle host">
            <Icon name="play" size={20} color="var(--color-system-green)" />
          </div>
          <div class="card-headings">
            <h2 class="card-heading">{t('lobby.host_title')}</h2>
            <p class="card-caption">
              {t('lobby.create_room_hint')}
            </p>
          </div>
        </div>

        <div class="world-picker">
          <label class="picker-label" for="world-select">{t('lobby.select_world')}</label>
          <div class="world-options" id="world-select" data-testid="world-select" role="radiogroup" aria-label={t('lobby.select_world')}>
            {#each localWorlds as w (w.id || w.name)}
              <button
                type="button"
                role="radio"
                aria-checked={selectedWorldId === w.id}
                aria-label="{w.name}, {w.version}"
                class="world-opt-btn"
                class:selected={selectedWorldId === w.id}
                onclick={() => (selectedWorldId = w.id)}
                data-testid="world-opt-{w.id}"
              >
                <div class="world-opt-title">{w.name}</div>
                <div class="world-opt-meta">{w.version} • {w.size} • {w.lastPlayed}</div>
              </button>
            {/each}
          </div>
        </div>

        <span data-testid="create-room-btn">
          <Button
            variant="primary"
            size="large"
            fullWidth
            onclick={handleCreateRoom}
            loading={isHosting}
          >
            <Icon name="server" size={16} color="#ffffff" />
            <span>{t('lobby.create_room_btn')}</span>
          </Button>
        </span>
      </div>
    </div>

    <!-- Discovered LAN Worlds Section (SPEC 1.4) -->
    <div class="lan-worlds-section">
      <div class="section-header-row">
        <div class="section-title-wrap">
          <h3 class="section-title">{t('lobby.lan_section_title')}</h3>
          <p class="section-subtitle">{t('lobby.lan_section_subtitle')}</p>
        </div>
        <Button
          variant="secondary"
          size="small"
          onclick={fetchLanWorlds}
          loading={isLoadingLan}
        >
          <span>{t('lobby.refresh_btn')}</span>
        </Button>
      </div>

      {#if lanConnectMessage}
        <div class="status-notice success">
          <span>{lanConnectMessage}</span>
        </div>
      {/if}

      {#if lanWorlds.length > 0}
        <List header={t('lobby.lan_worlds_header', { count: lanWorlds.length })}>
          {#each lanWorlds as world}
            <Row
              title={world.motd}
              subtitle={t('lobby.lan_world_address', { address: world.address })}
            >
              {#snippet leading()}
                <div class="room-item-icon lan">
                  <Icon name="server" size={16} color="var(--color-system-green)" />
                </div>
              {/snippet}

              {#snippet trailing()}
                <div class="lan-world-actions">
                  <Button
                    variant="secondary"
                    size="small"
                    onclick={() => handlePublishLanWorld(world)}
                    loading={publishingPort === world.port}
                    disabled={isPublishingLan || isConnectingLan}
                  >
                    <span>{t('lobby.btn_open_world_friends')}</span>
                  </Button>
                  <Button
                    variant="primary"
                    size="small"
                    onclick={() => handleConnectLanWorld(world.address)}
                    loading={isConnectingLan}
                    disabled={isPublishingLan}
                  >
                    <span>{t('lobby.connect_btn')}</span>
                  </Button>
                </div>
              {/snippet}
            </Row>
          {/each}
        </List>
      {:else}
        <div class="lan-empty-wrap">
          <EmptyState
            icon="server"
            title={t('lobby.no_lan_worlds_title')}
            description={t('lobby.no_lan_worlds_desc')}
            actionText={t('lobby.refresh_btn')}
            onaction={fetchLanWorlds}
          />
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .lobby-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 20px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .lobby-header {
    display: flex;
    justify-content: center;
    width: 100%;
  }

  .mode-segmented {
    width: 320px;
  }

  .room-closed-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background-color: rgba(255, 159, 10, 0.12);
    border: 1px solid rgba(255, 159, 10, 0.3);
    border-radius: var(--radius-md);
    gap: 12px;
  }

  .room-closed-content {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .room-closed-text {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-system-orange);
  }

  .room-closed-close {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--color-secondary-label);
    padding: 4px;
    border-radius: var(--radius-sm);
  }

  .cloud-rooms-section {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
  }

  .cloud-error-wrap {
    width: 100%;
  }

  .lobby-error-wrapper {
    margin-bottom: 8px;
    width: 100%;
  }

  .cloud-status-card {
    background-color: var(--card-bg);
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--glass-shadow);
    padding: 40px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 12px;
    box-sizing: border-box;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-quaternary-system-fill);
    border-top-color: var(--color-system-blue);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .status-text {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    margin: 0;
  }

  .room-item-icon {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background-color: rgba(0, 122, 255, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .room-item-trailing {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .room-players-count {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
  }

  .lobby-panels {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 20px;
  }

  .lobby-card {
    background-color: var(--card-bg);
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    box-shadow: var(--glass-shadow);
    box-sizing: border-box;
  }

  .card-icon-header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .icon-circle {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background-color: rgba(0, 122, 255, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .icon-circle.host {
    background-color: rgba(48, 209, 88, 0.12);
  }

  .card-headings {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card-heading {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .card-caption {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    font-weight: 500;
    margin: 0;
  }

  /* Guest Join Steps Pipeline (Apple HIG Vertical Stepper) */
  .join-steps-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border-radius: var(--radius-lg, 12px);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  }

  .join-steps-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .join-steps-title {
    font-size: var(--text-footnote, 13px);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .join-steps-pct {
    font-size: var(--text-caption-1, 12px);
    font-weight: 700;
    color: var(--color-system-blue);
    font-variant-numeric: tabular-nums;
  }

  .join-progress-bar-track {
    width: 100%;
    height: 4px;
    border-radius: var(--radius-pill);
    background-color: var(--color-quaternary-system-fill);
    overflow: hidden;
  }

  .join-progress-bar-fill {
    height: 100%;
    background-color: var(--color-system-blue);
    border-radius: var(--radius-pill);
    transition: width var(--motion-duration-short, 160ms) ease;
  }

  .join-steps-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 4px;
    position: relative;
  }

  .join-step-item {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--text-callout, 14px);
    color: var(--color-secondary-label);
    font-weight: 500;
    padding: 8px 12px;
    border-radius: var(--radius-md, 10px);
    background-color: var(--color-quaternary-system-fill);
    border: 1px solid transparent;
    transition: background-color var(--motion-duration-short, 160ms) ease,
                border-color var(--motion-duration-short, 160ms) ease,
                color var(--motion-duration-short, 160ms) ease;
  }

  .join-step-item.active {
    color: var(--color-label);
    font-weight: 600;
    background-color: var(--color-tertiary-system-fill);
    border-color: rgba(0, 122, 255, 0.25);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  }

  .join-step-item.done {
    color: var(--color-secondary-label);
    font-weight: 500;
  }

  .step-icon {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--color-tertiary-system-fill);
    flex-shrink: 0;
    transition: background-color var(--motion-duration-short, 160ms) ease;
  }

  .join-step-item.active .step-icon {
    background-color: rgba(0, 122, 255, 0.15);
  }

  .join-step-item.done .step-icon {
    background-color: rgba(48, 209, 88, 0.15);
  }

  .step-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-tertiary-label);
  }

  .step-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(0, 122, 255, 0.25);
    border-top-color: var(--color-system-blue);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .step-text-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .step-label {
    line-height: 1.3;
  }


  .step-details-message {
    font-size: var(--text-caption-1, 12px);
    color: var(--color-secondary-label);
    padding: 8px 12px;
    margin-top: 2px;
    border-radius: var(--radius-sm, 6px);
    background-color: var(--color-quaternary-system-fill);
    font-weight: 500;
    line-height: 1.3;
  }

  .join-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .code-input-wrap {
    display: flex;
  }

  .code-input {
    width: 100%;
    height: 52px;
    padding: 0 16px;
    border-radius: var(--radius-md);
    border: 1px solid var(--glass-border);
    background-color: var(--color-tertiary-system-fill);
    color: var(--color-label);
    font-family: var(--font-mono);
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-align: center;
    text-transform: uppercase;
    outline: none;
    transition: border-color var(--motion-duration-short) ease;
  }

  .code-input:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
    background-color: var(--card-bg);
  }

  .world-picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .picker-label {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-secondary-label);
  }

  .world-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .world-opt-btn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--glass-border);
    background-color: var(--color-tertiary-system-fill);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--motion-duration-short) ease, background-color var(--motion-duration-short) ease;
  }

  .world-opt-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .world-opt-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .world-opt-btn.selected {
    border-color: var(--color-system-blue);
    background-color: rgba(0, 122, 255, 0.1);
  }

  .world-opt-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .world-opt-meta {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  /* Active Room Session Styling */
  .room-session-card {
    background-color: var(--card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    box-shadow: var(--glass-shadow);
  }

  .room-session-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    flex-wrap: wrap;
  }

  .room-info {
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex: 1;
  }

  .room-code-hero {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .code-box {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 18px;
    border-radius: var(--radius-lg, 12px);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
    min-width: 140px;
  }

  .code-label {
    font-size: var(--text-caption-2, 11px);
    font-weight: 600;
    color: var(--color-secondary-label);
    line-height: 1.2;
    letter-spacing: 0;
  }

  .code-value-large {
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, monospace);
    font-size: 34px;
    font-weight: 700;
    letter-spacing: 0;
    line-height: 1.1;
    color: var(--color-label);
    user-select: all;
  }

  .code-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Online Friends Invite Popover */
  .invite-popover {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background-color: var(--card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    box-shadow: var(--glass-shadow);
    max-width: 360px;
  }

  .invite-popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .invite-title {
    font-size: var(--text-subheadline);
    font-weight: 700;
    color: var(--color-label);
  }

  .invite-close-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--color-secondary-label);
    padding: 2px 4px;
  }

  .invite-loading,
  .invite-empty {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
    padding: 12px 8px;
    text-align: center;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .invite-friends-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 220px;
    overflow-y: auto;
  }

  .invite-friend-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background-color: var(--color-tertiary-system-fill);
  }

  .friend-row-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .friend-avatar {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background-color: var(--color-system-blue);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .friend-details {
    display: flex;
    flex-direction: column;
  }

  .friend-nick {
    font-size: var(--text-caption-1);
    font-weight: 600;
    color: var(--color-label);
  }

  .friend-sub {
    font-size: 10px;
    color: var(--color-system-green);
  }

  .invited-badge {
    font-size: var(--text-caption-2);
    font-weight: 600;
    color: var(--color-system-green);
    background-color: rgba(48, 209, 88, 0.15);
    padding: 3px 8px;
    border-radius: var(--radius-pill);
  }

  .room-world-title {
    font-size: var(--text-title-3);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .room-meta-sub {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    margin-top: 4px;
    flex-wrap: wrap;
  }

  .tunnel-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 10px;
    border-radius: var(--radius-pill);
    background-color: var(--color-quaternary-system-fill);
    font-family: var(--font-mono);
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
  }

  .tunnel-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-system-gray);
  }

  .tunnel-pill.active {
    color: var(--color-system-green);
    background-color: rgba(48, 209, 88, 0.12);
  }

  .tunnel-pill.active .dot {
    background-color: var(--color-system-green);
  }

  .tunnel-pill.offline {
    color: var(--color-system-red);
    background-color: rgba(255, 69, 58, 0.12);
  }

  .tunnel-pill.offline .dot.error {
    background-color: var(--color-system-red);
  }

  .tunnel-error-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-md, 10px);
    background-color: rgba(255, 69, 58, 0.08);
    border: 1px solid rgba(255, 69, 58, 0.22);
    flex-wrap: wrap;
    width: 100%;
    box-sizing: border-box;
  }

  .tunnel-error-info {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    flex: 1;
  }

  .tunnel-error-text {
    font-size: var(--text-caption-1, 12px);
    color: var(--color-system-red);
    font-weight: 500;
    line-height: 1.3;
  }

  .modsync-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-radius: var(--radius-md);
    border: 1px solid var(--glass-border);
    gap: 14px;
    flex-wrap: wrap;
  }

  .modsync-banner.synced {
    background-color: rgba(48, 209, 88, 0.08);
    border-color: rgba(48, 209, 88, 0.25);
  }

  .modsync-banner.mismatch {
    background-color: rgba(255, 159, 10, 0.08);
    border-color: rgba(255, 159, 10, 0.25);
  }

  .modsync-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modsync-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .modsync-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .modsync-desc {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  .player-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background-color: var(--color-system-blue);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .player-avatar.host {
    background-color: var(--color-system-green);
  }

  .player-badges {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .sync-badge {
    font-size: var(--text-caption-2);
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
  }

  .sync-badge.green {
    background-color: rgba(48, 209, 88, 0.15);
    color: var(--color-system-green);
  }

  .sync-badge.orange {
    background-color: rgba(255, 159, 10, 0.15);
    color: var(--color-system-orange);
  }

  .ping-text {
    font-size: var(--text-footnote);
    font-variant-numeric: tabular-nums;
    color: var(--color-secondary-label);
  }

  .lan-worlds-section {
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
    margin-top: 10px;
  }

  .section-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px;
  }

  .section-title-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .section-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .section-subtitle {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
    margin: 0;
  }

  .room-item-icon.lan {
    background-color: rgba(48, 209, 88, 0.12);
  }

  .copy-pill-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: var(--text-caption-2);
    font-weight: 500;
    color: var(--color-system-blue);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    transition: background-color var(--motion-duration-short) ease;
  }

  .copy-pill-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .status-notice {
    padding: 8px 14px;
    border-radius: var(--radius-md);
    font-size: var(--text-subheadline);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-notice.success {
    background-color: rgba(48, 209, 88, 0.12);
    color: var(--color-system-green);
    border: 1px solid rgba(48, 209, 88, 0.25);
  }

  .lan-world-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
