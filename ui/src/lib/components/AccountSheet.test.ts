import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';

const ms = vi.hoisted(() => ({
  begin: vi.fn(),
  finish: vi.fn(),
  cancel: vi.fn(),
}));

vi.mock('../shell/accounts.svelte', () => ({
  OFFLINE_NAME_RE: /^[A-Za-z0-9_]{3,16}$/,
  accounts: {
    active: null,
    beginMicrosoft: ms.begin,
    finishMicrosoft: ms.finish,
    cancelMicrosoft: ms.cancel,
    openMicrosoftPage: vi.fn(),
    addOffline: vi.fn(),
  },
}));

import AccountSheet from './AccountSheet.svelte';

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => { resolve = r; });
  return { promise, resolve };
}

const code = { flow_id: 'flow-1', user_code: 'ABCD-1234', verification_uri: 'https://www.microsoft.com/link', expires_in: 900 };

describe('AccountSheet Microsoft sign-in', () => {
  let target: HTMLElement;
  let comp: ReturnType<typeof mount> | null = null;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    vi.clearAllMocks();
    ms.cancel.mockResolvedValue(undefined);
    ms.finish.mockReturnValue(new Promise(() => {}));
  });

  afterEach(() => {
    if (comp) unmount(comp);
    comp = null;
    target.remove();
  });

  function cancelButton(): HTMLButtonElement {
    const btn = [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Отмена');
    if (!btn) throw new Error('no Cancel button');
    return btn as HTMLButtonElement;
  }

  it('cancels the flow at once when the sheet closes while the code is being fetched', async () => {
    const begin = deferred<typeof code>();
    ms.begin.mockReturnValue(begin.promise);
    comp = mount(AccountSheet, { target, props: { open: true, start: 'microsoft' } });
    await tick();
    expect(ms.begin).toHaveBeenCalledTimes(1);

    cancelButton().click();
    await tick();
    begin.resolve(code);
    await Promise.resolve();
    await tick();

    expect(ms.cancel).toHaveBeenCalledWith('flow-1');
    expect(ms.finish).not.toHaveBeenCalled();
  });

  it('waits for the player when the sheet stays open', async () => {
    ms.begin.mockResolvedValue(code);
    comp = mount(AccountSheet, { target, props: { open: true, start: 'microsoft' } });
    await tick();
    await Promise.resolve();
    await tick();

    expect(ms.finish).toHaveBeenCalledWith('flow-1');
    expect(ms.cancel).not.toHaveBeenCalled();
    expect(target.textContent).toContain('ABCD-1234');
  });
});
