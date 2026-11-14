/**
 * Geometry for the decorated tree on the open build tile (EveningScene.svelte).
 * viewBox 0 0 220 280, the tree centred at x = 110.
 */

export const CX = 110;

export interface Tier {
  top: number;
  base: number;
  hw: number;
}

/** Four tiers, each wider and lower, overlapping like a real fir. */
export const TIERS: Tier[] = [
  { top: 30, base: 96, hw: 38 },
  { top: 62, base: 146, hw: 58 },
  { top: 106, base: 196, hw: 78 },
  { top: 150, base: 240, hw: 96 },
];

/** A tier: straight sides from the apex, a drooping scalloped lower edge. */
export function tierPath(t: Tier, scallops = 6): string {
  const l = CX - t.hw;
  const w = (t.hw * 2) / scallops;
  let d = `M${CX} ${t.top} L${l} ${t.base}`;
  for (let i = 0; i < scallops; i++) {
    const x0 = l + i * w;
    d += ` Q${(x0 + w / 2).toFixed(1)} ${(t.base + 9).toFixed(1)} ${(x0 + w).toFixed(1)} ${t.base}`;
  }
  return d + ' Z';
}

/** Snow resting along the lower edge of a tier. */
export function snowPath(t: Tier, scallops = 6): string {
  const l = CX - t.hw + 3;
  const r = CX + t.hw - 3;
  const w = (r - l) / scallops;
  let d = `M${l} ${t.base - 2}`;
  for (let i = 0; i < scallops; i++) {
    const x0 = l + i * w;
    d += ` Q${(x0 + w / 2).toFixed(1)} ${(t.base + 7).toFixed(1)} ${(x0 + w).toFixed(1)} ${t.base - 2}`;
  }
  for (let i = scallops; i > 0; i--) {
    const x0 = l + i * w;
    d += ` Q${(x0 - w / 2).toFixed(1)} ${(t.base + 2).toFixed(1)} ${(x0 - w).toFixed(1)} ${t.base - 3}`;
  }
  return d + ' Z';
}

/** Half width of the tree at height y (inside the tier that covers it). */
export function halfWidthAt(y: number): number {
  let best = 0;
  for (const t of TIERS) {
    if (y < t.top || y > t.base) continue;
    best = Math.max(best, ((y - t.top) / (t.base - t.top)) * t.hw);
  }
  return best;
}

export interface Swag {
  /** SVG path of the swag (sagging across the tree). */
  d: string;
  points: Array<{ x: number; y: number }>;
}

/** Garland swags: across each tier, sagging, inside the outline. */
export function swags(perSwag = 7): Swag[] {
  const ys = [82, 128, 178, 224];
  return ys.map((y, i) => {
    const hw = halfWidthAt(y) * 0.92;
    const x0 = CX - hw;
    const x1 = CX + hw;
    const sag = 8 + i * 2;
    const cy = y - 10 + sag * 2;
    const points = Array.from({ length: perSwag }, (_, k) => {
      const t = (k + 0.5) / perSwag;
      const x = (1 - t) * (1 - t) * x0 + 2 * (1 - t) * t * CX + t * t * x1;
      const yy = (1 - t) * (1 - t) * (y - 10) + 2 * (1 - t) * t * cy + t * t * (y - 10);
      return { x: +x.toFixed(1), y: +yy.toFixed(1) };
    });
    return { d: `M${x0.toFixed(1)} ${y - 10} Q${CX} ${cy} ${x1.toFixed(1)} ${y - 10}`, points };
  });
}
