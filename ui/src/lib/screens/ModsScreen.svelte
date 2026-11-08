<!--
  Wex Launcher — Screen «Моды»
  Rules:
  - Mod management list with individual Toggle controls
  - Filtering by category (Все / Включённые / Aura)
  - Quick access to mods directory
-->
<script lang="ts">
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';

  interface ModItem {
    id: string;
    name: string;
    version: string;
    description: string;
    enabled: boolean;
    isCore?: boolean;
  }

  let searchQuery = $state('');
  let filterTab = $state('all');

  let mods = $state<ModItem[]>([
    {
      id: 'wex-client',
      name: 'Wex Client',
      version: '2.0.0-release',
      description: 'Ядро интерфейса, OpenGL 3.3 рендер, Dual Kawase размытие, 3D эффекты',
      enabled: true,
      isCore: true,
    },
    {
      id: 'sodium',
      name: 'Sodium',
      version: '0.6.9',
      description: 'Современный движок рендеринга блоков и чанков, высокий FPS',
      enabled: true,
    },
    {
      id: 'iris',
      name: 'Iris Shaders',
      version: '1.8.0',
      description: 'Поддержка современных шейдеров без потерь производительности',
      enabled: true,
    },
    {
      id: 'fabric-api',
      name: 'Fabric API',
      version: '0.102.0',
      description: 'Базовый набор библиотек для экосистемы модов Fabric',
      enabled: true,
      isCore: true,
    },
    {
      id: 'voice-chat',
      name: 'Simple Voice Chat',
      version: '2.5.20',
      description: 'Позиционный голосовой чат для командной игры',
      enabled: false,
    },
    {
      id: 'mod-menu',
      name: 'Mod Menu',
      version: '11.0.2',
      description: 'Список модов и управление их настройками в меню игры',
      enabled: true,
    },
  ]);

  let filteredMods = $derived(
    mods.filter((m) => {
      const matchesSearch =
        m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        m.description.toLowerCase().includes(searchQuery.toLowerCase());
      if (!matchesSearch) return false;
      if (filterTab === 'enabled') return m.enabled;
      if (filterTab === 'core') return m.isCore;
      return true;
    })
  );

  const tabs = [
    { value: 'all', label: 'Все моды' },
    { value: 'enabled', label: 'Включённые' },
    { value: 'core', label: 'Wex Core' },
  ];
</script>

<div class="mods-screen">
  <div class="screen-toolbar">
    <SearchField placeholder="Поиск модов..." bind:value={searchQuery} />
    <SegmentedControl options={tabs} bind:value={filterTab} size="small" />
    <Button variant="secondary" size="small">
      <Icon name="folder" size={14} color="var(--color-secondary-label)" />
      <span>Папка с модами</span>
    </Button>
  </div>

  <div class="mods-content">
    <List header="Установленные моды (версия 1.21.1)">
      {#each filteredMods as m}
        <Row
          title="{m.name} {m.version}"
          subtitle={m.description}
        >
          {#snippet leading()}
            <div class="mod-icon">
              <Icon name="mods" size={18} color={m.enabled ? 'var(--color-system-blue)' : 'var(--color-tertiary-label)'} />
            </div>
          {/snippet}

          {#snippet trailing()}
            <div class="mod-controls">
              {#if m.isCore}
                <span class="core-badge">Обязательный</span>
              {:else}
                <Toggle bind:checked={m.enabled} />
              {/if}
            </div>
          {/snippet}
        </Row>
      {:else}
        <div class="empty-state">
          <span>Моды не найдены</span>
        </div>
      {/each}
    </List>
  </div>
</div>

<style>
  .mods-screen {
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
    gap: 12px;
    flex-wrap: wrap;
  }

  .mods-content {
    flex: 1;
  }

  .mod-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mod-controls {
    display: flex;
    align-items: center;
  }

  .core-badge {
    font-size: var(--text-caption-2);
    font-weight: 500;
    color: var(--color-secondary-label);
    background-color: var(--color-tertiary-system-fill);
    padding: 2px 8px;
    border-radius: var(--radius-pill);
  }

  .empty-state {
    padding: 32px;
    text-align: center;
    color: var(--color-tertiary-label);
    font-size: var(--text-subheadline);
  }
</style>
