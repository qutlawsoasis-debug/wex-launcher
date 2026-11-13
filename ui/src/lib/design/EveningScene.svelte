<!--
  Winter evening on the open build tile: a dusk sky (deep blue to violet to a
  warm glow at the horizon) with the first stars and a moon, distant snowy hills
  with a little village (log and plank houses under thick snow with icicles,
  carved window frames lit warm behind curtains, stone chimneys with smoke, a
  fence and a few firs), and a small decorated fir standing in the snow with three
  gifts under it. On the tree: a star, glass baubles, candy canes, silver tinsel
  («дождик») and a string of lights that changes colour.
  Decorative art: Apple has no guidance for seasonal scenes. Ornament colours are
  Apple system colours, and the window light is systemYellow and systemOrange;
  the dusk walls, roofs and snow are the scene's own colours. HIG › Motion
  («Make motion optional»): the lights, the star, the tinsel, the chimney smoke
  and a rare gentle window flicker move only when motion is not reduced; under
  Reduce Motion (HIG › Accessibility) everything holds still.
-->
<script lang="ts">
  import { CX, TIERS, snowPath, swags, tierPath } from './xmasTree';
  import { firTree } from './winter/scene';

  const tiers = TIERS.map((t) => ({ body: tierPath(t), snow: snowPath(t) }));
  const garland = swags(7);
  /** Steady colours under Reduce Motion (Apple system colours). */
  const LIGHTS = ['#ff453a', '#ff9f0a', '#ffd60a', '#30d158', '#0a84ff', '#bf5af2'];
  const bulbs = garland.flatMap((s, si) => s.points.map((p, i) => ({ ...p, k: si * 7 + i })));

  /** Glass baubles (system red, yellow, blue, purple, orange, pink), placed between the swags. */
  const baubles = [
    { x: 96, y: 98, r: 5.5, c: '#ff453a' },
    { x: 126, y: 104, r: 5, c: '#0a84ff' },
    { x: 80, y: 150, r: 6, c: '#ffd60a' },
    { x: 112, y: 156, r: 6.5, c: '#bf5af2' },
    { x: 146, y: 146, r: 5.5, c: '#ff453a' },
    { x: 66, y: 200, r: 6.5, c: '#0a84ff' },
    { x: 100, y: 206, r: 7, c: '#ff9f0a' },
    { x: 136, y: 198, r: 6.5, c: '#ff375f' },
    { x: 166, y: 206, r: 6, c: '#ffd60a' },
    { x: 84, y: 234, r: 6, c: '#bf5af2' },
    { x: 150, y: 232, r: 6.5, c: '#30d158' },
  ];
  /** Candy canes hanging from branch tips: [x, y, mirrored]. */
  const canes: Array<[number, number, boolean]> = [
    [70, 118, false],
    [154, 120, true],
    [48, 172, false],
    [176, 168, true],
    [122, 220, false],
  ];
  /** Tinsel strands hanging from the swags. */
  const strands = garland.flatMap((s, si) => s.points.filter((_, i) => i % 2 === 1).map((p) => ({ ...p, len: 10 + ((si * 5 + p.x) % 9), d: si })));
  /** Stars in the dusk sky (fixed). */
  const sky = Array.from({ length: 26 }, (_, i) => ({
    x: (i * 37.3) % 100,
    y: ((i * 53.7) % 46) + 2,
    r: i % 5 === 0 ? 1.4 : 0.8,
    tw: i % 4 === 0,
  }));
  /*
   * The far village, in a 600×96 box laid along the hills (ground at y 80).
   * Fixed data, worked out once.
   */
  const G = 80;
  type Win = { x: number; y: number; w: number; h: number };
  type House = {
    x: number;
    w: number;
    wall: number;
    roof: 'gable' | 'side';
    tex: 'log' | 'plank';
    wins: Win[];
    door?: { x: number; w: number; h: number };
    chimney?: { x: number; h: number; smoke: boolean };
  };
  const HOUSES: House[] = [
    { x: 14, w: 34, wall: 19, roof: 'gable', tex: 'log', wins: [{ x: 13, y: 6, w: 8, h: 8 }], chimney: { x: 24, h: 9, smoke: false } },
    { x: 66, w: 58, wall: 23, roof: 'side', tex: 'plank', wins: [{ x: 8, y: 7, w: 8, h: 9 }, { x: 22, y: 7, w: 8, h: 9 }], door: { x: 40, w: 9, h: 15 }, chimney: { x: 44, h: 10, smoke: true } },
    { x: 152, w: 28, wall: 15, roof: 'gable', tex: 'log', wins: [{ x: 10, y: 5, w: 7, h: 7 }] },
    { x: 214, w: 46, wall: 24, roof: 'gable', tex: 'log', wins: [{ x: 7, y: 8, w: 8, h: 9 }, { x: 31, y: 8, w: 8, h: 9 }, { x: 19.5, y: -11, w: 7, h: 7 }], door: { x: 18, w: 10, h: 15 }, chimney: { x: 33, h: 11, smoke: true } },
    { x: 300, w: 44, wall: 18, roof: 'side', tex: 'plank', wins: [{ x: 8, y: 5, w: 7, h: 8 }, { x: 28, y: 5, w: 7, h: 8 }], chimney: { x: 12, h: 8, smoke: false } },
    { x: 384, w: 32, wall: 17, roof: 'gable', tex: 'log', wins: [{ x: 6, y: 5, w: 7, h: 7 }], door: { x: 19, w: 8, h: 12 } },
    { x: 520, w: 40, wall: 20, roof: 'side', tex: 'log', wins: [{ x: 8, y: 6, w: 7, h: 8 }, { x: 24, y: 6, w: 7, h: 8 }] },
  ];
  const r1 = (n: number) => Math.round(n * 10) / 10;
  /** Small fixed generator for the lumps (same every start). */
  const lcg = (seed: number) => {
    let s = seed;
    return () => (s = (s * 9301 + 49297) % 233280) / 233280;
  };
  /** Thick snow along a roof line: lumpy on top, hanging over the eaves in rounded lumps. */
  function roofSnow(pts: Array<[number, number]>, thick: number, seed: number): string {
    const rnd = lcg(seed);
    let d = `M${r1(pts[0][0] - 1.5)} ${r1(pts[0][1] + thick * 0.6)}`;
    for (const [x, y] of pts) d += ` L${r1(x)} ${r1(y - thick * 0.35 - rnd() * 0.8)}`;
    const last = pts[pts.length - 1];
    d += ` Q${r1(last[0] + 2.5)} ${r1(last[1] + thick * 0.2)} ${r1(last[0] + 1.2)} ${r1(last[1] + thick)}`;
    for (let i = pts.length - 1; i > 0; i--) {
      const [ax, ay] = pts[i];
      const [bx, by] = pts[i - 1];
      const n = Math.max(2, Math.round(Math.hypot(bx - ax, by - ay) / 6));
      for (let k = 1; k <= n; k++) {
        const t = k / n;
        const m = t - 0.5 / n;
        const x = ax + (bx - ax) * t;
        const y = ay + (by - ay) * t + thick * (0.75 + rnd() * 0.35);
        d += ` Q${r1(ax + (bx - ax) * m)} ${r1(ay + (by - ay) * m + thick * (1.05 + rnd() * 0.4))} ${r1(x)} ${r1(y)}`;
      }
    }
    return d + ' Z';
  }
  /** Icicles under an eave: thin tapering spikes of different lengths. */
  function icicles(x0: number, x1: number, y: number, seed: number): string {
    const rnd = lcg(seed);
    let d = '';
    for (let x = x0 + 1.5; x < x1 - 1; x += 1.8 + rnd() * 2.2) {
      const len = 1.4 + rnd() * 3.4;
      d += `M${r1(x - 0.6)} ${r1(y)} L${r1(x + 0.6)} ${r1(y)} L${r1(x + 0.1)} ${r1(y + len)} Z `;
    }
    return d.trim();
  }
  const houses = HOUSES.map((h, i) => {
    const top = G - h.wall;
    const o = h.w * 0.12;
    const gable = h.roof === 'gable';
    const rise = gable ? h.w * 0.46 : h.w * 0.3;
    const apex: [number, number] = [h.x + h.w / 2, top - rise];
    const roofLine: Array<[number, number]> = gable
      ? [[h.x - o, top + 2], apex, [h.x + h.w + o, top + 2]]
      : [
          [h.x - o, top + 1.5],
          [h.x + h.w * 0.14, top - rise],
          [h.x + h.w * 0.86, top - rise],
          [h.x + h.w + o, top + 1.5],
        ];
    const roof = gable
      ? `M${r1(h.x - o)} ${r1(top + 2)} L${r1(apex[0])} ${r1(apex[1])} L${r1(h.x + h.w + o)} ${r1(top + 2)} L${r1(h.x + h.w + o - 1.6)} ${r1(top + 4)} L${r1(apex[0])} ${r1(apex[1] + 3)} L${r1(h.x - o + 1.6)} ${r1(top + 4)} Z`
      : `M${r1(h.x - o)} ${r1(top + 1.5)} L${r1(h.x + h.w * 0.14)} ${r1(top - rise)} L${r1(h.x + h.w * 0.86)} ${r1(top - rise)} L${r1(h.x + h.w + o)} ${r1(top + 1.5)} L${r1(h.x + h.w + o)} ${r1(top + 3.5)} L${r1(h.x - o)} ${r1(top + 3.5)} Z`;
    const gableWall = gable ? `M${r1(h.x)} ${r1(top + 0.5)} L${r1(apex[0])} ${r1(apex[1] + 2.5)} L${r1(h.x + h.w)} ${r1(top + 0.5)} Z` : '';
    // Wall texture: logs (horizontal, with round ends at the corners) or vertical planks.
    let tex = '';
    const ends: Array<[number, number]> = [];
    if (h.tex === 'log') {
      for (let y = top + 2.6; y < G; y += 2.8) {
        tex += `M${r1(h.x)} ${r1(y)} H${r1(h.x + h.w)} `;
        ends.push([h.x - 0.6, r1(y - 1.4)], [h.x + h.w + 0.6, r1(y - 1.4)]);
      }
    } else {
      for (let x = h.x + 3.2; x < h.x + h.w; x += 3.2) tex += `M${r1(x)} ${r1(top)} V${G} `;
    }
    // Chimney: from inside the roof up past the snow.
    const roofY = (x: number) => (gable ? apex[1] + (Math.abs(x - apex[0]) / (h.w / 2 + o)) * (top + 2 - apex[1]) : top - rise);
    const chim = h.chimney
      ? { x: h.x + h.chimney.x, y: r1(roofY(h.x + h.chimney.x + 5) - h.chimney.h), h: h.chimney.h + 4, smoke: h.chimney.smoke }
      : undefined;
    const eaveY = gable ? top + 4 : top + 3.5;
    return {
      ...h,
      top,
      roof,
      gableWall,
      grain: tex.trim(),
      ends,
      // A side roof shows its whole slope, so snow covers the face down to the eave.
      snow: gable
        ? roofSnow(roofLine, 3.4, i * 31 + 7)
        : `M${r1(h.x + h.w * 0.14)} ${r1(top - rise - 1.2)} L${r1(h.x + h.w * 0.86)} ${r1(top - rise - 1.2)} L${r1(h.x + h.w + o)} ${r1(top + 1.5)} L${r1(h.x - o)} ${r1(top + 1.5)} Z ` +
          roofSnow([roofLine[0], roofLine[3]], 4, i * 31 + 7),
      ice: gable
        ? icicles(h.x - o, h.x - o + h.w * 0.22, top + 4.5, i + 2) + ' ' + icicles(h.x + h.w + o - h.w * 0.22, h.x + h.w + o, top + 4.5, i + 5)
        : icicles(h.x - o, h.x + h.w + o, eaveY + 1.4, i + 3),
      chim,
      smokeX: h.chimney ? h.x + h.chimney.x + 2.5 : 0,
      flicker: 9 + ((i * 5) % 8),
    };
  });
  /** Little firs between the houses, from the same spruce generator as the window scene. */
  const vFirs = [
    [138, 30, 12],
    [196, 22, 9],
    [278, 34, 13],
    [362, 26, 10],
    [440, 30, 12],
  ].map(([x, h, w], i) => firTree(x, G + 1, h, w, 900 + i, 0, -1));
  /** Fence runs with pointed posts and a cap of snow on each. */
  const fences = [
    [126, 150],
    [262, 296],
    [418, 470],
  ].map(([x0, x1]) => {
    let posts = '';
    let caps = '';
    for (let x = x0; x <= x1; x += 4) {
      posts += `M${x - 0.6} ${G + 1} V${G - 6.5} L${x} ${G - 7.4} L${x + 0.6} ${G - 6.5} V${G + 1} Z `;
      caps += `M${x - 1.1} ${G - 6.8} q1.1 -1.8 2.2 0 Z `;
    }
    return { x0, x1, posts: posts.trim(), caps: caps.trim() };
  });
  // Snow the village stands in, tapering into the hills at the left end.
  const groundSnow = 'M0 96 Q14 90 34 82 Q50 77 70 79.5 T130 78.5 T210 80 T300 78 T390 80.5 T470 78.5 T560 79.5 T600 78 V96 Z';
</script>

<span class="eve-sky" aria-hidden="true">
  <span class="stars">
    {#each sky as s, i (i)}<i class:tw={s.tw} style:left="{s.x}%" style:top="{s.y}%" style:--s="{s.r * 1.6}px" style:animation-delay="{(i % 7) * 0.6}s"></i>{/each}
  </span>
  <i class="moon"></i>
  <svg class="hills" viewBox="0 0 1000 120" preserveAspectRatio="none">
    <path class="far" d="M0 70 Q120 34 260 58 T520 50 T800 44 T1000 60 V120 H0Z" />
    <path class="near" d="M0 92 Q160 64 330 84 T660 78 T1000 86 V120 H0Z" />
  </svg>
  <svg class="village" viewBox="0 0 600 96" preserveAspectRatio="xMidYMax meet" aria-hidden="true" focusable="false">
    <defs>
      <radialGradient id="vil-pane" cx="0.5" cy="0.55" r="0.65">
        <stop offset="0" stop-color="#ffd60a" />
        <stop offset="1" stop-color="#ff9f0a" />
      </radialGradient>
      <radialGradient id="vil-glow" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#ffd60a" stop-opacity="0.55" />
        <stop offset="0.45" stop-color="#ff9f0a" stop-opacity="0.22" />
        <stop offset="1" stop-color="#ff9f0a" stop-opacity="0" />
      </radialGradient>
      <linearGradient id="vil-wall" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#4b3c74" />
        <stop offset="1" stop-color="#2c2556" />
      </linearGradient>
      <linearGradient id="vil-snow" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#fbf1f6" />
        <stop offset="0.6" stop-color="#d9d4f0" />
        <stop offset="1" stop-color="#a9a3d6" />
      </linearGradient>
      <linearGradient id="vil-ground" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#cfcbee" />
        <stop offset="1" stop-color="#a3a9da" stop-opacity="0.6" />
      </linearGradient>
    </defs>

    {#each houses as h, i (i)}
      <!-- Smoke behind the house, curling up and drifting away. -->
      {#if h.chim && h.chim.smoke}
        <path class="puff" d="M{h.smokeX} {h.chim.y} c-3 -5 4 -8 1 -13 s-4 -7 0 -12" style:animation-delay="{-i * 1.3}s" />
        <path class="puff" d="M{h.smokeX} {h.chim.y} c3 -4 -3 -8 1 -12 s5 -8 1 -13" style:animation-delay="{-i * 1.3 - 3.5}s" />
      {/if}
      {#if h.chim}
        {@const c = h.chim}
        <rect x={c.x} y={c.y} width="5" height={c.h} fill="#5d4f7c" />
        <path
          class="mortar"
          d="M{c.x} {c.y + 2.5} h5 M{c.x} {c.y + 5} h5 M{c.x} {c.y + 7.5} h5 M{c.x + 2} {c.y} v2.5 M{c.x + 3.5} {c.y + 2.5} v2.5 M{c.x + 1.5} {c.y + 5} v2.5 M{c.x + 3} {c.y + 7.5} v2.5"
        />
        <path d="M{c.x - 0.8} {c.y + 0.6} q0.4 -2.4 3.3 -2.4 q3 0 3.3 2.4 Z" class="snowcap" />
      {/if}
      <rect x={h.x} y={h.top} width={h.w} height={G - h.top} fill="url(#vil-wall)" />
      {#if h.gableWall}<path d={h.gableWall} fill="url(#vil-wall)" />{/if}
      <path class="grain" d={h.grain} />
      {#each h.ends as [ex, ey], k (k)}<circle cx={ex} cy={ey + 1.4} r="1.25" class="log-end" />{/each}
      <!-- Warm light from the windows on the wall around them. -->
      {#each h.wins as w, k (k)}
        <ellipse cx={h.x + w.x + w.w / 2} cy={h.top + w.y + w.h / 2} rx={w.w * 1.6} ry={w.h * 1.4} fill="url(#vil-glow)" />
      {/each}
      {#each h.wins as w, k (k)}
        {@const wx = h.x + w.x}
        {@const wy = h.top + w.y}
        <g class="win" style:animation-duration="{h.flicker + k * 3}s" style:animation-delay="{-(i * 2.7 + k * 1.9)}s">
          <rect x={wx} y={wy} width={w.w} height={w.h} fill="url(#vil-pane)" />
          <!-- Curtains drawn back to the sides. -->
          <path d="M{wx} {wy} h{w.w * 0.32} q{-w.w * 0.12} {w.h * 0.5} {-w.w * 0.05} {w.h} H{wx} Z" class="curtain" />
          <path d="M{wx + w.w} {wy} h{-w.w * 0.32} q{w.w * 0.12} {w.h * 0.5} {w.w * 0.05} {w.h} H{wx + w.w} Z" class="curtain" />
        </g>
        <rect x={wx} y={wy} width={w.w} height={w.h} class="frame" />
        <path d="M{wx + w.w / 2} {wy} V{wy + w.h} M{wx} {wy + w.h * 0.42} H{wx + w.w}" class="bar" />
        <!-- Carved frame: a little pediment above, a sill with snow below. -->
        <path d="M{wx - 1.2} {wy - 0.6} L{wx + w.w / 2} {wy - 2.8} L{wx + w.w + 1.2} {wy - 0.6} Z" class="trim" />
        <rect x={wx - 1.2} y={wy + w.h} width={w.w + 2.4} height="1.2" class="trim" />
        <path d="M{wx - 1.2} {wy + w.h} q{(w.w + 2.4) / 2} -1.6 {w.w + 2.4} 0 Z" class="snowcap" />
      {/each}
      {#if h.door}
        {@const dx = h.x + h.door.x}
        {@const dh = h.door.h}
        {@const dw = h.door.w}
        <rect x={dx} y={G - dh} width={dw} height={dh} fill="#241a3c" />
        <path d="M{dx + dw / 2} {G - dh} V{G}" class="bar dark" />
        <rect x={dx} y={G - dh} width={dw} height={dh} class="frame" />
        <circle cx={dx + dw - 1.8} cy={G - dh * 0.45} r="0.6" fill="#ffd60a" />
        <path d="M{dx - 1.5} {G - dh - 0.5} h{dw + 3} v-1.6 h{-(dw + 3)} Z" fill="#3a2f5e" />
        <path d="M{dx - 2} {G - dh - 2.1} q{(dw + 4) / 2} -2.2 {dw + 4} 0 Z" class="snowcap" />
      {/if}
      <path d={h.roof} fill="#221c48" />
      <path d={h.snow} fill="url(#vil-snow)" />
      <path d={h.ice} class="ice" />
    {/each}

    {#each vFirs as f, i (i)}
      {#each f.tiers as t, k (k)}
        <path d={t.body} fill="#1d3150" />
        <path d={t.snow} fill="#dcd8f0" />
      {/each}
      <path d={f.base} fill="#dcd8f0" />
    {/each}

    {#each fences as f, i (i)}
      <path d="M{f.x0} {G - 4.6} H{f.x1} M{f.x0} {G - 1.8} H{f.x1}" class="rail" />
      <path d={f.posts} fill="#2c2452" />
      <path d={f.caps} class="snowcap" />
      <path d="M{f.x0} {G - 5.2} H{f.x1}" class="rail-snow" />
    {/each}

    <path d={groundSnow} fill="url(#vil-ground)" />
  </svg>
</span>

<span class="eve-tree" aria-hidden="true">
  <svg viewBox="0 0 220 280">
    <defs>
      <linearGradient id="eve-fir" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#2f8f55" />
        <stop offset="0.55" stop-color="#1d6b3f" />
        <stop offset="1" stop-color="#11482a" />
      </linearGradient>
      <radialGradient id="eve-glass" cx="0.35" cy="0.32" r="0.75">
        <stop offset="0" stop-color="#fff" stop-opacity="0.9" />
        <stop offset="0.25" stop-color="#fff" stop-opacity="0" />
        <stop offset="1" stop-color="#000" stop-opacity="0.35" />
      </radialGradient>
      <radialGradient id="eve-glow" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#ffd60a" stop-opacity="0.75" />
        <stop offset="1" stop-color="#ffd60a" stop-opacity="0" />
      </radialGradient>
    </defs>

    <!-- Warm light the tree casts on the snow. -->
    <ellipse cx={CX} cy="262" rx="104" ry="12" fill="#ffd60a" opacity="0.14" />
    <rect x="101" y="236" width="18" height="30" rx="2" fill="#6b4a2e" />

    {#each tiers as t, i (i)}
      <path d={t.body} fill="url(#eve-fir)" />
      <path d={t.snow} fill="#f4f8fd" opacity="0.95" />
    {/each}

    <!-- Tinsel: silver garlands along the swags and strands hanging from them. -->
    {#each garland as s, i (i)}
      <path class="tinsel" d={s.d} transform="translate(0 6)" style:animation-delay="{i * 0.4}s" />
    {/each}
    {#each strands as s, i (i)}
      <line class="strand" x1={s.x} y1={s.y + 6} x2={s.x + 1} y2={s.y + 6 + s.len} style:animation-delay="{(i % 5) * 0.3}s" />
    {/each}

    {#each canes as [x, y, m], i (i)}
      <g transform="translate({x} {y}) scale({m ? -1 : 1} 1)">
        <path class="cane" d="M0 14 V3 A3.5 3.5 0 0 1 7 3" />
        <path class="cane-stripe" d="M0 14 V3 A3.5 3.5 0 0 1 7 3" />
      </g>
    {/each}

    {#each baubles as b, i (i)}
      <rect x={b.x - 1.6} y={b.y - b.r - 2.4} width="3.2" height="2.8" rx="0.6" fill="#c7c7cc" />
      <circle cx={b.x} cy={b.y} r={b.r} fill={b.c} />
      <circle cx={b.x} cy={b.y} r={b.r} fill="url(#eve-glass)" />
    {/each}

    <!-- The string of lights: wire and bulbs that change colour, one after another. -->
    {#each garland as s, i (i)}<path class="wire" d={s.d} />{/each}
    {#each bulbs as b (b.k)}
      <circle class="light" fill={LIGHTS[b.k % LIGHTS.length]} cx={b.x} cy={b.y + 1.5} r="2.6" style:animation-delay="{-(b.k % 6) * 0.5}s" />
    {/each}

    <!-- Star -->
    <circle class="star-glow" cx={CX} cy="26" r="20" fill="url(#eve-glow)" />
    <path
      class="star"
      d="M110 12 L113.6 21.4 L123.6 21.8 L115.8 28 L118.6 37.6 L110 32 L101.4 37.6 L104.2 28 L96.4 21.8 L106.4 21.4 Z"
    />

    <!-- Three gifts under the tree. -->
    <g class="gift">
      <rect x="40" y="226" width="50" height="40" rx="3" fill="#ff453a" />
      <rect x="40" y="226" width="50" height="40" rx="3" fill="url(#eve-glass)" opacity="0.5" />
      <rect x="61" y="226" width="8" height="40" fill="#ffd60a" />
      <rect x="40" y="241" width="50" height="7" fill="#ffd60a" />
      <path d="M65 226 C54 214 48 222 60 227 M65 226 C76 214 82 222 70 227" stroke="#ffd60a" stroke-width="4" fill="none" stroke-linecap="round" />
    </g>
    <g class="gift">
      <rect x="132" y="236" width="42" height="30" rx="3" fill="#0a84ff" />
      <rect x="132" y="236" width="42" height="30" rx="3" fill="url(#eve-glass)" opacity="0.5" />
      <rect x="150" y="236" width="6" height="30" fill="#fff" />
      <rect x="132" y="248" width="42" height="6" fill="#fff" />
      <path d="M153 236 C144 226 139 233 149 237 M153 236 C162 226 167 233 157 237" stroke="#fff" stroke-width="3.5" fill="none" stroke-linecap="round" />
    </g>
    <g class="gift">
      <rect x="86" y="246" width="40" height="26" rx="3" fill="#30d158" />
      <rect x="86" y="246" width="40" height="26" rx="3" fill="url(#eve-glass)" opacity="0.5" />
      <rect x="102" y="246" width="7" height="26" fill="#ff375f" />
      <path d="M105.5 246 C96 236 91 243 101 247 M105.5 246 C115 236 120 243 110 247" stroke="#ff375f" stroke-width="3.5" fill="none" stroke-linecap="round" />
    </g>
  </svg>
</span>

<style>
  /* The dusk sky replaces the build colour on the open tile. */
  .eve-sky {
    position: absolute;
    inset: 0;
    z-index: -3;
    pointer-events: none;
    background:
      radial-gradient(120% 70% at 70% 105%, rgb(255 159 10 / 0.55), transparent 60%),
      linear-gradient(180deg, #141a4a 0%, #2b2a78 38%, #5b3a8e 66%, #b25a86 86%, #f08a5d 100%);
  }
  .stars {
    position: absolute;
    inset: 0;
  }
  .stars i {
    position: absolute;
    width: var(--s);
    height: var(--s);
    border-radius: 50%;
    background: #fff;
    opacity: 0.8;
    box-shadow: 0 0 3px rgb(255 255 255 / 0.6);
  }
  .stars .tw {
    animation: tw 4s ease-in-out infinite;
  }
  @keyframes tw {
    50% {
      opacity: 0.2;
    }
  }
  .moon {
    position: absolute;
    right: 34%;
    top: 16%;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    box-shadow:
      inset -8px -2px 0 0 #fff5d6,
      0 0 24px 4px rgb(255 245 214 / 0.25);
    transform: rotate(-20deg);
  }
  .hills {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 26px;
    width: 100%;
    height: 90px;
  }
  .hills .far {
    fill: #6b6fb3;
    opacity: 0.55;
  }
  .hills .near {
    fill: #9aa6d6;
    opacity: 0.75;
  }
  /* The village sits on the far hills over the right part of the tile, behind the tree. */
  .village {
    position: absolute;
    right: 0;
    bottom: 34px;
    width: 62%;
    height: auto;
    aspect-ratio: 600 / 96;
    display: block;
    pointer-events: none;
    overflow: visible;
  }
  .grain {
    fill: none;
    stroke: #1f1940;
    stroke-width: 0.5;
    opacity: 0.7;
  }
  .log-end {
    fill: #6a5788;
    stroke: #2a2350;
    stroke-width: 0.35;
  }
  .mortar {
    fill: none;
    stroke: #3a2f5a;
    stroke-width: 0.45;
  }
  .frame {
    fill: none;
    stroke: #efe6f2;
    stroke-width: 0.9;
  }
  .bar {
    fill: none;
    stroke: #efe6f2;
    stroke-width: 0.6;
  }
  .bar.dark {
    stroke: #120c24;
  }
  .trim {
    fill: #efe6f2;
  }
  .snowcap {
    fill: #f6eff7;
  }
  .curtain {
    fill: #ff9f0a;
    opacity: 0.75;
  }
  .ice {
    fill: #eef3ff;
    opacity: 0.85;
  }
  .rail {
    fill: none;
    stroke: #2c2452;
    stroke-width: 0.9;
  }
  .rail-snow {
    fill: none;
    stroke: #f2ecf7;
    stroke-width: 0.9;
    stroke-linecap: round;
  }
  /* Chimney smoke curls up, drifts with the air and fades. */
  .puff {
    fill: none;
    stroke: #d8d2ee;
    stroke-width: 4;
    stroke-linecap: round;
    opacity: 0.26;
    filter: blur(1.4px);
    transform-box: fill-box;
    transform-origin: 50% 100%;
    animation: smoke 7s ease-out infinite;
  }
  @keyframes smoke {
    0% {
      opacity: 0;
      transform: translate(0, 2px) scale(0.6);
    }
    25% {
      opacity: 0.32;
    }
    100% {
      opacity: 0;
      transform: translate(5px, -10px) scale(1.35);
    }
  }
  /* Now and then a window dims a little, as if someone walked past the lamp. */
  .win {
    animation: win 11s ease-in-out infinite;
  }
  @keyframes win {
    0%,
    84%,
    100% {
      opacity: 1;
    }
    88% {
      opacity: 0.7;
    }
    91% {
      opacity: 0.95;
    }
    94% {
      opacity: 0.78;
    }
  }

  /* The tree stands in the snow on the right, behind the snowballs. */
  .eve-tree {
    position: absolute;
    right: 6%;
    bottom: 12px;
    z-index: 0;
    width: clamp(150px, 16cqw, 220px);
    aspect-ratio: 220 / 280;
    max-height: 78%;
    pointer-events: none;
  }
  .eve-tree svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
    filter: drop-shadow(0 4px 6px rgb(0 0 0 / 0.25));
  }
  .tinsel {
    fill: none;
    stroke: #e5e5ea;
    stroke-width: 1.6;
    stroke-dasharray: 1.5 2;
    opacity: 0.9;
    animation: shimmer 1.8s linear infinite;
  }
  .strand {
    stroke: #d1d1d6;
    stroke-width: 0.9;
    stroke-dasharray: 1.2 1.4;
    opacity: 0.85;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    to {
      stroke-dashoffset: -7;
    }
  }
  .cane {
    fill: none;
    stroke: #fff;
    stroke-width: 2.6;
    stroke-linecap: round;
  }
  .cane-stripe {
    fill: none;
    stroke: #ff3b30;
    stroke-width: 2.6;
    stroke-dasharray: 2 2;
  }
  .wire {
    fill: none;
    stroke: #1d3b2a;
    stroke-width: 0.9;
  }
  .light {
    animation: rainbow 3s linear infinite;
  }
  /* Through Apple system colours: red, orange, yellow, green, blue, purple. */
  @keyframes rainbow {
    0%,
    100% {
      fill: #ff453a;
      filter: drop-shadow(0 0 3px #ff453a);
    }
    17% {
      fill: #ff9f0a;
      filter: drop-shadow(0 0 3px #ff9f0a);
    }
    33% {
      fill: #ffd60a;
      filter: drop-shadow(0 0 3px #ffd60a);
    }
    50% {
      fill: #30d158;
      filter: drop-shadow(0 0 3px #30d158);
    }
    67% {
      fill: #0a84ff;
      filter: drop-shadow(0 0 3px #0a84ff);
    }
    83% {
      fill: #bf5af2;
      filter: drop-shadow(0 0 3px #bf5af2);
    }
  }
  .star {
    fill: #ffd60a;
    stroke: #ff9f0a;
    stroke-width: 0.8;
    transform-origin: 110px 25px;
    animation: star 3.2s ease-in-out infinite;
  }
  .star-glow {
    animation: glow 3.2s ease-in-out infinite;
  }
  @keyframes star {
    50% {
      transform: scale(1.08);
    }
  }
  @keyframes glow {
    50% {
      opacity: 0.55;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .stars .tw,
    .win,
    .puff,
    .tinsel,
    .strand,
    .light,
    .star,
    .star-glow {
      animation: none;
    }
  }
  :global(html[data-reduce-motion='true']) .stars .tw,
  :global(html[data-reduce-motion='true']) .win,
  :global(html[data-reduce-motion='true']) .puff,
  :global(html[data-reduce-motion='true']) .tinsel,
  :global(html[data-reduce-motion='true']) .strand,
  :global(html[data-reduce-motion='true']) .light,
  :global(html[data-reduce-motion='true']) .star,
  :global(html[data-reduce-motion='true']) .star-glow {
    animation: none;
  }
</style>
