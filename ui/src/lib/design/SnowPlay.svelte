<!--
  Snowman building on the snowdrift of the open build tile (winter look).
  Press on the snow and drag to roll a snowball (it grows with the distance);
  press on a ball and drag to carry it, release over another ball to stack it.
  Three balls, each smaller than the one below, become a snowman. Double-click a
  ball to break it. The scene is kept for this tile on this computer.
  A decorative toy: Apple has no guidance for it. HIG › Pointing devices: grab
  and grabbing pointers show what can be moved; HIG › Motion: falling and the
  sparkle are skipped under Reduce Motion. The Play button keeps working where
  the snow overlaps it.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { driftHeights } from './drift';
  import { theme } from './theme.svelte';
  import { toasts } from '../shell/toast.svelte';
  import { t } from '../i18n';
  import { playWinter } from './winter/sounds';
  import { reportWinter } from './winter/snowballs';
  import { cap, drop, grow, load, pickUp, restOnSnow, R_MIN, save, settle, snowmen, surfaceAt, type Ball } from './snowplay';

  interface Props {
    /** The build id: the drift shape and the saved scene. */
    seed: string;
    /** Height of the snowdrift, px (SnowDrift). */
    driftH?: number;
  }

  let { seed, driftH = 44 }: Props = $props();

  let W = $state(0);
  let H = $state(0);
  let balls = $state<Ball[]>([]);
  let nextId = 1;
  let mode: { kind: 'roll'; id: number; x: number; y: number } | { kind: 'carry'; id: number; dx: number; dy: number } | null = null;
  let active = $state<number | null>(null);
  let sparkle = $state<number | null>(null);
  let layer: HTMLElement | undefined = $state();

  const heights = $derived(driftHeights(seed, 48, 0.22, 0.9, 1));
  const surface = (x: number) => surfaceAt(x, W, H, heights, driftH);
  const men = $derived(snowmen(balls));
  const heads = $derived(new Set(men.map((m) => m.head)));
  const middles = $derived(new Set(men.map((m) => m.middle)));
  let known = new Set<string>();

  const KEY = () => `wex_snowman_${seed}`;
  function persist() {
    try {
      localStorage.setItem(KEY(), JSON.stringify(save(balls, W)));
    } catch {
      // Storage can be unavailable; the scene just is not kept.
    }
  }

  function measure(node: HTMLElement) {
    const read = () => {
      const w = node.clientWidth;
      const h = node.clientHeight;
      if (!w || !h) return;
      const first = W === 0;
      const fx = W > 0 ? w / W : 1;
      W = w;
      H = h;
      if (first) {
        let saved: unknown = null;
        try {
          saved = JSON.parse(localStorage.getItem(KEY()) ?? 'null');
        } catch {}
        balls = load(saved, W, surface);
        nextId = Math.max(0, ...balls.map((b) => b.id)) + 1;
        known = new Set(snowmen(balls).map((m) => `${m.base}-${m.middle}-${m.head}`));
      } else {
        balls = settle(
          balls.map((b) => ({ ...b, x: b.x * fx })),
          surface
        );
      }
    };
    read();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(read);
    ro.observe(node);
    return { destroy: () => ro.disconnect() };
  }

  function local(e: PointerEvent) {
    const r = layer!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  /** The Play button keeps its clicks where the snow lies over it. */
  function overPlay(e: PointerEvent): HTMLButtonElement | null {
    const btn = layer?.parentElement?.querySelector<HTMLButtonElement>('.pane-play');
    if (!btn) return null;
    const r = btn.getBoundingClientRect();
    return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom ? btn : null;
  }

  function onSnowDown(e: PointerEvent) {
    const play = overPlay(e);
    if (play) {
      if (!play.disabled) play.click();
      return;
    }
    if (e.button !== 0) return;
    const p = local(e);
    if (p.y < surface(p.x) - 2) return;
    e.preventDefault();
    const b: Ball = { id: nextId++, x: p.x, y: 0, r: R_MIN, on: null, roll: 0 };
    b.y = restOnSnow(b, surface);
    balls = cap([...balls, b]);
    mode = { kind: 'roll', id: b.id, x: p.x, y: p.y };
    active = b.id;
    playWinter('crunch');
    follow();
  }

  function onBallDown(e: PointerEvent, b: Ball) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const p = local(e);
    mode = { kind: 'carry', id: b.id, dx: b.x - p.x, dy: b.y - p.y };
    active = b.id;
    const rest = settle(
      pickUp(b.id, balls).filter((x) => x.id !== b.id),
      surface
    );
    // Same order as before (a keyed move would drop the pointer); the held ball is raised by CSS.
    balls = balls.map((x) => (x.id === b.id ? { ...b, on: null } : rest.find((r) => r.id === x.id) ?? x));
    follow();
  }

  function onMove(e: PointerEvent) {
    if (!mode) return;
    const p = local(e);
    if (mode.kind === 'roll') {
      const m = mode;
      const d = Math.hypot(p.x - m.x, p.y - m.y);
      m.x = p.x;
      m.y = p.y;
      if (d > 0.5) playWinter('crunch');
      balls = balls.map((b) => {
        if (b.id !== m.id) return b;
        const r = grow(b.r, d);
        const x = Math.max(r, Math.min(W - r, p.x));
        const n = { ...b, r, x, roll: b.roll + d };
        n.y = restOnSnow(n, surface);
        return n;
      });
    } else {
      const m = mode;
      balls = balls.map((b) => {
        if (b.id !== m.id) return b;
        const x = Math.max(b.r, Math.min(W - b.r, p.x + m.dx));
        const y = Math.max(b.r, Math.min(H - b.r, p.y + m.dy));
        return { ...b, x, y, roll: b.roll + Math.abs(x - b.x) * 0.3 };
      });
    }
  }

  /** While rolling or carrying, the whole window drives the ball (the pointer may leave the snow). */
  function follow() {
    const up = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', up);
      onUp();
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', up);
  }

  function onUp() {
    if (!mode) return;
    const id = mode.id;
    const carried = mode.kind === 'carry';
    mode = null;
    active = null;
    balls = carried ? drop(id, balls, surface) : settle(balls, surface);
    celebrate();
    persist();
  }

  function breakBall(b: Ball) {
    balls = settle(
      pickUp(b.id, balls).filter((x) => x.id !== b.id),
      surface
    );
    persist();
  }

  let melting = $state(false);
  /** «Убрать снеговиков»: the snow melts away and the drift is clean for new ones. */
  function clearAll() {
    if (!balls.length || melting) return;
    const done = () => {
      balls = [];
      known = new Set();
      melting = false;
      persist();
    };
    if (theme.motionReduced) return done();
    melting = true;
    setTimeout(done, 450);
  }

  function celebrate() {
    const now = new Set(snowmen(balls).map((m) => `${m.base}-${m.middle}-${m.head}`));
    for (const k of now) {
      if (known.has(k)) continue;
      const head = +k.split('-')[2];
      if (!theme.motionReduced) {
        sparkle = head;
        setTimeout(() => sparkle === head && (sparkle = null), 1200);
      }
      toasts.show(t('home.snowman_done'));
      // Hooks: a chime and the winter achievements.
      playWinter('chime');
      reportWinter('first_snowman');
      if (now.size >= 3) reportWinter('snowman_town', 3);
    }
    known = now;
  }

  onMount(() => () => persist());
</script>

{#if balls.length}
  <button type="button" class="clear-snow" disabled={melting} onclick={clearAll}>
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 1.5v13M2.4 4.75l11.2 6.5M2.4 11.25l11.2-6.5M6.2 2.6 8 4.2l1.8-1.6M6.2 13.4 8 11.8l1.8 1.6" /></svg>
    {t('home.snow_clear')}
  </button>
{/if}
<span class="play" class:still={theme.motionReduced} class:melting bind:this={layer} use:measure aria-hidden="true">
  <!-- A pointer toy inside an aria-hidden layer: nothing for assistive tech or the keyboard. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="snow-hit" data-no-snowball style:height="{driftH}px" title={t('home.snow_hint')} onpointerdown={onSnowDown}></span>
  {#each balls as b (b.id)}
    {@const isHead = heads.has(b.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      class="ball"
      data-no-snowball
      class:held={active === b.id}
      class:spark={sparkle === b.id}
      style:left="{b.x - b.r}px"
      style:top="{b.y - b.r}px"
      style:width="{b.r * 2}px"
      style:height="{b.r * 2}px"
      style:--r="{b.r}px"
      onpointerdown={(e) => onBallDown(e, b)}
      ondblclick={() => breakBall(b)}
    >
      <i class="tex" style:transform="rotate({(b.roll / b.r) % 6.283}rad)"></i>
      {#if isHead}
        <i class="eye l"></i><i class="eye r"></i><i class="nose"></i><i class="smile"></i>
      {/if}
      {#if middles.has(b.id)}
        <i class="btn1"></i><i class="btn2"></i><i class="arm l"></i><i class="arm r"></i>
      {/if}
    </span>
  {/each}
</span>

<style>
  /* Secondary action (HIG › Buttons): a small glass pill under «Открыть сборку»;
     Play stays the one prominent button on the tile. */
  .clear-snow {
    position: absolute;
    top: 62px;
    right: 28px;
    z-index: 3;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-pill);
    background: rgb(255 255 255 / 0.16);
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.22);
    color: #fff;
    font: inherit;
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    cursor: default;
    transition: background-color var(--dur-micro);
  }
  .clear-snow:hover {
    background: rgb(255 255 255 / 0.26);
  }
  .clear-snow:disabled {
    opacity: 0.6;
  }
  .clear-snow svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
  }
  :global(html[data-reduce-transparency='true']) .clear-snow {
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
    background: rgb(0 0 0 / 0.35);
  }
  .melting .ball {
    transition:
      transform 0.45s ease-in,
      opacity 0.45s ease-in;
    transform: scale(0.2) translateY(40%);
    opacity: 0;
  }
  .play {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
  }
  .snow-hit {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    pointer-events: auto;
    cursor: grab;
    touch-action: none;
  }
  .snow-hit:active {
    cursor: grabbing;
  }
  .ball {
    position: absolute;
    pointer-events: auto;
    cursor: grab;
    touch-action: none;
    border-radius: 50%;
    background: radial-gradient(circle at 34% 30%, #ffffff 0, #f4f8fd 38%, #dbe6f3 72%, #b9c9de 100%);
    box-shadow:
      inset calc(var(--r) * -0.12) calc(var(--r) * -0.18) calc(var(--r) * 0.35) rgb(110 140 185 / 0.3),
      0 calc(var(--r) * 0.08 + 1px) calc(var(--r) * 0.2 + 2px) rgb(0 0 0 / 0.22);
    transition:
      top 0.38s cubic-bezier(0.5, 0, 1, 1),
      left 0.2s ease-out;
  }
  .ball.held {
    z-index: 1;
    cursor: grabbing;
    transition: none;
    filter: drop-shadow(0 6px 8px rgb(0 0 0 / 0.25));
  }
  .still .ball {
    transition: none;
  }
  /* Packed-snow texture that turns as the ball rolls. */
  .tex {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(circle at 30% 62%, rgb(150 175 210 / 0.35) 0 6%, transparent 7%),
      radial-gradient(circle at 66% 40%, rgb(150 175 210 / 0.3) 0 5%, transparent 6%),
      radial-gradient(circle at 52% 78%, rgb(150 175 210 / 0.28) 0 4%, transparent 5%),
      radial-gradient(circle at 74% 70%, rgb(255 255 255 / 0.7) 0 5%, transparent 6%);
  }
  /* Snowman: coal eyes, a carrot nose, a smile; buttons and stick arms on the middle. */
  .eye,
  .btn1,
  .btn2 {
    position: absolute;
    width: calc(var(--r) * 0.2);
    height: calc(var(--r) * 0.2);
    border-radius: 50%;
    background: #1c1c1e;
  }
  .eye {
    top: 34%;
  }
  .eye.l {
    left: 30%;
  }
  .eye.r {
    right: 30%;
  }
  .nose {
    position: absolute;
    left: 46%;
    top: 50%;
    width: calc(var(--r) * 0.75);
    height: calc(var(--r) * 0.2);
    background: linear-gradient(90deg, #ff9f0a, #ff7a00);
    clip-path: polygon(0 0, 100% 50%, 0 100%);
  }
  .smile {
    position: absolute;
    left: 30%;
    right: 30%;
    top: 62%;
    height: calc(var(--r) * 0.35);
    border-bottom: calc(var(--r) * 0.09 + 0.5px) dotted #1c1c1e;
    border-radius: 0 0 50% 50%;
  }
  .btn1,
  .btn2 {
    left: 50%;
    margin-left: calc(var(--r) * -0.1);
  }
  .btn1 {
    top: 32%;
  }
  .btn2 {
    top: 56%;
  }
  .arm {
    position: absolute;
    top: 38%;
    width: calc(var(--r) * 1.1);
    height: calc(var(--r) * 0.08 + 1px);
    background: #6b4a2e;
    border-radius: 2px;
  }
  .arm.l {
    right: 88%;
    transform: rotate(18deg);
    transform-origin: 100% 50%;
  }
  .arm.r {
    left: 88%;
    transform: rotate(-24deg);
    transform-origin: 0 50%;
  }
  .ball.spark::after {
    content: '';
    position: absolute;
    inset: -60%;
    border-radius: 50%;
    pointer-events: none;
    background:
      radial-gradient(circle at 20% 30%, #fff 0 2px, transparent 3px),
      radial-gradient(circle at 80% 25%, #fff 0 2px, transparent 3px),
      radial-gradient(circle at 70% 80%, #fff 0 1.5px, transparent 2.5px),
      radial-gradient(circle at 25% 75%, #fff 0 1.5px, transparent 2.5px),
      radial-gradient(circle at 50% 10%, #ffd60a 0 2px, transparent 3px);
    animation: spark 1.2s ease-out forwards;
  }
  @keyframes spark {
    from {
      opacity: 1;
      transform: scale(0.6);
    }
    to {
      opacity: 0;
      transform: scale(1.3);
    }
  }
</style>
