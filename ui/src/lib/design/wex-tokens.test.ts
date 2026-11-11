import { describe, it, expect, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { WexColors, WexType, WexRadii, WexMotion, WexLayout, motionReduced, duration } from './wex-tokens';
import { theme } from './theme.svelte';
import { meetsWcagAA } from './colors';

const css = readFileSync(resolve(process.cwd(), 'src/lib/design/wex.css'), 'utf8');
const reference = readFileSync(resolve(process.cwd(), '../docs/design/launcher-reference.html'), 'utf8');

/** Same value written differently: no spaces, "0.32" equals ".32". */
const norm = (v?: string) => v?.replace(/\s/g, '').replace(/(^|[(,])0\./g, '$1.');

/** Value of a custom property inside the first block that matches `selector`. */
function prop(source: string, selector: string, name: string): string | undefined {
  const start = source.indexOf(selector);
  if (start < 0) return undefined;
  const block = source.slice(start, source.indexOf('}', start));
  const m = block.match(new RegExp(`--${name}:\\s*([^;]+);`));
  return m?.[1].trim();
}

const rgb = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
};

describe('Wex tokens follow the reference', () => {
  it('light colours, radii, type and motion match launcher-reference.html', () => {
    for (const name of ['blue', 'blue-hi', 'blue-text', 'green', 'red', 'orange', 'label', 'base', 'card', 'cover-1211', 'cover-1201',
      'r-s', 'r-m', 'r-l', 't-body', 't-hero', 'ease', 'spring', 'dur-micro', 'dur-short', 'dur-ind', 'dur-page', 'dur-pair']) {
      const ref = prop(reference, ':root {', name);
      expect(ref, `reference has --${name}`).toBeDefined();
      expect(norm(prop(css, ':root {', name)), `--${name}`).toBe(norm(ref));
    }
  });

  it('dark colours match the reference for both system and forced dark', () => {
    for (const name of ['blue', 'blue-text', 'green', 'red', 'label', 'base', 'card', 'side', 'bar']) {
      const ref = prop(reference, ':root[data-theme="dark"]', name);
      expect(prop(css, ":root[data-theme='dark']", name)).toBe(ref);
      expect(prop(css, ":root:not([data-theme='light'])", name)).toBe(ref);
    }
  });

  it('TS mirror agrees with the CSS', () => {
    for (const mode of ['light', 'dark'] as const) {
      const sel = mode === 'light' ? ':root {' : ":root[data-theme='dark']";
      expect(prop(css, sel, 'blue')).toBe(WexColors[mode].blue);
      expect(prop(css, sel, 'green')).toBe(WexColors[mode].green);
      expect(prop(css, sel, 'red')).toBe(WexColors[mode].red);
      expect(prop(css, sel, 'label')).toBe(WexColors[mode].label);
      expect(prop(css, sel, 'base')).toBe(WexColors[mode].base);
    }
    expect(prop(css, ':root {', 'r-s')).toBe(`${WexRadii.s}px`);
    expect(prop(css, ':root {', 'r-m')).toBe(`${WexRadii.m}px`);
    expect(prop(css, ':root {', 'r-l')).toBe(`${WexRadii.l}px`);
    expect(prop(css, ':root {', 't-hero')).toBe(`${WexType.size.hero}px`);
    expect(prop(css, ':root {', 'side-w')).toBe(`${WexLayout.sideWidth}px`);
    expect(prop(css, ':root {', 'dur-ind')).toBe(`${WexMotion.ms.indicator}ms`);
    expect(prop(css, ':root {', 'dur-pair')).toBe(`${WexMotion.ms.pair}ms`);
    expect(prop(css, ':root {', 'stagger')).toBe(`${WexMotion.ms.stagger}ms`);
    expect(prop(css, ':root {', 'ease')).toBe(WexMotion.ease);
    expect(prop(css, ':root {', 'spring')).toBe(WexMotion.spring);
  });

  it('uses the locally bundled Onest, never a font CDN', () => {
    expect(css).toContain("@import '@fontsource-variable/onest/index.css'");
    expect(css).not.toMatch(/fonts\.googleapis|fonts\.gstatic/);
    expect(prop(css, ':root {', 'font')).toMatch(/^'Onest Variable'/);
  });

  it('white text on the blue primary action is readable in both themes (WCAG AA)', () => {
    expect(meetsWcagAA(rgb(WexColors.light.blueInk), rgb(WexColors.light.blue)).pass).toBe(true);
    expect(meetsWcagAA(rgb(WexColors.dark.blueInk), rgb(WexColors.dark.blue), true).pass).toBe(true);
    expect(meetsWcagAA(rgb(WexColors.light.label), rgb(WexColors.light.base)).pass).toBe(true);
    expect(meetsWcagAA(rgb(WexColors.dark.label), rgb(WexColors.dark.base)).pass).toBe(true);
  });

  it('Reduce Motion zeroes every duration, both the system query and the app switch', () => {
    const names = [...css.matchAll(/--(dur-[a-z-]+|stagger):/g)].map((m) => m[1]);
    const unique = [...new Set(names)];
    for (const sel of [":root[data-reduce-motion='true']", '@media (prefers-reduced-motion: reduce)']) {
      for (const name of unique) expect(prop(css, sel, name), `${sel} --${name}`).toBe('0ms');
      expect(prop(css, sel, 'press-scale')).toBe('1');
    }
  });

  it('Reduce Transparency makes the sidebar and launch bar solid', () => {
    for (const sel of [":root[data-reduce-transparency='true']", '@media (prefers-reduced-transparency: reduce)']) {
      expect(prop(css, sel, 'side')).toBe('var(--veil-strong)');
      expect(prop(css, sel, 'bar')).toBe('var(--veil-strong)');
    }
  });
});

describe('Theme state', () => {
  afterEach(() => {
    theme.setAppearance('auto');
    theme.setReduceMotion(false);
    theme.setReduceTransparency(false);
    theme.systemReduceMotion = false;
    theme.systemReduceTransparency = false;
    theme.applyDom();
  });

  it('auto follows the system, light and dark force a theme', () => {
    const root = document.documentElement;
    theme.systemIsDark = true;
    theme.setAppearance('auto');
    expect(root.dataset.theme).toBe('dark');
    theme.systemIsDark = false;
    theme.applyDom();
    expect(root.dataset.theme).toBe('light');
    theme.setAppearance('dark');
    expect(root.dataset.theme).toBe('dark');
    theme.setAppearance('light');
    expect(root.dataset.theme).toBe('light');
  });

  it('the app switch does not hide the system setting and vice versa', () => {
    theme.systemReduceMotion = true;
    theme.setReduceMotion(false);
    expect(theme.reduceMotion).toBe(false);
    expect(theme.motionReduced).toBe(true);
    expect(document.documentElement.dataset.reduceMotion).toBe('true');
    expect(motionReduced()).toBe(true);
    expect(duration('indicator')).toBe(0);

    theme.systemReduceMotion = false;
    theme.setReduceMotion(true);
    expect(theme.motionReduced).toBe(true);
    theme.setReduceMotion(false);
    expect(document.documentElement.dataset.reduceMotion).toBe('false');
    expect(duration('indicator')).toBe(WexMotion.ms.indicator);
  });

  it('reduce transparency comes from the system or the app', () => {
    theme.systemReduceTransparency = true;
    theme.applyDom();
    expect(document.documentElement.dataset.reduceTransparency).toBe('true');
    theme.systemReduceTransparency = false;
    theme.setReduceTransparency(true);
    expect(document.documentElement.dataset.reduceTransparency).toBe('true');
    theme.setReduceTransparency(false);
    expect(document.documentElement.dataset.reduceTransparency).toBe('false');
  });
});
