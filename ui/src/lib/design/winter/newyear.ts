/** The New Year countdown on the build tile (NewYearChip.svelte), local time. */

export type NewYearState =
  | { kind: 'days'; days: number }
  | { kind: 'clock'; h: number; m: number; s: number }
  | { kind: 'greeting'; year: number }
  | { kind: 'none' };

/** The coming midnight of 1 January after `now`. */
export function nextNewYear(now: Date): Date {
  return new Date(now.getFullYear() + 1, 0, 1, 0, 0, 0, 0);
}

/**
 * November and December: days to go, and on the last day a clock.
 * 1–7 January: a greeting. Otherwise nothing.
 */
export function newYearState(now: Date): NewYearState {
  const m = now.getMonth();
  if (m === 0 && now.getDate() <= 7) return { kind: 'greeting', year: now.getFullYear() };
  if (m !== 10 && m !== 11) return { kind: 'none' };
  const left = nextNewYear(now).getTime() - now.getTime();
  if (left <= 24 * 3600 * 1000) {
    const s = Math.max(0, Math.floor(left / 1000));
    return { kind: 'clock', h: Math.floor(s / 3600), m: Math.floor((s % 3600) / 60), s: s % 60 };
  }
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const days = Math.round((nextNewYear(now).getTime() - today.getTime()) / (24 * 3600 * 1000));
  return { kind: 'days', days };
}

/** True when midnight of 1 January falls between two ticks. */
export function crossedMidnight(prev: Date, now: Date): boolean {
  return prev.getFullYear() !== now.getFullYear() && now.getMonth() === 0;
}

export const pad2 = (n: number) => String(n).padStart(2, '0');
