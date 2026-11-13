/**
 * Quiet winter sounds, synthesized with the Web Audio API (no audio files):
 * sleigh bells when Santa passes, a soft crunch when a snowball is rolled or
 * thrown, a puff when it splats and a tiny chime when a snowman is done.
 *
 * Apple has no guidance on seasonal decoration. The rules applied:
 * - HIG › Playing audio: sounds are optional and quiet, and never carry
 *   information alone. They play only while «Звуки зимы» (winter_sounds) is on,
 *   which is off by default; every sound goes with something visible (the
 *   sleigh, the ball, the splat, the sparkle and toast), and the volume is low.
 * - HIG › Motion: the sounds follow the decorative motion they belong to, so
 *   they are not heard when that motion is off; nothing plays while the window
 *   is hidden.
 */
import { season } from '../season.svelte';

export type WinterSound = 'bells' | 'crunch' | 'puff' | 'chime';

/** Master volume (0..1). Kept low: the launcher is not a game. */
export const VOLUME = 0.12;

/** The shortest gap between two sounds of one kind, ms (a drag reports many moves). */
export const MIN_GAP: Record<WinterSound, number> = { bells: 3000, crunch: 140, puff: 60, chime: 400 };

interface Deps {
  /** The setting; checked on every play. */
  enabled: () => boolean;
  /** True while the window is hidden. */
  hidden: () => boolean;
  /** Creates the audio context on first use (null where Web Audio is missing). */
  makeContext: () => AudioContext | null;
  now?: () => number;
  random?: () => number;
}

export interface WinterSounds {
  /** Plays a sound if the setting allows it; returns whether it played. */
  play(kind: WinterSound): boolean;
  /** Closes the audio context (tests, teardown). */
  dispose(): void;
}

export function createWinterSounds(deps: Deps): WinterSounds {
  const now = deps.now ?? (() => performance.now());
  const random = deps.random ?? Math.random;
  let ctx: AudioContext | null = null;
  let master: GainNode | null = null;
  let noise: AudioBuffer | null = null;
  const last: Partial<Record<WinterSound, number>> = {};

  function context(): AudioContext | null {
    if (ctx) return ctx;
    try {
      ctx = deps.makeContext();
    } catch {
      ctx = null;
    }
    if (!ctx) return null;
    master = ctx.createGain();
    master.gain.value = VOLUME;
    master.connect(ctx.destination);
    return ctx;
  }

  function noiseBuffer(c: AudioContext): AudioBuffer {
    if (noise) return noise;
    const len = Math.floor(c.sampleRate * 0.5);
    noise = c.createBuffer(1, len, c.sampleRate);
    const data = noise.getChannelData(0);
    for (let i = 0; i < len; i++) data[i] = random() * 2 - 1;
    return noise;
  }

  /** A gain node with a fast attack and an exponential fall. */
  function envelope(c: AudioContext, t: number, peak: number, attack: number, decay: number): GainNode {
    const g = c.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.linearRampToValueAtTime(peak, t + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t + attack + decay);
    g.connect(master!);
    return g;
  }

  function tone(c: AudioContext, t: number, freq: number, peak: number, decay: number, type: OscillatorType = 'sine') {
    const o = c.createOscillator();
    o.type = type;
    o.frequency.setValueAtTime(freq, t);
    o.connect(envelope(c, t, peak, 0.004, decay));
    o.start(t);
    o.stop(t + decay + 0.05);
  }

  function burst(c: AudioContext, t: number, filter: BiquadFilterType, freq: number, endFreq: number, q: number, peak: number, dur: number) {
    const src = c.createBufferSource();
    src.buffer = noiseBuffer(c);
    const f = c.createBiquadFilter();
    f.type = filter;
    f.Q.value = q;
    f.frequency.setValueAtTime(freq, t);
    f.frequency.exponentialRampToValueAtTime(endFreq, t + dur);
    src.connect(f);
    f.connect(envelope(c, t, peak, 0.006, dur));
    src.start(t, random() * 0.3);
    src.stop(t + dur + 0.05);
  }

  /** Sleigh bells: a dozen small jingles, each a bright partial and an inharmonic one. */
  function bells(c: AudioContext, t0: number) {
    for (let i = 0; i < 12; i++) {
      const t = t0 + i * 0.11 + random() * 0.04;
      const f = 2400 + random() * 900;
      const p = 0.18 + random() * 0.12;
      tone(c, t, f, p, 0.22, 'triangle');
      tone(c, t, f * 2.76, p * 0.35, 0.12);
    }
  }

  /** Packed snow: three short, filtered noise grains. */
  function crunch(c: AudioContext, t0: number) {
    for (let i = 0; i < 3; i++) burst(c, t0 + i * 0.035 + random() * 0.015, 'bandpass', 1500 + random() * 800, 700, 1.2, 0.5, 0.05);
  }

  /** A soft thud of snow: low-passed noise falling in pitch. */
  function puff(c: AudioContext, t0: number) {
    burst(c, t0, 'lowpass', 1100, 220, 0.7, 0.7, 0.22);
  }

  /** Two small glassy notes, a fifth apart (G6, D7). */
  function chime(c: AudioContext, t0: number) {
    tone(c, t0, 1568, 0.3, 0.7);
    tone(c, t0 + 0.09, 2349.3, 0.22, 0.8);
  }

  const synth: Record<WinterSound, (c: AudioContext, t: number) => void> = { bells, crunch, puff, chime };

  return {
    play(kind) {
      if (!deps.enabled() || deps.hidden()) return false;
      const at = now();
      const prev = last[kind];
      if (prev !== undefined && at - prev < MIN_GAP[kind]) return false;
      const c = context();
      if (!c) return false;
      last[kind] = at;
      try {
        if (c.state === 'suspended') void c.resume().catch(() => {});
        synth[kind](c, c.currentTime + 0.01);
        return true;
      } catch {
        return false;
      }
    },
    dispose() {
      const c = ctx;
      ctx = null;
      master = null;
      noise = null;
      void c?.close?.().catch(() => {});
    },
  };
}

/** The app's sounds: on only while «Звуки зимы» is on. */
export const winterSounds = createWinterSounds({
  enabled: () => season.sounds,
  hidden: () => typeof document !== 'undefined' && document.hidden,
  makeContext: () => {
    const AC = typeof window !== 'undefined' ? (window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext) : undefined;
    return AC ? new AC() : null;
  },
});

/** Shorthand for the hooks in SnowPlay, Santa and Snowballs. */
export function playWinter(kind: WinterSound): boolean {
  return winterSounds.play(kind);
}
