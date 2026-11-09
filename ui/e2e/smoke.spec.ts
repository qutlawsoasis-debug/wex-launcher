import { test, expect, type Page } from '@playwright/test';
import { setupMockIpc, type MockIpcOptions } from './mock-ipc';

const SECTIONS = [
  { id: 'play', name: 'Играть' },
  { id: 'servers', name: 'Серверы' },
  { id: 'lobby', name: 'Лобби' },
  { id: 'friends', name: 'Друзья' },
  { id: 'wardrobe', name: 'Гардероб' },
  { id: 'workshop', name: 'Мастерская' },
  { id: 'mods', name: 'Моды' },
  { id: 'settings', name: 'Настройки' },
] as const;

const THEMES = ['dark', 'light'] as const;
const BACKGROUNDS = ['day', 'morning', 'night', 'sunset'] as const;

async function setupPageListeners(page: Page) {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];

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

  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  page.on('pageerror', (err) => {
    pageErrors.push(err.message);
  });

  return { consoleErrors, pageErrors };
}

async function assertViewIntegrity(
  page: Page,
  viewName: string,
  consoleErrors: string[],
  pageErrors: string[]
) {
  // 1. Check Console & Page errors
  expect(consoleErrors, `Console errors detected in ${viewName}`).toEqual([]);
  expect(pageErrors, `Page errors detected in ${viewName}`).toEqual([]);

  // 2. Check Horizontal Scroll
  const scrollInfo = await page.evaluate(() => ({
    docScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    bodyScroll: document.body.scrollWidth > document.body.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
  }));
  expect(
    scrollInfo.docScroll || scrollInfo.bodyScroll,
    `Horizontal scroll detected in ${viewName}: scrollWidth=${scrollInfo.scrollWidth}, clientWidth=${scrollInfo.clientWidth}`
  ).toBe(false);

  // 3. Check Text Truncation (scrollWidth > clientWidth for elements with text)
  const truncatedElements = await page.evaluate(() => {
    const truncated: Array<{
      tag: string;
      class: string;
      text: string;
      scrollWidth: number;
      clientWidth: number;
    }> = [];
    const elements = document.querySelectorAll('*');
    for (const el of elements) {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      const style = window.getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') continue;

      const hasDirectText = Array.from(el.childNodes).some(
        (n) => n.nodeType === Node.TEXT_NODE && n.textContent && n.textContent.trim().length > 0
      );
      if (!hasDirectText) continue;

      if (style.overflowX === 'scroll' || style.overflowX === 'auto') continue;
      if (el.closest('.stories-track') || el.closest('.carousel-track')) continue;

      // 1px subpixel tolerance
      if (el.scrollWidth > el.clientWidth + 1) {
        truncated.push({
          tag: el.tagName,
          class: el.className,
          text: (el.textContent || '').trim().slice(0, 50),
          scrollWidth: el.scrollWidth,
          clientWidth: el.clientWidth,
        });
      }
    }
    return truncated;
  });

  expect(
    truncatedElements,
    `Truncated text elements detected in ${viewName}: ${JSON.stringify(truncatedElements)}`
  ).toEqual([]);
}

async function assertSceneryTextContrast(page: Page, contextInfo: string) {
  const contrastResults = await page.evaluate(async () => {
    function parseRgb(colorStr: string) {
      const m = colorStr.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
      if (!m) return { r: 255, g: 255, b: 255 };
      return { r: parseInt(m[1], 10), g: parseInt(m[2], 10), b: parseInt(m[3], 10) };
    }

    function getRelativeLuminance(r: number, g: number, b: number) {
      const rs = r / 255;
      const gs = g / 255;
      const bs = b / 255;
      const R = rs <= 0.03928 ? rs / 12.92 : Math.pow((rs + 0.055) / 1.055, 2.4);
      const G = gs <= 0.03928 ? gs / 12.92 : Math.pow((gs + 0.055) / 1.055, 2.4);
      const B = bs <= 0.03928 ? bs / 12.92 : Math.pow((bs + 0.055) / 1.055, 2.4);
      return 0.2126 * R + 0.7152 * G + 0.0722 * B;
    }

    function getContrastRatio(l1: number, l2: number) {
      const lighter = Math.max(l1, l2);
      const darker = Math.min(l1, l2);
      return (lighter + 0.05) / (darker + 0.05);
    }

    const sceneryArt = document.querySelector('.scenery-canvas') as HTMLElement | null;
    const bgProp = sceneryArt
      ? sceneryArt.style.backgroundImage || window.getComputedStyle(sceneryArt).backgroundImage
      : '';
    const bgUrlMatch = bgProp.match(/url\(["']?([^"']+)["']?\)/);
    const bgUrl = bgUrlMatch ? bgUrlMatch[1] : null;
    if (!bgUrl) return [];

    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.src = bgUrl;
    await new Promise((resolve, reject) => {
      img.onload = resolve;
      img.onerror = reject;
    });

    const canvas = document.createElement('canvas');
    const width = window.innerWidth;
    const height = window.innerHeight;
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d')!;

    const scale = Math.max(width / img.width, height / img.height);
    const x = (width - img.width * scale) / 2;
    const y = (height - img.height * scale) / 2;
    ctx.drawImage(img, x, y, img.width * scale, img.height * scale);

    const vignetteGrad = ctx.createLinearGradient(0, 0, 0, height);
    vignetteGrad.addColorStop(0, 'rgba(0, 0, 0, 0.65)');
    vignetteGrad.addColorStop(0.25, 'rgba(0, 0, 0, 0.50)');
    vignetteGrad.addColorStop(0.50, 'rgba(0, 0, 0, 0.30)');
    vignetteGrad.addColorStop(0.75, 'rgba(0, 0, 0, 0.65)');
    vignetteGrad.addColorStop(1.00, 'rgba(0, 0, 0, 0.88)');
    ctx.fillStyle = vignetteGrad;
    ctx.fillRect(0, 0, width, height);

    const selectors = ['.game-heading', '.game-subheading', '.header-tools button span'];
    const results: Array<{ selector: string; text: string; ratio: number; pass: boolean }> = [];

    for (const sel of selectors) {
      const el = document.querySelector(sel);
      if (!el) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;

      const textStyle = window.getComputedStyle(el);
      const textRgb = parseRgb(textStyle.color);
      const textLum = getRelativeLuminance(textRgb.r, textRgb.g, textRgb.b);

      const sampleX = Math.floor(Math.max(0, Math.min(width - 1, rect.left + rect.width / 2)));
      const sampleY = Math.floor(Math.max(0, Math.min(height - 1, rect.top + rect.height / 2)));
      const pixel = ctx.getImageData(sampleX, sampleY, 1, 1).data;
      const bgLum = getRelativeLuminance(pixel[0], pixel[1], pixel[2]);

      const ratio = getContrastRatio(textLum, bgLum);
      results.push({
        selector: sel,
        text: (el.textContent || '').trim(),
        ratio: parseFloat(ratio.toFixed(2)),
        pass: ratio >= 4.5,
      });
    }
    return results;
  });

  for (const r of contrastResults) {
    expect(
      r.pass,
      `Contrast ratio below 4.5:1 (${r.ratio}:1) for "${r.text}" (${r.selector}) in ${contextInfo}`
    ).toBe(true);
  }
}

test.describe('Wex UI Smoke Tests', () => {
  // Test 1: Full Populated State across all 8 screens and themes
  for (const theme of THEMES) {
    test.describe(`Full State - Theme: ${theme}`, () => {
      for (const section of SECTIONS) {
        test(`renders section «${section.name}» without errors, truncation, or horizontal scroll`, async ({
          page,
        }) => {
          const { consoleErrors, pageErrors } = await setupPageListeners(page);
          await page.addInitScript(setupMockIpc, { mode: 'full' } as MockIpcOptions);

          await page.goto(`/?tab=${section.id}&theme=${theme}`);
          await page.waitForTimeout(300);

          await assertViewIntegrity(
            page,
            `${section.name} (Full, ${theme})`,
            consoleErrors,
            pageErrors
          );
        });
      }
    });
  }

  // Test 2: Empty State across all 8 screens and themes
  for (const theme of THEMES) {
    test.describe(`Empty State - Theme: ${theme}`, () => {
      for (const section of SECTIONS) {
        test(`renders empty state for «${section.name}» without errors, truncation, or horizontal scroll`, async ({
          page,
        }) => {
          const { consoleErrors, pageErrors } = await setupPageListeners(page);
          await page.addInitScript(setupMockIpc, { mode: 'empty' } as MockIpcOptions);

          await page.goto(`/?tab=${section.id}&theme=${theme}`);
          await page.waitForTimeout(300);

          await assertViewIntegrity(
            page,
            `${section.name} (Empty, ${theme})`,
            consoleErrors,
            pageErrors
          );
        });
      }
    });
  }

  // Test 3: Modals and Sheets
  for (const theme of THEMES) {
    test(`renders and closes modals and sheets in theme: ${theme}`, async ({ page }) => {
      const { consoleErrors, pageErrors } = await setupPageListeners(page);
      await page.addInitScript(setupMockIpc, { mode: 'full' } as MockIpcOptions);

      // Modal 1: Import Wizard Sheet
      await page.goto(`/?tab=play&theme=${theme}`);
      await page.waitForTimeout(300);
      const importBtn = page.locator('.header-tools button');
      if (await importBtn.isVisible()) {
        await importBtn.click();
        await page.waitForTimeout(300);
        await assertViewIntegrity(
          page,
          `ImportWizardSheet (${theme})`,
          consoleErrors,
          pageErrors
        );
        await page.keyboard.press('Escape');
        await page.waitForTimeout(200);
      }

      // Modal 2: Crash Sheet & Report Sheet
      await page.evaluate(() => (window as any).__TRIGGER_CRASH__());
      await page.waitForTimeout(300);
      await assertViewIntegrity(page, `CrashSheet (${theme})`, consoleErrors, pageErrors);

      const reportBtn = page.locator('.crash-actions button:has-text("Отправить отчёт")');
      if (await reportBtn.isVisible()) {
        await reportBtn.click();
        await page.waitForTimeout(300);
        await assertViewIntegrity(page, `ReportSheet (${theme})`, consoleErrors, pageErrors);
        await page.keyboard.press('Escape');
        await page.waitForTimeout(200);
      }
      await page.keyboard.press('Escape');
      await page.waitForTimeout(200);

      // Modal 3: Add Server Sheet
      await page.goto(`/?tab=servers&theme=${theme}`);
      await page.waitForTimeout(300);
      const addServerBtn = page.locator('.toolbar-controls button:has-text("Добавить сервер")');
      if (await addServerBtn.isVisible()) {
        await addServerBtn.click();
        await page.waitForTimeout(300);
        await assertViewIntegrity(
          page,
          `AddServerSheet (${theme})`,
          consoleErrors,
          pageErrors
        );
        await page.keyboard.press('Escape');
        await page.waitForTimeout(200);
      }

      // Modal 4: Workshop New World Sheet
      await page.goto(`/?tab=workshop&theme=${theme}`);
      await page.waitForTimeout(300);
      const newWorldBtn = page.locator('.workshop-toolbar button:has-text("Новый мир")');
      if (await newWorldBtn.isVisible()) {
        await newWorldBtn.click();
        await page.waitForTimeout(300);
        await assertViewIntegrity(
          page,
          `WorkshopNewWorldSheet (${theme})`,
          consoleErrors,
          pageErrors
        );
        await page.keyboard.press('Escape');
        await page.waitForTimeout(200);
      }
    });
  }

  // Test 4: Text Contrast >= 4.5:1 over All Scenery Backgrounds
  for (const bg of BACKGROUNDS) {
    for (const theme of THEMES) {
      test(`verifies WCAG AA contrast >= 4.5:1 on background: ${bg} (${theme})`, async ({
        page,
      }) => {
        const { consoleErrors, pageErrors } = await setupPageListeners(page);
        await page.addInitScript(setupMockIpc, { mode: 'full' } as MockIpcOptions);

        await page.goto(`/?tab=play&bg=${bg}&theme=${theme}`);
        await page.waitForTimeout(400);

        await assertSceneryTextContrast(page, `Background: ${bg}, Theme: ${theme}`);
        await assertViewIntegrity(
          page,
          `PlayScreen Contrast (${bg}, ${theme})`,
          consoleErrors,
          pageErrors
        );
      });
    }
  }
});
