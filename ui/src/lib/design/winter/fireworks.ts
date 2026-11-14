/** Fireworks physics for Fireworks.svelte: plain numbers, frame-rate independent. */

/** Apple system colours (dark appearance values). */
export const FW_COLORS = ['#ff453a', '#ff9f0a', '#ffd60a', '#30d158', '#64d2ff', '#0a84ff', '#bf5af2', '#ff375f'] as const;

export const GRAVITY = 120; // px/s²

export interface Rocket {
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Height (y) where it bursts. */
  burstY: number;
  color: string;
}

export interface Spark {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  max: number;
  color: string;
  size: number;
}

export function launchRocket(w: number, h: number, rnd: () => number): Rocket {
  const x = w * (0.15 + rnd() * 0.7);
  return {
    x,
    y: h + 4,
    vx: (rnd() - 0.5) * 40,
    vy: -(h * 0.75 + rnd() * h * 0.35),
    burstY: h * (0.15 + rnd() * 0.3),
    color: FW_COLORS[Math.floor(rnd() * FW_COLORS.length)],
  };
}

/** A burst: a ring of sparks with a little randomness, some in a second colour. */
export function burst(r: Rocket, rnd: () => number, count = 96): Spark[] {
  const second = FW_COLORS[Math.floor(rnd() * FW_COLORS.length)];
  const speed = 210 + rnd() * 130;
  return Array.from({ length: count }, (_, i) => {
    const a = (i / count) * Math.PI * 2 + rnd() * 0.12;
    const v = speed * (0.55 + rnd() * 0.45);
    const max = 1.3 + rnd() * 0.9;
    return { x: r.x, y: r.y, vx: Math.cos(a) * v, vy: Math.sin(a) * v, life: max, max, color: i % 4 === 0 ? second : r.color, size: 2 + rnd() * 1.8 };
  });
}

/** Moves a rocket; true when it reached its burst height (or started falling). */
export function stepRocket(r: Rocket, dt: number): boolean {
  r.vy += GRAVITY * dt;
  r.x += r.vx * dt;
  r.y += r.vy * dt;
  return r.y <= r.burstY || r.vy >= 0;
}

/** Moves a spark with gravity and air drag; false when it has burnt out. */
export function stepSpark(s: Spark, dt: number): boolean {
  const drag = Math.exp(-dt * 1.6);
  s.vx *= drag;
  s.vy = s.vy * drag + GRAVITY * 0.6 * dt;
  s.x += s.vx * dt;
  s.y += s.vy * dt;
  s.life -= dt;
  return s.life > 0;
}
