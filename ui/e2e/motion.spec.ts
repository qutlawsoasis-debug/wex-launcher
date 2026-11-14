import { test, expect, type Page } from '@playwright/test';
import { openApp } from './helpers';

const sample = (page: Page) =>
  page.evaluate(() => ({
    turn: getComputedStyle(document.querySelector('.rig')!).getPropertyValue('--turn'),
    // The WebGL model (skinview3d): a frame of the canvas.
    frame: (document.querySelector('canvas.gl.on') as HTMLCanvasElement | null)?.toDataURL() ?? '',
  }));

async function modelMoves(page: Page) {
  await openApp(page, '#wardrobe');
  await expect(page.locator('canvas.gl.on'), 'the WebGL model is drawn').toBeVisible();
  const a = await sample(page);
  await page.waitForTimeout(500);
  const b = await sample(page);
  expect(b.turn, 'turns slowly by itself until touched').not.toBe(a.turn);
  await page.locator('.stage-bar .seg button', { hasText: 'Идёт' }).click();
  await page.waitForTimeout(200);
  const w1 = await sample(page);
  await page.waitForTimeout(250);
  const w2 = await sample(page);
  expect(w2.frame, 'walking moves the arms and legs').not.toBe(w1.frame);
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

const ver = (page: Page) =>
  page.evaluate(() => {
    const pair = document.querySelector('.pair') as HTMLElement;
    return { x: Number(pair.style.getPropertyValue('--px') || 0), y: Number(pair.style.getPropertyValue('--py') || 0) };
  });

test.describe('home: version digits parallax', () => {
  test.use({ reducedMotion: 'no-preference' });
  test('follows the pointer over the whole page, smoothly, and drifts back when it leaves', async ({ page }) => {
    await openApp(page);
    const main = (await page.locator('main.main').boundingBox())!;
    const pane = (await page.locator('.pane.open').boundingBox())!;
    // Far below the pane, still on Home: the digits react.
    await page.mouse.move(main.x + main.width - 20, pane.y + pane.height + 120);
    const first = await ver(page);
    await page.waitForTimeout(600);
    const end = await ver(page);
    expect(end.x, 'moves to the right edge').toBeGreaterThan(0.3);
    expect(end.y, 'moves down').toBeGreaterThan(0);
    expect(first.x, 'not in one jump').toBeLessThan(end.x);
    // Off the page (over the sidebar): back to the centre, smoothly.
    await page.mouse.move(main.x - 40, main.y + 100);
    await page.waitForTimeout(32);
    const mid = await ver(page);
    expect(mid.x).toBeGreaterThan(0);
    await page.waitForTimeout(800);
    const back = await ver(page);
    expect(Math.abs(back.x)).toBeLessThan(0.01);
    expect(Math.abs(back.y)).toBeLessThan(0.01);
  });
});

test.describe('home: parallax under Reduce Motion', () => {
  test.use({ reducedMotion: 'reduce' });
  test('the digits stay put', async ({ page }) => {
    await openApp(page);
    const main = (await page.locator('main.main').boundingBox())!;
    await page.mouse.move(main.x + main.width - 20, main.y + main.height - 20);
    await page.waitForTimeout(400);
    expect(await ver(page)).toEqual({ x: 0, y: 0 });
  });
});
