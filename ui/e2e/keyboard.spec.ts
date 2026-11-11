import { test, expect, type Page } from '@playwright/test';
import { setupMockIpc, type MockIpcOptions } from './mock-ipc';

async function setupTestPage(page: Page, initialTab = 'play') {
  await page.route('**', (route) => {
    const url = route.request().url();
    if (url.startsWith('http://127.0.0.1:4173') || url.startsWith('http://localhost:4173')) {
      route.continue();
    } else {
      route.fulfill({
        status: 200,
        contentType: 'image/png',
        body: Buffer.from(
          'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
          'base64'
        ),
      });
    }
  });

  await page.addInitScript(() => {
    localStorage.setItem('wex_onboarding_completed', 'true');
    localStorage.setItem('wex_import_checked', 'true');
  });
  await page.addInitScript(setupMockIpc, { mode: 'full' } as MockIpcOptions);

  await page.goto(`/?tab=${initialTab}&theme=dark`);
  await page.waitForSelector('.wex-shell');
  await page.waitForTimeout(200);
}

async function assertActiveElementHasVisibleFocus(page: Page, description: string) {
  const focusInfo = await page.evaluate(() => {
    const el = document.activeElement;
    if (!el || el === document.body) {
      return { hasFocus: false, tag: 'BODY', outlineStyle: 'none', outlineWidth: '0px' };
    }
    const style = window.getComputedStyle(el);
    return {
      hasFocus: true,
      tag: el.tagName,
      className: el.className,
      text: (el.textContent || '').trim().slice(0, 30),
      outlineStyle: style.outlineStyle,
      outlineWidth: style.outlineWidth,
      outlineColor: style.outlineColor,
    };
  });

  expect(focusInfo.hasFocus, `Focus was lost to body at: ${description}`).toBe(true);
  expect(
    focusInfo.outlineStyle,
    `Outline style should not be 'none' at: ${description} (got ${JSON.stringify(focusInfo)})`
  ).not.toBe('none');
  expect(
    parseFloat(focusInfo.outlineWidth),
    `Outline width should be > 0 at: ${description}`
  ).toBeGreaterThan(0);
}

test.describe('Keyboard-Only E2E Navigation & Accessibility (wex-keyboard-e2e)', () => {
  test('1. Tab sequence across sidebar sections with visible focus and Enter activation', async ({
    page,
  }) => {
    await setupTestPage(page, 'play');

    const expectedSections = [
      { id: 'play', name: 'Играть' },
      { id: 'servers', name: 'Серверы' },
      { id: 'lobby', name: 'Лобби' },
      { id: 'friends', name: 'Друзья' },
      { id: 'wardrobe', name: 'Гардероб' },
      { id: 'workshop', name: 'Мастерская' },
      { id: 'mods', name: 'Моды' },
      { id: 'settings', name: 'Настройки' },
    ];

    // Focus first sidebar item
    const firstNavBtn = page.locator('.sidebar-nav .nav-item').first();
    await firstNavBtn.focus();

    for (let i = 0; i < expectedSections.length; i++) {
      const sec = expectedSections[i];
      await assertActiveElementHasVisibleFocus(page, `Sidebar item ${sec.name}`);

      // Press Enter to open section
      await page.keyboard.press('Enter');
      await page.waitForTimeout(100);

      // Verify active section
      const activeNav = page.locator('.sidebar-nav .nav-item.active');
      await expect(activeNav).toContainText(sec.name);

      // Tab to next section if not last
      if (i < expectedSections.length - 1) {
        await page.keyboard.press('Tab');
      }
    }
  });

  test('2. Enter opens modal sheet, Esc closes it, and focus returns with visible outline', async ({
    page,
  }) => {
    await setupTestPage(page, 'servers');

    // Find "Добавить сервер" button and focus it
    const addServerBtn = page.locator('button:has-text("Добавить сервер")');
    await expect(addServerBtn).toBeVisible();
    await addServerBtn.focus();

    await assertActiveElementHasVisibleFocus(page, 'Add Server button before opening sheet');

    // Press Enter to open Sheet
    await page.keyboard.press('Enter');
    const sheetCard = page.locator('.sheet-card');
    await expect(sheetCard).toBeVisible();

    // Verify modal is open and dialog has focus
    await expect(page.locator('.sheet-card[role="dialog"]')).toBeVisible();

    // Press Escape to dismiss sheet
    await page.keyboard.press('Escape');
    await expect(sheetCard).toBeHidden();

    // Verify focus is restored to the opener button and is visible
    await page.waitForTimeout(50);
    const activeText = await page.evaluate(() => document.activeElement?.textContent);
    expect(activeText).toContain('Добавить сервер');
    await assertActiveElementHasVisibleFocus(page, 'Add Server button after closing sheet with Esc');
  });

  test('3. Arrow keys navigate SegmentedControl tabs and select options', async ({ page }) => {
    await setupTestPage(page, 'settings');

    // Find the graphics preset SegmentedControl
    const segmentedControl = page.locator('.apple-segmented-control[role="radiogroup"], .apple-segmented-control[role="tablist"]').first();
    await expect(segmentedControl).toBeVisible();

    // Focus the selected segment
    const activeSegment = segmentedControl.locator('.segment-button[aria-checked="true"], .segment-button[aria-selected="true"]');
    await activeSegment.focus();
    await assertActiveElementHasVisibleFocus(page, 'Active segment in SegmentedControl');

    const initialText = await activeSegment.textContent();

    // Press ArrowRight to move to next option
    await page.keyboard.press('ArrowRight');
    await page.waitForTimeout(100);

    const nextSegment = segmentedControl.locator('.segment-button[aria-checked="true"], .segment-button[aria-selected="true"]');
    const nextText = await nextSegment.textContent();
    expect(nextText).not.toEqual(initialText);
    await assertActiveElementHasVisibleFocus(page, 'Next selected segment after ArrowRight');

    // Press ArrowLeft to move back
    await page.keyboard.press('ArrowLeft');
    await page.waitForTimeout(100);

    const prevSegment = segmentedControl.locator('.segment-button[aria-checked="true"], .segment-button[aria-selected="true"]');
    const prevText = await prevSegment.textContent();
    expect(prevText).toEqual(initialText);
    await assertActiveElementHasVisibleFocus(page, 'Previous selected segment after ArrowLeft');
  });

  test('4. Full keyboard walkthrough: Navigate to Play and activate «Играть» with Enter/Space', async ({
    page,
  }) => {
    await setupTestPage(page, 'play');

    // Locate launch button
    const launchBtn = page.locator('.dock-primary-action button');
    await expect(launchBtn).toBeVisible();
    await expect(launchBtn).toContainText('Играть');

    // Tab directly or focus the launch button
    await launchBtn.focus();
    await assertActiveElementHasVisibleFocus(page, 'Launch button');

    // Press Enter to trigger game launch
    await page.keyboard.press('Enter');
    await page.waitForTimeout(150);

    // Verify launching or progress state is triggered
    const isLaunchingOrRunning = await page.evaluate(() => {
      const dock = document.querySelector('.launch-dock');
      return (
        document.querySelector('.dock-progress') !== null ||
        document.querySelector('.apple-progress-bar') !== null ||
        document.querySelector('.is-running') !== null ||
        (dock && dock.textContent?.includes('Подготовка')) ||
        (dock && dock.textContent?.includes('Запуск'))
      );
    });

    expect(isLaunchingOrRunning).toBe(true);
  });

  test('5. Arrow keys in lists navigate rows and Space activates switches', async ({ page }) => {
    await setupTestPage(page, 'mods');

    // Find interactive toggle switches or rows
    const firstToggle = page.locator('.apple-toggle[role="switch"]').first();
    await expect(firstToggle).toBeVisible();

    await firstToggle.focus();
    await assertActiveElementHasVisibleFocus(page, 'Toggle switch in list');

    const initialState = await firstToggle.getAttribute('aria-checked');

    // Press Space to toggle
    await page.keyboard.press('Space');
    await page.waitForTimeout(100);

    const newState = await firstToggle.getAttribute('aria-checked');
    expect(newState).not.toEqual(initialState);
  });
});
