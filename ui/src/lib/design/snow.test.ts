import { describe, expect, it } from 'vitest';
import { flakeCount, makeFlake, stepFlake } from './snow';
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

  it('light snow: density follows the window, capped', () => {
    expect(flakeCount(1080, 720)).toBe(49);
    expect(flakeCount(3840, 2160)).toBe(120);
    expect(flakeCount(0, 0)).toBe(0);
  });

  it('flakes fall at their speed whatever the frame rate, and come back at the top', () => {
    let i = 0;
    const rnd = () => [0.5, 0.5, 0.5, 0.5, 0.5][i++ % 5];
    const a = makeFlake(800, 600, rnd, true);
    const b = { ...a };
    stepFlake(a, 1, 0, 800, 600, rnd);
    for (let k = 0; k < 60; k++) stepFlake(b, 1 / 60, 0, 800, 600, rnd);
    expect(b.y).toBeCloseTo(a.y, 3);
    a.y = 700;
    stepFlake(a, 0.01, 0, 800, 600, rnd);
    expect(a.y).toBeLessThan(0);
  });
});
