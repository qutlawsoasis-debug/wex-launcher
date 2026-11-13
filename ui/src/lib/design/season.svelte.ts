/**
 * Winter look: snowfall over the window and snow on the build tiles, from
 * November to February while «Зимнее оформление» is on (saved as winter_look).
 * Apple has no guidance on seasonal decoration; it follows HIG › Motion
 * (decorative motion stays off under Reduce Motion, see Snowfall.svelte).
 * «Звуки зимы» (winter_sounds, off by default) gates winter/sounds.ts
 * (HIG › Playing audio: sounds are optional and quiet).
 */

/** November, December, January, February. */
export function isWinterSeason(d: Date = new Date()): boolean {
  const m = d.getMonth();
  return m === 10 || m === 11 || m === 0 || m === 1;
}

class Season {
  /** The setting; on unless the player turned it off. */
  enabled = $state(true);
  /** Overridable for tests. */
  now = $state<() => Date>(() => new Date());

  get winter(): boolean {
    return this.enabled && isWinterSeason(this.now());
  }

  /** «Звуки зимы» (winter_sounds): quiet synthesized sounds, off unless the player turned them on. */
  sounds = $state(false);

  setEnabled(on: boolean): void {
    this.enabled = on;
  }

  setSounds(on: boolean): void {
    this.sounds = on;
  }
}

export const season = new Season();
