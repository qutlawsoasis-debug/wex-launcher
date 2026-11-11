import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  Durations,
  MotionCurves,
  PRESS_SCALE,
  screenTransition,
  sheetTransition,
  toastTransition,
  motionState,
  createManagedAnimation,
  isSystemReduceMotion,
} from './index';

describe('Apple HIG Motion Specifications & Tokens', () => {
  it('conforms to Apple HIG duration range (120–350 ms)', () => {
    for (const [name, ms] of Object.entries(Durations)) {
      expect(ms).toBeGreaterThanOrEqual(120);
      expect(ms).toBeLessThanOrEqual(350);
    }
  });

  it('guarantees exit transitions are strictly faster than entry transitions', () => {
    expect(Durations.screenExit).toBeLessThan(Durations.screenEnter);
    expect(Durations.sheetExit).toBeLessThan(Durations.sheetEnter);
    expect(Durations.modalExit).toBeLessThan(Durations.modalEnter);
    expect(Durations.toastExit).toBeLessThan(Durations.toastEnter);
  });

  it('provides critically damped spring without overshoot for system chrome', () => {
    expect(MotionCurves.springSystem.dampingRatio).toBe(1.0);
    expect(MotionCurves.springSystem.cssBezier).toBe('cubic-bezier(0.16, 1, 0.3, 1)');
  });

  it('provides light spring with subtle bounce for interactive capsules and cards', () => {
    expect(MotionCurves.springBouncy.dampingRatio).toBeLessThan(1.0);
    expect(MotionCurves.springBouncy.dampingRatio).toBeGreaterThan(0.8);
    expect(MotionCurves.springBouncy.cssBezier).toContain('1.05');
  });

  it('strictly sets button press scale to 0.97', () => {
    expect(PRESS_SCALE).toBe(0.97);
  });
});

describe('Effects Modes & Reduced Motion Accessibility', () => {
  beforeEach(() => {
    motionState.setMode('full');
    motionState.systemReduceMotion = false;
    motionState.setWindowMinimized(false);
    motionState.setGameRunning(false);
  });

  it('reports reduced motion when mode is "off" or system reduced motion is enabled', () => {
    expect(motionState.isReduced).toBe(false);

    motionState.setMode('off');
    expect(motionState.isReduced).toBe(true);

    motionState.setMode('full');
    motionState.systemReduceMotion = true;
    expect(motionState.isReduced).toBe(true);
  });

  it('produces purely opacity CSS in screenTransition when mode is "off"', () => {
    motionState.setMode('off');
    const dummyEl = document.createElement('div');
    const tr = screenTransition(dummyEl, { isEnter: true });

    expect(tr.css).toBeDefined();
    if (tr.css) {
      const cssOut = tr.css(0.5, 0.5);
      expect(cssOut).toContain('opacity');
      expect(cssOut).not.toContain('transform');
    }
  });

  it('animates only transform and opacity in screenTransition during normal mode', () => {
    motionState.setMode('full');
    const dummyEl = document.createElement('div');
    const tr = screenTransition(dummyEl, { isEnter: true, offsetY: 12 });

    expect(tr.css).toBeDefined();
    if (tr.css) {
      const cssOut = tr.css(0.5, 0.5);
      expect(cssOut).toContain('opacity: 0.5');
      expect(cssOut).toContain('transform: translateY(');
      // Ensure only transform and opacity are touched
      const properties = cssOut.split(';').map((s) => s.trim().split(':')[0]).filter(Boolean);
      for (const prop of properties) {
        expect(['opacity', 'transform']).toContain(prop);
      }
    }
  });

  it('produces bottom slide-up transform in sheetTransition and disables transform in "off" mode', () => {
    motionState.setMode('full');
    const dummyEl = document.createElement('div');
    const trNormal = sheetTransition(dummyEl, { isEnter: true });
    expect(trNormal.css).toBeDefined();
    const cssNormal = trNormal.css!(0.2, 0.8);
    expect(cssNormal).toContain('transform: translateY(');
    expect(cssNormal).toContain('scale(');

    motionState.setMode('off');
    const trOff = sheetTransition(dummyEl, { isEnter: true });
    const cssOff = trOff.css!(0.2, 0.8);
    expect(cssOff).toContain('opacity: 0.2');
    expect(cssOff).not.toContain('transform');
  });

  it('animates toast sliding down and only uses transform and opacity', () => {
    motionState.setMode('full');
    const dummyEl = document.createElement('div');
    const tr = toastTransition(dummyEl, { isEnter: true });
    expect(tr.css).toBeDefined();
    const css = tr.css!(0.5, 0.5);
    expect(css).toContain('opacity: 0.5');
    expect(css).toContain('transform: translateY(');
  });
});

describe('Animation Lifecycle & Pause during Game / Minimized Window', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    motionState.setMode('full');
    motionState.setWindowMinimized(false);
    motionState.setGameRunning(false);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('pauses lifecycle when window is minimized or game is active', () => {
    expect(motionState.isPaused).toBe(false);

    motionState.setWindowMinimized(true);
    expect(motionState.isPaused).toBe(true);

    motionState.setWindowMinimized(false);
    expect(motionState.isPaused).toBe(false);

    motionState.setGameRunning(true);
    expect(motionState.isPaused).toBe(true);

    motionState.setGameRunning(false);
    expect(motionState.isPaused).toBe(false);
  });

  it('defers safeRequestAnimationFrame callbacks while paused and flushes on resume', () => {
    motionState.setGameRunning(true);
    let called = false;

    motionState.safeRequestAnimationFrame(() => {
      called = true;
    });

    expect(called).toBe(false);

    // Resuming game execution flushes queued callbacks
    motionState.setGameRunning(false);
    expect(called).toBe(true);
  });

  it('creates managed animation ticker that pauses during active Minecraft session', () => {
    let tickCount = 0;
    const anim = createManagedAnimation(() => {
      tickCount++;
    });

    anim.start();
    expect(anim.isRunning()).toBe(true);

    // Running game pauses ticking
    motionState.setGameRunning(true);
    const beforeCount = tickCount;

    // Simulate elapsed time while game is running
    vi.advanceTimersByTime(200);
    expect(tickCount).toBe(beforeCount);

    // When game finishes, animation resumes
    motionState.setGameRunning(false);
    anim.stop();
    expect(anim.isRunning()).toBe(false);
  });
});
