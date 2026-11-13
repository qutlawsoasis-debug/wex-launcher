/**
 * Geometry for the winter night scene (WinterScene.svelte): stars, snowy firs and
 * snow hills, all from fixed seeds so the scene is the same on every start and is
 * computed once, never per frame. Pure functions, no DOM.
 */

/** Small fast deterministic PRNG (mulberry32); returns values in [0, 1). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const r1 = (n: number) => Math.round(n * 10) / 10;

export interface Star {
  /** Position in % of the window. */
  x: number;
  y: number;
  /** Diameter in px. */
  size: number;
  /** Resting opacity. */
  alpha: number;
  /** Twinkles slowly (CSS), off under Reduce Motion. */
  twinkle: boolean;
  /** Twinkle period and offset in seconds. */
  dur: number;
  delay: number;
}

/**
 * Stars over the sky. Denser near the top, none in the lowest band where the
 * snow lies. About one in eight twinkles.
 */
export function makeStars(count = 120, seed = 1201, maxY = 84): Star[] {
  const rand = rng(seed);
  const stars: Star[] = [];
  for (let i = 0; i < count; i++) {
    const x = rand() * 100;
    const y = Math.pow(rand(), 1.25) * maxY;
    const big = rand();
    const size = big > 0.9 ? 2.8 : big > 0.6 ? 2 : 1.5;
    const alpha = 0.45 + rand() * 0.5;
    const twinkle = rand() < 0.13;
    const dur = 3.5 + rand() * 4;
    const delay = -rand() * dur;
    stars.push({ x: r1(x), y: r1(y), size, alpha: r1(alpha * 10) / 10, twinkle, dur: r1(dur), delay: r1(delay) });
  }
  return stars;
}

export interface Fir {
  /** Silhouette (tiers and trunk). */
  body: string;
  /** Snow lying on top of each tier. */
  snow: string;
}

type P = [number, number];
const quadAt = (p0: P, p1: P, p2: P, s: number): P => {
  const u = 1 - s;
  return [u * u * p0[0] + 2 * u * s * p1[0] + s * s * p2[0], u * u * p0[1] + 2 * u * s * p1[1] + s * s * p2[1]];
};
const lerp = (a: P, b: P, s: number): P => [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
const pt = (p: P) => `${r1(p[0])} ${r1(p[1])}`;

/**
 * One snow-covered fir: stacked drooping tiers with a ragged lower edge, and a
 * snow cap on the upper part of every tier with scallops hanging down.
 * (cx, baseY) is the foot of the trunk; h the full height; w the widest span.
 */
export function firTree(cx: number, baseY: number, h: number, w: number, seed: number): Fir {
  const rand = rng(seed);
  const tiers = 5 + Math.floor(rand() * 3);
  const trunkH = h * 0.07;
  const apex = baseY - h;
  const crownH = h - trunkH;
  const step = crownH / tiers;
  const overlap = step * 0.6;
  let body = '';
  let snow = '';
  for (let i = 0; i < tiers; i++) {
    const t = i === 0 ? apex : apex + step * i - overlap;
    const b = apex + step * (i + 1);
    const th = b - t;
    const hw = (w / 2) * (0.22 + 0.78 * Math.pow((i + 1) / tiers, 0.9)) * (0.92 + rand() * 0.16);
    const top: P = [cx, t];
    const tipR: P = [cx + hw, b + th * 0.04];
    const tipL: P = [cx - hw, b + th * 0.04 * rand()];
    // Drooping sides: the edge bows inward and sweeps out at the tip.
    const cR: P = [cx + hw * 0.28, b - th * 0.18];
    const cL: P = [cx - hw * 0.28, b - th * 0.18];

    // Ragged lower edge, right tip to left tip: branch tips and notches.
    const teeth = 4 + Math.floor(rand() * 3);
    let edge = '';
    for (let j = 1; j < teeth * 2; j++) {
      const s = j / (teeth * 2);
      const x = tipR[0] + (tipL[0] - tipR[0]) * s;
      const arch = th * 0.1 * (1 - Math.pow((x - cx) / hw, 2));
      const notch = j % 2 === 1 ? th * (0.06 + rand() * 0.05) : -th * 0.02 * rand();
      edge += ` L${pt([x, b - arch - notch])}`;
    }
    body += `M${pt(top)} Q${pt(cR)} ${pt(tipR)}${edge} L${pt(tipL)} Q${pt(cL)} ${pt(top)} Z `;

    // Snow cap: the same sides down to s, then scallops back across.
    // The cap reaches almost to the branch tips and rises toward the trunk, so a
    // dark band of needles shows under the snow in the middle of every tier.
    const s = 0.86 + rand() * 0.06;
    const endR = quadAt(top, cR, tipR, s);
    const endL = quadAt(top, cL, tipL, s);
    const subR = lerp(top, cR, s);
    const subL = lerp(top, cL, s);
    const scallops = 4 + Math.floor(rand() * 3);
    const lift = th * (0.36 + rand() * 0.08);
    const lineY = (x: number) => {
      const k = (x - endL[0]) / (endR[0] - endL[0]);
      return endR[1] + (endL[1] - endR[1]) * (1 - k) - lift * Math.sin(Math.PI * k);
    };
    let sc = '';
    for (let j = 1; j <= scallops; j++) {
      const x0 = endR[0] + ((endL[0] - endR[0]) * (j - 1)) / scallops;
      const x1 = endR[0] + ((endL[0] - endR[0]) * j) / scallops;
      const xm = (x0 + x1) / 2;
      const sag = th * (0.08 + rand() * 0.08);
      sc += ` Q${pt([xm, lineY(xm) + sag])} ${pt([x1, lineY(x1)])}`;
    }
    snow += `M${pt(top)} Q${pt(subR)} ${pt(endR)}${sc} Q${pt(subL)} ${pt(top)} Z `;
  }
  const tw = Math.max(1.5, w * 0.045);
  body += `M${pt([cx - tw, baseY - trunkH - 2])} L${pt([cx + tw, baseY - trunkH - 2])} L${pt([cx + tw, baseY])} L${pt([cx - tw, baseY])} Z`;
  return { body: body.trim(), snow: snow.trim() };
}

export interface LayerSpec {
  /** How many trees. */
  count: number;
  /** Range of tree heights in viewBox units. */
  h: [number, number];
  /** Range of foot x positions in viewBox units. */
  x: [number, number];
  /** Foot y of the trees. */
  baseY: number;
  /** Width/height ratio of a tree. */
  ratio?: number;
}

export interface Tree extends Fir {
  x: number;
  h: number;
}

/** Trees of one depth layer, spread evenly over the x range with jitter, sorted back to front by height. */
export function forestLayer(spec: LayerSpec, seed: number): Tree[] {
  const rand = rng(seed);
  const trees: Tree[] = [];
  const [x0, x1] = spec.x;
  const span = (x1 - x0) / Math.max(1, spec.count);
  for (let i = 0; i < spec.count; i++) {
    const x = x0 + span * (i + 0.5) + (rand() - 0.5) * span * 0.7;
    const h = spec.h[0] + rand() * (spec.h[1] - spec.h[0]);
    const w = h * (spec.ratio ?? 0.46) * (0.9 + rand() * 0.2);
    const fir = firTree(x, spec.baseY + rand() * 6, h, w, Math.floor(rand() * 1e9));
    trees.push({ ...fir, x: r1(x), h: r1(h) });
  }
  return trees.sort((a, b) => a.h - b.h);
}

/**
 * A snow hill as a closed path: a smooth rolling top edge over [0, width], filled
 * down to bottomY. `rise` lifts one or both ends (drifts banked into the corners).
 */
export function hillPath(
  width: number,
  topY: number,
  bottomY: number,
  amp: number,
  seed: number,
  rise: { left?: number; right?: number } = {}
): string {
  const rand = rng(seed);
  const n = 8;
  const pts: P[] = [];
  for (let i = 0; i <= n; i++) {
    const k = i / n;
    const lift = (rise.left ?? 0) * Math.pow(1 - k, 3) + (rise.right ?? 0) * Math.pow(k, 3);
    pts.push([width * k, topY + (rand() - 0.5) * 2 * amp - lift]);
  }
  // Smooth through the midpoints (quadratic chain).
  let d = `M0 ${r1(bottomY)} L${pt(pts[0])}`;
  for (let i = 1; i < n; i++) {
    const mid = lerp(pts[i], pts[i + 1], 0.5);
    d += ` Q${pt(pts[i])} ${pt(mid)}`;
  }
  d += ` L${pt(pts[n])} L${r1(width)} ${r1(bottomY)} Z`;
  return d;
}

/**
 * A drift at the foot of a corner forest: high at the window edge, sinking to
 * the ground line toward the open side so no hard edge shows.
 * side 'right': the drift is banked against x = width.
 */
export function driftPath(width: number, baseY: number, height: number, seed: number, side: 'left' | 'right'): string {
  const rand = rng(seed);
  const n = 7;
  let d = `M0 ${r1(baseY + 44)}`;
  const pts: P[] = [];
  for (let i = 0; i <= n; i++) {
    const k = i / n;
    const toward = side === 'right' ? k : 1 - k;
    const env = toward * toward * (3 - 2 * toward);
    // Sinks below the ground line on the open side, so it fades under the ground.
    pts.push([width * k, baseY + 40 * (1 - env) - height * env * (0.75 + rand() * 0.25)]);
  }
  d += ` L${pt(pts[0])}`;
  for (let i = 1; i < n; i++) d += ` Q${pt(pts[i])} ${pt(lerp(pts[i], pts[i + 1], 0.5))}`;
  d += ` L${pt(pts[n])} L${r1(width)} ${r1(baseY + 44)} Z`;
  return d;
}

export interface Corner {
  /** viewBox width and height. */
  w: number;
  h: number;
  /** Depth layers back to front: 0 far, 1 mid, 2 near. */
  layers: Array<{ trees: Tree[]; drift: string }>;
}

/** Bottom-right forest: the big one, with a near fir cut by the window edge. */
export function rightCorner(seed = 7): Corner {
  const w = 640;
  const h = 720;
  const base = h - 30;
  return {
    w,
    h,
    layers: [
      { trees: forestLayer({ count: 7, h: [200, 300], x: [120, 640], baseY: base - 40 }, seed), drift: driftPath(w, base - 30, 70, seed + 1, 'right') },
      { trees: forestLayer({ count: 4, h: [320, 450], x: [250, 640], baseY: base - 10 }, seed + 2), drift: driftPath(w, base, 90, seed + 3, 'right') },
      { trees: forestLayer({ count: 2, h: [560, 680], x: [470, 700], baseY: base + 20, ratio: 0.5 }, seed + 4), drift: driftPath(w, base + 20, 110, seed + 5, 'right') },
    ],
  };
}

/** Bottom-left forest under the sidebar: smaller, partly showing. */
export function leftCorner(seed = 31): Corner {
  const w = 420;
  const h = 480;
  const base = h - 24;
  return {
    w,
    h,
    layers: [
      { trees: forestLayer({ count: 4, h: [140, 210], x: [0, 330], baseY: base - 30 }, seed), drift: driftPath(w, base - 22, 50, seed + 1, 'left') },
      { trees: forestLayer({ count: 2, h: [230, 300], x: [0, 220], baseY: base - 8 }, seed + 2), drift: driftPath(w, base, 64, seed + 3, 'left') },
      { trees: forestLayer({ count: 1, h: [360, 400], x: [-40, 60], baseY: base + 16, ratio: 0.5 }, seed + 4), drift: driftPath(w, base + 16, 80, seed + 5, 'left') },
    ],
  };
}
