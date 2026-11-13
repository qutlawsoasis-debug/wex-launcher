<!--
  Winter night scene behind the whole window (winter look, html.winter). Layers,
  back to front: the sky, stars, snow-laden spruces at the bottom-left and
  bottom-right in three depths, and banks of snowy ground along the bottom.
  Each spruce has 8 to 14 drooping tiers with a ragged needle edge, moonlight on
  the side facing the open middle of the window and a dark far side (gradients),
  a soft shadow under every tier, heavy uneven snow clumps with a moonlit rim,
  short needle strokes and snow at the foot. The far depth is hazy, bluer and
  slightly blurred; the near one is crisp, with a bit of trunk. Everything is
  static inline SVG computed once from fixed seeds (scene.ts); motion is CSS
  animations only, never per-frame script.

  Apple has no guidance on seasonal or decorative background art, so the colours
  here (navy night, pale dawn, snow, spruce green) are the scene's own and are
  not Apple system colours; nothing interactive uses them. Rules applied:
  - HIG › Motion, «Make motion optional»: all motion is slow, peripheral and
    carries no information: a few stars twinkle, the branch tiers rock a fraction
    of a degree in the wind, now and then a little snow slips off a branch tip,
    and the far trees shimmer faintly in the haze (an opacity change).
  - HIG › Accessibility › Reduce Motion: «reducing automatic and repetitive
    animations»: none of it runs under html[data-reduce-motion='true'] or the
    system's prefers-reduced-motion; the falling snow is then not drawn at all.
  - Reduce Transparency (html[data-reduce-transparency='true']): the scene is a
    plain solid sky with no stars, trees or glow.
  - Theme: html[data-theme='dark'] is a deep navy night, and the ground snow is a
    blue-grey night snow so white sidebar text over the glass stays readable;
    light is a pale blue winter dawn, so the light theme still reads as light.

  Stacking: inside .app (isolation: isolate) at z-index -1, so it paints over
  .app's own background and under every surface; the Snowfall canvas (z 40) stays
  above. Never takes pointer input.
-->
<script lang="ts">
  import { hillPath, leftCorner, makeStars, rightCorner, type Corner } from './scene';

  const stars = makeStars(120);
  const right = rightCorner();
  const left = leftCorner();
  const depth = ['far', 'mid', 'near'] as const;
  // Ground banks in a 1000×100 box stretched across the window; they rise into the corners.
  const hillBack = hillPath(1000, 36, 100, 14, 11, { left: 16, right: 24 });
  const hillFront = hillPath(1000, 64, 100, 12, 23, { left: 20, right: 26 });
</script>

{#snippet forest(c: Corner, side: 'left' | 'right')}
  <div class="corner {side}">
    {#each c.layers as layer, i (i)}
      {@const d = depth[i]}
      {@const id = `ws-${side}-${d}`}
      <svg class="layer {d}" viewBox="0 0 {c.w} {c.h}" preserveAspectRatio={side === 'right' ? 'xMaxYMax meet' : 'xMinYMax meet'} aria-hidden="true" focusable="false">
        <defs>
          <!-- Moonlight from the open middle of the window: lit side, dark side. -->
          <linearGradient id="{id}-body" x1={side === 'right' ? 0 : 1} y1="0" x2={side === 'right' ? 1 : 0} y2="0.35">
            <stop offset="0" class="s-lit" />
            <stop offset="0.5" class="s-mid" />
            <stop offset="1" class="s-dark" />
          </linearGradient>
          <linearGradient id="{id}-snow" x1={side === 'right' ? 0.1 : 0.9} y1="0" x2={side === 'right' ? 0.7 : 0.3} y2="1">
            <stop offset="0" class="s-snow" />
            <stop offset="0.55" class="s-snow-mid" />
            <stop offset="1" class="s-snow-shade" />
          </linearGradient>
          <linearGradient id="{id}-shade" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" class="s-shadow" stop-opacity="0.85" />
            <stop offset="1" class="s-shadow" stop-opacity="0" />
          </linearGradient>
          <linearGradient id="{id}-needle" x1={side === 'right' ? 0 : 1} y1="0" x2={side === 'right' ? 1 : 0} y2="0">
            <stop offset="0" class="s-needle" stop-opacity="0.9" />
            <stop offset="0.6" class="s-needle" stop-opacity="0.35" />
            <stop offset="1" class="s-needle" stop-opacity="0.12" />
          </linearGradient>
        </defs>
        {#each layer.trees as tree, j (j)}
          <g class="tree" style="--dur:{tree.dur}s;--delay:{tree.delay}s">
            {#if tree.trunk}<path class="trunk" d={tree.trunk} />{/if}
            {#each tree.tiers as t, k (k)}
              <g class="tier" style="transform-origin:{t.ox}px {t.oy}px;--sw:{(0.25 + (k / tree.tiers.length) * 0.15 * (i + 1)).toFixed(2)}deg">
                {#if t.shade}<path class="shade" d={t.shade} fill="url(#{id}-shade)" />{/if}
                <path class="body" d={t.body} fill="url(#{id}-body)" />
                {#if t.needles}<path class="needles" d={t.needles} stroke="url(#{id}-needle)" />{/if}
                <path class="snow" d={t.snow} fill="url(#{id}-snow)" />
                {#if t.rim}<path class="rim" d={t.rim} />{/if}
              </g>
            {/each}
            <path class="base" d={tree.base} fill="url(#{id}-snow)" />
            {#each tree.falls as f, k (k)}
              <g class="fall" style="--fx:{side === 'right' ? -1 : 1};animation-delay:{-((j * 7 + k * 11 + i * 5) % 23)}s;animation-duration:{19 + ((j * 5 + k * 3) % 9)}s">
                <circle cx={f.x} cy={f.y} r="2.6" />
                <circle cx={f.x + 3} cy={f.y + 2} r="1.8" />
                <circle cx={f.x - 2} cy={f.y + 3} r="1.4" />
              </g>
            {/each}
          </g>
        {/each}
        <path class="drift" d={layer.drift} />
      </svg>
    {/each}
  </div>
{/snippet}

<div class="winter-scene" aria-hidden="true">
  <div class="glow"></div>
  <div class="stars">
    {#each stars as s, i (i)}
      <i
        class:tw={s.twinkle}
        class:big={s.size >= 2}
        style="left:{s.x}%;top:{s.y}%;width:{s.size}px;height:{s.size}px;--a:{s.alpha};--dur:{s.dur}s;--delay:{s.delay}s"
      ></i>
    {/each}
  </div>
  {@render forest(left, 'left')}
  {@render forest(right, 'right')}
  <svg class="ground" viewBox="0 0 1000 100" preserveAspectRatio="none" aria-hidden="true" focusable="false">
    <defs>
      <linearGradient id="ws-ground-back" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="s-back-0" />
        <stop offset="1" class="s-back-1" />
      </linearGradient>
      <linearGradient id="ws-ground-front" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="s-front-0" />
        <stop offset="1" class="s-front-1" />
      </linearGradient>
    </defs>
    <path class="hill back" d={hillBack} />
    <path class="hill front" d={hillFront} />
  </svg>
</div>

<style>
  .winter-scene {
    /* Night (dark theme). */
    --ws-sky: linear-gradient(180deg, #040918 0%, #08132e 38%, #0d1d45 72%, #162a5e 100%);
    --ws-sky-solid: #08132e;
    --ws-glow: radial-gradient(ellipse 60% 55% at 88% 70%, rgba(92, 132, 222, 0.22), transparent 70%),
      radial-gradient(ellipse 45% 40% at 8% 85%, rgba(92, 132, 222, 0.14), transparent 70%);
    --ws-star: #ffffff;
    --ws-star-glow: rgba(190, 215, 255, 0.85);
    --ws-stars-op: 1;
    /* Spruces: moonlit side, middle, dark side; snow lit, mid, in shadow. */
    --ws-far-lit: #2c4170;
    --ws-far-mid: #22355f;
    --ws-far-dark: #1a2a50;
    --ws-far-snow: #7489bb;
    --ws-far-snow-mid: #5d72a5;
    --ws-far-snow-shade: #4a5e90;
    --ws-far-shadow: #13204a;
    --ws-far-needle: #5d79a8;
    --ws-mid-lit: #1f4258;
    --ws-mid-mid: #142c45;
    --ws-mid-dark: #0b1a33;
    --ws-mid-snow: #d2def5;
    --ws-mid-snow-mid: #9fb2dc;
    --ws-mid-snow-shade: #6a80b6;
    --ws-mid-shadow: #050d24;
    --ws-mid-needle: #5f8fa5;
    --ws-near-lit: #1f4e57;
    --ws-near-mid: #0f2d3d;
    --ws-near-dark: #061423;
    --ws-near-snow: #f4f8ff;
    --ws-near-snow-mid: #c4d2ee;
    --ws-near-snow-shade: #7e93c6;
    --ws-near-shadow: #020a18;
    --ws-near-needle: #6aa3ad;
    --ws-rim: rgba(235, 244, 255, 0.9);
    --ws-trunk: #20182a;
    --ws-fall: #eef4ff;
    --ws-drift-far: #3d5186;
    --ws-drift-mid: #7186b9;
    --ws-drift-near: #a9b8dc;
    --ws-snow-glow: rgba(150, 180, 240, 0.3);
    /* Night snow on the ground is blue-grey, so white sidebar text stays readable over it. */
    --ws-back-0: #4f6396;
    --ws-back-1: #2c3a66;
    --ws-front-0: #6e81b0;
    --ws-front-1: #45558a;

    position: fixed;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    overflow: hidden;
    background: var(--ws-sky);
    contain: strict;
  }
  :global(html[data-theme='light']) .winter-scene {
    /* Pale winter dawn (light theme). */
    --ws-sky: linear-gradient(180deg, #c9d8ec 0%, #dbe6f4 40%, #ebf1f9 75%, #f6f3f1 100%);
    --ws-sky-solid: #e3ebf6;
    --ws-glow: radial-gradient(ellipse 60% 50% at 85% 78%, rgba(255, 244, 232, 0.7), transparent 70%),
      radial-gradient(ellipse 40% 40% at 10% 88%, rgba(255, 255, 255, 0.6), transparent 70%);
    --ws-stars-op: 0;
    --ws-far-lit: #b3c3da;
    --ws-far-mid: #a3b5cf;
    --ws-far-dark: #94a8c5;
    --ws-far-snow: #f3f7fc;
    --ws-far-snow-mid: #e3eaf4;
    --ws-far-snow-shade: #d0dbea;
    --ws-far-shadow: #8297b8;
    --ws-far-needle: #c7d4e6;
    --ws-mid-lit: #86a3b4;
    --ws-mid-mid: #6f8ca3;
    --ws-mid-dark: #5a7592;
    --ws-mid-snow: #ffffff;
    --ws-mid-snow-mid: #eef3fa;
    --ws-mid-snow-shade: #cbd7ea;
    --ws-mid-shadow: #3f5875;
    --ws-mid-needle: #a7c2cc;
    --ws-near-lit: #5f8a96;
    --ws-near-mid: #476c80;
    --ws-near-dark: #34506a;
    --ws-near-snow: #ffffff;
    --ws-near-snow-mid: #eef3fa;
    --ws-near-snow-shade: #c2cfe4;
    --ws-near-shadow: #233a52;
    --ws-near-needle: #9cc3c9;
    --ws-rim: rgba(255, 255, 255, 0.95);
    --ws-trunk: #4a4350;
    --ws-fall: #ffffff;
    --ws-drift-far: #dfe7f2;
    --ws-drift-mid: #edf2f9;
    --ws-drift-near: #f8fafd;
    --ws-snow-glow: rgba(255, 255, 255, 0.7);
    --ws-back-0: #e9eff8;
    --ws-back-1: #d3deec;
    --ws-front-0: #ffffff;
    --ws-front-1: #e6edf6;
  }

  .glow {
    position: absolute;
    inset: 0;
    background: var(--ws-glow);
  }

  .stars {
    position: absolute;
    inset: 0;
    opacity: var(--ws-stars-op);
  }
  .stars i {
    position: absolute;
    border-radius: 50%;
    background: var(--ws-star);
    opacity: var(--a);
  }
  .stars i.big {
    /* Brighter stars get a faint halo. */
    box-shadow: 0 0 4px 1px var(--ws-star-glow);
  }
  .stars i.tw {
    animation: ws-twinkle var(--dur) ease-in-out var(--delay) infinite alternate;
  }
  @keyframes ws-twinkle {
    from {
      opacity: var(--a);
    }
    to {
      opacity: calc(var(--a) * 0.25);
    }
  }

  .corner {
    position: absolute;
    bottom: 0;
  }
  .corner.right {
    right: 0;
    height: clamp(260px, 56vh, 620px);
    aspect-ratio: 640 / 720;
    max-width: 62vw;
  }
  .corner.left {
    left: 0;
    height: clamp(190px, 44vh, 470px);
    aspect-ratio: 420 / 480;
    max-width: 40vw;
  }
  /* The three depths are separate SVGs on top of each other: the far one is a
     static blurred layer whose haze shimmer is a cheap opacity change. */
  .layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    overflow: visible;
  }
  .far {
    --lit: var(--ws-far-lit);
    --mid: var(--ws-far-mid);
    --dark: var(--ws-far-dark);
    --snow: var(--ws-far-snow);
    --snow-mid: var(--ws-far-snow-mid);
    --snow-shade: var(--ws-far-snow-shade);
    --shadow: var(--ws-far-shadow);
    --needle: var(--ws-far-needle);
    --drift: var(--ws-drift-far);
    opacity: 0.88;
    filter: blur(0.7px);
    animation: ws-haze 11s ease-in-out infinite alternate;
  }
  .mid {
    --lit: var(--ws-mid-lit);
    --mid: var(--ws-mid-mid);
    --dark: var(--ws-mid-dark);
    --snow: var(--ws-mid-snow);
    --snow-mid: var(--ws-mid-snow-mid);
    --snow-shade: var(--ws-mid-snow-shade);
    --shadow: var(--ws-mid-shadow);
    --needle: var(--ws-mid-needle);
    --drift: var(--ws-drift-mid);
  }
  .near {
    --lit: var(--ws-near-lit);
    --mid: var(--ws-near-mid);
    --dark: var(--ws-near-dark);
    --snow: var(--ws-near-snow);
    --snow-mid: var(--ws-near-snow-mid);
    --snow-shade: var(--ws-near-snow-shade);
    --shadow: var(--ws-near-shadow);
    --needle: var(--ws-near-needle);
    --drift: var(--ws-drift-near);
  }
  .s-lit {
    stop-color: var(--lit);
  }
  .s-mid {
    stop-color: var(--mid);
  }
  .s-dark {
    stop-color: var(--dark);
  }
  .s-snow {
    stop-color: var(--snow);
  }
  .s-snow-mid {
    stop-color: var(--snow-mid);
  }
  .s-snow-shade {
    stop-color: var(--snow-shade);
  }
  .s-shadow {
    stop-color: var(--shadow);
  }
  .s-needle {
    stop-color: var(--needle);
  }
  .needles {
    fill: none;
    stroke-width: 0.9;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .rim {
    fill: none;
    stroke: var(--ws-rim);
    stroke-width: 1.3;
    stroke-linecap: round;
    opacity: 0.8;
  }
  .trunk {
    fill: var(--ws-trunk);
  }
  .drift {
    fill: var(--drift);
  }
  .fall {
    fill: var(--ws-fall);
    opacity: 0;
    animation: ws-fall 23s ease-in infinite;
  }

  /* Wind: every tier rocks a fraction of a degree about the trunk, the lower
     (longer) ones a little more; each tree on its own slow period. */
  .mid .tier,
  .near .tier {
    animation: ws-sway var(--dur) ease-in-out var(--delay) infinite alternate;
  }
  @keyframes ws-sway {
    from {
      transform: rotate(calc(var(--sw) * -1));
    }
    to {
      transform: rotate(var(--sw));
    }
  }
  /* Now and then a little snow slips off a branch tip and falls away. */
  @keyframes ws-fall {
    0%,
    90% {
      opacity: 0;
      transform: translate(0, 0);
    }
    91% {
      opacity: 0.95;
    }
    100% {
      opacity: 0;
      transform: translate(calc(var(--fx) * 10px), 70px);
    }
  }
  @keyframes ws-haze {
    from {
      opacity: 0.8;
    }
    to {
      opacity: 0.94;
    }
  }

  .ground {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    width: 100%;
    height: clamp(56px, 11vh, 120px);
    display: block;
    filter: drop-shadow(0 -2px 8px var(--ws-snow-glow));
  }
  .s-back-0 {
    stop-color: var(--ws-back-0);
  }
  .s-back-1 {
    stop-color: var(--ws-back-1);
  }
  .s-front-0 {
    stop-color: var(--ws-front-0);
  }
  .s-front-1 {
    stop-color: var(--ws-front-1);
  }
  .hill.back {
    fill: url(#ws-ground-back);
  }
  .hill.front {
    fill: url(#ws-ground-front);
  }

  /* Reduce Motion (the app's switch or the system's): the stars hold still. */
  :global(html[data-reduce-motion='true']) .stars i.tw,
  :global(html[data-reduce-motion='true']) .tier,
  :global(html[data-reduce-motion='true']) .fall,
  :global(html[data-reduce-motion='true']) .far {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .stars i.tw,
    .mid .tier,
    .near .tier,
    .fall,
    .far {
      animation: none;
    }
  }

  /* Reduce Transparency: a plain solid sky, no art behind the solid materials. */
  :global(html[data-reduce-transparency='true']) .winter-scene {
    background: var(--ws-sky-solid);
  }
  :global(html[data-reduce-transparency='true']) .winter-scene > * {
    display: none;
  }
</style>
