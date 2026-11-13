/**
 * Layout for Garland.svelte: a wire hung from pins along the top edge, sagging
 * between them, with bulbs hanging from the wire. All in px for a given width,
 * so nothing is stretched.
 */

/** Apple system colours (dark appearance values, they read best on the build colour). */
export const BULB_COLORS = ['#ff453a', '#ffd60a', '#30d158', '#0a84ff', '#ff9f0a', '#bf5af2'] as const;

export interface Bulb {
  x: number;
  y: number;
  /** Tilt along the wire, degrees. */
  angle: number;
  color: string;
  /** Position in the chase (twinkle order). */
  i: number;
}

export interface GarlandLayout {
  wire: string;
  pins: number[];
  bulbs: Bulb[];
}

export interface GarlandOptions {
  /** Distance between bulbs, px. */
  step?: number;
  /** Bulbs per sagging span. */
  perSpan?: number;
  /** Pin height and how far the wire sags, px. */
  top?: number;
  sag?: number;
}

/** A point and the slope on the quadratic curve from (x0,top) to (x1,top) through the sag. */
function onSpan(x0: number, x1: number, top: number, sag: number, t: number): { x: number; y: number; slope: number } {
  // Control point at twice the sag gives a lowest point of `sag` below the pins.
  const cx = (x0 + x1) / 2;
  const cy = top + sag * 2;
  const x = (1 - t) * (1 - t) * x0 + 2 * (1 - t) * t * cx + t * t * x1;
  const y = (1 - t) * (1 - t) * top + 2 * (1 - t) * t * cy + t * t * top;
  const dx = 2 * (1 - t) * (cx - x0) + 2 * t * (x1 - cx);
  const dy = 2 * (1 - t) * (cy - top) + 2 * t * (top - cy);
  return { x, y, slope: dy / dx };
}

export function garlandLayout(width: number, o: GarlandOptions = {}): GarlandLayout {
  const step = o.step ?? 46;
  const perSpan = o.perSpan ?? 4;
  const top = o.top ?? 2;
  const sag = o.sag ?? 9;
  if (width <= 0) return { wire: '', pins: [], bulbs: [] };
  const spanW = step * perSpan;
  const spans = Math.max(1, Math.round(width / spanW));
  const w = width / spans;
  const pins = Array.from({ length: spans + 1 }, (_, i) => +(i * w).toFixed(1));
  let wire = `M0 ${top}`;
  const bulbs: Bulb[] = [];
  let k = 0;
  for (let s = 0; s < spans; s++) {
    const x0 = pins[s];
    const x1 = pins[s + 1];
    wire += ` Q${((x0 + x1) / 2).toFixed(1)} ${(top + sag * 2).toFixed(1)} ${x1.toFixed(1)} ${top}`;
    for (let b = 0; b < perSpan; b++) {
      const t = (b + 0.5) / perSpan;
      const p = onSpan(x0, x1, top, sag, t);
      bulbs.push({
        x: +p.x.toFixed(1),
        y: +p.y.toFixed(1),
        angle: +((Math.atan(p.slope) * 180) / Math.PI).toFixed(1),
        color: BULB_COLORS[k % BULB_COLORS.length],
        i: k,
      });
      k++;
    }
  }
  return { wire, pins, bulbs };
}
