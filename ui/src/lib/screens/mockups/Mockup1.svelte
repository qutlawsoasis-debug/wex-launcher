<!--
  Mockup 1: «Иммерсивный горизонт» (Apple Arcade / Full-Bleed Atmosphere)
  Apple HIG Rules:
  - materials.md: Liquid Glass functional layer floating above dynamic real game artwork.
  - layout.md: Restraint, one primary focal point (launch dock), left-aligned readable typography.
  - typography.md: Apple scale (Large Title 34px, Headline 17px, Subheadline 15px, Footnote 13px).
  - color.md: System colors with high contrast (> 4.5:1) over scrimmed backdrop.
  - Writing: Human, concise Russian without marketing buzzwords.
-->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import SegmentedControl from '../../components/SegmentedControl.svelte';
  import ProgressBar from '../../components/ProgressBar.svelte';
  import Icon from '../../components/icons/Icon.svelte';
  import { theme } from '../../design/theme.svelte';

  interface Props {
    selectedVersion?: string;
    playerName?: string;
    clientStatus?: string;
    isLaunching?: boolean;
    progressPercentage?: number;
    progressStage?: string;
    progressDetail?: string;
    progressSpeed?: string;
    onlaunch?: () => void;
    oncancel?: () => void;
  }

  let {
    selectedVersion = $bindable('1.21.1'),
    playerName = $bindable('Player'),
    clientStatus = '',
    isLaunching = false,
    progressPercentage = 0,
    progressStage = 'Готов к запуску',
    progressDetail = 'Нажмите «Играть» для старта',
    progressSpeed = '',
    onlaunch,
    oncancel,
  }: Props = $props();

  const versions = [
    { value: '1.20.1', label: '1.20.1' },
    { value: '1.21.1', label: '1.21.1' },
  ];

  let javaRuntime = $derived(
    selectedVersion === '1.20.1' ? 'Java 17 • 64-bit' : 'Java 21 • 64-bit'
  );

  // Background artwork based on active theme
  let bgImage = $derived(theme.isDark ? '/art/sunset.png' : '/art/day.png');
</script>

<div class="mockup1-container">
  <!-- Full-bleed game screenshot background -->
  <div class="scenery-backdrop" style="background-image: url('{bgImage}');">
    <div class="scenery-scrim"></div>
  </div>

  <!-- Content Layer -->
  <div class="mockup1-content">
    <!-- Top Left Title Area -->
    <header class="hero-header">
      <span class="client-label">Клиент Wex</span>
      <h1 class="game-title">Minecraft</h1>
      <p class="game-subtitle">Версия {selectedVersion} • Fabric Loader</p>
    </header>

    <!-- Mid Layer: Real Update Notes & Status -->
    <div class="info-shelf">
      <div class="news-capsule">
        <div class="capsule-header">
          <Icon name="play" size={14} color="var(--color-system-blue)" />
          <span class="capsule-title">Обновление 2.4</span>
          <span class="capsule-date">Вчера</span>
        </div>
        <ul class="changelog-list">
          <li>Переработанное освещение заката и ночного неба</li>
          <li>Снижение задержки ввода и плавная стабилизация кадров</li>
          <li>Синхронизация модов в кооперативном лобби</li>
        </ul>
      </div>

      <div class="quick-status-capsule">
        <div class="capsule-header">
          <Icon name="server" size={14} color="var(--color-secondary-label)" />
          <span class="capsule-title">Последний сервер</span>
        </div>
        <div class="last-server-row">
          <div class="server-dot"></div>
          <div class="server-info">
            <span class="server-name">Локальный сервер</span>
            <span class="server-meta">127.0.0.1:25565 • Пинг 2 мс</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Bottom Floating Launch Dock -->
    <div class="launch-dock">
      {#if isLaunching}
        <div class="dock-progress">
          <ProgressBar
            percentage={progressPercentage}
            label={progressStage}
            detail={progressSpeed ? `${progressSpeed} • ${progressDetail}` : progressDetail}
            height={8}
          />
          <div class="dock-cancel">
            <Button variant="secondary" size="regular" onclick={oncancel}>
              Отмена
            </Button>
          </div>
        </div>
      {:else}
        <div class="dock-controls">
          <!-- Version Pill -->
          <div class="dock-item version-item">
            <span class="dock-label">Версия</span>
            <SegmentedControl
              options={versions}
              bind:value={selectedVersion}
              disabled={isLaunching}
            />
          </div>

          <div class="dock-divider"></div>

          <!-- Player Nickname -->
          <div class="dock-item nickname-item">
            <span class="dock-label">Игрок</span>
            <div class="nickname-field">
              <Icon name="friends" size={15} color="var(--color-tertiary-label)" />
              <input
                type="text"
                bind:value={playerName}
                placeholder="Никнейм"
                class="nickname-input"
                maxlength={16}
                disabled={isLaunching}
              />
            </div>
          </div>

          <div class="dock-divider"></div>

          <!-- Secondary Tech Info -->
          <div class="dock-item tech-item">
            <span class="dock-label">Среда</span>
            <span class="tech-val">{javaRuntime}</span>
          </div>

          <!-- Primary Play Action -->
          <div class="dock-action">
            <Button
              variant="primary"
              size="large"
              onclick={onlaunch}
              disabled={!playerName.trim()}
            >
              <Icon name="play" size={18} color="#ffffff" />
              <span>Играть</span>
            </Button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .mockup1-container {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .scenery-backdrop {
    position: absolute;
    inset: 0;
    background-size: cover;
    background-position: center 35%;
    transition: background-image 0.4s ease;
    z-index: 0;
  }

  .scenery-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to bottom,
      rgba(0, 0, 0, 0.45) 0%,
      rgba(0, 0, 0, 0.15) 30%,
      rgba(0, 0, 0, 0.55) 75%,
      rgba(0, 0, 0, 0.85) 100%
    );
  }

  :global([data-theme='light']) .scenery-scrim {
    background: linear-gradient(
      to bottom,
      rgba(255, 255, 255, 0.65) 0%,
      rgba(255, 255, 255, 0.35) 30%,
      rgba(255, 255, 255, 0.75) 75%,
      rgba(255, 255, 255, 0.95) 100%
    );
  }

  .mockup1-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 100%;
    padding: 32px 36px 28px;
    box-sizing: border-box;
  }

  .hero-header {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }

  .client-label {
    font-size: var(--text-caption-1);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-system-blue);
    margin-bottom: 4px;
  }

  .game-title {
    font-size: 38px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.1;
    margin: 0 0 6px;
    color: var(--color-label);
  }

  .game-subtitle {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    margin: 0;
  }

  /* Mid Shelf */
  .info-shelf {
    display: flex;
    gap: 20px;
    margin-top: auto;
    margin-bottom: 24px;
    flex-wrap: wrap;
  }

  .news-capsule,
  .quick-status-capsule {
    background: var(--color-system-ultra-thin-material);
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    box-sizing: border-box;
  }

  .news-capsule {
    flex: 1 1 340px;
    max-width: 440px;
  }

  .quick-status-capsule {
    flex: 0 1 280px;
  }

  .capsule-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  .capsule-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .capsule-date {
    margin-left: auto;
    font-size: var(--text-caption-2);
    color: var(--color-tertiary-label);
  }

  .changelog-list {
    margin: 0;
    padding-left: 18px;
    list-style-type: disc;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    line-height: 1.4;
  }

  .last-server-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }

  .server-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-system-green);
    flex-shrink: 0;
  }

  .server-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .server-name {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
  }

  .server-meta {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
  }

  /* Bottom Launch Dock */
  .launch-dock {
    background: var(--color-system-thin-material);
    backdrop-filter: blur(32px) saturate(180%);
    -webkit-backdrop-filter: blur(32px) saturate(180%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl);
    padding: 12px 18px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
  }

  :global([data-theme='light']) .launch-dock {
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }

  .dock-controls {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .dock-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .dock-label {
    font-size: var(--text-caption-2);
    font-weight: 500;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .dock-divider {
    width: 1px;
    height: 36px;
    background-color: var(--color-separator);
    margin: 0 4px;
  }

  .nickname-field {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-tertiary-system-fill);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-sm);
    padding: 0 10px;
    height: 32px;
    box-sizing: border-box;
  }

  .nickname-input {
    border: none;
    background: transparent;
    color: var(--color-label);
    font-size: var(--text-subheadline);
    font-weight: 500;
    width: 120px;
    outline: none;
  }

  .nickname-input::placeholder {
    color: var(--color-tertiary-label);
  }

  .tech-item {
    min-width: 110px;
  }

  .tech-val {
    font-size: var(--text-footnote);
    color: var(--color-label);
    font-weight: 500;
    height: 32px;
    display: flex;
    align-items: center;
  }

  .dock-action {
    margin-left: auto;
  }

  .dock-progress {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 6px 4px;
  }

  .dock-progress :global(.progress-bar-container) {
    flex: 1;
  }

  .dock-cancel {
    flex-shrink: 0;
  }
</style>
