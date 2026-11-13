<!--
  Winter evening on the open build tile: a dusk sky (deep blue to violet to a
  warm glow at the horizon) with the first stars and a moon, distant snowy hills
  with lit windows, and a small decorated fir standing in the snow with three
  gifts under it. On the tree: a star, glass baubles, candy canes, silver tinsel
  («дождик») and a string of lights that changes colour.
  Decorative art: Apple has no guidance for seasonal scenes. Ornament colours are
  Apple system colours. HIG › Motion: the lights, the star and the tinsel shimmer
  only when motion is not reduced; under Reduce Motion everything holds still.
-->
<script lang="ts">
  import { CX, TIERS, snowPath, swags, tierPath } from './xmasTree';

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
  /** Lit windows in the far village: [x %, y px from the hills' top]. */
  const windows = [44, 46.5, 58, 64, 66.5, 73, 79, 81.5];
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
  <span class="village">
    {#each windows as x, i (i)}<i style:left="{x}%" style:animation-delay="{i * 0.9}s"></i>{/each}
  </span>
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
  .village {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 58px;
    height: 10px;
  }
  /* A small house: dark wall, snowy roof, one lit window that flickers now and then. */
  .village i {
    position: absolute;
    bottom: 0;
    width: 14px;
    height: 9px;
    margin-left: -7px;
    background: #3a3c78;
  }
  .village i::before {
    content: '';
    position: absolute;
    left: -2px;
    right: -2px;
    bottom: 100%;
    height: 7px;
    background: linear-gradient(#f4f8fd 0 45%, #2e3068 45%);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }
  .village i::after {
    content: '';
    position: absolute;
    left: 5px;
    top: 3px;
    width: 4px;
    height: 4px;
    border-radius: 1px;
    background: #ffd60a;
    box-shadow: 0 0 6px 2px rgb(255 214 10 / 0.55);
    animation: win 9s ease-in-out infinite;
    animation-delay: inherit;
  }
  @keyframes win {
    0%,
    86%,
    100% {
      opacity: 1;
    }
    92% {
      opacity: 0.35;
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
    .village i::after,
    .tinsel,
    .strand,
    .light,
    .star,
    .star-glow {
      animation: none;
    }
  }
  :global(html[data-reduce-motion='true']) .stars .tw,
  :global(html[data-reduce-motion='true']) .village i::after,
  :global(html[data-reduce-motion='true']) .tinsel,
  :global(html[data-reduce-motion='true']) .strand,
  :global(html[data-reduce-motion='true']) .light,
  :global(html[data-reduce-motion='true']) .star,
  :global(html[data-reduce-motion='true']) .star-glow {
    animation: none;
  }
</style>
