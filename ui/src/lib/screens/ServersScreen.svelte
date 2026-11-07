<!--
  Wex Launcher — Screen «Серверы»
  Rules:
  - List of servers without fabricated metrics when offline
  - Connect and add server flows
  - Inset grouped List and Row components
  - Clear empty states when no servers are configured
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
  }

  let searchQuery = $state('');
  let addSheetOpen = $state(false);
  let newServerName = $state('');
  let newServerAddress = $state('');

  // Default server list without fake online/ping metrics
  let servers = $state<ServerEntry[]>([
    {
      id: 'aura-main',
      name: 'Aura Official — Survival & PvP',
      address: 'mc.auramc.net',
      version: '1.21.1 / 1.20.1',
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
    {#if filteredServers.length > 0}
      <List header="Серверы Aura">
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
              <div class="server-actions">
                <Button variant="secondary" size="small">
                  Подключиться
                </Button>
              </div>
            {/snippet}
          </Row>
        {/each}
      </List>
    {:else}
      <div class="empty-state-card">
        <Icon name="server" size={36} color="var(--color-tertiary-label)" />
        <h3 class="empty-title">Серверы не найдены</h3>
        <p class="empty-desc">
          {searchQuery ? 'По запросу ничего не найдено.' : 'Список серверов пуст. Нажмите «Добавить сервер», чтобы сохранить адрес мира.'}
        </p>
      </div>
    {/if}
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
        placeholder="Например: Мой сервер"
        class="form-input"
      />
    </div>

    <div class="form-row">
      <label class="form-label" for="srv-addr">Адрес сервера (IP или домен)</label>
      <input
        id="srv-addr"
        type="text"
        bind:value={newServerAddress}
        placeholder="mc.example.com"
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

  .server-actions {
    display: flex;
    align-items: center;
    gap: 8px;
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
    max-width: 360px;
    margin: 0;
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
