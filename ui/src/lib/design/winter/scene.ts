/**
 * Geometry for the winter night scene (WinterScene.svelte): stars, snow-laden
 * spruces in three depths and snow hills, all from fixed seeds so the scene is the same on every start and is
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

/** Level of detail of a fir: 0 far (hazy silhouette), 1 mid, 2 near (needles, trunk). */
export type Detail = 0 | 1 | 2;

/** One branch tier of a fir, in paint order (the lowest tier is painted first). */
export interface FirTier {
  /** Soft shadow this tier casts on the tier under it (painted before its body). */
  shade: string;
  /** The tier silhouette: drooping upper surface, ragged needle edge below. */
  body: string;
  /** Short needle strokes over the tier (empty on far trees). */
  needles: string;
  /** Snow clumps lying on the upper surface. */
  snow: string;
  /** Thin moonlit rim along the top of the snow on the lit side (empty on far trees). */
  rim: string;
  /** Where the tier joins the trunk: the pivot of its sway, in viewBox units. */
  ox: number;
  oy: number;
}

export interface Fir {
  tiers: FirTier[];
  /** A bit of trunk under the lowest tier (near trees only, otherwise empty). */
  trunk: string;
  /** Snow heaped round the foot. */
  base: string;
  /** Branch tips where a little snow slides off now and then (near and mid trees). */
  falls: Array<{ x: number; y: number }>;
}

type P = [number, number];
const lerp = (a: P, b: P, s: number): P => [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
const cubicAt = (p0: P, p1: P, p2: P, p3: P, s: number): P => {
  const u = 1 - s;
  const a = u * u * u;
  const b = 3 * u * u * s;
  const c = 3 * u * s * s;
  const d = s * s * s;
  return [a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]];
};
const pt = (p: P) => `${r1(p[0])} ${r1(p[1])}`;

/** A smooth closed-ish chain through points: quadratic segments through the midpoints. */
function smooth(points: P[]): string {
  let d = '';
  for (let i = 1; i < points.length - 1; i++) d += ` Q${pt(points[i])} ${pt(lerp(points[i], points[i + 1], 0.5))}`;
  return d + ` L${pt(points[points.length - 1])}`;
}

/**
 * One snow-laden spruce seen at night. (cx, baseY) is the foot of the trunk, h the
 * full height, w the widest span; `light` is the side the moonlight comes from
 * (-1 left, 1 right). The crown is 8 to 14 drooping tiers that widen downwards,
 * each with a ragged, needle-like lower edge made of hanging sub-branches, heavy
 * uneven snow on its upper surface and a soft shadow cast on the tier below.
 */
export function firTree(cx: number, baseY: number, h: number, w: number, seed: number, detail: Detail = 2, light: -1 | 1 = -1): Fir {
  const rand = rng(seed);
  const n = detail === 0 ? 8 + Math.floor(rand() * 2) : detail === 1 ? 10 + Math.floor(rand() * 3) : 12 + Math.floor(rand() * 3);
  const trunkH = h * 0.075;
  const apex = baseY - h;
  const crownH = h - trunkH;
  const lean = (rand() - 0.5) * w * 0.05;
  const lobes = detail === 0 ? 3 : detail === 1 ? 4 : 5;
  const perLobe = detail === 0 ? 2 : 3;
  const needleCount = detail === 0 ? 0 : detail === 1 ? 10 : 40;

  const bottoms: number[] = [];
  for (let i = 0; i < n; i++) bottoms.push(apex + crownH * Math.pow((i + 1) / n, 0.94));

  const tiers: FirTier[] = [];
  const falls: Array<{ x: number; y: number }> = [];
  let lowest = { b: baseY - trunkH, arch: 0 };
  for (let i = 0; i < n; i++) {
    const f = (i + 1) / n;
    const b = bottoms[i];
    const prevB = i === 0 ? apex : bottoms[i - 1];
    const step = b - prevB;
    const t = i === 0 ? apex : Math.max(apex, prevB - step * (0.95 + rand() * 0.3));
    const th = b - t;
    const ax = cx + lean * (1 - f);
    const hw = (w / 2) * (0.1 + 0.9 * Math.pow(f, 0.88)) * (0.84 + rand() * 0.3);
    const hwR = hw * (rand() < 0.15 ? 0.78 : 0.9 + rand() * 0.2);
    const hwL = hw * (rand() < 0.15 ? 0.78 : 0.9 + rand() * 0.2);
    const top: P = [ax, t];
    const tipR: P = [ax + hwR, b + th * 0.06 * rand()];
    const tipL: P = [ax - hwL, b + th * 0.06 * rand()];
    // The upper surface: out from the trunk, bowed up, drooping at the tip.
    const side = (dir: 1 | -1, tip: P, hwS: number): [P, P, P, P] => [
      top,
      [ax + dir * hwS * 0.42, t + th * (0.26 + rand() * 0.08)],
      [ax + dir * hwS * 0.84, tip[1] - th * (0.32 + rand() * 0.08)],
      tip,
    ];
    const sR = side(1, tipR, hwR);
    const sL = side(-1, tipL, hwL);
    const archH = th * (0.3 + rand() * 0.08);
    const archY = (x: number) => {
      const u = x < ax ? (ax - x) / hwL : (x - ax) / hwR;
      return archH * (1 - Math.min(1, u) * Math.min(1, u));
    };

    // Ragged lower edge, right tip to left tip: hanging sub-branches (lobes) of fine needles.
    const edgePts: P[] = [];
    const total = lobes * 2 * perLobe;
    for (let j = 1; j < total; j++) {
      const s = j / total;
      const x = tipR[0] + (tipL[0] - tipR[0]) * s + (rand() - 0.5) * (hw / total) * 0.8;
      const lobePos = (s * lobes) % 1;
      const sag = th * 0.09 * Math.sin(Math.PI * lobePos) * (0.6 + rand() * 0.8);
      const needle = j % 2 === 1 ? th * (0.03 + rand() * 0.06) : -th * 0.015 * rand();
      const yBase = tipR[1] + (tipL[1] - tipR[1]) * s;
      edgePts.push([x, yBase - archY(x) + sag + needle]);
    }
    let body = `M${pt(top)} C${pt(sR[1])} ${pt(sR[2])} ${pt(sR[3])}`;
    // Branch tip: a few longer drooping needles.
    body += ` L${pt([tipR[0] + hw * 0.02, tipR[1] + th * 0.07])} L${pt([tipR[0] - hw * 0.04, tipR[1] + th * 0.02])}`;
    for (const p of edgePts) body += ` L${pt(p)}`;
    body += ` L${pt([tipL[0] + hw * 0.04, tipL[1] + th * 0.02])} L${pt([tipL[0] - hw * 0.02, tipL[1] + th * 0.07])} L${pt(tipL)}`;
    body += ` C${pt(sL[2])} ${pt(sL[1])} ${pt(top)} Z`;

    // Shadow on the tier below: just under this tier's edge, deepest near the trunk.
    let shade = '';
    if (i < n - 1) {
      const sparse = edgePts.filter((_, k) => k % 2 === 0);
      const down = (p: P): P => {
        const u = Math.min(1, Math.abs(p[0] - ax) / hw);
        return [ax + (p[0] - ax) * 0.9, p[1] + th * 0.32 * Math.pow(1 - u * u, 0.8) + th * 0.04];
      };
      // Starts a little above the needle edge so the gaps between needles are in shadow too.
      const up = (p: P): P => [p[0], p[1] - th * 0.1];
      shade = `M${pt(up(sparse[0]))}`;
      for (const p of sparse.slice(1)) shade += ` L${pt(up(p))}`;
      const back = [...sparse].reverse().map(down);
      shade += smooth(back) + ' Z';
    }

    // Snow on the upper surface: two or three heavy clumps per side with needles
    // showing between them; lumpy on top, hanging and rounded underneath, thickest
    // half-way out along the branch, gone at the drooping tip.
    let snow = '';
    let rim = '';
    const litSide = light < 0 ? -1 : 1;
    const clump = (s3: [P, P, P, P], dir: 1 | -1, sa: number, sb: number, end: number) => {
      const k = detail === 0 ? 2 : 3;
      const upper: P[] = [];
      const lower: P[] = [];
      for (let j = 0; j <= k; j++) {
        const s = sa + ((sb - sa) * j) / k;
        const p = cubicAt(s3[0], s3[1], s3[2], s3[3], s);
        const bulk = Math.sin(Math.PI * Math.min(1, (s + 0.08) / (end + 0.08))) * (0.6 + rand() * 0.7);
        upper.push([p[0], p[1] - th * (0.02 + 0.06 * bulk * rand())]);
        lower.push([p[0] + dir * th * 0.02 * (rand() - 0.5), p[1] + th * (0.05 + 0.2 * bulk)]);
      }
      const last = upper[k];
      const lastLow = lower[k];
      // Rounded start too, so no clump has a cut edge.
      let d = `M${pt(lower[0])} Q${pt([upper[0][0] - dir * th * 0.05, (upper[0][1] + lower[0][1]) / 2])} ${pt(upper[0])}` + smooth(upper);
      // Rounded end of the clump, bulging out along the branch.
      d += ` Q${pt([last[0] + dir * th * 0.07, (last[1] + lastLow[1]) / 2 + th * 0.04])} ${pt(lastLow)}`;
      for (let j = k - 1; j >= 0; j--) {
        const m = lerp(lower[j + 1], lower[j], 0.5);
        d += ` Q${pt([m[0] + dir * th * 0.03 * (rand() - 0.5), m[1] + th * (0.01 + rand() * 0.05)])} ${pt(lower[j])}`;
      }
      snow += d + ' Z ';
      if (detail > 0 && dir === litSide) rim += `M${pt(upper[0])}` + smooth(upper) + ' ';
    };
    for (const [s3, dir] of [
      [sL, -1],
      [sR, 1],
    ] as Array<[[P, P, P, P], 1 | -1]>) {
      const end = 0.7 + rand() * 0.2;
      const parts = detail === 0 ? 1 : 2 + Math.floor(rand() * 2);
      let s = 0;
      for (let c = 0; c < parts; c++) {
        const len = (end - s) / (parts - c);
        const sb = c === parts - 1 ? end : s + len * (0.75 + rand() * 0.25);
        clump(s3, dir, s, sb, end);
        s = sb + 0.03 + rand() * 0.05;
        if (s >= end - 0.05) break;
      }
    }
    snow = snow.trim();
    rim = rim.trim();

    // Needles: many short strokes hanging down and outward over the dark
    // underside, in little sprays, denser towards the branch tips.
    let needles = '';
    for (let j = 0; j < needleCount; j++) {
      const dir: 1 | -1 = rand() < 0.5 ? 1 : -1;
      const s3 = dir > 0 ? sR : sL;
      const s = 0.3 + Math.sqrt(rand()) * 0.68;
      const p = cubicAt(s3[0], s3[1], s3[2], s3[3], s);
      const bottom = b - archY(p[0]);
      const y = p[1] + th * 0.12 + Math.max(0, bottom - p[1] - th * 0.12) * rand();
      const len = th * (0.035 + rand() * 0.045);
      const ang = 0.7 + rand() * 0.8; // radians below horizontal
      needles += `M${pt([p[0], y])}l${r1(dir * Math.cos(ang) * len)} ${r1(Math.sin(ang) * len)}`;
    }
    needles = needles.trim();

    if (detail > 0 && i >= n - 5 && rand() < 0.45 && falls.length < 2) {
      const tipSide = light < 0 ? sL : sR;
      const p = cubicAt(tipSide[0], tipSide[1], tipSide[2], tipSide[3], 0.8);
      falls.push({ x: r1(p[0]), y: r1(p[1]) });
    }

    tiers.push({ shade, body, needles, snow, rim, ox: r1(ax), oy: r1(t) });
    if (i === n - 1) lowest = { b, arch: archH };
  }

  let trunk = '';
  if (detail === 2) {
    const tw = Math.max(1.5, w * 0.035);
    const top = lowest.b - lowest.arch - 4;
    trunk = `M${pt([cx - tw * 0.8, top])} L${pt([cx + tw * 0.8, top])} L${pt([cx + tw * 1.15, baseY + 2])} L${pt([cx - tw * 1.15, baseY + 2])} Z`;
  }
  // A heap of snow round the foot, lumpy on top.
  const bw = w * (0.45 + rand() * 0.1);
  const bh = h * 0.035;
  const mound: P[] = [];
  for (let j = 0; j <= 6; j++) {
    const k = j / 6;
    mound.push([cx - bw + 2 * bw * k, baseY + 4 - bh * Math.sin(Math.PI * k) * (0.7 + rand() * 0.5)]);
  }
  const base = `M${pt([cx - bw, baseY + 8])} L${pt(mound[0])}` + smooth(mound) + ` L${pt([cx + bw, baseY + 8])} Z`;

  // Paint the lowest tier first, so each tier overlaps the one under it.
  return { tiers: tiers.reverse(), trunk, base, falls };
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
  detail?: Detail;
  light?: -1 | 1;
}

export interface Tree extends Fir {
  x: number;
  h: number;
  /** Sway period and offset in seconds (CSS, off under Reduce Motion). */
  dur: number;
  delay: number;
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
    const fir = firTree(x, spec.baseY + rand() * 6, h, w, Math.floor(rand() * 1e9), spec.detail ?? 2, spec.light ?? -1);
    const dur = 7 + rand() * 5;
    trees.push({ ...fir, x: r1(x), h: r1(h), dur: r1(dur), delay: r1(-rand() * dur) });
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
      { trees: forestLayer({ count: 7, h: [200, 300], x: [120, 640], baseY: base - 40, detail: 0 }, seed), drift: driftPath(w, base - 30, 70, seed + 1, 'right') },
      { trees: forestLayer({ count: 4, h: [320, 450], x: [250, 640], baseY: base - 10, detail: 1 }, seed + 2), drift: driftPath(w, base, 90, seed + 3, 'right') },
      { trees: forestLayer({ count: 2, h: [560, 680], x: [470, 700], baseY: base + 20, ratio: 0.5, detail: 2 }, seed + 4), drift: driftPath(w, base + 20, 110, seed + 5, 'right') },
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
      { trees: forestLayer({ count: 4, h: [140, 210], x: [0, 330], baseY: base - 30, detail: 0, light: 1 }, seed), drift: driftPath(w, base - 22, 50, seed + 1, 'left') },
      { trees: forestLayer({ count: 2, h: [230, 300], x: [0, 220], baseY: base - 8, detail: 1, light: 1 }, seed + 2), drift: driftPath(w, base, 64, seed + 3, 'left') },
      { trees: forestLayer({ count: 1, h: [360, 400], x: [-40, 60], baseY: base + 16, ratio: 0.5, detail: 2, light: 1 }, seed + 4), drift: driftPath(w, base + 16, 80, seed + 5, 'left') },
    ],
  };
}
