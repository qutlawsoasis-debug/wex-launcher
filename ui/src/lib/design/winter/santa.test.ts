import { describe, expect, it } from 'vitest';
import { arcPoint, firstDelay, makePass, makeSparkles, nextDelay, rng, starPath } from './santa';

const draws = (n: number, seed: number) => {
  const r = rng(seed);
  return Array.from({ length: n }, () => r);
};

describe('santa scheduling', () => {
  it('the generator is deterministic and stays in [0, 1)', () => {
    const a = rng(3);
    const b = rng(3);
    const xs = Array.from({ length: 300 }, () => a());
    expect(xs).toEqual(Array.from({ length: 300 }, () => b()));
    expect(xs.every((x) => x >= 0 && x < 1)).toBe(true);
  });

  it('the first pass comes 8–15 s after start', () => {
    expect(firstDelay(() => 0)).toBe(8_000);
    expect(firstDelay(() => 0.999999)).toBe(15_000);
    for (const r of draws(200, 1)) {
      const d = firstDelay(r);
      expect(d).toBeGreaterThanOrEqual(8_000);
      expect(d).toBeLessThanOrEqual(15_000);
    }
  });

  it('the gap between passes is 45–110 s', () => {
    expect(nextDelay(() => 0)).toBe(45_000);
    expect(nextDelay(() => 0.999999)).toBe(110_000);
    for (const r of draws(200, 2)) {
      const d = nextDelay(r);
      expect(d).toBeGreaterThanOrEqual(45_000);
      expect(d).toBeLessThanOrEqual(110_000);
    }
  });

  it('a pass takes 9–12 s, stays in the upper part and goes both ways', () => {
    const r = rng(9);
    const dirs = new Set<string>();
    const tops = new Set<number>();
    for (let i = 0; i < 200; i++) {
      const p = makePass(r);
      dirs.add(p.dir);
      tops.add(Math.round(p.top * 100));
      expect(p.duration).toBeGreaterThanOrEqual(9_000);
      expect(p.duration).toBeLessThanOrEqual(12_000);
      expect(p.top - p.lift).toBeGreaterThan(0);
      expect(p.top).toBeLessThan(0.3);
    }
    expect(dirs).toEqual(new Set(['ltr', 'rtl']));
    expect(tops.size).toBeGreaterThan(5);
  });

  it('the arc starts and ends off-screen and peaks in the middle', () => {
    const p = { dir: 'ltr' as const, duration: 10_000, top: 0.2, lift: 0.08, tilt: 3 };
    expect(arcPoint(p, 0).x).toBeLessThan(0);
    expect(arcPoint(p, 1).x).toBeGreaterThan(1);
    expect(arcPoint(p, 0.5).y).toBeCloseTo(0.12);
    expect(arcPoint(p, 0).y).toBeCloseTo(0.2);
    expect(arcPoint(p, 0.25).y).toBeGreaterThan(arcPoint(p, 0.5).y);
    const back = { ...p, dir: 'rtl' as const };
    expect(arcPoint(back, 0).x).toBeGreaterThan(1);
    expect(arcPoint(back, 1).x).toBeLessThan(0);
    expect(arcPoint(back, 0.3).y).toBeCloseTo(arcPoint(p, 0.3).y);
  });
});

describe('santa trail', () => {
  it('sparkles are deterministic, shed behind the sleigh and drift backwards', () => {
    const a = makeSparkles(30);
    expect(a).toEqual(makeSparkles(30));
    expect(a).toHaveLength(30);
    for (const s of a) {
      expect(s.dx).toBeLessThan(0);
      expect(s.delay).toBeLessThanOrEqual(0);
      expect(s.dur).toBeGreaterThan(1);
      expect(s.r).toBeGreaterThan(0);
    }
    expect(new Set(a.map((s) => s.kind))).toEqual(new Set(['star', 'dot']));
  });

  it('the star path is a closed four-point shape', () => {
    const d = starPath(4);
    expect(d.startsWith('M0 -4')).toBe(true);
    expect(d.endsWith('Z')).toBe(true);
    expect((d.match(/Q/g) ?? []).length).toBe(4);
  });
});
