import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
    mocks.handlers.set(name, handler);
    return () => mocks.handlers.delete(name);
  }),
}));

import { router } from './router.svelte';
import { builds } from './builds.svelte';
import { launch } from './launch.svelte';
import { toasts } from './toast.svelte';
import { theme } from '../design/theme.svelte';
import { i18n } from '../i18n';
import LaunchBar from './LaunchBar.svelte';
import Sidebar from './Sidebar.svelte';
import TopLine from './TopLine.svelte';

const versions = [
  { id: '1.21.1', name: 'Minecraft 1.21.1', is_installed: true },
  { id: '1.20.1', name: 'Minecraft 1.20.1', is_installed: true },
];
let settings: Record<string, unknown>;

function emit(name: string, payload: unknown) {
  const h = mocks.handlers.get(name);
  if (!h) throw new Error(`no listener for ${name}`);
  h({ payload });
  flushSync();
}

let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;

beforeEach(async () => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  mocks.handlers.clear();
  mocks.invoke.mockReset();
  settings = { selected_version: '1.21.1', player_name: 'Alice', ram_mb: 6144 };
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'get_launcher_settings':
        return { ...settings };
      case 'save_launcher_settings':
        settings = { ...(args!.newSettings as Record<string, unknown>) };
        return null;
      case 'get_installed_versions':
        return versions;
      case 'list_mods':
        return new Array(args!.version === '1.21.1' ? 132 : 130).fill({});
      case 'shell_get_launch_state':
        return { is_running: false, is_launching: false, running_version: null };
      case 'get_prewarm_status':
        return { is_prewarming: false, is_ready: false, progress: 0, stage: '' };
      case 'start_game':
        return new Promise(() => {}); // the result is reported through events
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  router.reset();
  toasts.dismiss();
  await builds.load();
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  launch.detach();
  target.remove();
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

async function attachLaunch() {
  launch.attach();
  await vi.waitFor(() => expect(mocks.handlers.has('launch-status')).toBe(true));
  await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('get_prewarm_status', { version: '1.21.1' }));
  // Start from idle whatever an earlier test left behind.
  emit('launch-status', { state: 'idle', message: '' });
  toasts.dismiss();
}

describe('router', () => {
  it('keeps back and forward history like a browser', () => {
    router.go('servers');
    router.go('friends');
    expect(router.canBack).toBe(true);
    router.goBack();
    expect(router.route).toBe('servers');
    expect(router.canForward).toBe(true);
    router.goForward();
    expect(router.route).toBe('friends');
    router.go('wardrobe');
    expect(router.canForward).toBe(false);
  });

  it('writes the page to the hash and leaves home clean', () => {
    router.go('build-1.20.1');
    expect(location.hash).toBe('#build-1.20.1');
    router.go('home');
    expect(location.hash).toBe('');
  });
});

describe('builds', () => {
  it('loads both builds, mod counts and the selected one from the backend', () => {
    expect(builds.list.map((b) => b.id)).toEqual(['1.21.1', '1.20.1']);
    expect(builds.list.map((b) => b.mods)).toEqual([132, 130]);
    expect(builds.selectedId).toBe('1.21.1');
    expect(builds.playerName).toBe('Alice');
  });

  it('saves the selected build into the launcher settings without losing other fields', async () => {
    await builds.select('1.20.1');
    expect(builds.selectedId).toBe('1.20.1');
    expect(settings).toMatchObject({ selected_version: '1.20.1', player_name: 'Alice', ram_mb: 6144 });
  });

  it('rolls back the selection if saving fails', async () => {
    mocks.invoke.mockImplementationOnce(async () => {
      throw new Error('disk');
    });
    await expect(builds.select('1.20.1')).rejects.toThrow('disk');
    expect(builds.selectedId).toBe('1.21.1');
  });
});

describe('launch bar', () => {
  it('ready state: build, memory and Java from the backend, blue Play', async () => {
    await attachLaunch();
    comp = mount(LaunchBar, { target });
    await tick();
    expect(target.querySelector('.title')?.textContent).toContain('Wex 1.21.1');
    expect(target.querySelector('.sub')?.textContent).toBe('Игра скачана, можно играть, 6 ГБ памяти');
    const play = target.querySelector<HTMLButtonElement>('.play')!;
    expect(play.textContent).toContain('Играть');
    expect(play.getAttribute('aria-label')).toBe('Играть в Wex 1.21.1');
    expect(play.classList.contains('cancel') || play.classList.contains('stop')).toBe(false);
  });

  it('a build that is not downloaded says what Play will do; Play stays enabled', async () => {
    await attachLaunch();
    builds.list = builds.list.map((b) => ({ ...b, installed: false }));
    comp = mount(LaunchBar, { target });
    await tick();
    expect(target.querySelector('.sub')?.textContent).toContain('Игра ещё не скачана — нажмите «Играть», чтобы скачать');
    expect(target.querySelector<HTMLButtonElement>('.play')!.disabled).toBe(false);
  });

  it('no builds from core: the reason is shown and the button retries instead of standing disabled', async () => {
    await attachLaunch();
    builds.list = [];
    comp = mount(LaunchBar, { target });
    await tick();
    expect(target.querySelector('.sub')?.textContent).toBe('Не удалось получить сборки — нажмите «Повторить»');
    const btn = target.querySelector<HTMLButtonElement>('.play')!;
    expect(btn.disabled).toBe(false);
    expect(btn.textContent).toContain('Повторить');
    btn.click();
    await vi.waitFor(() => expect(builds.list.length).toBe(2));
    expect(target.querySelector('.play')?.textContent).toContain('Играть');
  });

  it('Play calls start_game for the selected build and shows backend stages with progress', async () => {
    await attachLaunch();
    comp = mount(LaunchBar, { target });
    target.querySelector<HTMLButtonElement>('.play')!.click();
    await vi.waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('start_game', { version: '1.21.1', nickname: 'Alice', jvmArgs: null, server: null })
    );
    flushSync();
    expect(target.querySelector('.play')?.textContent).toContain('Отменить');
    expect(target.querySelector('.track')).not.toBeNull();

    emit('launch-status', { state: 'installing', message: 'Скачиваем ресурсы игры' });
    emit('install-progress', { stage: 'Скачиваем ресурсы игры', percentage: 42.4, downloaded_bytes: 50 * 1048576, total_bytes: 120 * 1048576 });
    expect(target.querySelector('.title')?.textContent).toContain('Скачиваем ресурсы игры');
    expect(target.querySelector('.sub')?.textContent).toBe('50 МБ из 120 МБ, 42%');
    expect(target.querySelector('.track')?.getAttribute('aria-valuenow')).toBe('42');
    // No invented step numbers: core does not report them.
    expect(target.textContent).not.toMatch(/шаг \d/);
  });

  it('Cancel calls cancel_launch; the cancelled event returns to ready with a notice', async () => {
    await attachLaunch();
    comp = mount(LaunchBar, { target });
    target.querySelector<HTMLButtonElement>('.play')!.click();
    await vi.waitFor(() => expect(launch.phase).toBe('launching'));
    flushSync();
    target.querySelector<HTMLButtonElement>('.play')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('cancel_launch', undefined));
    emit('launch-status', { state: 'cancelled', message: 'Запуск отменён' });
    expect(launch.phase).toBe('idle');
    expect(toasts.current?.title).toBe('Запуск отменён');
    expect(target.querySelector('.play')?.textContent).toContain('Играть');
  });

  it('running: green dot, timer, red Stop that calls shell_stop_game; stopping is not a crash', async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    try {
      await attachLaunch();
      comp = mount(LaunchBar, { target });
      emit('launch-status', { state: 'running', message: 'Minecraft запущен' });
      expect(target.querySelector('.title')?.textContent).toContain('Игра идёт');
      expect(target.querySelector('.play')?.classList.contains('stop')).toBe(true);
      vi.advanceTimersByTime(65_000);
      flushSync();
      expect(target.querySelector('.sub')?.textContent).toBe('Wex 1.21.1, 01:05');
      target.querySelector<HTMLButtonElement>('.play')!.click();
      await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('shell_stop_game', { version: '1.21.1' }));
      emit('launch-status', { state: 'idle', message: 'Игра остановлена' });
      expect(toasts.current?.title).toBe('Игра остановлена');
      expect(launch.phase).toBe('idle');
    } finally {
      vi.useRealTimers();
    }
  });

  it('a failed launch returns to ready and explains the error once', async () => {
    await attachLaunch();
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'start_game') throw new Error('Java не найдена');
      return null;
    });
    await launch.start();
    expect(launch.phase).toBe('idle');
    expect(toasts.current).not.toBeNull();
    const first = toasts.current!.id;
    emit('launch-status', { state: 'error', message: 'Java не найдена' });
    expect(toasts.current!.id).toBe(first);
  });

  it('background preparation progress does not take over the bar', async () => {
    await attachLaunch();
    comp = mount(LaunchBar, { target });
    emit('install-progress', { stage: 'Фоновая подготовка клиента', percentage: 30 });
    expect(launch.phase).toBe('idle');
    expect(target.querySelector('.track')).toBeNull();
    emit('prewarm-status', { state: 'ready', progress: 100 });
    expect(target.querySelector('.sub')?.textContent).toBe('Готово к быстрому запуску, 6 ГБ памяти');
  });
});

describe('sidebar', () => {
  it('shows the five sections, both builds with real mod counts and marks the selected one', async () => {
    comp = mount(Sidebar, { target, props: { online: true } });
    await tick();
    const nav = [...target.querySelectorAll('.nav a')].map((a) => a.textContent?.trim());
    expect(nav).toEqual(['Главная', 'Серверы', 'Друзья', 'Гардероб', 'Настройки']);
    expect(target.querySelector('.nav a[aria-current="page"]')?.textContent).toContain('Главная');
    const lib = [...target.querySelectorAll('.lib a')];
    expect(lib.map((a) => a.querySelector('.lib-t b')?.textContent)).toEqual(['Wex 1.21.1', 'Wex 1.20.1']);
    expect(lib[0].textContent).toContain('132 мода');
    expect(lib[1].textContent).toContain('130 модов');
    expect(lib[0].querySelector('.sel')?.getAttribute('aria-label')).toBe('Выбрана для запуска');
    expect(lib[1].querySelector('.sel')).toBeNull();
    // Creating builds is not in core, so there is no "+" button.
    expect(target.querySelector('.side-h button')).toBeNull();
    expect(target.querySelector('.me')?.textContent).toContain('Alice');
    expect(target.querySelector('.me')?.textContent).toContain('В сети');
  });

  it('navigates with the router and moves the selection', async () => {
    comp = mount(Sidebar, { target, props: { online: false } });
    await tick();
    (target.querySelectorAll('.nav a')[1] as HTMLAnchorElement).click();
    flushSync();
    expect(router.route).toBe('servers');
    expect(target.querySelector('.nav a[aria-current="page"]')?.textContent).toContain('Серверы');
    (target.querySelectorAll('.lib a')[1] as HTMLAnchorElement).click();
    flushSync();
    expect(router.route).toBe('build-1.20.1');
    expect(target.querySelector('.lib a[aria-current="page"]')?.textContent).toContain('1.20.1');
    expect(target.querySelector('.me')?.textContent).toContain('Не в сети');
  });

  it('the selection indicator is a fill that jumps instantly under Reduce Motion', async () => {
    theme.setReduceMotion(true);
    try {
      comp = mount(Sidebar, { target, props: { online: true } });
      await tick();
      await tick();
      const ind = target.querySelector<HTMLElement>('.nav > .ind');
      expect(ind).not.toBeNull();
      expect(ind!.getAttribute('aria-hidden')).toBe('true');
      expect(document.documentElement.dataset.reduceMotion).toBe('true');
    } finally {
      theme.setReduceMotion(false);
    }
  });
});

describe('top line', () => {
  it('window buttons call the backend; back is disabled until there is history', async () => {
    comp = mount(TopLine, { target });
    await tick();
    const [back, fwd] = target.querySelectorAll<HTMLButtonElement>('.crumbs button');
    expect(back.disabled).toBe(true);
    expect(fwd.disabled).toBe(true);
    router.go('options');
    flushSync();
    expect(back.disabled).toBe(false);
    back.click();
    expect(router.route).toBe('home');

    mocks.invoke.mockResolvedValueOnce(null).mockResolvedValueOnce(true).mockResolvedValueOnce(null);
    const [min, max, close] = target.querySelectorAll<HTMLButtonElement>('.winctl button');
    min.click();
    max.click();
    close.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('shell_close_window', undefined));
    expect(mocks.invoke).toHaveBeenCalledWith('shell_minimize_window', undefined);
    expect(mocks.invoke).toHaveBeenCalledWith('shell_toggle_maximize_window', undefined);
    await vi.waitFor(() => expect(max.getAttribute('aria-label')).toBe('Восстановить окно'));
    expect(target.querySelector('.topline')?.hasAttribute('data-tauri-drag-region')).toBe(true);
  });
});
