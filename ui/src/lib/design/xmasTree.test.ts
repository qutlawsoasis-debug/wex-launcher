import { describe, expect, it } from 'vitest';
import { CX, TIERS, halfWidthAt, snowPath, swags, tierPath } from './xmasTree';

describe('decorated tree', () => {
  it('tiers grow wider and lower and overlap, like a fir', () => {
    for (let i = 1; i < TIERS.length; i++) {
      expect(TIERS[i].hw).toBeGreaterThan(TIERS[i - 1].hw);
      expect(TIERS[i].top).toBeLessThan(TIERS[i - 1].base);
    }
    expect(tierPath(TIERS[0]).startsWith(`M${CX} ${TIERS[0].top}`)).toBe(true);
    expect(snowPath(TIERS[0]).endsWith('Z')).toBe(true);
  });

  it('the garland swags and their lights stay inside the tree', () => {
    for (const s of swags(7)) {
      expect(s.points.length).toBe(7);
      for (const p of s.points) expect(Math.abs(p.x - CX)).toBeLessThanOrEqual(halfWidthAt(p.y) + 1);
    }
  });

  it('no width above the star or below the lowest tier', () => {
    expect(halfWidthAt(10)).toBe(0);
    expect(halfWidthAt(300)).toBe(0);
  });
});
