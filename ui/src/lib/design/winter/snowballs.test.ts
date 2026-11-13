// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  admit,
  ClickTracker,
  CLICK_MAX_MS,
  flightFrames,
  flightMs,
  hasOwnText,
  hitTest,
  launchPoint,
  markClip,
  MAX_BALLS,
  splatPuffs,
  throwCounter,
} from './snowballs';

/** The stack elementsFromPoint would give for the element: it and its ancestors. */
function stackAt(el: Element): Element[] {
  const out: Element[] = [];
  for (let n: Element | null = el; n; n = n.parentElement) out.push(n);
  return out;
}

function page(html: string) {
  document.body.innerHTML = `<div id="app"><div class="app"><nav class="side"><a href="#/">Главная</a><div class="gap"></div></nav><main class="main"><div class="page wx">${html}</div></main><div class="bar"><span class="room"></span></div></div></div>`;
  return (sel: string) => document.querySelector(sel)!;
}

afterEach(() => {
  document.body.innerHTML = '';
});

describe('hit-test for empty space', () => {
  it('the page background is empty', () => {
    const $ = page('<div class="grid"><div class="spacer"></div></div>');
    expect(hitTest(stackAt($('.page'))).kind).toBe('background');
    expect(hitTest(stackAt($('.spacer'))).kind).toBe('background');
    expect(hitTest([document.body, document.documentElement]).kind).toBe('background');
  });

  it('controls, links and text are never empty', () => {
    const $ = page(
      '<button class="b"><span class="in"></span></button><a class="l" href="#x">x</a><p class="t">Текст</p><div class="sw" role="switch"></div><input class="i" /><div class="tab" tabindex="0"></div>'
    );
    for (const sel of ['.b', '.in', '.l', '.t', '.sw', '.i', '.tab']) expect(hitTest(stackAt($(sel))).kind, sel).toBe('blocked');
  });

  it('the build tile, the sidebar and the launch bar are off limits', () => {
    const $ = page('<section class="pair"><div class="pane open"><div class="pane-body"><div class="pane-top"></div></div></div></section>');
    expect(hitTest(stackAt($('.pane-top'))).kind).toBe('blocked');
    expect(hitTest(stackAt($('.pair'))).kind).toBe('blocked');
    expect(hitTest(stackAt($('.side .gap'))).kind).toBe('blocked');
    expect(hitTest(stackAt($('.bar .room'))).kind).toBe('blocked');
  });

  it('pictures are content', () => {
    const $ = page('<img class="im" alt="" /><svg class="ic"><path class="p" d="M0 0"/></svg>');
    expect(hitTest(stackAt($('.im'))).kind).toBe('blocked');
    expect(hitTest(stackAt($('.p'))).kind).toBe('blocked');
  });

  it('a card surface or a bare wrapper inside it is a splat on the card', () => {
    const $ = page('<div class="panel"><div class="row"><b>Заголовок</b></div></div>');
    const hit = hitTest(stackAt($('.panel')));
    expect(hit.kind).toBe('card');
    expect(hit.kind === 'card' && hit.card).toBe($('.panel'));
    expect(hitTest(stackAt($('.row'))).kind).toBe('card');
    expect(hitTest(stackAt($('.panel b'))).kind).toBe('blocked');
  });

  it('a card with its own text is content', () => {
    const $ = page('<div class="tile">Только текст</div>');
    expect(hitTest(stackAt($('.tile'))).kind).toBe('blocked');
  });

  it('nothing is thrown while a modal is open', () => {
    const $ = page('<div class="spacer"></div><div aria-modal="true" class="sheet"></div>');
    expect(hitTest(stackAt($('.spacer'))).kind).toBe('blocked');
  });

  it('opt-out marker and an empty stack block the throw', () => {
    const $ = page('<div data-no-snowball><div class="x"></div></div>');
    expect(hitTest(stackAt($('.x'))).kind).toBe('blocked');
    expect(hitTest([]).kind).toBe('blocked');
  });

  it('own text means a direct, non-blank text node', () => {
    const d = document.createElement('div');
    d.innerHTML = '  <span>x</span>  ';
    expect(hasOwnText(d)).toBe(false);
    d.append('слово');
    expect(hasOwnText(d)).toBe(true);
  });
});

describe('click detection', () => {
  const ev = (x: number, y: number, button = 0) => ({ clientX: x, clientY: y, button, pointerId: 1 });

  it('a press and release in place is a click', () => {
    const c = new ClickTracker();
    c.press(ev(10, 10), 0);
    expect(c.release(ev(12, 11), 100)).toEqual({ x: 12, y: 11 });
  });

  it('a drag, a long press, a right click or a cancel is not', () => {
    const c = new ClickTracker();
    c.press(ev(10, 10), 0);
    c.move(ev(30, 10));
    expect(c.release(ev(10, 10), 50)).toBeNull();
    c.press(ev(10, 10), 0);
    expect(c.release(ev(10, 10), CLICK_MAX_MS + 1)).toBeNull();
    c.press(ev(10, 10, 2), 0);
    expect(c.release(ev(10, 10, 2), 10)).toBeNull();
    c.press(ev(10, 10), 0);
    c.cancel();
    expect(c.release(ev(10, 10), 10)).toBeNull();
    expect(c.release(ev(10, 10), 10)).toBeNull();
  });
});

describe('flight and splat', () => {
  it('starts at the bottom-centre below the window edge', () => {
    expect(launchPoint(1000, 700)).toEqual({ x: 500, y: 724 });
  });

  it('flies in an arc from the start to the point, shrinking', () => {
    const from = { x: 500, y: 724 };
    const to = { x: 200, y: 200 };
    const f = flightFrames(from, to);
    expect(f[0]).toMatchObject({ x: 500, y: 724, offset: 0 });
    expect(f.at(-1)).toMatchObject({ x: 200, y: 200, offset: 1 });
    const mid = f[6];
    expect(mid.y).toBeLessThan((from.y + to.y) / 2);
    expect(f.at(-1)!.scale).toBeLessThan(f[0].scale);
  });

  it('takes a short, bounded time', () => {
    expect(flightMs({ x: 0, y: 0 }, { x: 0, y: 0 })).toBe(320);
    expect(flightMs({ x: 0, y: 0 }, { x: 5000, y: 0 })).toBe(720);
  });

  it('bursts into small bits around the point', () => {
    const p = splatPuffs(8, () => 0.5);
    expect(p).toHaveLength(8);
    for (const b of p) {
      expect(Math.hypot(b.dx, b.dy - 6)).toBeGreaterThan(10);
      expect(b.r).toBeGreaterThan(1);
    }
  });

  it('a mark is clipped to the card edges', () => {
    const card = { left: 100, top: 100, right: 300, bottom: 200 };
    expect(markClip({ x: 200, y: 150 }, 40, card)).toBe('inset(0.0px 0.0px 0.0px 0.0px)');
    expect(markClip({ x: 105, y: 195 }, 40, card)).toBe('inset(0.0px 0.0px 15.0px 15.0px)');
    expect(markClip({ x: 105, y: 195 }, 40, card, 12)).toBe('inset(0.0px 0.0px 15.0px 15.0px round 12.0px)');
  });

  it('keeps at most six balls', () => {
    let list: number[] = [];
    for (let i = 0; i < 10; i++) list = admit(list, i);
    expect(MAX_BALLS).toBe(6);
    expect(list).toEqual([4, 5, 6, 7, 8, 9]);
  });
});

describe('throw counter', () => {
  it('reports throws in one batch after a pause', () => {
    vi.useFakeTimers();
    const flush = vi.fn();
    const c = throwCounter(flush, 1000);
    c.add();
    c.add();
    c.add();
    expect(flush).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1000);
    expect(flush).toHaveBeenCalledWith(3);
    c.add();
    c.flush();
    expect(flush).toHaveBeenLastCalledWith(1);
    c.flush();
    expect(flush).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });
});
