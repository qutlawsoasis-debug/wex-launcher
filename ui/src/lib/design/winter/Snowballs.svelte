<!--
  Snowball throwing (winter look). A click on empty space (the window or page
  background, never a control, text, link, picture or the build tile) throws a
  snowball from the bottom-centre of the window to the point, where it splats
  with a little burst of snow. Over a glass card it splats against the glass and
  leaves a soft mark that melts away in a few seconds.
  Apple has no guidance on seasonal decoration. The rules applied are HIG ›
  Motion: decorative motion is optional, never needed to use the app, and stays
  off under Reduce Motion (App.svelte leaves it out then; the handler checks
  again). HIG › Playing audio: the crunch and puff (sounds.ts) are optional,
  quiet and only while «Звуки зимы» is on; the throw is always visible.
  The overlay never takes pointer input (pointer-events: none) and the window
  listener never calls preventDefault, so every click still does what it did.
  At most MAX_BALLS balls fly at once; flights are Web Animations, no frame loop.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '../theme.svelte';
  import { playWinter } from './sounds';
  import {
    admit,
    ClickTracker,
    flightFrames,
    flightMs,
    hitTest,
    launchPoint,
    markClip,
    MARK_MS,
    MAX_BALLS,
    reportWinter,
    splatPuffs,
    throwCounter,
    type Point,
    type Puff,
  } from './snowballs';

  interface Flying {
    id: number;
    from: Point;
    to: Point;
    dur: number;
    card: Element | null;
  }
  interface Splat {
    id: number;
    x: number;
    y: number;
    puffs: Puff[];
    onCard: boolean;
  }
  interface Mark {
    id: number;
    x: number;
    y: number;
    size: number;
    rot: number;
    clip: string;
  }

  const R = 9;
  let balls = $state<Flying[]>([]);
  let splats = $state<Splat[]>([]);
  let marks = $state<Mark[]>([]);
  let nextId = 1;

  const counter = throwCounter((n) => reportWinter('snowball_fight', n));

  function throwAt(to: Point, card: Element | null) {
    const from = launchPoint(window.innerWidth, window.innerHeight);
    const b: Flying = { id: nextId++, from, to, dur: flightMs(from, to), card };
    balls = admit(balls, b, MAX_BALLS);
    counter.add();
    playWinter('crunch');
  }

  function land(b: Flying) {
    if (!balls.some((x) => x.id === b.id)) return;
    balls = balls.filter((x) => x.id !== b.id);
    const onCard = !!b.card?.isConnected;
    const s: Splat = { id: b.id, x: b.to.x, y: b.to.y, puffs: splatPuffs(onCard ? 9 : 7), onCard };
    splats = admit(splats, s, MAX_BALLS);
    setTimeout(() => (splats = splats.filter((x) => x.id !== s.id)), 700);
    playWinter('puff');
    if (onCard) {
      const size = 38 + Math.random() * 16;
      const m: Mark = { id: b.id, x: b.to.x, y: b.to.y, size, rot: Math.random() * 360, clip: markClip(b.to, size, b.card!.getBoundingClientRect(), parseFloat(getComputedStyle(b.card!).borderTopLeftRadius) || 0) };
      marks = admit(marks, m, MAX_BALLS * 2);
      setTimeout(() => (marks = marks.filter((x) => x.id !== m.id)), MARK_MS + 100);
    }
  }

  /** The flight: an arc sampled into keyframes (snowballs.ts › flightFrames). */
  function fly(node: HTMLElement, b: Flying) {
    const frames = flightFrames(b.from, b.to).map((f) => ({
      transform: `translate(${(f.x - R).toFixed(1)}px, ${(f.y - R).toFixed(1)}px) scale(${f.scale.toFixed(3)})`,
      offset: f.offset,
    }));
    let timer: ReturnType<typeof setTimeout> | undefined;
    let anim: Animation | undefined;
    if (typeof node.animate === 'function') {
      anim = node.animate(frames, { duration: b.dur, easing: 'cubic-bezier(0.25, 0.1, 0.5, 1)', fill: 'forwards' });
      anim.onfinish = () => land(b);
    } else {
      node.style.transform = frames.at(-1)!.transform;
      timer = setTimeout(() => land(b), b.dur);
    }
    return {
      destroy() {
        clearTimeout(timer);
        anim?.cancel();
      },
    };
  }

  onMount(() => {
    const tracker = new ClickTracker();
    const idle = () => document.documentElement.classList.contains('app-idle');
    const onDown = (e: PointerEvent) => tracker.press(e, e.timeStamp);
    const onMove = (e: PointerEvent) => tracker.move(e);
    const onCancel = () => tracker.cancel();
    const onUp = (e: PointerEvent) => {
      const p = tracker.release(e, e.timeStamp);
      if (!p || theme.motionReduced || idle() || e.pointerType === 'touch') return;
      // A click that ends a text selection selects; it does not throw.
      const sel = window.getSelection?.();
      if (sel && !sel.isCollapsed) return;
      const hit = hitTest(document.elementsFromPoint(p.x, p.y));
      if (hit.kind === 'blocked') return;
      throwAt(p, hit.kind === 'card' ? hit.card : null);
    };
    const opts = { capture: true, passive: true } as const;
    window.addEventListener('pointerdown', onDown, opts);
    window.addEventListener('pointermove', onMove, opts);
    window.addEventListener('pointerup', onUp, opts);
    window.addEventListener('pointercancel', onCancel, opts);
    window.addEventListener('blur', onCancel);
    return () => {
      window.removeEventListener('pointerdown', onDown, opts);
      window.removeEventListener('pointermove', onMove, opts);
      window.removeEventListener('pointerup', onUp, opts);
      window.removeEventListener('pointercancel', onCancel, opts);
      window.removeEventListener('blur', onCancel);
      counter.flush();
    };
  });
</script>

<div class="snowballs" aria-hidden="true">
  {#each marks as m (m.id)}
    <span class="mark-clip" style:left="{m.x - m.size / 2}px" style:top="{m.y - m.size / 2}px" style:width="{m.size}px" style:height="{m.size}px" style:clip-path={m.clip}>
      <i class="mark" style:transform="rotate({m.rot.toFixed(0)}deg)" style:animation-duration="{MARK_MS}ms"></i>
    </span>
  {/each}
  {#each balls as b (b.id)}
    <span class="ball" style:width="{R * 2}px" style:height="{R * 2}px" use:fly={b}></span>
  {/each}
  {#each splats as s (s.id)}
    <span class="splat" class:card={s.onCard} style:left="{s.x}px" style:top="{s.y}px">
      <i class="core"></i>
      {#each s.puffs as p, i (i)}
        <i class="bit" style:width="{p.r * 2}px" style:height="{p.r * 2}px" style:--dx="{p.dx.toFixed(1)}px" style:--dy="{p.dy.toFixed(1)}px" style:animation-delay="{p.delay.toFixed(0)}ms"></i>
      {/each}
    </span>
  {/each}
</div>

<style>
  .snowballs {
    position: fixed;
    inset: 0;
    z-index: 38;
    overflow: hidden;
    pointer-events: none;
  }
  /* Packed snow, lit from the upper left like the snowmen (SnowPlay). */
  .ball {
    position: absolute;
    left: 0;
    top: 0;
    border-radius: 50%;
    background: radial-gradient(circle at 34% 30%, #ffffff 0, #f4f8fd 40%, #d6e2f1 78%, #b9c9de 100%);
    box-shadow: 0 2px 4px rgb(0 0 0 / 0.25);
    will-change: transform;
  }
  .splat {
    position: absolute;
    width: 0;
    height: 0;
  }
  .core {
    position: absolute;
    left: -12px;
    top: -9px;
    width: 24px;
    height: 18px;
    border-radius: 50%;
    background: radial-gradient(ellipse at 50% 45%, #fff 0 40%, rgb(255 255 255 / 0.6) 60%, transparent 72%);
    animation: core 0.5s ease-out forwards;
  }
  @keyframes core {
    from {
      transform: scale(0.5);
      opacity: 1;
    }
    to {
      transform: scale(1.6, 1.2);
      opacity: 0;
    }
  }
  .bit {
    position: absolute;
    left: 0;
    top: 0;
    margin: -2px 0 0 -2px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 2px rgb(255 255 255 / 0.6);
    opacity: 0;
    animation: bit 0.55s cubic-bezier(0.2, 0.6, 0.4, 1) forwards;
  }
  @keyframes bit {
    0% {
      transform: translate(0, 0) scale(1);
      opacity: 1;
    }
    100% {
      transform: translate(var(--dx), var(--dy)) scale(0.4);
      opacity: 0;
    }
  }
  /* The mark: a soft, uneven patch of snow on the glass that melts and slides a little. */
  .mark-clip {
    position: absolute;
  }
  .mark {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(circle at 50% 50%, rgb(255 255 255 / 0.92) 0 22%, rgb(255 255 255 / 0.55) 34%, transparent 50%),
      radial-gradient(circle at 26% 38%, rgb(255 255 255 / 0.75) 0 9%, transparent 15%),
      radial-gradient(circle at 72% 30%, rgb(255 255 255 / 0.7) 0 8%, transparent 14%),
      radial-gradient(circle at 66% 74%, rgb(255 255 255 / 0.7) 0 7%, transparent 13%),
      radial-gradient(circle at 30% 72%, rgb(255 255 255 / 0.6) 0 6%, transparent 12%),
      radial-gradient(circle at 86% 56%, rgb(255 255 255 / 0.55) 0 4%, transparent 9%),
      radial-gradient(circle at 12% 56%, rgb(255 255 255 / 0.5) 0 4%, transparent 9%);
    filter: drop-shadow(0 1px 1.5px rgb(40 60 100 / 0.25));
    animation-name: melt;
    animation-timing-function: ease-in;
    animation-fill-mode: forwards;
  }
  @keyframes melt {
    0% {
      opacity: 0;
      scale: 0.6;
    }
    6% {
      opacity: 1;
      scale: 1;
    }
    55% {
      opacity: 0.9;
      translate: 0 0;
    }
    100% {
      opacity: 0;
      scale: 0.85 0.95;
      translate: 0 5px;
    }
  }
  /* Nothing moves while the launcher idles. */
  :global(html.app-idle) .snowballs * {
    animation-play-state: paused;
  }
</style>
