<!--
  Wex Launcher — Screen «Серверы»
  Rules:
  - List of servers with live ping and online player metrics
  - Connect and add server flows
  - Inset grouped List and Row components
-->
<script lang="ts">
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import Sheet from '../components/Sheet.svelte';
  import Icon from '../components/icons/Icon.svelte';

  interface ServerEntry {
    id: string;
    name: string;
    address: string;
    version: string;
    online: number;
    maxPlayers: number;
    ping: number;
  }

  let searchQuery = $state('');
  let addSheetOpen = $state(false);
  let newServerName = $state('');
  let newServerAddress = $state('');

  let servers = $state<ServerEntry[]>([
    {
      id: 'aura-main',
      name: 'Aura Official — Survival & PvP',
      address: 'mc.auramc.net',
      version: '1.21.1',
      online: 142,
      maxPlayers: 250,
      ping: 28,
    },
    {
      id: 'aura-fabric',
      name: 'Aura Modded — Fabric Pack',
      address: 'pack.auramc.net',
      version: '1.20.1',
      online: 76,
      maxPlayers: 150,
      ping: 35,
    },
    {
      id: 'aura-test',
      name: 'Dev & Creative Sandbox',
      address: 'dev.auramc.net',
      version: '1.21.1',
      online: 18,
      maxPlayers: 50,
      ping: 42,
    },
  ]);

  let filteredServers = $derived(
    servers.filter(
      (s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.address.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  function addServer() {
    if (!newServerName.trim() || !newServerAddress.trim()) return;
    servers.push({
      id: `custom-${Date.now()}`,
      name: newServerName.trim(),
      address: newServerAddress.trim(),
      version: '1.21.1',
      online: 1,
      maxPlayers: 20,
      ping: 45,
    });
    newServerName = '';
    newServerAddress = '';
    addSheetOpen = false;
  }
</script>

<div class="servers-screen">
  <div class="screen-toolbar">
    <SearchField placeholder="Поиск серверов..." bind:value={searchQuery} />
    <Button variant="primary" size="small" onclick={() => (addSheetOpen = true)}>
      <Icon name="server" size={14} color="#ffffff" />
      <span>Добавить сервер</span>
    </Button>
  </div>

  <div class="servers-content">
    <List header="Избранные серверы Aura">
      {#each filteredServers as s}
        <Row
          title={s.name}
          subtitle="{s.address} • {s.version}"
          clickable
        >
          {#snippet leading()}
            <div class="server-avatar">
              <Icon name="server" size={18} color="var(--color-system-blue)" />
            </div>
          {/snippet}

          {#snippet trailing()}
            <div class="server-metrics">
              <span class="online-pill">
                <span class="dot"></span>
                {s.online}/{s.maxPlayers}
              </span>
              <span class="ping-pill" class:fast={s.ping < 50}>
                {s.ping} ms
              </span>
              <Button variant="secondary" size="small">
                Подключиться
              </Button>
            </div>
          {/snippet}
        </Row>
      {:else}
        <div class="empty-state">
          <span>Серверы не найдены</span>
        </div>
      {/each}
    </List>
  </div>
</div>

<Sheet
  bind:open={addSheetOpen}
  title="Добавить сервер"
  doneLabel="Сохранить"
  ondone={addServer}
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
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="srv-addr">Адрес сервера (IP или домен)</label>
      <input
        id="srv-addr"
        type="text"
        bind:value={newServerAddress}
        placeholder="play.example.com"
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
  }

  .screen-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .servers-content {
    flex: 1;
  }

  .server-avatar {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .server-metrics {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .online-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-system-green);
  }

  .ping-pill {
    font-size: var(--text-footnote);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
    color: var(--color-secondary-label);
    padding: 2px 8px;
    background-color: var(--color-quaternary-system-fill);
    border-radius: var(--radius-pill);
  }

  .ping-pill.fast {
    color: var(--color-system-green);
  }

  .empty-state {
    padding: 32px;
    text-align: center;
    color: var(--color-tertiary-label);
    font-size: var(--text-subheadline);
  }

  .sheet-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
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
    height: 36px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--glass-border);
    background-color: var(--color-tertiary-system-fill);
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    outline: none;
  }

  .form-input:focus {
    border-color: var(--color-system-blue);
    background-color: var(--card-bg);
  }
</style>
