/**
 * Snowflake motion for Snowfall.svelte: plain numbers, frame-rate independent.
 * Three depths: far (tiny, slow, dim), mid, and near (big soft flakes close to the
 * glass, few of them). Wind from the pointer: a quick sweep blows the snow
 * sideways, and flakes drift out of the way of the cursor.
 */

export type Layer = 0 | 1 | 2;

export interface Flake {
  layer: Layer;
  x: number;
  y: number;
  /** Radius, px. */
  r: number;
  /** Fall speed, px/s. */
  vy: number;
  /** Sideways velocity from wind and the cursor, px/s (eases back to 0). */
  vx: number;
  /** Sway amplitude, px/s, and its phase. */
  sway: number;
  phase: number;
  alpha: number;
  /** Crystal angle and its slow spin, rad and rad/s. */
  rot: number;
  spin: number;
}

export interface Pointer {
  x: number;
  y: number;
}

/** How each depth looks and moves. */
export const LAYERS: Record<Layer, { r: [number, number]; vy: [number, number]; alpha: [number, number]; wind: number }> = {
  0: { r: [1.6, 2.6], vy: [10, 20], alpha: [0.3, 0.5], wind: 0.35 },
  1: { r: [3.5, 6.5], vy: [24, 44], alpha: [0.6, 0.95], wind: 0.7 },
  2: { r: [5, 9], vy: [55, 80], alpha: [0.12, 0.22], wind: 1 },
};

/** Flakes per depth: about one per 16 000 px² in all, at most 140; near flakes stay rare. */
export function layerCounts(w: number, h: number): [number, number, number] {
  const total = Math.max(0, Math.min(140, Math.round((w * h) / 16000)));
  const near = Math.min(7, Math.round(total * 0.06));
  const far = Math.round((total - near) * 0.55);
  return [far, total - near - far, near];
}

export function flakeCount(w: number, h: number): number {
  const [a, b, c] = layerCounts(w, h);
  return a + b + c;
}

const lerp = (a: [number, number], k: number) => a[0] + (a[1] - a[0]) * k;

export function makeFlake(w: number, h: number, rnd: () => number, anywhere = false, layer: Layer = 1): Flake {
  const L = LAYERS[layer];
  const k = rnd();
  const r = lerp(L.r, k);
  return {
    layer,
    x: rnd() * w,
    y: anywhere ? rnd() * h : -r * 2,
    r,
    vy: lerp(L.vy, k),
    vx: 0,
    sway: 6 + rnd() * 14,
    phase: rnd() * Math.PI * 2,
    alpha: lerp(L.alpha, k),
    rot: rnd() * Math.PI,
    spin: (rnd() - 0.5) * 1.2,
  };
}

/** Radius around the cursor that flakes drift out of, px. */
export const CURSOR_R = 110;

/**
 * Moves a flake by dt seconds under `wind` (px/s, from the pointer's sweep); flakes
 * near `pointer` are pushed away from it. A flake that left the window comes back
 * at the top.
 */
export function stepFlake(
  f: Flake,
  dt: number,
  t: number,
  w: number,
  h: number,
  rnd: () => number,
  wind = 0,
  pointer: Pointer | null = null
): void {
  const L = LAYERS[f.layer];
  if (pointer) {
    const dx = f.x - pointer.x;
    const dy = f.y - pointer.y;
    const d2 = dx * dx + dy * dy;
    if (d2 < CURSOR_R * CURSOR_R && d2 > 0.01) {
      const d = Math.sqrt(d2);
      // Up to 220 px/s at the cursor, nothing at the edge of the circle.
      const push = (1 - d / CURSOR_R) * 220 * L.wind * dt;
      f.x += (dx / d) * push;
      f.y += (dy / d) * push;
    }
  }
  // Sideways velocity eases toward the wind (about 0.6 s).
  f.vx += (wind * L.wind - f.vx) * Math.min(1, dt / 0.6);
  f.y += f.vy * dt;
  f.rot += f.spin * dt;
  f.x += (Math.sin(t * 0.8 + f.phase) * f.sway + f.vx) * dt;
  const m = f.r * 2 + 8;
  if (f.y > h + m) {
    Object.assign(f, makeFlake(w, h, rnd, false, f.layer));
  } else if (f.x < -m) f.x = w + m;
  else if (f.x > w + m) f.x = -m;
}

/**
 * A six-armed snow crystal on a square canvas of `size` px, centred: each arm
 * has two pairs of side branches at 60°, like a real dendrite flake.
 */
export function drawCrystal(c: CanvasRenderingContext2D, size: number, color: string): void {
  const R = size / 2;
  c.save();
  c.translate(R, R);
  c.strokeStyle = color;
  c.lineCap = 'round';
  c.lineWidth = Math.max(1, size * 0.07);
  c.shadowColor = color;
  c.shadowBlur = size * 0.08;
  const arm = R * 0.86;
  for (let i = 0; i < 6; i++) {
    c.save();
    c.rotate((i * Math.PI) / 3);
    c.beginPath();
    c.moveTo(0, 0);
    c.lineTo(0, -arm);
    for (const [at, len] of [
      [0.42, 0.3],
      [0.68, 0.22],
    ] as const) {
      const y = -arm * at;
      const l = arm * len;
      c.moveTo(0, y);
      c.lineTo(-l * Math.sin(Math.PI / 3), y - l * Math.cos(Math.PI / 3));
      c.moveTo(0, y);
      c.lineTo(l * Math.sin(Math.PI / 3), y - l * Math.cos(Math.PI / 3));
    }
    c.stroke();
    c.restore();
  }
  c.restore();
}
