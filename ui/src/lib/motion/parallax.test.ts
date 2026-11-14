import { describe, it, expect } from 'vitest';
import { follow, offsetIn, settled, PARALLAX_TAU_MS } from './parallax';

const rect = { left: 100, top: 50, width: 800, height: 400 };

describe('parallax: target from the pointer over the whole area', () => {
  it('is zero at the centre and ±0.5 at the edges', () => {
    expect(offsetIn(rect, 500, 250)).toEqual({ x: 0, y: 0 });
    expect(offsetIn(rect, 100, 50)).toEqual({ x: -0.5, y: -0.5 });
    expect(offsetIn(rect, 900, 450)).toEqual({ x: 0.5, y: 0.5 });
  });

  it('clamps outside the area, so crossing an edge cannot jump', () => {
    expect(offsetIn(rect, 5000, -5000)).toEqual({ x: 0.5, y: -0.5 });
  });

  it('is zero for an empty box', () => {
    expect(offsetIn({ left: 0, top: 0, width: 0, height: 0 }, 10, 10)).toEqual({ x: 0, y: 0 });
  });
});

describe('parallax: smoothed follow', () => {
  const zero = { x: 0, y: 0 };
  const edge = { x: 0.5, y: -0.5 };

  it('moves part of the way each frame, never past the target', () => {
    let p = zero;
    let prev = 0;
    for (let i = 0; i < 30; i++) {
      p = follow(p, edge, 16);
      expect(p.x).toBeGreaterThan(prev);
      expect(p.x).toBeLessThanOrEqual(0.5);
      prev = p.x;
    }
  });

  it('does not depend on the frame rate', () => {
    let a = zero;
    for (let i = 0; i < 4; i++) a = follow(a, edge, 8);
    let b = zero;
    for (let i = 0; i < 2; i++) b = follow(b, edge, 16);
    expect(a.x).toBeCloseTo(b.x, 10);
    expect(a.y).toBeCloseTo(b.y, 10);
  });

  it('is quick but smooth: most of the way within ~280 ms, not in one frame', () => {
    const oneFrame = follow(zero, edge, 16);
    expect(oneFrame.x).toBeLessThan(0.15);
    let p = zero;
    for (let t = 0; t < 280; t += 16) p = follow(p, edge, 16);
    expect(p.x).toBeGreaterThan(0.45);
    expect(PARALLAX_TAU_MS * 3).toBeLessThanOrEqual(280);
  });

  it('returns to the centre the same way when the pointer leaves', () => {
    let p = edge;
    for (let t = 0; t < 1000; t += 16) p = follow(p, zero, 16);
    expect(settled(p, zero)).toBe(true);
  });

  it('a zero or negative step changes nothing', () => {
    expect(follow(edge, zero, 0)).toEqual(edge);
    expect(follow(edge, zero, -5)).toEqual(edge);
  });
});
