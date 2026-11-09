import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import FriendsScreen from './FriendsScreen.svelte';
import { network } from '../design/network.svelte';

// Mock Tauri invoke & event
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe('FriendsScreen Component Tests', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    network.isOnline = true;
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
    network.isOnline = true;
  });

  // 1. Empty States
  it('renders authentic empty state on Friends tab when friends list is empty', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: [],
          incoming_requests: [],
          incoming_invites: [],
        });
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Список друзей пуст');
    expect(target.textContent).toContain('У вас пока нет добавленных друзей');

    unmount(comp);
  });

  it('renders authentic empty state on Requests tab when incoming requests list is empty', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: [],
          incoming_requests: [],
          incoming_invites: [],
        });
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'requests' } });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Нет входящих заявок');
    expect(target.textContent).toContain('Когда другие игроки отправят вам заявку');

    unmount(comp);
  });

  // 2. Loading / Submitting States
  it('disables add button and sets loading state while sending friend request', async () => {
    let finishRequest: (val: any) => void = () => {};
    const pendingPromise = new Promise((resolve) => {
      finishRequest = resolve;
    });

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({ friends: [], incoming_requests: [], incoming_invites: [] });
      }
      if (cmd === 'social_send_friend_request') {
        return pendingPromise;
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 50));

    const input = target.querySelector('.friend-code-input') as HTMLInputElement;
    input.value = '#TEST1';
    input.dispatchEvent(new Event('input'));
    await tick();

    const addBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Добавить')
    );
    expect(addBtn).toBeDefined();
    addBtn?.click();
    await new Promise((r) => setTimeout(r, 20));

    // Verify loading indicator is active
    expect(addBtn?.getAttribute('aria-busy')).toBe('true');

    // Finish
    finishRequest({});
    await new Promise((r) => setTimeout(r, 50));
    unmount(comp);
  });

  // 3. Error States
  it('renders error notification banner and toast when sending request fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({ friends: [], incoming_requests: [], incoming_invites: [] });
      }
      if (cmd === 'social_send_friend_request') {
        return Promise.reject(new Error('Пользователь с таким кодом не найден'));
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 50));

    const input = target.querySelector('.friend-code-input') as HTMLInputElement;
    input.value = '#FAIL1';
    input.dispatchEvent(new Event('input'));
    await tick();

    const addBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Добавить')
    );
    addBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    // Error banner and toast should be rendered
    const errorBanner = target.querySelector('.social-error-banner');
    expect(errorBanner).not.toBeNull();
    const toast = target.querySelector('.apple-toast');
    expect(toast).not.toBeNull();

    unmount(comp);
  });

  it('renders OfflineState when network is offline', async () => {
    network.isOnline = false;
    const comp = mount(FriendsScreen, { target });
    await tick();

    expect(target.textContent).toContain('Нет подключения');
    unmount(comp);
  });

  // 4. List Populated from IPC Mock
  it('renders friends list from social_sync IPC command with status and activity', async () => {
    const mockFriends = [
      {
        id: 'user-1',
        nick: 'Alex',
        status: 'playing',
        game_version: '1.21.1',
        world_name: 'Выживание в тайге',
      },
      {
        id: 'user-2',
        nick: 'Steve',
        status: 'offline',
        last_seen: Date.now() - 3600000,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: mockFriends,
          incoming_requests: [],
          incoming_invites: [],
        });
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Alex');
    expect(target.textContent).toContain('В игре');
    expect(target.textContent).toContain('Steve');
    expect(target.textContent).toContain('Не в сети');

    unmount(comp);
  });

  it('renders incoming requests list from social_sync IPC command with accept/decline buttons', async () => {
    const mockRequests = [
      {
        id: 'req-101',
        from_id: 'user-99',
        from_nick: 'MinerPro',
        created_at: Date.now(),
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: [],
          incoming_requests: mockRequests,
          incoming_invites: [],
        });
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'requests' } });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('MinerPro');
    expect(target.textContent).toContain('Принять');
    expect(target.textContent).toContain('Отклонить');

    unmount(comp);
  });

  it('renders achievements list from social_list_achievements IPC command', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({ friends: [], incoming_requests: [], incoming_invites: [] });
      }
      if (cmd === 'social_list_achievements') {
        return Promise.resolve({
          achievements: [
            { id: 'first_launch', title: 'Искра', description: 'Первый запуск игры', target: 1 },
            { id: 'ten_launches', title: 'Постоялец', description: '10 запусков игры', target: 10 },
          ],
          progress: [
            { id: 'first_launch', current: 1, unlocked: true, unlocked_at: '2026-11-08T12:00:00Z' },
            { id: 'ten_launches', current: 4, unlocked: false, unlocked_at: null },
          ],
        });
      }
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'achievements' } });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Искра');
    expect(target.textContent).toContain('Первый запуск игры');
    expect(target.textContent).toContain('Постоялец');
    expect(target.textContent).toContain('4 / 10');

    unmount(comp);
  });

  // 5. Button Clicks Call Expected IPC Commands With Arguments
  it('calls social_send_friend_request with trimmed target_friend_code when Add Friend is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({ friends: [], incoming_requests: [], incoming_invites: [] });
      }
      if (cmd === 'social_send_friend_request') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 50));

    const input = target.querySelector('.friend-code-input') as HTMLInputElement;
    input.value = '#A7X9';
    input.dispatchEvent(new Event('input'));
    await tick();

    const addBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Добавить')
    );
    expect(addBtn).toBeDefined();
    addBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('social_send_friend_request', {
      target_friend_code: '#A7X9',
    });

    unmount(comp);
  });

  it('calls social_accept_friend_request with request_id when Accept button is clicked', async () => {
    const mockRequests = [
      {
        id: 'req-42',
        from_id: 'user-77',
        from_nick: 'BuilderAlex',
        created_at: Date.now(),
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: [],
          incoming_requests: mockRequests,
          incoming_invites: [],
        });
      }
      if (cmd === 'social_accept_friend_request') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'requests' } });
    await new Promise((r) => setTimeout(r, 60));

    const acceptBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Принять')
    );
    expect(acceptBtn).toBeDefined();
    acceptBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('social_accept_friend_request', {
      request_id: 'req-42',
    });

    unmount(comp);
  });

  it('calls social_decline_friend_request with request_id when Decline button is clicked', async () => {
    const mockRequests = [
      {
        id: 'req-55',
        from_id: 'user-88',
        from_nick: 'GrieferBot',
        created_at: Date.now(),
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: [],
          incoming_requests: mockRequests,
          incoming_invites: [],
        });
      }
      if (cmd === 'social_decline_friend_request') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'requests' } });
    await new Promise((r) => setTimeout(r, 60));

    const declineBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Отклонить')
    );
    expect(declineBtn).toBeDefined();
    declineBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('social_decline_friend_request', {
      request_id: 'req-55',
    });

    unmount(comp);
  });

  it('calls social_send_lobby_invite with target_user_id and lobby_code when Invite to Lobby is clicked', async () => {
    const mockFriends = [
      {
        id: 'user-friend-10',
        nick: 'ActiveFriend',
        status: 'online',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'social_sync') {
        return Promise.resolve({
          friends: mockFriends,
          incoming_requests: [],
          incoming_invites: [],
        });
      }
      if (cmd === 'social_send_lobby_invite') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(FriendsScreen, { target, props: { tab: 'friends' } });
    await new Promise((r) => setTimeout(r, 60));

    const inviteBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Пригласить в лобби')
    );
    expect(inviteBtn).toBeDefined();
    inviteBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('social_send_lobby_invite', {
      target_user_id: 'user-friend-10',
      lobby_code: '#A7X9',
    });

    unmount(comp);
  });
});
