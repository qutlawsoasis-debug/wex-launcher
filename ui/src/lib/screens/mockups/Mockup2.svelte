<!--
  Mockup 2: «Редакционный обзор» (macOS App Store / Split Editorial)
  Apple HIG Rules:
  - layout.md: Two-tier editorial layout (featured showcase + structured two-column content).
  - cards.md: Content grouped into clean visual cards with 8pt grid margins.
  - typography.md: Editorial hierarchy (Overline 11px uppercase, Title 1 28px, Headline 17px, Body 15px).
  - lists-and-tables.md: Clean inset lists with hairline dividers.
  - Writing: Clear, factual Russian descriptions without marketing buzzwords.
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

  let heroImage = $derived(theme.isDark ? '/art/sunset.png' : '/art/day.png');
</script>

<div class="mockup2-container">
  <!-- Top Showcase Banner Card -->
  <div class="showcase-card" style="background-image: url('{heroImage}');">
    <div class="showcase-scrim"></div>
    <div class="showcase-content">
      <span class="showcase-overline">Новое в сборке</span>
      <h2 class="showcase-title">Wex {selectedVersion}</h2>
      <p class="showcase-desc">
        Переработанное сумеречное освещение горизонта и плавная синхронизация модов в лобби.
      </p>
    </div>
  </div>

  <!-- Unified Launch Strip -->
  <div class="launch-strip">
    <div class="strip-left">
      <div class="strip-control">
        <span class="strip-label">Версия</span>
        <SegmentedControl
          options={versions}
          bind:value={selectedVersion}
          disabled={isLaunching}
        />
      </div>

      <div class="strip-separator"></div>

      <div class="strip-control">
        <span class="strip-label">Игрок</span>
        <div class="strip-input-box">
          <Icon name="friends" size={15} color="var(--color-tertiary-label)" />
          <input
            type="text"
            bind:value={playerName}
            placeholder="Никнейм"
            class="strip-input"
            maxlength={16}
            disabled={isLaunching}
          />
        </div>
      </div>

      <div class="strip-separator"></div>

      <div class="strip-status">
        <span class="status-indicator-dot"></span>
        <span class="status-runtime">{javaRuntime}</span>
      </div>
    </div>

    <div class="strip-right">
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

  {#if isLaunching}
    <div class="progress-wrap">
      <ProgressBar
        percentage={progressPercentage}
        label={progressStage}
        detail={progressSpeed ? `${progressSpeed} • ${progressDetail}` : progressDetail}
        height={8}
      />
    </div>
  {/if}

  <!-- Lower Two-Column Content Grid -->
  <div class="editorial-grid">
    <!-- Left Column: Changelog / Updates -->
    <section class="grid-column">
      <div class="column-header">
        <h3 class="section-title">Журнал изменений</h3>
        <span class="section-badge">Сборка Wex</span>
      </div>

      <div class="content-card">
        <div class="changelog-item">
          <div class="item-title-row">
            <span class="item-headline">Синхронизация модов</span>
            <span class="item-date">Сегодня</span>
          </div>
          <p class="item-text">
            Автоматическая сверка файлов конфигурации и модов при входе в кооперативное лобби.
          </p>
        </div>

        <div class="hairline-divider"></div>

        <div class="changelog-item">
          <div class="item-title-row">
            <span class="item-headline">Графический движок</span>
            <span class="item-date">Вчера</span>
          </div>
          <p class="item-text">
            Исправлено отображение диска солнца и тёплых оттенков заката над линией горизонта.
          </p>
        </div>

        <div class="hairline-divider"></div>

        <div class="changelog-item">
          <div class="item-title-row">
            <span class="item-headline">Туннелирование Playit</span>
            <span class="item-date">06.11</span>
          </div>
          <p class="item-text">
            Прямое подключение друзей через встроенный агент без настройки сетевого роутера.
          </p>
        </div>
      </div>
    </section>

    <!-- Right Column: Recent Servers & Friends Status -->
    <section class="grid-column">
      <div class="column-header">
        <h3 class="section-title">Быстрый вход</h3>
      </div>

      <div class="content-card">
        <div class="server-tile">
          <div class="tile-icon-box">
            <Icon name="server" size={18} color="var(--color-system-blue)" />
          </div>
          <div class="tile-details">
            <span class="tile-title">Локальная тестовая сеть</span>
            <span class="tile-meta">127.0.0.1:25565 • Пинг 1 мс</span>
          </div>
          <span class="tile-tag">Онлайн</span>
        </div>

        <div class="hairline-divider"></div>

        <div class="friends-status-box">
          <div class="friends-icon-box">
            <Icon name="friends" size={16} color="var(--color-tertiary-label)" />
          </div>
          <div class="friends-text">
            <span class="friends-headline">Друзья</span>
            <span class="friends-subtext">Нет друзей в игре в данный момент</span>
          </div>
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .mockup2-container {
    display: flex;
    flex-direction: column;
    padding: 24px 32px 32px;
    box-sizing: border-box;
    gap: 20px;
    height: 100%;
    overflow-y: auto;
  }

  /* Top Showcase Card */
  .showcase-card {
    position: relative;
    width: 100%;
    height: 200px;
    border-radius: var(--radius-lg);
    background-size: cover;
    background-position: center 30%;
    overflow: hidden;
    flex-shrink: 0;
    border: 1px solid var(--color-separator);
  }

  .showcase-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      rgba(0, 0, 0, 0.8) 0%,
      rgba(0, 0, 0, 0.3) 50%,
      rgba(0, 0, 0, 0.1) 100%
    );
  }

  :global([data-theme='light']) .showcase-scrim {
    background: linear-gradient(
      to top,
      rgba(255, 255, 255, 0.88) 0%,
      rgba(255, 255, 255, 0.45) 50%,
      rgba(255, 255, 255, 0.15) 100%
    );
  }

  .showcase-content {
    position: absolute;
    left: 28px;
    bottom: 22px;
    right: 28px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }

  .showcase-overline {
    font-size: var(--text-caption-2);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-system-blue);
    margin-bottom: 4px;
  }

  .showcase-title {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
    margin: 0 0 4px;
    color: var(--color-label);
  }

  .showcase-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    margin: 0;
    max-width: 580px;
    line-height: 1.4;
  }

  /* Launch Strip */
  .launch-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-secondary-system-grouped-background);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md);
    padding: 12px 18px;
    flex-shrink: 0;
  }

  .strip-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .strip-control {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .strip-label {
    font-size: var(--text-caption-2);
    font-weight: 500;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .strip-separator {
    width: 1px;
    height: 32px;
    background-color: var(--color-separator);
    margin: 0 4px;
  }

  .strip-input-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-tertiary-system-fill);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-sm);
    padding: 0 10px;
    height: 32px;
  }

  .strip-input {
    border: none;
    background: transparent;
    color: var(--color-label);
    font-size: var(--text-subheadline);
    font-weight: 500;
    width: 130px;
    outline: none;
  }

  .strip-input::placeholder {
    color: var(--color-tertiary-label);
  }

  .strip-status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 8px;
  }

  .status-indicator-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-system-green);
  }

  .status-runtime {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-weight: 500;
  }

  .progress-wrap {
    padding: 0 4px;
  }

  /* Two Column Editorial Grid */
  .editorial-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    flex: 1;
  }

  .grid-column {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .column-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .section-badge {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
  }

  .content-card {
    background: var(--color-secondary-system-grouped-background);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .changelog-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .item-title-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .item-headline {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .item-date {
    font-size: var(--text-caption-2);
    color: var(--color-tertiary-label);
  }

  .item-text {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    line-height: 1.4;
    margin: 0;
  }

  .hairline-divider {
    height: 1px;
    background-color: var(--color-separator);
  }

  .server-tile {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .tile-icon-box {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--color-tertiary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .tile-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .tile-title {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
  }

  .tile-meta {
    font-size: var(--text-caption-2);
    color: var(--color-secondary-label);
  }

  .tile-tag {
    font-size: var(--text-caption-2);
    color: var(--color-system-green);
    font-weight: 600;
  }

  .friends-status-box {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }

  .friends-icon-box {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .friends-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .friends-headline {
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
  }

  .friends-subtext {
    font-size: var(--text-caption-2);
    color: var(--color-tertiary-label);
  }
</style>
