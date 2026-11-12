import { expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { setupMockIpc, type MockIpcOptions } from './mock-ipc';

/** Open the app with the IPC stand-in, wait for the splash to finish and the frame to be ready. */
export async function openApp(page: Page, hash = '', opts: MockIpcOptions & { scheme?: 'light' | 'dark' } = { mode: 'full' }) {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  if (opts.scheme) await page.emulateMedia({ colorScheme: opts.scheme });
  await page.addInitScript(setupMockIpc, { mode: opts.mode });
  await page.goto('/' + hash);
  await expect(page.locator('.app')).toBeVisible();
  await expect(page.locator('.bar .play')).toBeVisible();
  await page.waitForFunction(() => !document.querySelector('[class*="splash"]'), null, { timeout: 15000 });
  await expect(page.locator('.lib a')).toHaveCount(2);
  return errors;
}

/** Serious and critical accessibility problems (axe). Contrast is checked by unit tests on tokens. */
export async function axeProblems(page: Page) {
  const r = await new AxeBuilder({ page }).disableRules(['color-contrast']).analyze();
  return r.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical').map((v) => `${v.id}: ${v.nodes.length}`);
}

export async function ipcCalls(page: Page, cmd: string) {
  return page.evaluate((c) => ((window as unknown as { __ipcCalls: Array<{ cmd: string; args: unknown }> }).__ipcCalls ?? []).filter((x) => x.cmd === c).map((x) => x.args), cmd);
}
