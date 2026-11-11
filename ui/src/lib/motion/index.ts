/**
 * Wex Launcher — Central Motion System
 * Sourced strictly from Apple Human Interface Guidelines:
 * https://developer.apple.com/design/human-interface-guidelines/motion
 *
 * Rules:
 * - Only transform and opacity are animated.
 * - Apple spring curves: critically damped for system elements (no bounce),
 *   light spring for cards/capsules.
 * - Standard durations: 200–350 ms, exit is always faster than entry.
 * - Full respect for prefers-reduced-motion and launcher effects modes (full / low / off).
 * - In 'off' mode: transforms are omitted; only opacity transitions occur.
 * - RequestAnimationFrame loops and cycles automatically pause during Minecraft sessions
 *   or when window is minimized.
 */

export * from '../design/motion';

import type { TransitionConfig } from 'svelte/transition';
import { motionState, type EffectsMode } from './state.svelte';

export { motionState, type EffectsMode };

export interface SpringConfig {
  response: number;
  dampingRatio: number;
  durationMs: number;
  cssBezier: string;
}

/**
 * Standardized Apple HIG Motion Durations (in milliseconds)
 * Exit durations are engineered to be faster than entry durations.
 */
export const Durations = {
  /** Screen view enter */
  screenEnter: 260,
  /** Screen view exit (faster than enter) */
  screenExit: 180,
  /** Sheet & modal slide-up presentation */
  sheetEnter: 320,
  /** Sheet & modal dismissal (faster than enter) */
  sheetExit: 220,
  /** Modal dialog enter */
  modalEnter: 260,
  /** Modal dialog exit */
  modalExit: 180,
  /** Toast banner enter */
  toastEnter: 240,
  /** Toast banner exit (faster than enter) */
  toastExit: 180,
  /** Quick press down state feedback */
  press: 120,
  /** Interactive hover surface illumination */
  hover: 160,
  /** Morphing state transition between launcher modes */
  morph: 300,
} as const;

/**
 * Apple Spring & Easing Curves
 */
export const MotionCurves = {
  /** Critically damped Apple spring for system elements: titlebars, navigation, modals (no overshoot) */
  springSystem: {
    response: 0.35,
    dampingRatio: 1.0,
    durationMs: Durations.screenEnter,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Interactive spring with subtle pleasant bounce for cards, capsules, buttons */
  springInteractive: {
    response: 0.35,
    dampingRatio: 0.82,
    durationMs: 250,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Interactive spring with light bounce */
  springBouncy: {
    response: 0.32,
    dampingRatio: 0.86,
    durationMs: Durations.morph,
    cssBezier: 'cubic-bezier(0.2, 0.9, 0.3, 1.05)',
  },
  /** Sheet & modal presentation */
  sheetPresentation: {
    response: 0.42,
    dampingRatio: 0.92,
    durationMs: Durations.sheetEnter,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Rapid exit curve for dismissals */
  fastExit: {
    response: 0.22,
    dampingRatio: 0.98,
    durationMs: Durations.screenExit,
    cssBezier: 'cubic-bezier(0.3, 0, 0.8, 0.15)',
  },
  /** Tactile button press feedback */
  pressFeedback: {
    response: 0.15,
    dampingRatio: 0.95,
    durationMs: Durations.press,
    cssBezier: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
  },
  /** Pure linear fade (for reduced motion and off mode) */
  fadeOnly: {
    response: 0.2,
    dampingRatio: 1.0,
    durationMs: 150,
    cssBezier: 'cubic-bezier(0, 0, 1, 1)',
  },
} as const;

/**
 * Tactile scale feedback factor on active press
 */
export const PRESS_SCALE = 0.97;
export const CARD_HOVER_Y = -2;

/**
 * Checks system prefers-reduced-motion media query
 */
export function isSystemReduceMotion(): boolean {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * Svelte Transition: Screen Viewport Transition (morph & slide-fade)
 * Uses ONLY transform and opacity.
 * In 'off' mode or with reduced motion, transforms are omitted completely.
 */
export function screenTransition(
  _node: HTMLElement,
  options?: {
    isEnter?: boolean;
    offsetY?: number;
    duration?: number;
  }
): TransitionConfig {
  const isReduced = motionState.isReduced;
  const isEnter = options?.isEnter ?? true;
  const duration = options?.duration ?? (isEnter ? Durations.screenEnter : Durations.screenExit);
  const offsetY = options?.offsetY ?? (isEnter ? 10 : -8);

  if (isReduced) {
    return {
      duration: Math.min(duration, 150),
      css: (t) => `opacity: ${t};`,
    };
  }

  const easing = isEnter
    ? (t: number) => 1 - Math.pow(1 - t, 3)
    : (t: number) => t * t;

  return {
    duration,
    easing,
    css: (t, u) => {
      const y = isEnter ? (1 - t) * offsetY : u * offsetY;
      return `opacity: ${t}; transform: translateY(${y.toFixed(2)}px);`;
    },
  };
}

/**
 * Svelte Transition: Sheet presentation from bottom with backdrop dimming
 * Uses ONLY transform and opacity.
 */
export function sheetTransition(
  _node: HTMLElement,
  options?: {
    isEnter?: boolean;
    offsetY?: number;
    duration?: number;
  }
): TransitionConfig {
  const isReduced = motionState.isReduced;
  const isEnter = options?.isEnter ?? true;
  const duration = options?.duration ?? (isEnter ? Durations.sheetEnter : Durations.sheetExit);
  const offsetY = options?.offsetY ?? 40;

  if (isReduced) {
    return {
      duration: Math.min(duration, 160),
      css: (t) => `opacity: ${t};`,
    };
  }

  const easing = isEnter
    ? (t: number) => 1 - Math.pow(1 - t, 3)
    : (t: number) => t * t;

  return {
    duration,
    easing,
    css: (t, u) => {
      const y = isEnter ? (1 - t) * offsetY : u * (offsetY * 0.75);
      const scale = isEnter ? 0.98 + 0.02 * t : 0.98 + 0.02 * t;
      return `opacity: ${t}; transform: translateY(${y.toFixed(2)}px) scale(${scale.toFixed(3)});`;
    },
  };
}

/**
 * Svelte Transition: Toast HUD notification
 * Enters smoothly from top; exits quickly.
 */
export function toastTransition(
  _node: HTMLElement,
  options?: {
    isEnter?: boolean;
  }
): TransitionConfig {
  const isReduced = motionState.isReduced;
  const isEnter = options?.isEnter ?? true;
  const duration = isEnter ? Durations.toastEnter : Durations.toastExit;

  if (isReduced) {
    return {
      duration: Math.min(duration, 140),
      css: (t) => `opacity: ${t};`,
    };
  }

  const easing = isEnter
    ? (t: number) => 1 - Math.pow(1 - t, 3)
    : (t: number) => t * t;

  return {
    duration,
    easing,
    css: (t, u) => {
      const y = isEnter ? -(1 - t) * 16 : -u * 12;
      const scale = 0.96 + 0.04 * t;
      return `opacity: ${t}; transform: translateY(${y.toFixed(2)}px) scale(${scale.toFixed(3)});`;
    },
  };
}

/**
 * Managed animation loop that automatically pauses when window is minimized or game is running.
 */
export function createManagedAnimation(onTick: (deltaMs: number) => void): {
  start: () => void;
  stop: () => void;
  isRunning: () => boolean;
} {
  let active = false;
  let lastTime = 0;
  let frameId: number | null = null;

  function loop(time: number) {
    if (!active) return;
    const delta = lastTime > 0 ? time - lastTime : 16;
    lastTime = time;

    // Do not advance animation frames if paused
    if (!motionState.isPaused) {
      try {
        onTick(delta);
      } catch {
        // Safe execution
      }
    }

    frameId = motionState.safeRequestAnimationFrame(loop);
  }

  return {
    start() {
      if (active) return;
      active = true;
      lastTime = typeof performance !== 'undefined' ? performance.now() : Date.now();
      frameId = motionState.safeRequestAnimationFrame(loop);
    },
    stop() {
      active = false;
      if (frameId !== null) {
        motionState.safeCancelAnimationFrame(frameId);
        frameId = null;
      }
    },
    isRunning() {
      return active;
    },
  };
}
