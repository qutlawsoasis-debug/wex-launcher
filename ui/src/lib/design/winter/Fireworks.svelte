<!--
  Fireworks over the window: at midnight of 1 January for a minute, or a short
  show when the countdown chip is pressed (event `wex-fireworks` with
  detail.seconds). A canvas that never takes pointer input; it draws only while
  a show runs and pauses while the window is hidden.
  Apple has no guidance on seasonal decoration. HIG › Motion: decorative motion
  is optional and off under Reduce Motion (the mount checks it, and so does this).
  Colours are Apple system colours.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '../theme.svelte';
  import { burst, launchRocket, stepRocket, stepSpark, type Rocket, type Spark } from './fireworks';
  import { crossedMidnight } from './newyear';

  let canvas = $state<HTMLCanvasElement | null>(null);
  let on = $state(false);

  onMount(() => {
    const cv = canvas!;
    const ctx = cv.getContext('2d');
    if (!ctx) return;
    let rockets: Rocket[] = [];
    let sparks: Spark[] = [];
    let until = 0;
    let nextLaunch = 0;
    let raf = 0;
    let last = 0;
    let w = 0;
    let h = 0;
    let dpr = 1;

    const resize = () => {
      dpr = Math.min(2, window.devicePixelRatio || 1);
      w = window.innerWidth;
      h = window.innerHeight;
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const frame = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      if (!document.hidden) {
        if (now < until && now >= nextLaunch) {
          rockets.push(launchRocket(w, h, Math.random));
          if (Math.random() < 0.35) rockets.push(launchRocket(w, h, Math.random));
          nextLaunch = now + 420 + Math.random() * 700;
        }
        rockets = rockets.filter((r) => {
          if (stepRocket(r, dt)) {
            sparks.push(...burst(r, Math.random));
            return false;
          }
          return true;
        });
        sparks = sparks.filter((s) => stepSpark(s, dt));
        ctx.clearRect(0, 0, w, h);
        ctx.globalCompositeOperation = 'lighter';
        for (const r of rockets) {
          ctx.globalAlpha = 0.9;
          ctx.fillStyle = r.color;
          ctx.beginPath();
          ctx.arc(r.x, r.y, 2, 0, Math.PI * 2);
          ctx.fill();
          ctx.globalAlpha = 0.35;
          ctx.fillRect(r.x - 0.75, r.y, 1.5, 18);
        }
        for (const s of sparks) {
          const k = s.life / s.max;
          ctx.globalAlpha = Math.min(1, k * 1.4);
          ctx.fillStyle = s.color;
          ctx.beginPath();
          ctx.arc(s.x, s.y, s.size * (0.6 + k * 0.4), 0, Math.PI * 2);
          ctx.fill();
          // A soft halo around each spark (cheap: a bigger, faint dot).
          ctx.globalAlpha = Math.min(1, k * 1.4) * 0.22;
          ctx.beginPath();
          ctx.arc(s.x, s.y, s.size * 3.2, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.globalAlpha = 1;
        ctx.globalCompositeOperation = 'source-over';
      }
      if (now < until || rockets.length || sparks.length) raf = requestAnimationFrame(frame);
      else {
        raf = 0;
        on = false;
        ctx.clearRect(0, 0, w, h);
      }
    };

    const show = (seconds: number) => {
      if (theme.motionReduced) return;
      const now = performance.now();
      until = Math.max(until, now + seconds * 1000);
      on = true;
      if (!raf) {
        last = now;
        nextLaunch = now;
        raf = requestAnimationFrame(frame);
      }
    };

    const onShow = (e: Event) => show(Number((e as CustomEvent).detail?.seconds) || 8);
    // Midnight of 1 January while the launcher is open: a minute of fireworks.
    let prev = new Date();
    const tick = setInterval(() => {
      const now = new Date();
      if (crossedMidnight(prev, now)) show(60);
      prev = now;
    }, 1000);

    resize();
    window.addEventListener('resize', resize);
    window.addEventListener('wex-fireworks', onShow);
    return () => {
      cancelAnimationFrame(raf);
      clearInterval(tick);
      window.removeEventListener('resize', resize);
      window.removeEventListener('wex-fireworks', onShow);
    };
  });
</script>

<canvas class="fw" class:on bind:this={canvas} aria-hidden="true"></canvas>

<style>
  .fw {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    pointer-events: none;
    z-index: 41;
    display: none;
  }
  .fw.on {
    display: block;
  }
</style>
