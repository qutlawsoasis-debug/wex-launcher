<!--
  Wex Launcher — Stories Carousel (Apple HIG Info Card Carousel)
  Parity with old launcher StoriesCarousel.xaml & ISlideLifecycle.
  Features:
  - Top indicator progress segments with click-to-jump.
  - Multi-card support: News / Changelog, Latest Screenshot, Achievements, Playtime.
  - "Только настоящие данные": only slides with real data are included.
  - Gesture support (touch / pointer swipe left & right).
  - Apple HIG Reduce Motion support (disables animated progress bar and sliding transform).
  - Hover-to-pause auto-advance (6s interval).
  - "Скопировать для друзей" changelog copy action with toast notification.
  - Instant navigation to Workshop and Friends/Achievements.
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Icon from './icons/Icon.svelte';
  import Button from './Button.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { theme } from '../design/theme.svelte';

  export interface NewsSlideItem {
    badgeText: string;
    categoryClass: string;
    text: string;
  }

  export interface NewsSlideData {
    type: 'news';
    version: string;
    dateText?: string;
    shortTitle?: string;
    items: NewsSlideItem[];
  }

  export interface ScreenshotSlideData {
    type: 'screenshot';
    fileName: string;
    filePath: string;
    previewUrl?: string;
    createdAt?: string;
  }

  export interface AchievementsSlideData {
    type: 'achievements';
    unlockedCount: number;
    totalCount: number;
    latestAchievement?: {
      id: string;
      title: string;
      description: string;
      icon?: string;
      unlockedAt?: string;
    };
  }

  export interface PlaytimeSlideData {
    type: 'playtime';
    totalSeconds: number;
    formattedPlayTime: string;
    totalLaunches: number;
    lastPlayed?: string;
  }

  export type StorySlideData =
    | NewsSlideData
    | ScreenshotSlideData
    | AchievementsSlideData
    | PlaytimeSlideData;

  interface Props {
    slides: StorySlideData[];
    intervalMs?: number;
    onnavigatescreen?: (screen: string) => void;
    ontoast?: (toast: { type: 'info' | 'success' | 'warning' | 'error'; title: string; message: string }) => void;
  }

  let {
    slides = [],
    intervalMs = 6000,
    onnavigatescreen,
    ontoast,
  }: Props = $props();

  let activeIndex = $state(0);
  let progress = $state(0);
  let isPaused = $state(false);
  let copyFeedback = $state(false);

  // Swipe / Drag gesture state
  let isDragging = false;
  let startX = 0;
  let startY = 0;
  let dragOffset = $state(0);

  // Timer reference
  let timerInterval: any = null;
  const TICK_STEP_MS = 50;

  // Ensure activeIndex is within bounds if slides change
  $effect(() => {
    if (activeIndex >= slides.length) {
      activeIndex = Math.max(0, slides.length - 1);
    }
  });

  function startTimer() {
    stopTimer();
    if (slides.length <= 1) return;

    progress = 0;
    const increment = TICK_STEP_MS / intervalMs;

    timerInterval = setInterval(() => {
      if (isPaused || theme.reduceMotion) return;

      progress += increment;
      if (progress >= 1) {
        nextSlide();
      }
    }, TICK_STEP_MS);
  }

  function stopTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  function nextSlide() {
    if (slides.length <= 1) return;
    activeIndex = (activeIndex + 1) % slides.length;
    progress = 0;
  }

  function prevSlide() {
    if (slides.length <= 1) return;
    activeIndex = (activeIndex - 1 + slides.length) % slides.length;
    progress = 0;
  }

  function goToSlide(idx: number) {
    if (idx >= 0 && idx < slides.length) {
      activeIndex = idx;
      progress = 0;
    }
  }

  // Gesture handling
  function handlePointerDown(e: PointerEvent) {
    // Only drag with primary mouse button or touch
    if (e.button !== 0 && e.pointerType === 'mouse') return;
    isDragging = true;
    startX = e.clientX;
    startY = e.clientY;
    dragOffset = 0;
    isPaused = true;
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;

    // Favor horizontal gestures
    if (Math.abs(dx) > Math.abs(dy)) {
      dragOffset = dx;
    }
  }

  function handlePointerUp() {
    if (!isDragging) return;
    isDragging = false;
    isPaused = false;

    const threshold = 40;
    if (dragOffset < -threshold) {
      nextSlide();
    } else if (dragOffset > threshold) {
      prevSlide();
    }
    dragOffset = 0;
  }

  function handlePointerCancel() {
    isDragging = false;
    isPaused = false;
    dragOffset = 0;
  }

  // Key navigation
  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      nextSlide();
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      prevSlide();
    }
  }

  // Copy changelog for friends action
  async function copyChangelogForFriends(slide: NewsSlideData) {
    const titleLine = `Wex Launcher — ${slide.version}${slide.dateText ? ` (${slide.dateText})` : ''}`;
    const headerLine = slide.shortTitle ? `Заголовок: ${slide.shortTitle}` : '';
    const itemsText = slide.items.map((i) => `• [${i.badgeText}] ${i.text}`).join('\n');
    const fullText = [titleLine, headerLine, '', itemsText].filter(Boolean).join('\n');

    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard) {
        await navigator.clipboard.writeText(fullText);
      }
      copyFeedback = true;
      setTimeout(() => {
        copyFeedback = false;
      }, 2500);

      ontoast?.({
        type: 'info',
        title: 'Скопировано',
        message: `Список изменений ${slide.version} скопирован для друзей`,
      });
    } catch {
      copyFeedback = true;
      setTimeout(() => {
        copyFeedback = false;
      }, 2000);
    }
  }

  onMount(() => {
    startTimer();
  });

  onDestroy(() => {
    stopTimer();
  });

  let currentSlide = $derived(slides[activeIndex]);
</script>

{#if slides.length > 0 && currentSlide}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="stories-card"
    role="region"
    aria-label="Истории и активность"
    onkeydown={handleKeyDown}
    onmouseenter={() => (isPaused = true)}
    onmouseleave={() => (isPaused = false)}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerCancel}
  >
    <!-- 1. Top Story Bars Indicator (Exact parity with StoriesCarousel.xaml) -->
    {#if slides.length > 1}
      <div class="story-indicators" role="tablist">
        {#each slides as slide, idx}
          <button
            type="button"
            class="indicator-segment-btn"
            role="tab"
            aria-selected={idx === activeIndex}
            aria-label={`История ${idx + 1}`}
            onclick={(e) => {
              e.stopPropagation();
              goToSlide(idx);
            }}
          >
            <div class="indicator-track">
              <div
                class="indicator-fill"
                style="width: {idx < activeIndex ? '100%' : idx === activeIndex ? (theme.reduceMotion ? '100%' : `${progress * 100}%`) : '0%'}; transition: {theme.reduceMotion || isPaused ? 'none' : 'width 0.05s linear'};"
              ></div>
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <!-- 2. Active Slide Content -->
    <div
      class="slide-viewport"
      style="transform: {theme.reduceMotion ? 'none' : `translateX(${dragOffset}px)`};"
    >
      <!-- SLIDE: NEWS / CHANGELOG -->
      {#if currentSlide.type === 'news'}
        <div class="story-content slide-news" data-testid="story-news">
          <div class="story-header">
            <div class="header-left">
              <Icon name="play" size={15} color="var(--color-system-blue)" />
              <span class="story-title">Журнал изменений</span>
              <span class="version-pill">{currentSlide.version}</span>
            </div>
            {#if currentSlide.dateText}
              <span class="story-date">{currentSlide.dateText}</span>
            {/if}
          </div>

          <div class="news-items-list">
            {#each currentSlide.items as item, itemIdx (item.badgeText + '-' + item.text + '-' + itemIdx)}
              <div class="news-item-row">
                <span class="badge badge-{item.categoryClass}">{item.badgeText}</span>
                <span class="news-text">{item.text}</span>
              </div>
            {/each}
          </div>

          <div class="story-footer">
            <Button
              variant="secondary"
              size="small"
              onclick={() => copyChangelogForFriends(currentSlide as NewsSlideData)}
              title="Скопировать форматированный чейнджлог для Discord/Telegram"
            >
              <Icon name={copyFeedback ? 'check' : 'copy'} size={14} color="var(--color-secondary-label)" />
              <span>{copyFeedback ? 'Скопировано!' : 'Скопировать для друзей'}</span>
            </Button>
          </div>
        </div>

      <!-- SLIDE: LATEST SCREENSHOT -->
      {:else if currentSlide.type === 'screenshot'}
        <div class="story-content slide-screenshot" data-testid="story-screenshot">
          <div class="story-header">
            <div class="header-left">
              <Icon name="camera" size={15} color="var(--color-system-green)" />
              <span class="story-title">Последний скриншот</span>
            </div>
            {#if currentSlide.createdAt}
              <span class="story-date">{currentSlide.createdAt}</span>
            {/if}
          </div>

          <!-- Screenshot Click-through preview -->
          <button
            type="button"
            class="screenshot-preview-wrap"
            onclick={() => onnavigatescreen?.('workshop')}
            title="Нажмите для перехода в Мастерскую"
          >
            {#if currentSlide.previewUrl}
              <img
                src={currentSlide.previewUrl}
                alt={currentSlide.fileName}
                class="screenshot-img"
              />
            {:else}
              <div class="screenshot-placeholder">
                <Icon name="camera" size={32} color="var(--color-tertiary-label)" />
                <span class="file-name-caption">{currentSlide.fileName}</span>
              </div>
            {/if}
            <div class="screenshot-hover-badge">
              <Icon name="workshop" size={13} color="#ffffff" />
              <span>Перейти в Мастерскую</span>
            </div>
          </button>

          <div class="story-footer">
            <Button
              variant="secondary"
              size="small"
              onclick={() => onnavigatescreen?.('workshop')}
            >
              <Icon name="workshop" size={14} color="var(--color-secondary-label)" />
              <span>Все скриншоты</span>
            </Button>
          </div>
        </div>

      <!-- SLIDE: ACHIEVEMENTS -->
      {:else if currentSlide.type === 'achievements'}
        <div class="story-content slide-achievements" data-testid="story-achievements">
          <div class="story-header">
            <div class="header-left">
              <Icon name="trophy" size={15} color="var(--color-system-orange)" />
              <span class="story-title">Достижения</span>
            </div>
            <span class="story-date">
              {currentSlide.unlockedCount} из {currentSlide.totalCount}
            </span>
          </div>

          <div class="achievements-body">
            <div class="achievements-progress-row">
              <ProgressBar
                percentage={Math.round((currentSlide.unlockedCount / Math.max(1, currentSlide.totalCount)) * 100)}
                label="Прогресс достижений"
                height={6}
              />
            </div>

            {#if currentSlide.latestAchievement}
              <div class="latest-achievement-card">
                <div class="trophy-circle">
                  <Icon name="check" size={16} color="var(--color-system-orange)" />
                </div>
                <div class="achievement-text">
                  <span class="achievement-title">{currentSlide.latestAchievement.title}</span>
                  <span class="achievement-desc">{currentSlide.latestAchievement.description}</span>
                </div>
              </div>
            {:else}
              <div class="empty-achievement-text">
                Играйте, чтобы открывать достижения лаунчера
              </div>
            {/if}
          </div>

          <div class="story-footer">
            <Button
              variant="secondary"
              size="small"
              onclick={() => onnavigatescreen?.('friends')}
            >
              <Icon name="friends" size={14} color="var(--color-secondary-label)" />
              <span>Все достижения</span>
            </Button>
          </div>
        </div>

      <!-- SLIDE: PLAYTIME -->
      {:else if currentSlide.type === 'playtime'}
        <div class="story-content slide-playtime" data-testid="story-playtime">
          <div class="story-header">
            <div class="header-left">
              <Icon name="clock" size={15} color="var(--color-system-purple)" />
              <span class="story-title">Время в игре</span>
            </div>
            {#if currentSlide.totalLaunches > 0}
              <span class="story-date">Запусков: {currentSlide.totalLaunches}</span>
            {/if}
          </div>

          <div class="playtime-body">
            <div class="playtime-hero-stat">
              <span class="playtime-value">{currentSlide.formattedPlayTime}</span>
              <span class="playtime-caption">
                Суммарное время, проведённое в Minecraft
              </span>
            </div>

            {#if currentSlide.lastPlayed}
              <div class="playtime-subtext">
                Последний запуск: <span class="last-played-val">{currentSlide.lastPlayed}</span>
              </div>
            {/if}
          </div>

          <div class="story-footer">
            <div class="local-stats-hint">
              <Icon name="info" size={13} color="var(--color-tertiary-label)" />
              <span>Статистика учитывается автоматически</span>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- 3. Navigation Controls for Desktop Hover (Accessible) -->
    {#if slides.length > 1}
      <button
        type="button"
        class="nav-arrow nav-prev"
        onclick={(e) => {
          e.stopPropagation();
          prevSlide();
        }}
        aria-label="Предыдущая история"
      >
        <Icon name="chevron-left" size={14} color="var(--color-label)" />
      </button>

      <button
        type="button"
        class="nav-arrow nav-next"
        onclick={(e) => {
          e.stopPropagation();
          nextSlide();
        }}
        aria-label="Следующая история"
      >
        <Icon name="chevron-right" size={14} color="var(--color-label)" />
      </button>
    {/if}
  </div>
{/if}

<style>
  .stories-card {
    position: relative;
    background: var(--color-glass-bg);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl);
    padding: 14px 16px 12px 16px;
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    min-height: 196px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    user-select: none;
    outline: none;
    transition: border-color var(--duration-fast) var(--ease-spring), box-shadow var(--duration-fast) var(--ease-spring);
  }

  .stories-card:focus-visible {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 2px rgba(10, 132, 255, 0.35);
  }

  /* 1. Top Story Bars Indicator */
  .story-indicators {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
    width: 100%;
    z-index: 10;
  }

  .indicator-segment-btn {
    flex: 1;
    background: transparent;
    border: none;
    padding: 4px 0;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .indicator-track {
    width: 100%;
    height: 3px;
    background: rgba(255, 255, 255, 0.15);
    border-radius: 1.5px;
    overflow: hidden;
    position: relative;
  }

  :global([data-theme='light']) .indicator-track {
    background: rgba(0, 0, 0, 0.12);
  }

  .indicator-fill {
    height: 100%;
    background: var(--color-label);
    border-radius: 1.5px;
  }

  /* 2. Slide Content */
  .slide-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    transition: transform 0.15s ease-out;
  }

  .story-content {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .story-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .story-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-label);
    letter-spacing: -0.01em;
  }

  .version-pill {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    background: rgba(10, 132, 255, 0.15);
    color: var(--color-system-blue);
  }

  .story-date {
    font-size: 11px;
    color: var(--color-tertiary-label);
  }

  /* News Slide */
  .news-items-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    margin-bottom: 12px;
  }

  .news-item-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    line-height: 1.4;
  }

  .news-text {
    color: var(--color-secondary-label);
    flex: 1;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-feature {
    background: rgba(50, 215, 75, 0.15);
    color: var(--color-system-green);
  }

  .badge-improvement {
    background: rgba(10, 132, 255, 0.15);
    color: var(--color-system-blue);
  }

  .badge-fix {
    background: rgba(255, 159, 10, 0.15);
    color: var(--color-system-orange);
  }

  .badge-announcement {
    background: rgba(191, 90, 242, 0.15);
    color: var(--color-system-purple);
  }

  /* Screenshot Slide */
  .screenshot-preview-wrap {
    position: relative;
    width: 100%;
    height: 98px;
    border-radius: var(--radius-md);
    overflow: hidden;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--color-separator);
    cursor: pointer;
    margin-bottom: 10px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .screenshot-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform var(--duration-fast) var(--ease-spring);
  }

  .screenshot-preview-wrap:hover .screenshot-img {
    transform: scale(1.03);
  }

  .screenshot-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .file-name-caption {
    font-size: 11px;
    color: var(--color-secondary-label);
  }

  .screenshot-hover-badge {
    position: absolute;
    bottom: 6px;
    right: 6px;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(10px);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 500;
    color: #ffffff;
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0.9;
  }

  /* Achievements Slide */
  .achievements-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 10px;
  }

  .achievements-progress-row {
    width: 100%;
  }

  .latest-achievement-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: var(--color-fill-quaternary);
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .trophy-circle {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: rgba(255, 159, 10, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .achievement-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .achievement-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .achievement-desc {
    font-size: 11px;
    color: var(--color-secondary-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty-achievement-text {
    font-size: 12px;
    color: var(--color-secondary-label);
    padding: 8px 0;
  }

  /* Playtime Slide */
  .playtime-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    margin-bottom: 8px;
  }

  .playtime-hero-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .playtime-value {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--color-label);
    font-feature-settings: 'tnum';
  }

  .playtime-caption {
    font-size: 11px;
    color: var(--color-secondary-label);
  }

  .playtime-subtext {
    font-size: 11px;
    color: var(--color-tertiary-label);
  }

  .last-played-val {
    color: var(--color-secondary-label);
  }

  .local-stats-hint {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--color-tertiary-label);
  }

  /* Footer */
  .story-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: auto;
    padding-top: 4px;
  }

  /* Navigation Arrows */
  .nav-arrow {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: rgba(30, 30, 30, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-spring), background var(--duration-fast) var(--ease-spring);
    z-index: 20;
    padding: 0;
  }

  :global([data-theme='light']) .nav-arrow {
    background: rgba(255, 255, 255, 0.85);
    border: 1px solid rgba(0, 0, 0, 0.12);
  }

  .stories-card:hover .nav-arrow {
    opacity: 0.85;
  }

  .nav-arrow:hover {
    opacity: 1 !important;
    background: rgba(50, 50, 50, 0.9);
  }

  :global([data-theme='light']) .nav-arrow:hover {
    background: #ffffff;
  }

  .nav-prev {
    left: 8px;
  }

  .nav-next {
    right: 8px;
  }

  /* Reduced Motion */
  @media (prefers-reduced-motion: reduce) {
    .slide-viewport,
    .screenshot-img,
    .indicator-fill {
      transition: none !important;
    }
  }
</style>