<!--
  Wex Launcher — MusicDiskWidget Component
  Specification:
  - Background music widget based on the classic launcher's MusicDiskWidget.
  - Local playlist from the Wex/music folder.
  - Honest empty state when no files exist with a button to open the folder in Explorer.
  - Turntable vinyl record animation that spins when playing.
  - Reduced Motion support: respects system settings.
  - Auto-pause when game is running.
  - Volume control and mute toggle.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './icons/Icon.svelte';
  import Button from './Button.svelte';
  import { COMMANDS } from '../ipc/generated/commands';
  import type { MusicTrackDto, MusicPlaylistResponse, MusicGetTrackDataResponse, MusicSettingsDto } from '../ipc/generated';

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
  let positionSeconds = $state(0);
  let durationSeconds = $state(0);
  let volume = $state(75);
  let isMuted = $state(false);
  let isEnabled = $state(true);
  let isLoadingTrack = $state(false);
  let musicDir = $state('');

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

  async function loadAndPlayTrack(index: number, shouldPlay = true) {
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
          audioElement.play().catch(() => {
            isPlaying = false;
          });
          isPlaying = true;
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

  export function togglePlayPause() {
    if (!isEnabled) return;
    if (tracks.length === 0) return;

    if (isPlaying) {
      pause();
    } else {
      play();
    }
  }

  export function play() {
    if (!isEnabled || tracks.length === 0) return;
    if (audioElement && audioElement.src) {
      audioElement.play().catch(() => {
        isPlaying = false;
      });
      isPlaying = true;
    } else {
      loadAndPlayTrack(currentTrackIndex, true);
    }
  }

  export function pause() {
    if (audioElement) {
      audioElement.pause();
    }
    isPlaying = false;
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

  // Auto-pause when Minecraft game starts
  $effect(() => {
    if (isGameRunning && isPlaying) {
      pause();
    }
  });

  onMount(() => {
    loadPlaylist();
    loadSettings();
  });
</script>

<div class="music-disk-widget" class:playing={isPlaying} aria-label="Виджет фоновой музыки">
  <!-- Hidden HTML5 Audio Element -->
  <audio
    bind:this={audioElement}
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
    <!-- Active Player State -->
    <div class="active-player">
      <div class="turntable-mini">
        <div class="vinyl-disc" class:spinning={isPlaying} aria-hidden="true">
          <div class="vinyl-groove groove-1"></div>
          <div class="vinyl-groove groove-2"></div>
          <div class="vinyl-groove groove-3"></div>

          <div class="vinyl-center">
            {#if currentTrack?.cover_data_url}
              <img
                src={currentTrack.cover_data_url}
                alt="Обложка"
                class="vinyl-cover-img"
              />
            {:else}
              <span class="vinyl-label-text">Wex</span>
            {/if}
          </div>
          <div class="vinyl-spindle"></div>
        </div>

        <!-- Tonearm -->
        <div class="tonearm" class:on-record={isPlaying} aria-hidden="true">
          <div class="tonearm-pivot"></div>
          <div class="tonearm-stick"></div>
          <div class="tonearm-head"></div>
        </div>
      </div>

      <div class="player-content">
        <!-- Track metadata -->
        <div class="meta-row">
          <div class="meta-text">
            <span class="track-title" title={currentTrack?.title || ''}>
              {currentTrack?.title || 'Загрузка...'}
            </span>
            <span class="track-artist" title={currentTrack?.artist || ''}>
              {currentTrack?.artist || '—'}
            </span>
          </div>

          <button
            type="button"
            class="folder-icon-btn"
            onclick={openFolder}
            title="Открыть папку с музыкой в проводнике"
            aria-label="Открыть папку с музыкой"
          >
            <Icon name="folder" size={14} color="var(--color-secondary-label)" />
          </button>
        </div>

        <!-- Progress bar with timestamps -->
        <div class="progress-container">
          <button
            type="button"
            class="progress-track"
            aria-label="Позиция воспроизведения"
            onclick={handleSeek}
          >
            <div
              class="progress-fill"
              style="width: {durationSeconds > 0 ? (positionSeconds / durationSeconds) * 100 : 0}%"
            ></div>
          </button>
          <div class="time-row">
            <span>{formatTime(positionSeconds)}</span>
            <span>{formatTime(durationSeconds)}</span>
          </div>
        </div>

        <!-- Playback and volume controls -->
        <div class="controls-row">
          <div class="transport-buttons">
            <button
              type="button"
              class="ctrl-btn"
              onclick={previous}
              title="Предыдущий трек"
              aria-label="Предыдущий трек"
            >
              <Icon name="backward" size={14} color="var(--color-label)" />
            </button>

            <button
              type="button"
              class="ctrl-btn play-pause-btn"
              onclick={togglePlayPause}
              title={isPlaying ? 'Пауза' : 'Воспроизведение'}
              aria-label={isPlaying ? 'Пауза' : 'Воспроизведение'}
            >
              <Icon name={isPlaying ? 'pause' : 'play'} size={15} color="#ffffff" />
            </button>

            <button
              type="button"
              class="ctrl-btn"
              onclick={next}
              title="Следующий трек"
              aria-label="Следующий трек"
            >
              <Icon name="forward" size={14} color="var(--color-label)" />
            </button>
          </div>

          <div class="volume-container">
            <button
              type="button"
              class="mute-btn"
              onclick={toggleMute}
              title={isMuted ? 'Включить звук' : 'Выключить звук'}
              aria-label={isMuted ? 'Включить звук' : 'Выключить звук'}
            >
              <Icon name={isMuted || volume === 0 ? 'volume-x' : 'volume'} size={14} color="var(--color-secondary-label)" />
            </button>
            <input
              type="range"
              min="0"
              max="100"
              value={isMuted ? 0 : volume}
              oninput={(e) => handleVolumeChange(Number((e.target as HTMLInputElement).value))}
              class="volume-slider"
              aria-label="Громкость"
            />
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .music-disk-widget {
    background-color: var(--glass-card-bg);
    backdrop-filter: blur(20px) saturate(160%);
    -webkit-backdrop-filter: blur(20px) saturate(160%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 10px;
    margin-bottom: 8px;
    user-select: none;
    box-sizing: border-box;
    width: 100%;
  }

  .disabled-state {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
  }

  .disabled-text {
    font-size: var(--text-caption-1);
    color: var(--color-tertiary-label);
  }

  .empty-widget {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .active-player {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* Turntable & Vinyl Styling */
  .turntable-mini {
    position: relative;
    width: 52px;
    height: 52px;
    flex-shrink: 0;
  }

  .active-player .turntable-mini {
    align-self: center;
    width: 68px;
    height: 68px;
  }

  .vinyl-disc {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: radial-gradient(circle, #2c2d30 0%, #151618 60%, #0d0e10 100%);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.3s ease-out;
  }

  .vinyl-disc.spinning {
    animation: spin 3s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .vinyl-disc.spinning {
      animation: none;
    }
  }

  .vinyl-groove {
    position: absolute;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.07);
    pointer-events: none;
  }

  .groove-1 {
    width: 82%;
    height: 82%;
  }

  .groove-2 {
    width: 64%;
    height: 64%;
  }

  .groove-3 {
    width: 46%;
    height: 46%;
  }

  .vinyl-center {
    width: 40%;
    height: 40%;
    border-radius: 50%;
    background: var(--color-system-blue);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    z-index: 1;
  }

  .vinyl-center.empty {
    background: var(--color-fill-secondary);
  }

  .vinyl-label-text {
    font-size: 8px;
    font-weight: 700;
    color: #ffffff;
    letter-spacing: -0.2px;
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
