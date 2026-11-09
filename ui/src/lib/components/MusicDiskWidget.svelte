<!--
  Wex Launcher — MusicDiskWidget Component
  Specification:
  - Background music widget based on the classic launcher's MusicDiskWidget.
  - Local playlist from the Wex/music folder.
  - Honest empty state when no files exist with a button to open the folder in Explorer.
  - Turntable vinyl record animation that spins when playing.
  - Reduced Motion support: respects system settings.
  - Auto-pause when game is running with smooth 600 ms fade-out.
  - Windows System Media Transport Controls (SMTC) integration via navigator.mediaSession.
  - Hardware media keys support (MediaPlayPause, MediaTrackNext, MediaTrackPrevious, MediaStop).
  - Background audio playback when minimized.
  - Volume control and mute toggle.
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Icon from './icons/Icon.svelte';
  import Button from './Button.svelte';
  import Skeleton from './Skeleton.svelte';
  import { theme } from '../design/theme.svelte';
  import { COMMANDS } from '../ipc/generated/commands';
  import type {
    MusicTrackDto,
    MusicPlaylistResponse,
    MusicGetTrackDataResponse,
    MusicSettingsDto,
    MusicSmtcStateDto,
    MusicCheckCommandResponse,
  } from '../ipc/generated';

  interface Props {
    isGameRunning?: boolean;
    initialTracks?: MusicTrackDto[];
    initialSettings?: MusicSettingsDto;
  }

  let { isGameRunning = false, initialTracks, initialSettings }: Props = $props();

  // Music state
  let tracks = $state<MusicTrackDto[]>([]);
  let currentTrackIndex = $state(0);
  let isPlaying = $state(false);
  let isFadingOut = $state(false);
  let positionSeconds = $state(0);
  let durationSeconds = $state(0);
  let volume = $state(75);
  let isMuted = $state(false);
  let isEnabled = $state(true);
  let isLoadingTrack = $state(false);
  let musicDir = $state('');

  let fadeInterval: ReturnType<typeof setInterval> | null = null;
  let commandPollInterval: ReturnType<typeof setInterval> | null = null;
  let prevIsGameRunning = false;

  // Turntable Vinyl Record Inertia Physics
  let vinylAngle = $state(0);
  let angularVelocity = 0; // deg/ms
  const NOMINAL_VELOCITY = 360 / 3000; // 0.12 deg/ms (33.3 RPM)
  const ACCELERATION_RATE = 0.003;
  const FRICTION_DECAY = 0.965;
  let animFrameId: number | null = null;
  let lastTimestamp = 0;
  let isCoasting = $state(false);

  function updateVinylPhysics(timestamp: number) {
    if (!lastTimestamp) lastTimestamp = timestamp;
    const dt = Math.min(timestamp - lastTimestamp, 64);
    lastTimestamp = timestamp;

    if (theme.reduceMotion) {
      angularVelocity = 0;
      isCoasting = false;
      animFrameId = null;
      return;
    }

    if (isPlaying) {
      angularVelocity += (NOMINAL_VELOCITY - angularVelocity) * (1 - Math.pow(1 - ACCELERATION_RATE, dt));
      vinylAngle = (vinylAngle + angularVelocity * dt) % 360;
      isCoasting = false;
      animFrameId = requestAnimationFrame(updateVinylPhysics);
    } else {
      if (angularVelocity > 0.0008) {
        isCoasting = true;
        const frameFactor = Math.pow(FRICTION_DECAY, dt / 16.6);
        angularVelocity *= frameFactor;
        vinylAngle = (vinylAngle + angularVelocity * dt) % 360;
        animFrameId = requestAnimationFrame(updateVinylPhysics);
      } else {
        angularVelocity = 0;
        isCoasting = false;
        animFrameId = null;
      }
    }
  }

  $effect(() => {
    if (isPlaying) {
      lastTimestamp = typeof performance !== 'undefined' ? performance.now() : Date.now();
      if (!animFrameId && typeof requestAnimationFrame !== 'undefined') {
        animFrameId = requestAnimationFrame(updateVinylPhysics);
      }
    } else if (angularVelocity > 0.0008 && !animFrameId && typeof requestAnimationFrame !== 'undefined') {
      lastTimestamp = typeof performance !== 'undefined' ? performance.now() : Date.now();
      animFrameId = requestAnimationFrame(updateVinylPhysics);
    }
  });

  onDestroy(() => {
    if (animFrameId && typeof cancelAnimationFrame !== 'undefined') {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  });

  $effect(() => {
    if (initialTracks !== undefined) {
      tracks = initialTracks;
    }
  });

  $effect(() => {
    if (initialSettings) {
      volume = initialSettings.volume;
      isEnabled = initialSettings.enabled;
    }
  });

  let audioElement = $state<HTMLAudioElement | null>(null);

  let currentTrack = $derived(tracks.length > 0 ? tracks[currentTrackIndex] : null);

  export function getIsPlaying(): boolean {
    return isPlaying;
  }

  export function getIsFadingOut(): boolean {
    return isFadingOut;
  }

  export function getAudioVolume(): number {
    return audioElement ? audioElement.volume : 0;
  }

  export function getCurrentTrack(): MusicTrackDto | null {
    return currentTrack;
  }

  async function callTauri<T>(command: string, args: Record<string, any> = {}): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(command, args);
    } catch (e) {
      throw e;
    }
  }

  async function loadPlaylist() {
    if (initialTracks !== undefined) return;
    try {
      const res = await callTauri<MusicPlaylistResponse>(COMMANDS.MUSIC_GET_PLAYLIST);
      if (res) {
        tracks = res.tracks || [];
        musicDir = res.music_dir || '';
      }
    } catch {
      // Offline / fallback
      tracks = [];
    }
  }

  async function loadSettings() {
    if (initialSettings !== undefined) return;
    try {
      const res = await callTauri<MusicSettingsDto>(COMMANDS.MUSIC_GET_SETTINGS);
      if (res) {
        isEnabled = res.enabled;
        volume = res.volume;
      }
    } catch {
      // Defaults
      isEnabled = true;
      volume = 75;
    }
  }

  async function saveSettings() {
    try {
      await callTauri(COMMANDS.MUSIC_SET_SETTINGS, {
        settings: {
          enabled: isEnabled,
          volume,
          auto_pause_on_launch: true,
        },
      });
    } catch {}
  }

  async function loadAndPlayTrack(index: number, shouldPlay = true): Promise<void> {
    if (index < 0 || index >= tracks.length) return;
    currentTrackIndex = index;
    positionSeconds = 0;
    durationSeconds = 0;

    const track = tracks[index];
    if (!track) return;

    isLoadingTrack = true;
    try {
      const res = await callTauri<MusicGetTrackDataResponse>(COMMANDS.MUSIC_GET_TRACK_DATA, {
        request: { file_name: track.file_name },
      });

      if (audioElement && res?.audio_base64) {
        audioElement.src = `data:${res.mime_type};base64,${res.audio_base64}`;
        audioElement.volume = (isMuted ? 0 : volume) / 100;
        if (shouldPlay && isEnabled) {
          isPlaying = true;
          await audioElement.play().catch(() => {
            isPlaying = false;
          });
        } else {
          isPlaying = false;
        }
      }
    } catch {
      isPlaying = false;
    } finally {
      isLoadingTrack = false;
    }
  }

  export function togglePlayPause(): Promise<void> | void {
    if (!isEnabled) return;
    if (tracks.length === 0) return;

    if (isPlaying) {
      pause();
    } else {
      return play();
    }
  }

  export function play(): Promise<void> | void {
    if (fadeInterval) {
      clearInterval(fadeInterval);
      fadeInterval = null;
    }
    isFadingOut = false;

    if (!isEnabled || tracks.length === 0) return;
    if (audioElement && audioElement.src) {
      audioElement.volume = (isMuted ? 0 : volume) / 100;
      isPlaying = true;
      return audioElement.play().catch(() => {
        isPlaying = false;
      });
    } else {
      return loadAndPlayTrack(currentTrackIndex, true);
    }
  }

  export function pause() {
    if (fadeInterval) {
      clearInterval(fadeInterval);
      fadeInterval = null;
    }
    isFadingOut = false;
    if (audioElement) {
      audioElement.pause();
      audioElement.volume = (isMuted ? 0 : volume) / 100;
    }
    isPlaying = false;
  }

  export function fadeOutAndPause(durationMs = 600): Promise<void> {
    return new Promise((resolve) => {
      if (fadeInterval) {
        clearInterval(fadeInterval);
        fadeInterval = null;
      }

      if (!audioElement || !isPlaying) {
        pause();
        resolve();
        return;
      }

      isFadingOut = true;
      const initialVol = audioElement.volume;
      const steps = 20;
      const stepDuration = Math.max(10, Math.floor(durationMs / steps));
      let currentStep = 0;

      fadeInterval = setInterval(() => {
        currentStep++;
        const factor = Math.max(0, 1 - currentStep / steps);
        if (audioElement) {
          audioElement.volume = initialVol * factor;
        }

        if (currentStep >= steps || !isPlaying) {
          if (fadeInterval) {
            clearInterval(fadeInterval);
            fadeInterval = null;
          }
          pause();
          isFadingOut = false;
          // Restore volume setting so user can resume playback later
          if (audioElement) {
            audioElement.volume = (isMuted ? 0 : volume) / 100;
          }
          resolve();
        }
      }, stepDuration);
    });
  }

  export function notifyGameRunning(running: boolean) {
    if (running && !prevIsGameRunning) {
      prevIsGameRunning = true;
      if (isPlaying) {
        fadeOutAndPause(600);
      }
    } else if (!running && prevIsGameRunning) {
      prevIsGameRunning = false;
      // Requirement: "после выхода из игры сама не включается"
    }
  }

  export function next() {
    if (tracks.length === 0) return;
    const nextIdx = (currentTrackIndex + 1) % tracks.length;
    loadAndPlayTrack(nextIdx, isPlaying);
  }

  export function previous() {
    if (tracks.length === 0) return;
    if (positionSeconds > 3 && audioElement) {
      audioElement.currentTime = 0;
      positionSeconds = 0;
      return;
    }
    const prevIdx = (currentTrackIndex - 1 + tracks.length) % tracks.length;
    loadAndPlayTrack(prevIdx, isPlaying);
  }

  export async function openFolder() {
    try {
      await callTauri(COMMANDS.MUSIC_OPEN_FOLDER);
    } catch {}
  }

  function handleTimeUpdate() {
    if (audioElement) {
      positionSeconds = Math.floor(audioElement.currentTime);
      if (!isNaN(audioElement.duration)) {
        durationSeconds = Math.floor(audioElement.duration);
      }
    }
  }

  function handleLoadedMetadata() {
    if (audioElement && !isNaN(audioElement.duration)) {
      durationSeconds = Math.floor(audioElement.duration);
    }
  }

  function handleEnded() {
    next();
  }

  function handleSeek(e: MouseEvent) {
    if (!audioElement || durationSeconds <= 0) return;
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const pos = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const newTime = pos * durationSeconds;
    audioElement.currentTime = newTime;
    positionSeconds = Math.floor(newTime);
  }

  function handleVolumeChange(newVol: number) {
    volume = Math.max(0, Math.min(100, newVol));
    if (volume > 0) isMuted = false;
    if (audioElement) {
      audioElement.volume = (isMuted ? 0 : volume) / 100;
    }
    saveSettings();
  }

  function toggleMute() {
    isMuted = !isMuted;
    if (audioElement) {
      audioElement.volume = (isMuted ? 0 : volume) / 100;
    }
  }

  function formatTime(totalSec: number): string {
    const mins = Math.floor(totalSec / 60);
    const secs = totalSec % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  // Windows SMTC (System Media Transport Controls) synchronization
  export function updateMediaSession() {
    if (typeof navigator === 'undefined' || !('mediaSession' in navigator) || !navigator.mediaSession) return;

    if (!currentTrack) {
      try {
        navigator.mediaSession.metadata = null;
        navigator.mediaSession.playbackState = 'none';
      } catch {}
      return;
    }

    try {
      const artwork = currentTrack.cover_data_url
        ? [{ src: currentTrack.cover_data_url, sizes: '512x512', type: 'image/png' }]
        : [{ src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png' }];

      const MetaClass =
        typeof window !== 'undefined' && (window as any).MediaMetadata
          ? (window as any).MediaMetadata
          : typeof MediaMetadata !== 'undefined'
            ? MediaMetadata
            : null;

      if (MetaClass) {
        navigator.mediaSession.metadata = new MetaClass({
          title: currentTrack.title,
          artist: currentTrack.artist,
          album: 'Wex Launcher',
          artwork,
        });
      }

      navigator.mediaSession.playbackState = isPlaying ? 'playing' : (tracks.length > 0 ? 'paused' : 'none');

      if ('setPositionState' in navigator.mediaSession && durationSeconds > 0) {
        navigator.mediaSession.setPositionState({
          duration: Math.max(durationSeconds, positionSeconds),
          playbackRate: 1.0,
          position: Math.min(positionSeconds, durationSeconds),
        });
      }
    } catch {}
  }

  async function syncBackendSmtc() {
    const track = currentTrack;
    const state: MusicSmtcStateDto = {
      is_playing: isPlaying,
      track_title: track?.title ?? null,
      artist: track?.artist ?? null,
      album: track ? 'Wex Launcher' : null,
      has_cover: Boolean(track?.cover_data_url),
      position_seconds: BigInt(positionSeconds),
      duration_seconds: durationSeconds > 0 ? BigInt(durationSeconds) : null,
    };

    try {
      await callTauri(COMMANDS.MUSIC_SYNC_SMTC, { state });
    } catch {}

    try {
      await callTauri(COMMANDS.MUSIC_UPDATE_SESSION, {
        request: {
          session: {
            has_active_session: Boolean(track && isEnabled),
            source_app: 'Wex',
            app_id: 'WexLauncher',
            title: track?.title || '',
            artist: track?.artist || '',
            album: 'Wex Launcher',
            is_playing: isPlaying,
            position_seconds: positionSeconds,
            duration_seconds: durationSeconds,
            thumbnail_base64: track?.cover_data_url || null,
            timestamp_ms: BigInt(Date.now()),
          },
        },
      });
    } catch {}
  }

  export async function pollCommands() {
    try {
      const res = await callTauri<MusicCheckCommandResponse>(COMMANDS.MUSIC_CHECK_COMMAND);
      if (res?.command) {
        switch (res.command.toLowerCase()) {
          case 'play':
            play();
            break;
          case 'pause':
            pause();
            break;
          case 'toggle':
            togglePlayPause();
            break;
          case 'next':
            next();
            break;
          case 'previous':
          case 'prev':
            previous();
            break;
        }
      }
    } catch {}
  }

  // React to track, playback and timing changes for SMTC
  $effect(() => {
    const _t = currentTrack;
    const _p = isPlaying;
    const _pos = positionSeconds;
    const _dur = durationSeconds;
    const _en = isEnabled;

    updateMediaSession();
    syncBackendSmtc();
  });

  // Smooth fade-out (600 ms) when game starts; strictly do not auto-resume on game exit
  $effect(() => {
    notifyGameRunning(isGameRunning);
  });

  onMount(() => {
    loadPlaylist();
    loadSettings();

    // Register Media Session Action Handlers for Windows SMTC
    if (typeof navigator !== 'undefined' && 'mediaSession' in navigator && navigator.mediaSession) {
      const handlers: [MediaSessionAction, MediaSessionActionHandler][] = [
        ['play', () => { play(); }],
        ['pause', () => { pause(); }],
        ['previoustrack', () => { previous(); }],
        ['nexttrack', () => { next(); }],
        ['stop', () => { pause(); }],
        ['seekto', (details) => {
          if (details.seekTime !== undefined && audioElement) {
            audioElement.currentTime = details.seekTime;
            positionSeconds = Math.floor(details.seekTime);
          }
        }],
      ];

      for (const [action, handler] of handlers) {
        try {
          navigator.mediaSession.setActionHandler(action, handler);
        } catch {}
      }
    }

    // Hardware media keys listener on window
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'MediaPlayPause') {
        e.preventDefault();
        togglePlayPause();
      } else if (e.key === 'MediaTrackNext') {
        e.preventDefault();
        next();
      } else if (e.key === 'MediaTrackPrevious') {
        e.preventDefault();
        previous();
      } else if (e.key === 'MediaStop') {
        e.preventDefault();
        pause();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    commandPollInterval = setInterval(pollCommands, 1000);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (fadeInterval) {
        clearInterval(fadeInterval);
        fadeInterval = null;
      }
      if (commandPollInterval) {
        clearInterval(commandPollInterval);
        commandPollInterval = null;
      }
      if (typeof navigator !== 'undefined' && 'mediaSession' in navigator && navigator.mediaSession) {
        try {
          navigator.mediaSession.setActionHandler('play', null);
          navigator.mediaSession.setActionHandler('pause', null);
          navigator.mediaSession.setActionHandler('previoustrack', null);
          navigator.mediaSession.setActionHandler('nexttrack', null);
          navigator.mediaSession.setActionHandler('stop', null);
          navigator.mediaSession.setActionHandler('seekto', null);
        } catch {}
      }
    };
  });

  onDestroy(() => {
    if (fadeInterval) {
      clearInterval(fadeInterval);
      fadeInterval = null;
    }
    if (commandPollInterval) {
      clearInterval(commandPollInterval);
      commandPollInterval = null;
    }
  });
</script>

<div class="music-disk-widget" class:playing={isPlaying} aria-label="Виджет фоновой музыки">
  <!-- Hidden HTML5 Audio Element (preload auto and playsinline for background SMTC) -->
  <audio
    bind:this={audioElement}
    preload="auto"
    playsinline
    ontimeupdate={handleTimeUpdate}
    onloadedmetadata={handleLoadedMetadata}
    onended={handleEnded}
  ></audio>

  {#if !isEnabled}
    <div class="disabled-state">
      <Icon name="volume-x" size={16} color="var(--color-tertiary-label)" />
      <span class="disabled-text">Музыка отключена</span>
    </div>
  {:else if tracks.length === 0}
    <!-- Honest Empty State -->
    <div class="empty-widget">
      <div class="turntable-mini">
        <div class="vinyl-disc static" aria-hidden="true">
          <div class="vinyl-groove groove-1"></div>
          <div class="vinyl-groove groove-2"></div>
          <div class="vinyl-center empty">
            <span class="vinyl-label-text">Wex</span>
          </div>
          <div class="vinyl-spindle"></div>
        </div>
      </div>

      <div class="widget-details">
        <span class="track-title empty">Нет музыки</span>
        <span class="track-artist">Папка Wex/music пуста</span>

        <button
          type="button"
          class="open-folder-btn"
          onclick={openFolder}
          title="Открыть папку с музыкой в проводнике"
        >
          <Icon name="folder" size={13} color="var(--color-system-blue)" />
          <span>Открыть папку</span>
        </button>
      </div>
    </div>
  {:else}
    <!-- Active Player Widget with Turntable -->
    <div class="active-widget">
      <!-- Turntable Vinyl Graphic -->
      <div class="turntable-container">
        <div
          class="vinyl-platter"
          class:spinning={isPlaying || isCoasting}
          style={theme.reduceMotion ? '' : `transform: rotate(${vinylAngle.toFixed(2)}deg);`}
          aria-hidden="true"
        >
          <div class="vinyl-disc">
            <div class="vinyl-groove groove-1"></div>
            <div class="vinyl-groove groove-2"></div>
            <div class="vinyl-groove groove-3"></div>

            <div class="vinyl-center">
              {#if currentTrack?.cover_data_url}
                <img
                  src={currentTrack.cover_data_url}
                  alt={currentTrack.title}
                  class="vinyl-cover-img"
                />
              {:else}
                <div class="vinyl-default-label">
                  <span class="vinyl-label-text">Wex</span>
                </div>
              {/if}
              <div class="vinyl-spindle"></div>
            </div>
          </div>
        </div>

        <!-- Animated Tonearm -->
        <div class="tonearm" class:on-record={isPlaying || isCoasting} aria-hidden="true">
          <div class="tonearm-pivot"></div>
          <div class="tonearm-stick"></div>
          <div class="tonearm-head"></div>
        </div>
      </div>

      <!-- Player Controls and Meta -->
      <div class="player-content">
        <div class="meta-row">
          <div class="meta-text">
            {#if isLoadingTrack && !currentTrack}
              <div class="skeleton-track-meta">
                <Skeleton variant="text" width="100px" height="12px" />
                <Skeleton variant="text" width="60px" height="9px" />
              </div>
            {:else}
              <span class="track-title" title={currentTrack?.title || ''}>
                {currentTrack?.title || '—'}
              </span>
              <span class="track-artist" title={currentTrack?.artist || ''}>
                {currentTrack?.artist || '—'}
              </span>
            {/if}
          </div>
          <button
            type="button"
            class="folder-icon-btn"
            onclick={openFolder}
            title="Открыть папку с музыкой"
            aria-label="Открыть папку с музыкой"
          >
            <Icon name="folder" size={14} color="var(--color-secondary-label)" />
          </button>
        </div>

        <!-- Scrubber Bar -->
        <div class="progress-container">
          <button
            type="button"
            class="progress-track"
            onclick={handleSeek}
            aria-label="Перемотка трека"
          >
            <div
              class="progress-fill"
              style="width: {durationSeconds > 0
                ? (positionSeconds / durationSeconds) * 100
                : 0}%"
            ></div>
          </button>
          <div class="time-row">
            <span>{formatTime(positionSeconds)}</span>
            <span>{formatTime(durationSeconds)}</span>
          </div>
        </div>

        <!-- Transport Controls -->
        <div class="controls-row">
          <div class="transport-buttons">
            <button
              type="button"
              class="ctrl-btn"
              onclick={previous}
              title="Предыдущий трек"
              aria-label="Предыдущий трек"
            >
              <Icon name="chevron-left" size={16} />
            </button>

            <button
              type="button"
              class="ctrl-btn play-pause-btn"
              onclick={togglePlayPause}
              title={isPlaying ? 'Пауза' : 'Воспроизведение'}
              aria-label={isPlaying ? 'Пауза' : 'Воспроизведение'}
            >
              {#if isPlaying}
                <Icon name="pause" size={16} color="#ffffff" />
              {:else}
                <Icon name="play" size={16} color="#ffffff" />
              {/if}
            </button>

            <button
              type="button"
              class="ctrl-btn"
              onclick={next}
              title="Следующий трек"
              aria-label="Следующий трек"
            >
              <Icon name="chevron-right" size={16} />
            </button>
          </div>

          <!-- Volume Controls -->
          <div class="volume-container">
            <button
              type="button"
              class="mute-btn"
              onclick={toggleMute}
              title={isMuted ? 'Включить звук' : 'Выключить звук'}
              aria-label={isMuted ? 'Включить звук' : 'Выключить звук'}
            >
              <Icon
                name={isMuted || volume === 0 ? 'volume-x' : 'volume'}
                size={14}
                color="var(--color-secondary-label)"
              />
            </button>
            <input
              type="range"
              min="0"
              max="100"
              value={isMuted ? 0 : volume}
              oninput={(e) => handleVolumeChange(Number((e.target as HTMLInputElement).value))}
              class="volume-slider"
              aria-label="Громкость музыки"
            />
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .music-disk-widget {
    position: relative;
    padding: 10px 14px;
    background-color: var(--color-bg-secondary);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    min-width: 250px;
    max-width: 320px;
    user-select: none;
    transition: border-color var(--motion-duration-fast) var(--motion-spring-ease);
  }

  .disabled-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    color: var(--color-tertiary-label);
  }

  .disabled-text {
    font-size: var(--text-footnote);
  }

  .empty-widget {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .turntable-mini {
    flex-shrink: 0;
  }

  .widget-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  /* Active widget layout */
  .active-widget {
    display: grid;
    grid-template-columns: 80px 1fr;
    gap: 12px;
    align-items: center;
  }

  /* Turntable vinyl record styling */
  .turntable-container {
    position: relative;
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .vinyl-platter {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    background: radial-gradient(circle, #2a2a2e 0%, #161618 100%);
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.4), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    will-change: transform;
  }

  .skeleton-track-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 2px 0;
  }

  .vinyl-disc {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .vinyl-disc.static {
    width: 48px;
    height: 48px;
    background: radial-gradient(circle, #2a2a2e 0%, #161618 100%);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
  }

  .vinyl-groove {
    position: absolute;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.05);
    pointer-events: none;
  }

  .groove-1 {
    width: 85%;
    height: 85%;
  }

  .groove-2 {
    width: 68%;
    height: 68%;
  }

  .groove-3 {
    width: 52%;
    height: 52%;
  }

  .vinyl-center {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background-color: var(--color-system-blue);
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.5);
    z-index: 1;
  }

  .vinyl-center.empty {
    width: 22px;
    height: 22px;
    background-color: var(--color-tertiary-label);
  }

  .vinyl-default-label {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--color-system-blue) 0%, #0056b3 100%);
  }

  .vinyl-label-text {
    font-size: 8px;
    font-weight: 700;
    color: #ffffff;
    letter-spacing: 0.5px;
  }

  .vinyl-cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .vinyl-spindle {
    position: absolute;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #e2e8f0;
    box-shadow: 0 0 2px rgba(0, 0, 0, 0.5);
    z-index: 2;
  }

  /* Tonearm */
  .tonearm {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 20px;
    height: 38px;
    pointer-events: none;
    transform-origin: top right;
    transform: rotate(-22deg);
    transition: transform 0.4s var(--motion-spring-ease);
    z-index: 3;
  }

  .tonearm.on-record {
    transform: rotate(6deg);
  }

  .tonearm-pivot {
    position: absolute;
    top: 0;
    right: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-tertiary-label);
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .tonearm-stick {
    position: absolute;
    top: 4px;
    right: 3px;
    width: 2px;
    height: 28px;
    background: var(--color-secondary-label);
    border-radius: 1px;
  }

  .tonearm-head {
    position: absolute;
    bottom: 4px;
    right: 1px;
    width: 6px;
    height: 10px;
    background: var(--color-system-blue);
    border-radius: 1px;
    transform: rotate(15deg);
  }

  /* Player Content */
  .player-content {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .meta-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 6px;
  }

  .meta-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .track-title {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-title.empty {
    color: var(--color-secondary-label);
  }

  .track-artist {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .open-folder-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    background: none;
    border: none;
    padding: 0;
    color: var(--color-system-blue);
    font-size: var(--text-caption-1);
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .open-folder-btn:hover {
    text-decoration: underline;
  }

  .folder-icon-btn {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-secondary-label);
  }

  .folder-icon-btn:hover {
    background-color: var(--color-quaternary-fill);
  }

  /* Progress */
  .progress-container {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .progress-track {
    width: 100%;
    height: 4px;
    background-color: var(--color-quaternary-fill);
    border: none;
    border-radius: 2px;
    cursor: pointer;
    position: relative;
    overflow: hidden;
    padding: 0;
    display: block;
  }

  .progress-fill {
    height: 100%;
    background-color: var(--color-system-blue);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .time-row {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--color-tertiary-label);
  }

  /* Controls */
  .controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    margin-top: 2px;
  }

  .transport-buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .ctrl-btn {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--color-fill-secondary);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--color-label);
    transition: transform var(--motion-duration-fast) var(--motion-spring-ease),
      background-color var(--motion-duration-fast) var(--motion-spring-ease);
  }

  .ctrl-btn:hover {
    background-color: var(--color-fill-tertiary);
  }

  .ctrl-btn:active {
    transform: scale(0.92);
  }

  .play-pause-btn {
    width: 30px;
    height: 30px;
    background-color: var(--color-system-blue);
    color: #ffffff;
  }

  .play-pause-btn:hover {
    background-color: var(--color-system-blue);
    filter: brightness(1.1);
  }

  /* Volume */
  .volume-container {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    max-width: 76px;
  }

  .mute-btn {
    background: none;
    border: none;
    padding: 2px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
  }

  .mute-btn:hover {
    background-color: var(--color-quaternary-fill);
  }

  .volume-slider {
    width: 100%;
    height: 3px;
    accent-color: var(--color-system-blue);
    cursor: pointer;
  }
</style>
