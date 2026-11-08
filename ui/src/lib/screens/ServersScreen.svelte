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
  }

  interface Props {
    selectedVersion?: string;
    playerName?: string;
    onlaunch?: (server: ServerEntry) => void;
  }

  let {
    selectedVersion = '1.21.1',
    playerName = 'Player',
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
  let searchQuery = $state('');
  let isRefreshingAll = $state(false);
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

  const versionOptions = [
    { value: '1.21.1', label: '1.21.1' },
    { value: '1.20.1', label: '1.20.1' },
  ];

  let filteredServers = $derived(
    servers.filter(
      (s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.address.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (s.motd && s.motd.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );

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
    } catch {
      server.ping_ms = null;
      server.pingError = true;
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
    // Attempt reading configuration to match current active nickname and version
    import('@tauri-apps/api/core')
      .then(({ invoke }) => {
        invoke<{ selected_version?: string; player_name?: string }>('get_config')
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
  });
</script>

<div class="servers-screen">
  <!-- Top Screen Toolbar: search, version segmented control, refresh & add actions -->
  <div class="screen-toolbar">
    <div class="search-wrap">
      <SearchField
        placeholder="Поиск серверов по названию или адресу..."
        bind:value={searchQuery}
      />
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
        title="Обновить пинг всех серверов"
      >
        <Icon name="restore" size={13} color="var(--color-secondary-label)" />
        <span>Обновить</span>
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
        title="Добавить новый Minecraft сервер"
      >
        <Icon name="server" size={14} color="#ffffff" />
        <span>Добавить сервер</span>
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
      <OfflineState onretry={loadServers} />
    {:else if filteredServers.length > 0}
      <List header="Серверы Minecraft • {activeVersion}">
        {#each filteredServers as s (s.id)}
          <Row
            title={s.name}
            subtitle="{s.address}{s.version ? ' • ' + s.version : ''}{s.motd ? ' — ' + s.motd : ''}"
            clickable={false}
          >
            {#snippet leading()}
              <div class="server-avatar">
                {#if s.icon}
                  <img src={s.icon} class="server-icon-img" alt={s.name} />
                {:else}
                  <Icon name="server" size={18} color="var(--color-system-blue)" />
                {/if}
              </div>
            {/snippet}

            {#snippet trailing()}
              <div class="server-metrics">
                {#if s.isPinging}
                  <span class="ping-pill loading">
                    <span class="spinner-dot"></span>
                    Пинг...
                  </span>
                {:else if s.ping_ms !== null && s.ping_ms !== undefined}
                  <span
                    class="ping-pill"
                    class:fast={s.ping_ms < 60}
                    class:medium={s.ping_ms >= 60 && s.ping_ms < 150}
                    class:slow={s.ping_ms >= 150}
                    title="Задержка RTT"
                  >
                    <span class="dot"></span>
                    {s.ping_ms} ms
                  </span>

                  {#if s.online !== null && s.online !== undefined}
                    <span class="online-counter" title="Игроков в сети">
                      {s.online}/{s.max_players ?? '?'}
                    </span>
                  {/if}
                {:else}
                  <span class="ping-pill offline" title="Сервер недоступен">
                    Офлайн
                  </span>
                {/if}

                <!-- Primary Action: Connect & Launch -->
                <Button
                  variant="primary"
                  size="small"
                  onclick={() => handlePlayServer(s)}
                  loading={launchingServerId === s.id}
                  title="Запустить игру и подключиться к серверу"
                >
                  <Icon name="play" size={12} color="#ffffff" />
                  <span>Играть</span>
                </Button>

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
                  title="Удалить сервер"
                  aria-label="Удалить сервер {s.name}"
                >
                  <Icon name="close" size={12} color="var(--color-secondary-label)" />
                </button>
              </div>
            {/snippet}
          </Row>
        {/each}
      </List>
    {:else}
      <!-- Authentic empty state without fabricated mock data -->
      <div class="empty-state-card">
        <Icon name="server" size={44} color="var(--color-tertiary-label)" />
        <h3 class="empty-title">Серверы не найдены</h3>
        <p class="empty-desc">
          {searchQuery
            ? 'По запросу «' + searchQuery + '» ничего не найдено.'
            : 'Список серверов пуст. Нажмите «Добавить сервер», чтобы сохранить адрес любимого сервера.'}
        </p>
      </div>
    {/if}
  </div>
</div>

<!-- Add Server HIG Sheet -->
<Sheet
  bind:open={addSheetOpen}
  title="Добавить сервер"
  doneLabel="Сохранить"
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
      <label class="form-label" for="new-srv-name">Название сервера</label>
      <input
        id="new-srv-name"
        type="text"
        bind:value={newServerName}
        placeholder="Например: Aura Survival"
        class="form-input"
        maxlength={32}
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="new-srv-addr">Адрес сервера (домен или IP:порт)</label>
      <input
        id="new-srv-addr"
        type="text"
        bind:value={newServerAddress}
        placeholder="mc.example.com или 192.168.1.100:25565"
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

  .ping-pill.offline,
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
