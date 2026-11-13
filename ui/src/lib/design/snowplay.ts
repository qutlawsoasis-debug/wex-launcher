/**
 * Snowman building on the snowdrift of the open build tile (SnowPlay.svelte).
 * Plain geometry in px of the tile; y grows downwards.
 *
 * - Press on the snow and drag: a snowball rolls under the pointer and grows
 *   with the distance rolled.
 * - Press on a ball and drag: carry it; on release it lands on the ball under
 *   it (stacks) or falls onto the snow. Balls that stood on a carried ball fall.
 * - Three balls stacked, each smaller than the one below: a snowman.
 */

export interface Ball {
  id: number;
  /** Centre x, px. */
  x: number;
  /** Centre y, px. */
  y: number;
  r: number;
  /** The ball it stands on, or null on the snow. */
  on: number | null;
  /** Rolled distance, for the turning texture. */
  roll: number;
}

export const R_MIN = 6;
export const R_MAX = 40;
export const MAX_BALLS = 10;
/** Radius gained per px rolled. */
export const GROW = 0.075;
/** How deep a ball sinks into what it stands on, fraction of its radius. */
export const SINK = 0.18;

export function grow(r0: number, rolled: number): number {
  return Math.min(R_MAX, r0 + rolled * GROW);
}

/** The snow surface: y of the drift top at x, from drift heights (fractions of `driftH`). */
export function surfaceAt(x: number, width: number, height: number, heights: number[], driftH: number): number {
  if (!heights.length || width <= 0) return height;
  const n = heights.length - 1;
  const u = Math.max(0, Math.min(1, x / width)) * n;
  const i = Math.min(n - 1, Math.floor(u));
  const k = heights[i] + (heights[i + 1] - heights[i]) * (u - i);
  return height - k * driftH;
}

/** Where a ball rests on the snow at its x. */
export function restOnSnow(b: Pick<Ball, 'x' | 'r'>, surface: (x: number) => number): number {
  return surface(b.x) - b.r * (1 - SINK);
}

/** Where a ball rests on top of `under`. */
export function restOn(b: Pick<Ball, 'r'>, under: Ball): number {
  return under.y - under.r - b.r * (1 - SINK);
}

/** The ball a dropped ball lands on: the highest one horizontally under its centre. */
export function landingBall(b: Pick<Ball, 'id' | 'x' | 'r'>, balls: Ball[]): Ball | null {
  let best: Ball | null = null;
  for (const o of balls) {
    if (o.id === b.id) continue;
    if (Math.abs(o.x - b.x) > o.r * 0.85) continue;
    if (!best || o.y - o.r < best.y - best.r) best = o;
  }
  return best;
}

/**
 * Settles every ball: bottom-up, each stands on its support (centred on it) or on
 * the snow. A ball whose support is gone falls onto whatever is under it.
 */
export function settle(balls: Ball[], surface: (x: number) => number): Ball[] {
  const byId = new Map(balls.map((b) => [b.id, { ...b }]));
  const done = new Set<number>();
  const place = (b: Ball, depth = 0): void => {
    if (done.has(b.id)) return;
    if (depth > balls.length) b.on = null;
    let under = b.on !== null ? byId.get(b.on) : undefined;
    if (b.on !== null && !under) {
      b.on = null;
    }
    if (under) {
      place(under, depth + 1);
      b.x = under.x;
      b.y = restOn(b, under);
    } else {
      b.y = restOnSnow(b, surface);
    }
    done.add(b.id);
  };
  // Balls with nothing under them first, so stacks build from the bottom.
  for (const b of byId.values()) place(b);
  return balls.map((b) => byId.get(b.id)!);
}

/** Drop a carried ball at its current x: stack or fall. Returns the updated list. */
export function drop(id: number, balls: Ball[], surface: (x: number) => number): Ball[] {
  const b = balls.find((x) => x.id === id);
  if (!b) return balls;
  const others = balls.filter((x) => x.id !== id && !standsOn(x, id, balls));
  const under = landingBall(b, others);
  const next = balls.map((x) => (x.id === id ? { ...x, on: under ? under.id : null } : x));
  return settle(next, surface);
}

/** True when `b` stands (directly or higher up) on the ball `id`. */
export function standsOn(b: Ball, id: number, balls: Ball[]): boolean {
  const byId = new Map(balls.map((x) => [x.id, x]));
  let cur: Ball | undefined = b;
  let guard = 0;
  while (cur && cur.on !== null && guard++ < balls.length) {
    if (cur.on === id) return true;
    cur = byId.get(cur.on);
  }
  return false;
}

/** Picking up a ball: what stood on it is no longer supported and falls. */
export function pickUp(id: number, balls: Ball[]): Ball[] {
  return balls.map((x) => (x.on === id ? { ...x, on: null } : x));
}

export interface Snowman {
  base: number;
  middle: number;
  head: number;
}

/** Stacks of three, each smaller than the one below, are snowmen (base → head). */
export function snowmen(balls: Ball[]): Snowman[] {
  const out: Snowman[] = [];
  for (const head of balls) {
    const mid = balls.find((b) => b.id === head.on);
    if (!mid || mid.r <= head.r) continue;
    const base = balls.find((b) => b.id === mid.on);
    if (!base || base.r <= mid.r) continue;
    // The head must be the top of the stack.
    if (balls.some((b) => b.on === head.id)) continue;
    out.push({ base: base.id, middle: mid.id, head: head.id });
  }
  return out;
}

/** Keeps at most MAX_BALLS: the oldest loose balls melt first. */
export function cap(balls: Ball[]): Ball[] {
  let list = balls;
  while (list.length > MAX_BALLS) {
    const loose = list.find((b) => !list.some((o) => o.on === b.id)) ?? list[0];
    list = list.filter((b) => b.id !== loose.id).map((b) => (b.on === loose.id ? { ...b, on: null } : b));
  }
  return list;
}

/** Saved form: x as a fraction of the width so a resized tile keeps the scene. */
export interface SavedBall {
  id: number;
  fx: number;
  r: number;
  on: number | null;
}

export function save(balls: Ball[], width: number): SavedBall[] {
  return balls.map((b) => ({ id: b.id, fx: width > 0 ? +(b.x / width).toFixed(4) : 0, r: +b.r.toFixed(1), on: b.on }));
}

export function load(saved: unknown, width: number, surface: (x: number) => number): Ball[] {
  if (!Array.isArray(saved)) return [];
  const balls: Ball[] = [];
  for (const s of saved) {
    if (!s || typeof s !== 'object') continue;
    const { id, fx, r, on } = s as SavedBall;
    if (!Number.isFinite(id) || !Number.isFinite(fx) || !Number.isFinite(r)) continue;
    balls.push({ id, x: Math.max(0, Math.min(1, fx)) * width, y: 0, r: Math.max(R_MIN, Math.min(R_MAX, r)), on: Number.isFinite(on) ? on : null, roll: 0 });
  }
  return settle(cap(balls), surface);
}
