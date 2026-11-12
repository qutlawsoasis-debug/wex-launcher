/**
 * Wex design tokens for code (the CSS source of truth is wex.css).
 * Values come from docs/design/launcher-reference.html (v8). Keep both files in step;
 * design-system.test.ts checks that they agree.
 */

export const WexColors = {
  light: {
    blue: '#0064d2',
    blueHi: '#0a6fe0',
    blueText: '#0066d6',
    blueInk: '#ffffff',
    green: '#34c759',
    red: '#ff3b30',
    orange: '#ff9500',
    label: '#1d1d1f',
    base: '#f5f5f7',
    card: '#ffffff',
    cover1211: '#1e5fd8',
    cover1201: '#1f8a5b',
  },
  dark: {
    blue: '#0a6fe0',
    blueHi: '#1a7ef0',
    blueText: '#4ba3ff',
    blueInk: '#ffffff',
    green: '#30d158',
    red: '#ff453a',
    orange: '#ff9f0a',
    label: '#f5f5f7',
    base: '#161618',
    card: '#1f1f22',
    cover1211: '#1e5fd8',
    cover1201: '#1f8a5b',
  },
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
