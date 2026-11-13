/**
 * Where the account UI opens from: the profile in the sidebar, Play with no
 * account, and the first start. One sign-in sheet for the whole window (App).
 */

export type SignInStep = 'choose' | 'offline' | 'microsoft';

export const accountUi = $state<{
  sheetOpen: boolean;
  start: SignInStep;
  /** Settings scrolls to this section once when it opens (consumed). */
  settingsSection: string | null;
}>({ sheetOpen: false, start: 'choose', settingsSection: null });

export function openSignIn(start: SignInStep = 'choose'): void {
  accountUi.start = start;
  accountUi.sheetOpen = true;
}
