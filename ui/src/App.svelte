<!--
  Wex Launcher — Main App Shell
  Frameless window, custom titlebar, Liquid Glass sidebar, and active screen manager.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import './lib/design/tokens.css';
  import { theme } from './lib/design/theme.svelte';
  import { network } from './lib/design/network.svelte';
  import { formatError } from './lib/design/errors';
  import { i18n, t } from './lib/i18n';
  import Sidebar, { type SidebarItem } from './lib/components/Sidebar.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';
  import ProfileBadge from './lib/components/ProfileBadge.svelte';
  import Toast, { type ToastMessage } from './lib/components/Toast.svelte';
  import Button from './lib/components/Button.svelte';
  import Icon from './lib/components/icons/Icon.svelte';

  import PlayScreen from './lib/screens/PlayScreen.svelte';
  import ServersScreen from './lib/screens/ServersScreen.svelte';
  import LobbyScreen from './lib/screens/LobbyScreen.svelte';
  import FriendsScreen from './lib/screens/FriendsScreen.svelte';
  import WardrobeScreen from './lib/screens/WardrobeScreen.svelte';
  import ModsScreen from './lib/screens/ModsScreen.svelte';
  import WorkshopScreen from './lib/screens/WorkshopScreen.svelte';
  import SettingsScreen from './lib/screens/SettingsScreen.svelte';
  import ShowcaseScreen from './lib/screens/ShowcaseScreen.svelte';
  import CrashSheet from './lib/components/CrashSheet.svelte';
  import MusicDiskWidget from './lib/components/MusicDiskWidget.svelte';
  import type { CrashDiagnosisDto } from './lib/ipc/generated/models';
  import { initIdleManager, setGameRunning, createIdleInterval } from './lib/design/idleManager';

  // Navigation state
  let activeTab = $state('play');
  let friendsSubtab = $state<'friends' | 'requests' | 'achievements'>('friends');
  let workshopSubtab = $state<'worlds' | 'resourcepacks' | 'shaders' | 'screenshots'>('worlds');
  let isMaximized = $state(false);

  // Crash report modal state
  let crashSheetOpen = $state(false);
  let currentCrashDiagnosis = $state<CrashDiagnosisDto | null>(null);

  $effect(() => {
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      const tabParam = params.get('tab');
      if (tabParam && sidebarItems.some((i) => i.id === tabParam)) {
        activeTab = tabParam;
      }
      const subtabParam = params.get('subtab');
      if (subtabParam === 'friends' || subtabParam === 'requests' || subtabParam === 'achievements') {
        friendsSubtab = subtabParam;
      }
      const themeParam = params.get('theme');
      if (themeParam === 'light' || themeParam === 'dark') {
        theme.setAppearance(themeParam);
      }
      const langParam = params.get('lang');
      if (langParam === 'ru' || langParam === 'en') {
        i18n.setLocale(langParam);
      }
    }
  });

  // Launch state
  let selectedVersion = $state('1.21.1');
  let playerName = $state('Player');
  let clientStatus = $state('Клиент Wex готов');
  let isLaunching = $state(false);
  let isGameRunning = $state(false);
  $effect(() => {
    setGameRunning(isGameRunning);
  });
  let isOnline = $state(true);
  let skinHeadUrl = $state<string | undefined>(undefined);
  let progressPercentage = $state(0);
  let progressStage = $state('Готов к запуску');
  let progressDetail = $state('Нажмите «Играть» для старта');
  let progressSpeed = $state('');
  let launchError = $state<unknown>(null);
  let showImportWizard = $state(false);

  // Dynamic background based on theme and time of day
  let currentHour = new Date().getHours();
  let bgImage = $derived.by(() => {
    if (theme.isDark) {
      return currentHour >= 22 || currentHour < 5
        ? '/backgrounds/night.jpg'
        : '/backgrounds/sunset.jpg';
    } else {
      return currentHour >= 6 && currentHour < 12
        ? '/backgrounds/morning.jpg'
        : '/backgrounds/day.jpg';
    }
  });

  // Toast stack
  let toasts = $state<ToastMessage[]>([]);

  function addToast(type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) {
    const id = `toast-${Date.now()}-${Math.random()}`;
    toasts = [...toasts, { id, type, title, description, duration: 4000 }];
  }

  function dismissToast(id: string) {
    toasts = toasts.filter((t) => t.id !== id);
  }

  // Sidebar navigation destinations per SPEC.md 1.2
  let sidebarItems = $derived<SidebarItem[]>([
    { id: 'play', label: t('sidebar.play'), icon: 'play' },
    { id: 'servers', label: t('sidebar.servers'), icon: 'server' },
    { id: 'lobby', label: t('sidebar.lobby'), icon: 'lobby' },
    { id: 'friends', label: t('sidebar.friends'), icon: 'friends' },
    { id: 'wardrobe', label: t('sidebar.wardrobe'), icon: 'wardrobe' },
    { id: 'workshop', label: t('sidebar.workshop'), icon: 'workshop' },
    { id: 'mods', label: t('sidebar.mods'), icon: 'mods' },
    { id: 'settings', label: t('sidebar.settings'), icon: 'settings' },
  ]);

  let screenTitles = $derived<Record<string, string>>({
    play: t('sidebar.play'),
    servers: t('sidebar.servers'),
    lobby: t('sidebar.lobby'),
    friends: t('sidebar.friends'),
    wardrobe: t('sidebar.wardrobe'),
    workshop: t('sidebar.workshop'),
    mods: t('sidebar.mods'),
    settings: t('sidebar.settings'),
    showcase: 'Showcase',
  });

  // Tauri Window Control Actions
  async function minimizeWindow() {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    } catch {
      addToast('info', 'Окно свёрнуто (симуляция)');
    }
  }

  async function toggleMaximizeWindow() {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().toggleMaximize();
      isMaximized = !isMaximized;
    } catch {
      isMaximized = !isMaximized;
    }
  }

  async function closeWindow() {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch {
      addToast('info', 'Окно закрыто (симуляция)');
    }
  }

  // Real Network & Profile state management
  async function checkNetwork() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{ is_online: boolean; latency_ms?: number; status_text: string }>('shell_check_network');
      isOnline = res.is_online;
    } catch {
      isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true;
    }
  }

  let playerPlaytimeFormatted = $state<string | undefined>(undefined);

  async function loadProfile() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const prof = await invoke<{
        nickname: string;
        initials: string;
        is_online: boolean;
        skin_head_url?: string;
        total_playtime_seconds?: number;
        total_playtime_formatted?: string;
        total_launches?: number;
      }>('shell_get_profile', { nickname: playerName });
      if (prof.skin_head_url) {
        skinHeadUrl = prof.skin_head_url;
      }
      if (prof.total_playtime_formatted) {
        playerPlaytimeFormatted = prof.total_playtime_formatted;
      }
    } catch {
      skinHeadUrl = undefined;
    }
  }

  // Launch Game Action
  let cancelLaunchSimulation = false;

  async function handleLaunch() {
    if (isLaunching || isGameRunning) return;
    isLaunching = true;
    launchError = null;
    cancelLaunchSimulation = false;
    progressPercentage = 0;
    progressStage = 'Подготовка к запуску...';
    progressDetail = `Инициализация профиля ${selectedVersion}`;
    progressSpeed = '';

    try {
      // Try invoking Tauri 2 backend command
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<string>('launch_game', {
        version: selectedVersion,
        nickname: playerName,
      });
      isLaunching = false;
      isGameRunning = true;
      clientStatus = 'Minecraft запущен';
      addToast('success', 'Minecraft запущен', result);
    } catch (err: any) {
      const isTauri =
        typeof window !== 'undefined' &&
        ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
      if (isTauri) {
        isLaunching = false;
        launchError = err;
        const formatted = formatError(err, 'launch');
        addToast('error', formatted.title, formatted.description);
        return;
      }

      // Fallback in-browser progress simulation
      const steps = [
        { stage: 'Проверка клиента Wex', pct: 15, detail: 'Проверка целостности jar-файлов', speed: '' },
        { stage: 'Синхронизация сборки Wex', pct: 40, detail: 'Загрузка манифеста модов (142 мода)', speed: '14.8 МБ/с' },
        { stage: 'Проверка Java Runtime', pct: 75, detail: 'OpenJDK 21 HotSpot готов к старту', speed: '8.2 МБ/с' },
        { stage: 'Запуск Minecraft', pct: 100, detail: 'Инициализация графической сессии...', speed: '' },
      ];

      for (const step of steps) {
        if (cancelLaunchSimulation) {
          isLaunching = false;
          progressPercentage = 0;
          progressStage = 'Запуск отменён';
          progressDetail = 'Нажмите «Играть» для повторной попытки';
          progressSpeed = '';
          addToast('warning', 'Запуск остановлен', 'Процесс запуска был прерван пользователем.');
          return;
        }

        progressStage = step.stage;
        progressPercentage = step.pct;
        progressDetail = step.detail;
        progressSpeed = step.speed;
        await new Promise((r) => setTimeout(r, 400));
      }

      isLaunching = false;
      isGameRunning = true;
      progressStage = 'Minecraft запущен';
      progressDetail = `Сессия ${selectedVersion} активна`;
      progressSpeed = '';
      addToast('success', 'Игра запущена', `Minecraft ${selectedVersion} успешно запущен для ${playerName}!`);
    }
  }

  async function handleCancelLaunch() {
    cancelLaunchSimulation = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cancel_launch');
    } catch {
      // Handled by simulation flag
    }
  }

  async function handleStopGame() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('shell_stop_game', { version: selectedVersion });
      isGameRunning = false;
      isLaunching = false;
      clientStatus = 'Игра остановлена';
      addToast('info', 'Minecraft остановлен');
    } catch (e) {
      addToast('error', 'Ошибка остановки игры', String(e));
    }
  }

  onMount(() => {
    // Hash routing for screenshot automation: #screen=servers, #theme=light, #screen=showcase
    function applyHashRoute() {
      const hash = window.location.hash.replace(/^#/, '');
      if (!hash) return;
      const params = new URLSearchParams(hash);
      const screen = params.get('screen') || params.get('tab');
      const t = params.get('theme');

      if (screen && ['play', 'servers', 'lobby', 'friends', 'workshop', 'mods', 'settings', 'showcase'].includes(screen)) {
        activeTab = screen;
      }
      if (screen === 'import' || params.get('import') === 'true') {
        activeTab = 'play';
        showImportWizard = true;
      } else {
        showImportWizard = false;
      }
      if (t === 'light' || t === 'dark') {
        theme.setAppearance(t);
      }
      const l = params.get('lang');
      if (l === 'ru' || l === 'en') {
        i18n.setLocale(l);
      }
      const v = params.get('version');
      if (v === '1.20.1' || v === '1.21.1') {
        selectedVersion = v;
      }
      const player = params.get('player');
      if (player) {
        playerName = player;
      }
      const stageParam = params.get('stage');
      if (stageParam === 'installing') {
        isLaunching = true;
        isGameRunning = false;
        progressPercentage = Number(params.get('progress') || 45);
        progressStage = params.get('stageText') || 'Синхронизация сборки Wex';
        progressDetail = params.get('detail') || 'Загрузка манифеста модов (142 мода)';
        progressSpeed = params.get('speed') || '14.8 МБ/с';
      } else if (stageParam === 'running') {
        isLaunching = false;
        isGameRunning = true;
        addToast('success', 'Minecraft запущен', `Сессия ${selectedVersion} активна (PID 14820)`);
      } else if (stageParam === 'idle_restored') {
        isLaunching = false;
        isGameRunning = false;
        addToast('info', 'Сессия завершена', 'Minecraft закрыт штатно (код 0)');
      } else if (stageParam === 'idle') {
        isLaunching = false;
        isGameRunning = false;
      }

      // Hash crash triggers for tests and previews
      const crashParam = params.get('crash');
      if (crashParam === 'oom') {
        currentCrashDiagnosis = {
          kind: 'OutOfMemory',
          title: 'Нехватка оперативной памяти (ОЗУ)',
          user_advice: 'Увеличьте выделенный объём памяти в Настройках лаунчера (рекомендуется 6–8 ГБ) или закройте фоновые приложения.',
          technical_summary: 'Exception in thread "main" java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Main.main(Main.java:123)',
        };
        crashSheetOpen = true;
      } else if (crashParam === 'mod') {
        currentCrashDiagnosis = {
          kind: { MissingFabricDependency: { mod_name: 'iris', required_dep: 'sodium' } },
          title: 'Отсутствует зависимость для «iris»',
          user_advice: 'Мод «iris» требует обязательной установки «sodium». Установите недостающую библиотеку или удалите мод во вкладке Мастерская.',
          technical_summary: 'net.fabricmc.loader.impl.FormattedException: Mod \'iris\' requires \'sodium\'',
        };
        crashSheetOpen = true;
      } else if (crashParam === 'unknown') {
        currentCrashDiagnosis = {
          kind: { Unknown: { exit_code: -1 } },
          title: 'Игра завершилась с ошибкой (код -1)',
          user_advice: 'Причина не определена. Анализатор не нашёл известных сигнатур сбоя в логах игры.',
          technical_summary: 'Process finished with exit code -1',
        };
        crashSheetOpen = true;
      }
    }

    applyHashRoute();
    window.addEventListener('hashchange', applyHashRoute);

    // Initial checks: parallelized network, profile, and active running game state
    Promise.allSettled([
      checkNetwork(),
      loadProfile(),
      (async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const running = await invoke<boolean>('is_game_running');
          isGameRunning = running;
          if (running) {
            clientStatus = 'Minecraft запущен';
          }
        } catch {}
      })(),
    ]);

    const cleanupIdle = initIdleManager();

    if (typeof window !== 'undefined') {
      window.addEventListener('online', checkNetwork);
      window.addEventListener('offline', () => {
        isOnline = false;
      });
    }

    const cleanupNet = createIdleInterval(checkNetwork, 30000);

    // Tauri 2 Event Listeners (install-progress & launch-status as per SPEC.md)
    let unlistenProgress: (() => void) | undefined;
    let unlistenStatus: (() => void) | undefined;
    let unlistenAchievement: (() => void) | undefined;
    let unlistenInvite: (() => void) | undefined;
    let unlistenCrash: (() => void) | undefined;

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      import('@tauri-apps/api/event')
        .then(({ listen }) => {
          listen<CrashDiagnosisDto>('game-crash', (event) => {
            currentCrashDiagnosis = event.payload;
            crashSheetOpen = true;
          }).then((fn) => {
            unlistenCrash = fn;
          });
          listen<{
            stage: string;
            percentage: number;
            detail: string;
            speed_bytes_per_sec: number;
            total_bytes?: number;
            downloaded_bytes?: number;
          }>('install-progress', (event) => {
            const p = event.payload;
            progressStage = p.stage;
            progressPercentage = p.percentage;
            progressDetail = p.detail;
            progressSpeed = p.speed_bytes_per_sec > 0
              ? `${(p.speed_bytes_per_sec / (1024 * 1024)).toFixed(1)} МБ/с`
              : '';
          }).then((fn) => {
            unlistenProgress = fn;
          });

          listen<{
            state: string;
            message: string;
            exit_code?: number;
          }>('launch-status', (event) => {
            const s = event.payload;
            clientStatus = s.message;
            if (s.state === 'preparing') {
              progressStage = s.message;
            } else if (s.state === 'running') {
              isLaunching = false;
              isGameRunning = true;
              launchError = null;
              addToast('success', 'Minecraft запущен', s.message);
            } else if (s.state === 'cancelled') {
              isLaunching = false;
              isGameRunning = false;
              addToast('warning', 'Запуск отменён', s.message);
            } else if (s.state === 'error') {
              isLaunching = false;
              isGameRunning = false;
              launchError = s.message;
              const formatted = formatError(s.message, 'launch');
              addToast('error', formatted.title, formatted.description);
            } else if (s.state === 'idle') {
              isLaunching = false;
              isGameRunning = false;
              addToast('info', 'Сессия завершена', s.message);
            }
          }).then((fn) => {
            unlistenStatus = fn;
          });

          listen<{
            id: string;
            title: string;
            description: string;
            unlocked_at: string;
          }>('social-achievement-toast', (event) => {
            const a = event.payload;
            addToast('success', `Достижение разблокировано: «${a.title}»`, a.description);
          }).then((fn) => {
            unlistenAchievement = fn;
          });

          listen<{
            id: string;
            lobby_code: string;
            from_nick: string;
          }>('social-lobby-invite', (event) => {
            const inv = event.payload;
            addToast('info', `Приглашение в лобби ${inv.lobby_code}`, `${inv.from_nick} зовёт вас в игру`);
          }).then((fn) => {
            unlistenInvite = fn;
          });
        })
        .catch(() => {});
    }

    return () => {
      cleanupIdle?.();
      window.removeEventListener('hashchange', applyHashRoute);
      if (typeof window !== 'undefined') {
        window.removeEventListener('online', checkNetwork);
      }
      cleanupNet();
      unlistenProgress?.();
      unlistenStatus?.();
      unlistenAchievement?.();
      unlistenInvite?.();
      unlistenCrash?.();
    };
  });
</script>

<div class="wex-shell" data-theme={theme.isDark ? 'dark' : 'light'}>
  <!-- Real Game Scenery Backdrop under entire app shell -->
  <div class="scenery-canvas" style="background-image: url('{bgImage}');">
    <div class="scenery-vignette"></div>
  </div>

  <!-- Liquid Glass Sidebar -->
  <Sidebar
    items={sidebarItems}
    bind:activeId={activeTab}
  >
    {#snippet footer()}
      <div class="sidebar-footer-controls">
        <MusicDiskWidget {isGameRunning} />
        <div class="theme-quick-toggle">
          <Button
            variant="ghost"
            size="small"
            onclick={() => theme.setAppearance(theme.isDark ? 'light' : 'dark')}
            title="Переключить тему оформления"
          >
            <Icon name={theme.isDark ? 'sun' : 'moon'} size={15} color="var(--color-secondary-label)" />
            <span>{theme.isDark ? 'Светлая' : 'Тёмная'}</span>
          </Button>
        </div>
      </div>
    {/snippet}
  </Sidebar>

  <!-- Main View Area -->
  <main class="wex-main">
    <!-- Window Toolbar with Apple Profile Badge & Window Controls -->
    <Toolbar
      title={screenTitles[activeTab] || 'Wex'}
      {isMaximized}
      onminimize={minimizeWindow}
      onmaximize={toggleMaximizeWindow}
      onclose={closeWindow}
    >
      {#snippet trailing()}
        <ProfileBadge
          {playerName}
          isOnline={network.isOnline}
          {skinHeadUrl}
          playtimeFormatted={playerPlaytimeFormatted}
        />
      {/snippet}
    </Toolbar>

    <!-- Active Screen -->
    <div class="screen-viewport">
      {#if activeTab === 'play'}
        <PlayScreen
          bind:selectedVersion
          bind:playerName
          {clientStatus}
          {isLaunching}
          isRunning={isGameRunning}
          {progressPercentage}
          {progressStage}
          {progressDetail}
          {progressSpeed}
          bind:launchError
          initialShowImportWizard={showImportWizard}
          onlaunch={handleLaunch}
          oncancel={handleCancelLaunch}
          onstop={handleStopGame}
          onopensettings={() => {
            activeTab = 'settings';
          }}
          onnavigatescreen={(screen: string) => {
            activeTab = screen as any;
          }}
          ontoast={(t: { type: 'info' | 'success' | 'warning' | 'error'; title: string; message: string }) => addToast(t.type, t.title, t.message)}
          onopenworkshop={(subtab?: string) => {
            activeTab = 'workshop';
            if (subtab && ['worlds', 'resourcepacks', 'shaders', 'screenshots'].includes(subtab)) {
              workshopSubtab = subtab as any;
            }
          }}
        />
      {:else if activeTab === 'servers'}
        <ServersScreen />
      {:else if activeTab === 'lobby'}
        <LobbyScreen />
      {:else if activeTab === 'friends'}
        <FriendsScreen tab={friendsSubtab} />
      {:else if activeTab === 'wardrobe'}
        <WardrobeScreen {playerName} ontoast={addToast} />
      {:else if activeTab === 'workshop'}
        <WorkshopScreen {selectedVersion} initialTab={workshopSubtab} ontoast={addToast} />
      {:else if activeTab === 'mods'}
        <ModsScreen />
      {:else if activeTab === 'settings'}
        <SettingsScreen isGameRunning={isGameRunning} version={selectedVersion} />
      {:else if activeTab === 'showcase'}
        <ShowcaseScreen />
      {/if}
    </div>
  </main>

  <!-- Floating Toast Notifications -->
  {#if toasts.length > 0}
    <div class="toast-stack" aria-live="polite">
      {#each toasts as t (t.id)}
        <Toast toast={t} ondismiss={dismissToast} />
      {/each}
    </div>
  {/if}

  <!-- Game Crash Diagnosis Sheet (SPEC 2.10) -->
  <CrashSheet
    bind:open={crashSheetOpen}
    diagnosis={currentCrashDiagnosis}
    version={selectedVersion}
    onnavigate={(tab) => {
      activeTab = tab;
    }}
    ontoast={addToast}
  />
</div>

<style>
  .wex-shell {
    position: relative;
    display: flex;
    width: 100vw;
    height: 100vh;
    background-color: var(--color-system-background);
    color: var(--color-label);
    font-family: var(--font-system);
    overflow: hidden;
    box-sizing: border-box;
  }

  .scenery-canvas {
    position: absolute;
    inset: 0;
    background-size: cover;
    background-position: center 30%;
    z-index: 0;
    pointer-events: none;
    will-change: transform;
  }

  .scenery-vignette {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to bottom,
      rgba(0, 0, 0, 0.40) 0%,
      rgba(0, 0, 0, 0.15) 35%,
      rgba(0, 0, 0, 0.50) 75%,
      rgba(0, 0, 0, 0.82) 100%
    );
  }

  :global([data-theme='light']) .scenery-vignette {
    background: linear-gradient(
      to bottom,
      rgba(255, 255, 255, 0.65) 0%,
      rgba(255, 255, 255, 0.30) 35%,
      rgba(255, 255, 255, 0.65) 75%,
      rgba(255, 255, 255, 0.88) 100%
    );
  }

  .wex-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: hidden;
    background-color: transparent;
    box-sizing: border-box;
    z-index: 1;
  }

  .screen-viewport {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
    box-sizing: border-box;
    z-index: 1;
  }

  .sidebar-footer-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .theme-quick-toggle {
    display: flex;
    justify-content: center;
  }

  .toast-stack {
    position: fixed;
    top: 60px;
    right: 24px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 2000;
    pointer-events: auto;
  }
</style>
