import { test, expect, type Page } from '@playwright/test';
import { openApp } from './helpers';

// Real motion, as on a PC with animations on: the bug only shows when entrances animate.
test.use({ reducedMotion: 'no-preference' });

/** Every block of the current page is drawn (not held at the start of its entrance). */
async function pageIsDrawn(page: Page) {
  await expect
    .poll(() =>
      page.evaluate(() =>
        [...document.querySelectorAll('.page > *')].every((el) => Number(getComputedStyle(el).opacity) > 0.99)
      )
    )
    .toBe(true);
}

test('while the game runs, opening other sections shows full pages', async ({ page }) => {
  await openApp(page);
  await page.locator('.bar .play').click();
  await expect(page.locator('.app.game-active')).toBeVisible();
  for (const name of ['Настройки', 'Серверы', 'Гардероб', 'Друзья', 'Главная']) {
    await page.locator('.nav a', { hasText: name }).click();
    await pageIsDrawn(page);
  }
});

test('after hiding to the tray and back, pages are drawn', async ({ page }) => {
  await openApp(page);
  const setHidden = (hidden: boolean) =>
    page.evaluate((h) => {
      Object.defineProperty(document, 'hidden', { configurable: true, get: () => h });
      Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => (h ? 'hidden' : 'visible') });
      document.dispatchEvent(new Event('visibilitychange'));
    }, hidden);
  await setHidden(true);
  await page.locator('.nav a', { hasText: 'Настройки' }).click();
  await setHidden(false);
  await pageIsDrawn(page);
  await page.locator('.nav a', { hasText: 'Серверы' }).click();
  await pageIsDrawn(page);
});
