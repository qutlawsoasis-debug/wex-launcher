<!--
  Snow lying along the bottom of a tile (winter look, season.svelte.ts): two soft
  drifts, the back one a cool shade of white and the front one lit from above,
  with an inner shadow where the snow meets the tile and a few glints. The hills
  come from a fixed seed, so a tile keeps its own shape. Apple has no guidance on
  seasonal decoration; HIG › Motion: the glints twinkle only when motion is not
  reduced, the snow itself never moves.
-->
<script lang="ts">
  import { driftPath, glints } from './drift';

  interface Props {
    /** Seed for the hill shape (the build id). */
    seed: string;
    /** Height of the drift, px. */
    height?: number;
    /** How much of a full-width tile this one is (fewer, wider hills when narrow). */
    span?: number;
  }

  let { seed, height = 34, span = 1 }: Props = $props();

  const W = 1000;
  const H = 60;
  const back = $derived(driftPath(seed + ':back', W, H, 0.3, 1, 48, span));
  const front = $derived(driftPath(seed, W, H, 0.22, 0.9, 48, span));
  const sparks = $derived(glints(seed, W, H, Math.max(2, Math.round(9 * span)), span));
  const uid = $derived('d' + Math.abs([...seed].reduce((h, ch) => (h * 31 + ch.charCodeAt(0)) | 0, 7)).toString(36));
</script>

<span class="drift" style:height="{height}px" aria-hidden="true">
<svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
  <defs>
    <linearGradient id="{uid}-back" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#eef4fb" />
      <stop offset="1" stop-color="#c9d7e8" />
    </linearGradient>
    <linearGradient id="{uid}-front" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="0.55" stop-color="#f3f7fc" />
      <stop offset="1" stop-color="#dbe6f3" />
    </linearGradient>
  </defs>
  <path d={back} fill="url(#{uid}-back)" opacity="0.9" />
  <path d={front} fill="url(#{uid}-front)" />
</svg>
{#each sparks as g, i (i)}
  <i class="glint" style:left="{(g.x / W) * 100}%" style:top="{(g.y / H) * 100}%" style:animation-delay="{g.delay}s"></i>
{/each}
</span>

<style>
  .drift {
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    pointer-events: none;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    filter: drop-shadow(0 -1px 2px rgb(0 0 0 / 0.16));
  }
  .glint {
    position: absolute;
    width: 3px;
    height: 3px;
    margin: -1.5px 0 0 -1.5px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 5px 1px rgb(150 200 255 / 0.95);
    animation: glint 3.6s ease-in-out infinite;
    opacity: 0.2;
  }
  @keyframes glint {
    0%,
    80%,
    100% {
      opacity: 0.2;
    }
    88% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .glint {
      animation: none;
      opacity: 0.6;
    }
  }
  :global(html[data-reduce-motion='true']) .glint {
    animation: none;
    opacity: 0.6;
  }
</style>
