/**
 * Thin bridge to the Tauri backend for the shell. Every call goes to a real
 * command or event; outside Tauri (unit tests, plain browser) calls reject and
 * listeners are no-ops, so nothing is ever simulated.
 *
 * Static imports on purpose: concurrent dynamic imports of a mocked module can
 * resolve to the real one under Vitest, and the shell calls several commands at once.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function inTauri(): boolean {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
}

export function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

/** Subscribe to a backend event. Returns an unsubscribe function that is safe to call early. */
export function on<T>(event: string, handler: (payload: T) => void): () => void {
  let unlisten: (() => void) | null = null;
  let stopped = false;
  if (inTauri()) {
    listen<T>(event, (e) => handler(e.payload))
      .then((fn) => {
        if (stopped) fn();
        else unlisten = fn;
      })
      .catch(() => {});
  }
  return () => {
    stopped = true;
    unlisten?.();
  };
}
