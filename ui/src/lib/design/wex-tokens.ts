/**
 * Wex design tokens for code (the CSS variables live in wex.css).
 * Colours are not repeated here: they come from ApplePalette in colors.ts, the one
 * palette; palette.test.ts checks that wex.css writes exactly those values.
 * Type, radii, layout and motion come from docs/design/launcher-reference.html (v8);
 * wex-tokens.test.ts checks that they agree with wex.css.
 */

import { ApplePalette } from './colors';

const colorsOf = (p: (typeof ApplePalette)['light' | 'dark']) => ({
  blue: p.systemBlue,
  blueInk: p.systemBlueOnAccent,
  green: p.systemGreen,
  red: p.systemRed,
  orange: p.systemOrange,
  label: p.label,
  base: p.secondarySystemBackground,
  card: p.tertiarySystemBackground,
});

export const WexColors = {
  light: colorsOf(ApplePalette.light),
  dark: colorsOf(ApplePalette.dark),
} as const;

export const WexType = {
  family: "'Onest Variable', 'Onest', -apple-system, 'SF Pro Text', system-ui, sans-serif",
  size: { cap: 12, foot: 13, sub: 15, body: 16, t3: 20, t2: 24, t1: 32, hero: 56 },
  weight: { regular: 400, medium: 500, semibold: 600, bold: 700, display: 800 },
} as const;

export const WexRadii = { xs: 6, s: 8, m: 12, l: 16, window: 12, pill: 999 } as const;

export const WexLayout = {
  sideWidth: 248,
  windowMax: 1480,
  rowNav: 40,
  rowLib: 52,
  control: 40,
  controlBig: 52,
  gutter: 24,
} as const;

export const WexMotion = {
  ease: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
  spring: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
  ms: {
    micro: 180,
    short: 220,
    indicator: 260,
    page: 280,
    pageOut: 180,
    pair: 280,
    swap: 220,
    started: 600,
    stagger: 40,
    countUp: 700,
    toast: 2600,
  },
  pressScale: 0.97,
  hoverLift: -2,
} as const;

export type WexDuration = keyof typeof WexMotion.ms;

/** True when motion is reduced as theme resolved it (app switch over system, see theme.svelte.ts). */
export function motionReduced(): boolean {
  return typeof document !== 'undefined' && document.documentElement.dataset.reduceMotion === 'true';
}

/** A duration in ms, or 0 when motion is reduced (for JS-driven animation). */
export function duration(name: WexDuration): number {
  return motionReduced() ? 0 : WexMotion.ms[name];
}
