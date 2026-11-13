<!--
  Winter night scene behind the whole window (winter look, html.winter). Layers,
  back to front: the sky, stars, firs at the bottom-left and bottom-right in three
  depths (far ones bluer and dimmer, snow on every tier, a soft glow on the near
  snow), and banks of snowy ground along the bottom. Everything is static inline
  SVG and CSS computed once from fixed seeds (scene.ts); only a few stars twinkle,
  with a CSS opacity animation.

  Apple has no guidance on seasonal or decorative background art, so the colours
  here (navy night, pale dawn, snow) are the scene's own and are not Apple system
  colours; nothing interactive uses them. Rules applied:
  - HIG › Motion, «Make motion optional»: the twinkle is slow, peripheral and
    carries no information.
  - HIG › Accessibility › Reduce Motion: «reducing automatic and repetitive
    animations»: no twinkle under html[data-reduce-motion='true'] or the system's
    prefers-reduced-motion.
  - Reduce Transparency (html[data-reduce-transparency='true']): the scene is a
    plain solid sky with no stars, trees or glow.
  - Theme: html[data-theme='dark'] is a deep navy night; light is a pale blue
    winter dawn, so the light theme still reads as light.

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
  <svg class="corner {side}" viewBox="0 0 {c.w} {c.h}" preserveAspectRatio={side === 'right' ? 'xMaxYMax meet' : 'xMinYMax meet'} aria-hidden="true" focusable="false">
    {#each c.layers as layer, i (i)}
      <g class="layer {depth[i]}">
        {#each layer.trees as tree, j (j)}
          <path class="body" d={tree.body} />
          <path class="snow" d={tree.snow} />
        {/each}
        <path class="drift" d={layer.drift} />
      </g>
    {/each}
  </svg>
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
    --ws-far: #1a2a55;
    --ws-far-snow: #5a71a6;
    --ws-mid: #0f1c40;
    --ws-mid-snow: #9bb0dc;
    --ws-near: #071029;
    --ws-near-snow: #e4ecff;
    --ws-snow-glow: rgba(170, 200, 255, 0.55);
    --ws-drift-far: #3d5186;
    --ws-drift-mid: #8195c4;
    --ws-drift-near: #cfdaf3;
    --ws-back-0: #8aa0d0;
    --ws-back-1: #4a5f96;
    --ws-front-0: #e8efff;
    --ws-front-1: #a7b8df;

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
    --ws-far: #9fb2cf;
    --ws-far-snow: #e8eef7;
    --ws-mid: #7288ab;
    --ws-mid-snow: #f6f9fd;
    --ws-near: #5a7093;
    --ws-near-snow: #ffffff;
    --ws-snow-glow: rgba(255, 255, 255, 0.7);
    --ws-drift-far: #dfe7f2;
    --ws-drift-mid: #edf2f9;
    --ws-drift-near: #f8fafd;
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
    display: block;
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
  .far .body {
    fill: var(--ws-far);
  }
  .far .snow {
    fill: var(--ws-far-snow);
  }
  .far .drift {
    fill: var(--ws-drift-far);
  }
  .mid .body {
    fill: var(--ws-mid);
  }
  .mid .snow {
    fill: var(--ws-mid-snow);
  }
  .mid .drift {
    fill: var(--ws-drift-mid);
  }
  .near .body {
    fill: var(--ws-near);
  }
  .near .snow {
    fill: var(--ws-near-snow);
  }
  .near .drift {
    fill: var(--ws-drift-near);
  }
  .far {
    opacity: 0.9;
  }
  /* A soft moonlit glow on the nearer snow (drawn once, the scene is static). */
  .mid,
  .near {
    filter: drop-shadow(0 0 6px var(--ws-snow-glow));
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
  :global(html[data-reduce-motion='true']) .stars i.tw {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .stars i.tw {
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
