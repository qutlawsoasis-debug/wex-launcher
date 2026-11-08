<!--
  Wex Launcher — Screen «Моды»
  Specification: tasks/apple/hig/lists-and-tables.md, tasks/apple/hig/toggles.md, tasks/apple/hig/search-fields.md, tasks/apple/hig/segmented-controls.md
  Rules:
  - Mod management for the selected Minecraft instance (1.21.1 or 1.20.1)
  - Full metadata from fabric.mod.json: name, version, description, status
  - Individual Toggle controls with systemGreen (#30D158) for enabled/disabled state (.jar <-> .jar.disabled)
  - Pack mods are marked with «Пак Wex» badge and protected from deletion
  - User mods can be added via file picker and deleted via destructive action
  - Fast search and categorized tabs (Все моды, Включённые, Пак Wex, Пользовательские)
  - Quick access to open mods directory in system Explorer
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import ErrorState from '../components/ErrorState.svelte';

  export interface ModEntryModel {
    id: string;
    name: string;
    version: string;
    description: string;
    file_name: string;
    enabled: boolean;
    is_pack_mod: boolean;
    icon?: string | null;
  }

  interface Props {
    initialVersion?: string;
  }

  let { initialVersion = '1.21.1' }: Props = $props();

  let selectedVersion = $state('1.21.1');

  $effect(() => {
    if (initialVersion) {
      selectedVersion = initialVersion;
    }
  });
  let searchQuery = $state('');
  let filterTab = $state('all');
  let isAdding = $state(false);
  let loading = $state(true);
  let fileInputRef = $state<HTMLInputElement | null>(null);
  let modError = $state<unknown>(null);
  let loadError = $state<unknown>(null);

  let mods = $state<ModEntryModel[]>([]);

  const versionOptions = [
    { value: '1.21.1', label: '1.21.1' },
    { value: '1.20.1', label: '1.20.1' },
  ];

  const filterTabs = [
    { value: 'all', label: 'Все моды' },
    { value: 'enabled', label: 'Включённые' },
    { value: 'pack', label: 'Пак Wex' },
    { value: 'custom', label: 'Пользовательские' },
  ];

  let filteredMods = $derived(
    mods.filter((m) => {
      const q = searchQuery.toLowerCase().trim();
      const matchesSearch =
        !q ||
        m.name.toLowerCase().includes(q) ||
        m.description.toLowerCase().includes(q) ||
        m.file_name.toLowerCase().includes(q) ||
        m.id.toLowerCase().includes(q);

      if (!matchesSearch) return false;

      if (filterTab === 'enabled') return m.enabled;
      if (filterTab === 'pack') return m.is_pack_mod;
      if (filterTab === 'custom') return !m.is_pack_mod;
      return true;
    })
  );

  async function loadMods(version: string) {
    loading = true;
    loadError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<ModEntryModel[]>('list_mods', { version });
      if (Array.isArray(res)) {
        mods = res;
      } else {
        mods = [];
      }
    } catch (err) {
      // In-browser without Tauri: clean empty list
      mods = [];
      const isTauri =
        typeof window !== 'undefined' &&
        ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
      if (isTauri) {
        loadError = err;
      }
    } finally {
      loading = false;
    }
  }

  async function handleToggleMod(mod: ModEntryModel, newEnabled: boolean) {
    const oldEnabled = mod.enabled;
    mod.enabled = newEnabled;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const updated = await invoke<ModEntryModel>('toggle_mod', {
        version: selectedVersion,
        fileName: mod.file_name,
        enabled: newEnabled,
      });
      if (updated) {
        mod.file_name = updated.file_name;
        mod.enabled = updated.enabled;
      }
    } catch (err) {
      // Revert on IPC failure
      mod.enabled = oldEnabled;
      modError = err;
    }
  }

  async function handleOpenModsDir() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('open_mods_dir', { version: selectedVersion });
    } catch {
      // In-browser fallback
    }
  }

  function handleTriggerAddMod() {
    if (fileInputRef) {
      fileInputRef.click();
    }
  }

  async function handleFileSelected(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    if (!file.name.endsWith('.jar')) {
      modError = {
        code: 'MODS_INVALID_FILE',
        message: 'Неподдерживаемый файл мода',
        technical: `Выбран файл «${file.name}». Лаунчер принимает только архивы модификаций .jar.`,
      };
      return;
    }

    isAdding = true;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      // If running inside Tauri and webkitRelativePath / path property exists:
      const path = (file as unknown as { path?: string }).path;
      if (path) {
        const added = await invoke<ModEntryModel>('add_custom_mod', {
          version: selectedVersion,
          sourcePath: path,
        });
        if (added) {
          mods = [added, ...mods.filter((m) => m.file_name !== added.file_name)];
        }
      } else {
        // Fallback: local mockup representation
        const newMod: ModEntryModel = {
          id: file.name.replace('.jar', '').toLowerCase(),
          name: file.name.replace('.jar', ''),
          version: '1.0.0',
          description: 'Пользовательский файл мода',
          file_name: file.name,
          enabled: true,
          is_pack_mod: false,
        };
        mods = [newMod, ...mods];
      }
    } catch (err) {
      // In-browser mock fallback only if not in Tauri
      const isTauri =
        typeof window !== 'undefined' &&
        ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
      if (isTauri) {
        modError = err;
      } else {
        const newMod: ModEntryModel = {
          id: file.name.replace('.jar', '').toLowerCase(),
          name: file.name.replace('.jar', ''),
          version: '1.0.0',
          description: 'Пользовательский файл мода',
          file_name: file.name,
          enabled: true,
          is_pack_mod: false,
        };
        mods = [newMod, ...mods];
      }
    } finally {
      isAdding = false;
      input.value = '';
    }
  }

  async function handleDeleteMod(mod: ModEntryModel) {
    if (mod.is_pack_mod) {
      modError = {
        code: 'MODS_PROTECTED_PACK_MOD',
        message: 'Мод защищён от удаления',
        technical: `Мод «${mod.name}» (${mod.file_name}) входит в официальный пак Wex и не может быть удалён из клиента.`,
      };
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_custom_mod', {
        version: selectedVersion,
        fileName: mod.file_name,
      });
      mods = mods.filter((m) => m.file_name !== mod.file_name);
    } catch (err) {
      const isTauri =
        typeof window !== 'undefined' &&
        ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
      if (isTauri) {
        modError = err;
      } else {
        mods = mods.filter((m) => m.file_name !== mod.file_name);
      }
    }
  }

  function handleVersionChange(newVer: string) {
    selectedVersion = newVer;
    loadMods(newVer);
  }

  onMount(() => {
    loadMods(selectedVersion);
  });
</script>

<div class="mods-screen">
  <!-- Version switcher & Primary toolbar -->
  <div class="screen-header">
    <div class="header-left">
      <div style="width: 280px;">
        <SegmentedControl
          options={versionOptions}
          value={selectedVersion}
          onchange={handleVersionChange}
          size="small"
        />
      </div>
    </div>

    <div class="header-right">
      <input
        type="file"
        accept=".jar"
        class="hidden-file-input"
        bind:this={fileInputRef}
        onchange={handleFileSelected}
        aria-label="Выбрать файл мода .jar"
      />
      <Button
        variant="primary"
        size="small"
        onclick={handleTriggerAddMod}
        disabled={isAdding}
      >
        <Icon name="plus" size={13} color="#ffffff" />
        <span>Добавить мод...</span>
      </Button>

      <Button
        variant="secondary"
        size="small"
        onclick={handleOpenModsDir}
        title="Открыть папку инстанса с модами в Проводнике"
      >
        <Icon name="folder" size={13} color="var(--color-secondary-label)" />
        <span>Папка с модами</span>
      </Button>
    </div>
  </div>

  <!-- Search & Category Filters -->
  <div class="screen-filters">
    <div class="search-wrap">
      <SearchField placeholder="Поиск модов..." bind:value={searchQuery} />
    </div>
    <div class="tabs-wrap">
      <SegmentedControl options={filterTabs} bind:value={filterTab} size="small" />
    </div>
  </div>

  <!-- Error Banner -->
  {#if modError}
    <div class="mod-error-banner" role="alert">
      <ErrorState
        error={modError}
        domain="mods"
        compact={true}
        onaction={handleOpenModsDir}
        onclose={() => {
          modError = null;
        }}
      />
    </div>
  {/if}

  <!-- Mods List Area -->
  <div class="mods-content">
    <List header="Установленные моды (версия {selectedVersion}) • {filteredMods.length} из {mods.length}">
      {#if loading}
        <div class="empty-state">
          <Icon name="refresh" size={28} color="var(--color-tertiary-label)" />
          <span class="empty-title">Загрузка модов...</span>
        </div>
      {:else if loadError && mods.length === 0}
        <div class="error-state-wrap">
          <ErrorState
            error={loadError}
            domain="mods"
            onaction={() => loadMods(selectedVersion)}
            onsecondaryaction={handleOpenModsDir}
          />
        </div>
      {:else}
        {#each filteredMods as m (m.file_name)}
          <Row
            title="{m.name} {m.version}"
            subtitle={m.description || m.file_name}
          >
            {#snippet leading()}
              <div class="mod-icon">
                <Icon
                  name="mods"
                  size={18}
                  color={m.enabled ? 'var(--color-system-blue)' : 'var(--color-tertiary-label)'}
                />
              </div>
            {/snippet}

            {#snippet trailing()}
              <div class="mod-controls">
                {#if m.is_pack_mod}
                  <span class="pack-badge" title="Мод входит в официальную сборку Wex и защищён от удаления">Пак Wex</span>
                {:else}
                  <button
                    type="button"
                    class="delete-btn"
                    onclick={() => handleDeleteMod(m)}
                    title="Удалить пользовательский мод"
                    aria-label="Удалить {m.name}"
                  >
                    <Icon name="trash" size={14} color="var(--color-system-red)" />
                  </button>
                {/if}

                <Toggle
                  checked={m.enabled}
                  onchange={(checked) => handleToggleMod(m, checked)}
                />
              </div>
            {/snippet}
          </Row>
        {:else}
          <div class="empty-state">
            <Icon name="mods" size={32} color="var(--color-tertiary-label)" />
            <span class="empty-title">
              {searchQuery ? `По запросу «${searchQuery}» ничего не найдено` : 'Моды не найдены'}
            </span>
            <span class="empty-subtitle">
              {searchQuery ? 'Попробуйте изменить поисковый запрос или категорию' : 'В папке инстанса пока нет модов. Нажмите «Добавить мод...», чтобы установить первый мод'}
            </span>
          </div>
        {/each}
      {/if}
    </List>
  </div>
</div>

<style>
  .mod-error-banner {
    margin: 0;
  }

  .error-state-wrap {
    padding: 24px 16px;
  }

  .mods-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .screen-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .screen-filters {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .search-wrap {
    flex: 1;
    min-width: 260px;
    max-width: 420px;
  }

  .tabs-wrap {
    flex-shrink: 0;
  }

  .mods-content {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 860px;
  }

  .mod-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background-color: var(--color-tertiary-system-fill);
  }

  .mod-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .pack-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    background-color: var(--color-quaternary-system-fill);
    color: var(--color-secondary-label);
    letter-spacing: -0.2px;
    user-select: none;
  }

  .delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    cursor: pointer;
    transition: background-color var(--motion-duration-short) ease;
    padding: 0;
  }

  .delete-btn:hover {
    background-color: rgba(255, 69, 58, 0.12);
  }

  .delete-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .hidden-file-input {
    display: none;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    gap: 8px;
    text-align: center;
  }

  .empty-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
  }

  .empty-subtitle {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
  }
</style>
