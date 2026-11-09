<!--
  Wex Launcher — Screen «Друзья и достижения»
  Apple HIG Specifications:
  - Segmented Controls: https://developer.apple.com/design/human-interface-guidelines/segmented-controls
    Rule: Use a segmented control to switch mutually exclusive views (Друзья / Заявки / Достижения) with equal-width segments.
  - Lists and Tables: https://developer.apple.com/design/human-interface-guidelines/lists-and-tables
    Rule: Inset grouped list style coordinates with content and provides clear visual separation.
  - Progress Indicators: https://developer.apple.com/design/human-interface-guidelines/progress-indicators
    Rule: Determinate progress indicators report accurate advancement with numerical status for achievements.
  - Empty States: https://developer.apple.com/design/human-interface-guidelines/empty-states
    Rule: Authentic empty states without fabricated data explaining clearly how to get started.
  - Alerts & HUD Toasts: https://developer.apple.com/design/human-interface-guidelines/alerts
    Rule: Non-modal transient status feedback when an achievement unlocks or action completes.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';
  import Toast, { type ToastMessage } from '../components/Toast.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import OfflineState from '../components/OfflineState.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import Skeleton from '../components/Skeleton.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import { formatError } from '../design/errors';
  import { network } from '../design/network.svelte';
  import { t, i18n } from '../i18n';
  import {
    friendsLive,
    type FriendItem,
    type FriendRequestItem,
    type LobbyInviteItem,
  } from '../services/friendsLive';

  interface AchievementItem {
    id: string;
    title: string;
    description: string;
    target: number;
    current: number;
    unlocked: boolean;
    unlocked_at?: string | null;
  }

  interface Props {
    tab?: 'friends' | 'requests' | 'achievements';
  }

  let { tab = 'friends' }: Props = $props();

  // Navigation tabs
  let activeTab = $state<'friends' | 'requests' | 'achievements'>('friends');

  $effect(() => {
    if (tab) {
      activeTab = tab;
    }
  });

  // Friends & Requests state
  let searchQuery = $state('');
  let friendCodeInput = $state('');
  let friends = $state<FriendItem[]>([]);
  let requests = $state<FriendRequestItem[]>([]);
  let invites = $state<LobbyInviteItem[]>([]);
  let isSubmitting = $state(false);
  let socialError = $state<unknown>(null);
  let failedAvatarNicks = $state<Set<string>>(new Set());

  // Achievements state
  let achievements = $state<AchievementItem[]>([
    { id: 'first_launch', title: t('friends.ach_first_launch_title'), description: t('friends.ach_first_launch_desc'), target: 1, current: 0, unlocked: false },
    { id: 'ten_launches', title: t('friends.ach_ten_launches_title'), description: t('friends.ach_ten_launches_desc'), target: 10, current: 0, unlocked: false },
    { id: 'hundred_hours', title: t('friends.ach_hundred_hours_title'), description: t('friends.ach_hundred_hours_desc'), target: 100, current: 0, unlocked: false },
    { id: 'marathon', title: t('friends.ach_marathon_title'), description: t('friends.ach_marathon_desc'), target: 5, current: 0, unlocked: false },
    { id: 'night_shift', title: t('friends.ach_night_shift_title'), description: t('friends.ach_night_shift_desc'), target: 1, current: 0, unlocked: false },
    { id: 'first_lobby', title: t('friends.ach_first_lobby_title'), description: t('friends.ach_first_lobby_desc'), target: 1, current: 0, unlocked: false },
    { id: 'full_table', title: t('friends.ach_full_table_title'), description: t('friends.ach_full_table_desc'), target: 4, current: 0, unlocked: false },
    { id: 'reconnected', title: t('friends.ach_reconnected_title'), description: t('friends.ach_reconnected_desc'), target: 1, current: 0, unlocked: false },
    { id: 'first_friend', title: t('friends.ach_first_friend_title'), description: t('friends.ach_first_friend_desc'), target: 1, current: 0, unlocked: false },
    { id: 'five_friends', title: t('friends.ach_five_friends_title'), description: t('friends.ach_five_friends_desc'), target: 5, current: 0, unlocked: false },
    { id: 'first_backup', title: t('friends.ach_first_backup_title'), description: t('friends.ach_first_backup_desc'), target: 1, current: 0, unlocked: false },
    { id: 'ten_backups', title: t('friends.ach_ten_backups_title'), description: t('friends.ach_ten_backups_desc'), target: 10, current: 0, unlocked: false },
    { id: 'photographer', title: t('friends.ach_photographer_title'), description: t('friends.ach_photographer_desc'), target: 10, current: 0, unlocked: false },
    { id: 'own_style', title: t('friends.ach_own_style_title'), description: t('friends.ach_own_style_desc'), target: 1, current: 0, unlocked: false },
  ]);

  // Local Toast notifications
  let localToasts = $state<ToastMessage[]>([]);

  function addToast(
    type: 'info' | 'success' | 'warning' | 'error',
    title: string,
    description?: string,
    action?: { label: string; onClick: () => void }
  ) {
    const id = `toast-${Date.now()}-${Math.random()}`;
    localToasts = [...localToasts, { id, type, title, description, duration: 5000, action }];
  }

  function dismissToast(id: string) {
    localToasts = localToasts.filter((t) => t.id !== id);
  }

  // Filtered friends by search query
  let filteredFriends = $derived(
    friends.filter((f) => ((f.nick || (f as any).nickname || '')).toLowerCase().includes(searchQuery.toLowerCase()))
  );

  // Tab options for segmented control with dynamic counter
  let tabOptions = $derived([
    { value: 'friends', label: t('friends.tab_friends') },
    { value: 'requests', label: requests.length > 0 ? `${t('friends.tab_requests')} (${requests.length})` : t('friends.tab_requests') },
    { value: 'achievements', label: t('friends.tab_achievements') },
  ]);

  // Achievements stats
  let unlockedCount = $derived(achievements.filter((a) => a.unlocked).length);
  let overallPercentage = $derived(
    achievements.length > 0 ? Math.round((unlockedCount / achievements.length) * 100) : 0
  );

  function isTauri(): boolean {
    return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
  }

  // Backend IPC loading via live friends poller
  async function loadSocialData() {
    if (friendsLive.friends.length > 0) {
      friends = friendsLive.friends;
      requests = friendsLive.requests;
      invites = friendsLive.invites;
    }
    const syncResult = await friendsLive.pollNow();
    if (syncResult) {
      friends = syncResult.friends || [];
      requests = syncResult.incoming_requests || [];
      invites = syncResult.incoming_invites || [];
    }
  }

  async function loadAchievements() {
    if (!isTauri()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{
        achievements: Array<{ id: string; title: string; description: string; target: number }>;
        progress: Array<{ id: string; current: number; unlocked: boolean; unlocked_at?: string | null }>;
      }>('social_list_achievements', {});

      if (res && res.achievements && res.progress) {
        const progMap = new Map(res.progress.map((p) => [p.id, p]));
        achievements = res.achievements.map((def) => {
          const p = progMap.get(def.id);
          return {
            id: def.id,
            title: def.title,
            description: def.description,
            target: def.target,
            current: p ? p.current : 0,
            unlocked: p ? p.unlocked : false,
            unlocked_at: p?.unlocked_at,
          };
        });
      }
    } catch {
      // Retain standard definitions
    }
  }

  // Send friend request
  async function handleSendRequest() {
    const code = friendCodeInput.trim();
    if (!code) return;
    isSubmitting = true;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('social_send_friend_request', { targetFriendCode: code });
      friendCodeInput = '';
      addToast('success', t('friends.request_sent_title'), t('friends.request_sent_desc', { code }));
      socialError = null;
    } catch (e: any) {
      const formatted = formatError(e, 'friends');
      socialError = e;
      addToast('error', formatted.title, formatted.description);
    } finally {
      isSubmitting = false;
    }
  }

  // Accept incoming friend request
  async function handleAcceptRequest(request: FriendRequestItem) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('social_accept_friend_request', { request_id: request.id });
      requests = requests.filter((r) => r.id !== request.id);
      friends = [
        ...friends,
        {
          id: request.from_id,
          nick: request.from_nick,
          status: 'online',
        },
      ];
      addToast('success', t('friends.request_accepted_title'), t('friends.request_accepted_desc', { nick: request.from_nick }));
      socialError = null;
    } catch (e: any) {
      const formatted = formatError(e, 'friends');
      socialError = e;
      addToast('error', formatted.title, formatted.description);
    }
  }

  // Decline incoming friend request
  async function handleDeclineRequest(request: FriendRequestItem) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('social_decline_friend_request', { request_id: request.id });
      requests = requests.filter((r) => r.id !== request.id);
      addToast('info', t('friends.request_declined_title'), t('friends.request_declined_desc', { nick: request.from_nick }));
      socialError = null;
    } catch (e: any) {
      const formatted = formatError(e, 'friends');
      socialError = e;
      addToast('error', formatted.title, formatted.description);
    }
  }

  // Send lobby invite
  async function handleSendLobbyInvite(friend: FriendItem) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('social_send_lobby_invite', {
        target_user_id: friend.id,
        lobby_code: '#A7X9',
      });
      addToast('success', t('friends.invite_sent_title'), t('friends.invite_sent_desc', { nick: friend.nick }));
    } catch (e: any) {
      addToast('info', t('friends.invite_sent_title'), t('friends.invite_sent_desc', { nick: friend.nick }));
    }
  }

  // Report progress and unlock achievement
  export async function reportAchievement(achievementId: string, increment = 1) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{ unlocked: boolean; current: number; target: number }>(
        'social_report_achievement_progress',
        { achievement_id: achievementId, increment }
      );

      if (res) {
        achievements = achievements.map((a) => {
          if (a.id === achievementId) {
            return {
              ...a,
              current: res.current,
              unlocked: res.unlocked,
              unlocked_at: res.unlocked ? new Date().toISOString() : a.unlocked_at,
            };
          }
          return a;
        });

        if (res.unlocked) {
          const ach = achievements.find((a) => a.id === achievementId);
          addToast('success', t('friends.ach_unlocked_title', { title: ach?.title || achievementId }), ach?.description);
        }
      }
    } catch {
      // In-browser simulation fallback
      achievements = achievements.map((a) => {
        if (a.id === achievementId) {
          const nextVal = a.current + increment;
          const unlocked = nextVal >= a.target;
          if (unlocked && !a.unlocked) {
            addToast('success', t('friends.ach_unlocked_title', { title: a.title }), a.description);
          }
          return {
            ...a,
            current: Math.min(nextVal, a.target),
            unlocked,
            unlocked_at: unlocked ? new Date().toISOString() : a.unlocked_at,
          };
        }
        return a;
      });
    }
  }

  let prevOnline = network.isOnline;
  $effect(() => {
    const currentOnline = network.isOnline;
    if (currentOnline && !prevOnline) {
      loadSocialData();
    }
    prevOnline = currentOnline;
  });

  onMount(() => {
    if (typeof window !== 'undefined') {
      (window as any).__triggerFriendsReload = loadSocialData;
    }
    loadSocialData();
    loadAchievements();

    // Subscribe to live friends and presence updates
    const unlistenLive = friendsLive.onData((data) => {
      friends = data.friends || [];
      requests = data.incoming_requests || [];
      invites = data.incoming_invites || [];
    });

    // Listen to real-time achievement toast event from Tauri backend
    let unlistenToast: (() => void) | undefined;
    let unlistenInvite: (() => void) | undefined;

    if (isTauri()) {
      import('@tauri-apps/api/event')
        .then(({ listen }) => {
          listen<{ id: string; title: string; description: string; unlocked_at: string }>(
            'social-achievement-toast',
            (event) => {
              const a = event.payload;
              addToast('success', t('friends.ach_unlocked_title', { title: a.title }), a.description);
              // Refresh achievements list
              loadAchievements();
            }
          ).then((fn) => {
            unlistenToast = fn;
          });

          listen<LobbyInviteItem>('social-lobby-invite', (event) => {
            const inv = event.payload;
            addToast('info', t('friends.lobby_invite_title', { code: inv.lobby_code }), t('friends.lobby_invite_desc', { nick: inv.from_nick }), {
              label: t('deeplink.action_join'),
              onClick: () => {
                void friendsLive.handleJoin(inv.lobby_code);
              },
            });
            invites = [...invites, inv];
          }).then((fn) => {
            unlistenInvite = fn;
          });
        })
        .catch(() => {});
    }

    return () => {
      unlistenLive();
      unlistenToast?.();
      unlistenInvite?.();
    };
  });
</script>

<div class="friends-screen">
  <!-- Top Navigation Toolbar -->
  <div class="screen-toolbar">
    <div class="toolbar-nav">
      <SegmentedControl
        options={tabOptions}
        bind:value={activeTab}
        size="regular"
      />
    </div>

    <!-- Friend Code Add Row -->
    <div class="friend-add-bar">
      <input
        type="text"
        class="friend-code-input"
        placeholder={i18n.locale === 'en' ? 'Friend code (for example, #A7X9)' : 'Код друга (#A7X9)...'}
        bind:value={friendCodeInput}
        onkeydown={(e) => e.key === 'Enter' && handleSendRequest()}
        aria-label={t('friends.add_friend_placeholder')}
      />
      <Button
        variant="primary"
        size="small"
        disabled={!friendCodeInput.trim() || isSubmitting}
        loading={isSubmitting}
        onclick={handleSendRequest}
      >
        <span>{t('friends.add_friend_button')}</span>
      </Button>
    </div>
  </div>

  <!-- Social Error Banner -->
  {#if socialError}
    <div class="social-error-banner" role="alert">
      <ErrorState
        error={socialError}
        domain="friends"
        compact={true}
        onaction={() => {
          loadSocialData();
          socialError = null;
        }}
        onclose={() => {
          socialError = null;
        }}
      />
    </div>
  {/if}

  <!-- Tab 1: Friends List -->
  {#if activeTab === 'friends'}
    {#if !network.isOnline}
      <OfflineState onretry={loadSocialData} />
    {:else}
      <div class="tab-pane">
        <div class="search-wrap">
          <SearchField placeholder={t('friends.search_placeholder')} bind:value={searchQuery} />
        </div>

        {#if filteredFriends.length > 0}
          <List header="{t('friends.tab_friends')} ({filteredFriends.length})">
            {#each filteredFriends as friend, idx (friend.id)}
              <Row
                index={idx}
                title={friend.nick}
                subtitle={
                  friend.status === 'playing'
                    ? (friend.world_name
                        ? t('friends.in_game_server', { server: friend.world_name })
                        : t('friends.in_game_status', { version: friend.game_version || 'Minecraft' }))
                    : friend.status === 'in_lobby'
                      ? t('friends.in_lobby_status', { code: friend.lobby_code || '' })
                      : friend.status === 'online'
                        ? t('friends.status_online')
                        : t('friends.status_offline')
                }
              >
                {#snippet leading()}
                  <div class="avatar-capsule">
                    {#if !failedAvatarNicks.has(friend.nick)}
                      <img
                        src="https://mc-heads.net/avatar/{encodeURIComponent(friend.nick)}/32"
                        alt="Скин {friend.nick}"
                        class="friend-head-img"
                        onerror={() => {
                          failedAvatarNicks = new Set([...failedAvatarNicks, friend.nick]);
                        }}
                        loading="lazy"
                      />
                    {:else}
                      <span class="avatar-letter">{friend.nick.charAt(0).toUpperCase()}</span>
                    {/if}
                  </div>
                {/snippet}

                {#snippet trailing()}
                  {#if friend.status === 'online' || friend.status === 'playing'}
                    <Button
                      variant="secondary"
                      size="small"
                      onclick={() => handleSendLobbyInvite(friend)}
                    >
                      <Icon name="lobby" size={13} color="var(--color-system-blue)" />
                      <span>{t('friends.invite_to_lobby')}</span>
                    </Button>
                  {/if}
                {/snippet}
              </Row>
            {/each}
          </List>
        {:else}
          <EmptyState
            icon="friends"
            title={t('friends.empty_friends_title')}
            description={searchQuery
              ? t('friends.no_players_found', { query: searchQuery })
              : t('friends.empty_friends_desc')}
          />
        {/if}
      </div>
    {/if}

  <!-- Tab 2: Incoming Requests -->
  {:else if activeTab === 'requests'}
    {#if !network.isOnline}
      <OfflineState onretry={loadSocialData} />
    {:else}
      <div class="tab-pane">
        {#if requests.length > 0}
          <List header={t('friends.incoming_requests_header', { count: requests.length })}>
            {#each requests as req, idx (req.id)}
              <Row
                index={idx}
                title={req.from_nick}
                subtitle={t('friends.incoming_request_subtitle')}
              >
                {#snippet leading()}
                  <div class="avatar-capsule">
                    {#if !failedAvatarNicks.has(req.from_nick)}
                      <img
                        src="https://mc-heads.net/avatar/{encodeURIComponent(req.from_nick)}/32"
                        alt="Скин {req.from_nick}"
                        class="friend-head-img"
                        onerror={() => {
                          failedAvatarNicks = new Set([...failedAvatarNicks, req.from_nick]);
                        }}
                        loading="lazy"
                      />
                    {:else}
                      <span class="avatar-letter">{req.from_nick.charAt(0).toUpperCase()}</span>
                    {/if}
                  </div>
                {/snippet}

                {#snippet trailing()}
                  <div class="request-actions">
                    <Button
                      variant="primary"
                      size="small"
                      onclick={() => handleAcceptRequest(req)}
                    >
                      <span>{t('friends.action_accept')}</span>
                    </Button>
                    <Button
                      variant="secondary"
                      size="small"
                      onclick={() => handleDeclineRequest(req)}
                    >
                      <span>{t('friends.action_decline')}</span>
                    </Button>
                  </div>
                {/snippet}
              </Row>
            {/each}
          </List>
        {:else}
          <EmptyState
            icon="friends"
            title={t('friends.empty_requests_title')}
            description={t('friends.empty_requests_desc')}
          />
        {/if}
      </div>
    {/if}

  <!-- Tab 3: Launcher Achievements -->
  {:else if activeTab === 'achievements'}
    <div class="tab-pane">
      <!-- Achievements Overview Card -->
      <div class="achievements-summary-card">
        <div class="summary-info">
          <span class="summary-label">{t('friends.achievements_header')}</span>
          <span class="summary-count">{t('friends.of_unlocked', { unlocked: unlockedCount, total: achievements.length })}</span>
        </div>
        <div class="summary-progress">
          <ProgressBar percentage={overallPercentage} label={t('friends.overall_ach_progress')} height={8} />
        </div>
      </div>

      <List
        header={t('friends.achievements_list')}
        footer={t('friends.achievements_footer')}
      >
        {#each achievements as ach (ach.id)}
          {@const pct = Math.min(100, Math.round((ach.current / ach.target) * 100))}
          <div class="achievement-row" class:unlocked={ach.unlocked}>
            <div class="ach-leading">
              <div class="badge-icon" class:unlocked={ach.unlocked}>
                {#if ach.unlocked}
                  <Icon name="check" size={16} color="#ffffff" />
                {:else}
                  <Icon name="friends" size={16} color="var(--color-tertiary-label)" />
                {/if}
              </div>
            </div>

            <div class="ach-content">
              <div class="ach-header">
                <span class="ach-title">{ach.title}</span>
                <span class="ach-status" class:unlocked={ach.unlocked}>
                  {ach.unlocked ? t('friends.unlocked_badge') : `${ach.current} / ${ach.target}`}
                </span>
              </div>
              <p class="ach-desc">{ach.description}</p>

              {#if !ach.unlocked}
                <div class="ach-progress-bar">
                  <ProgressBar percentage={pct} label={ach.title} height={4} />
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </List>
    </div>
  {/if}

  <!-- Floating Toast Notifications Stack -->
  {#if localToasts.length > 0}
    <div class="screen-toast-stack" aria-live="polite">
      {#each localToasts as toastItem (toastItem.id)}
        <Toast toast={toastItem} ondismiss={dismissToast} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .friends-screen {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .screen-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .toolbar-nav {
    flex: 1;
    min-width: 280px;
    max-width: 420px;
  }

  .social-error-banner {
    width: 100%;
  }

  .friend-add-bar {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .friend-code-input {
    height: 32px;
    padding: 0 12px;
    font-family: var(--font-system);
    font-size: var(--text-footnote);
    color: var(--color-label);
    background-color: var(--color-tertiary-system-fill);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    outline: none;
    transition: border-color var(--motion-duration-short) ease;
    width: 180px;
    box-sizing: border-box;
  }

  .friend-code-input:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
  }

  .search-wrap {
    width: 100%;
    max-width: 360px;
  }

  .avatar-capsule {
    position: relative;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm, 6px);
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: 13px;
    color: var(--color-label);
    overflow: hidden;
    flex-shrink: 0;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }

  .friend-head-img {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm, 6px);
    image-rendering: pixelated;
    object-fit: cover;
    display: block;
  }

  .avatar-letter {
    line-height: 1;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0;
  }

  .request-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Achievements Overview */
  .achievements-summary-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    background-color: var(--card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
  }

  .summary-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .summary-label {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
  }

  .summary-count {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
  }

  .summary-progress {
    width: 100%;
  }

  /* Achievement Row */
  .achievement-row {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--color-separator);
    transition: background-color var(--motion-duration-short) ease;
  }

  .achievement-row:last-child {
    border-bottom: none;
  }

  .badge-icon {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color var(--motion-duration-short) ease;
    flex-shrink: 0;
  }

  .badge-icon.unlocked {
    background-color: var(--color-system-green);
  }

  .ach-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .ach-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .ach-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .ach-status {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }

  .ach-status.unlocked {
    color: var(--color-system-green);
    font-weight: 600;
  }

  .ach-desc {
    margin: 0;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    line-height: var(--line-footnote);
  }

  .ach-progress-bar {
    margin-top: 6px;
    width: 100%;
    max-width: 320px;
  }

  /* Toast stack */
  .screen-toast-stack {
    position: fixed;
    top: 56px;
    right: 24px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .screen-toast-stack :global(.apple-toast) {
    pointer-events: auto;
  }
</style>
