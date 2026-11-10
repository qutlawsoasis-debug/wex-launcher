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
  import UpdateCard from './lib/components/UpdateCard.svelte';
  import Toolbar from './lib/components/Toolbar.svelte';
  import ProfileBadge from './lib/components/ProfileBadge.svelte';
  import Toast, { type ToastMessage } from './lib/components/Toast.svelte';
  import Button from './lib/components/Button.svelte';
  import Icon from './lib/components/icons/Icon.svelte';

  import PlayScreen from './lib/screens/PlayScreen.svelte';
  import type { LobbyRoom } from './lib/screens/LobbyScreen.svelte';
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
  let initialLobbyRoom = $state<LobbyRoom | null>(null);

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
          title: t('deeplink.title_lobby', { code: code ? '#' + code : '' }),
          subtitle: t('deeplink.subtitle_lobby'),
          target: code ? `#${code}` : t('deeplink.target_lobby_default'),
          raw_uri: uri,
          confirm_text: t('deeplink.action_join'),
        };
      } else if (path.includes('server') || address) {
        return {
          kind: 'server',
          title: t('deeplink.title_server'),
          subtitle: t('deeplink.subtitle_server'),
          target: address || t('deeplink.target_server_default'),
          raw_uri: uri,
          confirm_text: t('deeplink.action_connect'),
        };
      }
    } catch {}
    return {
      kind: 'custom',
      title: t('deeplink.title_generic'),
      subtitle: t('deeplink.subtitle_generic'),
      target: uri,
      raw_uri: uri,
      confirm_text: t('deeplink.action_proceed'),
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
      try {
        await friendsLive.handleJoin(action.target);
      } catch (error) {
        const formatted = formatError(error, 'lobby');
        addToast('error', formatted.title, formatted.description);
      }
    } else if (action.kind === 'server') {
      activeTab = 'play';
      clientStatus = t('app.connecting_to_server', { target: action.target });
      addToast('info', t('app.toast_server_connecting'), action.target);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('launch_game', {
          version: selectedVersion,
          nickname: playerName,
          server: action.target,
        });
        isGameRunning = true;
        clientStatus = t('app.minecraft_running');
      } catch (e: any) {
        addToast('error', t('app.err_server_connect'), String(e));
      }
    } else if (action.kind === 'friend') {
      activeTab = 'friends';
      friendsSubtab = 'requests';
      addToast('info', t('app.toast_friend_request'), t('app.toast_friend_sending', { target: action.target }));
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('social_send_friend_request', { targetFriendCode: action.target });
        addToast('success', t('app.toast_request_sent'), t('app.toast_request_sent_desc', { target: action.target }));
      } catch (e: any) {
        addToast('error', t('app.err_friend_request'), String(e));
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
  let clientStatus = $state(t('play.status_client_ready'));
  let isLaunching = $state(false);
  let isGameRunning = $state(false);
  let playFastLaunchState = $state<'idle' | 'prewarming' | 'ready' | 'running'>('idle');
  let playFastLaunchProgress = $state<number>(0);
  $effect(() => {
    setGameRunning(isGameRunning);
  });
  let isOnline = $state(true);
  let skinHeadUrl = $state<string | undefined>(undefined);
  let progressPercentage = $state(0);
  let progressStage = $state(t('play.status_ready'));
  let progressDetail = $state(t('play.status_click_to_start'));
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
      addToast('info', t('app.window_minimized_sim'));
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
      addToast('info', t('app.window_closed_sim'));
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
    progressStage = t('app.prep_launch');
    progressDetail = t('app.init_profile', { version: selectedVersion });
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
      clientStatus = t('app.game_running');
      addToast('success', t('app.game_running'), result);
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
        { stage: t('app.client_check'), pct: 15, detail: t('app.check_jars'), speed: '' },
        { stage: t('app.sync_pack'), pct: 40, detail: t('app.downloading_manifest'), speed: '14.8 MB/s' },
        { stage: t('launch.check_java'), pct: 75, detail: t('app.jvm_start'), speed: '8.2 MB/s' },
        { stage: t('app.launching_game'), pct: 100, detail: t('app.jvm_start'), speed: '' },
      ];

      for (const step of steps) {
        if (cancelLaunchSimulation) {
          isLaunching = false;
          progressPercentage = 0;
          progressStage = t('launch.stopped');
          progressDetail = t('play.status_click_to_start');
          progressSpeed = '';
          addToast('warning', t('launch.stopped'), t('app.game_stopped_user'));
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
      progressStage = t('app.game_running');
      progressDetail = t('app.game_running');
      progressSpeed = '';
      addToast('success', t('app.game_running'), t('servers.feedback_game_running', { address: playerName }));
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
      clientStatus = t('app.game_stopped');
      addToast('info', t('app.game_stopped_exit'));
    } catch (e) {
      addToast('error', t('launch.stop_error'), String(e));
    }
  }

  onMount(() => {
    // Hash routing for screenshot automation: #screen=servers, #theme=light, #screen=showcase
    function applyHashRoute() {
      const hash = window.location.hash.replace(/^#/, '');
      if (!hash) return;
      const params = new URLSearchParams(hash);
      const screen = params.get('screen') || params.get('tab');
      const themeParam = params.get('theme');

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
      } else if (params.get('onboarding') === 'false') {
        showOnboarding = false;
      }
      if (themeParam === 'light' || themeParam === 'dark') {
        theme.setAppearance(themeParam);
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
      const playState = params.get('playState') || params.get('prewarmState') || stageParam;
      if (playState === 'prewarming' || playState === 'downloading') {
        playFastLaunchState = 'prewarming';
        playFastLaunchProgress = Number(params.get('progress') || 48);
        isLaunching = false;
        isGameRunning = false;
      } else if (playState === 'ready' || playState === 'prewarmed') {
        playFastLaunchState = 'ready';
        playFastLaunchProgress = 100;
        isLaunching = false;
        isGameRunning = false;
      } else if (stageParam === 'installing') {
        isLaunching = true;
        isGameRunning = false;
        progressPercentage = Number(params.get('progress') || 45);
        progressStage = params.get('stageText') || t('app.sync_pack');
        progressDetail = params.get('detail') || t('app.downloading_manifest');
        progressSpeed = params.get('speed') || '';
      } else if (playState === 'running' || stageParam === 'running') {
        isLaunching = false;
        isGameRunning = true;
        playFastLaunchState = 'running';
        addToast('success', t('app.game_running'), t('app.game_running'));
      } else if (playState === 'idle_restored' || stageParam === 'idle_restored' || playState === 'exit') {
        isLaunching = false;
        isGameRunning = false;
        playFastLaunchState = 'idle';
        addToast('info', t('app.game_stopped'), t('app.game_stopped_exit'));
      } else if (playState === 'idle' || stageParam === 'idle') {
        isLaunching = false;
        isGameRunning = false;
        playFastLaunchState = 'idle';
      }

      // Hash crash triggers for tests and previews
      const crashParam = params.get('crash');
      if (crashParam === 'oom') {
        currentCrashDiagnosis = {
          kind: 'OutOfMemory',
          title: t('crash.mock_oom_title'),
          user_advice: t('crash.mock_oom_advice'),
          technical_summary: 'Exception in thread "main" java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Main.main(Main.java:123)',
        };
        crashSheetOpen = true;
      } else if (crashParam === 'mod') {
        currentCrashDiagnosis = {
          kind: { MissingFabricDependency: { mod_name: 'iris', required_dep: 'sodium' } },
          title: t('crash.mock_mod_title'),
          user_advice: t('crash.mock_mod_advice'),
          technical_summary: 'net.fabricmc.loader.impl.FormattedException: Mod \'iris\' requires \'sodium\'',
        };
        crashSheetOpen = true;
      } else if (crashParam === 'unknown') {
        currentCrashDiagnosis = {
          kind: { Unknown: { exit_code: -1 } },
          title: t('crash.mock_unknown_title'),
          user_advice: t('crash.mock_unknown_advice'),
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
          world_name: t('lobby.world_taiga'),
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
          world_name: t('lobby.room_default_name', { name: 'Alex' }),
          tunnel_address: null,
          tunnel_active: false,
          tunnel_error: t('lobby.tunnel_service_unavailable'),
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
      (window as any).__triggerToast = (type: 'info' | 'success' | 'warning' | 'error', title: string, message: string) => {
        addToast(type, title, message);
      };
    }

    applyHashRoute();
    window.addEventListener('hashchange', applyHashRoute);

    // Real-time Friends Poller & Notifications
    const unlistenLiveNotifs = friendsLive.onNotification((notif) => {
      addToast('info', notif.title, notif.description, notif.action);
    });
    const unlistenLiveJoin = friendsLive.onJoin((_code, room) => {
      if (room) initialLobbyRoom = room;
      activeTab = 'lobby';
    });
    const unlistenLiveError = friendsLive.onError((error, context) => {
      const formatted = formatError(error, context);
      addToast('error', formatted.title, formatted.description);
    });
    friendsLive.start();

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
            clientStatus = t('app.game_running');
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
              ? `${(p.speed_bytes_per_sec / (1024 * 1024)).toFixed(1)} MB/s`
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
            } else if (s.state === 'launching') {
              isLaunching = true;
            } else if (s.state === 'running') {
              isLaunching = false;
              isGameRunning = true;
              playFastLaunchState = 'running';
              launchError = null;
              addToast('success', t('app.game_running'), s.message);
            } else if (s.state === 'cancelled') {
              isLaunching = false;
              isGameRunning = false;
              playFastLaunchState = 'idle';
              addToast('warning', t('launch.stopped'), s.message);
            } else if (s.state === 'error') {
              isLaunching = false;
              isGameRunning = false;
              playFastLaunchState = 'idle';
              launchError = s.message;
              const formatted = formatError(s.message, 'launch');
              addToast('error', formatted.title, formatted.description);
            } else if (s.state === 'idle') {
              isLaunching = false;
              isGameRunning = false;
              playFastLaunchState = 'idle';
              addToast('info', t('app.game_stopped'), s.message);
            }
          }).then((fn) => {
            unlistenStatus = fn;
          });

          listen<{
            state: string;
            progress?: number;
            message?: string;
          }>('prewarm-status', (event) => {
            const p = event.payload;
            if (p.state === 'prewarming') {
              playFastLaunchState = 'prewarming';
              playFastLaunchProgress = p.progress ?? 48;
            } else if (p.state === 'ready') {
              playFastLaunchState = 'ready';
              playFastLaunchProgress = 100;
            } else if (p.state === 'idle') {
              playFastLaunchState = 'idle';
              playFastLaunchProgress = 0;
            }
          });

          listen<{
            id: string;
            title: string;
            description: string;
            unlocked_at: string;
          }>('social-achievement-toast', (event) => {
            const a = event.payload;
            addToast('success', t('friends.ach_unlocked_title', { title: a.title }), a.description);
          }).then((fn) => {
            unlistenAchievement = fn;
          });

          listen<{
            id: string;
            lobby_code: string;
            from_nick: string;
          }>('social-lobby-invite', (event) => {
            const inv = event.payload;
            addToast('info', t('friends.lobby_invite_title', { code: inv.lobby_code }), t('friends.lobby_invite_desc', { nick: inv.from_nick }));
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
      unlistenLiveError();
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
    friendsLive.setSessionContext({ playerName, gameVersion: selectedVersion, lobby: initialLobbyRoom });
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
        <UpdateCard />
        {#if MusicDiskWidget}
          <MusicDiskWidget {isGameRunning} />
        {/if}
        <div class="theme-quick-toggle">
          <Button
            variant="ghost"
            size="small"
            onclick={() => theme.setAppearance(theme.isDark ? 'light' : 'dark')}
            title={t('theme.toggle')}
          >
            <Icon name={theme.isDark ? 'sun' : 'moon'} size={15} color="var(--color-secondary-label)" />
            <span>{theme.isDark ? t('theme.light') : t('theme.dark')}</span>
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
              prewarmState={playFastLaunchState}
              prewarmProgress={playFastLaunchProgress}
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
              <LobbyScreen {playerName} version={selectedVersion} initialRoom={initialLobbyRoom} onroomchange={(room: LobbyRoom | null) => { initialLobbyRoom = room; }} />
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
