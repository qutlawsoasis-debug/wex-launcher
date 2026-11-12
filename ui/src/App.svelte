<!--
  Wex window (reference: LauncherWindow). Sidebar on the left, the top line and
  the page in the middle, the launch bar at the bottom. Frameless window: the
  top line is the drag region.

  Stage 2 of the redesign: the frame is new; pages that are not redesigned yet
  still render the previous screens inside it until their stage replaces them.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from './lib/design/theme.svelte';
  import { motionState } from './lib/motion';
  import { network } from './lib/design/network.svelte';
  import { formatError } from './lib/design/errors';
  import { t } from './lib/i18n';
  import type { LobbyRoom } from './lib/screens/LobbyScreen.svelte';
  import type { CrashDiagnosisDto, DeepLinkActionDto } from './lib/ipc/generated/models';
  import type { DeepLinkAction } from './lib/components/DeepLinkConfirmationModal.svelte';
  import { initIdleManager, setGameRunning } from './lib/design/idleManager';
  import { friendsLive, parseWexJoinUri } from './lib/services/friendsLive';
  import UpdateCard from './lib/components/UpdateCard.svelte';

  import Sidebar from './lib/shell/Sidebar.svelte';
  import TopLine from './lib/shell/TopLine.svelte';
  import LaunchBar from './lib/shell/LaunchBar.svelte';
  import ToastHost from './lib/shell/ToastHost.svelte';
  import CommandPalette from './lib/shell/CommandPalette.svelte';
  import { palette } from './lib/shell/commands.svelte';
  import { handleHotkey } from './lib/shell/hotkeys';
  import HomePage from './lib/pages/home/HomePage.svelte';
  import BuildPage from './lib/pages/build/BuildPage.svelte';
  import SettingsPage from './lib/pages/settings/SettingsPage.svelte';
  import { applyLook } from './lib/shell/look';
  import { router, buildIdOf, isRoute, buildRoute, type Route } from './lib/shell/router.svelte';
  import { builds } from './lib/shell/builds.svelte';
  import { launch } from './lib/shell/launch.svelte';
  import { toasts } from './lib/shell/toast.svelte';
  import { call, on, inTauri } from './lib/shell/ipc';
  import { stagger } from './lib/shell/stagger';

  type ToastKind = 'info' | 'success' | 'warning' | 'error';
  /** Bridge for the previous screens, which report (type, title, message). */
  const notify = (_type: ToastKind, title: string, message?: string) => toasts.show(title, message);

  // Previous screens, loaded on demand until their stage replaces them.
  let screens = $state<Record<string, any>>({});
  async function loadScreen(name: string) {
    if (screens[name]) return;
    const loaders: Record<string, () => Promise<{ default: any }>> = {
      servers: () => import('./lib/screens/ServersScreen.svelte'),
      friends: () => import('./lib/screens/FriendsScreen.svelte'),
      lobby: () => import('./lib/screens/LobbyScreen.svelte'),
      wardrobe: () => import('./lib/screens/WardrobeScreen.svelte'),
      splash: () => import('./lib/screens/SplashScreen.svelte'),
    };
    const load = loaders[name];
    if (load) screens[name] = (await load()).default;
  }

  let isStartingUp = $state(true);
  let showOnboarding = $state(false);
  let crashSheetOpen = $state(false);
  let crashDiagnosis = $state<CrashDiagnosisDto | null>(null);
  let deepLinkModalOpen = $state(false);
  let pendingDeepLink = $state<DeepLinkAction | DeepLinkActionDto | null>(null);
  let lobbyRoom = $state<LobbyRoom | null>(null);
  let friendsSubtab = $state<'friends' | 'requests' | 'achievements'>('friends');
  let mainEl: HTMLElement | undefined = $state();

  let CrashSheet = $state<any>(null);
  let OnboardingModal = $state<any>(null);
  let DeepLinkConfirmationModal = $state<any>(null);

  const isRunning = $derived(launch.phase === 'running');
  const routeBuild = $derived(buildIdOf(router.route));

  $effect(() => {
    const r = router.route;
    if (r === 'servers' || r === 'friends' || r === 'lobby' || r === 'wardrobe') void loadScreen(r);
  });

  $effect(() => {
    if (isStartingUp) void loadScreen('splash');
    if (crashSheetOpen && !CrashSheet) import('./lib/components/CrashSheet.svelte').then((m) => (CrashSheet = m.default));
    if (showOnboarding && !OnboardingModal) import('./lib/components/OnboardingModal.svelte').then((m) => (OnboardingModal = m.default));
    if (deepLinkModalOpen && !DeepLinkConfirmationModal)
      import('./lib/components/DeepLinkConfirmationModal.svelte').then((m) => (DeepLinkConfirmationModal = m.default));
  });

  $effect(() => {
    setGameRunning(isRunning);
    motionState.setGameRunning(isRunning);
  });

  // Each page starts at the top.
  $effect(() => {
    void router.version;
    if (mainEl) mainEl.scrollTop = 0;
  });


  $effect(() => {
    friendsLive.setSessionContext({ playerName: builds.playerName, gameVersion: launch.version ?? builds.selectedId, lobby: lobbyRoom });
    friendsLive.setGameRunning(isRunning);
  });

  /** Map the previous screens' tab names to routes. */
  function openLegacyTab(tab: string) {
    const map: Record<string, Route> = {
      play: 'home',
      settings: 'options',
      servers: 'servers',
      friends: 'friends',
      wardrobe: 'wardrobe',
      lobby: 'lobby',
    };
    if (tab === 'mods' || tab === 'workshop') {
      router.go(buildRoute(builds.selectedId));
    } else if (map[tab]) {
      router.go(map[tab]);
    }
  }

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
      }
      if (path.includes('server') || address) {
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
      if (inTauri()) {
        const parsed = await call<DeepLinkActionDto | null>('parse_deep_link_uri', { uri });
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
      await call('clear_pending_deep_link');
    } catch {}
    if (action.kind === 'join') {
      try {
        await friendsLive.handleJoin(action.target);
      } catch (error) {
        const f = formatError(error, 'lobby');
        toasts.show(f.title, f.description);
      }
    } else if (action.kind === 'server') {
      router.go('home');
      void launch.start(action.target);
    } else if (action.kind === 'friend') {
      friendsSubtab = 'requests';
      router.go('friends');
      try {
        await call('social_send_friend_request', { targetFriendCode: action.target });
        toasts.show(t('app.toast_request_sent'), t('app.toast_request_sent_desc', { target: action.target }));
      } catch (e) {
        const f = formatError(e, 'friends');
        toasts.show(t('app.err_friend_request'), f.description);
      }
    }
  }

  async function handleCancelDeepLink() {
    deepLinkModalOpen = false;
    pendingDeepLink = null;
    try {
      await call('clear_pending_deep_link');
    } catch {}
  }

  function openFromHash() {
    const hash = window.location.hash.replace(/^#/, '');
    const join = parseWexJoinUri(window.location.hash) || parseWexJoinUri(window.location.search);
    if (join) {
      router.go('lobby', false);
      void friendsLive.handleJoin(join);
      return;
    }
    if (hash && isRoute(hash, builds.list.map((b) => b.id))) router.go(hash, false);
  }

  function handleKeydown(e: KeyboardEvent) {
    const used = handleHotkey(e, {
      overlayOpen: crashSheetOpen || showOnboarding || deepLinkModalOpen || isStartingUp,
      closeOverlay: () => {
        if (crashSheetOpen) {
          crashSheetOpen = false;
          return true;
        }
        // Sheets of the previous screens listen for this.
        window.dispatchEvent(new CustomEvent('wex-escape'));
        return false;
      },
    });
    if (used) e.preventDefault();
  }

  onMount(() => {
    const stops: Array<() => void> = [];
    stops.push(launch.attach());
    void builds.load().then(() => {
      // Theme, motion, transparency and language come from the saved settings.
      applyLook(builds.settings);
      if (builds.error) {
        const f = formatError(builds.error, 'general');
        toasts.show(t('shell.builds_failed'), f.description);
      }
      openFromHash();
    });

    try {
      if (localStorage.getItem('wex_onboarding_completed') !== 'true') showOnboarding = true;
    } catch {}
    const showOnboardingEvent = () => (showOnboarding = true);
    const openCommands = () => palette.show();
    window.addEventListener('wex-open-command', openCommands);
    stops.push(() => window.removeEventListener('wex-open-command', openCommands));
    window.addEventListener('wex-show-onboarding', showOnboardingEvent);
    window.addEventListener('hashchange', openFromHash);
    stops.push(() => {
      window.removeEventListener('wex-show-onboarding', showOnboardingEvent);
      window.removeEventListener('hashchange', openFromHash);
    });

    stops.push(
      friendsLive.onNotification((n) =>
        toasts.show(n.title, n.description, n.action ? { label: n.action.label, run: n.action.onClick } : undefined)
      ),
      friendsLive.onJoin((_code, room) => {
        if (room) lobbyRoom = room;
        router.go('lobby');
      }),
      friendsLive.onError((error, context) => {
        const f = formatError(error, context);
        toasts.show(f.title, f.description);
      })
    );
    friendsLive.start();
    stops.push(() => friendsLive.stop());

    void (async () => {
      try {
        const cfg = await call<{ friend_notifications?: boolean }>('get_launcher_settings');
        if (cfg?.friend_notifications !== undefined) friendsLive.setNotificationsEnabled(cfg.friend_notifications);
        const pending = await call<DeepLinkActionDto | null>('get_pending_deep_link');
        if (pending) {
          pendingDeepLink = pending;
          deepLinkModalOpen = true;
        }
      } catch {}
    })();

    stops.push(initIdleManager() ?? (() => {}));

    stops.push(
      on<CrashDiagnosisDto>('game-crash', (d) => {
        crashDiagnosis = d;
        crashSheetOpen = true;
      }),
      on<DeepLinkActionDto>('wex://deep-link', (d) => {
        if (d) {
          pendingDeepLink = d;
          deepLinkModalOpen = true;
        }
      }),
      on<string | string[]>('deep-link://new-url', (payload) => {
        const urls = Array.isArray(payload) ? payload : [payload];
        const first = urls.find((u) => typeof u === 'string' && (u.startsWith('wex://') || u.startsWith('aura://')));
        if (first) void handleRawDeepLink(first);
      }),
      on<{ title: string; description: string }>('social-achievement-toast', (a) =>
        toasts.show(t('friends.ach_unlocked_title', { title: a.title }), a.description)
      ),
      on<{ lobby_code: string; from_nick: string }>('social-lobby-invite', (inv) =>
        toasts.show(t('friends.lobby_invite_title', { code: inv.lobby_code }), t('friends.lobby_invite_desc', { nick: inv.from_nick }))
      )
    );

    return () => stops.forEach((stop) => stop());
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app" class:game-active={isRunning} data-theme={theme.isDark ? 'dark' : 'light'}>
  <Sidebar online={network.isOnline}>
    {#snippet footer()}<UpdateCard />{/snippet}
  </Sidebar>

  <main class="main" bind:this={mainEl}>
    <TopLine />
    {#key router.version}
      <div class="page" class:wx={router.route === 'home' || router.route === 'options' || !!routeBuild} use:stagger>
        {#if router.route === 'home'}
          <HomePage />
        {:else if routeBuild}
          <BuildPage version={routeBuild} />
        {:else if router.route === 'servers' && screens.servers}
          {@const ServersScreen = screens.servers}
          <div class="legacy"><ServersScreen selectedVersion={builds.selectedId} playerName={builds.playerName} /></div>
        {:else if router.route === 'friends' && screens.friends}
          {@const FriendsScreen = screens.friends}
          <div class="legacy"><FriendsScreen tab={friendsSubtab} /></div>
        {:else if router.route === 'lobby' && screens.lobby}
          {@const LobbyScreen = screens.lobby}
          <div class="legacy">
            <LobbyScreen
              playerName={builds.playerName}
              version={builds.selectedId}
              initialRoom={lobbyRoom}
              onroomchange={(room: LobbyRoom | null) => (lobbyRoom = room)}
            />
          </div>
        {:else if router.route === 'wardrobe' && screens.wardrobe}
          {@const WardrobeScreen = screens.wardrobe}
          <div class="legacy"><WardrobeScreen playerName={builds.playerName} ontoast={notify} /></div>
        {:else if router.route === 'options'}
          <SettingsPage onopenonboarding={() => (showOnboarding = true)} />
        {/if}
      </div>
    {/key}
  </main>

  <LaunchBar />
  <ToastHost />
  <CommandPalette />

  {#if isStartingUp && screens.splash}
    {@const SplashScreen = screens.splash}
    <SplashScreen oncomplete={() => (isStartingUp = false)} />
  {/if}

  {#if crashSheetOpen && CrashSheet}
    <CrashSheet bind:open={crashSheetOpen} diagnosis={crashDiagnosis} version={launch.version ?? builds.selectedId} onnavigate={openLegacyTab} ontoast={notify} />
  {/if}

  {#if showOnboarding && OnboardingModal}
    <OnboardingModal
      bind:open={showOnboarding}
      oncomplete={async (data: { nickname: string; version: string; launchImmediately?: boolean }) => {
        await builds.load();
        toasts.show(t('app.onboarding_completed_title'), t('app.onboarding_welcome_toast', { name: data.nickname }));
        if (data.launchImmediately) void launch.start();
      }}
    />
  {/if}

  {#if deepLinkModalOpen && DeepLinkConfirmationModal}
    <DeepLinkConfirmationModal bind:open={deepLinkModalOpen} action={pendingDeepLink} onconfirm={handleConfirmDeepLink} oncancel={handleCancelDeepLink} />
  {/if}
</div>

<style>
  .app {
    position: relative;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    isolation: isolate;
    display: grid;
    grid-template-columns: var(--side-w) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    background: var(--base);
    color: var(--label);
    font-family: var(--font);
  }
  .main {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: auto;
  }
  .page {
    padding: 0 var(--gutter) 32px;
    display: flex;
    flex-direction: column;
    gap: 28px;
    view-transition-name: page;
  }
  :global(.page.stagger > *) {
    animation: rise var(--dur-page) var(--ease) both;
    animation-delay: calc(var(--i, 0) * var(--stagger));
  }
  @keyframes -global-rise {
    from {
      opacity: 0;
      transform: translateY(14px);
    }
  }
  :global(::view-transition-old(root)),
  :global(::view-transition-new(root)),
  :global(::view-transition-new(page)) {
    animation: none;
  }
  :global(::view-transition-old(page)) {
    animation: vt-out var(--dur-page-out) var(--ease) both;
  }
  @keyframes -global-vt-out {
    to {
      opacity: 0;
      transform: translateY(-6px);
    }
  }

  .legacy {
    margin-top: 16px;
    min-height: 0;
  }
  .page > .legacy {
    margin-top: 0;
  }

  /* While the game runs the launcher idles: no animation, no blur (energy). */
  :global(.app.game-active *) {
    animation-play-state: paused !important;
  }
  :global(.app.game-active .side),
  :global(.app.game-active .bar) {
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
  }
</style>
