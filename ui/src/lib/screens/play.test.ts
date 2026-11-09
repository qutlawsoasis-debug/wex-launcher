import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import PlayScreen from './PlayScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('PlayScreen Component Tests', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  // 1. Play Button & Version Selection
  it('triggers onlaunch callback when Play button is clicked with selected version and nickname', async () => {
    let launched = false;
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        onlaunch: () => {
          launched = true;
        },
      },
    });

    // Verify dynamic Java runtime strictly matches 1.21.1
    expect(target.textContent).toContain('Java 21 • 64-bit');

    const playBtn = target.querySelector('.dock-action button') as HTMLButtonElement;
    expect(playBtn).not.toBeNull();
    expect(playBtn.textContent).toContain('Играть');
    expect(playBtn.disabled).toBe(false);

    playBtn.click();
    await tick();

    expect(launched).toBe(true);

    unmount(comp);
  });

  it('adapts Java runtime dynamically to Java 17 for version 1.20.1', async () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.20.1',
        playerName: 'Alex',
      },
    });

    expect(target.textContent).toContain('Java 17 • 64-bit');
    unmount(comp);
  });

  // 2. Control Locks & Stop Action While Game is Running
  it('locks nickname and version inputs and displays stop action when Minecraft is running', async () => {
    let stopped = false;
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        isRunning: true,
        onstop: () => {
          stopped = true;
        },
      },
    });

    // Nickname input must be disabled
    const nicknameInput = target.querySelector('.nickname-input') as HTMLInputElement;
    expect(nicknameInput.disabled).toBe(true);

    // Button transforms to neutral "В игре • Остановить"
    const stopBtn = target.querySelector('.dock-action button') as HTMLButtonElement;
    expect(stopBtn).not.toBeNull();
    expect(stopBtn.textContent).toContain('В игре');
    expect(stopBtn.textContent).toContain('Остановить');

    // Click triggers onstop callback
    stopBtn.click();
    await tick();

    expect(stopped).toBe(true);

    unmount(comp);
  });

  // 3. Progress Bar & Cancel Action During Launch
  it('renders progress bar and triggers oncancel when Cancel button is clicked during launch', async () => {
    let cancelled = false;
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
        isLaunching: true,
        progressPercentage: 65,
        progressStage: 'Синхронизация сборки Wex',
        progressDetail: 'Загрузка ресурс-паков...',
        oncancel: () => {
          cancelled = true;
        },
      },
    });

    // Progress bar must be visible with stage and detail
    expect(target.textContent).toContain('Синхронизация сборки Wex');
    expect(target.textContent).toContain('Загрузка ресурс-паков...');

    // Cancel button must be present in progress dock
    const cancelBtn = target.querySelector('.dock-cancel button') as HTMLButtonElement;
    expect(cancelBtn).not.toBeNull();
    expect(cancelBtn.textContent).toContain('Отмена');

    cancelBtn.click();
    await tick();

    expect(cancelled).toBe(true);

    unmount(comp);
  });

  // 4. Launch Error State
  it('renders error alert banner with retry action when launchError occurs', async () => {
    let retried = false;
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        launchError: new Error('Сбой загрузки файлов манифеста'),
        onlaunch: () => {
          retried = true;
        },
      },
    });

    // Error banner should be rendered
    const errorBanner = target.querySelector('.launch-error-banner');
    expect(errorBanner).not.toBeNull();

    // Clicking retry button in error state triggers onlaunch
    const retryBtn = errorBanner?.querySelector('.error-actions-row button') as HTMLButtonElement;
    expect(retryBtn).not.toBeNull();
    retryBtn?.click();
    await tick();

    expect(retried).toBe(true);

    unmount(comp);
  });

  // 5. Playtime Display
  it('renders playtime statistics when summary data is provided', async () => {
    const comp = mount(PlayScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Steve',
        initialPlaytime: {
          total_seconds: 7200,
          total_launches: 4,
          formatted_play_time: '2 ч. 0 мин.',
        },
      },
    });

    await tick();
    expect(target.textContent).toContain('2 ч. 0 мин.');

    unmount(comp);
  });
});
