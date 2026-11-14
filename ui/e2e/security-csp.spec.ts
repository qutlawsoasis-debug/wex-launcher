import { test, expect } from '@playwright/test';
import { setupMockIpc } from './mock-ipc';

test('CSP blocks injected scripts, eval and remote fetch while the UI loads', async ({ page }) => {
  await page.addInitScript(setupMockIpc, { mode: 'empty' });
  const response = await page.goto('/#options');
  expect(response?.headers()['content-security-policy']).toContain("default-src 'none'");
  await expect(page.locator('.app')).toBeVisible();
  // Evaluate through a normal same-origin script, not DevTools Runtime.evaluate:
  // the latter intentionally bypasses the page's eval CSP restriction.
  await page.route('**/security-probe.js', route => route.fulfill({
    contentType: 'application/javascript',
    body: `
    (async () => {
    const violations = new Set();
    const connectionViolation = new Promise(resolve => {
      document.addEventListener('securitypolicyviolation', event => {
        if (event.effectiveDirective === 'connect-src' && event.blockedURI.startsWith('https://example.invalid')) {
          violations.add(event.effectiveDirective);
          resolve();
        }
      });
    });
    const script = document.createElement('script');
    script.textContent = 'window.__csp_injected = true';
    document.head.append(script);
    let evalBlocked = false;
    try { (0, eval)('1 + 1'); } catch { evalBlocked = true; }
    let networkBlocked = false;
    try { await fetch('https://example.invalid/csp-probe'); } catch { networkBlocked = true; }
    await Promise.race([connectionViolation, new Promise(resolve => setTimeout(resolve, 1000))]);
    window.__csp_result = {
      inlineBlocked: !window.__csp_injected,
      evalBlocked,
      networkBlocked: networkBlocked && violations.has('connect-src'),
    };
    })();`,
  }));
  await page.evaluate(() => {
    const script = document.createElement('script');
    script.src = '/security-probe.js';
    document.head.append(script);
  });
  await page.waitForFunction(() => (window as unknown as { __csp_result?: unknown }).__csp_result);
  const blocked = await page.evaluate(() => (window as unknown as { __csp_result: unknown }).__csp_result);
  expect(blocked).toEqual({ inlineBlocked: true, evalBlocked: true, networkBlocked: true });
});
