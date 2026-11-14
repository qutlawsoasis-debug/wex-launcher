import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { accounts } from './accounts.svelte';
import { accountUi } from './accountUi.svelte';
import { launch } from './launch.svelte';
import { router } from './router.svelte';
import { i18n } from '../i18n';
import Sidebar from './Sidebar.svelte';

type Acc = { id: string; kind: string; name: string; uuid: string; needs_sign_in: boolean; last_used_at: null };
let list: Acc[];
let active: string | null;

const overview = () => ({ accounts: list, active_id: active, microsoft_available: false, token_storage: 'dpapi-user' });

beforeEach(async () => {
  mocks.invoke.mockReset();
  i18n.setLocale('ru');
  accountUi.sheetOpen = false;
  list = [
    { id: 'offline-a', kind: 'offline', name: 'Steve', uuid: '1', needs_sign_in: false, last_used_at: null },
    { id: 'ms-b', kind: 'microsoft', name: 'Notch', uuid: '2', needs_sign_in: false, last_used_at: null },
  ];
  active = 'offline-a';
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'accounts_list':
        return overview();
      case 'accounts_switch':
        active = args!.id as string;
        return overview();
      default:
        return null;
    }
  });
  await accounts.load();
});

let target: HTMLElement;
let app: Record<string, unknown> | undefined;
async function render() {
  target = document.createElement('div');
  document.body.appendChild(target);
  app = mount(Sidebar, { target, props: { online: true } });
  flushSync();
  await tick();
}
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  target?.remove();
});

describe('account menu', () => {
  it('the profile opens a menu of accounts, not the wardrobe; picking one switches', async () => {
    router.go('home');
    await render();
    const me = target.querySelector<HTMLButtonElement>('button.me')!;
    expect(me.getAttribute('aria-haspopup')).toBe('menu');
    me.click();
    flushSync();
    expect(router.route).toBe('home');
    const items = [...target.querySelectorAll('[role="menuitemradio"]')];
    expect(items.map((i) => i.querySelector('b')?.textContent)).toEqual(['Steve', 'Notch']);
    expect(items[0].getAttribute('aria-checked')).toBe('true');
    (items[1] as HTMLButtonElement).click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('accounts_switch', { id: 'ms-b' }));
    await vi.waitFor(() => expect(accounts.activeId).toBe('ms-b'));
    expect(target.querySelector('[role="menu"]')).toBeNull();
  });

  it('add opens the sign-in sheet; manage goes to Settings › Accounts; Escape closes', async () => {
    await render();
    const me = target.querySelector<HTMLButtonElement>('button.me')!;
    me.click();
    flushSync();
    const [add, manage] = [...target.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    expect(add.textContent).toBe('Добавить аккаунт…');
    add.click();
    flushSync();
    expect(accountUi.sheetOpen).toBe(true);
    me.click();
    flushSync();
    target.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')[1].click();
    flushSync();
    expect(router.route).toBe('options');
    expect(accountUi.settingsSection).toBe('accounts');
    accountUi.settingsSection = null;
    void manage;
    me.click();
    flushSync();
    target.querySelector('[role="menu"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();
    expect(target.querySelector('[role="menu"]')).toBeNull();
  });

  it('no account: the profile says «Войти» and Play asks to sign in instead of launching', async () => {
    list = [];
    active = null;
    await accounts.load();
    await render();
    const me = target.querySelector<HTMLButtonElement>('button.me')!;
    expect(me.textContent).toContain('Войти');
    me.click();
    flushSync();
    expect(accountUi.sheetOpen).toBe(true);
    accountUi.sheetOpen = false;
    await launch.start();
    expect(accountUi.sheetOpen).toBe(true);
    expect(mocks.invoke).not.toHaveBeenCalledWith('start_game', expect.anything());
  });
});
