<!--
  Wex Launcher — Screen «Мастерская» (SPEC 1.7)
  Apple HIG specifications:
  - https://developer.apple.com/design/human-interface-guidelines/segmented-controls
  - https://developer.apple.com/design/human-interface-guidelines/lists-and-tables
  - https://developer.apple.com/design/human-interface-guidelines/collections
  - https://developer.apple.com/design/human-interface-guidelines/buttons
  - https://developer.apple.com/design/human-interface-guidelines/sheets
  - https://developer.apple.com/design/human-interface-guidelines/content-view-empty-states
  Rules:
  - Segmented control switches between «Миры» and «Скриншоты»
  - Inset Grouped list for single-player worlds with game mode, last played date, and size
  - Secondary button for creating compressed zip backups
  - Destructive red button for deleting a world with modal confirmation
  - Modal sheet for backup restore confirmation
  - Uniform grid collection for screenshots with system file manager reveal
  - Authentic empty states without mock data or slogans
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../components/Button.svelte';
  import Sheet from '../components/Sheet.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import { formatError } from '../design/errors';
  import type { WorldItemDto, WorldBackupItemDto, ScreenshotItemDto } from '../ipc/generated';

  interface Props {
    selectedVersion?: string;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) => void;
  }

  let { selectedVersion = '1.21.1', ontoast }: Props = $props();

  // Active section tab: 'worlds' | 'screenshots'
  let activeTab = $state('worlds');
  let currentVersion = $state('1.21.1');

  $effect(() => {
    if (selectedVersion) {
      currentVersion = selectedVersion;
    }
  });

  // Data state
  let worlds = $state<WorldItemDto[]>([]);
  let backups = $state<WorldBackupItemDto[]>([]);
  let screenshots = $state<ScreenshotItemDto[]>([]);
  let isLoading = $state(false);
  let sheetError = $state<unknown>(null);

  // Confirmation modals state
  let deleteSheetOpen = $state(false);
  let worldToDelete = $state<WorldItemDto | null>(null);
  let isDeleting = $state(false);

  let backupsSheetOpen = $state(false);
  let selectedWorldForBackups = $state<WorldItemDto | null>(null);

  let restoreSheetOpen = $state(false);
  let backupToRestore = $state<WorldBackupItemDto | null>(null);
  let isRestoring = $state(false);

  let isBackingUp = $state<Record<string, boolean>>({});

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

  // Convert local file path to Tauri asset URI if running in Tauri
  function getSafeAssetUrl(filePath: string | null): string | null {
    if (!filePath) return null;
    try {
      // In Tauri 2, convertFileSrc converts windows/unix path to asset URL
      // If window.__TAURI_INTERNALS__ is not present (tests), return raw path
      return (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
        ? `https://asset.localhost/${encodeURIComponent(filePath.replace(/\\/g, '/'))}`
        : filePath;
    } catch {
      return filePath;
    }
  }

  // Load Worlds
  async function loadWorlds() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<WorldItemDto[]>('workshop_list_worlds', {
        version: currentVersion,
      });
      worlds = list || [];
    } catch {
      // Offline fallback / in-test fallback
      worlds = [];
    } finally {
      isLoading = false;
    }
  }

  // Load Screenshots
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

  // Create Backup
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

  // Open Backups Sheet
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

  // Trigger Restore Confirmation
  function promptRestore(backup: WorldBackupItemDto) {
    backupToRestore = backup;
    sheetError = null;
    restoreSheetOpen = true;
  }

  // Execute Restore
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

  // Trigger Delete Confirmation
  function promptDelete(world: WorldItemDto) {
    worldToDelete = world;
    sheetError = null;
    deleteSheetOpen = true;
  }

  // Execute Delete
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

  // Open Path in Explorer
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
    if (activeTab === 'worlds') {
      loadWorlds();
    } else {
      loadScreenshots();
    }
  }

  onMount(() => {
    loadWorlds();
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
            { value: 'screenshots', label: 'Скриншоты' },
          ]}
          bind:value={activeTab}
          onchange={(val: string) => {
            activeTab = val;
            if (val === 'worlds') loadWorlds();
            else loadScreenshots();
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

      <Button
        variant="ghost"
        size="small"
        onclick={() => (activeTab === 'worlds' ? loadWorlds() : loadScreenshots())}
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
      <!-- WORLDS TAB -->
      {#if worlds.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-worlds">
          <div class="empty-icon-wrap">
            <Icon name="folder" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h3 class="empty-title">Сохранённых миров нет</h3>
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
    {:else}
      <!-- SCREENSHOTS TAB -->
      {#if screenshots.length === 0 && !isLoading}
        <div class="empty-state-card" data-testid="empty-screenshots">
          <div class="empty-icon-wrap">
            <Icon name="sun" size={36} color="var(--color-tertiary-label)" />
          </div>
          <h3 class="empty-title">Снимков экрана пока нет</h3>
          <p class="empty-description">
            Нажмите клавишу F2 во время игры, чтобы сделать снимок экрана в Minecraft {currentVersion}.
          </p>
        </div>
      {:else}
        <div class="screenshots-grid" data-testid="screenshots-grid">
          {#each screenshots as shot (shot.file_name)}
            <button
              type="button"
              class="screenshot-card"
              onclick={() => handleOpenPath(shot.file_path)}
              title="Нажмите, чтобы открыть в проводнике"
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
                  <Icon name="folder" size={18} color="#ffffff" />
                  <span class="overlay-text">Открыть в проводнике</span>
                </div>
              </div>

              <div class="screenshot-caption">
                <span class="caption-name" title={shot.file_name}>{shot.file_name}</span>
                <span class="caption-meta">
                  {formatBytes(shot.size_bytes)} • {formatIsoDate(shot.created_at)}
                </span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

<!-- Modal Sheet: Delete Confirmation (Apple HIG Alerts / Destructive Sheet) -->
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
    backdrop-filter: var(--glass-blur-header);
    -webkit-backdrop-filter: var(--glass-blur-header);
    background-color: var(--glass-surface-overlay);
    gap: 16px;
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
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
    max-width: 440px;
    border-radius: var(--radius-lg);
    background-color: var(--glass-surface-translucent);
    border: 1px solid var(--glass-border);
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
    margin: 0;
  }

  /* Worlds List */
  .worlds-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 900px;
  }

  .world-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background-color: var(--glass-surface-translucent);
    border: 1px solid var(--glass-border);
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .world-card:hover {
    background-color: var(--color-system-fill);
    border-color: rgba(255, 255, 255, 0.18);
  }

  .world-icon-wrap {
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

  .world-icon-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }

  .world-icon-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .world-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .world-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .world-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
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

  .world-meta-row {
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

  .world-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
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
