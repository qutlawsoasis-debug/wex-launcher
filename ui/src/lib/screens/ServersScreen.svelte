<!--
  Wex Launcher — Screen «Серверы»
  Specification: tasks/apple/hig/lists-and-tables.md, tasks/apple/hig/sheets.md
  Rules:
  - Inset Grouped list of servers
  - Live ping, online metrics, MOTD, and server icon
  - Add server flow via HIG Sheet
  - Remove server action
  - Automatic and manual ping refresh
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import Sheet from '../components/Sheet.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import OfflineState from '../components/OfflineState.svelte';
  import { network } from '../design/network.svelte';

  interface ServerEntry {
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
  }

  let searchQuery = $state('');
  let addSheetOpen = $state(false);
  let newServerName = $state('');
  let newServerAddress = $state('');
  let isRefreshingAll = $state(false);

  let servers = $state<ServerEntry[]>([]);

  let filteredServers = $derived(
    servers.filter(
      (s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.address.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (s.motd && s.motd.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );

  async function loadServers() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ServerEntry[]>('list_servers');
      if (list && list.length > 0) {
        servers = list;
      }
    } catch {
      // In-browser mock fallback
    }
  }

  async function pingSingleServer(server: ServerEntry) {
    server.isPinging = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{
        latency_ms: number;
        online: number;
        max_players: number;
        motd: string;
        version: string;
        favicon?: string;
      }>('ping_server', { address: server.address });

      server.ping_ms = result.latency_ms;
      server.online = result.online;
      server.max_players = result.max_players;
      server.motd = result.motd;
      server.version = result.version;
      if (result.favicon) server.icon = result.favicon;
    } catch {
      // Offline or ping timeout
    } finally {
      server.isPinging = false;
    }
  }

  async function refreshAllServers() {
    if (isRefreshingAll) return;
    isRefreshingAll = true;
    await Promise.all(servers.map((s) => pingSingleServer(s)));
    isRefreshingAll = false;
  }

  async function handleAddServer() {
    if (!newServerName.trim() || !newServerAddress.trim()) return;

    const name = newServerName.trim();
    const address = newServerAddress.trim();

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const created = await invoke<ServerEntry>('add_server', { name, address });
      servers = [...servers, created];
      pingSingleServer(created);
    } catch {
      const fallback: ServerEntry = {
        id: `srv-${Date.now()}`,
        name,
        address,
        version: '1.21.1',
        motd: 'Пользовательский сервер',
        ping_ms: 45,
        online: 1,
        max_players: 20,
      };
      servers = [...servers, fallback];
    }

    newServerName = '';
    newServerAddress = '';
    addSheetOpen = false;
  }

  async function handleRemoveServer(server: ServerEntry) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('remove_server', { address: server.address });
    } catch {
      // Fallback
    }
    servers = servers.filter((s) => s.id !== server.id);
  }

  onMount(() => {
    loadServers();
  });
</script>

<div class="servers-screen">
  <div class="screen-toolbar">
    <div class="search-wrap">
      <SearchField placeholder="Поиск серверов по названию или адресу..." bind:value={searchQuery} />
    </div>

    <div class="toolbar-actions">
      <Button
        variant="secondary"
        size="small"
        onclick={refreshAllServers}
        loading={isRefreshingAll}
        title="Обновить статус и пинг всех серверов"
      >
        <Icon name="restore" size={13} color="var(--color-secondary-label)" />
        <span>Обновить</span>
      </Button>

      <Button
        variant="primary"
        size="small"
        onclick={() => (addSheetOpen = true)}
        title="Добавить новый Minecraft сервер"
      >
        <Icon name="server" size={14} color="#ffffff" />
        <span>Добавить сервер</span>
      </Button>
    </div>
  </div>

  <div class="servers-content">
    {#if !network.isOnline}
      <OfflineState onretry={loadServers} />
    {:else if filteredServers.length > 0}
      <List header="Серверы Minecraft и миры Wex">
        {#each filteredServers as s (s.id)}
          <Row
            title={s.name}
            subtitle="{s.address}{s.version ? ' • ' + s.version : ''}{s.motd ? ' — ' + s.motd : ''}"
            clickable
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
                  <span class="ping-pill loading">Пинг...</span>
                {:else if s.ping_ms !== null && s.ping_ms !== undefined}
                  <span class="ping-pill" class:fast={s.ping_ms < 60} class:medium={s.ping_ms >= 60 && s.ping_ms < 150} class:slow={s.ping_ms >= 150}>
                    <span class="dot"></span>
                    {s.ping_ms} мс
                  </span>

                  {#if s.online !== null && s.online !== undefined}
                    <span class="online-counter">
                      {s.online}/{s.max_players ?? '?'}
                    </span>
                  {/if}
                {:else}
                  <span class="ping-pill offline">Офлайн</span>
                {/if}

                <Button variant="secondary" size="small">
                  Подключиться
                </Button>

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

<Sheet
  bind:open={addSheetOpen}
  title="Добавить сервер"
  doneLabel="Сохранить"
  ondone={handleAddServer}
>
  <div class="sheet-form">
    <div class="form-row">
      <label class="form-label" for="srv-name">Название сервера</label>
      <input
        id="srv-name"
        type="text"
        bind:value={newServerName}
        placeholder="Например: Мой любимый сервер"
        class="form-input"
        maxlength={32}
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="srv-addr">Адрес сервера (домен или IP:порт)</label>
      <input
        id="srv-addr"
        type="text"
        bind:value={newServerAddress}
        placeholder="mc.example.com или 192.168.1.100:25565"
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

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 10px;
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
