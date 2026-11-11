/**
 * Wex Launcher — Motion Reactive State & Lifecycle Store
 * Sourced from Apple HIG Motion guidelines:
 * https://developer.apple.com/design/human-interface-guidelines/motion
 *
 * Rules:
 * - Respects system prefers-reduced-motion media query
 * - Supports launcher effects modes: 'full' | 'low' | 'off' (in 'off', only opacity is animated)
 * - Automatically pauses rAF animations when window is minimized or Minecraft session is running
 */

export type EffectsMode = 'full' | 'low' | 'off';

class MotionStateManager {
  mode = $state<EffectsMode>('full');
  systemReduceMotion = $state<boolean>(false);
  isWindowMinimized = $state<boolean>(false);
  isGameRunning = $state<boolean>(false);

  private queuedFrameCallbacks: Array<{ id: number; cb: FrameRequestCallback }> = [];
  private nextCallbackId = 1;
  private visibilityListenerAttached = false;

  constructor() {
    if (typeof window !== 'undefined') {
      // 1. Listen to system prefers-reduced-motion
      if (typeof window.matchMedia === 'function') {
        const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
        this.systemReduceMotion = motionQuery.matches;
        motionQuery.addEventListener('change', (e) => {
          this.systemReduceMotion = e.matches;
          this.syncDomAttributes();
        });
      }

      // 2. Listen to visibility change to pause rAF when window is minimized or hidden
      if (typeof document !== 'undefined' && !this.visibilityListenerAttached) {
        document.addEventListener('visibilitychange', () => {
          this.isWindowMinimized = document.hidden;
          if (!this.isPaused) {
            this.flushQueuedCallbacks();
          }
        });
        this.visibilityListenerAttached = true;
      }
    }
  }

  /**
   * Returns true if motion should be treated as reduced / off
   */
  get isReduced(): boolean {
    return this.mode === 'off' || this.systemReduceMotion;
  }

  /**
   * Returns true if all animations and rAF cycles must be paused
   * (e.g. launcher window is minimized/hidden or game is currently running)
   */
  get isPaused(): boolean {
    return this.isWindowMinimized || this.isGameRunning;
  }

  setMode(newMode: EffectsMode) {
    this.mode = newMode;
    this.syncDomAttributes();
  }

  setWindowMinimized(minimized: boolean) {
    this.isWindowMinimized = minimized;
    if (!this.isPaused) {
      this.flushQueuedCallbacks();
    }
  }

  setGameRunning(running: boolean) {
    this.isGameRunning = running;
    if (!this.isPaused) {
      this.flushQueuedCallbacks();
    }
  }

  syncDomAttributes() {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    root.setAttribute('data-effects', this.mode);
    root.setAttribute('data-reduce-motion', String(this.isReduced));
  }

  /**
   * Safe requestAnimationFrame that respects paused state:
   * When paused (minimized or game running), queues the callback and executes when resumed.
   */
  safeRequestAnimationFrame(cb: FrameRequestCallback): number {
    const id = this.nextCallbackId++;
    if (this.isPaused) {
      this.queuedFrameCallbacks.push({ id, cb });
      return id;
    }

    if (typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
      return window.requestAnimationFrame(cb);
    }

    // SSR / test fallback
    const timer = setTimeout(() => cb(Date.now()), 16);
    return timer as unknown as number;
  }

  /**
   * Cancels a queued or native animation frame
   */
  safeCancelAnimationFrame(id: number) {
    this.queuedFrameCallbacks = this.queuedFrameCallbacks.filter((item) => item.id !== id);
    if (typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
      window.cancelAnimationFrame(id);
    } else {
      clearTimeout(id);
    }
  }

  private flushQueuedCallbacks() {
    if (this.queuedFrameCallbacks.length === 0) return;
    const toRun = [...this.queuedFrameCallbacks];
    this.queuedFrameCallbacks = [];
    const timestamp = typeof performance !== 'undefined' ? performance.now() : Date.now();
    for (const item of toRun) {
      try {
        item.cb(timestamp);
      } catch {
        // Suppress errors during deferred callback flush
      }
    }
  }
}

export const motionState = new MotionStateManager();
