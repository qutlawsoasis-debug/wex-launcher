/**
 * Smoothed pointer parallax for decorative layers (the big version digits on Home).
 *
 * The pointer is tracked over a whole area, not just the layer: the target is the
 * pointer's offset from the area's centre, in [-0.5, 0.5] on each axis. The drawn
 * value follows the target with a frame-rate independent lerp, so it never jumps at
 * an edge, and returns to the centre the same way when the pointer leaves the area
 * or the window.
 *
 * HIG › Motion: motion is optional and brief; under Reduce Motion the layer stays put.
 */
import { createManagedAnimation } from './index';
import { motionReduced } from '../design/wex-tokens';

export interface Point {
  x: number;
  y: number;
}

/** Time constant of the follow, ms: about 95% of the way in 3τ ≈ 270 ms. */
export const PARALLAX_TAU_MS = 90;
/** Below this the value counts as settled and the frame loop stops. */
const SETTLE = 0.0005;

const clampHalf = (v: number) => Math.max(-0.5, Math.min(0.5, v));

/** The pointer's offset from the rect's centre, as a fraction of its size, clamped to ±0.5. */
export function offsetIn(rect: { left: number; top: number; width: number; height: number }, x: number, y: number): Point {
  if (rect.width <= 0 || rect.height <= 0) return { x: 0, y: 0 };
  return {
    x: clampHalf((x - rect.left) / rect.width - 0.5),
    y: clampHalf((y - rect.top) / rect.height - 0.5),
  };
}

/** One lerp step of `dt` ms towards the target; independent of the frame rate. */
export function follow(current: Point, target: Point, dt: number, tau = PARALLAX_TAU_MS): Point {
  const k = 1 - Math.exp(-Math.max(0, dt) / tau);
  return { x: current.x + (target.x - current.x) * k, y: current.y + (target.y - current.y) * k };
}

export const settled = (a: Point, b: Point) => Math.abs(a.x - b.x) < SETTLE && Math.abs(a.y - b.y) < SETTLE;

export interface ParallaxOptions {
  /** The element whose box the pointer is tracked over. Defaults to the node. */
  area?: () => Element | null;
}

/**
 * Svelte action: writes `--px` / `--py` (each in [-0.5, 0.5]) on the node, smoothed.
 * Descendants use them in a transform.
 */
export function parallax(node: HTMLElement, options: ParallaxOptions = {}) {
  let area = options.area;
  let current: Point = { x: 0, y: 0 };
  let target: Point = { x: 0, y: 0 };

  const write = () => {
    node.style.setProperty('--px', current.x.toFixed(4));
    node.style.setProperty('--py', current.y.toFixed(4));
  };

  const loop = createManagedAnimation((dt) => {
    current = follow(current, target, dt);
    if (settled(current, target)) {
      current = { ...target };
      loop.stop();
    }
    write();
  });

  function aim(next: Point) {
    if (motionReduced()) {
      loop.stop();
      target = current = { x: 0, y: 0 };
      write();
      return;
    }
    target = next;
    if (!settled(current, target)) loop.start();
  }

  function onMove(e: PointerEvent) {
    if (e.pointerType === 'touch') return;
    const el = area?.() ?? node;
    const r = el.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    aim(inside ? offsetIn(r, e.clientX, e.clientY) : { x: 0, y: 0 });
  }
  const home = () => aim({ x: 0, y: 0 });
  /** The pointer left the window: `relatedTarget` is null only then. */
  const onOut = (e: PointerEvent) => {
    if (!e.relatedTarget) home();
  };

  window.addEventListener('pointermove', onMove, { passive: true });
  document.addEventListener('pointerout', onOut);
  window.addEventListener('blur', home);
  write();

  return {
    update(next: ParallaxOptions = {}) {
      area = next.area;
    },
    destroy() {
      loop.stop();
      window.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerout', onOut);
      window.removeEventListener('blur', home);
    },
  };
}
