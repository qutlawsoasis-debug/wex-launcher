<!--
  Snowfall for the winter look (season.svelte.ts). A canvas over the window that
  never takes pointer input. Three depths (far, mid, near soft flakes) with depth
  parallax; a sweep of the pointer blows the snow, and flakes drift out of the
  cursor's way. Apple has no guidance on seasonal decoration; the rules applied
  are HIG › Motion: decorative motion is gentle, never needed to use the app, and
  stays off under Reduce Motion (the app's switch or the system's); it pauses
  while the window is hidden or the launcher idles, so it costs nothing in the
  background. Flakes are white on dark and a cool grey on light.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from './theme.svelte';
  import { layerCounts, makeFlake, stepFlake, type Flake, type Layer, type Pointer } from './snow';

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
    let pointer: Pointer | null = null;
    let prev: { x: number; t: number } | null = null;
    /** Wind from the pointer's sideways sweep, px/s; dies down by itself. */
    let wind = 0;
    /** Soft near flakes are drawn from a pre-rendered sprite (cheap blur). */
    let sprite: HTMLCanvasElement | null = null;
    let spriteDark: boolean | null = null;

    const makeSprite = (dark: boolean) => {
      const s = document.createElement('canvas');
      s.width = s.height = 64;
      const c = s.getContext('2d')!;
      const g = c.createRadialGradient(32, 32, 0, 32, 32, 32);
      const col = dark ? '255,255,255' : '142,157,179';
      g.addColorStop(0, `rgba(${col},1)`);
      g.addColorStop(0.45, `rgba(${col},0.55)`);
      g.addColorStop(1, `rgba(${col},0)`);
      c.fillStyle = g;
      c.fillRect(0, 0, 64, 64);
      return s;
    };

    const resize = () => {
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      w = window.innerWidth;
      h = window.innerHeight;
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const want = layerCounts(w, h);
      const next: Flake[] = [];
      ([0, 1, 2] as Layer[]).forEach((layer) => {
        const have = flakes.filter((f) => f.layer === layer).slice(0, want[layer]);
        while (have.length < want[layer]) have.push(makeFlake(w, h, Math.random, true, layer));
        next.push(...have);
      });
      flakes = next;
    };

    const onMove = (e: PointerEvent) => {
      const now = performance.now();
      if (prev && now > prev.t) {
        const v = ((e.clientX - prev.x) / (now - prev.t)) * 1000;
        // Only a real sweep makes wind; capped so the snow never races.
        wind = Math.max(-260, Math.min(260, wind + v * 0.08));
      }
      prev = { x: e.clientX, t: now };
      pointer = { x: e.clientX, y: e.clientY };
    };
    const onLeave = () => {
      pointer = null;
      prev = null;
    };

    const frame = (now: number) => {
      raf = requestAnimationFrame(frame);
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      if (document.hidden || document.documentElement.classList.contains('app-idle')) return;
      wind *= Math.exp(-dt / 0.9);
      ctx.clearRect(0, 0, w, h);
      const dark = theme.isDark;
      if (spriteDark !== dark) {
        sprite = makeSprite(dark);
        spriteDark = dark;
      }
      ctx.fillStyle = dark ? '#ffffff' : '#8e9db3';
      const t = now / 1000;
      for (const f of flakes) {
        stepFlake(f, dt, t, w, h, Math.random, wind, pointer);
        if (f.layer === 2 && sprite) {
          ctx.globalAlpha = f.alpha;
          const s = f.r * 4;
          ctx.drawImage(sprite, f.x - s / 2, f.y - s / 2, s, s);
        } else {
          // Small flakes catch the light now and then.
          const tw = f.layer === 0 ? 0.75 + 0.25 * Math.sin(t * 2.3 + f.phase * 3) : 1;
          ctx.globalAlpha = f.alpha * tw * (dark ? 1 : 0.8);
          ctx.beginPath();
          ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.globalAlpha = 1;
    };

    resize();
    window.addEventListener('resize', resize);
    window.addEventListener('pointermove', onMove, { passive: true });
    document.addEventListener('pointerleave', onLeave);
    window.addEventListener('blur', onLeave);
    raf = requestAnimationFrame(frame);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener('resize', resize);
      window.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerleave', onLeave);
      window.removeEventListener('blur', onLeave);
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
