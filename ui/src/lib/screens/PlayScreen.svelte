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
  import StoriesCarousel, { type StorySlideData } from '../components/StoriesCarousel.svelte';
  import { theme } from '../design/theme.svelte';
  import { t, i18n } from '../i18n';
  import type { ScreenshotItemDto } from '../ipc/generated';

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

  interface PlaytimeSummary {
    total_seconds: number;
    total_formatted: string;
    version_seconds: number;
    version_formatted: string;
    version_summary: string;
    total_launches: number;
    last_played_utc?: string | null;
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
    initialScreenshots?: Array<{ file_name: string; file_path: string; preview_data_url?: string; created_at?: string }>;
    initialAchievements?: { achievements: any[]; progress: any[] } | null;
    initialPlaytime?: { total_seconds: number; total_launches: number; formatted_play_time: string; last_played_utc?: string } | null;
    onlaunch?: () => void;
    oncancel?: () => void;
    onstop?: () => void;
    onopensettings?: () => void;
    onnavigatescreen?: (screen: string) => void;
    ontoast?: (toast: { type: 'info' | 'success' | 'warning' | 'error'; title: string; message: string }) => void;
    onopenworkshop?: (subtab?: string) => void;
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
    initialScreenshots = undefined,
    initialAchievements = undefined,
    initialPlaytime = undefined,
    onlaunch,
    oncancel,
    onstop,
    onopensettings,
    onnavigatescreen,
    ontoast,
    onopenworkshop,
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

  // Latest screenshot from Workshop (Parity Item 10)
  let latestScreenshot = $state<ScreenshotItemDto | null>(null);

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

  async function loadLatestScreenshot() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<ScreenshotItemDto[]>('workshop_list_screenshots', {
        version: selectedVersion,
      });
      if (list && list.length > 0) {
        latestScreenshot = list[0];
      } else {
        latestScreenshot = null;
      }
    } catch {
      latestScreenshot = null;
    }
  }

  $effect(() => {
    if (selectedVersion) {
      loadLatestScreenshot();
    }
  });

  // Unobtrusive update notification banner state
  let fetchedUpdateInfo = $state<UpdateInfo | null>(null);
  let updateInfo = $derived(initialUpdateInfo !== null ? initialUpdateInfo : fetchedUpdateInfo);
  let updateBannerDismissed = $state(false);
  let showUpdateBanner = $derived(Boolean(updateInfo?.update_available) && !updateBannerDismissed);

  // Import wizard sheet state
  let manualImportWizardOpen = $state(false);
  let isImportWizardOpen = $derived(Boolean(initialShowImportWizard) || manualImportWizardOpen);

  // Screenshots state (from workshop_list_screenshots)
  let fetchedScreenshots = $state<Array<{ file_name: string; file_path: string; preview_data_url?: string; created_at?: string }>>([]);
  let screenshotsList = $derived(initialScreenshots !== undefined ? initialScreenshots : fetchedScreenshots);

  // Achievements state (from social_list_achievements)
  let fetchedAchievements = $state<{ achievements: any[]; progress: any[] } | null>(null);
  let achievementsData = $derived(initialAchievements !== undefined ? initialAchievements : fetchedAchievements);
  let achievementsSummary = $derived.by(() => {
    if (!achievementsData || !achievementsData.achievements) return null;
    const defs = achievementsData.achievements;
    const prog = achievementsData.progress || [];
    const unlockedList = prog.filter((p: any) => p.unlocked);
    const unlockedCount = unlockedList.length;
    const totalCount = defs.length;

    let latestAch: any = undefined;
    if (unlockedList.length > 0) {
      const lastP = unlockedList[unlockedList.length - 1];
      const foundDef = defs.find((d: any) => d.id === lastP.id);
      if (foundDef) {
        latestAch = {
          id: foundDef.id,
          title: foundDef.title,
          description: foundDef.description,
          icon: foundDef.icon,
          unlockedAt: lastP.unlocked_at,
        };
      }
    } else if (defs.length > 0) {
      latestAch = {
        id: defs[0].id,
        title: defs[0].title,
        description: defs[0].description,
        icon: defs[0].icon,
      };
    }

    return {
      unlockedCount,
      totalCount,
      latestAchievement: latestAch,
    };
  });

  // Playtime state (from overview_get_playtime)
  let fetchedPlaytime = $state<{ total_seconds: number; total_launches: number; formatted_play_time: string; last_played_utc?: string } | null>(null);
  let playtimeData = $derived(initialPlaytime !== undefined ? initialPlaytime : fetchedPlaytime);

  // Stories Carousel Slides (Strictly only real data! Card without data is not displayed)
  let storySlides = $derived.by<StorySlideData[]>(() => {
    const list: StorySlideData[] = [];

    // 1. What's new / Changelog
    if (newsVersions.length > 0 && latestVersionNote && formattedNewsItems.length > 0) {
      list.push({
        type: 'news',
        version: latestVersionNote.version,
        dateText: latestVersionNote.date_text,
        shortTitle: latestVersionNote.short_title,
        items: formattedNewsItems,
      });
    }

    // 2. Latest Screenshot
    if (latestScreenshot) {
      list.push({
        type: 'screenshot',
        fileName: latestScreenshot.file_name,
        filePath: latestScreenshot.file_path,
        previewUrl: (latestScreenshot as any).preview_data_url || (latestScreenshot.file_path ? getSafeAssetUrl(latestScreenshot.file_path) || undefined : undefined),
        createdAt: latestScreenshot.created_at ?? undefined,
      });
    }

    // 3. Achievements
    if (achievementsSummary && (achievementsSummary.unlockedCount > 0 || achievementsSummary.totalCount > 0)) {
      list.push({
        type: 'achievements',
        unlockedCount: achievementsSummary.unlockedCount,
        totalCount: achievementsSummary.totalCount,
        latestAchievement: achievementsSummary.latestAchievement,
      });
    }

    // 4. Playtime (only if real seconds or launches recorded)
    if (playtimeData && (playtimeData.total_seconds > 0 || playtimeData.total_launches > 0)) {
      list.push({
        type: 'playtime',
        totalSeconds: playtimeData.total_seconds,
        formattedPlayTime: playtimeData.formatted_play_time || `${Math.floor(playtimeData.total_seconds / 60)} мин.`,
        totalLaunches: playtimeData.total_launches,
        lastPlayed: playtimeData.last_played_utc ? new Date(playtimeData.last_played_utc).toLocaleDateString() : undefined,
      });
    }

    return list;
  });

  // Session playtime auto-tracking
  let gameSessionStartTime: number | null = null;
  $effect(() => {
    if (isRunning && gameSessionStartTime === null) {
      gameSessionStartTime = Date.now();
    } else if (!isRunning && gameSessionStartTime !== null) {
      const elapsedSec = Math.floor((Date.now() - gameSessionStartTime) / 1000);
      gameSessionStartTime = null;
      if (elapsedSec >= 5) {
        import('@tauri-apps/api/core').then(({ invoke }) => {
          invoke<any>('overview_record_playtime', { addedSeconds: elapsedSec, incLaunch: false })
            .then((res) => {
              if (res) fetchedPlaytime = res;
              loadPlaytime();
            })
            .catch(() => {});
        }).catch(() => {});
      }
    }
  });

  // Playtime summary state
  let playtimeSummary = $state<PlaytimeSummary | null>(null);

  async function loadPlaytime() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<PlaytimeSummary>('get_playtime_summary', { version: selectedVersion });
      if (res) {
        playtimeSummary = res;
      }
    } catch {
      playtimeSummary = null;
    }
  }

  $effect(() => {
    if (selectedVersion) {
      loadPlaytime();
    }
  });

  $effect(() => {
    if (!isRunning) {
      loadPlaytime();
    }
  });

  onMount(async () => {
    // Parallelize all startup IPC calls using Promise.allSettled for fastest first-frame render
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      const fetchServers = invoke<Array<{ name: string; address: string; ping_ms?: number }>>('list_servers')
        .then((list) => {
          recentServer = list && list.length > 0 ? list[0] : null;
        })
        .catch(() => {
          recentServer = null;
        });

      const fetchNews = initialNews === undefined
        ? invoke<any>('overview_get_news')
            .then((res) => {
              if (Array.isArray(res) && res.length > 0) {
                fetchedNews = res;
              } else if (res?.versions && Array.isArray(res.versions) && res.versions.length > 0) {
                fetchedNews = res.versions;
              } else {
                fetchedNews = [];
              }
            })
            .catch(() => {
              fetchedNews = [];
            })
        : Promise.resolve();

      const fetchUpdates = initialUpdateInfo === null
        ? invoke<UpdateInfo>('overview_check_update')
            .then((res) => {
              if (res && res.update_available) {
                fetchedUpdateInfo = res;
              }
            })
            .catch(() => {})
        : Promise.resolve();

      const checkOldLauncher = (typeof localStorage !== 'undefined' && !localStorage.getItem('wex_import_checked'))
        ? (async () => {
            localStorage.setItem('wex_import_checked', 'true');
            try {
              const detected = await invoke<string | null>('extras_detect_old_launcher');
              if (detected) {
                manualImportWizardOpen = true;
              }
            } catch {}
          })()
        : Promise.resolve();

      const fetchScreenshots = initialScreenshots === undefined
        ? invoke<any[]>('workshop_list_screenshots', { version: selectedVersion })
            .then((list) => {
              if (Array.isArray(list) && list.length > 0) {
                fetchedScreenshots = list;
              } else {
                fetchedScreenshots = [];
              }
            })
            .catch(() => {
              fetchedScreenshots = [];
            })
        : Promise.resolve();

      const fetchAchievements = initialAchievements === undefined
        ? invoke<any>('social_list_achievements')
            .then((res) => {
              if (res && res.achievements) {
                fetchedAchievements = res;
              }
            })
            .catch(() => {
              fetchedAchievements = null;
            })
        : Promise.resolve();

      const fetchPlaytime = initialPlaytime === undefined
        ? invoke<any>('overview_get_playtime')
            .then((res) => {
              if (res && (res.total_seconds > 0 || res.total_launches > 0)) {
                fetchedPlaytime = res;
              }
            })
            .catch(() => {
              fetchedPlaytime = null;
            })
        : Promise.resolve();

      await Promise.allSettled([
        fetchServers,
        fetchNews,
        fetchUpdates,
        checkOldLauncher,
        fetchScreenshots,
        fetchAchievements,
        fetchPlaytime,
        loadPlaytime(),
      ]);
    } catch {
      recentServer = null;
    }

    // Load latest screenshot from Workshop
    loadLatestScreenshot();
  });

  function handleImportCompleted(result: {
    nickname?: string;
    ramMb?: number;
    selectedVersion?: string;
    serversCount: number;
    skinsCount: number;
    worldsCount?: number;
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
          <p class="game-subheading">
            {t('play.client_title')} • {t('play.version_label')} {selectedVersion}
            {#if playtimeSummary && playtimeSummary.total_seconds > 0}
              <span class="header-playtime-meta">• {playtimeSummary.version_seconds > 0 ? (i18n.locale === 'en' ? `${playtimeSummary.version_formatted} on this version` : `${playtimeSummary.version_formatted} на этой версии`) : (i18n.locale === 'en' ? `${playtimeSummary.total_formatted} total` : `${playtimeSummary.total_formatted} всего`)}</span>
            {/if}
          </p>
        </div>
        <div class="header-tools">
          <Button
            variant="secondary"
            size="small"
            onclick={() => (manualImportWizardOpen = true)}
            title="Мастер импорта данных из старого лаунчера"
          >
            <Icon name="folder" size={14} color="var(--color-secondary-label)" />
            <span>{t('import.action_import')}</span>
          </Button>
        </div>
      </div>
    </header>

    <!-- Middle Shelf: Real Updates & Activity Glance -->
    <div class="dashboard-shelf">
      <!-- Stories Carousel: STRICTLY REAL DATA ONLY (Parity with StoriesCarousel.xaml) -->
      {#if storySlides.length > 0}
        <div class="stories-shelf-col updates-card">
          <StoriesCarousel
            slides={storySlides}
            {onnavigatescreen}
            {ontoast}
          />
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

      <!-- Recent Screenshot Glance (Parity Item 10) -->
      {#if latestScreenshot}
        <div class="glass-card screenshot-glance-card" data-testid="latest-screenshot-card">
          <div class="card-title-row">
            <Icon name="sun" size={15} color="var(--color-system-orange)" />
            <span class="card-title">Свежий скриншот</span>
            <span class="card-meta">
              {latestScreenshot.file_name}
            </span>
          </div>

          <button
            type="button"
            class="screenshot-glance-btn"
            onclick={() => {
              if (onopenworkshop) {
                onopenworkshop('screenshots');
              }
            }}
            title="Нажмите, чтобы открыть в Мастерской"
          >
            <div class="glance-img-container">
              <img
                src={getSafeAssetUrl(latestScreenshot.file_path) || ''}
                alt={latestScreenshot.file_name}
                class="glance-img"
                loading="lazy"
              />
              <div class="glance-hover-hint">
                <Icon name="folder" size={15} color="#ffffff" />
                <span>Открыть в Мастерской</span>
              </div>
            </div>
          </button>
        </div>
      {/if}
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
              {t('common.cancel')}
            </Button>
          </div>
        </div>
      {:else}
        <div class="dock-controls-row">
          <!-- Version Selector -->
          <div class="dock-block version-block">
            <span class="dock-caption">{t('play.version_label')}</span>
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
            <span class="dock-caption">{t('play.account_label')}</span>
            <div class="nickname-field-box">
              <Icon name="friends" size={15} color="var(--color-tertiary-label)" />
              <input
                type="text"
                bind:value={playerName}
                placeholder={t('play.nickname_placeholder')}
                class="player-name-input nickname-input"
                maxlength={16}
                disabled={isLaunching || isRunning}
                aria-label={t('play.nickname_placeholder')}
              />
            </div>
          </div>

          <div class="dock-vertical-divider"></div>

          <!-- Java Environment & Core Status -->
          <div class="dock-block tech-block">
            <span class="dock-caption">
              {isRunning ? (i18n.locale === 'en' ? 'Session Status' : 'Статус сессии') : (i18n.locale === 'en' ? 'Java Runtime' : 'Среда выполнения')}
            </span>
            <div class="tech-runtime-box">
              <span class="tech-runtime-text">{javaRuntime}</span>
              {#if isRunning}
                <span class="tech-status-badge running">• {i18n.locale === 'en' ? 'Playing' : 'В игре'}</span>
              {:else if clientStatus}
                <span class="tech-status-badge">• {clientStatus}</span>
              {/if}
            </div>
            {#if playtimeSummary && playtimeSummary.total_seconds > 0}
              <div class="tech-playtime-line" title="Суммарно: {playtimeSummary.total_formatted} • Запусков: {playtimeSummary.total_launches}">
                <Icon name="clock" size={11} color="var(--color-tertiary-label)" />
                <span class="tech-playtime-label">
                  {playtimeSummary.version_seconds > 0 ? `В игре: ${playtimeSummary.version_formatted}` : `В игре: ${playtimeSummary.total_formatted}`}
                </span>
              </div>
            {/if}
          </div>

          <!-- Action Button: Play (systemBlue) -> In Game / Stop (neutral) -->
          <div class="dock-primary-action dock-action" class:is-running={isRunning}>
            {#if isRunning}
              <Button
                variant="secondary"
                size="large"
                onclick={onstop}
                title={i18n.locale === 'en' ? 'Stop Minecraft' : 'Остановить процесс Minecraft'}
              >
                <span class="running-indicator-dot"></span>
                <span>{i18n.locale === 'en' ? 'Stop Minecraft' : 'В игре • Остановить'}</span>
              </Button>
            {:else}
              <Button
                variant="primary"
                size="large"
                onclick={onlaunch}
                disabled={!playerName.trim() || isLaunching || isRunning}
              >
                <Icon name="play" size={18} color="#ffffff" />
                <span>{t('play.launch')}</span>
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
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  }

  .game-subheading {
    font-size: 16px;
    color: var(--color-label);
    opacity: 0.94;
    margin: 0;
    font-weight: 600;
    text-shadow: 0 1px 8px rgba(0, 0, 0, 0.4);
  }

  :global([data-theme="light"]) .game-heading {
    text-shadow: 0 1px 8px rgba(255, 255, 255, 0.8);
  }

  :global([data-theme="light"]) .game-subheading {
    color: #1c1c1e;
    opacity: 1;
    text-shadow: 0 1px 8px rgba(255, 255, 255, 0.8);
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
    background: var(--color-system-ultra-thin-material, var(--card-bg));
    backdrop-filter: blur(28px) saturate(160%);
    -webkit-backdrop-filter: blur(28px) saturate(160%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    box-sizing: border-box;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.14);
  }

  .updates-card,
  .stories-shelf-col {
    flex: 1 1 380px;
    max-width: 520px;
    display: flex;
    flex-direction: column;
  }

  .status-card {
    flex: 0 1 300px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .screenshot-glance-card {
    flex: 0 1 240px;
    display: flex;
    flex-direction: column;
  }

  .screenshot-glance-btn {
    all: unset;
    cursor: pointer;
    border-radius: var(--radius-md);
    overflow: hidden;
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    display: block;
    transition: transform 0.15s ease, box-shadow 0.15s ease;
  }

  .screenshot-glance-btn:hover {
    transform: scale(1.02);
  }

  .screenshot-glance-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .glance-img-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--color-secondary-system-fill);
    border-radius: var(--radius-md);
  }

  .screenshot-glance-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .glance-hover-hint {
    position: absolute;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 500;
    color: #ffffff;
    opacity: 0;
    transition: opacity 0.15s ease;
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
  }

  .screenshot-glance-btn:hover .glance-hover-hint {
    opacity: 1;
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
    color: var(--color-label);
    opacity: 0.88;
    font-weight: 500;
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
    color: var(--color-label);
    opacity: 0.88;
  }

  .friends-status-label {
    font-size: var(--text-footnote);
    color: var(--color-label);
    opacity: 0.88;
    font-weight: 500;
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

  .tech-playtime-line {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--color-secondary-label);
    margin-top: -2px;
  }

  .tech-playtime-label {
    line-height: 1;
  }

  .header-playtime-meta {
    color: var(--color-secondary-label);
    font-weight: var(--font-weight-regular);
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

  :global([data-reduce-transparency="true"]) .launch-dock,
  :global([data-reduce-transparency="true"]) .glass-card {
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    background: var(--color-secondary-system-background) !important;
  }
</style>
