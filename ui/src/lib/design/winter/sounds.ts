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
 * - Soft by design: pure sine notes from one pentatonic scale, slow attacks
 *   (no clicks), low-passed noise only (no hiss) and a master low-pass at 4 kHz.
 * - HIG › Motion: the sounds follow the decorative motion they belong to, so
 *   they are not heard when that motion is off; nothing plays while the window
 *   is hidden.
 */
import { season } from '../season.svelte';

export type WinterSound = 'bells' | 'crunch' | 'puff' | 'chime';

/** Master volume (0..1). Kept low: the launcher is not a game. */
export const VOLUME = 0.09;

/** The shortest gap between two sounds of one kind, ms (a drag reports many moves). */
export const MIN_GAP: Record<WinterSound, number> = { bells: 6000, crunch: 380, puff: 80, chime: 600 };

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
    // Nothing sharp reaches the speakers: a gentle low-pass over everything.
    const soften = ctx.createBiquadFilter();
    soften.type = 'lowpass';
    soften.frequency.value = 4000;
    soften.Q.value = 0.5;
    master.connect(soften);
    soften.connect(ctx.destination);
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

  function tone(c: AudioContext, t: number, freq: number, peak: number, decay: number, attack = 0.012) {
    const o = c.createOscillator();
    o.type = 'sine';
    o.frequency.setValueAtTime(freq, t);
    o.connect(envelope(c, t, peak, attack, decay));
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
    f.connect(envelope(c, t, peak, 0.015, dur));
    src.start(t, random() * 0.3);
    src.stop(t + dur + 0.05);
  }

  /** A soft music-box jingle when Santa passes: five notes of E major pentatonic, rising and falling. */
  function bells(c: AudioContext, t0: number) {
    const scale = [1318.5, 1480, 1661.2, 1975.5, 2217.5]; // E6 F#6 G#6 B6 C#7
    const order = [0, 2, 4, 3, 1].map((i) => (random() < 0.3 ? Math.max(0, i - 1) : i));
    order.forEach((n, i) => {
      const t = t0 + i * 0.16;
      tone(c, t, scale[n], 0.22, 0.9);
      tone(c, t, scale[n] * 2, 0.05, 0.35); // a quiet octave for shimmer
    });
  }

  /** Packed snow underfoot: one short, dull, low-passed grain. */
  function crunch(c: AudioContext, t0: number) {
    burst(c, t0, 'lowpass', 900 + random() * 200, 350, 0.5, 0.22, 0.08);
  }

  /** A soft thud of snow: low-passed noise falling in pitch. */
  function puff(c: AudioContext, t0: number) {
    burst(c, t0, 'lowpass', 420, 110, 0.5, 0.45, 0.2);
  }

  /** A small rising triad when a snowman is done (E6, G#6, B6). */
  function chime(c: AudioContext, t0: number) {
    tone(c, t0, 1318.5, 0.2, 1.1);
    tone(c, t0 + 0.08, 1661.2, 0.18, 1.1);
    tone(c, t0 + 0.16, 1975.5, 0.18, 1.3);
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
