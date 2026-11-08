<!--
  Apple HIG First Launch / Onboarding Import Wizard Sheet
  Specification:
  - Apple HIG Sheets: https://developer.apple.com/design/human-interface-guidelines/sheets
  - Apple HIG Onboarding: https://developer.apple.com/design/human-interface-guidelines/onboarding
  - Apple HIG Toggles: https://developer.apple.com/design/human-interface-guidelines/toggles
  - Apple HIG Lists and Tables: https://developer.apple.com/design/human-interface-guidelines/lists-and-tables
  Rules:
  - Non-destructive: clearly states old launcher data is never altered or deleted.
  - Transparent itemization: displays exactly what was found and lets user pick what to transfer.
  - Sentence case Russian typography, system colors, no marketing slogans.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Sheet from './Sheet.svelte';
  import Button from './Button.svelte';
  import Toggle from './Toggle.svelte';
  import Icon from './icons/Icon.svelte';
  import Row from './Row.svelte';
  import { t, i18n } from '../i18n';
  import type { OldLauncherDataDto, ImportedServerDto, ImportedSkinDto } from '../ipc/generated/models';

  interface Props {
    open?: boolean;
    initialDir?: string;
    initialData?: OldLauncherDataDto | null;
    onimported?: (result: {
      nickname?: string;
      ramMb?: number;
      selectedVersion?: string;
      serversCount: number;
      skinsCount: number;
      worldsCount?: number;
    }) => void;
    onclose?: () => void;
  }

  let {
    open = $bindable(false),
    initialDir = '',
    initialData = null,
    onimported,
    onclose,
  }: Props = $props();

  let scannedDir = $state('');
  let scannedData = $state<OldLauncherDataDto | null>(null);

  let oldDir = $derived(initialDir ? initialDir : scannedDir);
  let oldData = $derived(initialData !== null ? initialData : scannedData);

  let isScanning = $state(false);
  let isImporting = $state(false);
  let errorMessage = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  // What to transfer selections
  let importNickname = $state(true);
  let importRam = $state(true);
  let importVersion = $state(true);
  let importServers = $state(true);
  let importSkins = $state(true);
  let importWorlds = $state(true);

  export async function scanLauncher(targetDir?: string) {
    isScanning = true;
    errorMessage = null;
    successMessage = null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');

      let dirToScan = targetDir || oldDir;
      if (!dirToScan) {
        const detected = await invoke<string | null>('extras_detect_old_launcher');
        if (detected) {
          dirToScan = detected;
          scannedDir = detected;
        }
      }

      if (!dirToScan) {
        errorMessage = 'Каталог старого лаунчера не найден автоматически.';
        isScanning = false;
        return;
      }

      const data = await invoke<OldLauncherDataDto>('extras_import_old_launcher', {
        request: { old_launcher_dir: dirToScan },
      });

      scannedData = data;
    } catch (err: any) {
      errorMessage = err?.message || String(err) || 'Ошибка при чтении данных старого лаунчера';
    } finally {
      isScanning = false;
    }
  }

  async function handleExecuteImport() {
    if (!oldData || isImporting) return;
    isImporting = true;
    errorMessage = null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<{
        success: boolean;
        imported_nickname?: string;
        imported_ram_gb?: number;
        imported_servers_count: number;
        imported_skins_count: number;
        imported_worlds_count: number;
        message: string;
      }>('extras_apply_import', {
        request: {
          old_launcher_dir: oldDir,
          import_nickname: importNickname,
          nickname: oldData.nickname,
          import_ram: importRam,
          ram_mb: oldData.ram_mb,
          import_version: importVersion,
          selected_version: oldData.selected_version,
          graphics_preset: oldData.graphics_preset,
          import_servers: importServers,
          servers: oldData.servers || [],
          import_skins: importSkins,
          skins: oldData.skins || [],
          capes: oldData.capes || [],
          import_worlds: importWorlds,
          worlds: oldData.worlds || [],
        },
      });

      if (response && response.success) {
        successMessage = response.message;
        onimported?.({
          nickname: importNickname ? oldData.nickname : undefined,
          ramMb: importRam ? oldData.ram_mb : undefined,
          selectedVersion: importVersion ? oldData.selected_version : undefined,
          serversCount: response.imported_servers_count,
          skinsCount: response.imported_skins_count,
          worldsCount: response.imported_worlds_count,
        });

        setTimeout(() => {
          open = false;
          onclose?.();
        }, 1200);
      }
    } catch (err: any) {
      errorMessage = err?.message || String(err) || 'Ошибка при переносе данных';
    } finally {
      isImporting = false;
    }
  }

  $effect(() => {
    if (open && !oldData && !isScanning && !errorMessage) {
      scanLauncher();
    }
  });
</script>

<Sheet
  bind:open
  title={t('import.sheet_title')}
  cancelLabel={t('import.action_skip')}
  doneLabel={isImporting ? (i18n.locale === 'en' ? 'Importing…' : 'Импорт...') : t('import.action_import')}
  showCancel={true}
  showDone={Boolean(oldData) && !successMessage}
  maxWidth="580px"
  ondone={handleExecuteImport}
  onclose={() => {
    open = false;
    onclose?.();
  }}
>
  <div class="import-wizard-container">
    {#if isScanning}
      <div class="scanning-state">
        <Icon name="refresh" size={24} color="var(--color-system-blue)" />
        <p class="scanning-text">{t('import.scan_progress')}</p>
      </div>
    {:else if successMessage}
      <div class="success-state">
        <div class="success-icon-wrap">
          <Icon name="check" size={28} color="var(--color-system-green)" />
        </div>
        <h3 class="success-title">{t('import.success')}</h3>
        <p class="success-description">{successMessage}</p>
      </div>
    {:else if oldData}
      <!-- Explanation of safe isolation -->
      <div class="isolation-notice">
        <Icon name="info" size={16} color="var(--color-system-blue)" />
        <span class="notice-text">
          {t('import.found_desc')}
        </span>
      </div>

      <!-- Found directory path -->
      {#if oldDir}
        <div class="found-source-row">
          <span class="source-label">{t('import.path_label')}</span>
          <span class="source-path" title={oldDir}>{oldDir}</span>
        </div>
      {/if}

      <!-- Transfer items checklist -->
      <div class="section-heading">{t('import.items_to_transfer')}</div>

      <div class="transfer-options-list">
        <!-- Nickname -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">{t('import.nickname')}</span>
            <span class="option-subtitle">{oldData.nickname || (i18n.locale === 'en' ? 'Not specified' : 'Не указано')}</span>
          </div>
          <Toggle bind:checked={importNickname} label={i18n.locale === 'en' ? 'Import player name' : 'Перенести имя игрока'} />
        </div>

        <div class="option-divider"></div>

        <!-- RAM -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">{t('import.ram')}</span>
            <span class="option-subtitle">
              {oldData.ram_mb ? `${Math.round(oldData.ram_mb / 1024)} ${i18n.locale === 'en' ? 'GB' : 'ГБ'} (${oldData.ram_mb} ${i18n.locale === 'en' ? 'MB' : 'МБ'})` : (i18n.locale === 'en' ? 'Default' : 'По умолчанию')}
            </span>
          </div>
          <Toggle bind:checked={importRam} label={i18n.locale === 'en' ? 'Import memory settings' : 'Перенести настройки памяти'} />
        </div>

        <div class="option-divider"></div>

        <!-- Version -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">{t('import.version')}</span>
            <span class="option-subtitle">{oldData.selected_version || '1.21.1'}</span>
          </div>
          <Toggle bind:checked={importVersion} label={i18n.locale === 'en' ? 'Import selected game version' : 'Перенести выбранную версию'} />
        </div>

        <div class="option-divider"></div>

        <!-- Servers -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">{t('import.servers')}</span>
            <span class="option-subtitle">
              {oldData.servers?.length
                ? (i18n.locale === 'en' ? `Servers found: ${oldData.servers.length}` : `${oldData.servers.length} сервер(-ов) найдено`)
                : (i18n.locale === 'en' ? 'No servers found' : 'Серверы не найдены')}
            </span>
          </div>
          <Toggle
            bind:checked={importServers}
            disabled={!oldData.servers || oldData.servers.length === 0}
            label={i18n.locale === 'en' ? 'Import saved servers' : 'Перенести сохранённые серверы'}
          />
        </div>

        <div class="option-divider"></div>

        <!-- Skins and wardrobe -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">{t('import.skins')}</span>
            <span class="option-subtitle">
              {oldData.skins?.length
                ? (i18n.locale === 'en' ? `Skins found: ${oldData.skins.length}` : `${oldData.skins.length} скин(-ов) найдено`)
                : (i18n.locale === 'en' ? 'No skins found' : 'Скины не найдены')}
            </span>
          </div>
          <Toggle
            bind:checked={importSkins}
            disabled={!oldData.skins || oldData.skins.length === 0}
            label={i18n.locale === 'en' ? 'Import skins and capes' : 'Перенести скины и плащи'}
          />
        </div>

        <div class="option-divider"></div>

        <!-- Worlds -->
        <div class="option-item">
          <div class="option-details">
            <span class="option-title">Локальные миры</span>
            <span class="option-subtitle">
              {oldData.worlds?.length
                ? `${oldData.worlds.length} мир(-ов) найдено`
                : 'Миры не найдены'}
            </span>
          </div>
          <Toggle
            bind:checked={importWorlds}
            disabled={!oldData.worlds || oldData.worlds.length === 0}
            label="Перенести сохранённые миры"
          />
        </div>
      </div>
    {:else}
      <!-- Empty detect state / error -->
      <div class="empty-detect-state">
        <Icon name="folder" size={32} color="var(--color-tertiary-label)" />
        <h3 class="empty-title">{t('import.not_found')}</h3>
        <p class="empty-subtitle">
          {errorMessage || (i18n.locale === 'en' ? 'No previous launcher settings found. You can continue with the defaults.' : 'Wex не обнаружил каталог .aura в стандартной папке AppData. Вы можете продолжить со стандартными настройками.')}
        </p>
        <div class="empty-actions">
          <Button variant="secondary" size="regular" onclick={() => scanLauncher()}>
            {t('common.repeat')}
          </Button>
        </div>
      </div>
    {/if}

    {#if errorMessage && oldData}
      <div class="inline-error-banner">
        <Icon name="warning" size={15} color="var(--color-system-red)" />
        <span>{errorMessage}</span>
      </div>
    {/if}
  </div>
</Sheet>

<style>
  .import-wizard-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px 0;
  }

  .scanning-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 36px 16px;
    gap: 12px;
  }

  .scanning-text {
    font-size: var(--text-body);
    color: var(--color-secondary-label);
    margin: 0;
  }

  .success-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 24px 16px;
    gap: 8px;
  }

  .success-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--color-secondary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 8px;
  }

  .success-title {
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    color: var(--color-label);
    margin: 0;
  }

  .success-description {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    margin: 0;
  }

  /* Isolation Notice */
  .isolation-notice {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--color-secondary-system-fill);
    color: var(--color-secondary-label);
    font-size: var(--text-footnote);
    line-height: var(--line-footnote);
  }

  .notice-text {
    flex: 1;
  }

  .found-source-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-caption-1);
    color: var(--color-tertiary-label);
    padding: 0 4px;
    overflow: hidden;
  }

  .source-label {
    flex-shrink: 0;
  }

  .source-path {
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .section-heading {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-secondary-label);
    text-transform: none;
    margin-top: 4px;
  }

  /* Transfer options list */
  .transfer-options-list {
    display: flex;
    flex-direction: column;
    background: var(--color-secondary-system-fill);
    border-radius: 12px;
    overflow: hidden;
    padding: 4px 16px;
  }

  .option-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;
    gap: 16px;
  }

  .option-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .option-title {
    font-size: var(--text-body);
    color: var(--color-label);
  }

  .option-subtitle {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
  }

  .option-divider {
    height: 1px;
    background: var(--color-separator);
    margin-left: 0;
  }

  /* Empty state */
  .empty-detect-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 32px 16px;
    gap: 8px;
  }

  .empty-title {
    font-size: var(--text-headline);
    color: var(--color-label);
    margin: 8px 0 0;
  }

  .empty-subtitle {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    margin: 0;
    max-width: 400px;
    line-height: var(--line-footnote);
  }

  .empty-actions {
    margin-top: 12px;
  }

  .inline-error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(255, 69, 58, 0.12);
    color: var(--color-system-red);
    font-size: var(--text-footnote);
  }
</style>
