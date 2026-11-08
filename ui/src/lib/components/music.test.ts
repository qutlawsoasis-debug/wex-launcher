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

    // Mock MediaMetadata and navigator.mediaSession for Windows SMTC
    class MockMediaMetadata {
      title: string;
      artist: string;
      album: string;
      artwork: any[];
      constructor(init?: any) {
        this.title = init?.title || '';
        this.artist = init?.artist || '';
        this.album = init?.album || '';
        this.artwork = init?.artwork || [];
      }
    }
    (window as any).MediaMetadata = MockMediaMetadata;

    const handlers = new Map<string, Function>();
    (navigator as any).mediaSession = {
      metadata: null,
      playbackState: 'none',
      handlers,
      setActionHandler: vi.fn((action: string, handler: Function | null) => {
        if (handler) handlers.set(action, handler);
        else handlers.delete(action);
      }),
      setPositionState: vi.fn(),
    };
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

  it('synchronizes track metadata and transport controls with Windows SMTC (navigator.mediaSession)', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Wet Hands',
        artist: 'C418',
        file_name: 'C418 - Wet Hands.mp3',
        cover_data_url: 'data:image/png;base64,mockcover',
        duration_seconds: 130,
      },
      {
        id: '2',
        title: 'Mice on Venus',
        artist: 'C418',
        file_name: 'C418 - Mice on Venus.mp3',
        cover_data_url: null,
        duration_seconds: 281,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
      },
    });

    await tick();

    // Verify SMTC metadata is set on navigator.mediaSession
    expect((navigator as any).mediaSession.metadata).not.toBeNull();
    expect((navigator as any).mediaSession.metadata.title).toBe('Wet Hands');
    expect((navigator as any).mediaSession.metadata.artist).toBe('C418');
    expect((navigator as any).mediaSession.metadata.album).toBe('Wex Music');
    expect((navigator as any).mediaSession.metadata.artwork).toHaveLength(1);
    expect((navigator as any).mediaSession.metadata.artwork[0].src).toBe('data:image/png;base64,mockcover');

    // Verify SMTC action handlers are registered
    const handlers = (navigator as any).mediaSession.handlers;
    expect(handlers.has('play')).toBe(true);
    expect(handlers.has('pause')).toBe(true);
    expect(handlers.has('nexttrack')).toBe(true);
    expect(handlers.has('previoustrack')).toBe(true);

    // Trigger nexttrack via SMTC handler
    const nextHandler = handlers.get('nexttrack');
    nextHandler();
    await tick();

    expect((navigator as any).mediaSession.metadata.title).toBe('Mice on Venus');

    unmount(comp);
  });

  it('smoothly fades out volume when game launches and strictly does not auto-resume on game exit', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Sweden',
        artist: 'C418',
        file_name: 'C418 - Sweden.mp3',
        cover_data_url: null,
        duration_seconds: 215,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
        isGameRunning: false,
      },
    });

    await tick();

    // Set audio src so playback can start without backend IPC fetch in unit test
    const audio = target.querySelector('audio') as HTMLAudioElement;
    if (audio) {
      audio.src = 'data:audio/mp3;base64,mockaudio';
    }

    // Start playback
    (comp as any).play();
    await tick();
    expect((comp as any).getIsPlaying()).toBe(true);

    // Call fadeOutAndPause with short duration for test (50ms)
    const fadePromise = (comp as any).fadeOutAndPause(50);
    expect((comp as any).getIsFadingOut()).toBe(true);

    await fadePromise;
    expect((comp as any).getIsFadingOut()).toBe(false);
    expect((comp as any).getIsPlaying()).toBe(false);
    expect(window.HTMLMediaElement.prototype.pause).toHaveBeenCalled();

    // Verify after game exits, playback stays paused (does not auto-resume)
    await tick();
    expect((comp as any).getIsPlaying()).toBe(false);

    unmount(comp);
  });
});
