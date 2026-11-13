import { describe, expect, it } from 'vitest';
import { burst, FW_COLORS, launchRocket, stepRocket, stepSpark } from './fireworks';

const fixed = () => 0.5;

describe('fireworks', () => {
  it('a rocket starts below the window and bursts in the upper part', () => {
    const r = launchRocket(1600, 900, fixed);
    expect(r.y).toBeGreaterThan(900);
    expect(r.burstY).toBeLessThan(900 * 0.5);
    let t = 0;
    while (!stepRocket(r, 1 / 60) && t < 600) t++;
    expect(t).toBeLessThan(600);
    expect(r.y).toBeLessThan(900 * 0.5);
  });

  it('a burst is a ring of sparks in system colours that fall and burn out', () => {
    const sparks = burst(launchRocket(1600, 900, fixed), fixed, 40);
    expect(sparks.length).toBe(40);
    expect(sparks.every((s) => (FW_COLORS as readonly string[]).includes(s.color))).toBe(true);
    const s = sparks[10];
    const y0 = s.y;
    let alive = true;
    let n = 0;
    while (alive && n < 1000) {
      alive = stepSpark(s, 1 / 60);
      n++;
    }
    expect(alive).toBe(false);
    expect(s.y).toBeGreaterThan(y0 - 400);
  });
});
