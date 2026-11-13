<!--
  A string of lights along the top edge of a tile (winter look). The wire hangs
  from pins and sags between them; glass bulbs in Apple system colours hang from
  it and glow. Lights chase slowly along the string; under Reduce Motion they
  glow steadily (HIG › Motion: decorative motion is optional and stays off when
  motion is reduced). Apple has no guidance on seasonal decoration.
-->
<script lang="ts">
  import { garlandLayout } from './garland';

  interface Props {
    /** Wider spacing on a narrow tile. */
    step?: number;
  }

  let { step = 46 }: Props = $props();
  let width = $state(0);
  const g = $derived(garlandLayout(width, { step, perSpan: step > 46 ? 2 : 4 }));
</script>

<span class="garland" bind:clientWidth={width} aria-hidden="true">
  {#if width > 0}
    <svg width={width} height="34" viewBox="0 0 {width} 34">
      <path class="wire" d={g.wire} />
      {#each g.pins as x (x)}<circle class="pin" cx={x} cy="2" r="2.2" />{/each}
    </svg>
    {#each g.bulbs as b (b.i)}
      <i
        class="bulb"
        style:left="{b.x}px"
        style:top="{b.y}px"
        style:--c={b.color}
        style:--a="{b.angle}deg"
        style:animation-delay="{(b.i % 6) * 0.35}s"
      ><b></b></i>
    {/each}
  {/if}
</span>

<style>
  .garland {
    position: absolute;
    inset: 0 0 auto;
    height: 34px;
    pointer-events: none;
    z-index: 2;
  }
  svg {
    position: absolute;
    inset: 0;
    overflow: visible;
  }
  .wire {
    fill: none;
    stroke: #1d3b2a;
    stroke-width: 1.8;
    stroke-linecap: round;
    filter: drop-shadow(0 1px 0.5px rgb(0 0 0 / 0.35));
  }
  .pin {
    fill: #1d3b2a;
  }
  /* Socket on the wire; the bulb hangs below it and turns with the wire. */
  .bulb {
    position: absolute;
    width: 6px;
    height: 5px;
    margin-left: -3px;
    border-radius: 1px;
    background: #3a4a3f;
    transform-origin: 50% 0;
    transform: rotate(var(--a, 0deg));
    animation: chase 2.1s ease-in-out infinite;
  }
  .bulb b {
    position: absolute;
    left: 50%;
    top: 4px;
    width: 10px;
    height: 14px;
    margin-left: -5px;
    border-radius: 50% 50% 50% 50% / 40% 40% 60% 60%;
    background:
      radial-gradient(circle at 35% 30%, rgb(255 255 255 / 0.85) 0 1.5px, transparent 2.5px),
      radial-gradient(circle at 50% 55%, color-mix(in srgb, var(--c) 70%, #fff) 0, var(--c) 60%, color-mix(in srgb, var(--c) 70%, #000) 100%);
    box-shadow:
      0 0 6px 2px color-mix(in srgb, var(--c) 75%, transparent),
      0 0 16px 6px color-mix(in srgb, var(--c) 35%, transparent);
  }
  @keyframes chase {
    0%,
    100% {
      filter: brightness(1);
    }
    50% {
      filter: brightness(0.55) saturate(0.8);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bulb {
      animation: none;
    }
  }
  :global(html[data-reduce-motion='true']) .bulb {
    animation: none;
  }
</style>
