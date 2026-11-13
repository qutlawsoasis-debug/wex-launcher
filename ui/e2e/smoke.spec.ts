import { test, expect } from '@playwright/test';
import { openApp, axeProblems, ipcCalls } from './helpers';

const PAGES: Array<{ hash: string; heading: string | RegExp }> = [
  { hash: '', heading: 'Wex 1.21.1' },
  { hash: '#build-1.20.1', heading: 'Wex 1.20.1' },
  { hash: '#servers', heading: 'Серверы' },
  { hash: '#friends', heading: 'Друзья' },
  { hash: '#wardrobe', heading: 'Гардероб' },
  { hash: '#options', heading: 'Настройки' },
];

for (const scheme of ['light', 'dark'] as const) {
  test.describe(`every page renders without errors (${scheme})`, () => {
    for (const p of PAGES) {
      test(`${p.hash || '#home'}`, async ({ page }) => {
        const errors = await openApp(page, p.hash, { mode: 'full', scheme });
        await expect(page.locator('.page h1').first()).toHaveText(p.heading);
        expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe(scheme);
        expect(await axeProblems(page)).toEqual([]);
        expect(errors).toEqual([]);
      });
    }
  });
}

test('sidebar: sections, both builds with mod counts, the selected one marked', async ({ page }) => {
  await openApp(page);
  await expect(page.locator('.nav a')).toHaveText(['Главная', 'Серверы', 'Друзья', 'Гардероб', 'Настройки']);
  await expect(page.locator('.lib a').nth(0)).toContainText('4 мода');
  await expect(page.locator('.lib a').nth(1)).toContainText('3 мода');
  await expect(page.locator('.lib a').nth(0).locator('.sel')).toHaveCount(1);
  await page.locator('.nav a', { hasText: 'Серверы' }).click();
  await expect(page.locator('.nav a[aria-current="page"]')).toHaveText('Серверы');
});

test('launch: Play shows the stages and progress from core, then the running game, then Stop', async ({ page }) => {
  await openApp(page);
  await page.locator('.bar .play').click();
  await expect(page.locator('.bar .title')).toContainText('Скачиваем ресурсы игры');
  await expect(page.locator('.bar .sub')).toContainText('47%');
  await expect(page.locator('.bar .play')).toHaveText('Отменить');
  await expect(page.locator('.bar .title')).toContainText('Игра идёт');
  await expect(page.locator('.bar .play')).toHaveText('Остановить');
  expect(await ipcCalls(page, 'start_game')).toEqual([{ version: '1.21.1', nickname: 'Player', jvmArgs: null, server: null }]);
  await page.locator('.bar .play').click();
  await expect(page.locator('.toast')).toContainText('Игра остановлена');
  await expect(page.locator('.bar .play')).toContainText('Играть');
});

test('both builds launch: choosing 1.20.1 on Home saves it and Play starts 1.20.1', async ({ page }) => {
  await openApp(page);
  await page.locator('.pane:not(.open)').click();
  await expect(page.locator('.pane.open h1')).toHaveText('Wex 1.20.1');
  await expect(page.locator('.toast')).toContainText('Для запуска выбрана Wex 1.20.1');
  await expect(page.locator('.lib a').nth(1).locator('.sel')).toHaveCount(1);
  const saved = (await ipcCalls(page, 'save_launcher_settings')) as Array<{ newSettings: { selected_version: string } }>;
  expect(saved.at(-1)?.newSettings.selected_version).toBe('1.20.1');
  await page.locator('.pane.open .pane-play').click();
  await expect(page.locator('.bar .title')).toContainText('Игра идёт');
  expect(((await ipcCalls(page, 'start_game')) as Array<{ version: string }>)[0].version).toBe('1.20.1');
});

test('build page: mods toggle through core, workshop tabs are there', async ({ page }) => {
  await openApp(page, '#build-1.21.1');
  await expect(page.locator('[role="tab"]')).toHaveText(['Моды', 'Миры', 'Ресурспаки', 'Шейдеры', 'Скриншоты', 'Бэкапы', 'Настройки сборки']);
  const menu = page.locator('.lrow', { hasText: 'Mod Menu' });
  await expect(menu.locator('.switch')).toHaveAttribute('aria-checked', 'false');
  await menu.locator('.switch').click();
  await expect(menu.locator('.switch')).toHaveAttribute('aria-checked', 'true');
  expect(await ipcCalls(page, 'toggle_mod')).toEqual([{ version: '1.21.1', fileName: 'menu.jar', enabled: true }]);
  await page.locator('[role="tab"]', { hasText: 'Миры' }).click();
  await expect(page.locator('.empty')).toContainText('В этой сборке ещё нет миров.');
});

test('settings: theme applies at once and is saved', async ({ page }) => {
  await openApp(page, '#options', { mode: 'full', scheme: 'light' });
  await page.locator('.theme-card', { hasText: 'Тёмная' }).click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe('dark');
  const saved = (await ipcCalls(page, 'save_launcher_settings')) as Array<{ newSettings: { appearance: string } }>;
  expect(saved.at(-1)?.newSettings.appearance).toBe('dark');
});

test('wardrobe: 3D model with all parts, poses, achievements in Profile', async ({ page }) => {
  await openApp(page, '#wardrobe');
  await expect(page.locator('.rig .joint')).toHaveCount(6);
  await page.locator('.stage-bar .seg button', { hasText: 'Идёт' }).click();
  await expect(page.locator('.rig')).toHaveClass(/pose-walk/);
  await page.locator('.wd-panel [role="tab"]', { hasText: 'Профиль' }).click();
  await expect(page.locator('.ach-row')).toContainText('1 из 1');
});

test('empty data: honest empty states, no sample content', async ({ page }) => {
  await openApp(page, '', { mode: 'empty' });
  await expect(page.locator('.panel:not(.together)')).toContainText('Друзья появятся, когда подключим аккаунты.');
  await page.locator('.nav a', { hasText: 'Серверы' }).click();
  await expect(page.locator('.list .empty')).toContainText('Серверов пока нет.');
  await expect(page.locator('body')).not.toContainText(/kotya|Aura Creative|Aura Minigames/);
});

test('servers: add a server through core', async ({ page }) => {
  await openApp(page, '#servers');
  await expect(page.locator('.lrow-srv')).toHaveCount(2);
  await page.getByRole('button', { name: 'Добавить сервер' }).click();
  await page.locator('.editor input').nth(0).fill('Мой сервер');
  await page.locator('.editor input').nth(1).fill('my.example.net');
  await page.locator('.editor .btn.blue').click();
  await expect(page.locator('.lrow-srv')).toHaveCount(3);
});

for (const [w, h] of [[880, 600], [1080, 720]] as const) {
  test(`window ${w}×${h} (minimum and default): no page spills sideways, wardrobe keeps two columns`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    for (const p of PAGES) {
      await openApp(page, p.hash);
      await expect(page.locator('.page h1').first()).toHaveText(p.heading);
      const spill = await page.evaluate(() => {
        const main = document.querySelector('main')!;
        return main.scrollWidth - main.clientWidth;
      });
      expect(spill, `${p.hash || '#home'} overflows by ${spill}px`).toBeLessThanOrEqual(1);
    }
    await openApp(page, '#wardrobe');
    const cols = await page.locator('.wd2').evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length);
    expect(cols).toBe(2);
    // The whole model stands inside the scene: the nickname tag sits between its top and the bottom bar.
    await expect
      .poll(() =>
        page.evaluate(() => {
          const s = document.querySelector('.stage3d')!.getBoundingClientRect();
          const tag = document.querySelector('.nametag')!.getBoundingClientRect();
          const bar = document.querySelector('.stage-bar')!.getBoundingClientRect();
          return tag.top >= s.top && tag.top < bar.top;
        })
      )
      .toBe(true);
  });
}
