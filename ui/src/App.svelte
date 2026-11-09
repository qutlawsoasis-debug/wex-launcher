<!--
  Wex Launcher — Main App Shell
  Frameless window, custom titlebar, Liquid Glass sidebar, and active screen manager.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import './lib/design/tokens.css';
  import { theme } from './lib/design/theme.svelte';
  import { motionState, screenTransition } from './lib/motion';
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
  import type { CrashDiagnosisDto, DeepLinkActionDto } from './lib/ipc/generated/models';
  import type { DeepLinkAction } from './lib/components/DeepLinkConfirmationModal.svelte';
  import { initIdleManager, setGameRunning, createIdleInterval } from './lib/design/idleManager';
  import { friendsLive, parseWexJoinUri } from './lib/services/friendsLive';

  // Lazy screen loading
  let screenComponents = $state<Record<string, any>>({
    play: PlayScreen,
  });

  async function loadScreen(tab: string) {
    if (screenComponents[tab]) return screenComponents[tab];
    let comp: any = null;
    switch (tab) {
      case 'servers':
        comp = (await import('./lib/screens/ServersScreen.svelte')).default;
        break;
      case 'lobby':
        comp = (await import('./lib/screens/LobbyScreen.svelte')).default;
        break;
      case 'friends':
        comp = (await import('./lib/screens/FriendsScreen.svelte')).default;
        break;
      case 'wardrobe':
        comp = (await import('./lib/screens/WardrobeScreen.svelte')).default;
        break;
      case 'workshop':
        comp = (await import('./lib/screens/WorkshopScreen.svelte')).default;
        break;
      case 'mods':
        comp = (await import('./lib/screens/ModsScreen.svelte')).default;
        break;
      case 'settings':
        comp = (await import('./lib/screens/SettingsScreen.svelte')).default;
        break;
      case 'showcase':
        comp = (await import('./lib/screens/ShowcaseScreen.svelte')).default;
        break;
      case 'splash':
        comp = (await import('./lib/screens/SplashScreen.svelte')).default;
        break;
    }
    if (comp) {
      screenComponents[tab] = comp;
    }
    return comp;
  }

  // Lazy components
  let MusicDiskWidget = $state<any>(null);
  let CrashSheet = $state<any>(null);
  let OnboardingModal = $state<any>(null);
  let DeepLinkConfirmationModal = $state<any>(null);

  $effect(() => {
    if (activeTab && !screenComponents[activeTab]) {
      loadScreen(activeTab);
    }
    if (isStartingUp && !screenComponents.splash) {
      loadScreen('splash');
    }
    if (crashSheetOpen && !CrashSheet) {
      import('./lib/components/CrashSheet.svelte').then((m) => (CrashSheet = m.default));
    }
    if (showOnboarding && !OnboardingModal) {
      import('./lib/components/OnboardingModal.svelte').then((m) => (OnboardingModal = m.default));
    }
    if (deepLinkModalOpen && !DeepLinkConfirmationModal) {
      import('./lib/components/DeepLinkConfirmationModal.svelte').then((m) => (DeepLinkConfirmationModal = m.default));
    }
  });

  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    const tabParam = params.get('tab');
    if (tabParam && tabParam !== 'play') {
      loadScreen(tabParam);
    }
  }

  onMount(() => {
    import('./lib/components/MusicDiskWidget.svelte').then((m) => (MusicDiskWidget = m.default));
    if (typeof window !== 'undefined' && 'requestIdleCallback' in window) {
      window.requestIdleCallback(() => {
        loadScreen('servers');
        loadScreen('settings');
      });
    }
  });

  // Startup splash state
  let isStartingUp = $state(true);

  // Navigation state
  let activeTab = $state('play');
  let friendsSubtab = $state<'friends' | 'requests' | 'achievements'>('friends');
  let workshopSubtab = $state<'worlds' | 'resourcepacks' | 'shaders' | 'screenshots'>('worlds');
  let isMaximized = $state(false);
  let initialLobbyRoom = $state<any>(null);

  // Crash report modal state
  let crashSheetOpen = $state(false);
  let currentCrashDiagnosis = $state<CrashDiagnosisDto | null>(null);

  // Deep link modal state (Apple HIG Alert / SPEC 2.11)
  let deepLinkModalOpen = $state(false);
  let pendingDeepLink = $state<DeepLinkAction | DeepLinkActionDto | null>(null);

  function parseClientDeepLink(uri: string): DeepLinkAction {
    try {
      const url = new URL(uri.replace(/^wex:\/\//, 'https://wex/'));
      const path = url.pathname.replace(/^\//, '');
      const code = url.searchParams.get('code') || '';
      const address = url.searchParams.get('address') || '';

      if (path.includes('join') || path.includes('lobby') || code) {
        return {
          kind: 'join',
          title: `Вход в лобби ${code ? '#' + code : ''}`,
          subtitle: 'Игрок приглашает вас присоединиться к общему миру Minecraft.',
          target: code ? `#${code}` : 'Комната',
          raw_uri: uri,
          confirm_text: 'Присоединиться',
        };
      } else if (path.includes('server') || address) {
        return {
          kind: 'server',
          title: 'Подключение к серверу',
          subtitle: 'Добавить сервер в список избранных и запустить игру.',
          target: address || 'Сервер',
          raw_uri: uri,
          confirm_text: 'Подключиться',
        };
      }
    } catch {}
    return {
      kind: 'custom',
      title: 'Ссылка wex://',
      subtitle: 'Лаунчер получил запрос на выполнение внешнего действия.',
      target: uri,
      raw_uri: uri,
      confirm_text: 'Продолжить',
    };
  }

  async function handleRawDeepLink(uri: string) {
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core');
        const parsed = await invoke<any>('parse_deep_link_uri', { uri });
        if (parsed) {
          pendingDeepLink = parsed;
          deepLinkModalOpen = true;
          return;
        }
      }
    } catch {}
    pendingDeepLink = parseClientDeepLink(uri);
    deepLinkModalOpen = true;
  }

  async function handleConfirmDeepLink(action: DeepLinkAction | DeepLinkActionDto) {
    deepLinkModalOpen = false;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clear_pending_deep_link');
    } catch {}

    if (action.kind === 'join') {
      activeTab = 'lobby';
      addToast('info', 'Подключение к лобби', `Код: ${action.target}`);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('join_lobby_room', { code: action.target.toUpperCase(), playerName });
      } catch (e: any) {
        // Ignored in mock preview
      }
    } else if (action.kind === 'server') {
      activeTab = 'play';
      clientStatus = `Подключение к ${action.target}...`;
      addToast('info', 'Подключение к серверу', action.target);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('launch_game', {
          version: selectedVersion,
          nickname: playerName,
          server: action.target,
        });
        isGameRunning = true;
        clientStatus = 'Minecraft запущен';
      } catch (e: any) {
        addToast('error', 'Ошибка подключения к серверу', String(e));
      }
    } else if (action.kind === 'friend') {
      activeTab = 'friends';
      friendsSubtab = 'requests';
      addToast('info', 'Запрос в друзья', `Отправка запроса: ${action.target}`);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('social_send_friend_request', { friend_code: action.target });
        addToast('success', 'Запрос отправлен', `Запрос для ${action.target} успешно отправлен`);
      } catch (e: any) {
        addToast('error', 'Не удалось отправить запрос в друзья', String(e));
      }
    }
  }

  async function handleCancelDeepLink() {
    deepLinkModalOpen = false;
    pendingDeepLink = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('clear_pending_deep_link');
    } catch {}
  }

  onMount(() => {
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
  let showOnboarding = $state(false);

  $effect(() => {
    motionState.setGameRunning(isGameRunning);
  });

  // Dynamic background based on time of day (SPEC 1.1 / Island parity)
  let customBg = $state<string | null>(null);
  let currentHour = new Date().getHours();
  let bgImage = $derived.by(() => {
    if (customBg) return customBg;
    if (currentHour >= 22 || currentHour < 6) {
      return '/backgrounds/night.webp';
    } else if (currentHour >= 6 && currentHour < 12) {
      return '/backgrounds/morning.webp';
    } else if (currentHour >= 12 && currentHour < 18) {
      return '/backgrounds/day.webp';
    } else {
      return '/backgrounds/sunset.webp';
    }
  });

  // Toast stack
  let toasts = $state<ToastMessage[]>([]);

  function addToast(
    type: 'info' | 'success' | 'warning' | 'error',
    title: string,
    description?: string,
    action?: { label: string; onClick: () => void }
  ) {
    const id = `toast-${Date.now()}-${Math.random()}`;
    toasts = [...toasts, { id, type, title, description, duration: 5000, action }];
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

  // Keyboard Navigation & Shortcuts per wex-window SPEC:
  // - Esc: closes open modals and sheets
  // - Ctrl+1..7: switches active screen sections
  function handleWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (crashSheetOpen) {
        crashSheetOpen = false;
        e.preventDefault();
        return;
      }
      if (showImportWizard) {
        showImportWizard = false;
        e.preventDefault();
        return;
      }
      // Broadcast custom escape event for any active local sheets/modals
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('wex-escape'));
      }
    }

    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey) {
      const num = parseInt(e.key, 10);
      if (num >= 1 && num <= 7) {
        const item = sidebarItems[num - 1];
        if (item) {
          activeTab = item.id;
          e.preventDefault();
        }
      }
    }
  }

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
        { stage: 'Проверка клиента Wex', pct: 15, detail: 'Проверка целостности файлов игры', speed: '' },
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

      if (screen === 'splash') {
        isStartingUp = true;
      } else if (screen && ['play', 'servers', 'lobby', 'friends', 'wardrobe', 'workshop', 'mods', 'settings', 'showcase'].includes(screen)) {
        activeTab = screen;
        isStartingUp = false;
      }
      if (screen === 'import' || params.get('import') === 'true') {
        activeTab = 'play';
        showImportWizard = true;
      } else {
        showImportWizard = false;
      }
      if (screen === 'onboarding' || params.get('onboarding') === 'true') {
        showOnboarding = true;
      }
      if (t === 'light' || t === 'dark') {
        theme.setAppearance(t);
      }
      const bg = params.get('bg');
      if (bg === 'night' || bg === 'day' || bg === 'sunset' || bg === 'morning') {
        customBg = `/backgrounds/${bg}.webp`;
      } else if (bg && bg.endsWith('.jpg')) {
        customBg = bg.replace('.jpg', '.webp');
      } else if (bg === 'reset') {
        customBg = null;
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
      // Hash deeplink triggers for tests, protocol launches, and previews
      const deeplinkParam = params.get('deeplink') || params.get('uri');
      if (deeplinkParam) {
        handleRawDeepLink(deeplinkParam);
      }

      const subtabParam = params.get('subtab');
      if (subtabParam === 'friends' || subtabParam === 'requests' || subtabParam === 'achievements') {
        friendsSubtab = subtabParam;
      }

      const lobbyState = params.get('lobbyState');
      if (lobbyState === 'host') {
        initialLobbyRoom = {
          code: '#A7X9',
          host_name: 'Alex',
          world_name: 'Выживание в тайге',
          tunnel_address: 'tunnel.playit.gg:25565',
          tunnel_active: true,
          tunnel_error: null,
          is_host: true,
          players: [
            { name: 'Alex', is_host: true, ping_ms: 0, mods_synced: true },
            { name: 'Steve', is_host: false, ping_ms: 24, mods_synced: true },
            { name: 'Charlie', is_host: false, ping_ms: 42, mods_synced: true },
          ],
          mod_mismatch_count: 0,
          total_mods: 142,
        };
      } else if (lobbyState === 'tunnel_error') {
        initialLobbyRoom = {
          code: '#FAIL1',
          host_name: 'Alex',
          world_name: 'Мир Alex',
          tunnel_address: null,
          tunnel_active: false,
          tunnel_error: 'Не удалось поднять Playit-туннель: служба туннелей временно недоступна',
          is_host: true,
          players: [
            { name: 'Alex', is_host: true, ping_ms: 0, mods_synced: true },
          ],
          mod_mismatch_count: 0,
          total_mods: 142,
        };
      } else if (lobbyState === 'none' || lobbyState === 'reset') {
        initialLobbyRoom = null;
      }

      // Support wex://join and #join=ROOM123 deep links
      const joinCode = parseWexJoinUri(window.location.hash) || parseWexJoinUri(window.location.search);
      if (joinCode) {
        activeTab = 'lobby';
        void friendsLive.handleJoin(joinCode);
      }
    }

    // Check first-launch onboarding state
    if (typeof localStorage !== 'undefined' && localStorage.getItem('wex_onboarding_completed') !== 'true') {
      showOnboarding = true;
    }

    function handleShowOnboardingEvent() {
      showOnboarding = true;
    }
    if (typeof window !== 'undefined') {
      window.addEventListener('wex-show-onboarding', handleShowOnboardingEvent);
    }

    applyHashRoute();
    window.addEventListener('hashchange', applyHashRoute);

    // Real-time Friends Poller & Notifications
    friendsLive.start();
    const unlistenLiveNotifs = friendsLive.onNotification((notif) => {
      addToast('info', notif.title, notif.description, notif.action);
    });
    const unlistenLiveJoin = friendsLive.onJoin(() => {
      activeTab = 'lobby';
    });

    // Initial checks: parallelized network, profile, deep link and active running game state
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
      (async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const cfg = await invoke<{ friend_notifications?: boolean }>('get_launcher_settings');
          if (cfg && cfg.friend_notifications !== undefined) {
            friendsLive.setNotificationsEnabled(cfg.friend_notifications);
          }
          const pending = await invoke<DeepLinkActionDto | null>('get_pending_deep_link');
          if (pending) {
            pendingDeepLink = pending;
            deepLinkModalOpen = true;
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
    let unlistenDeepLink: (() => void) | undefined;
    let unlistenPluginDeepLink: (() => void) | undefined;

    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      import('@tauri-apps/api/event')
        .then(({ listen }) => {
          listen<CrashDiagnosisDto>('game-crash', (event) => {
            currentCrashDiagnosis = event.payload;
            crashSheetOpen = true;
          }).then((fn) => {
            unlistenCrash = fn;
          });
          listen<DeepLinkActionDto>('wex://deep-link', (event) => {
            if (event.payload) {
              pendingDeepLink = event.payload;
              deepLinkModalOpen = true;
            }
          }).then((fn) => {
            unlistenDeepLink = fn;
          });
          listen<string | string[]>('deep-link://new-url', async (event) => {
            const urls = Array.isArray(event.payload) ? event.payload : [event.payload];
            const firstUrl = urls.find((u) => typeof u === 'string' && (u.startsWith('wex://') || u.startsWith('aura://')));
            if (firstUrl) {
              await handleRawDeepLink(firstUrl);
            }
          }).then((fn) => {
            unlistenPluginDeepLink = fn;
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
        window.removeEventListener('wex-show-onboarding', handleShowOnboardingEvent);
      }
      cleanupNet();
      unlistenLiveNotifs();
      unlistenLiveJoin();
      friendsLive.stop();
      unlistenProgress?.();
      unlistenStatus?.();
      unlistenAchievement?.();
      unlistenInvite?.();
      unlistenCrash?.();
      unlistenDeepLink?.();
      unlistenPluginDeepLink?.();
    };
  });

  $effect(() => {
    friendsLive.setGameRunning(isGameRunning);
  });
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="wex-shell" class:game-active={isGameRunning} data-theme={theme.isDark ? 'dark' : 'light'}>
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
        {#if MusicDiskWidget}
          <MusicDiskWidget {isGameRunning} />
        {/if}
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
      {#key activeTab}
        <div class="screen-slide-wrapper" in:screenTransition>
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
            {#if screenComponents.servers}
              {@const ServersScreen = screenComponents.servers}
              <ServersScreen {selectedVersion} {playerName} />
            {/if}
          {:else if activeTab === 'lobby'}
            {#if screenComponents.lobby}
              {@const LobbyScreen = screenComponents.lobby}
              <LobbyScreen {playerName} initialRoom={initialLobbyRoom} />
            {/if}
          {:else if activeTab === 'friends'}
            {#if screenComponents.friends}
              {@const FriendsScreen = screenComponents.friends}
              <FriendsScreen tab={friendsSubtab} />
            {/if}
          {:else if activeTab === 'wardrobe'}
            {#if screenComponents.wardrobe}
              {@const WardrobeScreen = screenComponents.wardrobe}
              <WardrobeScreen {playerName} ontoast={addToast} />
            {/if}
          {:else if activeTab === 'workshop'}
            {#if screenComponents.workshop}
              {@const WorkshopScreen = screenComponents.workshop}
              <WorkshopScreen
                {selectedVersion}
                initialTab={workshopSubtab}
                ontoast={addToast}
                {playerName}
                onopenlobby={(room: any) => {
                  initialLobbyRoom = room;
                  activeTab = 'lobby';
                }}
              />
            {/if}
          {:else if activeTab === 'mods'}
            {#if screenComponents.mods}
              {@const ModsScreen = screenComponents.mods}
              <ModsScreen />
            {/if}
          {:else if activeTab === 'settings'}
            {#if screenComponents.settings}
              {@const SettingsScreen = screenComponents.settings}
              <SettingsScreen isGameRunning={isGameRunning} version={selectedVersion} onopenonboarding={() => { showOnboarding = true; }} />
            {/if}
          {:else if activeTab === 'showcase'}
            {#if screenComponents.showcase}
              {@const ShowcaseScreen = screenComponents.showcase}
              <ShowcaseScreen />
            {/if}
          {/if}
        </div>
      {/key}
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

  <!-- Startup Splash Screen per SPEC 1.1 -->
  {#if isStartingUp && screenComponents.splash}
    {@const SplashScreen = screenComponents.splash}
    <SplashScreen
      oncomplete={() => {
        isStartingUp = false;
      }}
    />
  {/if}

  <!-- Game Crash Diagnosis Sheet (SPEC 2.10) -->
  {#if crashSheetOpen && CrashSheet}
    <CrashSheet
      bind:open={crashSheetOpen}
      diagnosis={currentCrashDiagnosis}
      version={selectedVersion}
      onnavigate={(tab: string) => {
        activeTab = tab;
      }}
      ontoast={addToast}
    />
  {/if}

  <!-- First Launch Onboarding Modal (SPEC 1.1 / Apple HIG) -->
  {#if showOnboarding && OnboardingModal}
    <OnboardingModal
      bind:open={showOnboarding}
      oncomplete={(data: any) => {
        playerName = data.nickname;
        selectedVersion = data.version;
        loadProfile();
        addToast('success', 'Настройка завершена', `Добро пожаловать в Wex, ${data.nickname}!`);
        if (data.launchImmediately) {
          handleLaunch();
        }
      }}
    />
  {/if}

  <!-- Deep Link Confirmation Modal (SPEC 2.11 / Apple HIG Alert) -->
  {#if deepLinkModalOpen && DeepLinkConfirmationModal}
    <DeepLinkConfirmationModal
      bind:open={deepLinkModalOpen}
      action={pendingDeepLink}
      onconfirm={handleConfirmDeepLink}
      oncancel={handleCancelDeepLink}
    />
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
    z-index: 0;
    pointer-events: none;
    will-change: transform;
  }

  .scenery-vignette {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to bottom,
      rgba(0, 0, 0, 0.65) 0%,
      rgba(0, 0, 0, 0.50) 25%,
      rgba(0, 0, 0, 0.30) 50%,
      rgba(0, 0, 0, 0.65) 75%,
      rgba(0, 0, 0, 0.88) 100%
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

  .screen-slide-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
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

  /* Weak PC / Active Game Optimization: pause animations and disable backdrop filters to reduce CPU/GPU load to 0% */
  :global(.wex-shell.game-active *),
  :global(.wex-shell.game-active) {
    animation-play-state: paused !important;
  }

  :global(.wex-shell.game-active) [class*="glass"],
  :global(.wex-shell.game-active) .scenery-canvas,
  :global(.wex-shell.game-active) {
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
  }
</style>
