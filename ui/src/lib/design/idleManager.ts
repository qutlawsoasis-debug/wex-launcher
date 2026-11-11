/**
 * Idle and Power Management for Wex Launcher
 *
 * Implements Apple Human Interface Guidelines and battery/CPU efficiency directives:
 * - Automatically pauses animations, intervals, and rendering when window is minimized / hidden.
 * - Pauses background polling and animations while Minecraft is actively running.
 * - Adds `.app-idle` CSS class to document root to pause all CSS animations and transitions.
 */

type IdleListener = (suspended: boolean) => void;

let isDocHidden = typeof document !== 'undefined' ? document.hidden : false;
let isGameActive = false;
const listeners = new Set<IdleListener>();

/**
 * Returns true if the launcher UI should be suspended (minimized, hidden, or game running).
 */
export function isIdleSuspended(): boolean {
  return isDocHidden || isGameActive;
}

/**
 * Notify all listeners and toggle CSS class on root element.
 */
function updateSuspensionState() {
  const suspended = isIdleSuspended();
  if (typeof document !== 'undefined' && document.documentElement) {
    if (suspended) {
      document.documentElement.classList.add('app-idle');
    } else {
      document.documentElement.classList.remove('app-idle');
    }
  }

  for (const listener of listeners) {
    try {
      listener(suspended);
    } catch (err) {
      console.error('[idleManager] Listener error:', err);
    }
  }
}

/**
 * Subscribe to suspension state changes.
 */
export function onIdleStateChange(listener: IdleListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Update the game running state (called from App.svelte or launch events).
 */
export function setGameRunning(running: boolean) {
  if (isGameActive === running) return;
  isGameActive = running;
  updateSuspensionState();
}

/**
 * Initialize global document visibility listeners (safe for SSR / browser).
 */
export function initIdleManager() {
  if (typeof document === 'undefined') return;

  const handleVisibility = () => {
    isDocHidden = document.hidden;
    updateSuspensionState();
  };

  document.addEventListener('visibilitychange', handleVisibility);
  window.addEventListener('pagehide', handleVisibility);
  window.addEventListener('pageshow', handleVisibility);

  // Initialize initial state
  handleVisibility();

  return () => {
    document.removeEventListener('visibilitychange', handleVisibility);
    window.removeEventListener('pagehide', handleVisibility);
    window.removeEventListener('pageshow', handleVisibility);
  };
}

/**
 * Helper to create an interval that pauses automatically when idle/suspended.
 * When waking up from suspension, immediately triggers once if `runOnResume` is true.
 */
export function createIdleInterval(
  fn: () => void | Promise<void>,
  ms: number,
  runOnResume = true,
): () => void {
  let timerId: number | null = null;
  let wasSuspended = isIdleSuspended();

  const startTimer = () => {
    if (timerId !== null) return;
    timerId = window.setInterval(() => {
      if (!isIdleSuspended()) {
        fn();
      }
    }, ms);
  };

  const stopTimer = () => {
    if (timerId !== null) {
      clearInterval(timerId);
      timerId = null;
    }
  };

  const unsubscribe = onIdleStateChange((suspended) => {
    if (suspended) {
      wasSuspended = true;
      stopTimer();
    } else {
      if (wasSuspended && runOnResume) {
        fn();
      }
      wasSuspended = false;
      startTimer();
    }
  });

  if (!isIdleSuspended()) {
    startTimer();
  }

  return () => {
    stopTimer();
    unsubscribe();
  };
}
