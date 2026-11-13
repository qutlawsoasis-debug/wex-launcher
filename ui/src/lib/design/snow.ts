/** Snowflake motion for Snowfall.svelte: plain numbers, frame-rate independent. */

export interface Flake {
  x: number;
  y: number;
  /** Radius, px. */
  r: number;
  /** Fall speed, px/s (bigger flakes are nearer and fall faster). */
  vy: number;
  /** Sideways sway amplitude, px/s, and its phase. */
  sway: number;
  phase: number;
  alpha: number;
}

/** About one flake per 16 000 px² of window, at most 120: light, never a blizzard. */
export function flakeCount(w: number, h: number): number {
  return Math.max(0, Math.min(120, Math.round((w * h) / 16000)));
}

export function makeFlake(w: number, h: number, rnd: () => number, anywhere = false): Flake {
  const depth = rnd();
  return {
    x: rnd() * w,
    y: anywhere ? rnd() * h : -4,
    r: 0.8 + depth * 1.8,
    vy: 16 + depth * 38,
    sway: 6 + rnd() * 14,
    phase: rnd() * Math.PI * 2,
    alpha: 0.35 + depth * 0.5,
  };
}

/** Moves a flake by dt seconds; a flake that left the window comes back at the top. */
export function stepFlake(f: Flake, dt: number, t: number, w: number, h: number, rnd: () => number): void {
  f.y += f.vy * dt;
  f.x += Math.sin(t * 0.8 + f.phase) * f.sway * dt;
  if (f.y > h + 4 || f.x < -8 || f.x > w + 8) {
    Object.assign(f, makeFlake(w, h, rnd));
  }
}
