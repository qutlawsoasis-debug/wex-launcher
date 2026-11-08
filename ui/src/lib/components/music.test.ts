import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import MusicDiskWidget from './MusicDiskWidget.svelte';
import SettingsScreen from '../screens/SettingsScreen.svelte';

describe('MusicDiskWidget and Background Music', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);

    // Mock HTMLMediaElement play/pause
    window.HTMLMediaElement.prototype.play = vi.fn().mockImplementation(() => Promise.resolve());
    window.HTMLMediaElement.prototype.pause = vi.fn();
  });

  afterEach(() => {
    target.remove();
    vi.restoreAllMocks();
  });

  it('renders honest empty state when no tracks exist in folder', async () => {
    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: [],
        initialSettings: { enabled: true, volume: 75, auto_pause_on_launch: true },
      },
    });

    await tick();

    expect(target.textContent).toContain('Нет музыки');
    expect(target.textContent).toContain('Папка Wex/music пуста');
    expect(target.textContent).toContain('Открыть папку');

    const folderBtn = target.querySelector('.open-folder-btn');
    expect(folderBtn).not.toBeNull();

    unmount(comp);
  });

  it('renders disabled state when music is turned off in settings', async () => {
    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: [],
        initialSettings: { enabled: false, volume: 75, auto_pause_on_launch: true },
      },
    });

    await tick();

    expect(target.textContent).toContain('Музыка отключена');
    unmount(comp);
  });

  it('renders active player with track metadata and turntable', async () => {
    const mockTracks = [
      {
        id: 'track-1',
        title: 'Subwoofer Lullaby',
        artist: 'C418',
        file_name: 'C418 - Subwoofer Lullaby.mp3',
        cover_data_url: null,
        duration_seconds: null,
      },
      {
        id: 'track-2',
        title: 'Pigstep',
        artist: 'Lena Raine',
        file_name: 'Lena Raine - Pigstep.mp3',
        cover_data_url: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
        duration_seconds: null,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
        initialSettings: { enabled: true, volume: 80, auto_pause_on_launch: true },
      },
    });

    await tick();

    expect(target.textContent).toContain('Subwoofer Lullaby');
    expect(target.textContent).toContain('C418');

    // Turntable vinyl record exists
    const vinyl = target.querySelector('.vinyl-disc');
    expect(vinyl).not.toBeNull();

    // Transport buttons exist
    const playPauseBtn = target.querySelector('.play-pause-btn') as HTMLButtonElement;
    expect(playPauseBtn).not.toBeNull();

    const forwardBtn = target.querySelector('[aria-label="Следующий трек"]') as HTMLButtonElement;
    expect(forwardBtn).not.toBeNull();

    const backwardBtn = target.querySelector('[aria-label="Предыдущий трек"]') as HTMLButtonElement;
    expect(backwardBtn).not.toBeNull();

    unmount(comp);
  });

  it('navigates to next and previous track in playlist', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Track A',
        artist: 'Artist A',
        file_name: 'track_a.mp3',
        cover_data_url: null,
        duration_seconds: null,
      },
      {
        id: '2',
        title: 'Track B',
        artist: 'Artist B',
        file_name: 'track_b.mp3',
        cover_data_url: null,
        duration_seconds: null,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
      },
    });

    await tick();
    expect(target.textContent).toContain('Track A');

    // Click next track
    const forwardBtn = target.querySelector('[aria-label="Следующий трек"]') as HTMLButtonElement;
    forwardBtn.click();

    await tick();
    expect(target.textContent).toContain('Track B');

    // Click previous track
    const backwardBtn = target.querySelector('[aria-label="Предыдущий трек"]') as HTMLButtonElement;
    backwardBtn.click();

    await tick();
    expect(target.textContent).toContain('Track A');

    unmount(comp);
  });

  it('renders background music setting toggle in SettingsScreen', async () => {
    const comp = mount(SettingsScreen, { target });
    await tick();

    expect(target.textContent).toContain('Фоновая музыка');
    expect(target.textContent).toContain('Воспроизведение саундтрека из папки Wex/music');

    const toggle = target.querySelector('[aria-label="Фоновая музыка"]');
    expect(toggle).not.toBeNull();

    unmount(comp);
  });
});
