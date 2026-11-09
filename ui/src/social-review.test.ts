import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import App from './App.svelte';
import { friendsLive, FriendsLiveService } from './lib/services/friendsLive';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), handlers: new Map<string, (event: any) => void>(), remove: vi.fn(), action: vi.fn(), focus: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, convertFileSrc: (s: string) => s }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (name: string, handler: any) => { mocks.handlers.set(name, handler); return () => mocks.handlers.delete(name); }) }));
vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ unminimize: mocks.focus, show: mocks.focus, setFocus: mocks.focus }) }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ minimize: vi.fn(), toggleMaximize: vi.fn(), close: vi.fn(), startDragging: vi.fn() }) }));
vi.mock('@tauri-apps/plugin-notification', () => ({ onAction: mocks.action, isPermissionGranted: vi.fn(async () => true), sendNotification: vi.fn() }));

const room = { code: '#AB12', host_name: 'Host', world_name: 'Actual world from IPC', tunnel_address: '127.0.0.1:25565', tunnel_active: true, is_host: false, players: [{ name: 'Alice', is_host: false, ping_ms: 0, mods_synced: true }], mod_mismatch_count: 0, total_mods: 0 };
let target: HTMLDivElement;
let app: ReturnType<typeof mount> | undefined;
let pending: any;

beforeEach(() => {
  vi.clearAllMocks(); mocks.handlers.clear();
  mocks.action.mockResolvedValue({ unregister: mocks.remove });
  pending = null;
  localStorage.clear(); localStorage.setItem('wex_onboarding_completed', 'true');
  history.replaceState({}, '', '/#player=Alice');
  (window as any).__TAURI_INTERNALS__ = {};
  HTMLElement.prototype.animate = vi.fn(() => {
    const animation: any = { cancel: vi.fn(), finish: vi.fn(), effect: { getComputedTiming: () => ({ progress: 1 }) }, currentTime: 0, playState: 'finished', finished: Promise.resolve() };
    setTimeout(() => animation.onfinish?.(), 0);
    return animation;
  });
  mocks.invoke.mockImplementation(async (cmd: string, args?: any) => {
    if (cmd === 'get_pending_deep_link') return pending;
    if (cmd === 'social_sync') return { friends: [], incoming_requests: [], incoming_invites: [] };
    if (cmd === 'shell_get_profile') return { nickname: 'Alice', is_online: true };
    if (cmd === 'shell_check_network') return { is_online: true };
    if (cmd === 'get_launcher_settings') return {};
    if (cmd === 'is_game_running') return false;
    if (cmd === 'join_lobby_room') return room;
    if (cmd === 'social_send_friend_request') {
      if (!args?.targetFriendCode && !args?.request?.target_friend_code) throw new Error('Missing friend code according to Tauri command contract');
      return null;
    }
    return null;
  });
  target = document.createElement('div'); document.body.appendChild(target);
});
afterEach(async () => {
  if (app) { await unmount(app); app = undefined; }
  friendsLive.stop(); target.remove(); delete (window as any).__TAURI_INTERNALS__;
});
async function start() { app = mount(App, { target }); await vi.waitFor(() => expect(mocks.handlers.has('launch-status')).toBe(true)); }
async function confirm() { await vi.waitFor(() => expect(target.querySelector('[role="dialog"]')).not.toBeNull(),{timeout:3000}); const buttons=[...target.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')]; const button=buttons.find(b=>b.textContent?.includes('Confirm test')); expect(button).toBeDefined(); button!.click(); }

describe('social review regressions 19–22', () => {
  it('19: confirmed friend link sends the code under the real camelCase Tauri argument', async () => {
    pending={kind:'friend',title:'Friend',subtitle:'Friend request',target:'#AB12',confirm_text:'Confirm test'};
    await start(); mocks.handlers.get('wex://deep-link')!({payload:pending}); await confirm();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('social_send_friend_request', { targetFriendCode: '#AB12' }));
    expect(target.textContent).not.toContain('Missing friend code');
  });
  it('20: stopping releases the real plugin action listener, including late registration', async () => {
    const service=new FriendsLiveService({syncFn:async()=>({friends:[],incoming_requests:[],incoming_invites:[]})});
    service.start(); await vi.waitFor(()=>expect(mocks.action).toHaveBeenCalled()); service.stop();
    await vi.waitFor(()=>expect(mocks.remove).toHaveBeenCalledTimes(1));
  });
  it('20: a listener registered after stop is immediately unregistered', async () => {
    let resolve!: (listener: any) => void;
    mocks.action.mockReturnValueOnce(new Promise(r=>{resolve=r;}));
    const service=new FriendsLiveService({syncFn:async()=>({friends:[],incoming_requests:[],incoming_invites:[]})});
    service.start(); await vi.waitFor(()=>expect(mocks.action).toHaveBeenCalled());
    service.stop(); resolve({unregister:mocks.remove});
    await vi.waitFor(()=>expect(mocks.remove).toHaveBeenCalledTimes(1));
  });
  it('21: real launch events publish Playing/version and Online on exit', async () => {
    await start();
    mocks.handlers.get('launch-status')!({payload:{state:'running',message:'Running'}});
    await vi.waitFor(()=>expect(mocks.invoke).toHaveBeenCalledWith('social_set_presence', {presence:{status:'Playing',game_version:'1.21.1',world_name:null,lobby_code:null}}));
    mocks.handlers.get('launch-status')!({payload:{state:'idle',message:'Exited'}});
    await vi.waitFor(()=>expect(mocks.invoke).toHaveBeenCalledWith('social_set_presence', {presence:{status:'Online',game_version:null,world_name:null,lobby_code:null}}));
  });
  it('22: notification join uses the current player and renders the returned room', async () => {
    await start();
    await friendsLive.handleNotificationClick({id:'invite',friendId:'host',type:'invite',title:'Invite',description:'Join',lobbyCode:'#AB12'});
    expect(mocks.invoke).toHaveBeenCalledWith('join_lobby_room',{code:'AB12',playerName:'Alice'});
    await vi.waitFor(()=>expect(target.textContent).toContain('Actual world from IPC'));
  });
  it('22: confirmed join link renders the same room returned by IPC', async () => {
    pending={kind:'join',title:'Join',subtitle:'Join room',target:'#AB12',confirm_text:'Confirm test'};
    await start(); mocks.handlers.get('wex://deep-link')!({payload:pending}); await confirm();
    await vi.waitFor(()=>expect(target.textContent).toContain('Actual world from IPC'));
    expect(mocks.invoke.mock.calls.filter(([cmd])=>cmd==='join_lobby_room')).toHaveLength(1);
  });
  it('21: presence retries failures, freezes the actual game version and sends Offline on stop', async () => {
    const service=new FriendsLiveService({syncFn:async()=>({friends:[],incoming_requests:[],incoming_invites:[]})});
    let fail=true;
    const original=mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation(async(cmd: string,args?:any)=>{if(cmd==='social_set_presence'&&fail)throw new Error('presence offline');return original(cmd,args);});
    service.setSessionContext({playerName:'Alice',gameVersion:'1.20.1',lobby:null});
    service.start(); await vi.waitFor(()=>expect(service.lastPresenceError).not.toBeNull());
    fail=false; await service.pollNow(); await vi.waitFor(()=>expect(service.lastPresenceError).toBeNull());
    service.setGameRunning(true); service.setSessionContext({playerName:'Alice',gameVersion:'1.21.1',lobby:null});
    await vi.waitFor(()=>expect(mocks.invoke).toHaveBeenCalledWith('social_set_presence',{presence:{status:'Playing',game_version:'1.20.1',world_name:null,lobby_code:null}}));
    service.stop(); await vi.waitFor(()=>expect(mocks.invoke).toHaveBeenCalledWith('social_set_presence',{presence:{status:'Offline',game_version:null,world_name:null,lobby_code:null}}));
  });
  it('22: a failed join rejects without publishing a room or a fake success', async()=>{
    const service=new FriendsLiveService();const joined=vi.fn();service.onJoin(joined);
    mocks.invoke.mockRejectedValue(new Error('join failed'));
    await expect(service.handleJoin('#AB12')).rejects.toThrow('join failed');
    expect(joined).not.toHaveBeenCalled();
  });
  it('21: leaving the webview attempts Offline publication without opening any native UI', async()=>{
    const service=new FriendsLiveService({syncFn:async()=>({friends:[],incoming_requests:[],incoming_invites:[]})});
    service.start(); await vi.waitFor(()=>expect(mocks.action).toHaveBeenCalled());
    window.dispatchEvent(new Event('beforeunload'));
    await vi.waitFor(()=>expect(mocks.invoke).toHaveBeenCalledWith('social_set_presence',{presence:{status:'Offline',game_version:null,world_name:null,lobby_code:null}}));
    expect(service.running).toBe(false);
  });
});
