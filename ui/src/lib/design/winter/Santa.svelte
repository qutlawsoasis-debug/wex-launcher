<!--
  Santa's sleigh flying across the window now and then (winter look).
  Apple has no guidance on seasonal art, so the drawing itself follows no HIG
  page. The rules applied are HIG › Motion: decorative motion is optional, never
  needed to use the app, and stays off under Reduce Motion. The mount in
  App.svelte already leaves it out then; the guard is repeated here
  (theme.motionReduced) in case it is ever mounted differently. Passes are
  scheduled with setTimeout (santa.ts), the flight and the gallop are CSS
  animations only (no per-frame script), nothing is scheduled while the window is
  hidden or the launcher idles, and it never takes pointer input. Colours: Apple
  systemRed #ff3b30 and systemYellow #ffd60a for the sleigh, coat and harness,
  systemBlue/systemGreen/systemPurple for the gifts; browns and whites are
  illustration colours where no system colour fits.
  Debug (decorative only): window.dispatchEvent(new Event('wex-santa')) flies a
  pass right away.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '../theme.svelte';
  import { firstDelay, makePass, makeSparkles, nextDelay, PAUSED_RETRY, starPath, type Pass } from './santa';
  import { playWinter } from './sounds';
  import { reportWinter } from './snowballs';

  interface Deer {
    x: number;
    y: number;
    s: number;
    far: boolean;
    rudolph: boolean;
    /** Gallop phase, seconds (negative animation delay). */
    phase: number;
  }

  /** Two pairs and Rudolph in front, climbing a little ahead of the sleigh. */
  const far: Deer[] = [
    { x: 334, y: 57, s: 0.9, far: true, rudolph: false, phase: -0.38 },
    { x: 238, y: 71, s: 0.9, far: true, rudolph: false, phase: -0.12 },
  ];
  const near: Deer[] = [
    { x: 422, y: 44, s: 0.96, far: false, rudolph: true, phase: -0.47 },
    { x: 320, y: 62, s: 1, far: false, rudolph: false, phase: -0.27 },
    { x: 222, y: 77, s: 1, far: false, rudolph: false, phase: 0 },
  ];
  /** A point of a reindeer drawing, in the rig's coordinates. */
  const at = (d: Deer, px: number, py: number) => `${d.x + d.s * px} ${d.y + d.s * py}`;
  /** Where the traces leave the sleigh (inside the front curl). */
  const HITCH = '193 128';
  const HAND = '165 101';

  const sparkles = makeSparkles(34).map((s) => ({ ...s, d: s.kind === 'star' ? starPath(s.r * 1.6) : '' }));

  // Reindeer parts, drawn facing right in a 0..92 box (body centre ≈ 40,38).
  const BODY = 'M17 36 C16 27 28 23.5 42 25.5 C50 26.5 56 24.5 62 26.5 C69 29.5 69.5 40 64 46 C57 51 46 50.5 36 49.5 C26 48.5 18 45 17 36 Z';
  const BELLY = 'M26 45.5 C36 49 50 49.5 60 46.5 C55 50.5 45 51 36 50 C31 49.5 28 48 26 45.5 Z';
  const HEAD =
    'M55.5 30 C59.5 24 63.5 18 67.5 13 C70.5 9 76.5 7.5 81 9.5 C84 10.8 87 12.8 90 14.3 C92.6 15.5 92.3 19 89.6 19.7 L83 20.6 C79 21.6 76.2 24 74.6 28 C72.4 34 70.2 40 65 45.5 C60.8 40.5 57.6 35.5 55.5 30 Z';
  const THROAT = 'M65 45.5 C69 39.5 72 33 74.6 28 C76.2 24.8 79 22.2 82.4 21 C79.4 24.4 77.2 29 75.2 34 C73.2 39 70.4 43.2 66.4 46.4 Z';
  const MUZZLE = 'M85.6 13.6 C89 14.2 92.2 15.4 92 17.6 C91.8 19.2 90.2 19.7 88 19.9 C87 18 86.2 15.8 85.6 13.6 Z';
  const EAR = 'M71 11.6 C67 7.8 62.4 7.2 60.8 8.8 C63 11.6 67 13.2 71.6 14 Z';
  const ANTLER_NEAR = 'M73.6 9.6 C72.8 3 70 -2 65 -6.2 M70.8 1.4 C73.4 -2 74 -6 72.6 -10.2 M67.6 -2.6 C64.4 -3.2 61.6 -1.6 59.4 1 M65.4 -5.2 C63.8 -8.6 64.2 -12 66.4 -14.4';
  const ANTLER_FAR = 'M77 9.2 C78.2 3 77.4 -3 74 -8.2 M77.2 1.8 C80.2 -1 82 -5 81.6 -9.2 M75.6 -4.2 C76.2 -8.2 78 -11 80.2 -12.4';
  const FRONT_UPPER = 'M-3.3 -1 C-3.6 5 -2.4 9 -1.7 12.4 L1.9 12.4 C2.8 8.4 3.8 4 3.6 -1 Z';
  const HIND_UPPER = 'M-5.4 -3 C-6.6 5 -3.4 11 -1.6 14.4 L2.2 14.4 C3.8 9.4 5.6 4 5.4 -3 Z';
  const LOWER = 'M-1.7 0 L-1.2 11 L1.3 11 L1.9 0 Z';
  const HOOF = 'M-1.7 10.6 L1.9 10.6 L2.5 13.6 L-1.9 13.6 Z';

  let pass = $state<Pass | null>(null);
  let run = $state(0);
  let finish: () => void = () => {};

  onMount(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let bells: ReturnType<typeof setTimeout> | undefined;
    const paused = () => document.hidden || document.documentElement.classList.contains('app-idle');
    const later = (ms: number, fn: () => void) => {
      clearTimeout(timer);
      timer = setTimeout(fn, ms);
    };
    const fly = () => {
      if (theme.motionReduced) return later(PAUSED_RETRY, tick);
      const p = makePass();
      pass = p;
      run += 1;
      // Hooks: «Видел Санту» when the window is visible; sleigh bells as he nears the middle.
      if (!document.hidden) reportWinter('saw_santa');
      clearTimeout(bells);
      bells = setTimeout(() => playWinter('bells'), p.duration * 0.3);
      // animationend normally ends the pass; this is only a safety net.
      later(p.duration * 2 + 5_000, finish);
    };
    const tick = () => (paused() || theme.motionReduced ? later(PAUSED_RETRY, tick) : fly());
    finish = () => {
      pass = null;
      later(nextDelay(), tick);
    };
    const onHook = () => fly();
    later(firstDelay(), tick);
    window.addEventListener('wex-santa', onHook);
    return () => {
      clearTimeout(timer);
      clearTimeout(bells);
      finish = () => {};
      window.removeEventListener('wex-santa', onHook);
    };
  });

  const onEnd = (e: AnimationEvent) => {
    if (e.target === e.currentTarget) finish();
  };
</script>

{#snippet leg(kind: 'front' | 'hind', px: number, py: number, d: Deer, back: boolean)}
  {@const delay = `animation-delay:${(d.phase + (back ? -0.11 : 0)).toFixed(2)}s`}
  <g transform="translate({px} {py})">
    <g class="leg" class:front={kind === 'front'} class:hind={kind === 'hind'}>
      <g class="upper" style={delay}>
        <path d={kind === 'front' ? FRONT_UPPER : HIND_UPPER} class={back ? 'fur-back' : 'fur-leg'} />
        <g transform="translate(0 {kind === 'front' ? 12 : 14})">
          <g class="lower" style={delay}>
            <path d={LOWER} class={back ? 'fur-back' : 'fur-leg'} />
            <path d={HOOF} fill="#2a1c13" />
          </g>
        </g>
      </g>
    </g>
  </g>
{/snippet}

{#snippet deer(d: Deer)}
  <g transform="translate({d.x} {d.y}) scale({d.s})" class="deer" class:far={d.far}>
    <g transform="translate(40 38)">
      <g class="gait" style="animation-delay:{d.phase}s">
        <g transform="translate(-40 -38)">
          {@render leg('hind', 25, 40, d, true)}
          {@render leg('front', 58, 42, d, true)}
          <path d="M19 31.5 C14 26.5 9.6 28 10.8 32.2 C12.4 35.4 17 35.2 20 34 Z" fill="#f4ede2" />
          <path d={BODY} fill={d.far ? 'url(#ws-fur-far)' : 'url(#ws-fur)'} />
          <path d={BELLY} fill="#e3c9a3" opacity={d.far ? 0.5 : 0.8} />
          <!-- Harness: saddle pad, girth and a breast strap with bells. -->
          <path d="M40 26.2 C46 24.4 52 24.6 57 26.4" stroke="#ff3b30" stroke-width="4.2" fill="none" stroke-linecap="round" />
          <path d="M40.6 24.2 C46 22.6 52 22.8 56.6 24.4" stroke="#ffd60a" stroke-width="0.9" fill="none" stroke-linecap="round" />
          <path d="M49 26 C48 34 48.4 42 50 49.6" stroke="#ff3b30" stroke-width="2.8" fill="none" stroke-linecap="round" />
          <rect x="47.1" y="35.4" width="3.2" height="3.6" rx="0.6" fill="none" stroke="#ffd60a" stroke-width="0.9" />
          <g transform="translate(55.5 30)">
            <g class="nod" style="animation-delay:{d.phase}s">
              <g transform="translate(-55.5 -30)">
                <path d={ANTLER_FAR} stroke={d.far ? '#8f7052' : '#a8865f'} stroke-width="1.7" fill="none" stroke-linecap="round" />
                <path d={HEAD} fill={d.far ? 'url(#ws-fur-far)' : 'url(#ws-fur)'} />
                <path d={THROAT} fill="#ecd8b8" opacity={d.far ? 0.55 : 0.9} />
                <path d={MUZZLE} fill={d.far ? '#4a2c1a' : '#5e3822'} />
                <path d={EAR} fill={d.far ? '#6a4329' : '#8f5a35'} />
                <path d="M69.6 12 C67 9.8 64.4 9.4 63.2 9.9 C65 11.6 67.4 12.6 70 13.2 Z" fill="#d9a184" opacity="0.7" />
                <path d={ANTLER_NEAR} stroke={d.far ? '#b89a76' : '#dcc092'} stroke-width="1.9" fill="none" stroke-linecap="round" />
                <circle cx="78.2" cy="13.2" r="1.25" fill="#160e09" />
                <circle cx="78.6" cy="12.8" r="0.4" fill="#fff" />
                <path d="M76.4 11.6 Q78.2 10.6 80 11.4" stroke="#3a2416" stroke-width="0.6" fill="none" stroke-linecap="round" />
                <!-- Collar with bells. -->
                <path d="M58 27.4 C62 31.4 66.6 36.2 69.8 41" stroke="#ff3b30" stroke-width="3.2" fill="none" stroke-linecap="round" />
                <circle cx="61.4" cy="31.8" r="1.7" fill="#ffd60a" />
                <circle cx="64.8" cy="35.4" r="1.7" fill="#ffd60a" />
                <circle cx="67.8" cy="39.2" r="1.7" fill="#ffd60a" />
                <!-- Bridle. -->
                <path d="M80.6 9.8 C81.6 13.6 83 17.6 84.4 20.4 M76 17.6 L88.6 17" stroke="#ff3b30" stroke-width="1.1" fill="none" stroke-linecap="round" />
                {#if d.rudolph}
                  <circle class="halo" cx="91.8" cy="16.8" r="15" fill="url(#ws-glow)" />
                  <circle class="nose" cx="92" cy="16.8" r="3.3" fill="url(#ws-nose)" />
                  <circle cx="91" cy="15.9" r="0.8" fill="#fff" opacity="0.85" />
                {/if}
              </g>
            </g>
          </g>
          {@render leg('hind', 25, 40, d, false)}
          {@render leg('front', 58, 42, d, false)}
        </g>
      </g>
    </g>
  </g>
{/snippet}

<div class="santa" aria-hidden="true">
  {#if pass}
    {#key run}
      <div
        class="flight"
        class:rtl={pass.dir === 'rtl'}
        style="--dur:{pass.duration}ms; --top:{pass.top}; --lift:{pass.lift}; --t0:{pass.dir === 'ltr' ? -pass.tilt : pass.tilt}deg; --t1:{pass.dir === 'ltr' ? pass.tilt : -pass.tilt}deg"
        onanimationend={onEnd}
      >
        <div class="arc">
          <svg viewBox="-170 -24 700 214" focusable="false">
            <defs>
              <linearGradient id="ws-lacquer" x1="0" y1="84" x2="0" y2="156" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#ff6a5f" />
                <stop offset="0.35" stop-color="#ff3b30" />
                <stop offset="0.78" stop-color="#c2241c" />
                <stop offset="1" stop-color="#861510" />
              </linearGradient>
              <linearGradient id="ws-coat" x1="0" y1="0" x2="0.3" y2="1">
                <stop offset="0" stop-color="#ff5a50" />
                <stop offset="0.5" stop-color="#ff3b30" />
                <stop offset="1" stop-color="#b0211a" />
              </linearGradient>
              <linearGradient id="ws-fur" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#7a482a" />
                <stop offset="0.55" stop-color="#9a6438" />
                <stop offset="1" stop-color="#c39262" />
              </linearGradient>
              <linearGradient id="ws-fur-far" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#57361f" />
                <stop offset="0.55" stop-color="#6e4629" />
                <stop offset="1" stop-color="#8d6541" />
              </linearGradient>
              <linearGradient id="ws-beard" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#ffffff" />
                <stop offset="1" stop-color="#d9dee8" />
              </linearGradient>
              <linearGradient id="ws-sack" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0" stop-color="#b98650" />
                <stop offset="1" stop-color="#6f4423" />
              </linearGradient>
              <linearGradient id="ws-skin" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#fbd6b9" />
                <stop offset="1" stop-color="#eeb18d" />
              </linearGradient>
              <radialGradient id="ws-nose" cx="0.35" cy="0.35" r="0.7">
                <stop offset="0" stop-color="#ff8f86" />
                <stop offset="0.5" stop-color="#ff3b30" />
                <stop offset="1" stop-color="#c81e17" />
              </radialGradient>
              <radialGradient id="ws-glow">
                <stop offset="0" stop-color="#ff3b30" stop-opacity="0.85" />
                <stop offset="0.4" stop-color="#ff3b30" stop-opacity="0.35" />
                <stop offset="1" stop-color="#ff3b30" stop-opacity="0" />
              </radialGradient>
              <radialGradient id="ws-pom" cx="0.4" cy="0.35" r="0.65">
                <stop offset="0" stop-color="#ffffff" />
                <stop offset="1" stop-color="#d4d9e4" />
              </radialGradient>
              <linearGradient id="ws-trail" x1="40" y1="0" x2="-168" y2="0" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#ffd60a" stop-opacity="0.6" />
                <stop offset="0.45" stop-color="#fff4c2" stop-opacity="0.28" />
                <stop offset="1" stop-color="#ffffff" stop-opacity="0" />
              </linearGradient>
              <!-- Moonlight from the upper left: a pale rim on the lit edges and a faint halo so the shape reads on the night sky. -->
              <filter id="ws-rim" x="-6%" y="-14%" width="112%" height="128%" color-interpolation-filters="sRGB">
                <feOffset in="SourceAlpha" dx="1.4" dy="1.6" result="shift" />
                <feComposite in="SourceAlpha" in2="shift" operator="out" result="edge" />
                <feGaussianBlur in="edge" stdDeviation="0.35" result="soft" />
                <feFlood flood-color="#e8f0ff" flood-opacity="0.95" />
                <feComposite in2="soft" operator="in" result="rim" />
                <feGaussianBlur in="SourceAlpha" stdDeviation="3.2" result="spread" />
                <feFlood flood-color="#a9c1ff" flood-opacity="0.32" />
                <feComposite in2="spread" operator="in" result="halo" />
                <feMerge>
                  <feMergeNode in="halo" />
                  <feMergeNode in="SourceGraphic" />
                  <feMergeNode in="rim" />
                </feMerge>
              </filter>
              <filter id="ws-blur" x="-10%" y="-60%" width="120%" height="220%">
                <feGaussianBlur stdDeviation="2.4" />
              </filter>
            </defs>

            <g class="bob">
              <!-- Trail: a soft ribbon of stardust and glitter shed behind the runners. -->
              <g transform={pass.dir === 'rtl' ? 'translate(360 0) scale(-1 1)' : undefined}>
                <path d="M44 170 C4 160 -76 150 -168 128 L-168 146 C-76 170 4 178 44 172 Z" fill="url(#ws-trail)" filter="url(#ws-blur)" />
                {#each sparkles as s, i (i)}
                  <g transform="translate({s.x.toFixed(1)} {s.y.toFixed(1)})">
                    {#if s.kind === 'star'}
                      <path
                        class="spark"
                        d={s.d}
                        fill={s.warm ? '#ffe680' : '#ffffff'}
                        style="--dx:{s.dx.toFixed(0)}px; --dy:{s.dy.toFixed(0)}px; animation-duration:{s.dur.toFixed(2)}s; animation-delay:{s.delay.toFixed(2)}s"
                      />
                    {:else}
                      <circle
                        class="spark"
                        r={(s.r * 0.55).toFixed(2)}
                        fill={s.warm ? '#ffd60a' : '#ffffff'}
                        style="--dx:{s.dx.toFixed(0)}px; --dy:{s.dy.toFixed(0)}px; animation-duration:{s.dur.toFixed(2)}s; animation-delay:{s.delay.toFixed(2)}s"
                      />
                    {/if}
                  </g>
                {/each}
              </g>

              <g filter="url(#ws-rim)">
                <g transform={pass.dir === 'rtl' ? 'translate(360 0) scale(-1 1)' : undefined}>
                  <!-- Far side of the team, behind. -->
                  {#each far as d (d.x)}
                    <path d="M{HITCH} Q{d.x - 24} {d.y + 52} {at(d, 49, 40)}" stroke="#7a1a14" stroke-width="1.3" fill="none" />
                  {/each}
                  <path d="M{at(far[1], 66, 42)} Q{far[0].x - 18} {far[0].y + 46} {at(far[0], 49, 40)}" stroke="#7a1a14" stroke-width="1.2" fill="none" />
                  {#each far as d (d.x)}{@render deer(d)}{/each}

                  <!-- Sleigh: it rocks a little on the hitch so the traces stay attached. -->
                  <g transform="translate(193 128)">
                    <g class="rock">
                      <g transform="translate(-193 -128)">
                        <!-- Sack of presents in the back. -->
                        <path d="M66 124 C57 108 60 89 72 80.5 L100.5 80.5 C111 90 113 108 106.5 124 Z" fill="url(#ws-sack)" />
                        <ellipse cx="86.2" cy="80.6" rx="14.8" ry="3.8" fill="#4a2e17" />
                        <g transform="translate(93 62) rotate(11)">
                          <rect x="-7.6" y="-7.6" width="15.2" height="15.2" rx="1.2" fill="#30d158" />
                          <rect x="-1.4" y="-7.6" width="2.8" height="15.2" fill="#ff3b30" />
                          <rect x="-7.6" y="-1.4" width="15.2" height="2.8" fill="#ff3b30" />
                          <path d="M0 -7.6 C-4 -12.4 -8 -10.2 -5.6 -7.6 Z M0 -7.6 C4 -12.4 8 -10.2 5.6 -7.6 Z" fill="#ff3b30" />
                        </g>
                        <g transform="translate(76 66.5) rotate(-15)">
                          <rect x="-7" y="-7.4" width="14" height="14.4" rx="1.2" fill="#0a84ff" />
                          <rect x="-1.2" y="-7.4" width="2.4" height="14.4" fill="#ffd60a" />
                          <rect x="-7" y="-1.2" width="14" height="2.4" fill="#ffd60a" />
                          <path d="M0 -7.4 C-3.6 -11.8 -7.4 -9.8 -5 -7.4 Z M0 -7.4 C3.6 -11.8 7.4 -9.8 5 -7.4 Z" fill="#ffd60a" />
                        </g>
                        <g transform="translate(85 73.5) rotate(4)">
                          <rect x="-5.6" y="-4.8" width="11.2" height="9.6" rx="1" fill="#bf5af2" />
                          <rect x="-1" y="-4.8" width="2" height="9.6" fill="#ffffff" />
                        </g>
                        <path d="M71 81.4 C78 86.4 94.6 86.4 101.6 81.4" stroke="#8a5a30" stroke-width="3.4" fill="none" stroke-linecap="round" />
                        <path d="M79 92 C76.6 104 78 114 82 122 M95 90 C99 100 99.6 112 97 122" stroke="#5c3818" stroke-width="1" fill="none" opacity="0.55" />

                        <!-- Santa. -->
                        <path d="M99.5 128 C96 108 100 90 112 84.4 L132.6 84.4 C142.6 90 146.4 108 144.6 128 Z" fill="url(#ws-coat)" />
                        <path d="M135.6 86 C139.8 96 141.6 110 141.8 128 L135.8 128 C135.8 112 134 98.4 131 88.4 Z" fill="#f7f7fa" />
                        <path d="M99.2 111.6 C112 113.8 130 113.8 145.2 110.8 L145.6 119 C130 122 112 122 99.6 120 Z" fill="#1c1c1e" />
                        <rect x="129.2" y="110.4" width="9.6" height="10.6" rx="1.4" fill="#1c1c1e" stroke="#ffd60a" stroke-width="2" />
                        <!-- Waving arm: raised from the far shoulder, behind the head. -->
                        <g transform="translate(130 89)">
                          <g class="wave">
                            <path d="M0 0 C6 -8 12 -18 14 -30" stroke="#e8322a" stroke-width="8.6" fill="none" stroke-linecap="round" />
                            <path d="M3 2 C9 -6 15 -16 17 -27" stroke="#a91f18" stroke-width="2.2" fill="none" stroke-linecap="round" opacity="0.5" />
                            <ellipse cx="14.4" cy="-31.6" rx="6.2" ry="3.5" fill="#f7f7fa" transform="rotate(14 14.4 -31.6)" />
                            <path d="M10.6 -34 C8.8 -40.2 11.6 -46 15.8 -46 C20 -46 21.8 -42 21 -37 C20.6 -34.6 18.6 -33.2 15.8 -33.2 Z" fill="#2c2c2e" />
                            <path d="M11.2 -38.6 C8.4 -40.8 6 -39.2 7.8 -36.2 C8.6 -35.2 10 -35 11.2 -35.4" fill="#2c2c2e" />
                          </g>
                        </g>
                        <!-- Head, beard and hat. -->
                        <ellipse cx="119.4" cy="73.4" rx="2.2" ry="3" fill="#eeb18d" />
                        <ellipse cx="127.4" cy="73" rx="8.8" ry="9.2" fill="url(#ws-skin)" />
                        <path d="M117 70 C113 80 111.6 94 119.6 104.2 C123.8 108.6 130.4 108.6 134.4 104.2 C141.4 96 142.4 84 137.4 71.6 C133.4 79 122.4 79 117 70 Z" fill="url(#ws-beard)" />
                        <path d="M117.4 80 C110 84 105.6 92.6 107.6 99 C111.4 95 114 91 118 88.4 Z M118.4 93 C113 98 111.6 104 113.4 108 C116 104.6 118.4 101.6 121.4 100.4 Z" fill="#eef0f5" />
                        <path d="M121.6 88 C123 95 125 100 127.4 104 M129.6 86.6 C131 93 131 99 130.2 104" stroke="#c9cfdb" stroke-width="0.8" fill="none" stroke-linecap="round" />
                        <path d="M126 78.4 C129.6 75.2 134.6 75 138.4 77.8 C135.6 80.4 131.2 80.6 127.8 79.4 Z" fill="#ffffff" />
                        <path d="M127 78.6 C123.6 78 121 80 119.8 82.6 C123.4 82.4 126 81 127.6 79.6 Z" fill="#f4f5f8" />
                        <circle cx="131.2" cy="76.6" r="3" fill="#ff6961" opacity="0.55" />
                        <circle cx="135.4" cy="74.6" r="2.3" fill="#f39c86" />
                        <circle cx="134.9" cy="73.9" r="0.6" fill="#fff" opacity="0.7" />
                        <ellipse cx="131.8" cy="70.2" rx="1.1" ry="1.3" fill="#2b1d16" />
                        <path d="M129 67.2 Q131.8 65 135 66.6" stroke="#ffffff" stroke-width="1.7" fill="none" stroke-linecap="round" />
                        <path d="M117 68.2 C116 57 124 50 133 52 C138.2 54 140.2 60 140.4 66.4 Z" fill="url(#ws-coat)" />
                        <g transform="translate(121 57)">
                          <g class="flap">
                            <path d="M0 -2 C-9 -10 -21 -10 -30 -2 C-22 -4.4 -12.6 -2.6 -2.4 6 Z" fill="#e8322a" />
                            <path d="M-4 -3.6 C-11 -7.6 -19 -7.4 -26 -3.4" stroke="#ff6a5f" stroke-width="0.9" fill="none" opacity="0.7" />
                            <circle cx="-31" cy="-1.2" r="5.6" fill="url(#ws-pom)" />
                          </g>
                        </g>
                        <path d="M114.4 64 C122 61.4 132 61.4 141.2 64.4 C141.8 67 141.4 69.6 140.4 71.2 C131 68.6 122 68.6 115 71.2 C114 69 113.8 66.2 114.4 64 Z" fill="#f7f7fa" />
                        <path d="M116 66.6 C123 64.8 131 64.8 139 66.8" stroke="#d7dbe4" stroke-width="0.8" fill="none" />

                        <!-- Sleigh body: lacquered red with gilt edges and scrolls. -->
                        <path
                          d="M56 154 C44 150 36 132 37 112 C38 96 46 84 58 84 C68 84 72 94 66 100 C62 104 60 110 66 118 C72 124 82 126 96 126 L148 126 C162 126 170 120 177 110 C183 101 194 98 200 104 C206 111 200 124 190 134 C178 147 160 155 138 155 Z"
                          fill="url(#ws-lacquer)"
                          stroke="#ffd60a"
                          stroke-width="2.2"
                          stroke-linejoin="round"
                        />
                        <path d="M58 84 C68 84 72 94 66 100 C62 104 56.4 100.4 58.4 96.4 C60.2 93.2 64 94.4 63 97.4" stroke="#ffd60a" stroke-width="2" fill="none" stroke-linecap="round" />
                        <path d="M200 104 C196 98.6 188.4 100.4 188.4 106.4 C188.4 110.4 193 111.4 194.2 108.4" stroke="#ffd60a" stroke-width="2" fill="none" stroke-linecap="round" />
                        <path d="M60 145 C51 137 48.4 124 52 112 M78 146.6 L136 146.6 C155 146.6 168 141 178.6 131" stroke="#ffd60a" stroke-width="1.3" fill="none" stroke-linecap="round" opacity="0.9" />
                        <path d="M112.4 138.6 c-6 -6.4 -14.4 -2.4 -12.4 3.6 c1.8 4.8 8 4 7.6 0 M120.4 138.6 c6 -6.4 14.4 -2.4 12.4 3.6 c-1.8 4.8 -8 4 -7.6 0" stroke="#ffd60a" stroke-width="1.4" fill="none" stroke-linecap="round" />
                        <path d="M116.4 133.4 L119 136.6 L116.4 139.8 L113.8 136.6 Z" fill="#ffd60a" />
                        <path d="M86 134 c-4 -1 -6.6 2.4 -4.4 5.4 M150 132 c4 -1 6.6 2.4 4.4 5.4" stroke="#ffd60a" stroke-width="1.2" fill="none" stroke-linecap="round" opacity="0.85" />
                        <path d="M42.6 112 C42.4 100 48 91 55 89.4 M150 130.6 C163 130.4 171.6 124 179 114" stroke="#ffffff" stroke-width="1.8" fill="none" stroke-linecap="round" opacity="0.38" />
                        <!-- Struts and runners curling up at the front. -->
                        <path d="M74 153 L72 173 M108 155.4 L108 173.4 M146 154.6 L148 173.4 M178 143 L184 170" stroke="#e0b400" stroke-width="2.6" stroke-linecap="round" />
                        <path d="M30 172.4 C33 177 39.4 176.4 44 174.4 L184 174.4 C202 174.4 214.4 164 214.4 150 C214.4 140 205 136 200.4 142 C196.6 147 202 152 207 148" stroke="#ffd60a" stroke-width="3.3" fill="none" stroke-linecap="round" stroke-linejoin="round" />
                        <path d="M44 176.6 L184 176.6 C200 176.6 213 168 216.2 154" stroke="#b58800" stroke-width="1" fill="none" stroke-linecap="round" opacity="0.8" />

                        <!-- Arm forward with the reins. -->
                        <path d="M133.6 93.4 C143.6 99.6 151.6 103.6 159.6 102.2" stroke="#e8322a" stroke-width="8" fill="none" stroke-linecap="round" />
                        <path d="M135 97.4 C144 102.6 151 105.4 158.6 104.6" stroke="#a91f18" stroke-width="2" fill="none" stroke-linecap="round" opacity="0.5" />
                        <ellipse cx="159.4" cy="102" rx="3.4" ry="5.6" fill="#f7f7fa" transform="rotate(-12 159.4 102)" />
                        <circle cx="165" cy="101.2" r="4.3" fill="#2c2c2e" />
                      </g>
                    </g>
                  </g>

                  <!-- Near side of the team: traces from the hitch, reins from Santa's hand. -->
                  <path d="M{HITCH} Q{near[2].x + 6} {near[2].y + 54} {at(near[2], 49, 40)}" stroke="#ffd60a" stroke-width="1.5" fill="none" />
                  <path d="M{at(near[2], 66, 42)} Q{near[1].x - 6} {near[1].y + 48} {at(near[1], 49, 40)}" stroke="#ffd60a" stroke-width="1.4" fill="none" />
                  <path d="M{at(near[1], 66, 42)} Q{near[0].x - 6} {near[0].y + 46} {at(near[0], 49, 40)}" stroke="#ffd60a" stroke-width="1.4" fill="none" />
                  {#each near as d (d.x)}{@render deer(d)}{/each}
                  <path d="M{HAND} Q{near[2].x + 34} {near[2].y + 34} {at(near[2], 84.4, 20)}" stroke="#7a1a14" stroke-width="1" fill="none" />
                  <path d="M{HAND} Q{near[1].x - 10} {near[1].y + 22} {at(near[1], 84.4, 20)}" stroke="#7a1a14" stroke-width="0.9" fill="none" />
                  <path d="M{HAND} Q{near[0].x - 20} {near[0].y + 16} {at(near[0], 84.4, 20)}" stroke="#7a1a14" stroke-width="0.8" fill="none" />
                </g>
              </g>
            </g>
          </svg>
        </div>
      </div>
    {/key}
  {/if}
</div>

<style>
  .santa {
    position: fixed;
    inset: 0;
    z-index: 39;
    overflow: hidden;
    pointer-events: none;
  }
  .flight {
    position: absolute;
    left: 0;
    top: calc(var(--top) * 100vh);
    width: clamp(250px, 29vw, 620px);
    animation: fly-ltr var(--dur) linear both;
    will-change: transform;
  }
  .flight.rtl {
    animation-name: fly-rtl;
  }
  .arc {
    animation: arc var(--dur) both;
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  @keyframes fly-ltr {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(100vw);
    }
  }
  @keyframes fly-rtl {
    from {
      transform: translateX(100vw);
    }
    to {
      transform: translateX(-100%);
    }
  }
  /* Rises to the middle of the window and sinks again (santa.ts › arcPoint). */
  @keyframes arc {
    0% {
      transform: translateY(0) rotate(var(--t0));
      animation-timing-function: cubic-bezier(0.33, 0.66, 0.66, 1);
    }
    50% {
      transform: translateY(calc(var(--lift) * -100vh)) rotate(0deg);
      animation-timing-function: cubic-bezier(0.33, 0, 0.66, 0.34);
    }
    100% {
      transform: translateY(0) rotate(var(--t1));
    }
  }

  .bob {
    animation: bob 1.9s ease-in-out infinite alternate;
  }
  @keyframes bob {
    from {
      transform: translateY(-2.5px);
    }
    to {
      transform: translateY(3px);
    }
  }
  .rock {
    animation: rock 1.6s ease-in-out infinite alternate;
  }
  @keyframes rock {
    from {
      transform: rotate(-1.2deg);
    }
    to {
      transform: rotate(1.4deg);
    }
  }
  .wave {
    animation: wave 0.9s ease-in-out infinite alternate;
  }
  @keyframes wave {
    from {
      transform: rotate(-12deg);
    }
    to {
      transform: rotate(14deg);
    }
  }
  .flap {
    animation: flap 0.42s ease-in-out infinite alternate;
  }
  @keyframes flap {
    from {
      transform: rotate(-5deg);
    }
    to {
      transform: rotate(6deg);
    }
  }

  /* Gallop: three poses per leg segment, the far legs a beat behind. */
  .fur-leg {
    fill: #7a4a2c;
  }
  .fur-back {
    fill: #573520;
  }
  .far .fur-leg {
    fill: #5a3820;
  }
  .far .fur-back {
    fill: #432817;
  }
  .leg .upper,
  .leg .lower,
  .gait,
  .nod {
    animation-duration: 0.62s;
    animation-iteration-count: infinite;
    animation-timing-function: ease-in-out;
  }
  .front .upper {
    animation-name: front-upper;
  }
  .front .lower {
    animation-name: front-lower;
  }
  .hind .upper {
    animation-name: hind-upper;
  }
  .hind .lower {
    animation-name: hind-lower;
  }
  .gait {
    animation-name: gait;
  }
  .nod {
    animation-name: nod;
  }
  @keyframes front-upper {
    0%,
    100% {
      transform: rotate(-62deg);
    }
    40% {
      transform: rotate(-28deg);
    }
    70% {
      transform: rotate(8deg);
    }
  }
  @keyframes front-lower {
    0%,
    100% {
      transform: rotate(98deg);
    }
    40% {
      transform: rotate(52deg);
    }
    70% {
      transform: rotate(8deg);
    }
  }
  @keyframes hind-upper {
    0%,
    100% {
      transform: rotate(52deg);
    }
    40% {
      transform: rotate(18deg);
    }
    70% {
      transform: rotate(-30deg);
    }
  }
  @keyframes hind-lower {
    0%,
    100% {
      transform: rotate(14deg);
    }
    40% {
      transform: rotate(40deg);
    }
    70% {
      transform: rotate(72deg);
    }
  }
  @keyframes gait {
    0%,
    100% {
      transform: translateY(0) rotate(-2deg);
    }
    50% {
      transform: translateY(-2.4px) rotate(2deg);
    }
  }
  @keyframes nod {
    0%,
    100% {
      transform: rotate(2deg);
    }
    50% {
      transform: rotate(-4deg);
    }
  }

  /* Rudolph's nose glows and pulses. */
  .halo {
    transform-box: fill-box;
    transform-origin: center;
    animation: halo 1.1s ease-in-out infinite alternate;
  }
  .nose {
    animation: nose 1.1s ease-in-out infinite alternate;
  }
  @keyframes halo {
    from {
      opacity: 0.55;
      transform: scale(0.75);
    }
    to {
      opacity: 1;
      transform: scale(1.25);
    }
  }
  @keyframes nose {
    from {
      filter: brightness(0.92);
    }
    to {
      filter: brightness(1.25);
    }
  }

  /* Glitter: shed at the runners, drifts back, twinkles and fades. */
  .spark {
    animation-name: shed;
    animation-timing-function: ease-out;
    animation-iteration-count: infinite;
  }
  @keyframes shed {
    0% {
      transform: translate(0, 0) scale(1) rotate(0deg);
      opacity: 0;
    }
    8% {
      opacity: 1;
    }
    30% {
      opacity: 0.6;
    }
    45% {
      opacity: 1;
    }
    100% {
      transform: translate(var(--dx), var(--dy)) scale(0.2) rotate(90deg);
      opacity: 0;
    }
  }

  /* Nothing moves while the launcher idles. */
  :global(html.app-idle) .santa * {
    animation-play-state: paused;
  }
</style>
