import { describe, expect, it } from 'vitest';
import { CURSOR_R, flakeCount, layerCounts, makeFlake, stepFlake } from './snow';
import { isWinterSeason, season } from './season.svelte';

describe('winter look', () => {
  it('is the season from November to February only', () => {
    const on = [10, 11, 0, 1].map((m) => isWinterSeason(new Date(2026, m, 15)));
    const off = [2, 3, 4, 5, 6, 7, 8, 9].map((m) => isWinterSeason(new Date(2026, m, 15)));
    expect(on.every(Boolean)).toBe(true);
    expect(off.some(Boolean)).toBe(false);
  });

  it('the switch turns it off even in winter', () => {
    season.now = () => new Date(2026, 11, 31);
    season.setEnabled(true);
    expect(season.winter).toBe(true);
    season.setEnabled(false);
    expect(season.winter).toBe(false);
    season.setEnabled(true);
    season.now = () => new Date(2026, 6, 1);
    expect(season.winter).toBe(false);
    season.now = () => new Date();
  });

  it('light snow in three depths: density follows the window, capped; near flakes stay rare', () => {
    expect(flakeCount(1080, 720)).toBe(49);
    expect(flakeCount(3840, 2160)).toBe(140);
    expect(flakeCount(0, 0)).toBe(0);
    const [far, mid, near] = layerCounts(1920, 1080);
    expect(near).toBeLessThanOrEqual(7);
    expect(far).toBeGreaterThan(near);
    expect(mid).toBeGreaterThan(near);
  });

  const fixed = () => 0.5;

  it('flakes fall at their speed whatever the frame rate, and come back at the top', () => {
    const a = makeFlake(800, 600, fixed, true, 1);
    const b = { ...a };
    stepFlake(a, 1, 0, 800, 600, fixed);
    for (let k = 0; k < 60; k++) stepFlake(b, 1 / 60, 0, 800, 600, fixed);
    expect(b.y).toBeCloseTo(a.y, 3);
    a.y = 700;
    stepFlake(a, 0.01, 0, 800, 600, fixed);
    expect(a.y).toBeLessThan(0);
    expect(a.layer).toBe(1);
  });

  it('wind blows snow sideways, near flakes most; it wraps around the sides', () => {
    const far = makeFlake(800, 600, fixed, true, 0);
    const near = makeFlake(800, 600, fixed, true, 2);
    const [fx, nx] = [far.x, near.x];
    for (let k = 0; k < 30; k++) {
      stepFlake(far, 1 / 30, 0, 800, 600, fixed, 200);
      stepFlake(near, 1 / 30, 0, 800, 600, fixed, 200);
    }
    expect(far.x - fx).toBeGreaterThan(0);
    expect(near.x - nx).toBeGreaterThan(far.x - fx);
    near.x = 900;
    stepFlake(near, 0.001, 0, 800, 600, fixed);
    expect(near.x).toBeLessThan(0);
  });

  it("flakes drift out of the cursor's way, not beyond its circle", () => {
    const f = makeFlake(800, 600, fixed, true, 1);
    f.x = 400;
    f.y = 300;
    stepFlake(f, 0.016, 0, 800, 600, fixed, 0, { x: 390, y: 300 });
    const g = { ...makeFlake(800, 600, fixed, true, 1), x: 400, y: 300 };
    stepFlake(g, 0.016, 0, 800, 600, fixed, 0, null);
    expect(f.x).toBeGreaterThan(g.x);
    const h = { ...makeFlake(800, 600, fixed, true, 1), x: 400, y: 300 };
    stepFlake(h, 0.016, 0, 800, 600, fixed, 0, { x: 400 - CURSOR_R - 5, y: 300 });
    expect(h.x).toBeCloseTo(g.x, 6);
  });
});
