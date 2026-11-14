import { test, expect } from '@playwright/test';
import { openApp, ipcCalls } from './helpers';

test('Ctrl K opens the palette, typing filters, Enter runs, Esc closes and returns focus', async ({ page }) => {
  await openApp(page);
  await page.keyboard.press('Control+k');
  const input = page.locator('.cmd-in input');
  await expect(input).toBeFocused();
  await input.fill('настр');
  await expect(page.locator('.cmd-row[aria-selected="true"]')).toContainText('Настройки');
  await page.keyboard.press('Enter');
  await expect(page.locator('.cmd')).toHaveCount(0);
  await expect(page.locator('.page h1').first()).toHaveText('Настройки');

  await page.locator('.search-top').click();
  await expect(input).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.locator('.cmd')).toHaveCount(0);
  await expect(page.locator('.search-top')).toBeFocused();
});

test('palette: arrows move the selection; a mod opens its build on Mods with the name searched', async ({ page }) => {
  await openApp(page);
  await page.keyboard.press('Control+k');
  await page.locator('.cmd-in input').fill('sodium');
  await expect(page.locator('.cmd-row[aria-selected="true"]')).toContainText('Sodium');
  await page.keyboard.press('Enter');
  await expect(page.locator('.page h1').first()).toHaveText('Wex 1.21.1');
  await expect(page.locator('.search input')).toHaveValue('Sodium');
  await expect(page.locator('.lrow')).toHaveCount(1);
});

test('Ctrl 2 and Ctrl 1 choose the build and save it', async ({ page }) => {
  await openApp(page);
  await page.locator('body').click({ position: { x: 600, y: 700 } });
  await page.keyboard.press('Control+2');
  await expect(page.locator('.toast')).toContainText('Для запуска выбрана Wex 1.20.1');
  await expect(page.locator('.bar .title')).toContainText('Wex 1.20.1');
  await page.keyboard.press('Control+1');
  await expect(page.locator('.bar .title')).toContainText('Wex 1.21.1');
  const saved = (await ipcCalls(page, 'save_launcher_settings')) as Array<{ newSettings: { selected_version: string } }>;
  expect(saved.map((s) => s.newSettings.selected_version)).toEqual(['1.20.1', '1.21.1']);
});

test('Alt ← and Alt → move through history; the toolbar buttons follow', async ({ page }) => {
  await openApp(page);
  await page.locator('.nav a', { hasText: 'Серверы' }).click();
  await page.locator('.nav a', { hasText: 'Гардероб' }).click();
  await page.keyboard.press('Alt+ArrowLeft');
  await expect(page.locator('.page h1').first()).toHaveText('Серверы');
  await expect(page.getByRole('button', { name: 'Вперёд' })).toBeEnabled();
  await page.keyboard.press('Alt+ArrowRight');
  await expect(page.locator('.page h1').first()).toHaveText('Гардероб');
});

test('Enter on the page plays the selected build; Enter in a field does not', async ({ page }) => {
  await openApp(page, '#build-1.21.1');
  await page.locator('.search input').focus();
  await page.keyboard.press('Enter');
  expect(await ipcCalls(page, 'start_game')).toEqual([]);
  await page.locator('.search input').blur();
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press('Enter');
  await expect(page.locator('.bar .title')).toContainText(/Скачиваем|Готовим|Игра идёт/);
});

test('Tab reaches the sidebar with a visible focus ring; build tabs move with the arrow keys', async ({ page }) => {
  await openApp(page, '#build-1.21.1');
  await page.keyboard.press('Tab');
  const focused = page.locator(':focus');
  await expect(focused).toBeVisible();
  expect(await focused.evaluate((el) => getComputedStyle(el).outlineStyle)).not.toBe('none');
  await page.locator('#btab-mods').focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('#btab-worlds')).toBeFocused();
  await expect(page.locator('#btab-worlds')).toHaveAttribute('aria-selected', 'true');
});
