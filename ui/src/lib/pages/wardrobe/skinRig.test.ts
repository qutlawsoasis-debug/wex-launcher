import { describe, expect, it } from 'vitest';
import { SHADE } from './skinRig';

describe('skin model lighting', () => {
  it('lights faces as the game does: top brightest, then front, back, sides, bottom darkest', () => {
    expect(SHADE.top).toBe(1);
    expect(SHADE.front).toBeLessThan(SHADE.top);
    expect(SHADE.back).toBeLessThan(SHADE.front);
    expect(SHADE.right).toBeLessThan(SHADE.back);
    expect(SHADE.left).toBe(SHADE.right);
    expect(SHADE.bottom).toBeLessThan(SHADE.right);
  });
});
