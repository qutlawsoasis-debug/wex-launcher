/**
 * Snowball throwing (winter look): the hit-test for "empty space", the click
 * detector, the flight path and the splat. Snowballs.svelte draws them.
 * Apple has no guidance on seasonal decoration. HIG › Motion: decorative
 * motion is optional, never needed to use the app and off under Reduce Motion;
 * the throw never takes a click away from a control (it only listens).
 */
import { call } from '../../shell/ipc';

/** At most this many balls in the air at once. */
export const MAX_BALLS = 6;
/** A press that moves farther than this is a drag, not a click, px. */
export const CLICK_SLOP = 5;
/** A press held longer than this is not a throw, ms. */
export const CLICK_MAX_MS = 600;
/** The snow mark on a card melts away after this long, ms. */
export const MARK_MS = 4200;

/** Controls and anything that acts on a click. */
const INTERACTIVE = [
  'a[href]',
  'button',
  'input',
  'select',
  'textarea',
  'label',
  'summary',
  'details',
  'option',
  'video',
  'audio',
  'iframe',
  '[contenteditable]:not([contenteditable="false"])',
  '[role="button"]',
  '[role="link"]',
  '[role="switch"]',
  '[role="checkbox"]',
  '[role="radio"]',
  '[role="tab"]',
  '[role="slider"]',
  '[role="menuitem"]',
  '[role="option"]',
  '[role="textbox"]',
  '[role="combobox"]',
  '[role="searchbox"]',
  '[role="spinbutton"]',
  '[role="dialog"]',
  '[role="alertdialog"]',
  '[tabindex]:not([tabindex="-1"])',
  '[draggable="true"]',
  '[data-no-snowball]',
].join(',');

/** Glass cards a ball splats against (the frosted surfaces of winter.css and the open build tile). */
export const CARD_SELECTOR = ['.pane', '.panel', '.tile', '.list', '.form', '.fact', '.preset', '.theme-card'].join(',');

export type Hit = { kind: 'blocked' } | { kind: 'background' } | { kind: 'card'; card: Element };

/** True when the element itself shows text (a direct, non-blank text node). */
export function hasOwnText(el: Element): boolean {
  for (const n of Array.from(el.childNodes)) {
    if (n.nodeType === 3 && (n.textContent ?? '').trim() !== '') return true;
  }
  return false;
}

/**
 * What a click at a point lands on, given the elements under it from the top
 * (document.elementsFromPoint). Only the element the click actually reaches
 * matters: if it is (or sits inside) a control, the snowman toy or an opt-out,
 * or a modal is open, nothing is thrown. Text, pictures, the winter scene, the
 * sidebar and launch bar are all fair game, since a click there does nothing
 * else. Over a glass card (or the open build tile) the ball splats on it.
 */
export function hitTest(stack: readonly Element[], doc: Document | null = typeof document !== 'undefined' ? document : null): Hit {
  if (doc?.querySelector('dialog[open], [aria-modal="true"]')) return { kind: 'blocked' };
  const top = stack[0];
  if (!top) return { kind: 'blocked' };
  if (top.closest(INTERACTIVE)) return { kind: 'blocked' };
  const card = top.closest(CARD_SELECTOR);
  return card ? { kind: 'card', card } : { kind: 'background' };
}

/** Tells a click (press and release in place) from a drag or a long press. */
export class ClickTracker {
  private down: { x: number; y: number; t: number; id: number } | null = null;

  press(e: { clientX: number; clientY: number; button: number; pointerId?: number; isPrimary?: boolean }, t: number): void {
    this.down = e.button === 0 && e.isPrimary !== false ? { x: e.clientX, y: e.clientY, t, id: e.pointerId ?? 0 } : null;
  }

  move(e: { clientX: number; clientY: number }): void {
    if (this.down && Math.hypot(e.clientX - this.down.x, e.clientY - this.down.y) > CLICK_SLOP) this.down = null;
  }

  cancel(): void {
    this.down = null;
  }

  /** The click point, or null when this release does not end a click. */
  release(e: { clientX: number; clientY: number; button: number; pointerId?: number }, t: number): { x: number; y: number } | null {
    const d = this.down;
    this.down = null;
    if (!d || e.button !== 0 || (e.pointerId ?? 0) !== d.id) return null;
    if (t - d.t > CLICK_MAX_MS) return null;
    if (Math.hypot(e.clientX - d.x, e.clientY - d.y) > CLICK_SLOP) return null;
    return { x: e.clientX, y: e.clientY };
  }
}

export interface Point {
  x: number;
  y: number;
}

/** The ball leaves from the bottom-centre of the window, just below the edge. */
export function launchPoint(w: number, h: number): Point {
  return { x: w / 2, y: h + 24 };
}

/** Flight time, ms: farther throws take a little longer. */
export function flightMs(from: Point, to: Point): number {
  const d = Math.hypot(to.x - from.x, to.y - from.y);
  return Math.round(Math.max(320, Math.min(720, 260 + d * 0.45)));
}

/**
 * Keyframes of the throw: a straight line in x, an arc in y (it rises above the
 * line by a share of the distance) and a shrinking size (it flies away from the
 * viewer). Positions are the ball's centre.
 */
export function flightFrames(from: Point, to: Point, steps = 12): Array<{ x: number; y: number; scale: number; offset: number }> {
  const d = Math.hypot(to.x - from.x, to.y - from.y);
  const lift = Math.min(160, d * 0.28);
  const frames = [];
  for (let i = 0; i <= steps; i++) {
    const k = i / steps;
    frames.push({
      x: from.x + (to.x - from.x) * k,
      y: from.y + (to.y - from.y) * k - lift * 4 * k * (1 - k),
      scale: 1.7 - 0.9 * k,
      offset: k,
    });
  }
  return frames;
}

/** Bits of snow flying out of a splat. */
export interface Puff {
  dx: number;
  dy: number;
  r: number;
  delay: number;
}

export function splatPuffs(n: number, random: () => number = Math.random): Puff[] {
  return Array.from({ length: n }, (_, i) => {
    const a = (i / n) * Math.PI * 2 + (random() - 0.5) * 0.7;
    const dist = 14 + random() * 22;
    return {
      dx: Math.cos(a) * dist,
      // Bits fall a little: gravity pulls the burst down.
      dy: Math.sin(a) * dist + 6,
      r: 1.6 + random() * 2.4,
      delay: random() * 40,
    };
  });
}

/**
 * Where a snow mark sits on a card, clipped to the card's edges (it is drawn in
 * a fixed overlay, so the clip keeps it on the glass).
 */
export function markClip(at: Point, size: number, card: { left: number; top: number; right: number; bottom: number }, radius = 0): string {
  const half = size / 2;
  const top = Math.max(0, card.top - (at.y - half));
  const left = Math.max(0, card.left - (at.x - half));
  const right = Math.max(0, at.x + half - card.right);
  const bottom = Math.max(0, at.y + half - card.bottom);
  // The card's corner radius keeps the mark off its rounded corners (the mark is round anyway).
  const round = radius > 0 ? ` round ${radius.toFixed(1)}px` : '';
  return `inset(${top.toFixed(1)}px ${right.toFixed(1)}px ${bottom.toFixed(1)}px ${left.toFixed(1)}px${round})`;
}

/** Adds an item, dropping the oldest ones past the limit. */
export function admit<T>(list: readonly T[], item: T, max = MAX_BALLS): T[] {
  const next = [...list, item];
  return next.length > max ? next.slice(next.length - max) : next;
}

// --- Winter achievements (defined in core: wex-social achievements.rs) -------

export type WinterAchievement = 'first_snowman' | 'snowman_town' | 'saw_santa' | 'snowball_fight';

/** Reports progress; outside the app (tests, plain browser) the call just fails quietly. */
export function reportWinter(id: WinterAchievement, increment = 1): void {
  try {
    void call('social_report_achievement_progress', { achievementId: id, increment }).catch(() => {});
  } catch {
    // No backend: nothing to report to.
  }
}

/**
 * Batches throws: a fight is many clicks, so the count goes to core once the
 * throwing pauses (and on teardown), not on every ball.
 */
export function throwCounter(
  flush: (n: number) => void,
  delay = 1500,
  // Wrapped: a bare setTimeout called as a method throws «Illegal invocation» in browsers.
  timers = { set: (fn: () => void, ms: number) => setTimeout(fn, ms), clear: (id: ReturnType<typeof setTimeout> | undefined) => clearTimeout(id) }
) {
  let pending = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const send = () => {
    timers.clear(timer);
    timer = undefined;
    if (pending > 0) flush(pending);
    pending = 0;
  };
  return {
    add() {
      pending += 1;
      timers.clear(timer);
      timer = timers.set(send, delay);
    },
    flush: send,
    get pending() {
      return pending;
    },
  };
}
