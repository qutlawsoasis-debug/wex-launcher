/** Hill shapes for SnowDrift.svelte: smooth, uneven, fixed per seed. */

/** A small seeded generator (mulberry32). */
export function seeded(seed: string): () => number {
  let h = 1779033703 ^ seed.length;
  for (let i = 0; i < seed.length; i++) h = Math.imul(h ^ seed.charCodeAt(i), 3432918353), (h = (h << 13) | (h >>> 19));
  let a = h >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * The top edge of a drift: a sum of a few waves of random length and phase,
 * between `low` and `high` (fractions of H measured from the bottom).
 */
export function driftHeights(seed: string, n: number, low: number, high: number, span = 1): number[] {
  const rnd = seeded(seed);
  // A long swell, a few drifts and small lumps on them.
  const waves = [
    { f: 0.6 + rnd() * 0.6, a: 1 },
    { f: 2 + rnd() * 1.5, a: 0.7 },
    { f: 5 + rnd() * 3, a: 0.28 },
    { f: 11 + rnd() * 6, a: 0.1 },
  ].map((w) => ({ ...w, f: Math.max(0.35, w.f * span), p: rnd() * Math.PI * 2 }));
  const raw = Array.from({ length: n + 1 }, (_, i) => waves.reduce((s, w) => s + w.a * Math.sin((i / n) * Math.PI * 2 * w.f + w.p), 0));
  const mn = Math.min(...raw);
  const mx = Math.max(...raw);
  return raw.map((v) => low + ((v - mn) / (mx - mn || 1)) * (high - low));
}

/** SVG path of a drift filling the bottom of a W×H box, smoothed with quadratic curves. */
export function driftPath(seed: string, W: number, H: number, low: number, high: number, n = 48, span = 1): string {
  const hs = driftHeights(seed, n, low, high, span);
  const pts = hs.map((k, i) => [(i / n) * W, H - k * H] as const);
  let d = `M0 ${H} L${pts[0][0]} ${pts[0][1].toFixed(1)}`;
  for (let i = 1; i < pts.length; i++) {
    const [x0, y0] = pts[i - 1];
    const [x1, y1] = pts[i];
    d += ` Q${x0.toFixed(1)} ${y0.toFixed(1)} ${((x0 + x1) / 2).toFixed(1)} ${((y0 + y1) / 2).toFixed(1)}`;
  }
  const last = pts[pts.length - 1];
  return d + ` L${last[0]} ${last[1].toFixed(1)} L${W} ${H} Z`;
}

/** Glints on the front drift: near its top edge, with staggered twinkle delays. */
export function glints(seed: string, W: number, H: number, count: number, span = 1): Array<{ x: number; y: number; delay: number }> {
  const rnd = seeded(seed + ':glint');
  const hs = driftHeights(seed, 48, 0.22, 0.9, span);
  return Array.from({ length: count }, () => {
    const u = rnd();
    const i = Math.min(47, Math.floor(u * 48));
    const k = hs[i] + (hs[i + 1] - hs[i]) * (u * 48 - i);
    return { x: u * W, y: H - k * H + 3 + rnd() * 6, delay: +(rnd() * 3.6).toFixed(2) };
  });
}
