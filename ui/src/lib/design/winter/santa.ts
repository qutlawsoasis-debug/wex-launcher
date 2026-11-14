/**
 * Santa's sleigh (Santa.svelte): when passes happen, which way they go, the arc
 * they follow, and the glitter the sleigh sheds. Pure functions so they can be
 * unit-tested; randomness comes in as a `rand` function returning [0, 1).
 * Apple has no guidance on seasonal art; see Santa.svelte for the Motion rules.
 */

export type Rand = () => number;
export type Direction = 'ltr' | 'rtl';

export interface Pass {
  /** ltr: flies left to right (art as drawn); rtl: mirrored. */
  dir: Direction;
  /** Length of the crossing, ms. */
  duration: number;
  /** Height of the path at the window edges, as a fraction of the window height. */
  top: number;
  /** How far the arc rises in the middle, fraction of the window height. */
  lift: number;
  /** Small nose-up tilt at the start (and nose-down at the end), degrees. */
  tilt: number;
}

/** Deterministic generator (mulberry32) for the glitter layout and tests. */
export function rng(seed: number): Rand {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const between = (rand: Rand, lo: number, hi: number) => lo + (hi - lo) * rand();

/** The first pass comes 8–15 s after start. */
export function firstDelay(rand: Rand = Math.random): number {
  return Math.round(between(rand, 8_000, 15_000));
}

/** After a pass the sky stays empty for 45–110 s. */
export function nextDelay(rand: Rand = Math.random): number {
  return Math.round(between(rand, 45_000, 110_000));
}

/** Retry interval while the window is hidden or the launcher idles. */
export const PAUSED_RETRY = 4_000;

export function makePass(rand: Rand = Math.random): Pass {
  return {
    dir: rand() < 0.5 ? 'ltr' : 'rtl',
    duration: Math.round(between(rand, 9_000, 12_000)),
    top: between(rand, 0.1, 0.22),
    lift: between(rand, 0.04, 0.09),
    tilt: between(rand, 2, 4),
  };
}

/**
 * Where the sleigh is at progress t (0..1) of a pass, as fractions of the window:
 * x runs edge to edge (just off-screen at both ends), y is a gentle parabola.
 * The CSS keyframes in Santa.svelte follow the same curve.
 */
export function arcPoint(pass: Pass, t: number): { x: number; y: number } {
  const p = Math.min(1, Math.max(0, t));
  const along = -0.35 + 1.7 * p;
  return { x: pass.dir === 'ltr' ? along : 1 - along, y: pass.top - pass.lift * 4 * p * (1 - p) };
}

export interface Sparkle {
  /** Where it is shed, in art units (behind the runners). */
  x: number;
  y: number;
  /** Drift backwards and down before it fades, art units. */
  dx: number;
  dy: number;
  r: number;
  /** Seconds. Negative delay so the trail is already full when a pass starts. */
  delay: number;
  dur: number;
  /** star: four-point twinkle; dot: round glitter. */
  kind: 'star' | 'dot';
  /** Gold or white. */
  warm: boolean;
}

/** Glitter shed behind the sleigh; laid out once, the motion is pure CSS. */
export function makeSparkles(count: number, seed = 7): Sparkle[] {
  const rand = rng(seed);
  return Array.from({ length: count }, (_, i) => {
    const dur = between(rand, 1.4, 2.4);
    return {
      x: between(rand, 26, 60),
      y: between(rand, 150, 176),
      dx: -between(rand, 120, 230),
      dy: between(rand, -16, 26),
      r: between(rand, 1.4, 3.6),
      delay: -((i / count) * dur + rand() * 0.2),
      dur,
      kind: rand() < 0.4 ? 'star' : 'dot',
      warm: rand() < 0.6,
    };
  });
}

/** A four-point star centred on the origin (concave sides), radius r. */
export function starPath(r: number): string {
  const k = r * 0.28;
  const f = (n: number) => +n.toFixed(2);
  return `M0 ${f(-r)} Q${f(k)} ${f(-k)} ${f(r)} 0 Q${f(k)} ${f(k)} 0 ${f(r)} Q${f(-k)} ${f(k)} ${f(-r)} 0 Q${f(-k)} ${f(-k)} 0 ${f(-r)} Z`;
}
