<!--
  Wex Launcher — Screen «Мастерская» (SPEC 1.7)
  Apple HIG specifications:
  - https://developer.apple.com/design/human-interface-guidelines/segmented-controls
  - https://developer.apple.com/design/human-interface-guidelines/lists-and-tables
  - https://developer.apple.com/design/human-interface-guidelines/collections
  - https://developer.apple.com/design/human-interface-guidelines/buttons
  - https://developer.apple.com/design/human-interface-guidelines/toggles
  - https://developer.apple.com/design/human-interface-guidelines/sheets
  - https://developer.apple.com/design/human-interface-guidelines/content-view-empty-states
  Rules:
  - Segmented control switches between «Миры», «Ресурс-паки», «Шейдеры» and «Скриншоты»
  - Inset Grouped list for single-player worlds with game mode, last played date, and size
  - Resource packs list with on/off toggles, pack.mcmeta description, and pack.png icons
  - Shaders list with active shader selection (Iris iris.properties) and disable option
  - Add pack via ZIP file and open folders in system file manager
  - Red destructive button for deleting custom packs with modal confirmation (Aura-Pack files protected)
  - Secondary button for creating compressed zip backups
  - Modal sheet for backup restore confirmation
  - Uniform grid collection for screenshots with system file manager reveal
  - Authentic empty states without mock data or slogans
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../components/Button.svelte';
  import Sheet from '../components/Sheet.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import { formatError } from '../design/errors';
  import type { WorldItemDto, WorldBackupItemDto, ScreenshotItemDto } from '../ipc/generated';

  export interface ResourcePackItem {
    name: string;
    file_name: string;
    file_path: string;
    size_bytes: number | bigint;
    description: string | null;
    pack_format: number | null;
    icon_data_url: string | null;
    is_enabled: boolean;
    is_from_pack: boolean;
  }

  export interface ShaderPackItem {
    name: string;
    file_name: string;
    file_path: string;
    size_bytes: number | bigint;
    description: string | null;
    icon_data_url: string | null;
    is_active: boolean;
    is_from_pack: boolean;
  }

  interface Props {
    selectedVersion?: string;
    initialTab?: string;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) => void;
  }

  let { selectedVersion = '1.21.1', initialTab = 'worlds', ontoast }: Props = $props();

  // Active section tab: 'worlds' | 'resourcepacks' | 'shaders' | 'screenshots'
  let activeTab = $state('worlds');
  let currentVersion = $state('1.21.1');

  $effect(() => {
    if (selectedVersion) {
      currentVersion = selectedVersion;
    }
  });

  $effect(() => {
    if (initialTab && initialTab !== 'worlds') {
      activeTab = initialTab;
      refreshCurrentTab();
    }
  });

  // Data state
  let worlds = $state<WorldItemDto[]>([]);
  let backups = $state<WorldBackupItemDto[]>([]);
  let screenshots = $state<ScreenshotItemDto[]>([]);
  let previewScreenshot = $state<ScreenshotItemDto | null>(null);
  let isScreenshotPreviewOpen = $state(false);
  let resourcePacks = $state<ResourcePackItem[]>([]);
  let shaderPacks = $state<ShaderPackItem[]>([]);
  let isLoading = $state(false);
  let sheetError = $state<unknown>(null);

  // Hidden file input for adding ZIP packs
  let fileInputEl: HTMLInputElement | null = null;
  let pendingAddType = $state<'resourcepack' | 'shaderpack'>('resourcepack');

  // Confirmation modals state: World Delete
  let deleteSheetOpen = $state(false);
  let worldToDelete = $state<WorldItemDto | null>(null);
  let isDeleting = $state(false);

  // Confirmation modals state: World Backups
  let backupsSheetOpen = $state(false);
  let selectedWorldForBackups = $state<WorldItemDto | null>(null);

  let restoreSheetOpen = $state(false);
  let backupToRestore = $state<WorldBackupItemDto | null>(null);
  let isRestoring = $state(false);

  let isBackingUp = $state<Record<string, boolean>>({});

  // Confirmation modal state: Pack Delete (Resource pack or Shader)
  let deletePackSheetOpen = $state(false);
  let packToDelete = $state<{
    name: string;
    file_name: string;
    type: 'resourcepack' | 'shaderpack';
    is_from_pack: boolean;
  } | null>(null);
  let isDeletingPack = $state(false);

  // Add Pack Modal Sheet (fallback / manual path input)
  let addPackSheetOpen = $state(false);
  let manualPackPath = $state('');
  let isAddingPack = $state(false);

  function notify(type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) {
    if (ontoast) {
      ontoast(type, title, description);
    }
  }

  function formatBytes(bytes: number | bigint): string {
    const num = Number(bytes);
    if (num <= 0) return '0 КБ';
    if (num < 1024) return `${num} Б`;
    if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} КБ`;
    if (num < 1024 * 1024 * 1024) return `${(num / (1024 * 1024)).toFixed(1)} МБ`;
    return `${(num / (1024 * 1024 * 1024)).toFixed(2)} ГБ`;
  }

  function formatTimestamp(ts: number | bigint): string {
    const num = Number(ts);
    if (!num) return 'Не запускался';
    const date = new Date(num);
    return date.toLocaleDateString('ru-RU', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function formatIsoDate(iso: string | null): string {
    if (!iso) return 'Неизвестно';
    try {
      const date = new Date(iso);
      return date.toLocaleDateString('ru-RU', {
        day: '2-digit',
        month: '2-digit',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return iso;
    }
  }

  function getGameModeLabel(mode: string): string {
    switch (mode) {
      case 'Survival':
        return 'Выживание';
      case 'Creative':
        return 'Творческий';
      case 'Adventure':
        return 'Приключение';
      case 'Spectator':
        return 'Наблюдатель';
      default:
        return 'Неизвестно';
    }
  }

  function getSafeAssetUrl(filePath: string | null): string | null {
    if (!filePath) return null;
    try {
      return (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
        ? `https://asset.localhost/${encodeURIComponent(filePath.replace(/\\/g, '/'))}`
        : filePath;
    } catch {
      return filePath;
    }
  }

  // --- Loader functions ---

  async function loadWorlds() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<WorldItemDto[]>('workshop_list_worlds', {
        version: currentVersion,
      });
      worlds = list || [];
    } catch {
      worlds = [];
    } finally {
      isLoading = false;
    }
  }

  async function loadResourcePacks() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ResourcePackItem[]>('workshop_list_resource_packs', {
        version: currentVersion,
      });
      resourcePacks = list || [];
    } catch {
      resourcePacks = [];
    } finally {
      isLoading = false;
    }
  }

  async function loadShaderPacks() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ShaderPackItem[]>('workshop_list_shader_packs', {
        version: currentVersion,
      });
      shaderPacks = list || [];
    } catch {
      shaderPacks = [];
    } finally {
      isLoading = false;
    }
  }

  async function loadScreenshots() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ScreenshotItemDto[]>('workshop_list_screenshots', {
        version: currentVersion,
      });
      screenshots = list || [];
    } catch {
      screenshots = [];
    } finally {
      isLoading = false;
    }
  }

  async function handleCopyScreenshot(shot: ScreenshotItemDto, e?: Event) {
    if (e) e.stopPropagation();
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_copy_screenshot_image', { path: shot.file_path });
      if (ontoast) {
        ontoast('success', 'Скриншот скопирован', 'Изображение скопировано в буфер обмена');
      }
    } catch {
      try {
        const assetUrl = getSafeAssetUrl(shot.file_path);
        if (assetUrl) {
          const res = await fetch(assetUrl);
          const blob = await res.blob();
          if (typeof navigator !== 'undefined' && navigator.clipboard && typeof window !== 'undefined' && (window as any).ClipboardItem) {
            await navigator.clipboard.write([
              new (window as any).ClipboardItem({ [blob.type || 'image/png']: blob }),
            ]);
            if (ontoast) {
              ontoast('success', 'Скриншот скопирован', 'Изображение скопировано в буфер обмена');
            }
            return;
          }
        }
        throw new Error('ClipboardItem unavailable');
      } catch {
        if (ontoast) {
          ontoast('error', 'Ошибка копирования', 'Не удалось скопировать изображение в буфер обмена');
        }
      }
    }
  }

  function refreshCurrentTab() {
    if (activeTab === 'worlds') loadWorlds();
    else if (activeTab === 'resourcepacks') loadResourcePacks();
    else if (activeTab === 'shaders') loadShaderPacks();
    else loadScreenshots();
  }

  // --- Worlds actions ---

  async function handleCreateBackup(world: WorldItemDto) {
    isBackingUp = { ...isBackingUp, [world.folder_name]: true };
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const backup = await invoke<WorldBackupItemDto>('workshop_create_world_backup', {
        worldFolder: world.folder_path,
        version: currentVersion,
      });
      notify('success', 'Резервная копия создана', backup.file_name);
    } catch (err) {
      const formatted = formatError(err, 'workshop');
      notify('error', formatted.title, formatted.description);
    } finally {
      isBackingUp = { ...isBackingUp, [world.folder_name]: false };
    }
  }

  async function openBackupsSheet(world: WorldItemDto) {
    selectedWorldForBackups = world;
    backups = [];
    sheetError = null;
    backupsSheetOpen = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<WorldBackupItemDto[]>('workshop_list_world_backups', {
        version: currentVersion,
        worldFolderName: world.folder_name,
        displayName: world.display_name,
      });
      backups = list || [];
    } catch (err) {
      sheetError = err;
      const formatted = formatError(err, 'workshop');
      notify('error', formatted.title, formatted.description);
    }
  }

  function promptRestore(backup: WorldBackupItemDto) {
    backupToRestore = backup;
    sheetError = null;
    restoreSheetOpen = true;
  }

  async function handleConfirmRestore() {
    if (!backupToRestore || !selectedWorldForBackups) return;
    isRestoring = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_restore_world_backup', {
        backupPath: backupToRestore.file_path,
        targetWorldFolder: selectedWorldForBackups.folder_path,
        version: currentVersion,
      });
      notify('success', 'Мир успешно восстановлен', selectedWorldForBackups.display_name);
      restoreSheetOpen = false;
      backupsSheetOpen = false;
      await loadWorlds();
    } catch (err) {
      sheetError = err;
      const formatted = formatError(err, 'workshop');
      notify('error', formatted.title, formatted.description);
    } finally {
      isRestoring = false;
    }
  }

  function promptDelete(world: WorldItemDto) {
    worldToDelete = world;
    sheetError = null;
    deleteSheetOpen = true;
  }

  async function handleConfirmDelete() {
    if (!worldToDelete) return;
    isDeleting = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_delete_world', {
        worldFolder: worldToDelete.folder_path,
      });
      notify('success', 'Мир удалён', worldToDelete.display_name);
      deleteSheetOpen = false;
      worldToDelete = null;
      await loadWorlds();
    } catch (err) {
      sheetError = err;
      const formatted = formatError(err, 'workshop');
      notify('error', formatted.title, formatted.description);
    } finally {
      isDeleting = false;
    }
  }

  // --- Resource packs actions ---

  async function handleToggleResourcePack(pack: ResourcePackItem, enabled: boolean) {
    pack.is_enabled = enabled;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_toggle_resource_pack', {
        fileName: pack.file_name,
        enabled,
        version: currentVersion,
      });
      notify('info', enabled ? 'Ресурс-пак включён' : 'Ресурс-пак отключён', pack.name);
      await loadResourcePacks();
    } catch (err) {
      notify('error', 'Не удалось изменить состояние ресурс-пака', String(err));
      pack.is_enabled = !enabled;
    }
  }

  // --- Shaders actions ---

  async function handleSelectShader(shader: ShaderPackItem | null) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_set_active_shader_pack', {
        shaderFileName: shader ? shader.file_name : null,
        version: currentVersion,
      });
      notify(
        'success',
        shader ? 'Шейдер-пак выбран' : 'Шейдеры отключены',
        shader ? shader.name : 'Используется стандартный рендеринг'
      );
      await loadShaderPacks();
    } catch (err) {
      notify('error', 'Ошибка переключения шейдера', String(err));
    }
  }

  // --- Common Pack Actions (Add & Delete & Open Folder) ---

  function openFolderForType(type: 'resourcepack' | 'shaderpack') {
    import('@tauri-apps/api/core').then(({ invoke }) => {
      invoke('workshop_open_packs_folder', {
        packType: type,
        version: currentVersion,
      }).catch((err) => {
        notify('warning', 'Не удалось открыть папку', String(err));
      });
    });
  }

  function promptDeletePack(pack: { name: string; file_name: string; is_from_pack: boolean }, type: 'resourcepack' | 'shaderpack') {
    if (pack.is_from_pack) {
      notify('warning', 'Защита файлов Aura-Pack', 'Файлы, входящие в официальную сборку, нельзя удалить.');
      return;
    }
    packToDelete = {
      name: pack.name,
      file_name: pack.file_name,
      type,
      is_from_pack: pack.is_from_pack,
    };
    deletePackSheetOpen = true;
  }

  async function handleConfirmDeletePack() {
    if (!packToDelete) return;
    isDeletingPack = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_delete_pack_file', {
        fileName: packToDelete.file_name,
        packType: packToDelete.type,
        version: currentVersion,
      });
      notify('success', 'Файл удалён', packToDelete.name);
      deletePackSheetOpen = false;
      packToDelete = null;
      if (activeTab === 'resourcepacks') await loadResourcePacks();
      else if (activeTab === 'shaders') await loadShaderPacks();
    } catch (err) {
      notify('error', 'Ошибка удаления файла', String(err));
    } finally {
      isDeletingPack = false;
    }
  }

  function triggerAddPack(type: 'resourcepack' | 'shaderpack') {
    pendingAddType = type;
    manualPackPath = '';
    addPackSheetOpen = true;
  }

  async function handleConfirmAddPack() {
    if (!manualPackPath.trim()) return;
    isAddingPack = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const added = await invoke<string>('workshop_add_pack_file', {
        sourcePath: manualPackPath.trim(),
        packType: pendingAddType,
        version: currentVersion,
      });
      notify('success', 'Пак успешно добавлен', added);
      addPackSheetOpen = false;
      manualPackPath = '';
      if (pendingAddType === 'resourcepack') await loadResourcePacks();
      else await loadShaderPacks();
    } catch (err) {
      notify('error', 'Ошибка добавления пака', String(err));
    } finally {
      isAddingPack = false;
    }
  }

  async function handleOpenPath(path: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('workshop_open_path', { path });
    } catch (err) {
      const formatted = formatError(err, 'workshop');
      notify('error', formatted.title, formatted.description);
    }
  }

  function handleVersionChange(ver: string) {
    currentVersion = ver;
    refreshCurrentTab();
  }

  onMount(() => {
    if (activeTab === 'worlds') {
      loadWorlds();
    } else {
      refreshCurrentTab();
    }
  });
</script>

<div class="workshop-screen" data-testid="workshop-screen">
  <!-- Header Bar with Segmented Control -->
  <header class="workshop-header">
    <div class="header-left">
      <div class="section-segmented">
        <SegmentedControl
          options={[
            { value: 'worlds', label: 'Миры' },
            { value: 'resourcepacks', label: 'Ресурс-паки' },
            { value: 'shaders', label: 'Шейдеры' },
            { value: 'screenshots', label: 'Скриншоты' },
          ]}
          bind:value={activeTab}
          onchange={(val: string) => {
            activeTab = val;
            refreshCurrentTab();
          }}
        />
      </div>
    </div>

    <div class="header-right">
      <!-- Version Picker -->
      <div class="version-segmented">
        <SegmentedControl
          options={[
            { value: '1.21.1', label: '1.21.1' },
            { value: '1.20.1', label: '1.20.1' },
          ]}
          bind:value={currentVersion}
          onchange={(val: string) => handleVersionChange(val)}
        />
      </div>

      {#if activeTab === 'resourcepacks'}
        <Button
          variant="secondary"
          size="small"
          onclick={() => triggerAddPack('resourcepack')}
          title="Добавить ZIP файл ресурс-пака"
        >
          <Icon name="folder" size={14} color="var(--color-label)" />
          <span>Добавить ZIP…</span>
        </Button>
        <Button
          variant="ghost"
          size="small"
          onclick={() => openFolderForType('resourcepack')}
          title="Открыть папку ресурс-паков в проводнике"
        >
          <span>Папка паков</span>
        </Button>
      {:else if activeTab === 'shaders'}
        <Button
          variant="secondary"
          size="small"
          onclick={() => triggerAddPack('shaderpack')}
          title="Добавить ZIP файл шейдеров"
        >
          <Icon name="folder" size={14} color="var(--color-label)" />
          <span>Добавить ZIP…</span>
        </Button>
        <Button
          variant="ghost"
          size="small"
          onclick={() => openFolderForType('shaderpack')}
          title="Открыть папку шейдеров в проводнике"
        >
          <span>Папка шейдеров</span>
        </Button>
      {/if}

      <Button
        variant="ghost"
        size="small"
        onclick={refreshCurrentTab}
        loading={isLoading}
        title="Обновить список"
      >
        <Icon name="refresh" size={15} color="var(--color-secondary-label)" />
        <span>Обновить</span>
      </Button>
    </div>
  </header>

  <!-- Main Content Area -->
  <div class="workshop-content">
    {#if activeTab === 'worlds'}
      <!-- TAB 1: WORLDS -->
      {#if worlds.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-worlds">
          <div class="empty-icon-wrap">
            <Icon name="folder" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h2 class="empty-title">Сохранённых миров нет</h2>
          <p class="empty-description">
            Создайте одиночный мир в игре Minecraft {currentVersion}, чтобы управлять им здесь.
          </p>
        </div>
      {:else}
        <div class="worlds-list">
          {#each worlds as world (world.folder_name)}
            <div class="world-card" data-testid="world-row-{world.folder_name}">
              <div class="world-icon-wrap">
                {#if world.icon_path}
                  <img
                    src={getSafeAssetUrl(world.icon_path) || ''}
                    alt={world.display_name}
                    class="world-icon-img"
                    onerror={(e) => {
                      (e.currentTarget as HTMLElement).style.display = 'none';
                    }}
                  />
                {:else}
                  <div class="world-icon-placeholder">
                    <Icon name="folder" size={24} color="var(--color-secondary-label)" />
                  </div>
                {/if}
              </div>

              <div class="world-info">
                <div class="world-title-row">
                  <span class="world-title">{world.display_name}</span>
                  <span class="world-tag mode-{world.game_mode.toLowerCase()}">
                    {getGameModeLabel(world.game_mode)}
                  </span>
                </div>

                <div class="world-meta-row">
                  <span class="meta-item">
                    <span class="meta-label">Последняя игра:</span>
                    {formatTimestamp(world.last_played)}
                  </span>
                  <span class="meta-dot">•</span>
                  <span class="meta-item">
                    <span class="meta-label">Размер:</span>
                    {formatBytes(world.size_bytes)}
                  </span>
                  <span class="meta-dot">•</span>
                  <span class="meta-item folder-name" title={world.folder_path}>
                    {world.folder_name}
                  </span>
                </div>
              </div>

              <div class="world-actions">
                <Button
                  variant="secondary"
                  size="small"
                  onclick={() => handleCreateBackup(world)}
                  loading={isBackingUp[world.folder_name]}
                  title="Создать zip-архив мира"
                >
                  <Icon name="restore" size={14} color="var(--color-label)" />
                  <span>Резервная копия</span>
                </Button>

                <Button
                  variant="secondary"
                  size="small"
                  onclick={() => openBackupsSheet(world)}
                  title="Восстановить из имеющейся копии"
                >
                  <span>Восстановить…</span>
                </Button>

                <Button
                  variant="destructive"
                  size="small"
                  onclick={() => promptDelete(world)}
                  title="Удалить мир с диска"
                >
                  <Icon name="close" size={13} color="#ffffff" />
                  <span>Удалить</span>
                </Button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    {:else if activeTab === 'resourcepacks'}
      <!-- TAB 2: RESOURCE PACKS -->
      {#if resourcePacks.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-resourcepacks">
          <div class="empty-icon-wrap">
            <Icon name="folder" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h2 class="empty-title">Ресурс-паков пока нет</h2>
          <p class="empty-description">
            Поместите ZIP-архив с ресурс-паком в папку resourcepacks инстанса Minecraft {currentVersion} или нажмите кнопку «Добавить ZIP…».
          </p>
          <div style="margin-top: 16px;">
            <Button variant="secondary" size="small" onclick={() => openFolderForType('resourcepack')}>
              <span>Открыть папку ресурс-паков</span>
            </Button>
          </div>
        </div>
      {:else}
        <div class="packs-list" data-testid="resourcepacks-list">
          {#each resourcePacks as pack (pack.file_name)}
            <div
              class="pack-card"
              class:enabled={pack.is_enabled}
              data-testid="pack-row-{pack.file_name}"
            >
              <div class="pack-icon-wrap">
                {#if pack.icon_data_url}
                  <img
                    src={pack.icon_data_url}
                    alt={pack.name}
                    class="pack-icon-img"
                    onerror={(e) => {
                      (e.currentTarget as HTMLElement).style.display = 'none';
                    }}
                  />
                {:else}
                  <div class="pack-icon-placeholder">
                    <Icon name="folder" size={24} color="var(--color-secondary-label)" />
                  </div>
                {/if}
              </div>

              <div class="pack-info">
                <div class="pack-title-row">
                  <span class="pack-title">{pack.name}</span>
                  {#if pack.is_from_pack}
                    <span class="pack-badge from-pack">В сборке</span>
                  {/if}
                  {#if pack.is_enabled}
                    <span class="pack-badge active-badge">Включён</span>
                  {/if}
                </div>

                {#if pack.description}
                  <p class="pack-description" title={pack.description}>{pack.description}</p>
                {/if}

                <div class="pack-meta-row">
                  <span class="meta-item">
                    <span class="meta-label">Размер:</span>
                    {formatBytes(pack.size_bytes)}
                  </span>
                  <span class="meta-dot">•</span>
                  <span class="meta-item folder-name" title={pack.file_path}>
                    {pack.file_name}
                  </span>
                </div>
              </div>

              <div class="pack-actions">
                <div class="toggle-wrap" title={pack.is_enabled ? 'Отключить ресурс-пак' : 'Включить ресурс-пак'}>
                  <Toggle
                    checked={pack.is_enabled}
                    ariaLabel={`Включить ресурс-пак ${pack.name}`}
                    onchange={(checked: boolean) => handleToggleResourcePack(pack, checked)}
                  />
                </div>

                {#if !pack.is_from_pack}
                  <Button
                    variant="destructive"
                    size="small"
                    onclick={() => promptDeletePack(pack, 'resourcepack')}
                    title="Удалить ресурс-пак"
                  >
                    <Icon name="close" size={13} color="#ffffff" />
                    <span>Удалить</span>
                  </Button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}

    {:else if activeTab === 'shaders'}
      <!-- TAB 3: SHADERS -->
      {#if shaderPacks.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-shaders">
          <div class="empty-icon-wrap">
            <Icon name="sun" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h2 class="empty-title">Шейдер-паков пока нет</h2>
          <p class="empty-description">
            Поместите ZIP-архив с шейдерами в папку shaderpacks инстанса Minecraft {currentVersion} или нажмите кнопку «Добавить ZIP…».
          </p>
          <div style="margin-top: 16px;">
            <Button variant="secondary" size="small" onclick={() => openFolderForType('shaderpack')}>
              <span>Открыть папку шейдеров</span>
            </Button>
          </div>
        </div>
      {:else}
        <div class="shaders-container" data-testid="shaders-list">
          <!-- Standalone option: Shaders OFF -->
          <div
            class="shader-card off-card"
            class:active={!shaderPacks.some((s) => s.is_active)}
          >
            <div class="shader-icon-wrap">
              <Icon name="sun" size={22} color="var(--color-secondary-label)" />
            </div>

            <div class="shader-info">
              <div class="shader-title-row">
                <span class="shader-title">Без шейдеров (OFF)</span>
                {#if !shaderPacks.some((s) => s.is_active)}
                  <span class="pack-badge active-badge">Активен</span>
                {/if}
              </div>
              <p class="shader-description">Стандартный быстрый рендеринг Minecraft без внешних шейдеров.</p>
            </div>

            <div class="shader-actions">
              {#if !shaderPacks.some((s) => s.is_active)}
                <span class="active-indicator">Выбрано</span>
              {:else}
                <Button
                  variant="secondary"
                  size="small"
                  onclick={() => handleSelectShader(null)}
                >
                  Выбрать
                </Button>
              {/if}
            </div>
          </div>

          <!-- List of Shader Packs -->
          <div class="shaders-list">
            {#each shaderPacks as shader (shader.file_name)}
              <div
                class="shader-card"
                class:active={shader.is_active}
                data-testid="shader-row-{shader.file_name}"
              >
                <div class="shader-icon-wrap">
                  {#if shader.icon_data_url}
                    <img
                      src={shader.icon_data_url}
                      alt={shader.name}
                      class="shader-icon-img"
                      onerror={(e) => {
                        (e.currentTarget as HTMLElement).style.display = 'none';
                      }}
                    />
                  {:else}
                    <Icon name="sun" size={22} color="var(--color-secondary-label)" />
                  {/if}
                </div>

                <div class="shader-info">
                  <div class="shader-title-row">
                    <span class="shader-title">{shader.name}</span>
                    {#if shader.is_from_pack}
                      <span class="pack-badge from-pack">В сборке</span>
                    {/if}
                    {#if shader.is_active}
                      <span class="pack-badge active-badge">Активен</span>
                    {/if}
                  </div>

                  {#if shader.description}
                    <p class="shader-description" title={shader.description}>{shader.description}</p>
                  {/if}

                  <div class="shader-meta-row">
                    <span class="meta-item">
                      <span class="meta-label">Размер:</span>
                      {formatBytes(shader.size_bytes)}
                    </span>
                    <span class="meta-dot">•</span>
                    <span class="meta-item folder-name" title={shader.file_path}>
                      {shader.file_name}
                    </span>
                  </div>
                </div>

                <div class="shader-actions">
                  {#if shader.is_active}
                    <span class="active-indicator">Активен</span>
                  {:else}
                    <Button
                      variant="secondary"
                      size="small"
                      onclick={() => handleSelectShader(shader)}
                    >
                      Выбрать
                    </Button>
                  {/if}

                  {#if !shader.is_from_pack}
                    <Button
                      variant="destructive"
                      size="small"
                      onclick={() => promptDeletePack(shader, 'shaderpack')}
                      title="Удалить шейдер-пак"
                    >
                      <Icon name="close" size={13} color="#ffffff" />
                      <span>Удалить</span>
                    </Button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    {:else}
      <!-- TAB 4: SCREENSHOTS -->
      {#if screenshots.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-screenshots">
          <div class="empty-icon-wrap">
            <Icon name="sun" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h2 class="empty-title">Снимков экрана пока нет</h2>
          <p class="empty-description">
            Нажмите клавишу F2 во время игры, чтобы сделать снимок экрана в Minecraft {currentVersion}.
          </p>
        </div>
      {:else}
        <div class="screenshots-grid" data-testid="screenshots-grid">
          {#each screenshots as shot (shot.file_name)}
            <div class="screenshot-card" data-testid="screenshot-item">
              <div
                class="screenshot-thumb-btn"
                role="button"
                tabindex="0"
                onclick={() => {
                  previewScreenshot = shot;
                  isScreenshotPreviewOpen = true;
                }}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    previewScreenshot = shot;
                    isScreenshotPreviewOpen = true;
                  }
                }}
                title="Нажмите для полноразмерного просмотра"
              >
                <div class="screenshot-thumb-wrap">
                  <img
                    src={getSafeAssetUrl(shot.file_path) || ''}
                    alt={shot.file_name}
                    class="screenshot-thumb"
                    loading="lazy"
                    onerror={(e) => {
                      const el = e.currentTarget as HTMLElement;
                      el.style.display = 'none';
                      if (el.parentElement) {
                        el.parentElement.classList.add('broken');
                      }
                    }}
                  />
                  <div class="thumb-hover-overlay">
                    <div class="overlay-actions">
                      <button
                        type="button"
                        class="overlay-btn"
                        onclick={(e) => handleCopyScreenshot(shot, e)}
                        title="Скопировать изображение в буфер обмена"
                        aria-label="Скопировать изображение"
                      >
                        <Icon name="copy" size={15} color="#ffffff" />
                        <span>Копия</span>
                      </button>
                      <button
                        type="button"
                        class="overlay-btn"
                        onclick={(e) => {
                          e.stopPropagation();
                          handleOpenPath(shot.file_path);
                        }}
                        title="Открыть в проводнике"
                        aria-label="Открыть в проводнике"
                      >
                        <Icon name="folder" size={15} color="#ffffff" />
                        <span>Файл</span>
                      </button>
                    </div>
                  </div>
                </div>

                <div class="screenshot-caption">
                  <span class="caption-name" title={shot.file_name}>{shot.file_name}</span>
                  <span class="caption-meta">
                    {formatBytes(shot.size_bytes)} • {formatIsoDate(shot.created_at)}
                  </span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

<!-- Modal Sheet: Screenshot Preview -->
<Sheet
  bind:open={isScreenshotPreviewOpen}
  title={previewScreenshot?.file_name || 'Просмотр скриншота'}
  cancelLabel="Закрыть"
  showDone={false}
  maxWidth="720px"
>
  {#if previewScreenshot}
    <div class="screenshot-preview-dialog" data-testid="screenshot-preview-dialog">
      <div class="preview-img-container">
        <img
          src={getSafeAssetUrl(previewScreenshot.file_path) || ''}
          alt={previewScreenshot.file_name}
          class="preview-img-full"
        />
      </div>

      <div class="preview-meta-row">
        <span>{formatBytes(previewScreenshot.size_bytes)}</span>
        <span>•</span>
        <span>{formatIsoDate(previewScreenshot.created_at)}</span>
      </div>

      <div class="preview-actions-bar">
        <Button
          variant="secondary"
          size="regular"
          onclick={() => handleCopyScreenshot(previewScreenshot!)}
        >
          <Icon name="copy" size={16} color="var(--color-primary-label)" />
          <span>Скопировать в буфер</span>
        </Button>
        <Button
          variant="secondary"
          size="regular"
          onclick={() => handleOpenPath(previewScreenshot!.file_path)}
        >
          <Icon name="folder" size={16} color="var(--color-primary-label)" />
          <span>Открыть в проводнике</span>
        </Button>
      </div>
    </div>
  {/if}
</Sheet>

<!-- Modal Sheet: Add Pack File (Manual Path / Import) -->
<Sheet
  bind:open={addPackSheetOpen}
  title={pendingAddType === 'resourcepack' ? 'Добавить ресурс-пак' : 'Добавить шейдер-пак'}
  cancelLabel="Отмена"
  doneLabel="Добавить"
  showDone={false}
  maxWidth="500px"
>
  <div class="modal-dialog-body" data-testid="add-pack-modal">
    <p class="dialog-text">
      Укажите абсолютный путь к ZIP-архиву
      {pendingAddType === 'resourcepack' ? 'ресурс-пака' : 'шейдер-пака'} на вашем компьютере:
    </p>

    <div class="modal-input-wrap">
      <input
        type="text"
        class="apple-text-field"
        placeholder="C:\Downloads\Pack.zip"
        bind:value={manualPackPath}
        onkeydown={(e) => {
          if (e.key === 'Enter') handleConfirmAddPack();
        }}
        aria-label="Путь к ZIP-архиву"
      />
    </div>

    <p class="dialog-subtext">
      Файл будет скопирован в папку
      {pendingAddType === 'resourcepack' ? 'resourcepacks' : 'shaderpacks'} инстанса {currentVersion}.
    </p>

    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="regular"
        onclick={() => (addPackSheetOpen = false)}
        disabled={isAddingPack}
      >
        Отмена
      </Button>

      <Button
        variant="primary"
        size="regular"
        onclick={handleConfirmAddPack}
        loading={isAddingPack}
        disabled={!manualPackPath.trim()}
      >
        Добавить файл
      </Button>
    </div>
  </div>
</Sheet>

<!-- Modal Sheet: Delete Pack Confirmation -->
<Sheet
  bind:open={deletePackSheetOpen}
  title="Удаление пака"
  cancelLabel="Отмена"
  doneLabel="Удалить"
  showDone={false}
  maxWidth="460px"
>
  <div class="modal-dialog-body" data-testid="delete-pack-modal">
    <div class="dialog-icon-destructive">
      <Icon name="warning" size={32} color="var(--color-system-red)" />
    </div>

    <p class="dialog-text">
      Вы действительно хотите удалить
      {packToDelete?.type === 'resourcepack' ? 'ресурс-пак' : 'шейдер-пак'}
      <strong>«{packToDelete?.name}»</strong>?
    </p>

    <p class="dialog-subtext">
      Файл <code>{packToDelete?.file_name}</code> будет безвозвратно удалён из папки инстанса.
    </p>

    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="regular"
        onclick={() => (deletePackSheetOpen = false)}
        disabled={isDeletingPack}
      >
        Отмена
      </Button>

      <Button
        variant="destructive"
        size="regular"
        onclick={handleConfirmDeletePack}
        loading={isDeletingPack}
      >
        Удалить
      </Button>
    </div>
  </div>
</Sheet>

<!-- Modal Sheet: Delete World Confirmation -->
<Sheet
  bind:open={deleteSheetOpen}
  title="Удаление мира"
  cancelLabel="Отмена"
  doneLabel="Удалить"
  showDone={false}
  maxWidth="460px"
>
  <div class="modal-dialog-body" data-testid="delete-world-modal">
    <div class="dialog-icon-destructive">
      <Icon name="warning" size={32} color="var(--color-system-red)" />
    </div>

    <p class="dialog-text">
      Вы действительно хотите удалить мир
      <strong>«{worldToDelete?.display_name || worldToDelete?.folder_name}»</strong>?
    </p>

    <p class="dialog-subtext">
      Это действие необратимо. Папка сохранения будет безвозвратно удалена с диска.
    </p>

    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="regular"
        onclick={() => (deleteSheetOpen = false)}
        disabled={isDeleting}
      >
        Отмена
      </Button>

      <Button
        variant="destructive"
        size="regular"
        onclick={handleConfirmDelete}
        loading={isDeleting}
      >
        Удалить мир
      </Button>
    </div>
  </div>
</Sheet>

<!-- Modal Sheet: Backups List & Restore -->
<Sheet
  bind:open={backupsSheetOpen}
  title="Резервные копии"
  cancelLabel="Закрыть"
  showDone={false}
  maxWidth="540px"
>
  <div class="modal-dialog-body" data-testid="backups-modal">
    <p class="dialog-text">
      Копии для мира <strong>«{selectedWorldForBackups?.display_name}»</strong>:
    </p>

    {#if sheetError}
      <div style="margin-bottom: 12px;">
        <ErrorState
          error={sheetError}
          domain="workshop"
          compact={true}
          onclose={() => (sheetError = null)}
        />
      </div>
    {/if}

    {#if backups.length === 0}
      <div class="empty-modal-state">
        <Icon name="info" size={24} color="var(--color-secondary-label)" />
        <p>Резервных копий для этого мира пока не создано.</p>
      </div>
    {:else}
      <div class="modal-list">
        {#each backups as b (b.file_name)}
          <div class="backup-row">
            <div class="backup-info">
              <span class="backup-filename" title={b.file_name}>{b.file_name}</span>
              <span class="backup-meta">
                {formatBytes(b.size_bytes)} • {formatIsoDate(b.created_at)}
              </span>
            </div>

            <Button
              variant="secondary"
              size="small"
              onclick={() => promptRestore(b)}
            >
              Восстановить
            </Button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</Sheet>

<!-- Modal Sheet: Confirm Restore -->
<Sheet
  bind:open={restoreSheetOpen}
  title="Восстановление из копии"
  cancelLabel="Отмена"
  showDone={false}
  maxWidth="460px"
>
  <div class="modal-dialog-body" data-testid="restore-confirm-modal">
    <div class="dialog-icon-warning">
      <Icon name="warning" size={32} color="var(--color-system-orange)" />
    </div>

    <p class="dialog-text">
      Восстановить мир <strong>«{selectedWorldForBackups?.display_name}»</strong> из архива
      <strong>«{backupToRestore?.file_name}»</strong>?
    </p>

    <p class="dialog-subtext">
      Текущее состояние мира будет перезаписано данными из архива.
    </p>

    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="regular"
        onclick={() => (restoreSheetOpen = false)}
        disabled={isRestoring}
      >
        Отмена
      </Button>

      <Button
        variant="primary"
        size="regular"
        onclick={handleConfirmRestore}
        loading={isRestoring}
      >
        Подтвердить восстановление
      </Button>
    </div>
  </div>
</Sheet>

<style>
  .workshop-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
    background-color: transparent;
    color: var(--color-label);
  }

  .workshop-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    border-bottom: 1px solid var(--glass-border);
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    background-color: var(--glass-surface-overlay);
    gap: 16px;
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .section-segmented {
    width: 440px;
  }

  .version-segmented {
    width: 170px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .workshop-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px 32px;
  }

  /* Empty State */
  .empty-state-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 60px 24px;
    margin: 40px auto;
    max-width: 460px;
    border-radius: var(--radius-lg);
    background-color: var(--card-bg);
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
  }

  .empty-icon-wrap {
    margin-bottom: 16px;
    opacity: 0.8;
  }

  .empty-title {
    font-size: var(--text-headline);
    font-weight: 600;
    margin: 0 0 8px;
    color: var(--color-label);
  }

  .empty-description {
    font-size: var(--text-subheadline);
    line-height: 1.5;
    color: var(--color-secondary-label);
    font-weight: 500;
    margin: 0;
  }

  /* Worlds List */
  .worlds-list,
  .packs-list,
  .shaders-container {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 920px;
  }

  .world-card,
  .pack-card,
  .shader-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background-color: var(--glass-surface-translucent);
    border: 1px solid var(--glass-border);
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .world-card:hover,
  .pack-card:hover,
  .shader-card:hover {
    background-color: var(--color-system-fill);
    border-color: rgba(255, 255, 255, 0.18);
  }

  .pack-card.enabled,
  .shader-card.active {
    background-color: rgba(10, 132, 255, 0.08);
    border-color: rgba(10, 132, 255, 0.28);
  }

  .world-icon-wrap,
  .pack-icon-wrap,
  .shader-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    overflow: hidden;
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .world-icon-img,
  .pack-icon-img,
  .shader-icon-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }

  .world-icon-placeholder,
  .pack-icon-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .world-info,
  .pack-info,
  .shader-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .world-title-row,
  .pack-title-row,
  .shader-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .world-title,
  .pack-title,
  .shader-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pack-description,
  .shader-description {
    font-size: 12px;
    line-height: 1.4;
    color: var(--color-secondary-label);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .world-tag {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: var(--radius-sm);
    background-color: var(--color-secondary-system-fill);
    color: var(--color-secondary-label);
  }

  .world-tag.mode-survival {
    color: #4cd964;
    background-color: rgba(76, 217, 100, 0.15);
  }

  .world-tag.mode-creative {
    color: var(--color-system-blue);
    background-color: rgba(10, 132, 255, 0.15);
  }

  .world-tag.mode-adventure {
    color: #ff9500;
    background-color: rgba(255, 149, 0, 0.15);
  }

  .world-tag.mode-spectator {
    color: #af52de;
    background-color: rgba(175, 82, 222, 0.15);
  }

  .pack-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: var(--radius-sm);
  }

  .pack-badge.from-pack {
    color: #bf5af2;
    background-color: rgba(191, 90, 242, 0.15);
  }

  .pack-badge.active-badge {
    color: #30d158;
    background-color: rgba(48, 209, 88, 0.15);
  }

  .world-meta-row,
  .pack-meta-row,
  .shader-meta-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-caption);
    color: var(--color-secondary-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta-label {
    opacity: 0.75;
    margin-right: 3px;
  }

  .meta-dot {
    opacity: 0.5;
  }

  .folder-name {
    font-family: monospace;
    opacity: 0.7;
  }

  .world-actions,
  .pack-actions,
  .shader-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .toggle-wrap {
    display: flex;
    align-items: center;
  }

  .active-indicator {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-system-blue);
    padding: 6px 12px;
  }

  .shaders-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 4px;
  }

  /* Screenshots Grid */
  .screenshots-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
    max-width: 1100px;
  }

  .screenshot-card {
    display: flex;
    flex-direction: column;
    padding: 0;
    border-radius: var(--radius-md);
    background-color: var(--glass-surface-translucent);
    border: 1px solid var(--glass-border);
    overflow: hidden;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    transition: transform 0.15s ease, background-color 0.15s ease, border-color 0.15s ease;
  }

  .screenshot-card:hover {
    transform: translateY(-2px);
    background-color: var(--color-system-fill);
    border-color: rgba(255, 255, 255, 0.22);
  }

  .screenshot-card:active {
    transform: scale(0.98);
  }

  .screenshot-card:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .screenshot-thumb-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background-color: var(--color-secondary-system-fill);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .screenshot-thumb-wrap.broken {
    background: repeating-linear-gradient(
      45deg,
      rgba(255, 255, 255, 0.05),
      rgba(255, 255, 255, 0.05) 10px,
      rgba(0, 0, 0, 0.05) 10px,
      rgba(0, 0, 0, 0.05) 20px
    );
  }

  .screenshot-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb-hover-overlay {
    position: absolute;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    opacity: 0;
    transition: opacity 0.15s ease;
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
  }

  .screenshot-card:hover .thumb-hover-overlay {
    opacity: 1;
  }

  .overlay-text {
    font-size: 11px;
    font-weight: 500;
    color: #ffffff;
  }

  .screenshot-caption {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .caption-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .caption-meta {
    font-size: 11px;
    color: var(--color-secondary-label);
  }

  .screenshot-thumb-btn {
    all: unset;
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: pointer;
  }

  .overlay-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: center;
  }

  .overlay-btn {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background-color: rgba(255, 255, 255, 0.2);
    color: #ffffff;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s ease, transform 0.1s ease;
  }

  .overlay-btn:hover {
    background-color: rgba(255, 255, 255, 0.35);
    transform: scale(1.05);
  }

  .screenshot-preview-dialog {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 0;
  }

  .preview-img-container {
    width: 100%;
    max-height: 400px;
    border-radius: var(--radius-md);
    overflow: hidden;
    background-color: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-img-full {
    width: 100%;
    height: auto;
    max-height: 400px;
    object-fit: contain;
  }

  .preview-meta-row {
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: var(--color-secondary-label);
    justify-content: center;
  }

  .preview-actions-bar {
    display: flex;
    gap: 12px;
    justify-content: center;
  }

  /* Modals Body */
  .modal-dialog-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 4px;
  }

  .dialog-icon-destructive,
  .dialog-icon-warning {
    display: flex;
    justify-content: center;
    margin-bottom: 8px;
  }

  .dialog-text {
    font-size: var(--text-subheadline);
    line-height: 1.5;
    color: var(--color-label);
    margin: 0;
  }

  .dialog-subtext {
    font-size: var(--text-caption);
    line-height: 1.4;
    color: var(--color-secondary-label);
    margin: 0;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 16px;
  }

  .modal-input-wrap {
    margin: 6px 0;
  }

  .apple-text-field {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
    color: var(--color-label);
    font-size: var(--text-subheadline);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .apple-text-field:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
  }

  .modal-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 300px;
    overflow-y: auto;
    margin-top: 8px;
  }

  .backup-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
    gap: 12px;
  }

  .backup-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .backup-filename {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .backup-meta {
    font-size: 11px;
    color: var(--color-secondary-label);
  }

  .empty-modal-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px;
    gap: 8px;
    color: var(--color-secondary-label);
    font-size: var(--text-subheadline);
    text-align: center;
  }
</style>
