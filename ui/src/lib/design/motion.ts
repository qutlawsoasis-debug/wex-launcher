/**
 * Apple HIG Motion & Animation Specifications.
 * Sourced from tasks/apple/hig/motion.md and Aura spring models
 */

export interface SpringConfig {
  response: number;
  dampingRatio: number;
  durationMs: number;
  cssBezier: string;
}

export const MotionCurves = {
  /** Default Apple interactive spring for buttons, tabs, controls */
  springInteractive: {
    response: 0.35,
    dampingRatio: 0.82,
    durationMs: 250,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Sheet & modal slide-up presentation */
  sheetPresentation: {
    response: 0.45,
    dampingRatio: 0.88,
    durationMs: 320,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Popover scale and fade */
  popoverScale: {
    response: 0.28,
    dampingRatio: 0.85,
    durationMs: 200,
    cssBezier: 'cubic-bezier(0.16, 1, 0.3, 1)',
  },
  /** Quick press down state feedback */
  pressFeedback: {
    response: 0.15,
    dampingRatio: 0.95,
    durationMs: 120,
    cssBezier: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
  },
} as const;

/**
 * Checks system prefers-reduced-motion media query
 */
export function isSystemReduceMotion(): boolean {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
