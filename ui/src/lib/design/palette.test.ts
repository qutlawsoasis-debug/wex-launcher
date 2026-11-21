/**
 * One palette: wex.css must write exactly the colours of ApplePalette (colors.ts),
 * which are the values of the verified aura-knowledge cards. The test parses the
 * CSS, so a value changed in one place and not the other fails here.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { ApplePalette, CssPaletteMap, parseColor, meetsWcagAA } from './colors';

const root = resolve(process.cwd(), 'src');
const css = readFileSync(join(root, 'lib/design/wex.css'), 'utf8');

/** Custom properties declared directly inside the block opened by `selector`. */
function block(selector: string): Record<string, string> {
  const start = css.indexOf(selector);
  expect(start, `wex.css has ${selector}`).toBeGreaterThanOrEqual(0);
  const body = css.slice(start + selector.length, css.indexOf('}', start));
  return Object.fromEntries([...body.matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
}

const LIGHT = block(':root {');
const DARK_SYSTEM = block(":root:not([data-theme='light']) {");
const DARK_FORCED = block(":root[data-theme='dark'] {");

const same = (a: string, b: string) => {
  const x = parseColor(a);
  const y = parseColor(b);
  expect(x, `parse ${a}`).not.toBeNull();
  expect(y, `parse ${b}`).not.toBeNull();
  return x!.r === y!.r && x!.g === y!.g && x!.b === y!.b && Math.abs(x!.a - y!.a) < 0.001;
};

describe('wex.css and colors.ts are one palette', () => {
  for (const [name, key] of Object.entries(CssPaletteMap)) {
    it(`--${name} is ${key} in light and dark`, () => {
      expect(LIGHT[name], `light --${name}`).toBeDefined();
      expect(same(LIGHT[name], ApplePalette.light[key]), `light --${name} ${LIGHT[name]} vs ${ApplePalette.light[key]}`).toBe(true);
      for (const [label, dark] of [['system dark', DARK_SYSTEM], ['forced dark', DARK_FORCED]] as const) {
        expect(dark[name], `${label} --${name}`).toBeDefined();
        expect(same(dark[name], ApplePalette.dark[key]), `${label} --${name} ${dark[name]} vs ${ApplePalette.dark[key]}`).toBe(true);
      }
    });
  }

  it('system dark and forced dark declare the same variables with the same values', () => {
    expect(DARK_SYSTEM).toEqual(DARK_FORCED);
  });

  it('the card values themselves (spot check against the aura-knowledge cards)', () => {
    expect(LIGHT.blue).toBe('#0088ff');
    expect(DARK_FORCED.blue).toBe('#0091ff');
    expect(LIGHT.red).toBe('#ff383c');
    expect(DARK_FORCED.red).toBe('#ff4245');
    expect(LIGHT.label).toBe('#000000');
    expect(DARK_FORCED.label).toBe('#ffffff');
    expect(LIGHT['fill-2']).toBe('rgba(120, 120, 128, 0.16)');
    expect(LIGHT['fill-3']).toBe('rgba(120, 120, 128, 0.12)');
    expect(DARK_FORCED['fill-2']).toBe('rgba(120, 120, 128, 0.32)');
  });

  it('no colour variable outside the palette: no extra blues, tints or hover shades', () => {
    for (const vars of [LIGHT, DARK_SYSTEM, DARK_FORCED]) {
      for (const gone of ['blue-hi', 'blue-text', 'blue-tint', 'card-hover', 'warn-fill']) {
        expect(vars[gone], `--${gone}`).toBeUndefined();
      }
    }
    for (const banned of ['#0064d2', '#0a6fe0', '#0066d6', '#1a7ef0', '#4ba3ff', '#007aff', '#0a84ff', '#64d2ff']) {
      expect(css.toLowerCase(), banned).not.toContain(banned);
    }
  });
});

/** Every .svelte / .css / .ts source under src, except tests. */
function sources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) sources(path, out);
    else if (/\.(svelte|css|ts)$/.test(name) && !/\.test\.ts$/.test(name)) out.push(path);
  }
  return out;
}

describe('components use palette tokens, not their own colours', () => {
  const files = sources(root);
  const rel = (p: string) => p.slice(root.length + 1).replace(/\\/g, '/');

  it('removed tokens are referenced nowhere', () => {
    const offenders = files
      .filter((p) => /var\(--(blue-hi|blue-text|blue-tint|card-hover|warn-fill)\)/.test(readFileSync(p, 'utf8')))
      .map(rel);
    expect(offenders).toEqual([]);
  });

  it('no hard-coded colours in the controls and sheets that had them', () => {
    // Winter illustrations (Santa, scenes, Snowfall) are the documented exception:
    // Apple does not cover seasonal illustration.
    for (const file of ['Button.svelte', 'SegmentedControl.svelte', 'LogsSheet.svelte', 'CrashSheet.svelte']) {
      const src = readFileSync(join(root, 'lib/components', file), 'utf8');
      const style = src.slice(src.indexOf('<style'));
      const markup = src.slice(0, src.indexOf('<style'));
      expect(style.match(/#[0-9a-fA-F]{3,8}\b|rgba?\(/g), `${file} <style>`).toBeNull();
      expect(markup.match(/color="#[0-9a-fA-F]{3,8}"/g), `${file} markup`).toBeNull();
    }
  });

  it('no coloured glow under a button', () => {
    const src = readFileSync(join(root, 'lib/components/Button.svelte'), 'utf8');
    expect(src).not.toMatch(/box-shadow:[^;]*var\(--(blue|red|green)\)/);
  });
});

/** A palette colour laid over an opaque one (alpha compositing), as RGB. */
function over(top: string, bottom: string) {
  const t = parseColor(top)!;
  const b = parseColor(bottom)!;
  const mix = (x: number, y: number) => Math.round(x * t.a + y * (1 - t.a));
  return { r: mix(t.r, b.r), g: mix(t.g, b.g), b: mix(t.b, b.b) };
}
const hex = (c: { r: number; g: number; b: number }) => `#${[c.r, c.g, c.b].map((v) => v.toString(16).padStart(2, '0')).join('')}`;

describe('legibility of states (review follow-ups)', () => {
  const controls = readFileSync(join(root, 'lib/design/wex-controls.css'), 'utf8');

  it('disabled button text is secondaryLabel, readable on quaternarySystemFill (≥ 3:1, was ≈1.25:1)', () => {
    expect(controls).toMatch(/\.wx \.mini\[disabled\] \{[^}]*color: var\(--label-2\);/);
    for (const file of ['lib/components/Button.svelte', 'lib/shell/LaunchBar.svelte']) {
      expect(readFileSync(join(root, file), 'utf8'), file).not.toContain('var(--label-4)');
    }
    for (const mode of ['light', 'dark'] as const) {
      const p = ApplePalette[mode];
      const bg = over(p.quaternarySystemFill, p.secondarySystemBackground);
      const fg2 = over(p.secondaryLabel, hex(bg));
      const fg3 = over(p.tertiaryLabel, hex(bg));
      // tertiaryLabel (the first suggestion) is too faint in light mode; secondary passes 3:1.
      expect(meetsWcagAA(fg3, bg).ratio, `${mode} tertiary`).toBeLessThan(meetsWcagAA(fg2, bg).ratio);
      expect(meetsWcagAA(fg2, bg).ratio, `${mode} secondary`).toBeGreaterThanOrEqual(3);
    }
  });

  it('progress and meter fills reach 3:1 on their track (label in light, systemGray in dark)', () => {
    const light = ApplePalette.light;
    const dark = ApplePalette.dark;
    const lightTrack = over(light.secondarySystemFill, light.secondarySystemBackground);
    const darkTrack = over(dark.secondarySystemFill, dark.secondarySystemBackground);
    expect(meetsWcagAA(parseColor(light.label)!, lightTrack).ratio).toBeGreaterThanOrEqual(3);
    expect(meetsWcagAA(parseColor(dark.systemGray)!, darkTrack).ratio).toBeGreaterThanOrEqual(3);
    // systemGray in light would not (the reason for --meter).
    expect(meetsWcagAA(parseColor(light.systemGray)!, lightTrack).ratio).toBeLessThan(3);
    expect(LIGHT.meter).toBe('var(--label)');
    expect(DARK_FORCED.meter).toBe('var(--gray)');
    for (const file of ['lib/shell/LaunchBar.svelte', 'lib/components/ProgressBar.svelte', 'lib/pages/servers/ServersPage.svelte']) {
      expect(readFileSync(join(root, file), 'utf8'), file).toContain('var(--meter)');
    }
  });

  it('an on-but-disabled switch still shows green (40 %)', () => {
    expect(controls).toMatch(/\.wx \.switch\[disabled\]\[aria-checked='true'\] \{\s*background: color-mix\(in srgb, var\(--green\) 40%, transparent\);/);
  });

  it('selected choice cards carry a checkmark, not only a denser gray', () => {
    const app = readFileSync(join(root, 'app.css'), 'utf8');
    for (const sel of [".preset[aria-checked='true']::after", ".theme-card[aria-checked='true']::after", ".cape-card[aria-pressed='true']::after"]) {
      expect(app, sel).toContain(sel);
    }
  });
});
