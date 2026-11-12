import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), pick: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock('../../shell/filePick', () => ({ pickImageFile: mocks.pick, pickFiles: vi.fn(async () => []) }));

import { boxUV, skinParts, CAPE_UV, buildRig, isSkinSize, isCapeSize, NICK_RE } from './skinRig';
import { wardrobe, safeName } from './wardrobe.svelte';
import { builds } from '../../shell/builds.svelte';
import { toasts } from '../../shell/toast.svelte';
import { i18n } from '../../i18n';
import WardrobePage from './WardrobePage.svelte';

describe('skin layout', () => {
  it('cuts boxes by the standard Minecraft unwrap', () => {
    expect(boxUV(0, 0, 8, 8, 8)).toEqual({
      top: [8, 0, 8, 8],
      bottom: [16, 0, 8, 8],
      right: [0, 8, 8, 8],
      front: [8, 8, 8, 8],
      left: [16, 8, 8, 8],
      back: [24, 8, 8, 8],
    });
  });

  it('has base and outer layers at the 64×64 positions, wide and slim arms', () => {
    const wide = Object.fromEntries(skinParts(false).map((p) => [p.name, p]));
    expect(wide.head.overlay.front).toEqual([40, 8, 8, 8]); // hat
    expect(wide.body.base.front).toEqual([20, 20, 8, 12]);
    expect(wide.body.overlay.front).toEqual([20, 36, 8, 12]); // jacket
    expect(wide.armR.base.front).toEqual([44, 20, 4, 12]);
    expect(wide.armL.base.front).toEqual([36, 52, 4, 12]);
    expect(wide.armL.overlay.front).toEqual([52, 52, 4, 12]); // left sleeve
    expect(wide.legR.base.front).toEqual([4, 20, 4, 12]);
    expect(wide.legL.base.front).toEqual([20, 52, 4, 12]);
    expect(wide.legL.overlay.front).toEqual([4, 52, 4, 12]); // left pants
    const slim = Object.fromEntries(skinParts(true).map((p) => [p.name, p]));
    expect(slim.armR.size[0]).toBe(3);
    expect(slim.armR.base.front).toEqual([44, 20, 3, 12]);
    expect(slim.armR.base.back).toEqual([51, 20, 3, 12]);
    expect(slim.armL.base.front).toEqual([36, 52, 3, 12]);
  });

  it('cape: outer side seen from behind, inner side towards the body', () => {
    expect(CAPE_UV.back).toEqual([1, 1, 10, 16]);
    expect(CAPE_UV.front).toEqual([12, 1, 10, 16]);
  });

  it('builds head, body, arms, legs with outer layers, and a cape only when there is one', () => {
    const rig = document.createElement('div');
    const tex = document.createElement('canvas');
    buildRig(rig, tex, false, null);
    const joints = [...rig.children].map((j) => j.className);
    expect(joints).toEqual(['joint j-head', 'joint j-body', 'joint j-armR', 'joint j-armL', 'joint j-legR', 'joint j-legL']);
    expect(rig.querySelectorAll('.j-head .box').length).toBe(2);
    expect(rig.querySelector('.j-head .hat')).not.toBeNull();
    expect(rig.querySelectorAll('.face').length).toBe(6 * 2 * 6);
    // Wide arms sit 6 px from the centre, slim 5.5 px.
    expect((rig.querySelector('.j-armR') as HTMLElement).style.transform).toBe('translate3d(-66px, -88px, 0px)');
    buildRig(rig, tex, true, { canvas: tex, scale: 1 });
    expect((rig.querySelector('.j-armR') as HTMLElement).style.transform).toBe('translate3d(-60.5px, -88px, 0px)');
    expect(rig.querySelector('.j-cape .cape')).not.toBeNull();
  });

  it('checks sizes and nicknames', () => {
    expect(isSkinSize(64, 64)).toBe(true);
    expect(isSkinSize(64, 32)).toBe(true);
    expect(isSkinSize(128, 128)).toBe(false);
    expect(isCapeSize(64, 32)).toBe(true);
    expect(isCapeSize(128, 64)).toBe(true);
    expect(isCapeSize(64, 64)).toBe(false);
    expect(NICK_RE.test('Player_1')).toBe(true);
    expect(NICK_RE.test('ab')).toBe(false);
    expect(NICK_RE.test('Имя')).toBe(false);
    expect(safeName('../my skin!.png')).toBe('my skin');
  });
});

let state: Record<string, unknown>;
let settings: Record<string, unknown>;

beforeEach(async () => {
  mocks.invoke.mockReset();
  mocks.pick.mockReset();
  state = { nickname: 'Player', skin_name: '', skin_base64: null, model: 'Classic', cape_id: 'none', cape_name: '', cape_base64: null };
  settings = { selected_version: '1.21.1', player_name: 'Player', ram_mb: 6144 };
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, any>) => {
    switch (cmd) {
      case 'wardrobe_get_state':
        return { ...state };
      case 'wardrobe_apply':
        state = { ...state, ...args!.request };
        return { success: true, message: 'ok' };
      case 'extras_list_skin_history':
        return { items: [{ id: 'h1', name: 'Лис', model: 'slim' }] };
      case 'wardrobe_get_skin':
        return { item: { id: 'h1' }, skin_bytes_base64: 'SAVEDSKIN' };
      case 'get_launcher_settings':
        return { ...settings };
      case 'save_launcher_settings':
        settings = { ...args!.newSettings };
        return null;
      case 'get_installed_versions':
        return [];
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  toasts.dismiss();
  await builds.load();
});

let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  target = document.createElement('div');
  target.className = 'wx';
  document.body.appendChild(target);
});
afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  target.remove();
});

async function open() {
  comp = mount(WardrobePage, { target });
  await vi.waitFor(() => expect(target.querySelectorAll('.skin-card').length).toBe(2));
  await tick();
}

describe('wardrobe page', () => {
  it('shows the stage with nickname, poses, turn tools and the current skin', async () => {
    await open();
    expect(target.querySelector('.nametag')?.textContent).toBe('Player');
    expect([...target.querySelectorAll('.stage-bar .seg button')].map((b) => b.textContent)).toEqual(['Стоит', 'Идёт', 'Бежит', 'Машет']);
    expect(target.querySelectorAll('.icon-b').length).toBe(3);
    expect(target.querySelector('.stage-top b')?.textContent).toBe('Стандартный скин');
    expect([...target.querySelectorAll('.wd-panel [role="tab"]')].map((b) => b.textContent)).toEqual(['Скины', 'Плащи', 'Профиль']);
  });

  it('poses switch the rig class; buttons and arrow keys turn the model', async () => {
    await open();
    const rig = target.querySelector<HTMLElement>('.rig')!;
    expect(rig.classList.contains('pose-idle')).toBe(true);
    [...target.querySelectorAll<HTMLButtonElement>('.stage-bar .seg button')][1].click();
    flushSync();
    expect(rig.classList.contains('pose-walk')).toBe(true);
    target.querySelectorAll<HTMLButtonElement>('.icon-b')[1].click();
    flushSync();
    expect(rig.style.getPropertyValue('--turn')).toBe('66deg');
    target.querySelector<HTMLElement>('.stage3d')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    flushSync();
    expect(rig.style.getPropertyValue('--turn')).toBe('51deg');
    const auto = target.querySelectorAll<HTMLButtonElement>('.icon-b')[2];
    auto.click();
    flushSync();
    expect(auto.getAttribute('aria-pressed')).toBe('true');
    auto.click();
  });

  it('a saved skin is put on through wardrobe_apply with its model', async () => {
    await open();
    const saved = [...target.querySelectorAll<HTMLButtonElement>('.skin-card')].find((c) => c.textContent?.includes('Лис'))!;
    saved.click();
    await vi.waitFor(() => expect(state.skin_base64).toBe('SAVEDSKIN'));
    expect(state.model).toBe('Slim');
    expect(state.skin_name).toBe('Лис');
    await vi.waitFor(() => expect(toasts.current?.title).toBe('Скин надет'));
  });

  it('arm model switches through core', async () => {
    await open();
    [...target.querySelectorAll<HTMLButtonElement>('.wrow .seg button')].find((b) => b.textContent === 'Узкая')!.click();
    await vi.waitFor(() => expect(state.model).toBe('Slim'));
  });

  it('upload: «Загрузить скин» opens the file chooser; a cancelled choice changes nothing', async () => {
    mocks.pick.mockResolvedValueOnce(null);
    await open();
    target.querySelector<HTMLButtonElement>('.drop')!.click();
    await vi.waitFor(() => expect(mocks.pick).toHaveBeenCalled());
    expect(mocks.invoke).not.toHaveBeenCalledWith('wardrobe_apply', expect.anything());
  });

  it('cape: no preset capes, only your own or none', async () => {
    await open();
    (target.querySelectorAll<HTMLButtonElement>('.wd-panel [role="tab"]')[1]).click();
    flushSync();
    const cards = [...target.querySelectorAll('.cape-card b')].map((b) => b.textContent);
    expect(cards).toEqual(['Свой', 'Без плаща']);
  });

  it('nickname: invalid input is explained and not saved; a valid one goes to the wardrobe and the launcher settings', async () => {
    await open();
    (target.querySelectorAll<HTMLButtonElement>('.wd-panel [role="tab"]')[2]).click();
    flushSync();
    const input = target.querySelector<HTMLInputElement>('#wd-nick')!;
    input.value = 'ab';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector('#wd-nick-hint')?.textContent).toContain('Нужно 3–16 символов');
    expect(input.getAttribute('aria-invalid')).toBe('true');
    input.value = 'Steve_42';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector('.nametag')?.textContent).toBe('Steve_42');
    await vi.waitFor(() => expect(state.nickname).toBe('Steve_42'), { timeout: 2000 });
    await vi.waitFor(() => expect(settings.player_name).toBe('Steve_42'));
  });
});

void wardrobe;
