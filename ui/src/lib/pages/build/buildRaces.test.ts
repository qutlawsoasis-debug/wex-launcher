import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { build } from './build.svelte';

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  mocks.invoke.mockReset();
  build.version = '';
  build.open('1.21.1');
});

describe('build page actions survive switching builds', () => {
  it('addMods keeps targeting the build it started on and does not touch the other build', async () => {
    const gate = deferred<{ file_name: string }>();
    const calls: Array<Record<string, unknown>> = [];
    mocks.invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd !== 'add_custom_mod') return null;
      calls.push(args);
      return calls.length === 1 ? gate.promise : { file_name: 'b.jar' };
    });
    build.mods = [];
    const run = build.addMods(['a.jar', 'b.jar']);
    build.open('1.20.1');
    build.mods = [];
    gate.resolve({ file_name: 'a.jar' });
    await run;
    expect(calls.map((c) => c.version)).toEqual(['1.21.1', '1.21.1']);
    expect(build.mods).toEqual([]);
  });

  it('toggleMod does not write the result into another build', async () => {
    const gate = deferred<{ file_name: string; enabled: boolean }>();
    mocks.invoke.mockImplementation(async (cmd: string) => (cmd === 'toggle_mod' ? gate.promise : null));
    build.mods = [{ file_name: 'x.jar', enabled: true } as never];
    const run = build.toggleMod({ file_name: 'x.jar', enabled: true } as never);
    build.open('1.20.1');
    build.mods = [{ file_name: 'x.jar', enabled: true } as never];
    gate.resolve({ file_name: 'x.jar', enabled: false });
    await run;
    expect(build.mods?.[0].enabled).toBe(true);
  });
});
