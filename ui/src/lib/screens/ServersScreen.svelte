<!--
  Wex Launcher — Screen «Серверы»
  Specification:
  - Apple HIG Lists and Tables: https://developer.apple.com/design/human-interface-guidelines/lists-and-tables
  - Apple HIG Sheets: https://developer.apple.com/design/human-interface-guidelines/sheets
  - Apple HIG Buttons: https://developer.apple.com/design/human-interface-guidelines/buttons
  - Apple HIG Search Fields: https://developer.apple.com/design/human-interface-guidelines/search-fields
  - Apple HIG Color: https://developer.apple.com/design/human-interface-guidelines/color
  Rules:
  - Inset Grouped list of real servers loaded from servers.dat of the chosen instance
  - Parallel asynchronous SLP pinging with latency, online/max players, MOTD and favicon
  - Add and Edit server workflows via HIG Sheet modal cards
  - Direct «Играть» launch action with --quickPlayMultiplayer server auto-connect
  - System Apple colors only (systemBlue, systemGreen, systemOrange, systemRed)
  - Zero fabricated data: authentic empty state when no servers exist
-->
<script module lang="ts">
  export interface ServerEntry {
    id: string;
    name: string;
    address: string;
    icon?: string | null;
    ping_ms?: number | null;
    online?: number | null;
    max_players?: number | null;
    motd?: string | null;
    version?: string | null;
    isPinging?: boolean;
    pingError?: boolean;
    errorMessage?: string | null;
    errorReason?: 'offline' | 'invalid_address' | 'no_internet' | null;
    isFriendServer?: boolean;
    friendNick?: string;
    worldName?: string;
    lobbyCode?: string;
  }

  export interface FriendItem {
    id: string;
    nick: string;
    status: 'online' | 'playing' | 'in_lobby' | 'offline';
    last_seen?: number | null;
    game_version?: string | null;
    world_name?: string | null;
    lobby_code?: string | null;
  }

  /**
   * Categorizes server ping error into clear reasons required by specification:
   * - offline: «Сервер выключен или недоступен»
   * - invalid_address: «Неверный адрес сервера»
   * - no_internet: «Отсутствует подключение к интернету»
   */
  export function diagnosePingError(
    error: any,
    isOnline = true
  ): {
    reason: 'offline' | 'invalid_address' | 'no_internet';
    message: string;
  } {
    if (!isOnline) {
      return {
        reason: 'no_internet',
        message: 'Отсутствует подключение к интернету',
      };
    }

    const errStr = (typeof error === 'string' ? error : error?.message || '').toLowerCase();

    // 1. Network / Internet unreachable
    if (
      errStr.includes('no_internet') ||
      errStr.includes('network unreachable') ||
      errStr.includes('enetunreach') ||
      errStr.includes('enetdown') ||
      errStr.includes('нет интернета')
    ) {
      return {
        reason: 'no_internet',
        message: 'Отсутствует подключение к интернету',
      };
    }

    // 2. Invalid address / DNS resolution failure
    if (
      errStr.includes('invalid_address') ||
      errStr.includes('no such host') ||
      errStr.includes('failed to lookup') ||
      errStr.includes('dns') ||
      errStr.includes('not found') ||
      errStr.includes('addrnotavailable') ||
      errStr.includes('cannot assign') ||
      errStr.includes('неверный адрес')
    ) {
      return {
        reason: 'invalid_address',
        message: 'Неверный адрес сервера',
      };
    }

    // 3. Server turned off or connection timed out
    return {
      reason: 'offline',
      message: 'Сервер выключен или недоступен',
    };
  }
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import Sheet from '../components/Sheet.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import OfflineState from '../components/OfflineState.svelte';
  import { network } from '../design/network.svelte';
  import { t, i18n } from '../i18n';

  interface Props {
    selectedVersion?: string;
    playerName?: string;
    mockFriendServers?: ServerEntry[];
    onlaunch?: (server: ServerEntry) => void;
  }

  let {
    selectedVersion = '1.21.1',
    playerName = 'Player',
    mockFriendServers,
    onlaunch,
  }: Props = $props();

  let activeVersion = $state('1.21.1');
  let effectivePlayerName = $state('Player');

  $effect(() => {
    if (selectedVersion) activeVersion = selectedVersion;
  });

  $effect(() => {
    if (playerName) effectivePlayerName = playerName;
  });

  $effect(() => {
    if (mockFriendServers) {
      friendServers = mockFriendServers;
    }
  });
  let searchQuery = $state('');
  let isRefreshingAll = $state(false);
  let isImporting = $state(false);
  let launchingServerId = $state<string | null>(null);
  let launchFeedback = $state<string | null>(null);

  // Add Server Sheet state
  let addSheetOpen = $state(false);
  let newServerName = $state('');
  let newServerAddress = $state('');
  let addError = $state<string | null>(null);

  // Edit Server Sheet state
  let editSheetOpen = $state(false);
  let editingServer = $state<ServerEntry | null>(null);
  let editServerName = $state('');
  let editServerAddress = $state('');
  let editError = $state<string | null>(null);

  let servers = $state<ServerEntry[]>([]);
  let friendServers = $state<ServerEntry[]>([]);

  const versionOptions = [
    { value: '1.21.1', label: '1.21.1' },
    { value: '1.20.1', label: '1.20.1' },
  ];

  // Friend servers are pinned at the very top of the list
  let filteredServers = $derived.by(() => {
    const q = searchQuery.toLowerCase();
    const matchedFriends = friendServers.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.address.toLowerCase().includes(q) ||
        (s.motd && s.motd.toLowerCase().includes(q)) ||
        (s.friendNick && s.friendNick.toLowerCase().includes(q))
    );
    const matchedNormal = servers.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.address.toLowerCase().includes(q) ||
        (s.motd && s.motd.toLowerCase().includes(q))
    );
    return [...matchedFriends, ...matchedNormal];
  });

  async function loadFriendServers() {
    if (mockFriendServers) {
      friendServers = mockFriendServers;
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backendFriends = await invoke<
        Array<{
          id: string;
          friend_nick: string;
          world_name: string;
          address: string;
          lobby_code?: string;
          game_version?: string;
        }>
      >('get_friend_hosted_servers');

      if (backendFriends && backendFriends.length > 0) {
        friendServers = backendFriends.map((f) => ({
          id: f.id,
          name: `Мир: ${f.friend_nick}`,
          address: f.address,
          motd: `Хост: ${f.friend_nick} • ${f.world_name}`,
          version: f.game_version || activeVersion,
          isFriendServer: true,
          friendNick: f.friend_nick,
          lobbyCode: f.lobby_code,
          worldName: f.world_name,
        }));
        return;
      }

      // Fallback: query social_sync
      const syncResult = await invoke<{
        friends?: FriendItem[];
      }>('social_sync', {});

      if (syncResult && syncResult.friends) {
        const fromSync: ServerEntry[] = [];
        for (const f of syncResult.friends) {
          if ((f.status === 'playing' || f.status === 'in_lobby') && (f.world_name || f.lobby_code)) {
            const addr = f.lobby_code
              ? `#${f.lobby_code}`
              : f.world_name?.includes(':')
                ? f.world_name
                : '127.0.0.1:25565';
            fromSync.push({
              id: `friend-srv-${f.id}`,
              name: `Мир: ${f.nick}`,
              address: addr,
              motd: `Хост: ${f.nick} • ${f.world_name || 'Кооператив'}`,
              version: f.game_version || activeVersion,
              isFriendServer: true,
              friendNick: f.nick,
              lobbyCode: f.lobby_code || undefined,
              worldName: f.world_name || undefined,
            });
          }
        }
        friendServers = fromSync;
      }
    } catch {
      // In-browser mock fallback
    }
  }

  async function loadServers(ver: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ServerEntry[]>('list_servers', { version: ver });
      servers = list || [];
      if (servers.length > 0) {
        refreshAllServers();
      }
    } catch {
      // In-browser mock / test fallback: keep empty array for authentic empty state
      servers = [];
    }
  }

  async function pingSingleServer(server: ServerEntry) {
    server.isPinging = true;
    server.pingError = false;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{
        latency_ms: number;
        online: number;
        max_players: number;
        motd: string;
        version: string;
        favicon?: string;
      }>('ping_server', { address: server.address, timeout_ms: 2500 });

      server.ping_ms = result.latency_ms;
      server.online = result.online;
      server.max_players = result.max_players;
      server.motd = result.motd;
      server.version = result.version;
      if (result.favicon) {
        server.icon = result.favicon;
      }
      server.errorReason = null;
      server.errorMessage = null;
    } catch (e: any) {
      // No fabricated numbers on error: online and max_players stay null
      server.ping_ms = null;
      server.online = null;
      server.max_players = null;
      server.pingError = true;
      const diag = diagnosePingError(e, network.isOnline);
      server.errorReason = diag.reason;
      server.errorMessage = diag.message;
    } finally {
      server.isPinging = false;
    }
  }

  async function refreshAllServers() {
    if (isRefreshingAll || servers.length === 0) return;
    isRefreshingAll = true;
    await Promise.allSettled(servers.map((s) => pingSingleServer(s)));
    isRefreshingAll = false;
  }

  async function handleImportBothVersions() {
    isImporting = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const imported = await invoke<ServerEntry[]>('import_servers_both_versions');
      if (imported && imported.length > 0) {
        servers = imported;
        refreshAllServers();
      }
    } catch {
      // Browser preview / mock fallback
    } finally {
      isImporting = false;
    }
  }

  async function handleAddServer() {
    addError = null;
    const name = newServerName.trim();
    const address = newServerAddress.trim();

    if (!name || !address) {
      addError = 'Укажите название и адрес сервера.';
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const created = await invoke<ServerEntry>('add_server', {
        version: activeVersion,
        name,
        address,
      });

      servers = [...servers, created];
      pingSingleServer(created);
      addSheetOpen = false;
      newServerName = '';
      newServerAddress = '';
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || 'Не удалось сохранить сервер.';
      addError = msg;
    }
  }

  function openEditSheet(server: ServerEntry) {
    editingServer = server;
    editServerName = server.name;
    editServerAddress = server.address;
    editError = null;
    editSheetOpen = true;
  }

  async function handleEditServer() {
    if (!editingServer) return;
    editError = null;

    const name = editServerName.trim();
    const address = editServerAddress.trim();

    if (!name || !address) {
      editError = 'Укажите название и адрес сервера.';
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const updated = await invoke<ServerEntry>('edit_server', {
        version: activeVersion,
        old_address: editingServer.address,
        name,
        new_address: address,
      });

      servers = servers.map((s) => (s.id === editingServer!.id ? { ...s, ...updated } : s));
      const target = servers.find((s) => s.id === editingServer!.id);
      if (target) {
        pingSingleServer(target);
      }

      editSheetOpen = false;
      editingServer = null;
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : e?.message || 'Не удалось обновить сервер.';
      editError = msg;
    }
  }

  async function handleRemoveServer(server: ServerEntry) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('remove_server', {
        version: activeVersion,
        address: server.address,
      });
    } catch {
      // Ignored
    }
    servers = servers.filter((s) => s.id !== server.id);
  }

  async function handlePlayServer(server: ServerEntry) {
    if (onlaunch) {
      onlaunch(server);
      return;
    }

    if (server.lobbyCode) {
      launchingServerId = server.id;
      launchFeedback = `Подключение к лобби #${server.lobbyCode}...`;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('join_lobby_room', {
          code: server.lobbyCode,
          playerName: effectivePlayerName,
        });
        launchFeedback = `Присоединились к лобби #${server.lobbyCode}`;
      } catch (e: any) {
        launchFeedback = typeof e === 'string' ? e : e?.message || 'Ошибка входа в лобби';
      } finally {
        setTimeout(() => {
          launchingServerId = null;
          launchFeedback = null;
        }, 4000);
      }
      return;
    }

    launchingServerId = server.id;
    launchFeedback = `Запуск Minecraft ${activeVersion} с подключением к ${server.name}...`;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('launch_game', {
        version: activeVersion,
        nickname: effectivePlayerName,
        server: server.address,
      });
      launchFeedback = `Игра запущена. Подключение к ${server.address}`;
    } catch (e: any) {
      launchFeedback = typeof e === 'string' ? e : e?.message || 'Ошибка запуска игры';
    } finally {
      setTimeout(() => {
        launchingServerId = null;
        launchFeedback = null;
      }, 4000);
    }
  }

  function onVersionChange(newVer: string) {
    activeVersion = newVer;
    loadServers(newVer);
  }

  onMount(() => {
    // 1. Attempt reading configuration to match current active nickname and version
    import('@tauri-apps/api/core')
      .then(({ invoke }) => {
        invoke<{ selected_version?: string; player_name?: string }>('get_launcher_settings')
          .then((cfg) => {
            if (cfg?.selected_version) activeVersion = cfg.selected_version;
            if (cfg?.player_name) effectivePlayerName = cfg.player_name;
            loadServers(activeVersion);
          })
          .catch(() => {
            loadServers(activeVersion);
          });
      })
      .catch(() => {
        loadServers(activeVersion);
      });

    // 2. Load friend hosted servers
    loadFriendServers();
  });
</script>

<div class="servers-screen">
  <!-- Top Screen Toolbar: search, version segmented control, refresh & add actions -->
  <div class="screen-toolbar">
    <div class="search-wrap">
      <SearchField placeholder={t('servers.search_placeholder')} bind:value={searchQuery} />
    </div>

    <div class="toolbar-controls">
      <div class="version-picker" title="Выбор инстанса servers.dat">
        <SegmentedControl
          options={versionOptions}
          value={activeVersion}
          size="small"
          onchange={onVersionChange}
        />
      </div>

      <Button
        variant="secondary"
        size="small"
        onclick={refreshAllServers}
        loading={isRefreshingAll}
        title={t('servers.refresh_all')}
      >
        <Icon name="restore" size={13} color="var(--color-secondary-label)" />
        <span>{t('common.refresh')}</span>
      </Button>

      <Button
        variant="secondary"
        size="small"
        onclick={handleImportBothVersions}
        loading={isImporting}
        title="Импортировать серверы из servers.dat 1.20.1 и 1.21.1 без дублей"
      >
        <Icon name="folder" size={13} color="var(--color-secondary-label)" />
        <span>{i18n.locale === 'en' ? 'Import' : 'Импорт'}</span>
      </Button>

      <Button
        variant="primary"
        size="small"
        onclick={() => {
          newServerName = '';
          newServerAddress = '';
          addError = null;
          addSheetOpen = true;
        }}
        title={t('servers.add_server')}
      >
        <Icon name="server" size={14} color="#ffffff" />
        <span>{t('servers.add_server')}</span>
      </Button>
    </div>
  </div>

  {#if launchFeedback}
    <div class="launch-toast" role="status">
      <Icon name="play" size={14} color="var(--color-system-blue)" />
      <span>{launchFeedback}</span>
    </div>
  {/if}

  <!-- Main Servers List View (Apple HIG Inset Grouped) -->
  <div class="servers-content">
    {#if !network.isOnline}
      <OfflineState onretry={() => loadServers(activeVersion)} />
    {:else if filteredServers.length > 0}
      <List header="Серверы Minecraft • {activeVersion}">
        {#each filteredServers as s (s.id)}
          <Row
            title={s.name}
            subtitle={s.pingError && s.errorMessage
              ? `${s.address} • ${s.errorMessage}`
              : `${s.address}${s.version ? ' • ' + s.version : ''}${s.motd ? ' — ' + s.motd : ''}`}
            clickable={false}
          >
            {#snippet leading()}
              <div class="server-avatar" class:friend-avatar={s.isFriendServer}>
                {#if s.isFriendServer}
                  <Icon name="friends" size={18} color="var(--color-system-green)" />
                {:else if s.icon}
                  <img src={s.icon} class="server-icon-img" alt={s.name} />
                {:else}
                  <Icon name="server" size={18} color="var(--color-system-blue)" />
                {/if}
              </div>
            {/snippet}

            {#if s.isFriendServer}
              <div class="pinned-row-tag">
                <span class="pinned-badge" title="Сервер друга закреплён вверху">
                  <Icon name="friends" size={11} color="var(--color-system-orange)" />
                  <span>{i18n.locale === 'en' ? 'Friend Server' : 'Сервер друга'}</span>
                </span>
              </div>
            {/if}

            {#snippet trailing()}
              <div class="server-metrics">
                {#if s.isPinging}
                  <span class="ping-pill loading">
                    <span class="spinner-dot"></span>
                    {i18n.locale === 'en' ? 'Checking…' : 'Пинг...'}
                  </span>
                {:else if s.pingError}
                  {#if s.errorReason === 'offline'}
                    <span class="ping-pill offline" title={s.errorMessage || 'Сервер выключен или недоступен'}>
                      {i18n.locale === 'en' ? 'Offline' : 'Выключен'}
                    </span>
                  {:else if s.errorReason === 'invalid_address'}
                    <span class="ping-pill error" title={s.errorMessage || 'Неверный адрес сервера'}>
                      {i18n.locale === 'en' ? 'Bad address' : 'Неверный адрес'}
                    </span>
                  {:else if s.errorReason === 'no_internet'}
                    <span class="ping-pill warning" title={s.errorMessage || 'Отсутствует подключение к интернету'}>
                      {i18n.locale === 'en' ? 'No internet' : 'Нет сети'}
                    </span>
                  {:else}
                    <span class="ping-pill offline" title="Сервер недоступен">
                      {i18n.locale === 'en' ? 'Offline' : 'Офлайн'}
                    </span>
                  {/if}

                  <!-- Retry Button for Failed Ping -->
                  <Button
                    variant="ghost"
                    size="small"
                    onclick={() => pingSingleServer(s)}
                    loading={s.isPinging}
                    title={i18n.locale === 'en' ? 'Retry pinging server' : 'Повторить проверку сервера'}
                  >
                    <Icon name="restore" size={11} color="var(--color-system-blue)" />
                    <span>{i18n.locale === 'en' ? 'Retry' : 'Повторить'}</span>
                  </Button>
                {:else if s.ping_ms !== null && s.ping_ms !== undefined}
                  <span
                    class="ping-pill"
                    class:fast={s.ping_ms < 60}
                    class:medium={s.ping_ms >= 60 && s.ping_ms < 150}
                    class:slow={s.ping_ms >= 150}
                    title="Задержка RTT"
                  >
                    <span class="dot"></span>
                    {s.ping_ms} {i18n.locale === 'en' ? 'ms' : 'мс'}
                  </span>

                  {#if s.online !== null && s.online !== undefined}
                    <span class="online-counter" title="Игроков в сети">
                      {s.online}/{s.max_players ?? '?'}
                    </span>
                  {/if}
                {:else}
                  <span class="ping-pill offline" title="Сервер недоступен">
                    {i18n.locale === 'en' ? 'Offline' : 'Офлайн'}
                  </span>
                {/if}

                <!-- Primary Action: Connect & Launch -->
                <Button
                  variant="primary"
                  size="small"
                  onclick={() => handlePlayServer(s)}
                  loading={launchingServerId === s.id}
                  title={i18n.locale === 'en' ? 'Launch game and connect to server' : 'Запустить игру и подключиться к серверу'}
                >
                  <Icon name="play" size={12} color="#ffffff" />
                  <span>{i18n.locale === 'en' ? 'Play' : 'Играть'}</span>
                </Button>

                {#if !s.isFriendServer}
                  <!-- Edit Server -->
                  <Button
                    variant="ghost"
                    size="small"
                    onclick={() => openEditSheet(s)}
                    title="Изменить название или адрес сервера"
                  >
                    <span>Изменить</span>
                  </Button>

                  <!-- Remove Server -->
                  <button
                    type="button"
                    class="remove-btn"
                    onclick={() => handleRemoveServer(s)}
                    title={t('servers.delete_server')}
                    aria-label="{t('servers.delete_server')} {s.name}"
                  >
                    <Icon name="close" size={12} color="var(--color-secondary-label)" />
                  </button>
                {/if}
              </div>
            {/snippet}
          </Row>
        {/each}
      </List>
    {:else}
      <!-- Authentic empty state without fabricated mock data -->
      <div class="empty-state-card">
        <Icon name="server" size={44} color="var(--color-tertiary-label)" />
        <h2 class="empty-title">{t('servers.empty_title')}</h2>
        <p class="empty-desc">
          {searchQuery
            ? t('servers.empty_search', { query: searchQuery })
            : t('servers.empty_default')}
        </p>
      </div>
    {/if}
  </div>
</div>

<!-- Add Server HIG Sheet -->
<Sheet
  bind:open={addSheetOpen}
  title={t('servers.sheet_title')}
  doneLabel={t('servers.sheet_save')}
  ondone={handleAddServer}
>
  <div class="sheet-form">
    {#if addError}
      <div class="sheet-error" role="alert">
        <Icon name="warning" size={14} color="var(--color-system-red)" />
        <span>{addError}</span>
      </div>
    {/if}

    <div class="form-row">
      <label class="form-label" for="new-srv-name">{t('servers.sheet_name_label')}</label>
      <input
        id="new-srv-name"
        type="text"
        bind:value={newServerName}
        placeholder={t('servers.sheet_name_placeholder')}
        class="form-input"
        maxlength={32}
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="new-srv-addr">{t('servers.sheet_address_label')}</label>
      <input
        id="new-srv-addr"
        type="text"
        bind:value={newServerAddress}
        placeholder={t('servers.sheet_address_placeholder')}
        class="form-input"
      />
    </div>
  </div>
</Sheet>

<!-- Edit Server HIG Sheet -->
<Sheet
  bind:open={editSheetOpen}
  title="Изменить сервер"
  doneLabel="Сохранить"
  ondone={handleEditServer}
>
  <div class="sheet-form">
    {#if editError}
      <div class="sheet-error" role="alert">
        <Icon name="warning" size={14} color="var(--color-system-red)" />
        <span>{editError}</span>
      </div>
    {/if}

    <div class="form-row">
      <label class="form-label" for="edit-srv-name">Название сервера</label>
      <input
        id="edit-srv-name"
        type="text"
        bind:value={editServerName}
        class="form-input"
        maxlength={32}
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="edit-srv-addr">Адрес сервера (домен или IP:порт)</label>
      <input
        id="edit-srv-addr"
        type="text"
        bind:value={editServerAddress}
        class="form-input"
      />
    </div>
  </div>
</Sheet>

<style>
  .servers-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .screen-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .search-wrap {
    flex: 1;
    min-width: 260px;
  }

  .toolbar-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .version-picker {
    display: flex;
    align-items: center;
  }

  .launch-toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background-color: var(--color-tertiary-system-fill);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-subheadline);
    color: var(--color-label);
  }

  .servers-content {
    flex: 1;
  }

  .server-avatar {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-sm);
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
  }

  .server-avatar.friend-avatar {
    background-color: rgba(48, 209, 88, 0.18);
  }

  .pinned-row-tag {
    margin-top: 4px;
  }

  .pinned-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    background-color: rgba(255, 159, 10, 0.15);
    color: var(--color-system-orange);
    border-radius: var(--radius-sm);
    font-size: var(--text-caption2);
    font-weight: 600;
  }

  .server-icon-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .server-metrics {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ping-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--text-footnote);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    padding: 2px 8px;
    background-color: var(--color-quaternary-system-fill);
    border-radius: var(--radius-pill);
    color: var(--color-secondary-label);
  }

  .ping-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: currentColor;
  }

  .ping-pill.fast {
    color: var(--color-system-green);
    background-color: rgba(48, 209, 88, 0.12);
  }

  .ping-pill.medium {
    color: var(--color-system-orange);
    background-color: rgba(255, 159, 10, 0.12);
  }

  .ping-pill.slow {
    color: var(--color-system-red);
    background-color: rgba(255, 69, 58, 0.12);
  }

  .ping-pill.offline {
    color: var(--color-system-red);
    background-color: rgba(255, 69, 58, 0.12);
  }

  .ping-pill.error {
    color: var(--color-system-red);
    background-color: rgba(255, 69, 58, 0.16);
  }

  .ping-pill.warning {
    color: var(--color-system-orange);
    background-color: rgba(255, 159, 10, 0.16);
  }

  .ping-pill.loading {
    color: var(--color-tertiary-label);
  }

  .spinner-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-tertiary-label);
    animation: pulse 1s infinite alternate;
  }

  @keyframes pulse {
    0% { opacity: 0.3; }
    100% { opacity: 1; }
  }

  .online-counter {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }

  .remove-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background-color var(--motion-duration-short) ease;
  }

  .remove-btn:hover {
    background-color: rgba(255, 69, 58, 0.15);
  }

  .remove-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .remove-btn:hover :global(svg) {
    stroke: var(--color-system-red);
  }

  .empty-state-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 24px;
    gap: 12px;
    text-align: center;
    background-color: var(--card-bg);
    border: 1px dashed var(--glass-border);
    border-radius: var(--radius-lg);
  }

  .empty-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .empty-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    max-width: 380px;
    line-height: 1.4;
    margin: 0;
  }

  .sheet-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px 0;
  }

  .sheet-error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background-color: rgba(255, 69, 58, 0.12);
    border-radius: var(--radius-sm);
    color: var(--color-system-red);
    font-size: var(--text-footnote);
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-label {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
  }

  .form-input {
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--glass-border);
    background-color: var(--color-tertiary-system-fill);
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    outline: none;
    transition: border-color var(--motion-duration-short) ease;
  }

  .form-input:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
    background-color: var(--card-bg);
  }
</style>
