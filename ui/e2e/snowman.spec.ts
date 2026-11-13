import { test, expect, type Page } from '@playwright/test';
import { openApp } from './helpers';

test.use({ reducedMotion: 'reduce', viewport: { width: 1600, height: 900 } });

/** Press on the snow at x and roll back and forth for `dist` px. */
async function roll(page: Page, x: number, y: number, dist: number) {
  await page.mouse.move(x, y);
  await page.mouse.down();
  let done = 0;
  let dir = 1;
  while (done < dist) {
    const step = Math.min(40, dist - done);
    x += step * dir;
    await page.mouse.move(x, y, { steps: 2 });
    done += step;
    if (done % 200 === 0) dir = -dir;
  }
  await page.mouse.up();
}

const balls = (page: Page) =>
  page.locator('.pane.open .ball').evaluateAll((els) =>
    els.map((e) => {
      const r = e.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, d: r.width };
    })
  ).then((list) => list.sort((p, q) => q.d - p.d)); // biggest first

async function carry(page: Page, from: { x: number; y: number }, to: { x: number; y: number }) {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move((from.x + to.x) / 2, to.y - 80, { steps: 6 });
  await page.mouse.move(to.x, to.y, { steps: 6 });
  await page.mouse.up();
}

test('winter: roll three snowballs of different sizes on the snow and stack them into a snowman', async ({ page }) => {
  await openApp(page, '#home');
  await page.evaluate(() => localStorage.removeItem('wex_snowman_1.21.1'));
  const pane = (await page.locator('.pane.open').boundingBox())!;
  const y = pane.y + pane.height - 8;
  await roll(page, pane.x + pane.width * 0.55, y, 600);
  await roll(page, pane.x + pane.width * 0.75, y, 260);
  await roll(page, pane.x + pane.width * 0.9, y, 80);
  let b = await balls(page);
  expect(b.length).toBe(3);
  expect(b[0].d).toBeGreaterThan(b[1].d);
  expect(b[1].d).toBeGreaterThan(b[2].d);
  // Pick up the middle ball and put it on the big one, then the small one on top.
  await carry(page, b[1], { x: b[0].x, y: b[0].y - b[0].d });
  b = await balls(page);
  await carry(page, b[2], { x: b[0].x, y: b[1].y - b[1].d });
  b = await balls(page);
  expect(Math.abs(b[1].x - b[0].x)).toBeLessThan(2);
  expect(Math.abs(b[2].x - b[0].x)).toBeLessThan(2);
  expect(b[2].y).toBeLessThan(b[1].y);
  expect(b[1].y).toBeLessThan(b[0].y);
  // A snowman: a face on the head, buttons on the middle.
  await expect(page.locator('.pane.open .ball .nose')).toHaveCount(1);
  await expect(page.locator('.pane.open .ball .btn1')).toHaveCount(1);
  await expect(page.getByText('Снеговик готов!')).toBeVisible();
  // Kept for this tile: back after a reload.
  await page.reload();
  await expect(page.locator('.pane.open .ball')).toHaveCount(3);
  await expect(page.locator('.pane.open .ball .nose')).toHaveCount(1);
  // Play still works where the snow lies over it.
  await expect(page.locator('.pane-play')).toBeEnabled();
});
