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
  import { network } from '../design/network.svelte';
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
    initialShowImportWizard?: boolean;
    initialScreenshots?: Array<{ file_name: string; file_path: string; preview_data_url?: string; created_at?: string }>;
    initialAchievements?: { achievements: any[]; progress: any[] } | null;
    initialPlaytime?: { total_seconds: number; total_launches: number; formatted_play_time: string; last_played_utc?: string } | null;
    prewarmState?: 'idle' | 'prewarming' | 'ready' | 'running';
    prewarmProgress?: number;
    prewarmDetail?: string;
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
    progressStage = t('play.status_ready'),
    progressDetail = t('play.status_click_to_start'),
    progressSpeed = '',
    launchError = $bindable<unknown>(null),
    initialNews = undefined,
    initialShowImportWizard = false,
    initialScreenshots = undefined,
    initialAchievements = undefined,
    initialPlaytime = undefined,
    prewarmState = undefined,
    prewarmProgress = undefined,
    prewarmDetail = undefined,
    onlaunch,
    oncancel,
    onstop,
    onopensettings,
    onnavigatescreen,
    ontoast,
    onopenworkshop,
  }: Props = $props();

  // Fast-launch prewarm lifecycle states
  let currentPrewarmState = $state<'idle' | 'prewarming' | 'ready' | 'running'>('idle');
  let currentPrewarmProgress = $state<number>(0);
  let currentPrewarmDetail = $state<string>('');

  $effect(() => {
    if (prewarmState !== undefined) {
      currentPrewarmState = prewarmState;
    }
  });

  $effect(() => {
    if (prewarmProgress !== undefined) {
      currentPrewarmProgress = prewarmProgress;
    }
  });

  $effect(() => {
    if (prewarmDetail !== undefined) {
      currentPrewarmDetail = prewarmDetail;
    }
  });

  $effect(() => {
    if (isRunning) {
      currentPrewarmState = 'running';
    } else if (currentPrewarmState === 'running' && prewarmState !== 'running') {
      currentPrewarmState = 'idle';
    }
  });

  let effectiveState = $derived.by(() => {
    if (isRunning) return 'running';
    if (currentPrewarmState === 'running') return 'running';
    if (currentPrewarmState === 'prewarming') return 'prewarming';
    if (currentPrewarmState === 'ready') return 'ready';
    return 'idle';
  });

  let buttonTitle = $derived.by(() => {
    if (effectiveState === 'running') {
      return t('play.stop_minecraft_title');
    }
    if (effectiveState === 'prewarming') {
      return t('play.prewarming_title_progress', { pct: Math.round(currentPrewarmProgress) });
    }
    if (effectiveState === 'ready') {
      return t('play.ready_to_launch_prewarm');
    }
    return t('play.launch');
  });

  function handleActionClick() {
    if (effectiveState === 'running') {
      onstop?.();
    } else {
      onlaunch?.();
    }
  }

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
          badgeText: t('play.badge_new'),
          text: item.text.replace(/^\[(?:Новое|New)\]\s*/i, ''),
        };
      }
      if (cat === 'improved' || raw.startsWith('[Улучшено]')) {
        return {
          categoryClass: 'improved',
          badgeText: t('play.badge_improved'),
          text: item.text.replace(/^\[(?:Улучшено|Improved)\]\s*/i, ''),
        };
      }
      if (cat === 'fixed' || raw.startsWith('[Исправлено]')) {
        return {
          categoryClass: 'fixed',
          badgeText: t('play.badge_fixed'),
          text: item.text.replace(/^\[(?:Исправлено|Fixed)\]\s*/i, ''),
        };
      }
      return {
        categoryClass: 'general',
        badgeText: t('play.badge_info'),
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
    if (filePath.startsWith('data:') || filePath.startsWith('blob:') || filePath.startsWith('http:') || filePath.startsWith('https:')) return filePath;
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
        formattedPlayTime: playtimeData.formatted_play_time || t('play.minutes_short', { minutes: Math.floor(playtimeData.total_seconds / 60) }),
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

  onMount(() => {
    let unlistenPrewarm: (() => void) | undefined;

    function checkHash() {
      if (typeof window !== 'undefined' && window.location.hash) {
        const p = new URLSearchParams(window.location.hash.replace(/^#/, ''));
        const state = p.get('playState') || p.get('prewarmState') || p.get('stage');
        const progress = p.get('progress') || p.get('prewarmProgress');
        if (state === 'prewarming' || state === 'downloading') {
          currentPrewarmState = 'prewarming';
          currentPrewarmProgress = progress ? parseInt(progress, 10) : 48;
          currentPrewarmDetail = t('play.prewarm_sync_libraries');
        } else if (state === 'ready' || state === 'prewarmed') {
          currentPrewarmState = 'ready';
          currentPrewarmProgress = 100;
        } else if (state === 'running' || state === 'playing') {
          currentPrewarmState = 'running';
        } else if (state === 'idle' || state === 'reset' || state === 'exit' || state === 'idle_restored') {
          currentPrewarmState = 'idle';
          currentPrewarmProgress = 0;
        }
      }
    }

    checkHash();
    if (typeof window !== 'undefined') {
      window.addEventListener('hashchange', checkHash);
      (window as any).__setPlayFastLaunchState = (
        state: 'idle' | 'prewarming' | 'ready' | 'running',
        progress = 48,
        detail = ''
      ) => {
        currentPrewarmState = state;
        currentPrewarmProgress = progress;
        currentPrewarmDetail = detail;
      };
    }

    (async () => {
      // 1. Listen to updater download progress and prewarm status
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenPrewarm = await listen<any>('prewarm-status', (event) => {
          if (event.payload) {
            const s = event.payload.state;
            if (s === 'prewarming') {
              currentPrewarmState = 'prewarming';
              currentPrewarmProgress = event.payload.progress ?? 48;
              currentPrewarmDetail = event.payload.message ?? t('play.btn_prewarming');
            } else if (s === 'ready') {
              currentPrewarmState = 'ready';
              currentPrewarmProgress = 100;
              currentPrewarmDetail = event.payload.message ?? t('play.btn_ready_title');
            } else if (s === 'idle') {
              currentPrewarmState = 'idle';
              currentPrewarmProgress = 0;
            }
          }
        });
      } catch {}

      // 2. Parallelize all startup IPC calls using Promise.allSettled for fastest first-frame render
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
  })();

  return () => {
    if (unlistenPrewarm) unlistenPrewarm();
    if (typeof window !== 'undefined') {
      window.removeEventListener('hashchange', checkHash);
      delete (window as any).__setPlayFastLaunchState;
    }
  };
});

async function loadOnlineData() {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    if (initialNews === undefined) {
      try {
        const res = await invoke<any>('overview_get_news');
        if (Array.isArray(res) && res.length > 0) {
          fetchedNews = res;
        } else if (res?.versions && Array.isArray(res.versions) && res.versions.length > 0) {
          fetchedNews = res.versions;
        }
      } catch {}
    }
    try {
      const list = await invoke<any[]>('list_servers');
      if (list && list.length > 0) {
        recentServer = list[0];
      }
    } catch {}
  } catch {}
}

let prevOnline = network.isOnline;
$effect(() => {
  const currentOnline = network.isOnline;
  if (currentOnline && !prevOnline) {
    loadOnlineData();
  }
  prevOnline = currentOnline;
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
    <!-- Honest Offline State Banner -->
    {#if !network.isOnline}
      <div class="offline-banner" role="status" aria-live="polite">
        <div class="offline-banner-icon">
          <Icon name="wifi-off" size={16} color="var(--color-system-orange)" />
        </div>
        <div class="offline-banner-body">
          <span class="offline-banner-headline">
            {t('offline.title')}
          </span>
          <span class="offline-banner-subtext">
            {t('play.offline_launch_note')}
          </span>
        </div>
        <div class="offline-banner-action">
          <Button
            variant="glass"
            size="small"
            onclick={() => network.check()}
            title={t('mods.check_connection_title')}
          >
            <Icon name="refresh" size={12} color="rgba(255, 255, 255, 0.9)" />
            <span>{t('offline.retry')}</span>
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
              <span class="header-playtime-meta">• {playtimeSummary.version_seconds > 0 ? t('play.on_this_version', { time: playtimeSummary.version_formatted }) : t('play.total_time', { time: playtimeSummary.total_formatted })}</span>
            {/if}
          </p>
        </div>
        <div class="header-tools">
          <Button
            variant="glass"
            size="small"
            onclick={() => (manualImportWizardOpen = true)}
            title={t('play.import_wizard_title')}
          >
            <Icon name="folder" size={14} color="rgba(255, 255, 255, 0.90)" />
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
      {:else if !network.isOnline}
        <div class="stories-shelf-col updates-card offline-card">
          <div class="offline-card-inner">
            <Icon name="wifi-off" size={24} color="var(--color-secondary-label)" />
            <span class="offline-card-title">{t('offline.title')}</span>
            <span class="offline-card-desc">
              {t('play.offline_news_note')}
            </span>
          </div>
        </div>
      {/if}

      <!-- Recent Server & Friends Glance -->
      <div class="glass-card status-card">
        <div class="card-title-row">
          <Icon name="server" size={15} color="var(--color-secondary-label)" />
          <span class="card-title">{t('play.card_last_server')}</span>
        </div>

        {#if recentServer}
          <div class="recent-server-item">
            <span class="status-indicator" class:online={typeof recentServer.ping_ms === 'number' && recentServer.ping_ms >= 0}></span>
            <div class="server-text-wrap">
              <span class="server-display-name">{recentServer.name}</span>
              <span class="server-network-meta">
                {recentServer.address}{typeof recentServer.ping_ms === 'number' && recentServer.ping_ms >= 0 ? ` • ${recentServer.ping_ms} мс` : ''}
              </span>
            </div>
          </div>
        {:else}
          <div class="empty-status-text">{t('play.no_servers_played')}</div>
        {/if}
      </div>

      <!-- Recent Screenshot Glance (Parity Item 10) -->
      {#if latestScreenshot}
        <div class="glass-card screenshot-glance-card" data-testid="latest-screenshot-card">
          <div class="card-title-row">
            <Icon name="sun" size={15} color="var(--color-system-orange)" />
            <span class="card-title">{t('stories.latest_screenshot')}</span>
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
            title={t('workshop.click_to_open_workshop')}
            aria-label={t('workshop.latest_screenshot_aria')}
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
                <span>{t('workshop.open_in_workshop')}</span>
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
              {isRunning ? t('play.session_status') : t('play.runtime_env')}
            </span>
            <div class="tech-runtime-box">
              <span class="tech-runtime-text">{javaRuntime}</span>
              {#if isRunning}
                <span class="tech-status-badge running">({t('play.status_in_game')})</span>
              {:else if !network.isOnline}
                <span class="tech-status-badge offline">• {t('play.offline_launch_note')}</span>
              {:else if clientStatus}
                <span class="tech-status-badge">• {clientStatus}</span>
              {/if}
            </div>
            {#if playtimeSummary && playtimeSummary.total_seconds > 0}
              <div class="tech-playtime-line" title={t('play.playtime_tooltip', { total: playtimeSummary.total_formatted, launches: playtimeSummary.total_launches })}>
                <Icon name="clock" size={11} color="var(--color-tertiary-label)" />
                <span class="tech-playtime-label">
                  {playtimeSummary.version_seconds > 0 ? t('play.in_game_label', { time: playtimeSummary.version_formatted }) : t('play.in_game_label', { time: playtimeSummary.total_formatted })}
                </span>
              </div>
            {/if}
          </div>

          <!-- Action Button: Play / Prewarming / Ready / In Game -->
          <div
            class="dock-primary-action dock-action"
            class:is-running={effectiveState === 'running'}
            class:is-ready={effectiveState === 'ready'}
            class:is-prewarming={effectiveState === 'prewarming'}
          >
            <button
              type="button"
              class="play-morph-btn {effectiveState} {effectiveState === 'running' ? 'secondary' : 'primary'}"
              data-testid="play-btn-{effectiveState}"
              disabled={(!playerName.trim() && effectiveState === 'idle') || (isLaunching && effectiveState !== 'prewarming')}
              onclick={handleActionClick}
              title={buttonTitle}
            >
              {#if effectiveState === 'prewarming'}
                <!-- Honest progress fill layer inside button -->
                <div
                  class="prewarm-progress-fill"
                  style="width: {Math.max(0, Math.min(100, currentPrewarmProgress))}%;"
                  data-testid="prewarm-progress-fill"
                ></div>
                <div class="prewarm-btn-content">
                  <div class="prewarm-spinner" aria-hidden="true"></div>
                  <span class="prewarm-btn-title">{t('play.btn_prewarming')}</span>
                  <span class="prewarm-btn-pct">{Math.round(currentPrewarmProgress)}%</span>
                </div>
              {:else if effectiveState === 'ready'}
                <div class="ready-btn-content">
                  <Icon name="play" size={17} color="#ffffff" />
                  <div class="ready-btn-text">
                    <span class="ready-title">{t('play.btn_ready_title')}</span>
                    <span class="ready-sub">{t('play.btn_ready_sub')}</span>
                  </div>
                </div>
              {:else if effectiveState === 'running'}
                <div class="running-btn-content">
                  <span class="running-icon-stop" aria-hidden="true"></span>
                  <span class="running-title">{t('play.btn_playing_stop')}</span>
                </div>
              {:else}
                <!-- Idle State: Play -->
                <div class="idle-btn-content">
                  <Icon name="play" size={18} color="#ffffff" />
                  <span class="idle-title">{t('play.launch')}</span>
                </div>
              {/if}
            </button>
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

  .offline-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: var(--color-system-thin-material);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(255, 159, 10, 0.3);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    margin-bottom: 12px;
    animation: fadeIn 0.3s ease;
  }

  .offline-banner-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(255, 159, 10, 0.15);
    flex-shrink: 0;
  }

  .offline-banner-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .offline-banner-headline {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-label);
  }

  .offline-banner-subtext {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  .offline-banner-action {
    flex-shrink: 0;
  }

  .offline-card {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px 20px;
    text-align: center;
  }

  .offline-card-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .offline-card-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--color-label);
  }

  .offline-card-desc {
    font-size: 12px;
    color: var(--color-secondary-label);
    max-width: 280px;
    line-height: 1.4;
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
    color: rgba(255, 255, 255, 1.0);
    text-shadow: 0 2px 14px rgba(0, 0, 0, 0.75), 0 1px 3px rgba(0, 0, 0, 0.90);
  }

  .game-subheading {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.88);
    margin: 0;
    font-weight: 600;
    text-shadow: 0 1px 8px rgba(0, 0, 0, 0.75), 0 1px 3px rgba(0, 0, 0, 0.90);
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
    transition:
      transform var(--motion-duration-normal) var(--motion-curve-spring-bouncy),
      box-shadow var(--motion-duration-hover) ease,
      background-color var(--motion-duration-hover) ease;
  }

  .glass-card:hover {
    transform: translateY(var(--motion-card-hover-y, -2px));
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.18);
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
    justify-content: flex-start;
    align-self: flex-start;
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

  .glance-img {
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

  /* Green only when a ping actually answered; neutral otherwise. */
  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-tertiary-label);
    flex-shrink: 0;
  }

  .status-indicator.online {
    background-color: var(--color-system-green);
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
    flex: 0 1 200px;
    min-width: 120px;
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
    min-width: 0;
  }

  /* The status text is the only part of the dock that may shrink, so the
     Play button always stays inside the dock at the minimum window width. */
  .tech-block {
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
  }

  .tech-runtime-box {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 34px;
    white-space: nowrap;
    min-width: 0;
  }

  .tech-runtime-box .tech-status-badge {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tech-runtime-text {
    flex-shrink: 0;
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

  .dock-primary-action {
    margin-left: auto;
    flex-shrink: 0;
    transition:
      transform var(--motion-duration-morph) var(--motion-curve-spring-bouncy),
      opacity var(--motion-duration-screen-enter) ease;
  }

  .dock-controls-row {
    display: flex;
    align-items: center;
    width: 100%;
    gap: 16px;
    animation: dockRowIn var(--motion-duration-morph) var(--motion-curve-spring-bouncy) forwards;
  }

  .dock-progress-mode {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    animation: dockProgressIn var(--motion-duration-morph) var(--motion-curve-spring-bouncy) forwards;
  }

  @keyframes dockRowIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes dockProgressIn {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  :global([data-effects="off"]) .dock-controls-row,
  :global([data-effects="off"]) .dock-progress-mode,
  :global([data-reduce-motion="true"]) .dock-controls-row,
  :global([data-reduce-motion="true"]) .dock-progress-mode {
    animation: none;
    transform: none !important;
  }

  .dock-progress-mode :global(.progress-bar-wrapper) {
    flex: 1;
  }

  .dock-cancel-action {
    flex-shrink: 0;
  }

  /* ========================================================================
     Action Button: Morphing States (Idle -> Prewarming -> Ready -> Running)
     Apple HIG Kinetics & Spring Specifications
     ======================================================================== */
  .play-morph-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 160px;
    height: 44px;
    padding: 0 20px;
    border-radius: 12px;
    font-family: var(--font-system);
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0;
    text-decoration: none;
    cursor: pointer;
    user-select: none;
    overflow: hidden;
    border: 1px solid transparent;
    transition:
      transform var(--motion-duration-press, 120ms) var(--motion-curve-spring-system, cubic-bezier(0.16, 1, 0.3, 1)),
      background-color var(--motion-duration-morph, 300ms) var(--motion-curve-spring-bouncy, cubic-bezier(0.2, 0.9, 0.3, 1.05)),
      border-color var(--motion-duration-morph, 300ms) var(--motion-curve-spring-bouncy, cubic-bezier(0.2, 0.9, 0.3, 1.05)),
      color var(--motion-duration-short, 150ms) ease,
      min-width var(--motion-duration-morph, 300ms) var(--motion-curve-spring-bouncy, cubic-bezier(0.2, 0.9, 0.3, 1.05)),
      box-shadow var(--motion-duration-hover, 160ms) ease;
    outline: none;
  }

  .play-morph-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .play-morph-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 2px;
  }

  .play-morph-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    transform: none !important;
  }

  /* Idle State: Standard Prominent System Blue */
  .play-morph-btn.idle {
    background-color: var(--color-system-blue);
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(0, 122, 255, 0.28);
  }

  .play-morph-btn.idle:hover:not(:disabled) {
    filter: brightness(1.06);
    box-shadow: 0 4px 12px rgba(0, 122, 255, 0.36);
  }

  .idle-btn-content {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
  }

  /* Prewarming State: Honest progress bar directly inside the button */
  .play-morph-btn.prewarming {
    min-width: 220px;
    background-color: rgba(0, 122, 255, 0.12);
    border-color: rgba(0, 122, 255, 0.38);
    color: var(--color-label);
    padding: 0 16px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .prewarm-progress-fill {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    background: rgba(0, 122, 255, 0.28);
    border-radius: inherit;
    transition: width 0.28s cubic-bezier(0.16, 1, 0.3, 1);
    pointer-events: none;
    z-index: 1;
  }

  .prewarm-btn-content {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    justify-content: space-between;
  }

  .prewarm-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(0, 122, 255, 0.3);
    border-top-color: var(--color-system-blue);
    border-radius: 50%;
    animation: prewarmSpin 0.75s linear infinite;
    flex-shrink: 0;
  }

  @keyframes prewarmSpin {
    to {
      transform: rotate(360deg);
    }
  }

  .prewarm-btn-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-label);
    white-space: nowrap;
  }

  .prewarm-btn-pct {
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--color-system-blue);
    padding: 2px 6px;
    background: rgba(0, 122, 255, 0.16);
    border-radius: 6px;
    margin-left: auto;
  }

  /* Ready State: Prewarm ready, instant start */
  .play-morph-btn.ready {
    min-width: 195px;
    background-color: var(--color-system-blue);
    color: #ffffff;
    box-shadow: 0 2px 10px rgba(0, 122, 255, 0.34);
  }

  .play-morph-btn.ready:hover:not(:disabled) {
    filter: brightness(1.06);
    box-shadow: 0 4px 14px rgba(0, 122, 255, 0.44);
  }

  .ready-btn-content {
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }

  .ready-btn-text {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  .ready-title {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0;
  }

  .ready-sub {
    font-size: 11px;
    font-weight: 400;
    opacity: 0.88;
    letter-spacing: 0;
  }

  /* Running State: Neutral glass stop button */
  .play-morph-btn.running {
    min-width: 185px;
    background-color: var(--color-fill-secondary);
    border-color: var(--color-separator);
    color: var(--color-label);
    backdrop-filter: blur(8px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  }

  .play-morph-btn.running:hover:not(:disabled) {
    background-color: var(--color-fill-tertiary);
    border-color: rgba(255, 59, 48, 0.4);
    color: var(--color-system-red);
  }

  .running-btn-content {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
  }

  .running-icon-stop {
    width: 10px;
    height: 10px;
    background-color: currentColor;
    border-radius: 2px;
    flex-shrink: 0;
  }

  :global([data-effects="off"]) .play-morph-btn,
  :global([data-reduce-motion="true"]) .play-morph-btn,
  :global([data-effects="off"]) .prewarm-progress-fill,
  :global([data-reduce-motion="true"]) .prewarm-progress-fill {
    transition: opacity 150ms ease !important;
    animation: none !important;
    transform: none !important;
  }

  :global([data-reduce-transparency="true"]) .launch-dock,
  :global([data-reduce-transparency="true"]) .glass-card {
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    background: var(--color-secondary-system-background) !important;
  }
</style>
