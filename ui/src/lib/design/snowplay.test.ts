import { describe, expect, it } from 'vitest';
import { cap, drop, grow, landingBall, load, MAX_BALLS, pickUp, R_MAX, restOn, save, settle, snowmen, surfaceAt, type Ball } from './snowplay';

const flat = () => 100;
const ball = (id: number, x: number, r: number, on: number | null = null): Ball => ({ id, x, y: 0, r, on, roll: 0 });

describe('snowman building', () => {
  it('a rolled ball grows with the distance, up to a limit', () => {
    expect(grow(6, 0)).toBe(6);
    expect(grow(6, 200)).toBeCloseTo(21, 5);
    expect(grow(6, 100000)).toBe(R_MAX);
  });

  it('the snow surface follows the drift heights', () => {
    const hs = [0.5, 1, 0.5];
    expect(surfaceAt(0, 200, 400, hs, 40)).toBe(380);
    expect(surfaceAt(100, 200, 400, hs, 40)).toBe(360);
    expect(surfaceAt(50, 200, 400, hs, 40)).toBe(370);
  });

  it('a ball dropped over another stacks on it, centred; elsewhere it falls onto the snow', () => {
    let balls = settle([ball(1, 100, 30), ball(2, 300, 20)], flat);
    expect(balls[0].y).toBeCloseTo(100 - 30 * 0.82, 5);
    balls = balls.map((b) => (b.id === 2 ? { ...b, x: 110, y: 10 } : b));
    balls = drop(2, balls, flat);
    const top = balls.find((b) => b.id === 2)!;
    expect(top.on).toBe(1);
    expect(top.x).toBe(100);
    expect(top.y).toBeCloseTo(restOn(top, balls[0]), 5);
    balls = balls.map((b) => (b.id === 2 ? { ...b, x: 400 } : b));
    balls = drop(2, pickUp(2, balls), flat);
    expect(balls.find((b) => b.id === 2)!.on).toBeNull();
  });

  it('lands on the top of a stack, never on itself or on what stood on it', () => {
    const balls = settle([ball(1, 100, 30), ball(2, 100, 20, 1), ball(3, 400, 10)], flat);
    expect(landingBall({ id: 3, x: 102, r: 10 }, balls)!.id).toBe(2);
    // Carrying the base: the ball on it does not count as a landing place.
    const after = drop(1, balls.map((b) => (b.id === 1 ? { ...b, x: 100 } : b)), flat);
    expect(after.find((b) => b.id === 1)!.on).toBeNull();
  });

  it('picking up a ball lets what stood on it fall onto the snow', () => {
    let balls = settle([ball(1, 100, 30), ball(2, 100, 20, 1)], flat);
    balls = settle(pickUp(1, balls).filter((b) => b.id !== 1), flat);
    const fallen = balls.find((b) => b.id === 2)!;
    expect(fallen.on).toBeNull();
    expect(fallen.y).toBeCloseTo(100 - 20 * 0.82, 5);
  });

  it('three balls, each smaller than the one below, make a snowman', () => {
    const ok = settle([ball(1, 100, 30), ball(2, 100, 20, 1), ball(3, 100, 12, 2)], flat);
    expect(snowmen(ok)).toEqual([{ base: 1, middle: 2, head: 3 }]);
    const wrong = settle([ball(1, 100, 20), ball(2, 100, 30, 1), ball(3, 100, 12, 2)], flat);
    expect(snowmen(wrong)).toEqual([]);
    expect(snowmen(settle([ball(1, 100, 30), ball(2, 100, 20, 1)], flat))).toEqual([]);
  });

  it('keeps at most MAX_BALLS; the oldest loose ball melts first', () => {
    const many = Array.from({ length: MAX_BALLS + 2 }, (_, i) => ball(i, i * 50, 10));
    const kept = cap(many);
    expect(kept.length).toBe(MAX_BALLS);
    expect(kept[0].id).toBe(2);
  });

  it('saved scenes come back at the same place in a resized tile; bad data is ignored', () => {
    const balls = settle([ball(1, 250, 30), ball(2, 250, 20, 1)], flat);
    const back = load(save(balls, 1000), 2000, flat);
    expect(back.map((b) => [b.x, b.on])).toEqual([
      [500, null],
      [500, 1],
    ]);
    expect(load('nope', 1000, flat)).toEqual([]);
    expect(load([{ id: 'x' }, null], 1000, flat)).toEqual([]);
  });
});
