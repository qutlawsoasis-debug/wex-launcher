import { describe, expect, it } from 'vitest';
import { driftPath, firTree, forestLayer, hillPath, leftCorner, makeStars, rightCorner, rng } from './scene';

const numbers = (d: string) => (d.match(/-?\d+(\.\d+)?/g) ?? []).map(Number);
const isPath = (d: string) => /^M[-\d. ]+/.test(d) && d.trim().endsWith('Z') && numbers(d).every(Number.isFinite);

describe('winter scene geometry', () => {
  it('the generator is deterministic and stays in [0, 1)', () => {
    const a = rng(42);
    const b = rng(42);
    const xs = Array.from({ length: 500 }, () => a());
    expect(xs).toEqual(Array.from({ length: 500 }, () => b()));
    expect(xs.every((x) => x >= 0 && x < 1)).toBe(true);
    expect(rng(43)()).not.toBe(xs[0]);
  });

  it('stars: the same 120 every time, inside the sky, a few twinkling', () => {
    const s = makeStars(120);
    expect(s).toHaveLength(120);
    expect(makeStars(120)).toEqual(s);
    expect(s.every((x) => x.x >= 0 && x.x <= 100 && x.y >= 0 && x.y <= 84)).toBe(true);
    expect(s.every((x) => x.size > 0 && x.size <= 3 && x.alpha > 0 && x.alpha <= 1)).toBe(true);
    const tw = s.filter((x) => x.twinkle).length;
    expect(tw).toBeGreaterThan(4);
    expect(tw).toBeLessThan(30);
    // Slow twinkle: periods of several seconds.
    expect(s.every((x) => x.dur >= 3 && x.dur <= 8)).toBe(true);
  });

  it('a fir is a spruce of 8 to 14 drooping tiers, widening downwards, snow inside its height', () => {
    const f = firTree(100, 500, 300, 140, 9);
    expect(firTree(100, 500, 300, 140, 9)).toEqual(f);
    expect(f.tiers.length).toBeGreaterThanOrEqual(12);
    expect(f.tiers.length).toBeLessThanOrEqual(14);
    for (const t of f.tiers) {
      expect(isPath(t.body)).toBe(true);
      expect(isPath(t.snow)).toBe(true);
      expect(t.needles.length).toBeGreaterThan(0);
    }
    // Painted lowest first: tiers get narrower and higher towards the end.
    const span = (d: string) => {
      const xs = numbers(d).filter((_, i) => i % 2 === 0);
      return Math.max(...xs) - Math.min(...xs);
    };
    expect(span(f.tiers[0].body)).toBeGreaterThan(span(f.tiers[f.tiers.length - 1].body) * 3);
    expect(f.tiers[0].oy).toBeGreaterThan(f.tiers[f.tiers.length - 1].oy);
    const ys = f.tiers.flatMap((t) => numbers(t.snow).filter((_, i) => i % 2 === 1));
    expect(Math.min(...ys)).toBeGreaterThanOrEqual(200 - 6);
    expect(Math.max(...ys)).toBeLessThanOrEqual(500);
    const xs = f.tiers.flatMap((t) => numbers(t.body).filter((_, i) => i % 2 === 0));
    expect(Math.min(...xs)).toBeGreaterThan(100 - 90);
    expect(Math.max(...xs)).toBeLessThan(100 + 90);
    // The near tree has a bit of trunk; a far one is a plain hazy silhouette.
    expect(isPath(f.trunk)).toBe(true);
    const far = firTree(100, 500, 300, 140, 9, 0);
    expect(far.trunk).toBe('');
    expect(far.tiers.every((t) => t.needles === '' && t.rim === '')).toBe(true);
    expect(far.tiers.length).toBeGreaterThanOrEqual(8);
  });

  it('the whole forest stays within a static budget of path points', () => {
    let points = 0;
    for (const c of [rightCorner(), leftCorner()])
      for (const l of c.layers)
        for (const t of l.trees)
          for (const d of [t.trunk, t.base, ...t.tiers.flatMap((x) => [x.shade, x.body, x.needles, x.snow, x.rim])]) points += numbers(d).length / 2;
    // Dense needles on the near trees, plain hazy silhouettes far away; computed once.
    expect(points).toBeLessThan(30000);
  });

  it('a layer places its trees in range, sorted small to large', () => {
    const t = forestLayer({ count: 5, h: [100, 200], x: [0, 500], baseY: 400 }, 3);
    expect(t).toHaveLength(5);
    expect(t.every((x) => x.h >= 100 && x.h <= 200 && x.x >= -50 && x.x <= 550)).toBe(true);
    expect(t.map((x) => x.h)).toEqual([...t.map((x) => x.h)].sort((a, b) => a - b));
  });

  it('hills and drifts are closed paths reaching the bottom', () => {
    const h = hillPath(1000, 40, 100, 10, 5, { left: 10, right: 20 });
    expect(isPath(h)).toBe(true);
    expect(h.startsWith('M0 100')).toBe(true);
    const d = driftPath(600, 680, 80, 2, 'right');
    expect(isPath(d)).toBe(true);
    // The drift sinks to the ground line on its open side.
    const ys = numbers(d).filter((_, i) => i % 2 === 1);
    expect(ys[1]).toBeGreaterThan(700);
  });

  it('corners have three depths, nearer trees taller', () => {
    for (const c of [rightCorner(), leftCorner()]) {
      expect(c.layers).toHaveLength(3);
      const tallest = c.layers.map((l) => Math.max(...l.trees.map((t) => t.h)));
      expect(tallest[0]).toBeLessThan(tallest[1]);
      expect(tallest[1]).toBeLessThan(tallest[2]);
      expect(tallest[2]).toBeLessThanOrEqual(c.h);
    }
    expect(rightCorner()).toEqual(rightCorner());
  });
});
