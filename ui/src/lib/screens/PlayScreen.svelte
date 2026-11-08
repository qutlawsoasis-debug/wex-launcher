<!--
  Wex Launcher — Screen «Играть» (Иммерсивный дизайн Apple HIG)
  Specification:
  - Real in-game scenery background, crisp without blur.
  - Floating Liquid Glass launch dock and info cards.
  - Left-aligned typography (Apple scale, natural Russian, no marketing fluff).
  - Version selector with full label visibility (no truncation).
  - Dynamic runtime Java (1.20.1 -> Java 17, 1.21.1 -> Java 21).
  - Real IPC data for servers and empty states without fabricated metrics.
  - wex-overview: news/changelog block from wex-social (hidden when no source/empty),
    unobtrusive update notification banner, and first-run import wizard sheet.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../components/Button.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import ImportWizardSheet from '../components/ImportWizardSheet.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import { theme } from '../design/theme.svelte';

  interface ReleaseNoteItem {
    category?: string;
    text: string;
    raw?: string;
  }

  interface ReleaseNoteVersion {
    version: string;
    internal_version?: string;
    date_text?: string;
    short_title?: string;
    is_current?: boolean;
    items?: ReleaseNoteItem[];
  }

  interface UpdateInfo {
    update_available: boolean;
    current_version: string;
    latest_version?: string;
    notes?: string;
  }

  interface Props {
    selectedVersion?: string;
    playerName?: string;
    clientStatus?: string;
    isLaunching?: boolean;
    isRunning?: boolean;
    progressPercentage?: number;
    progressStage?: string;
    progressDetail?: string;
    progressSpeed?: string;
    launchError?: unknown;
    initialNews?: ReleaseNoteVersion[];
    initialUpdateInfo?: UpdateInfo | null;
    initialShowImportWizard?: boolean;
    onlaunch?: () => void;
    oncancel?: () => void;
    onstop?: () => void;
    onopensettings?: () => void;
  }

  let {
    selectedVersion = $bindable('1.21.1'),
    playerName = $bindable('Player'),
    clientStatus = '',
    isLaunching = false,
    isRunning = false,
    progressPercentage = 0,
    progressStage = 'Готов к запуску',
    progressDetail = 'Нажмите «Играть» для старта',
    progressSpeed = '',
    launchError = $bindable<unknown>(null),
    initialNews = undefined,
    initialUpdateInfo = null,
    initialShowImportWizard = false,
    onlaunch,
    oncancel,
    onstop,
    onopensettings,
  }: Props = $props();

  const versions = [
    { value: '1.20.1', label: '1.20.1' },
    { value: '1.21.1', label: '1.21.1' },
  ];

  // Dynamic Java runtime strictly matches selected version
  let javaRuntime = $derived(
    selectedVersion === '1.20.1' ? 'Java 17 • 64-bit' : 'Java 21 • 64-bit'
  );

  // News state (wex-social news)
  let fetchedNews = $state<ReleaseNoteVersion[]>([]);
  let newsVersions = $derived(initialNews !== undefined ? initialNews : fetchedNews);
  let latestVersionNote = $derived(newsVersions.length > 0 ? newsVersions[0] : null);

  // Formatted items with Apple HIG badges
  let formattedNewsItems = $derived.by(() => {
    if (!latestVersionNote || !latestVersionNote.items) return [];
    return latestVersionNote.items.map((item) => {
      const raw = item.raw || item.text || '';
      const cat = (item.category || '').toLowerCase();

      if (cat === 'new' || raw.startsWith('[Новое]')) {
        return {
          categoryClass: 'new',
          badgeText: 'Новое',
          text: item.text.replace(/^\[Новое\]\s*/, ''),
        };
      }
      if (cat === 'improved' || raw.startsWith('[Улучшено]')) {
        return {
          categoryClass: 'improved',
          badgeText: 'Улучшено',
          text: item.text.replace(/^\[Улучшено\]\s*/, ''),
        };
      }
      if (cat === 'fixed' || raw.startsWith('[Исправлено]')) {
        return {
          categoryClass: 'fixed',
          badgeText: 'Исправлено',
          text: item.text.replace(/^\[Исправлено\]\s*/, ''),
        };
      }
      return {
        categoryClass: 'general',
        badgeText: 'Инфо',
        text: item.text,
      };
    });
  });

  // Recent server state from IPC
  let recentServer = $state<{ name: string; address: string; ping_ms?: number } | null>(null);

  // Unobtrusive update notification banner state
  let fetchedUpdateInfo = $state<UpdateInfo | null>(null);
  let updateInfo = $derived(initialUpdateInfo !== null ? initialUpdateInfo : fetchedUpdateInfo);
  let updateBannerDismissed = $state(false);
  let showUpdateBanner = $derived(Boolean(updateInfo?.update_available) && !updateBannerDismissed);

  // Import wizard sheet state
  let manualImportWizardOpen = $state(false);
  let isImportWizardOpen = $derived(Boolean(initialShowImportWizard) || manualImportWizardOpen);

  onMount(async () => {
    // 1. Fetch servers from IPC
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<Array<{ name: string; address: string; ping_ms?: number }>>('list_servers');
      if (list && list.length > 0) {
        recentServer = list[0];
      } else {
        recentServer = null;
      }
    } catch {
      recentServer = null;
    }

    // 2. Fetch news from wex-social / overview IPC if not provided via props
    if (initialNews === undefined) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke<any>('overview_get_news');
        if (Array.isArray(res) && res.length > 0) {
          fetchedNews = res;
        } else if (res?.versions && Array.isArray(res.versions) && res.versions.length > 0) {
          fetchedNews = res.versions;
        } else {
          fetchedNews = [];
        }
      } catch {
        fetchedNews = [];
      }
    }

    // 3. Unobtrusive launcher update check
    if (initialUpdateInfo === null) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke<UpdateInfo>('overview_check_update');
        if (res && res.update_available) {
          fetchedUpdateInfo = res;
        }
      } catch {
        // Silent fallback for update check
      }
    }

    // 4. Check for old launcher data on first launch
    try {
      if (typeof localStorage !== 'undefined' && !localStorage.getItem('wex_import_checked')) {
        localStorage.setItem('wex_import_checked', 'true');
        const { invoke } = await import('@tauri-apps/api/core');
        const detected = await invoke<string | null>('extras_detect_old_launcher');
        if (detected) {
          manualImportWizardOpen = true;
        }
      }
    } catch {
      // Ignore if localStorage unavailable
    }
  });

  function handleImportCompleted(result: {
    nickname?: string;
    ramMb?: number;
    selectedVersion?: string;
    serversCount: number;
    skinsCount: number;
  }) {
    if (result.nickname) {
      playerName = result.nickname;
    }
    if (result.selectedVersion) {
      selectedVersion = result.selectedVersion;
    }
  }
</script>

<div class="play-screen-immersive">
  <!-- Interactive Content Layer -->
  <div class="play-content-container">
    <!-- Unobtrusive Update Notification Banner (Apple HIG Banners & Alerts) -->
    {#if showUpdateBanner && updateInfo}
      <div class="update-banner" role="status" aria-live="polite">
        <div class="update-banner-icon">
          <Icon name="info" size={16} color="var(--color-system-blue)" />
        </div>
        <div class="update-banner-body">
          <span class="update-banner-headline">
            Доступна новая версия Wex {updateInfo.latest_version || ''}
          </span>
          {#if updateInfo.notes}
            <span class="update-banner-subtext">{updateInfo.notes}</span>
          {/if}
        </div>
        <div class="update-banner-close">
          <Button
            variant="ghost"
            size="small"
            onclick={() => (updateBannerDismissed = true)}
            title="Закрыть уведомление"
          >
            <Icon name="close" size={14} color="var(--color-secondary-label)" />
          </Button>
        </div>
      </div>
    {/if}

    <!-- Top Header: Natural Apple Typography -->
    <header class="app-header-area">
      <div class="header-main-row">
        <div class="header-titles">
          <h1 class="game-heading">Minecraft</h1>
          <p class="game-subheading">Клиент Wex • Версия {selectedVersion}</p>
        </div>
        <div class="header-tools">
          <Button
            variant="secondary"
            size="small"
            onclick={() => (manualImportWizardOpen = true)}
            title="Мастер импорта данных из старого лаунчера"
          >
            <Icon name="folder" size={14} color="var(--color-secondary-label)" />
            <span>Импорт</span>
          </Button>
        </div>
      </div>
    </header>

    <!-- Middle Shelf: Real Updates & Activity Glance -->
    <div class="dashboard-shelf">
      <!-- Changelog Card: STRICTLY HIDDEN IF NO NEWS SOURCE (Apple HIG & Task Spec) -->
      {#if newsVersions.length > 0 && latestVersionNote}
        <div class="glass-card updates-card">
          <div class="card-title-row">
            <Icon name="play" size={15} color="var(--color-system-blue)" />
            <span class="card-title">Журнал изменений</span>
            <span class="card-meta">
              {latestVersionNote.version}{latestVersionNote.date_text ? ` • ${latestVersionNote.date_text}` : ''}
            </span>
          </div>

          {#if formattedNewsItems.length > 0}
            <div class="changelog-entries">
              {#each formattedNewsItems as item}
                <div class="changelog-entry-row">
                  <span class="badge badge-{item.categoryClass}">{item.badgeText}</span>
                  <span class="changelog-entry-text">{item.text}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Recent Server & Friends Glance -->
      <div class="glass-card status-card">
        <div class="card-title-row">
          <Icon name="server" size={15} color="var(--color-secondary-label)" />
          <span class="card-title">Последний сервер</span>
        </div>

        {#if recentServer}
          <div class="recent-server-item">
            <span class="status-indicator-green"></span>
            <div class="server-text-wrap">
              <span class="server-display-name">{recentServer.name}</span>
              <span class="server-network-meta">
                {recentServer.address}{recentServer.ping_ms !== undefined ? ` • ${recentServer.ping_ms} мс` : ''}
              </span>
            </div>
          </div>
        {:else}
          <div class="empty-status-text">Вы ещё не играли на серверах</div>
        {/if}

        <div class="shelf-card-divider"></div>

        <div class="friends-glance-row">
          <Icon name="friends" size={15} color="var(--color-tertiary-label)" />
          <span class="friends-status-label">Нет друзей в игре</span>
        </div>
      </div>
    </div>

    <!-- Launch Error Banner (Apple HIG Alert State) -->
    {#if launchError}
      <div class="launch-error-banner" role="alert">
        <ErrorState
          error={launchError}
          domain="launch"
          compact={true}
          onaction={onlaunch}
          onsecondaryaction={onopensettings}
          onclose={() => {
            launchError = null;
          }}
        />
      </div>
    {/if}

    <!-- Bottom Floating Launch Dock (Liquid Glass) -->
    <div class="launch-dock">
      {#if isLaunching}
        <div class="dock-progress-mode dock-progress">
          <ProgressBar
            percentage={progressPercentage}
            label={progressStage}
            detail={progressSpeed ? `${progressSpeed} • ${progressDetail}` : progressDetail}
            height={8}
          />
          <div class="dock-cancel-action dock-cancel">
            <Button variant="secondary" size="regular" onclick={oncancel}>
              Отмена
            </Button>
          </div>
        </div>
      {:else}
        <div class="dock-controls-row">
          <!-- Version Selector -->
          <div class="dock-block version-block">
            <span class="dock-caption">Версия игры</span>
            <div class="version-segmented-container">
              <SegmentedControl
                options={versions}
                bind:value={selectedVersion}
                disabled={isLaunching || isRunning}
              />
            </div>
          </div>

          <div class="dock-vertical-divider"></div>

          <!-- Player Nickname -->
          <div class="dock-block nickname-block">
            <span class="dock-caption">Имя игрока</span>
            <div class="nickname-field-box">
              <Icon name="friends" size={15} color="var(--color-tertiary-label)" />
              <input
                type="text"
                bind:value={playerName}
                placeholder="Имя игрока"
                class="player-name-input nickname-input"
                maxlength={16}
                disabled={isLaunching || isRunning}
                aria-label="Имя игрока"
              />
            </div>
          </div>

          <div class="dock-vertical-divider"></div>

          <!-- Java Environment & Core Status -->
          <div class="dock-block tech-block">
            <span class="dock-caption">
              {isRunning ? 'Статус сессии' : 'Среда выполнения'}
            </span>
            <div class="tech-runtime-box">
              <span class="tech-runtime-text">{javaRuntime}</span>
              {#if isRunning}
                <span class="tech-status-badge running">• В игре</span>
              {:else if clientStatus}
                <span class="tech-status-badge">• {clientStatus}</span>
              {/if}
            </div>
          </div>

          <!-- Action Button: Play (systemBlue) -> In Game / Stop (neutral) -->
          <div class="dock-primary-action dock-action" class:is-running={isRunning}>
            {#if isRunning}
              <Button
                variant="secondary"
                size="large"
                onclick={onstop}
                title="Остановить процесс Minecraft"
              >
                <span class="running-indicator-dot"></span>
                <span>В игре • Остановить</span>
              </Button>
            {:else}
              <Button
                variant="primary"
                size="large"
                onclick={onlaunch}
                disabled={!playerName.trim() || isLaunching || isRunning}
              >
                <Icon name="play" size={18} color="#ffffff" />
                <span>Играть</span>
              </Button>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- First Launch Import Wizard Sheet -->
  <ImportWizardSheet
    open={isImportWizardOpen}
    onclose={() => {
      manualImportWizardOpen = false;
    }}
    onimported={handleImportCompleted}
  />
</div>

<style>
  .play-screen-immersive {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .play-content-container {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 100%;
    padding: 24px 44px 28px;
    box-sizing: border-box;
  }

  /* Unobtrusive Update Banner (Apple HIG Banners) */
  .update-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: var(--color-system-thin-material);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    margin-bottom: 12px;
    animation: fadeIn 0.3s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .update-banner-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(0, 145, 255, 0.12);
    flex-shrink: 0;
  }

  .update-banner-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .update-banner-headline {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-label);
  }

  .update-banner-subtext {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  .update-banner-close {
    flex-shrink: 0;
  }

  /* Header Area: Apple Large Title */
  .app-header-area {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    padding-top: 8px;
  }

  .header-main-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .game-heading {
    font-size: 44px;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.08;
    margin: 0 0 4px;
    color: var(--color-label);
  }

  .game-subheading {
    font-size: 16px;
    color: var(--color-secondary-label);
    margin: 0;
    font-weight: 500;
  }

  .header-tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Shelf Cards: Elevated Composition */
  .dashboard-shelf {
    display: flex;
    gap: 24px;
    margin-top: 20px;
    margin-bottom: auto;
    flex-wrap: wrap;
  }

  .glass-card {
    background: var(--color-system-ultra-thin-material);
    backdrop-filter: blur(28px) saturate(160%);
    -webkit-backdrop-filter: blur(28px) saturate(160%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    box-sizing: border-box;
  }

  .updates-card {
    flex: 1 1 380px;
    max-width: 520px;
  }

  .status-card {
    flex: 0 1 300px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .card-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .card-meta {
    margin-left: auto;
    font-size: var(--text-caption-2);
    color: var(--color-tertiary-label);
  }

  /* Changelog entries with Apple HIG badges */
  .changelog-entries {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .changelog-entry-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    line-height: 1.4;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 9999px;
    padding: 1px 7px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: -0.01em;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .badge-new {
    background: rgba(0, 145, 255, 0.16);
    color: var(--color-system-blue);
  }

  .badge-improved {
    background: var(--color-secondary-system-fill);
    color: var(--color-label);
  }

  .badge-fixed {
    background: rgba(255, 159, 10, 0.16);
    color: var(--color-system-orange);
  }

  .badge-general {
    background: var(--color-tertiary-system-fill);
    color: var(--color-secondary-label);
  }

  .changelog-entry-text {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    flex: 1;
  }

  .recent-server-item {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 2px 0;
  }

  .status-indicator-green {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-system-green);
    flex-shrink: 0;
  }

  .server-text-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .server-display-name {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
  }

  .server-network-meta {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
  }

  .empty-status-text {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    padding: 4px 0;
  }

  .shelf-card-divider {
    height: 1px;
    background-color: var(--color-separator);
    margin: 10px 0 8px;
  }

  .friends-glance-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  .friends-status-label {
    font-size: var(--text-footnote);
  }

  .launch-error-banner {
    margin-bottom: 14px;
    max-width: 680px;
    width: 100%;
    margin-left: auto;
    margin-right: auto;
  }

  /* Bottom Launch Dock */
  .launch-dock {
    background: var(--color-system-thin-material);
    backdrop-filter: blur(36px) saturate(180%);
    -webkit-backdrop-filter: blur(36px) saturate(180%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl);
    padding: 12px 20px;
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.28);
  }

  .dock-controls-row {
    display: flex;
    align-items: center;
    gap: 20px;
  }

  .dock-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .dock-caption {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-secondary-label);
    text-transform: none;
    letter-spacing: -0.01em;
  }

  .version-block {
    flex-shrink: 0;
  }

  .version-segmented-container {
    width: 170px;
  }

  .dock-vertical-divider {
    width: 1px;
    height: 38px;
    background-color: var(--color-separator);
    flex-shrink: 0;
  }

  .nickname-block {
    flex: 1 1 200px;
    max-width: 260px;
  }

  .nickname-field-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-secondary-system-fill);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 0 10px;
    height: 32px;
    box-sizing: border-box;
    transition: border-color var(--motion-duration-short) ease, box-shadow var(--motion-duration-short) ease, background-color var(--motion-duration-short) ease;
  }

  .nickname-field-box:focus-within {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
    background: var(--color-system-background);
  }

  .player-name-input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-footnote);
    width: 100%;
  }

  .tech-block {
    flex-shrink: 0;
  }

  .tech-runtime-box {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 34px;
    white-space: nowrap;
  }

  .tech-runtime-text {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-family: var(--font-mono);
    font-weight: 500;
    display: flex;
    align-items: center;
    line-height: 32px;
  }

  .tech-status-badge {
    font-size: 11px;
    color: var(--color-secondary-label);
  }

  .tech-status-badge.running {
    color: var(--color-system-green);
    font-weight: 600;
  }

  .running-indicator-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-system-green);
    margin-right: 6px;
    box-shadow: 0 0 6px rgba(48, 209, 88, 0.6);
  }

  .dock-primary-action {
    margin-left: auto;
    flex-shrink: 0;
  }

  .dock-progress-mode {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
  }

  .dock-progress-mode :global(.progress-bar-wrapper) {
    flex: 1;
  }

  .dock-cancel-action {
    flex-shrink: 0;
  }
</style>
