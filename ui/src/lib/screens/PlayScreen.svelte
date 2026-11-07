<!--
  Wex Launcher — Screen «Играть»
  Rules:
  - Version selection: 1.20.1 / 1.21.1
  - Player nickname input
  - Large prominent «Играть» button
  - Real-time installation and sync progress indicator
-->
<script lang="ts">
  import Button from '../components/Button.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';
  import Icon from '../components/icons/Icon.svelte';

  interface Props {
    selectedVersion?: string;
    playerName?: string;
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
    isLaunching = false,
    progressPercentage = 0,
    progressStage = 'Готов к запуску',
    progressDetail = 'Нажмите «Играть» для старта',
    progressSpeed = '',
    onlaunch,
    oncancel,
  }: Props = $props();

  const versions = [
    { value: '1.21.1', label: 'Minecraft 1.21.1 (Aura Core)' },
    { value: '1.20.1', label: 'Minecraft 1.20.1 (Aura Pack)' },
  ];
</script>

<div class="play-screen">
  <!-- Hero Art Banner -->
  <div class="hero-banner">
    <div class="hero-gradient"></div>
    <div class="hero-content">
      <div class="hero-badge">
        <span class="status-dot"></span>
        <span>Aura Pack Active</span>
      </div>
      <h2 class="hero-title">Aura Client</h2>
      <p class="hero-subtitle">
        Собственный GL-рендер, размытие Liquid Glass, 3D-эффекты и оптимизация кадров.
      </p>
    </div>
  </div>

  <!-- Launch Configuration Area -->
  <div class="launch-card">
    <div class="launch-controls">
      <!-- Version Selector -->
      <div class="control-group version-group">
        <label class="control-label" for="version-select">Версия игры</label>
        <SegmentedControl
          options={versions}
          bind:value={selectedVersion}
          disabled={isLaunching}
        />
      </div>

      <!-- Player Nickname -->
      <div class="control-group nickname-group">
        <label class="control-label" for="nickname-input">Имя игрока</label>
        <div class="nickname-input-box">
          <Icon name="friends" size={16} color="var(--color-tertiary-label)" />
          <input
            id="nickname-input"
            type="text"
            bind:value={playerName}
            disabled={isLaunching}
            placeholder="Никнейм"
            class="nickname-input"
            maxlength={16}
          />
        </div>
      </div>
    </div>

    <!-- Progress Bar (shown when launching or syncing) -->
    {#if isLaunching}
      <div class="progress-section">
        <ProgressBar
          percentage={progressPercentage}
          label={progressStage}
          detail={progressSpeed ? `${progressSpeed} • ${progressDetail}` : progressDetail}
          height={8}
        />
      </div>
    {/if}

    <!-- Launch Action Bar -->
    <div class="launch-actions">
      <div class="status-summary">
        <div class="summary-item">
          <Icon name="server" size={14} color="var(--color-system-green)" />
          <span>Сервер: Онлайн (86/200)</span>
        </div>
        <div class="summary-item">
          <Icon name="cpu" size={14} color="var(--color-secondary-label)" />
          <span>Java 21 • 4 ГБ RAM</span>
        </div>
      </div>

      <div class="action-buttons">
        {#if isLaunching}
          <Button variant="secondary" size="large" onclick={oncancel}>
            Отмена
          </Button>
        {:else}
          <Button
            variant="primary"
            size="large"
            onclick={onlaunch}
            disabled={!playerName.trim()}
          >
            <Icon name="play" size={18} color="#ffffff" />
            <span>Играть</span>
          </Button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .play-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 20px;
    overflow-y: auto;
    font-family: var(--font-system);
  }

  .hero-banner {
    position: relative;
    height: 220px;
    border-radius: var(--radius-xl);
    overflow: hidden;
    background: linear-gradient(135deg, #0e1e38 0%, #17325c 50%, #0d4b85 100%);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: flex-end;
    padding: 28px;
    border: 1px solid var(--glass-border);
  }

  :global([data-theme="light"]) .hero-banner {
    background: linear-gradient(135deg, #1a4f8f 0%, #2f7acc 50%, #4facfe 100%);
  }

  .hero-gradient {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 80% 20%, rgba(0, 145, 255, 0.45), transparent 60%);
    pointer-events: none;
  }

  .hero-content {
    position: relative;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: #ffffff;
  }

  .hero-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    background-color: rgba(255, 255, 255, 0.18);
    backdrop-filter: blur(10px);
    font-size: var(--text-caption-1);
    font-weight: 600;
    width: fit-content;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #30d158;
    box-shadow: 0 0 8px #30d158;
  }

  .hero-title {
    font-size: var(--text-large-title);
    font-weight: 800;
    letter-spacing: var(--tracking-large-title);
    margin: 0;
    line-height: var(--line-large-title);
  }

  .hero-subtitle {
    font-size: var(--text-subheadline);
    color: rgba(255, 255, 255, 0.85);
    max-width: 520px;
    line-height: 1.4;
    margin: 0;
  }

  .launch-card {
    background-color: var(--card-bg);
    backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    box-shadow: var(--glass-shadow);
  }

  .launch-controls {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .version-group {
    flex: 2;
    min-width: 280px;
  }

  .nickname-group {
    flex: 1;
    min-width: 180px;
  }

  .control-label {
    font-size: var(--text-caption-1);
    font-weight: 600;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .nickname-input-box {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 12px;
    background-color: var(--color-tertiary-system-fill);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    transition: border-color var(--motion-duration-short) ease;
  }

  .nickname-input-box:focus-within {
    border-color: var(--color-system-blue);
    background-color: var(--card-bg);
  }

  .nickname-input {
    flex: 1;
    border: none;
    background: transparent;
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    color: var(--color-label);
    font-weight: 500;
    outline: none;
  }

  .progress-section {
    padding-top: 8px;
    animation: fadeIn 0.25s ease;
  }

  .launch-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 12px;
    border-top: 1px solid var(--color-separator);
    flex-wrap: wrap;
    gap: 16px;
  }

  .status-summary {
    display: flex;
    gap: 20px;
  }

  .summary-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
  }

  .action-buttons {
    display: flex;
    gap: 12px;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
