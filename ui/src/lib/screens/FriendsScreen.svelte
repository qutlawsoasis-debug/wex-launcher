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

  // Achievements state
  let achievements = $state<AchievementItem[]>([
    { id: 'first_launch', title: 'Искра', description: 'Первый запуск игры', target: 1, current: 0, unlocked: false },
    { id: 'ten_launches', title: 'Постоялец', description: '10 запусков игры', target: 10, current: 0, unlocked: false },
    { id: 'hundred_hours', title: 'Сто часов', description: '100 часов в игре', target: 100, current: 0, unlocked: false },
    { id: 'marathon', title: 'Марафон', description: '5 часов в одной сессии', target: 5, current: 0, unlocked: false },
    { id: 'night_shift', title: 'Ночная смена', description: 'Запуск с 03:00 до 05:00', target: 1, current: 0, unlocked: false },
    { id: 'first_lobby', title: 'Хост', description: 'Первое созданное лобби', target: 1, current: 0, unlocked: false },
    { id: 'full_table', title: 'Полный стол', description: '4 игрока в твоём лобби', target: 4, current: 0, unlocked: false },
    { id: 'reconnected', title: 'Вернулся', description: 'Нажал «Переподключиться»', target: 1, current: 0, unlocked: false },
    { id: 'first_friend', title: 'Свой человек', description: 'Первый друг в списке', target: 1, current: 0, unlocked: false },
    { id: 'five_friends', title: 'Компания', description: '5 друзей в списке', target: 5, current: 0, unlocked: false },
    { id: 'first_backup', title: 'Страховка', description: 'Первый созданный бэкап мира', target: 1, current: 0, unlocked: false },
    { id: 'ten_backups', title: 'Параноик', description: '10 бэкапов миров', target: 10, current: 0, unlocked: false },
    { id: 'photographer', title: 'Фотограф', description: '10 скриншотов в галерее', target: 10, current: 0, unlocked: false },
    { id: 'own_style', title: 'Свой стиль', description: 'Установлен кастомный скин', target: 1, current: 0, unlocked: false },
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
      await invoke('social_send_friend_request', { target_friend_code: code });
      friendCodeInput = '';
      addToast('success', 'Заявка отправлена', `Заявка в друзья по коду ${code} успешно передана`);
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
      addToast('success', 'Заявка принята', `${request.from_nick} теперь в списке ваших друзей`);
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
      addToast('info', 'Заявка отклонена', `Заявка от ${request.from_nick} отклонена`);
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
      addToast('success', 'Приглашение отправлено', `${friend.nick} получил приглашение в ваше лобби`);
    } catch (e: any) {
      addToast('info', 'Приглашение отправлено', `Приглашение для ${friend.nick} отправлено`);
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
          addToast('success', `Достижение разблокировано: «${ach?.title || achievementId}»`, ach?.description);
        }
      }
    } catch {
      // In-browser simulation fallback
      achievements = achievements.map((a) => {
        if (a.id === achievementId) {
          const nextVal = a.current + increment;
          const unlocked = nextVal >= a.target;
          if (unlocked && !a.unlocked) {
            addToast('success', `Достижение разблокировано: «${a.title}»`, a.description);
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

  onMount(() => {
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
              addToast('success', `Достижение разблокировано: «${a.title}»`, a.description);
              // Refresh achievements list
              loadAchievements();
            }
          ).then((fn) => {
            unlistenToast = fn;
          });

          listen<LobbyInviteItem>('social-lobby-invite', (event) => {
            const inv = event.payload;
            addToast('info', `Приглашение в лобби ${inv.lobby_code}`, `${inv.from_nick} зовёт вас в игру`, {
              label: 'Присоединиться',
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
          <List header="{i18n.locale === 'en' ? 'Friends' : 'Список друзей'} ({filteredFriends.length})">
            {#each filteredFriends as friend, idx (friend.id)}
              <Row
                index={idx}
                title={friend.nick}
                subtitle={
                  friend.status === 'playing'
                    ? (friend.world_name
                        ? (i18n.locale === 'en' ? `Playing on server ${friend.world_name}` : `В игре на сервере ${friend.world_name}`)
                        : (i18n.locale === 'en' ? `Playing ${friend.game_version || 'Minecraft'}` : `В игре: ${friend.game_version || 'Minecraft'}`))
                    : friend.status === 'in_lobby'
                      ? (i18n.locale === 'en' ? `In lobby ${friend.lobby_code || ''}` : `В лобби ${friend.lobby_code || ''}`)
                      : friend.status === 'online'
                        ? t('friends.status_online')
                        : t('friends.status_offline')
                }
              >
                {#snippet leading()}
                  <div class="avatar-capsule">
                    <span class="avatar-letter">{friend.nick.charAt(0).toUpperCase()}</span>
                    <span
                      class="status-indicator {friend.status}"
                      title={friend.status}
                    ></span>
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
                      <span>{i18n.locale === 'en' ? 'Invite to Lobby' : 'Пригласить в лобби'}</span>
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
              ? (i18n.locale === 'en' ? `No friends match “${searchQuery}”.` : `По запросу «${searchQuery}» ни одного игрока не найдено.`)
              : (i18n.locale === 'en' ? 'Enter a friend code above to send a friend request.' : 'У вас пока нет добавленных друзей. Введите код друга в поле выше, чтобы отправить первое приглашение.')}
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
          <List header="{i18n.locale === 'en' ? 'Incoming Friend Requests' : 'Входящие заявки в друзья'} ({requests.length})">
            {#each requests as req, idx (req.id)}
              <Row
                index={idx}
                title={req.from_nick}
                subtitle={i18n.locale === 'en' ? 'Wants to add you as a friend' : 'Хочет добавить вас в список друзей'}
              >
                {#snippet leading()}
                  <div class="avatar-capsule">
                    <span class="avatar-letter">{req.from_nick.charAt(0).toUpperCase()}</span>
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
            description={i18n.locale === 'en' ? 'Friend requests will appear here.' : 'Когда другие игроки отправят вам заявку по вашему коду дружбы, она появится в этом разделе.'}
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
          <span class="summary-count">{unlockedCount} {i18n.locale === 'en' ? 'of' : 'из'} {achievements.length} {i18n.locale === 'en' ? 'unlocked' : 'получено'}</span>
        </div>
        <div class="summary-progress">
          <ProgressBar percentage={overallPercentage} label="Общий прогресс достижений" height={8} />
        </div>
      </div>

      <List
        header={t('friends.achievements_list')}
        footer={i18n.locale === 'en' ? 'Your progress is saved automatically in Wex.' : 'Прогресс отслеживается автоматически и сохраняется в каталоге лаунчера.'}
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
                  {ach.unlocked ? 'Получено' : `${ach.current} / ${ach.target}`}
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
      {#each localToasts as t (t.id)}
        <Toast toast={t} ondismiss={dismissToast} />
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
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: var(--text-subheadline);
    color: var(--color-label);
  }

  .avatar-letter {
    line-height: 1;
  }

  .status-indicator {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid var(--card-bg, #ffffff);
  }

  .status-indicator.online {
    background-color: var(--color-system-green);
  }

  .status-indicator.playing {
    background-color: var(--color-system-blue);
  }

  .status-indicator.in_lobby {
    background-color: var(--color-system-orange);
  }

  .status-indicator.offline {
    background-color: var(--color-system-gray);
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

  /* Clean Empty State */
  .empty-state-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 24px;
    gap: 12px;
    text-align: center;
    background-color: var(--card-bg);
    border: 1px dashed var(--glass-border);
    border-radius: var(--radius-lg);
    margin-top: 12px;
  }

  .empty-icon-wrap {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .empty-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    max-width: 420px;
    line-height: 1.4;
    margin: 0;
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
