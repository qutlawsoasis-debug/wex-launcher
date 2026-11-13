import { describe, expect, it } from 'vitest';
import { driftHeights, driftPath, glints, seeded } from './drift';
import { drawCrystal } from './snow';

describe('snowdrift', () => {
  it('a tile keeps its own hills: same seed, same shape; another seed, another', () => {
    expect(driftPath('1.21.1', 1000, 60, 0.2, 0.9)).toBe(driftPath('1.21.1', 1000, 60, 0.2, 0.9));
    expect(driftPath('1.21.1', 1000, 60, 0.2, 0.9)).not.toBe(driftPath('1.20.1', 1000, 60, 0.2, 0.9));
    const r = seeded('x');
    expect(r()).not.toBe(r());
  });

  it('hills stay between the given heights and the path closes along the bottom', () => {
    const hs = driftHeights('1.21.1', 48, 0.22, 0.9);
    expect(Math.min(...hs)).toBeCloseTo(0.22, 5);
    expect(Math.max(...hs)).toBeCloseTo(0.9, 5);
    const d = driftPath('1.21.1', 1000, 60, 0.22, 0.9);
    expect(d.startsWith('M0 60')).toBe(true);
    expect(d.endsWith('L1000 60 Z')).toBe(true);
  });

  it('a narrow tile gets fewer, wider hills (no spikes when squeezed)', () => {
    const turns = (hs: number[]) => hs.slice(1, -1).filter((h, i) => (h - hs[i]) * (hs[i + 2] - h) < 0).length;
    expect(turns(driftHeights('1.20.1', 48, 0.2, 0.9, 0.12))).toBeLessThan(turns(driftHeights('1.20.1', 48, 0.2, 0.9, 1)));
  });

  it('glints sit on the snow, inside the box', () => {
    for (const g of glints('1.21.1', 1000, 60, 9)) {
      expect(g.x).toBeGreaterThanOrEqual(0);
      expect(g.x).toBeLessThanOrEqual(1000);
      expect(g.y).toBeGreaterThan(0);
      expect(g.y).toBeLessThan(60);
    }
  });
});

describe('snow crystal', () => {
  it('has six arms, each with two pairs of side branches', () => {
    const lines: Array<[string, number, number]> = [];
    let rotations = 0;
    const ctx = {
      save() {},
      restore() {},
      translate() {},
      rotate() {
        rotations++;
      },
      beginPath() {},
      stroke() {},
      moveTo(x: number, y: number) {
        lines.push(['m', x, y]);
      },
      lineTo(x: number, y: number) {
        lines.push(['l', x, y]);
      },
    } as unknown as CanvasRenderingContext2D;
    drawCrystal(ctx, 48, '#fff');
    expect(rotations).toBe(6);
    expect(lines.filter((l) => l[0] === 'l').length).toBe(6 * 5);
  });
});
