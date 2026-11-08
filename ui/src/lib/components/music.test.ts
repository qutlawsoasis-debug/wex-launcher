import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import MusicDiskWidget from './MusicDiskWidget.svelte';
import SettingsScreen from '../screens/SettingsScreen.svelte';

vi.mock('@tauri-apps/api/core', () => {
  return {
    invoke: vi.fn(async (cmd: string, args?: Record<string, any>) => {
      if (cmd === 'music_get_track_data') {
        return {
          file_name: args?.request?.file_name || 'track.mp3',
          mime_type: 'audio/mpeg',
          audio_base64: 'bW9jayBhdWRpbyBieXRlcw==',
        };
      }
      if (cmd === 'music_sync_smtc' || cmd === 'music_update_session') {
        return;
      }
      if (cmd === 'music_check_command') {
        return { command: null };
      }
      if (cmd === 'music_get_settings') {
        return { enabled: true, volume: 75, auto_pause_on_launch: true };
      }
      if (cmd === 'music_set_settings') {
        return;
      }
      if (cmd === 'music_get_playlist') {
        return { tracks: [], music_dir: 'C:/Users/User/AppData/Roaming/Wex/music' };
      }
      return null;
    }),
  };
});

describe('MusicDiskWidget and Background Music', () => {
  let target: HTMLElement;

  let mockActionHandlers = new Map<string, Function>();
  let mockMediaSession: {
    metadata: any;
    playbackState: string;
    handlers: Map<string, Function>;
    setActionHandler: ReturnType<typeof vi.fn>;
    setPositionState: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);

    // Mock HTMLMediaElement play/pause
    window.HTMLMediaElement.prototype.play = vi.fn().mockImplementation(() => Promise.resolve());
    window.HTMLMediaElement.prototype.pause = vi.fn();

    // Mock MediaSession
    mockActionHandlers = new Map<string, Function>();
    mockMediaSession = {
      metadata: null as any,
      playbackState: 'none' as string,
      handlers: mockActionHandlers,
      setActionHandler: vi.fn((action: string, handler: Function | null) => {
        if (handler) {
          mockActionHandlers.set(action, handler);
        } else {
          mockActionHandlers.delete(action);
        }
      }),
      setPositionState: vi.fn(),
    };

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
    (globalThis as any).MediaMetadata = MockMediaMetadata;

    Object.defineProperty(navigator, 'mediaSession', {
      value: mockMediaSession,
      writable: true,
      configurable: true,
    });
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

  it('registers Windows SMTC metadata and updates title/artist/artwork', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Subwoofer Lullaby',
        artist: 'C418',
        file_name: 'C418 - Subwoofer Lullaby.mp3',
        cover_data_url: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
        duration_seconds: 210n,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
      },
    });

    await tick();

    // Verify SMTC metadata registered
    expect(mockMediaSession.metadata).not.toBeNull();
    expect(mockMediaSession.metadata.title).toBe('Subwoofer Lullaby');
    expect(mockMediaSession.metadata.artist).toBe('C418');
    expect(mockMediaSession.metadata.album).toBe('Wex Launcher');
    expect(mockMediaSession.metadata.artwork).toHaveLength(1);
    expect(mockMediaSession.metadata.artwork[0].src).toContain('data:image/png;base64');

    // Action handlers registered for Windows media flyout
    expect(mockActionHandlers.has('play')).toBe(true);
    expect(mockActionHandlers.has('pause')).toBe(true);
    expect(mockActionHandlers.has('nexttrack')).toBe(true);
    expect(mockActionHandlers.has('previoustrack')).toBe(true);

    unmount(comp);
  });

  it('handles SMTC action handlers and hardware media keys', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Track One',
        artist: 'Artist One',
        file_name: 'track1.mp3',
        cover_data_url: null,
        duration_seconds: 180n,
      },
      {
        id: '2',
        title: 'Track Two',
        artist: 'Artist Two',
        file_name: 'track2.mp3',
        cover_data_url: null,
        duration_seconds: 200n,
      },
    ];

    const comp = mount(MusicDiskWidget, {
      target,
      props: {
        initialTracks: mockTracks,
      },
    });

    await tick();
    expect(target.textContent).toContain('Track One');

    // Test SMTC nexttrack action
    const nextHandler = mockActionHandlers.get('nexttrack');
    expect(nextHandler).toBeDefined();
    nextHandler!();
    await tick();
    expect(target.textContent).toContain('Track Two');

    // Test hardware keyboard media key: MediaTrackPrevious
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'MediaTrackPrevious' }));
    await tick();
    expect(target.textContent).toContain('Track One');

    // Test hardware keyboard media key: MediaPlayPause
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'MediaPlayPause' }));
    await new Promise((r) => setTimeout(r, 50));
    await tick();
    expect(mockMediaSession.playbackState).toBe('playing');

    // Test SMTC pause action
    const pauseHandler = mockActionHandlers.get('pause');
    expect(pauseHandler).toBeDefined();
    pauseHandler!();
    await tick();
    expect(mockMediaSession.playbackState).toBe('paused');

    unmount(comp);
  });

  it('smoothly fades out music over 600 ms when game launches and stays stopped on exit', async () => {
    vi.useFakeTimers();

    const mockTracks = [
      {
        id: '1',
        title: 'Wet Hands',
        artist: 'C418',
        file_name: 'C418 - Wet Hands.mp3',
        cover_data_url: null,
        duration_seconds: 130n,
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

    // Start playing
    await comp.play();
    await tick();
    expect(mockMediaSession.playbackState).toBe('playing');

    const audio = target.querySelector('audio') as HTMLAudioElement;
    expect(audio).not.toBeNull();
    const initialVolume = audio.volume;
    expect(initialVolume).toBeGreaterThan(0);

    // Game starts: notifyGameRunning(true)
    comp.notifyGameRunning(true);
    await tick();

    // Advance 300 ms (halfway through the 600 ms fade)
    vi.advanceTimersByTime(300);
    expect(audio.volume).toBeLessThan(initialVolume);

    // Advance through the remaining 350 ms of the 600 ms fade
    vi.advanceTimersByTime(350);
    await tick();

    // Now playback is paused and state is paused
    expect(mockMediaSession.playbackState).toBe('paused');

    // Game exits: notifyGameRunning(false)
    comp.notifyGameRunning(false);
    await tick();
    vi.advanceTimersByTime(500);
    await tick();

    // Per requirement: "после выхода из игры сама не включается"
    expect(mockMediaSession.playbackState).toBe('paused');

    vi.useRealTimers();
    unmount(comp);
  });

  it('supports SMTC session helpers (getIsPlaying, getIsFadingOut, fadeOutAndPause)', async () => {
    const mockTracks = [
      {
        id: '1',
        title: 'Sweden',
        artist: 'C418',
        file_name: 'C418 - Sweden.mp3',
        cover_data_url: null,
        duration_seconds: 215n,
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

    const audio = target.querySelector('audio') as HTMLAudioElement;
    if (audio) {
      audio.src = 'data:audio/mp3;base64,mockaudio';
    }

    comp.play();
    await tick();
    expect(comp.getIsPlaying()).toBe(true);

    const fadePromise = comp.fadeOutAndPause(50);
    expect(comp.getIsFadingOut()).toBe(true);

    await fadePromise;
    expect(comp.getIsFadingOut()).toBe(false);
    expect(comp.getIsPlaying()).toBe(false);
    expect(window.HTMLMediaElement.prototype.pause).toHaveBeenCalled();

    unmount(comp);
  });
});
