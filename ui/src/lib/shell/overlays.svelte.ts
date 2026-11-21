/**
 * How many modal sheets are open right now. A sheet registers while open so
 * window hotkeys (Ctrl 1/2, Alt arrows, Enter) do not act behind it.
 */

import { untrack } from 'svelte';

class Overlays {
  count = $state(0);

  /** Register an open overlay; returns the function that unregisters it. */
  register(): () => void {
    // Read-modify-write on state: keep it out of the caller's effect dependencies.
    untrack(() => this.count++);
    let done = false;
    return () => {
      if (done) return;
      done = true;
      untrack(() => (this.count = Math.max(0, this.count - 1)));
    };
  }
}

export const overlays = new Overlays();
