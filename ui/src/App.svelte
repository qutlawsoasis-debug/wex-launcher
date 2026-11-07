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
  import FriendsScreen from './lib/screens/FriendsScreen.svelte';
  import ModsScreen from './lib/screens/ModsScreen.svelte';
  import SettingsScreen from './lib/screens/SettingsScreen.svelte';
  import ShowcaseScreen from './lib/screens/ShowcaseScreen.svelte';

  // Navigation state
  let activeTab = $state('play');
  let isMaximized = $state(false);

  // Launch state
  let selectedVersion = $state('1.21.1');
  let playerName = $state('Player');
  let isLaunching = $state(false);
  let progressPercentage = $state(0);
  let progressStage = $state('Готов к запуску');
  let progressDetail = $state('Нажмите «Играть» для старта');
  let progressSpeed = $state('');

  // Toast stack
  let toasts = $state<ToastMessage[]>([]);

  function addToast(type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) {
    const id = `toast-${Date.now()}-${Math.random()}`;
    toasts = [...toasts, { id, type, title, description, duration: 4000 }];
  }

  function dismissToast(id: string) {
    toasts = toasts.filter((t) => t.id !== id);
  }

  const sidebarItems: SidebarItem[] = [
    { id: 'play', label: 'Играть', icon: 'play' },
    { id: 'servers', label: 'Серверы', icon: 'server', badge: 3 },
    { id: 'friends', label: 'Друзья', icon: 'friends', badge: 2 },
    { id: 'mods', label: 'Моды', icon: 'mods' },
    { id: 'settings', label: 'Настройки', icon: 'settings' },
  ];

  const screenTitles: Record<string, string> = {
    play: 'Играть',
    servers: 'Серверы',
    friends: 'Друзья',
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
      // Browser preview mode fallback
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
    if (isLaunching) return;
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
        { stage: 'Проверка клиента Aura', pct: 15, detail: 'Проверка целостности jar-файлов', speed: '24 МБ/с' },
        { stage: 'Синхронизация Aura-Pack', pct: 45, detail: 'Загрузка модов и шейдеров', speed: '18 МБ/с' },
        { stage: 'Проверка Java 21', pct: 75, detail: 'Инициализация HotSpot VM', speed: '8 МБ/с' },
        { stage: 'Запуск Minecraft', pct: 100, detail: 'Fabric Loader стартовал', speed: '' },
      ];

      for (const step of steps) {
        if (cancelLaunchSimulation) {
          isLaunching = false;
          progressPercentage = 0;
          progressStage = 'Отменено';
          progressDetail = 'Запуск остановлен пользователем';
          addToast('warning', 'Запуск отменён');
          return;
        }

        progressStage = step.stage;
        progressPercentage = step.pct;
        progressDetail = `${step.detail} (${playerName} / ${selectedVersion})`;
        progressSpeed = step.speed;
        await new Promise((r) => setTimeout(r, 400));
      }

      isLaunching = false;
      addToast('success', 'Minecraft запущен успешно', `Версия ${selectedVersion} работает в фоне`);
    }
  }

  function handleCancelLaunch() {
    cancelLaunchSimulation = true;
    try {
      import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke('cancel_launch').catch(() => {});
      });
    } catch {
      // noop
    }
  }

  onMount(() => {
    theme.applyDom();

    function applyHashParams() {
      if (typeof window === 'undefined') return;
      const hash = window.location.hash.startsWith('#')
        ? window.location.hash.slice(1)
        : window.location.hash;
      if (!hash) return;
      const params = new URLSearchParams(hash);
      const tab = params.get('tab');
      if (tab && screenTitles[tab]) {
        activeTab = tab;
      }
      const th = params.get('theme');
      if (th === 'dark' || th === 'light') {
        theme.setAppearance(th);
      }
    }

    window.addEventListener('hashchange', applyHashParams);
    applyHashParams();

    // Listen to Tauri progress events if running inside Tauri
    let unlisten: (() => void) | undefined;
    import('@tauri-apps/api/event')
      .then(({ listen }) => {
        listen<{ stage: string; percentage: number; detail: string; speed_bytes_per_sec: number }>(
          'launch-progress',
          (event) => {
            const p = event.payload;
            progressStage = p.stage;
            progressPercentage = p.percentage;
            progressDetail = p.detail;
            progressSpeed = p.speed_bytes_per_sec > 0
              ? `${(p.speed_bytes_per_sec / (1024 * 1024)).toFixed(1)} МБ/с`
              : '';
          }
        ).then((fn) => {
          unlisten = fn;
        });
      })
      .catch(() => {});

    return () => {
      unlisten?.();
    };
  });
</script>

<div class="wex-shell" data-theme={theme.isDark ? 'dark' : 'light'}>
  <!-- Liquid Glass Sidebar -->
  <Sidebar
    items={sidebarItems}
    bind:activeId={activeTab}
  >
    {#snippet footer()}
      <div class="sidebar-footer-controls">
        <button
          type="button"
          class="showcase-nav-btn"
          class:active={activeTab === 'showcase'}
          onclick={() => (activeTab = 'showcase')}
        >
          <Icon name="cpu" size={16} color="var(--color-secondary-label)" />
          <span>Компоненты HIG</span>
        </button>

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
          {isLaunching}
          {progressPercentage}
          {progressStage}
          {progressDetail}
          {progressSpeed}
          onlaunch={handleLaunch}
          oncancel={handleCancelLaunch}
        />
      {:else if activeTab === 'servers'}
        <ServersScreen />
      {:else if activeTab === 'friends'}
        <FriendsScreen />
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
  }

  .wex-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: hidden;
    background-color: var(--color-system-background);
  }

  .screen-viewport {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  .sidebar-footer-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .showcase-nav-btn {
    width: 100%;
    height: 32px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    border-radius: var(--radius-sm);
    background-color: transparent;
    cursor: pointer;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    transition: background-color var(--motion-duration-short) ease;
  }

  .showcase-nav-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .showcase-nav-btn.active {
    background-color: var(--color-system-fill);
    color: var(--color-system-blue);
    font-weight: 600;
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
