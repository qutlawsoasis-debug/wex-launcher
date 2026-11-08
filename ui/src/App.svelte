<!--
  Wex Launcher — Main App Shell
  Frameless window, custom titlebar, Liquid Glass sidebar, and active screen manager.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import './lib/design/tokens.css';
  import { theme } from './lib/design/theme.svelte';
  import Sidebar, { type SidebarItem } from './lib/components/Sidebar.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';
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

  // Navigation state
  let activeTab = $state('play');
  let isMaximized = $state(false);

  // Launch state
  let selectedVersion = $state('1.21.1');
  let playerName = $state('Player');
  let clientStatus = $state('Клиент Wex готов');
  let isLaunching = $state(false);
  let isGameRunning = $state(false);
  let progressPercentage = $state(0);
  let progressStage = $state('Готов к запуску');
  let progressDetail = $state('Нажмите «Играть» для старта');
  let progressSpeed = $state('');

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
  const sidebarItems: SidebarItem[] = [
    { id: 'play', label: 'Играть', icon: 'play' },
    { id: 'servers', label: 'Серверы', icon: 'server' },
    { id: 'lobby', label: 'Лобби', icon: 'lobby' },
    { id: 'friends', label: 'Друзья', icon: 'friends' },
    { id: 'wardrobe', label: 'Гардероб', icon: 'wardrobe' },
    { id: 'workshop', label: 'Мастерская', icon: 'workshop' },
    { id: 'mods', label: 'Моды', icon: 'mods' },
    { id: 'settings', label: 'Настройки', icon: 'settings' },
  ];

  const screenTitles: Record<string, string> = {
    play: 'Играть',
    servers: 'Серверы',
    lobby: 'Кооператив и Лобби',
    friends: 'Друзья',
    wardrobe: 'Гардероб',
    workshop: 'Мастерская',
    mods: 'Моды и шейдеры',
    settings: 'Настройки',
    showcase: 'Компоненты Apple HIG',
  };

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

  // Launch Game Action
  let cancelLaunchSimulation = false;

  async function handleLaunch() {
    if (isLaunching || isGameRunning) return;
    isLaunching = true;
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
      addToast('success', 'Minecraft запущен', result);
    } catch {
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

  onMount(() => {
    // Hash routing for screenshot automation: #screen=servers, #theme=light, #screen=showcase
    function applyHashRoute() {
      const hash = window.location.hash.replace(/^#/, '');
      if (!hash) return;
      const params = new URLSearchParams(hash);
      const screen = params.get('screen') || params.get('tab');
      const t = params.get('theme');

      if (screen && ['play', 'servers', 'lobby', 'friends', 'mods', 'settings', 'showcase'].includes(screen)) {
        activeTab = screen;
      }
      if (t === 'light' || t === 'dark') {
        theme.setAppearance(t);
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
    }

    applyHashRoute();
    window.addEventListener('hashchange', applyHashRoute);

    // Tauri 2 Event Listeners (install-progress & launch-status as per SPEC.md)
    let unlistenProgress: (() => void) | undefined;
    let unlistenStatus: (() => void) | undefined;
    let unlistenAchievement: (() => void) | undefined;
    let unlistenInvite: (() => void) | undefined;

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      import('@tauri-apps/api/event')
        .then(({ listen }) => {
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
              addToast('success', 'Minecraft запущен', s.message);
            } else if (s.state === 'cancelled') {
              isLaunching = false;
              isGameRunning = false;
              addToast('warning', 'Запуск отменён', s.message);
            } else if (s.state === 'error') {
              isLaunching = false;
              isGameRunning = false;
              addToast('error', 'Ошибка запуска', s.message);
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
      window.removeEventListener('hashchange', applyHashRoute);
      unlistenProgress?.();
      unlistenStatus?.();
      unlistenAchievement?.();
      unlistenInvite?.();
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
    <!-- Window Toolbar -->
    <Toolbar
      title={screenTitles[activeTab] || 'Wex'}
      {isMaximized}
      onminimize={minimizeWindow}
      onmaximize={toggleMaximizeWindow}
      onclose={closeWindow}
    />

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
          onlaunch={handleLaunch}
          oncancel={handleCancelLaunch}
        />
      {:else if activeTab === 'servers'}
        <ServersScreen />
      {:else if activeTab === 'lobby'}
        <LobbyScreen />
      {:else if activeTab === 'friends'}
        <FriendsScreen />
      {:else if activeTab === 'wardrobe'}
        <WardrobeScreen {playerName} ontoast={addToast} />
      {:else if activeTab === 'workshop'}
        <WorkshopScreen {selectedVersion} ontoast={addToast} />
      {:else if activeTab === 'mods'}
        <ModsScreen />
      {:else if activeTab === 'settings'}
        <SettingsScreen />
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
    transition: background-image 0.5s ease;
    z-index: 0;
    pointer-events: none;
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
