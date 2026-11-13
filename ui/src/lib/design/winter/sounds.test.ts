import { afterEach, describe, expect, it, vi } from 'vitest';
import { createWinterSounds, MIN_GAP, VOLUME, playWinter, winterSounds } from './sounds';
import { season } from '../season.svelte';

/** A minimal AudioContext stand-in that records the nodes it makes. */
function mockContext() {
  const param = () => ({ value: 0, setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn(), exponentialRampToValueAtTime: vi.fn() });
  const made = { gain: 0, osc: 0, src: 0, filter: 0, buffers: 0, started: 0 };
  const node = () => ({ connect: vi.fn() });
  const ctx = {
    state: 'running' as AudioContextState,
    currentTime: 1,
    sampleRate: 8000,
    destination: {},
    resume: vi.fn(() => Promise.resolve()),
    close: vi.fn(() => Promise.resolve()),
    createGain: () => (made.gain++, { ...node(), gain: param() }),
    createOscillator: () => (made.osc++, { ...node(), type: 'sine', frequency: param(), start: () => made.started++, stop: vi.fn() }),
    createBufferSource: () => (made.src++, { ...node(), buffer: null, start: () => made.started++, stop: vi.fn() }),
    createBiquadFilter: () => (made.filter++, { ...node(), type: 'lowpass', Q: param(), frequency: param() }),
    createBuffer: (_c: number, len: number) => (made.buffers++, { getChannelData: () => new Float32Array(len) }),
  };
  return { ctx: ctx as unknown as AudioContext, made, raw: ctx };
}

function setup(over: { enabled?: boolean; hidden?: boolean } = {}) {
  const m = mockContext();
  let t = 0;
  const flags = { enabled: over.enabled ?? true, hidden: over.hidden ?? false };
  const makeContext = vi.fn(() => m.ctx);
  const s = createWinterSounds({
    enabled: () => flags.enabled,
    hidden: () => flags.hidden,
    makeContext,
    now: () => t,
    random: () => 0.5,
  });
  return { s, m, flags, makeContext, advance: (ms: number) => (t += ms) };
}

afterEach(() => {
  season.setSounds(false);
});

describe('winter sounds', () => {
  it('play nothing and make no context while the setting is off', () => {
    const { s, makeContext } = setup({ enabled: false });
    for (const k of ['bells', 'crunch', 'puff', 'chime'] as const) expect(s.play(k)).toBe(false);
    expect(makeContext).not.toHaveBeenCalled();
  });

  it('play nothing while the window is hidden', () => {
    const { s, makeContext } = setup({ hidden: true });
    expect(s.play('bells')).toBe(false);
    expect(makeContext).not.toHaveBeenCalled();
  });

  it('synthesize every kind with oscillators and noise, no audio files', () => {
    const { s, m, advance } = setup();
    expect(s.play('bells')).toBe(true);
    expect(m.made.osc).toBe(10);
    expect(m.made.filter).toBe(1); // the master low-pass
    expect(s.play('crunch')).toBe(true);
    expect(m.made.src).toBe(1);
    expect(m.made.filter).toBe(2);
    expect(s.play('puff')).toBe(true);
    expect(m.made.src).toBe(2);
    expect(s.play('chime')).toBe(true);
    expect(m.made.osc).toBe(13);
    expect(m.made.buffers).toBe(1);
    expect(m.made.started).toBe(15);
  });

  it('keep the master volume low', () => {
    expect(VOLUME).toBeGreaterThan(0);
    expect(VOLUME).toBeLessThanOrEqual(0.2);
  });

  it('make one context and reuse it', () => {
    const { s, makeContext, advance } = setup();
    s.play('puff');
    advance(1000);
    s.play('puff');
    expect(makeContext).toHaveBeenCalledTimes(1);
  });

  it('throttle a kind that is reported on every move', () => {
    const { s, advance } = setup();
    expect(s.play('crunch')).toBe(true);
    advance(MIN_GAP.crunch - 1);
    expect(s.play('crunch')).toBe(false);
    advance(1);
    expect(s.play('crunch')).toBe(true);
  });

  it('resume a suspended context', () => {
    const { s, m } = setup();
    m.raw.state = 'suspended';
    s.play('chime');
    expect(m.raw.resume).toHaveBeenCalled();
  });

  it('stop at once when the setting is turned off', () => {
    const { s, flags, advance } = setup();
    expect(s.play('chime')).toBe(true);
    flags.enabled = false;
    advance(10_000);
    expect(s.play('chime')).toBe(false);
  });

  it('survive a missing Web Audio', () => {
    const s = createWinterSounds({ enabled: () => true, hidden: () => false, makeContext: () => null });
    expect(s.play('bells')).toBe(false);
    const t = createWinterSounds({
      enabled: () => true,
      hidden: () => false,
      makeContext: () => {
        throw new Error('no audio');
      },
    });
    expect(t.play('bells')).toBe(false);
  });

  it('the app instance follows season.sounds (off by default)', () => {
    expect(season.sounds).toBe(false);
    expect(playWinter('chime')).toBe(false);
    winterSounds.dispose();
  });
});
