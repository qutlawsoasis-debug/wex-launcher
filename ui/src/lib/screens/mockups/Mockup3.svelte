<!--
  Mockup 3: «Сфокусированный лаунчер» (Apple Music / Media Player Minimalist)
  Apple HIG Rules:
  - buttons.md: Dominant primary action button (height 50px, systemBlue, bold white typography).
  - segmented-controls.md: Compact, clean version switching with immediate response.
  - materials.md: Restrained Liquid Glass capsule centered on a softly blurred atmospheric canvas.
  - layout.md: Maximum focus and quiet minimalism, zero decorative clutter.
  - Writing: Human, simple and direct Russian text.
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

  let backdropImage = $derived(theme.isDark ? '/art/sunset.webp' : '/art/day.webp');
</script>

<div class="mockup3-container">
  <!-- Soft blurred atmospheric canvas -->
  <div class="blurred-backdrop" style="background-image: url('{backdropImage}');">
    <div class="backdrop-overlay"></div>
  </div>

  <div class="mockup3-content">
    <!-- Focused Player Capsule -->
    <div class="player-capsule">
      <div class="capsule-top">
        <div class="title-cluster">
          <h1 class="main-title">Minecraft</h1>
          <span class="sub-title">Клиент Wex • Fabric</span>
        </div>

        <div class="version-segmented-wrap">
          <SegmentedControl
            options={versions}
            bind:value={selectedVersion}
            disabled={isLaunching}
          />
        </div>
      </div>

      <!-- Action Row -->
      <div class="action-row">
        <div class="player-input-wrap">
          <Icon name="friends" size={16} color="var(--color-tertiary-label)" />
          <input
            type="text"
            bind:value={playerName}
            placeholder="Никнейм игрока"
            class="player-input"
            maxlength={16}
            disabled={isLaunching}
          />
        </div>

        {#if isLaunching}
          <div class="launch-btn-wrap">
            <Button variant="secondary" size="large" onclick={oncancel}>
              Отмена
            </Button>
          </div>
        {:else}
          <div class="launch-btn-wrap">
            <Button
              variant="primary"
              size="large"
              onclick={onlaunch}
              disabled={!playerName.trim()}
            >
              <Icon name="play" size={20} color="#ffffff" />
              <span class="launch-text">Играть</span>
            </Button>
          </div>
        {/if}
      </div>

      {#if isLaunching}
        <div class="capsule-progress">
          <ProgressBar
            percentage={progressPercentage}
            label={progressStage}
            detail={progressSpeed ? `${progressSpeed} • ${progressDetail}` : progressDetail}
            height={8}
          />
        </div>
      {/if}

      <!-- Quiet Specification Meta -->
      <div class="meta-strip">
        <div class="meta-item">
          <span class="meta-label">Среда выполнения</span>
          <span class="meta-value">{javaRuntime}</span>
        </div>

        <div class="meta-sep"></div>

        <div class="meta-item">
          <span class="meta-label">Загрузчик</span>
          <span class="meta-value">Fabric 0.16.9</span>
        </div>

        <div class="meta-sep"></div>

        <div class="meta-item">
          <span class="meta-label">Модификации</span>
          <span class="meta-value">142 мода активны</span>
        </div>
      </div>
    </div>

    <!-- Quiet Bottom Updates Feed -->
    <div class="quiet-feed">
      <div class="feed-header">
        <h3 class="feed-title">Последние изменения</h3>
        <span class="feed-subtitle">Сборка Wex 2.4</span>
      </div>

      <div class="feed-items">
        <div class="feed-row">
          <span class="feed-dot"></span>
          <div class="feed-desc">
            <strong class="feed-topic">Новое небо:</strong>
            Мягкий закатный горизонт и корректная фаза луны без пересветов
          </div>
          <span class="feed-time">Сегодня</span>
        </div>

        <div class="feed-row">
          <span class="feed-dot"></span>
          <div class="feed-desc">
            <strong class="feed-topic">Кооператив:</strong>
            Создание комнат с мгновенной сверкой модов между участниками
          </div>
          <span class="feed-time">Вчера</span>
        </div>

        <div class="feed-row">
          <span class="feed-dot"></span>
          <div class="feed-desc">
            <strong class="feed-topic">Быстродействие:</strong>
            Оптимизация GL-контекста и снижение времени отрисовки кадра
          </div>
          <span class="feed-time">05.11</span>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .mockup3-container {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .blurred-backdrop {
    position: absolute;
    inset: 0;
    background-size: cover;
    background-position: center 30%;
    filter: blur(28px) saturate(140%);
    transform: scale(1.08);
    z-index: 0;
  }

  .backdrop-overlay {
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at 50% 30%,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(0, 0, 0, 0.8) 100%
    );
  }

  :global([data-theme='light']) .backdrop-overlay {
    background: radial-gradient(
      circle at 50% 30%,
      rgba(255, 255, 255, 0.7) 0%,
      rgba(255, 255, 255, 0.92) 100%
    );
  }

  .mockup3-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 36px 32px 40px;
    box-sizing: border-box;
    gap: 32px;
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
  }

  /* Player Capsule */
  .player-capsule {
    width: 100%;
    background: var(--color-system-thin-material);
    backdrop-filter: blur(36px) saturate(180%);
    -webkit-backdrop-filter: blur(36px) saturate(180%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl);
    padding: 24px 28px;
    box-sizing: border-box;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.32);
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  :global([data-theme='light']) .player-capsule {
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.08);
  }

  .capsule-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title-cluster {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .main-title {
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.025em;
    margin: 0;
    color: var(--color-label);
  }

  .sub-title {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
  }

  .action-row {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .player-input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--color-tertiary-system-fill);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md);
    padding: 0 14px;
    height: 48px;
    box-sizing: border-box;
  }

  .player-input {
    border: none;
    background: transparent;
    color: var(--color-label);
    font-size: var(--text-body);
    font-weight: 500;
    width: 100%;
    outline: none;
  }

  .player-input::placeholder {
    color: var(--color-tertiary-label);
  }

  .launch-btn-wrap {
    flex-shrink: 0;
  }

  .launch-btn-wrap :global(.apple-button) {
    height: 48px;
    padding: 0 28px;
    font-size: var(--text-headline);
    border-radius: var(--radius-md);
  }

  .launch-text {
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .capsule-progress {
    padding-top: 4px;
  }

  /* Quiet Meta Strip */
  .meta-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 14px;
    border-top: 1px solid var(--color-separator);
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-label {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .meta-value {
    font-size: var(--text-footnote);
    color: var(--color-label);
    font-weight: 500;
  }

  .meta-sep {
    width: 1px;
    height: 24px;
    background-color: var(--color-separator);
  }

  /* Quiet Feed */
  .quiet-feed {
    width: 100%;
    background: var(--color-secondary-system-grouped-background);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-lg);
    padding: 20px 24px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .feed-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    border-bottom: 1px solid var(--color-separator);
    padding-bottom: 10px;
  }

  .feed-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .feed-subtitle {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }

  .feed-items {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .feed-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--text-footnote);
  }

  .feed-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-system-blue);
    flex-shrink: 0;
  }

  .feed-desc {
    flex: 1;
    color: var(--color-secondary-label);
    line-height: 1.4;
  }

  .feed-topic {
    color: var(--color-label);
    font-weight: 600;
    margin-right: 4px;
  }

  .feed-time {
    font-size: var(--text-caption-2);
    color: var(--color-tertiary-label);
    flex-shrink: 0;
  }
</style>
