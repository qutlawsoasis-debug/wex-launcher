import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  isIdleSuspended,
  setGameRunning,
  onIdleStateChange,
  createIdleInterval,
  initIdleManager,
} from './idleManager';

describe('Idle & Power Management (idleManager)', () => {
  beforeEach(() => {
    setGameRunning(false);
    Object.defineProperty(document, 'hidden', {
      configurable: true,
      get: () => false,
    });
    document.documentElement.classList.remove('app-idle');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    setGameRunning(false);
    document.documentElement.classList.remove('app-idle');
  });

  it('reports not suspended when window is visible and game is not running', () => {
    expect(isIdleSuspended()).toBe(false);
    expect(document.documentElement.classList.contains('app-idle')).toBe(false);
  });

  it('suspends when game is running and adds .app-idle CSS class', () => {
    const states: boolean[] = [];
    const unsub = onIdleStateChange((suspended) => {
      states.push(suspended);
    });

    setGameRunning(true);
    expect(isIdleSuspended()).toBe(true);
    expect(document.documentElement.classList.contains('app-idle')).toBe(true);

    setGameRunning(false);
    expect(isIdleSuspended()).toBe(false);
    expect(document.documentElement.classList.contains('app-idle')).toBe(false);

    unsub();
    expect(states).toEqual([true, false]);
  });

  it('pauses idle intervals when suspended and resumes upon wake', () => {
    vi.useFakeTimers();
    const tickFn = vi.fn();

    const cleanup = createIdleInterval(tickFn, 1000, true);

    // Initial state: not suspended
    vi.advanceTimersByTime(1000);
    expect(tickFn).toHaveBeenCalledTimes(1);

    // Suspend (e.g. game starts)
    setGameRunning(true);
    vi.advanceTimersByTime(5000);
    // Interval should NOT fire while suspended
    expect(tickFn).toHaveBeenCalledTimes(1);

    // Resume (game finishes)
    setGameRunning(false);
    // With runOnResume=true, should fire immediately upon resume
    expect(tickFn).toHaveBeenCalledTimes(2);

    vi.advanceTimersByTime(1000);
    expect(tickFn).toHaveBeenCalledTimes(3);

    cleanup();
    vi.useRealTimers();
  });

  it('initializes document visibility listener', () => {
    const cleanup = initIdleManager();

    // Simulate document becoming hidden (minimized window)
    Object.defineProperty(document, 'hidden', {
      configurable: true,
      get: () => true,
    });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(isIdleSuspended()).toBe(true);
    expect(document.documentElement.classList.contains('app-idle')).toBe(true);

    // Restore window
    Object.defineProperty(document, 'hidden', {
      configurable: true,
      get: () => false,
    });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(isIdleSuspended()).toBe(false);
    expect(document.documentElement.classList.contains('app-idle')).toBe(false);

    cleanup?.();
  });
});
