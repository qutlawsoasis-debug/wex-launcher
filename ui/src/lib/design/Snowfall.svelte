<!--
  Snowfall for the winter look (season.svelte.ts). A canvas over the window that
  never takes pointer input. Apple has no guidance on seasonal decoration; the
  rules applied are HIG › Motion: decorative motion is gentle, never needed to use
  the app, and stays off under Reduce Motion (the app's switch or the system's);
  it pauses while the window is hidden or the launcher idles, so it costs nothing
  in the background. Flakes are light on dark and a cool grey on light, so they
  stay visible in both appearances without an accent colour.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from './theme.svelte';
  import { flakeCount, makeFlake, stepFlake, type Flake } from './snow';

  let canvas = $state<HTMLCanvasElement | null>(null);

  onMount(() => {
    const cv = canvas;
    const ctx = cv?.getContext('2d');
    if (!cv || !ctx) return;
    let flakes: Flake[] = [];
    let w = 0;
    let h = 0;
    let raf = 0;
    let last = performance.now();

    const resize = () => {
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      w = window.innerWidth;
      h = window.innerHeight;
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const n = flakeCount(w, h);
      while (flakes.length < n) flakes.push(makeFlake(w, h, Math.random, true));
      flakes.length = n;
    };

    const frame = (now: number) => {
      raf = requestAnimationFrame(frame);
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      if (document.hidden || document.documentElement.classList.contains('app-idle')) return;
      ctx.clearRect(0, 0, w, h);
      const dark = theme.isDark;
      ctx.fillStyle = dark ? '#ffffff' : '#8e9db3';
      for (const f of flakes) {
        stepFlake(f, dt, now / 1000, w, h, Math.random);
        ctx.globalAlpha = f.alpha * (dark ? 1 : 0.8);
        ctx.beginPath();
        ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    };

    resize();
    window.addEventListener('resize', resize);
    raf = requestAnimationFrame(frame);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener('resize', resize);
    };
  });
</script>

<canvas class="snow" bind:this={canvas} aria-hidden="true"></canvas>

<style>
  .snow {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    pointer-events: none;
    z-index: 40;
  }
</style>
