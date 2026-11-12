import { test, expect, type Page } from '@playwright/test';
import { openApp } from './helpers';

const sample = (page: Page) =>
  page.evaluate(() => ({
    turn: getComputedStyle(document.querySelector('.rig')!).getPropertyValue('--turn'),
    arm: getComputedStyle(document.querySelector('.j-armR .swing')!).transform,
    leg: getComputedStyle(document.querySelector('.j-legL .swing')!).transform,
  }));

async function modelMoves(page: Page) {
  await openApp(page, '#wardrobe');
  const a = await sample(page);
  await page.waitForTimeout(500);
  const b = await sample(page);
  expect(b.turn, 'turns slowly by itself until touched').not.toBe(a.turn);
  await page.locator('.stage-bar .seg button', { hasText: 'Идёт' }).click();
  await page.waitForTimeout(200);
  const w1 = await sample(page);
  await page.waitForTimeout(250);
  const w2 = await sample(page);
  expect(w2.arm, 'walking swings the arms').not.toBe(w1.arm);
  expect(w2.leg, 'walking swings the legs').not.toBe(w1.leg);
  // Touching the model ends the idle spin.
  const box = (await page.locator('.stage3d').boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2);
  await page.mouse.up();
  const t1 = await sample(page);
  await page.waitForTimeout(400);
  expect((await sample(page)).turn).toBe(t1.turn);
}

test.describe('motion on', () => {
  test.use({ reducedMotion: 'no-preference' });
  test('wardrobe: poses animate, idle spin until touched', async ({ page }) => {
    await modelMoves(page);
  });
});

test.describe('Windows animations off, Animations turned on in Wex', () => {
  test.use({ reducedMotion: 'reduce' });
  test('the app switch wins: the model still moves', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('wex_motion_on', '1'));
    await modelMoves(page);
    expect(await page.evaluate(() => document.documentElement.dataset.reduceMotion)).toBe('false');
  });
});
