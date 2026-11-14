import { describe, expect, it } from 'vitest';
import { crossedMidnight, newYearState, nextNewYear } from './newyear';

describe('New Year countdown', () => {
  it('counts whole days in November and December', () => {
    expect(newYearState(new Date(2026, 10, 13, 20, 0))).toEqual({ kind: 'days', days: 49 });
    expect(newYearState(new Date(2026, 11, 30, 9, 0))).toEqual({ kind: 'days', days: 2 });
  });

  it('on the last day it is a clock to midnight', () => {
    expect(newYearState(new Date(2026, 11, 31, 23, 58, 30))).toEqual({ kind: 'clock', h: 0, m: 1, s: 30 });
    expect(newYearState(new Date(2026, 11, 31, 0, 0, 1)).kind).toBe('clock');
  });

  it('greets in the first week of January, then nothing until November', () => {
    expect(newYearState(new Date(2027, 0, 1, 0, 0, 5))).toEqual({ kind: 'greeting', year: 2027 });
    expect(newYearState(new Date(2027, 0, 8)).kind).toBe('none');
    expect(newYearState(new Date(2027, 6, 1)).kind).toBe('none');
  });

  it('knows the coming midnight and when it passed', () => {
    expect(nextNewYear(new Date(2026, 11, 31, 12)).getTime()).toBe(new Date(2027, 0, 1).getTime());
    expect(crossedMidnight(new Date(2026, 11, 31, 23, 59, 59), new Date(2027, 0, 1, 0, 0, 0))).toBe(true);
    expect(crossedMidnight(new Date(2026, 11, 31, 23, 59, 58), new Date(2026, 11, 31, 23, 59, 59))).toBe(false);
  });
});
