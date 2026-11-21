/**
 * Friends sign-in in the UI service (friendsLive): core signs in with the
 * active launcher account; the service follows account switches, retries an
 * unreachable server on its own, and never invents friends.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/plugin-notification', () => ({ onAction: vi.fn(async () => ({ unregister: vi.fn() })) }));
vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ unminimize: vi.fn(), show: vi.fn(), setFocus: vi.fn() }) }));

import { ACTIVE_ACCOUNT_EVENT, FriendsLiveService, lobbyInviteTitle, normalizeStatus, type FriendsConnection, type SyncResponse } from '../../services/friendsLive';
import { ACTIVE_ACCOUNT_EVENT as ACCOUNTS_EVENT, accounts } from '../../shell/accounts.svelte';
import { describeConnection } from './friendsState';
import { i18n } from '../../i18n';

const empty: SyncResponse = { friends: [], incoming_requests: [], incoming_invites: [] };
const signedIn = (nick = 'Player', code = 'K7Q2'): FriendsConnection => ({ state: 'signed_in', nick, friend_code: code, server: 'lobby-api.vercel.app' });

let service: FriendsLiveService | null = null;
beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  mocks.invoke.mockReset();
  i18n.setLocale('ru');
});
afterEach(() => {
  service?.stop();
  service = null;
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('friends sign-in (friendsLive)', () => {
  it('signs in on start, then syncs; nothing is synced before sign-in', async () => {
    const order: string[] = [];
    service = new FriendsLiveService({
      connectFn: async () => (order.push('connect'), signedIn()),
      statusFn: async () => signedIn(),
      syncFn: async () => (order.push('sync'), empty),
    });
    expect(service.connection.state).toBe('connecting');
    service.start();
    await vi.waitFor(() => expect(order).toEqual(['connect', 'sync']));
    expect(service.connection.state).toBe('signed_in');
    expect(service.connection.friend_code).toBe('K7Q2');
  });

  it('without an account it waits for one: no sync, no retries', async () => {
    const connect = vi.fn(async (): Promise<FriendsConnection> => ({ state: 'signed_out', reason: 'no_account' }));
    const sync = vi.fn(async () => empty);
    service = new FriendsLiveService({ connectFn: connect, syncFn: sync });
    service.start();
    await vi.waitFor(() => expect(service!.connection.reason).toBe('no_account'));
    await service.pollNow();
    expect(connect).toHaveBeenCalledTimes(1);
    expect(sync).not.toHaveBeenCalled();
  });

  it('an unreachable server is retried on the next poll without flashing «connecting»', async () => {
    let up = false;
    const seen: string[] = [];
    service = new FriendsLiveService({
      connectFn: async () => (up ? signedIn() : { state: 'unreachable', reason: 'network', server: 'lobby-api.vercel.app' }),
      statusFn: async () => signedIn(),
      syncFn: async () => empty,
    });
    service.onConnection((c) => seen.push(c.state));
    service.start();
    await vi.waitFor(() => expect(service!.connection.state).toBe('unreachable'));
    up = true;
    await service.pollNow();
    expect(service.connection.state).toBe('signed_in');
    expect(seen).toEqual(['unreachable', 'signed_in']);
  });

  it('«Повторить» shows «connecting» and signs in again', async () => {
    let up = false;
    service = new FriendsLiveService({
      connectFn: async () => (up ? signedIn() : { state: 'unusable', reason: 'nick_rejected', detail: 'Nick is taken' }),
      statusFn: async () => signedIn(),
      syncFn: async () => empty,
    });
    service.start();
    await vi.waitFor(() => expect(service!.connection.state).toBe('unusable'));
    await service.pollNow(); // unusable waits for the player
    expect(service.connection.state).toBe('unusable');
    up = true;
    const states: string[] = [];
    service.onConnection((c) => states.push(c.state));
    await service.retry();
    expect(states).toEqual(['connecting', 'signed_in']);
  });

  it('after a sync it picks up what core saw (server down, session ended)', async () => {
    let status: FriendsConnection = signedIn();
    service = new FriendsLiveService({ connectFn: async () => signedIn(), statusFn: async () => status, syncFn: async () => empty });
    service.start();
    await vi.waitFor(() => expect(service!.connection.state).toBe('signed_in'));
    status = { state: 'unreachable', reason: 'timeout', server: 'lobby-api.vercel.app' };
    await service.pollNow();
    expect(service.connection.state).toBe('unreachable');
  });

  it('an account switch drops the old friends and signs in with the new account', async () => {
    let nick = 'First_1';
    const data: SyncResponse[] = [];
    service = new FriendsLiveService({
      connectFn: async () => signedIn(nick, nick === 'First_1' ? 'AAAA' : 'BBBB'),
      statusFn: async () => signedIn(nick, nick === 'First_1' ? 'AAAA' : 'BBBB'),
      syncFn: async () => (nick === 'First_1' ? { ...empty, friends: [{ id: 'u2', nick: 'Old', status: 'online' }] } : empty),
    });
    service.onData((d) => data.push(d));
    service.start();
    await vi.waitFor(() => expect(service!.friends.map((f) => f.nick)).toEqual(['Old']));
    nick = 'Second_2';
    window.dispatchEvent(new CustomEvent(ACTIVE_ACCOUNT_EVENT, { detail: { id: 'offline-Second_2' } }));
    expect(service.friends).toEqual([]);
    await vi.waitFor(() => expect(service!.connection.friend_code).toBe('BBBB'));
    expect(data.at(-1)?.friends).toEqual([]);
  });

  it('the accounts store fires the hook only when the active account changes', async () => {
    expect(ACCOUNTS_EVENT).toBe(ACTIVE_ACCOUNT_EVENT);
    const fired = vi.fn();
    window.addEventListener(ACTIVE_ACCOUNT_EVENT, fired);
    const overview = (active: string | null) => ({
      accounts: [
        { id: 'a', kind: 'offline' as const, name: 'First_1', uuid: '1', needs_sign_in: false, last_used_at: null },
        { id: 'b', kind: 'microsoft' as const, name: 'Second_2', uuid: '2', needs_sign_in: false, last_used_at: null },
      ],
      active_id: active,
      microsoft_available: true,
      token_storage: 'dpapi-user',
    });
    accounts.loaded = false;
    accounts.apply(overview('a')); // first load: friendsLive signs in on start anyway
    accounts.apply(overview('a'));
    expect(fired).not.toHaveBeenCalled();
    accounts.apply(overview('b'));
    accounts.apply(overview(null));
    expect(fired).toHaveBeenCalledTimes(2);
    window.removeEventListener(ACTIVE_ACCOUNT_EVENT, fired);
  });

  it('core statuses ("Playing", "InLobby") become the UI statuses', async () => {
    expect(['Online', 'Playing', 'InLobby', 'Offline', 'in_lobby', 'weird'].map(normalizeStatus)).toEqual(['online', 'playing', 'in_lobby', 'offline', 'in_lobby', 'offline']);
    service = new FriendsLiveService({
      connectFn: async () => signedIn(),
      statusFn: async () => signedIn(),
      syncFn: async () => ({ ...empty, friends: [{ id: 'u2', nick: 'Snow', status: 'Playing' as never }] }),
    });
    service.start();
    await vi.waitFor(() => expect(service!.friends[0]?.status).toBe('playing'));
  });

  it('an invite without a room code is accepted with core, then joins the code it returns', async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'social_respond_lobby_invite') return 'ROOM7';
      if (cmd === 'join_lobby_room') return { code: 'ROOM7' };
      return null;
    });
    service = new FriendsLiveService({ connectFn: async () => signedIn(), syncFn: async () => empty });
    await service.handleNotificationClick({ id: 'n', friendId: 'u2', type: 'invite', title: 'Invite', description: 'Join', inviteId: 'inv1' });
    expect(mocks.invoke).toHaveBeenCalledWith('social_respond_lobby_invite', { inviteId: 'inv1', accept: true });
    expect(mocks.invoke).toHaveBeenCalledWith('join_lobby_room', { code: 'ROOM7', playerName: '' });
  });

  it('a new invite without a code notifies with an action that accepts it', async () => {
    let round = 0;
    const toasts: Array<{ inviteId?: string; lobbyCode?: string }> = [];
    service = new FriendsLiveService({
      connectFn: async () => signedIn(),
      statusFn: async () => signedIn(),
      syncFn: async () =>
        round++ === 0 ? empty : { ...empty, incoming_invites: [{ id: 'inv1', lobby_code: '', from_id: 'u2', from_nick: 'Snow' }] },
    });
    service.onNotification((n) => toasts.push(n));
    service.onWindowsNotification((n) => toasts.push(n));
    service.start();
    await vi.waitFor(() => expect(round).toBe(1));
    await service.pollNow();
    await vi.waitFor(() => expect(toasts.length).toBeGreaterThan(0));
    expect(toasts[0].inviteId).toBe('inv1');
    expect(toasts[0].lobbyCode).toBeUndefined();
  });

  it('an invite notification with a code also carries the invite id', async () => {
    let round = 0;
    const notes: Array<{ inviteId?: string; lobbyCode?: string }> = [];
    service = new FriendsLiveService({
      connectFn: async () => signedIn(),
      statusFn: async () => signedIn(),
      syncFn: async () =>
        round++ === 0 ? empty : { ...empty, incoming_invites: [{ id: 'inv2', lobby_code: 'ROOM2', from_id: 'u2', from_nick: 'Snow' }] },
    });
    service.onNotification((n) => notes.push(n));
    service.onWindowsNotification((n) => notes.push(n));
    service.start();
    await vi.waitFor(() => expect(round).toBe(1));
    await service.pollNow();
    await vi.waitFor(() => expect(notes.length).toBeGreaterThan(0));
    expect(notes[0]).toMatchObject({ inviteId: 'inv2', lobbyCode: 'ROOM2' });
  });

  it('an OS notification for an invite no longer in the list answers the server first, then joins', async () => {
    const order: string[] = [];
    mocks.invoke.mockImplementation(async (cmd: string) => {
      order.push(cmd);
      if (cmd === 'social_respond_lobby_invite') return 'ROOM3';
      if (cmd === 'join_lobby_room') return { code: 'ROOM3' };
      return null;
    });
    service = new FriendsLiveService({ connectFn: async () => signedIn(), syncFn: async () => empty });
    expect(service.invites).toEqual([]);
    await service.handleNotificationClick({ id: 'n', friendId: 'u2', type: 'invite', title: 'Invite', description: 'Join', lobbyCode: 'ROOM3', inviteId: 'gone1' });
    expect(mocks.invoke).toHaveBeenCalledWith('social_respond_lobby_invite', { inviteId: 'gone1', accept: true });
    expect(order.filter((c) => c === 'social_respond_lobby_invite' || c === 'join_lobby_room')).toEqual(['social_respond_lobby_invite', 'join_lobby_room']);
  });

  it('a server failure on that answer still joins by the code the notification carried', async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'social_respond_lobby_invite') throw new Error('offline');
      if (cmd === 'join_lobby_room') return { code: 'ROOM3' };
      return null;
    });
    vi.spyOn(console, 'warn').mockImplementationOnce(() => {});
    service = new FriendsLiveService({ connectFn: async () => signedIn(), syncFn: async () => empty });
    await service.handleNotificationClick({ id: 'n', friendId: 'u2', type: 'invite', title: 'Invite', description: 'Join', lobbyCode: 'ROOM3', inviteId: 'gone1' });
    expect(mocks.invoke).toHaveBeenCalledWith('join_lobby_room', { code: 'ROOM3', playerName: '' });
  });

  it('with only a room code and no known invite it joins by code and logs it', async () => {
    const info = vi.spyOn(console, 'info').mockImplementation(() => {});
    mocks.invoke.mockImplementation(async (cmd: string) => (cmd === 'join_lobby_room' ? { code: 'ROOM4' } : null));
    service = new FriendsLiveService({ connectFn: async () => signedIn(), syncFn: async () => empty });
    await service.handleNotificationClick({ id: 'n', friendId: 'u2', type: 'invite', title: 'Invite', description: 'Join', lobbyCode: '#room4' });
    expect(mocks.invoke).not.toHaveBeenCalledWith('social_respond_lobby_invite', expect.anything());
    expect(mocks.invoke).toHaveBeenCalledWith('join_lobby_room', { code: 'ROOM4', playerName: '' });
    expect(info).toHaveBeenCalledWith(expect.stringContaining('ROOM4'));
    info.mockRestore();
  });

  it('a failed answer with a known code is reported, still joins, and later syncs keep the invite hidden', async () => {
    const invite = { id: 'inv5', lobby_code: 'ROOM5', from_id: 'u2', from_nick: 'Snow' };
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'social_respond_lobby_invite') throw new Error('respond offline');
      if (cmd === 'join_lobby_room') return { code: 'ROOM5' };
      return null;
    });
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    service = new FriendsLiveService({ connectFn: async () => signedIn(), statusFn: async () => signedIn(), syncFn: async () => ({ ...empty, incoming_invites: [invite] }) });
    const errors: Array<[unknown, string]> = [];
    service.onError((e, c) => errors.push([e, c]));
    await service.pollNow();
    expect(service.invites.map((i) => i.id)).toEqual(['inv5']);
    await service.joinInvite(service.invites[0]);
    expect(mocks.invoke).toHaveBeenCalledWith('join_lobby_room', { code: 'ROOM5', playerName: '' });
    expect(errors).toHaveLength(1);
    expect(errors[0][1]).toBe('friends');
    expect(String(errors[0][0])).toContain('respond offline');
    await service.pollNow();
    expect(service.invites).toEqual([]);
    warn.mockRestore();
  });

  it('a poll in flight when Join is clicked does not bring the answered invite back', async () => {
    const invite = { id: 'inv6', lobby_code: 'ROOM6', from_id: 'u2', from_nick: 'Snow' };
    let release!: (r: SyncResponse) => void;
    let calls = 0;
    mocks.invoke.mockImplementation(async (cmd: string) => (cmd === 'social_respond_lobby_invite' ? 'ROOM6' : cmd === 'join_lobby_room' ? { code: 'ROOM6' } : null));
    service = new FriendsLiveService({
      connectFn: async () => signedIn(),
      statusFn: async () => signedIn(),
      syncFn: () => (calls++ === 0 ? Promise.resolve({ ...empty, incoming_invites: [invite] }) : new Promise<SyncResponse>((r) => (release = r))),
    });
    await service.pollNow();
    const inFlight = service.pollNow();
    await vi.waitFor(() => expect(release).toBeDefined());
    await service.joinInvite(service.invites[0]);
    expect(service.invites).toEqual([]);
    release({ ...empty, incoming_invites: [invite] });
    await inFlight;
    expect(service.invites).toEqual([]);
  });

  it('a poll that started before an account switch is dropped', async () => {
    let release!: (r: SyncResponse) => void;
    let calls = 0;
    service = new FriendsLiveService({
      connectFn: async () => signedIn(),
      statusFn: async () => signedIn(),
      // Later polls never answer: only the stale first result could change the lists.
      syncFn: () => (calls++ === 0 ? new Promise<SyncResponse>((r) => (release = r)) : new Promise<SyncResponse>(() => {})),
    });
    service.start();
    await vi.waitFor(() => expect(release).toBeDefined());
    window.dispatchEvent(new Event(ACTIVE_ACCOUNT_EVENT));
    release({ ...empty, friends: [{ id: 'old', nick: 'OldFriend', status: 'online' }], incoming_invites: [{ id: 'invOld', lobby_code: 'OLD1', from_id: 'x', from_nick: 'X' }] });
    await vi.waitFor(() => expect(calls).toBeGreaterThanOrEqual(2));
    await new Promise((r) => setTimeout(r, 0));
    expect(service.friends).toEqual([]);
    expect(service.invites).toEqual([]);
  });

  it('page, toast and OS notification joining the same invite share one answer and one join', async () => {
    const invite = { id: 'inv7', lobby_code: 'ROOM7', from_id: 'u2', from_nick: 'Snow' };
    let answer!: (code: string) => void;
    mocks.invoke.mockImplementation((cmd: string) => {
      if (cmd === 'social_respond_lobby_invite') return new Promise((r) => (answer = r));
      if (cmd === 'join_lobby_room') return Promise.resolve({ code: 'ROOM7' });
      return Promise.resolve(null);
    });
    service = new FriendsLiveService({ connectFn: async () => signedIn(), statusFn: async () => signedIn(), syncFn: async () => ({ ...empty, incoming_invites: [invite] }) });
    await service.pollNow();
    const page = service.joinInvite(service.invites[0]);
    const toast = service.joinInvite({ ...invite });
    const os = service.handleNotificationClick({ id: 'n', friendId: 'u2', type: 'invite', title: 'Invite', description: 'Join', lobbyCode: 'ROOM7', inviteId: 'inv7' });
    expect(toast).toBe(page);
    await vi.waitFor(() => expect(answer).toBeDefined());
    // Let the notification click reach joinInvite while the answer is pending.
    await new Promise((r) => setTimeout(r, 20));
    answer('ROOM7');
    await Promise.all([page, toast, os]);
    const count = (cmd: string) => mocks.invoke.mock.calls.filter(([c]) => c === cmd).length;
    expect(count('social_respond_lobby_invite')).toBe(1);
    expect(count('join_lobby_room')).toBe(1);
  });

  it('the invite toast title has no dangling code when the server sent none', () => {
    expect(lobbyInviteTitle('AB12')).toBe('Приглашение в лобби AB12');
    expect(lobbyInviteTitle('')).toBe('Приглашение в лобби');
    expect(lobbyInviteTitle(null)).toBe('Приглашение в лобби');
    i18n.setLocale('en');
    expect(lobbyInviteTitle('  ')).toBe('Lobby invite');
  });
});

describe('friends state words', () => {
  it('each state has a true message; signed in has none', () => {
    expect(describeConnection(signedIn())).toBeNull();
    expect(describeConnection({ state: 'connecting' })?.busy).toBe(true);
    expect(describeConnection({ state: 'signed_out', reason: 'no_account' })?.title).toBe('Войдите в аккаунт, чтобы видеть друзей.');
    const down = describeConnection({ state: 'unreachable', reason: 'no_friends_api', server: 'example.org' })!;
    expect(down.title).toBe('Сервер друзей недоступен.');
    expect(down.text).toContain('example.org');
    expect(down.retry).toBe(true);
    expect(describeConnection({ state: 'unreachable', reason: 'something_new' })!.text).not.toMatch(/friends2\./);
    const refused = describeConnection({ state: 'unusable', reason: 'nick_rejected', nick: 'Steve', detail: 'Nick is taken' })!;
    expect(refused.text).toContain('Steve');
    expect(refused.text).toContain('Nick is taken');
    expect(describeConnection({ state: 'not_configured', reason: 'disabled' })!.retry).toBe(false);
    i18n.setLocale('en');
    expect(describeConnection({ state: 'unreachable', reason: 'timeout', server: 'example.org' })!.title).not.toMatch(/[А-я]|friends2\./);
  });
});
