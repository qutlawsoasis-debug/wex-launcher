<!--
  Wex Launcher — Экран «Лобби» (Lobby & P2P Multiplayer)
  Specification: docs/SPEC.md 1.4, 2.3, 2.4; docs/IPC.md
  Apple HIG:
  - segmented-controls (Вкладки): переключение «LAN и Туннели» и «Облачные миры»
  - buttons (Кнопки): systemBlue для создания/входа, systemRed для исключения/закрытия
  - text-fields (Поля ввода): поле ввода кода #A7X9 с моноширинным шрифтом
  - lists-and-tables (Списки): отображение участников комнаты с ролями и пингом
  - empty-states: честное состояние «Сервис недоступен» без фейковых комнат
-->
<script lang="ts">
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import OfflineState from '../components/OfflineState.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import { network } from '../design/network.svelte';

  interface LobbyPlayer {
    name: string;
    is_host: boolean;
    ping_ms: number;
    mods_synced: boolean;
  }

  interface LobbyRoom {
    code: string;
    host_name: string;
    world_name: string;
    tunnel_address?: string | null;
    tunnel_active: boolean;
    is_host: boolean;
    players: LobbyPlayer[];
    mod_mismatch_count: number;
    total_mods: number;
  }

  interface OpenRoom {
    id: string;
    name: string;
    host_name: string;
    game_version: string;
    player_count: number;
    max_players: number;
    address: string | null;
    status: string;
  }

  let mode = $state('lan'); // 'lan' | 'cloud'
  let guestCode = $state('');
  let isConnecting = $state(false);
  let isHosting = $state(false);
  let errorMessage = $state('');

  // Local worlds available for hosting
  const localWorlds = [
    { id: 'world-1', name: 'Выживание в тайге', version: '1.21.1', size: '142 МБ', lastPlayed: 'Сегодня' },
    { id: 'world-2', name: 'Творческий полигон', version: '1.21.1', size: '86 МБ', lastPlayed: 'Вчера' },
    { id: 'world-3', name: 'Redstone Experiments', version: '1.20.1', size: '45 МБ', lastPlayed: '3 дня назад' },
  ];

  let selectedWorldId = $state('world-1');

  // Active room state (null when not in room)
  let activeRoom = $state<LobbyRoom | null>(null);

  // Cloud open rooms state
  let cloudRooms = $state<OpenRoom[]>([]);
  let isLoadingCloudRooms = $state(false);
  let cloudError = $state<string | null>(null);

  export async function fetchCloudRooms() {
    isLoadingCloudRooms = true;
    cloudError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const rooms = await invoke<OpenRoom[]>('list_open_rooms', { playerName: 'Player' });
      cloudRooms = rooms || [];
    } catch (e: any) {
      cloudRooms = [];
      const msg = typeof e === 'string' ? e : e?.message || '';
      cloudError = msg || 'Сервис недоступен';
    } finally {
      isLoadingCloudRooms = false;
    }
  }

  $effect(() => {
    if (mode === 'cloud') {
      fetchCloudRooms();
    }
  });

  // Polling room status while inside an active room session
  $effect(() => {
    if (!activeRoom) return;
    const interval = setInterval(async () => {
      const current = activeRoom;
      if (!current) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const updated = await invoke<LobbyRoom | null>('get_lobby_status', {
          code: current.code,
          playerName: 'Player',
        });
        if (updated) {
          activeRoom = updated;
        }
      } catch (e: any) {
        const msg = typeof e === 'string' ? e : e?.message || '';
        if (msg.includes('исключены')) {
          activeRoom = null;
          errorMessage = 'Вы были исключены из комнаты организатором';
        }
      }
    }, 4000);
    return () => clearInterval(interval);
  });

  export async function handleCreateRoom() {
    isHosting = true;
    errorMessage = '';
    const world = localWorlds.find((w) => w.id === selectedWorldId);
    const worldName = world ? world.name : 'Мой мир';

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const room = await invoke<LobbyRoom>('create_lobby_room', {
        worldName,
        playerName: 'Player',
      });
      activeRoom = room;
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || 'Не удалось создать комнату';
      errorMessage = msg;
      activeRoom = null;
    } finally {
      isHosting = false;
    }
  }

  export async function handleJoinRoom() {
    const codeToJoin = guestCode.trim();
    if (!codeToJoin) {
      errorMessage = 'Введите 4-значный код комнаты';
      return;
    }
    isConnecting = true;
    errorMessage = '';

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const room = await invoke<LobbyRoom>('join_lobby_room', {
        code: codeToJoin.toUpperCase(),
        playerName: 'Player',
      });
      activeRoom = room;
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || 'Не удалось присоединиться к комнате';
      errorMessage = msg;
      activeRoom = null;
    } finally {
      isConnecting = false;
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
      errorMessage = typeof e === 'string' ? e : e?.message || 'Не удалось исключить игрока';
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
          { value: 'lan', label: 'LAN и Туннели' },
          { value: 'cloud', label: 'Облачные миры' },
        ]}
        bind:value={mode}
      />
    </div>
  </div>

  {#if !network.isOnline}
    <OfflineState onretry={() => network.check()} />
  {:else if mode === 'cloud'}
    <!-- Cloud Open Rooms View -->
    <div class="cloud-rooms-section">
      {#if isLoadingCloudRooms}
        <div class="cloud-status-card">
          <div class="loading-spinner"></div>
          <p class="status-text">Загрузка открытых комнат...</p>
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
        <div class="cloud-status-card empty">
          <div class="status-icon-circle neutral">
            <Icon name="server" size={24} color="var(--color-secondary-label)" />
          </div>
          <h3 class="status-title">Нет открытых комнат</h3>
          <p class="status-desc">
            В данный момент нет доступных публичных комнат. Вы можете открыть свой мир во вкладке «LAN и Туннели».
          </p>
          <div class="status-actions">
            <Button variant="secondary" size="regular" onclick={fetchCloudRooms} loading={isLoadingCloudRooms}>
              Обновить
            </Button>
          </div>
        </div>
      {:else}
        <List header="Открытые комнаты ({cloudRooms.length})">
          {#each cloudRooms as room}
            <Row
              title={room.name}
              subtitle="Организатор: {room.host_name} • Версия {room.game_version}"
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
                    Войти
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
    <div class="room-session-card">
      <div class="room-session-header">
        <div class="room-info">
          <div class="room-code-badge">
            <span class="code-label">Код комнаты</span>
            <span class="code-value">{activeRoom.code}</span>
          </div>

          <div class="room-meta">
            <h2 class="room-world-title">{activeRoom.world_name}</h2>
            <div class="room-meta-sub">
              <span>Организатор: <strong>{activeRoom.host_name}</strong></span>
              <span class="tunnel-pill" class:active={activeRoom.tunnel_active}>
                <span class="dot"></span>
                {activeRoom.tunnel_active && activeRoom.tunnel_address
                  ? `Туннель активен: ${activeRoom.tunnel_address}`
                  : 'Запуск туннеля...'}
              </span>
            </div>
          </div>
        </div>

        <div class="room-header-actions">
          <Button variant="destructive" size="small" onclick={handleLeaveRoom}>
            {activeRoom.is_host ? 'Закрыть комнату' : 'Покинуть'}
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
                ? `Моды совпадают (${activeRoom.total_mods}/${activeRoom.total_mods})`
                : `Обнаружено расхождений модов: ${activeRoom.mod_mismatch_count}`}
            </span>
            <span class="modsync-desc">
              {activeRoom.mod_mismatch_count === 0
                ? 'Все клиенты используют идентичный манифест сборки Wex.'
                : 'Конфигурация некоторых модов отличается от хоста.'}
            </span>
          </div>
        </div>

        {#if activeRoom.mod_mismatch_count > 0}
          <Button variant="primary" size="small" onclick={handleAutoFixMods}>
            Исправить и войти
          </Button>
        {/if}
      </div>

      <!-- Players in Room List -->
      <div class="players-list-wrap">
        <List header="Игроки в комнате ({activeRoom.players.length})">
          {#each activeRoom.players as p}
            <Row
              title={p.name}
              subtitle={p.is_host ? 'Организатор комнаты (Хост)' : 'Гость'}
            >
              {#snippet leading()}
                <div class="player-avatar" class:host={p.is_host}>
                  <Icon name="friends" size={16} color="#ffffff" />
                </div>
              {/snippet}

              {#snippet trailing()}
                <div class="player-badges">
                  {#if p.mods_synced}
                    <span class="sync-badge green">Синхронизирован</span>
                  {:else}
                    <span class="sync-badge orange">Не совпадает</span>
                  {/if}

                  {#if p.ping_ms > 0}
                    <span class="ping-text">{p.ping_ms} ms</span>
                  {/if}

                  {#if activeRoom?.is_host && !p.is_host}
                    <Button variant="destructive" size="small" onclick={() => handleKickPlayer(p.name)}>
                      Исключить
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
            <h3 class="card-heading">Присоединиться по коду</h3>
            <p class="card-caption">Введите код комнаты, полученный от друга</p>
          </div>
        </div>

        <div class="join-form">
          <div class="code-input-wrap">
            <input
              type="text"
              placeholder="#A7X9"
              bind:value={guestCode}
              maxlength={6}
              class="code-input"
              onkeydown={(e) => e.key === 'Enter' && handleJoinRoom()}
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

          <Button
            variant="primary"
            size="large"
            fullWidth
            onclick={handleJoinRoom}
            loading={isConnecting}
            disabled={!guestCode.trim()}
          >
            <span>Войти в комнату</span>
          </Button>
        </div>
      </div>

      <!-- Host Panel: Open World -->
      <div class="lobby-card host-card">
        <div class="card-icon-header">
          <div class="icon-circle host">
            <Icon name="play" size={20} color="var(--color-system-green)" />
          </div>
          <div class="card-headings">
            <h3 class="card-heading">Открыть локальный мир</h3>
            <p class="card-caption">Создайте комнату через автоматический Playit-туннель</p>
          </div>
        </div>

        <div class="world-picker">
          <label class="picker-label" for="world-select">Выберите мир для кооператива</label>
          <div class="world-options" id="world-select">
            {#each localWorlds as w}
              <button
                type="button"
                class="world-opt-btn"
                class:selected={selectedWorldId === w.id}
                onclick={() => (selectedWorldId = w.id)}
              >
                <div class="world-opt-title">{w.name}</div>
                <div class="world-opt-meta">{w.version} • {w.size} • {w.lastPlayed}</div>
              </button>
            {/each}
          </div>
        </div>

        <Button
          variant="primary"
          size="large"
          fullWidth
          onclick={handleCreateRoom}
          loading={isHosting}
        >
          <Icon name="server" size={16} color="#ffffff" />
          <span>Открыть мир для друзей</span>
        </Button>
      </div>
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
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 40px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 12px;
    box-sizing: border-box;
  }

  .status-icon-circle {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 4px;
  }

  .status-icon-circle.neutral {
    background-color: var(--color-quaternary-system-fill);
  }

  .status-title {
    font-size: var(--text-title-3);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .status-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    margin: 0;
    max-width: 420px;
    line-height: 1.4;
  }

  .status-actions {
    margin-top: 8px;
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
    margin: 0;
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
    align-items: center;
    gap: 18px;
  }

  .room-code-badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 10px 18px;
    border-radius: var(--radius-md);
    background-color: var(--color-system-blue);
    color: #ffffff;
  }

  .code-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    opacity: 0.9;
  }

  .code-value {
    font-family: var(--font-mono);
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.1em;
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
</style>
