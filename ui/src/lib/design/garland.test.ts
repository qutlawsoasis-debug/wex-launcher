import { describe, expect, it } from 'vitest';
import { BULB_COLORS, garlandLayout } from './garland';

describe('garland', () => {
  it('spans the whole width from pin to pin, bulbs evenly along the wire', () => {
    const g = garlandLayout(1200);
    expect(g.pins[0]).toBe(0);
    expect(g.pins.at(-1)).toBe(1200);
    expect(g.bulbs.length).toBe((g.pins.length - 1) * 4);
    const xs = g.bulbs.map((b) => b.x);
    expect([...xs].sort((a, b) => a - b)).toEqual(xs);
    expect(xs.every((x) => x > 0 && x < 1200)).toBe(true);
  });

  it('the wire sags between pins: middle bulbs hang lower and level, outer ones tilt', () => {
    const g = garlandLayout(184, { step: 46, perSpan: 4, top: 2, sag: 9 });
    const [a, b, c, d] = g.bulbs;
    expect(b.y).toBeGreaterThan(a.y);
    expect(b.y).toBeCloseTo(c.y, 5);
    expect(Math.abs(a.angle)).toBeGreaterThan(Math.abs(b.angle));
    expect(Math.sign(a.angle)).toBe(-Math.sign(d.angle));
    expect(Math.max(...g.bulbs.map((x) => x.y))).toBeLessThanOrEqual(2 + 9);
  });

  it('bulbs cycle through Apple system colours in order', () => {
    const g = garlandLayout(1200);
    g.bulbs.forEach((b, i) => expect(b.color).toBe(BULB_COLORS[i % BULB_COLORS.length]));
  });

  it('nothing to draw before the tile has a width', () => {
    expect(garlandLayout(0).bulbs).toEqual([]);
  });
});
