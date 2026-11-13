/**
 * Player accounts, as core keeps them (commands/accounts.rs): Microsoft
 * (licensed) and offline. Core owns storage and tokens; the UI only sees names,
 * UUIDs and which account is active. Switching updates the shell's player name.
 */

import { call } from './ipc';
import { builds } from './builds.svelte';
import { uiLog } from './uiLog';

/** Window event fired when the active account changes (friendsLive listens). */
export const ACTIVE_ACCOUNT_EVENT = 'wex-active-account';

export type AccountKind = 'offline' | 'microsoft';

export interface Account {
  id: string;
  kind: AccountKind;
  name: string;
  uuid: string;
  needs_sign_in: boolean;
  last_used_at: string | null;
}

export interface AccountsOverview {
  accounts: Account[];
  active_id: string | null;
  microsoft_available: boolean;
  token_storage: string;
}

export interface DeviceCode {
  flow_id: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
}

/** Vanilla offline name rule: 3–16 Latin letters, digits or _. */
export const OFFLINE_NAME_RE = /^[A-Za-z0-9_]{3,16}$/;

class Accounts {
  list = $state<Account[]>([]);
  activeId = $state<string | null>(null);
  microsoftAvailable = $state(false);
  tokenStorage = $state('');
  loaded = $state(false);
  error = $state<unknown>(null);

  get active(): Account | undefined {
    return this.list.find((a) => a.id === this.activeId);
  }

  apply(o: AccountsOverview): void {
    const before = this.loaded ? this.activeId : undefined;
    this.list = o.accounts ?? [];
    this.activeId = o.active_id ?? null;
    this.microsoftAvailable = !!o.microsoft_available;
    this.tokenStorage = o.token_storage ?? '';
    const active = this.active;
    if (active) builds.playerName = active.name;
    this.loaded = true;
    this.error = null;
    // Hook: the active account changed (added, switched, signed out). Friends sign in again.
    if (before !== undefined && before !== this.activeId && typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent(ACTIVE_ACCOUNT_EVENT, { detail: { id: this.activeId } }));
    }
  }

  async load(): Promise<void> {
    try {
      this.apply(await call<AccountsOverview>('accounts_list'));
    } catch (e) {
      this.error = e;
      this.loaded = true;
      uiLog('warn', `accounts_list failed: ${String(e)}`);
    }
  }

  async addOffline(name: string): Promise<void> {
    this.apply(await call<AccountsOverview>('accounts_add_offline', { name: name.trim() }));
  }

  async switchTo(id: string): Promise<void> {
    if (id === this.activeId) return;
    this.apply(await call<AccountsOverview>('accounts_switch', { id }));
  }

  async signOut(id: string): Promise<void> {
    this.apply(await call<AccountsOverview>('accounts_sign_out', { id }));
  }

  beginMicrosoft(): Promise<DeviceCode> {
    return call<DeviceCode>('accounts_ms_begin');
  }

  /** Resolves when the player has finished on the Microsoft page and the account is saved. */
  async finishMicrosoft(flowId: string): Promise<void> {
    this.apply(await call<AccountsOverview>('accounts_ms_finish', { flowId }));
  }

  cancelMicrosoft(flowId: string): Promise<void> {
    return call('accounts_ms_cancel', { flowId });
  }

  openMicrosoftPage(flowId: string): Promise<void> {
    return call('accounts_ms_open_page', { flowId });
  }
}

export const accounts = new Accounts();
