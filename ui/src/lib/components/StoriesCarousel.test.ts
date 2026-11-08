import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import StoriesCarousel, {
  type StorySlideData,
  type NewsSlideData,
  type ScreenshotSlideData,
  type AchievementsSlideData,
  type PlaytimeSlideData,
} from './StoriesCarousel.svelte';
import PlayScreen from '../screens/PlayScreen.svelte';
import { theme } from '../design/theme.svelte';

describe('StoriesCarousel (SPEC 1.7 / Old Launcher Parity)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    theme.reduceMotion = false;

    // Polyfill PointerEvent in jsdom if not defined
    if (typeof globalThis.PointerEvent === 'undefined') {
      // @ts-expect-error jsdom polyfill
      globalThis.PointerEvent = class extends MouseEvent {
        pointerType: string;
        constructor(type: string, params: any = {}) {
          super(type, params);
          this.pointerType = params.pointerType || 'mouse';
        }
      };
    }

    return () => {
      target.remove();
    };
  });

  it('renders nothing when slides list is empty (Only real data rule)', async () => {
    const comp = mount(StoriesCarousel, {
      target,
      props: { slides: [] },
    });
    await tick();

    expect(target.querySelector('.stories-card')).toBeNull();
    unmount(comp);
  });

  it('renders single news card without indicator bars or nav arrows', async () => {
    const newsSlide: NewsSlideData = {
      type: 'news',
      version: 'v1.2.0',
      dateText: '8 ноября 2026',
      shortTitle: 'Мажорный релиз',
      items: [
        { badgeText: 'Новое', categoryClass: 'feature', text: 'Карусель историй' },
        { badgeText: 'Улучшено', categoryClass: 'improvement', text: 'Скорость анимаций' },
      ],
    };

    const comp = mount(StoriesCarousel, {
      target,
      props: { slides: [newsSlide] },
    });
    await tick();

    expect(target.querySelector('.stories-card')).not.toBeNull();
    expect(target.querySelector('[data-testid="story-news"]')).not.toBeNull();
    expect(target.textContent).toContain('Журнал изменений');
    expect(target.textContent).toContain('v1.2.0');
    expect(target.textContent).toContain('Карусель историй');

    // Single slide: no indicators or navigation arrows
    expect(target.querySelector('.story-indicators')).toBeNull();
    expect(target.querySelector('.nav-arrow')).toBeNull();

    unmount(comp);
  });

  it('copies changelog for friends to clipboard and fires toast', async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    const toastSpy = vi.fn();
    const newsSlide: NewsSlideData = {
      type: 'news',
      version: 'v1.0.0',
      dateText: '8 ноября 2026',
      shortTitle: 'Стартовый релиз',
      items: [
        { badgeText: 'Новое', categoryClass: 'feature', text: 'Поддержка Fabric 1.21.1' },
      ],
    };

    const comp = mount(StoriesCarousel, {
      target,
      props: {
        slides: [newsSlide],
        ontoast: toastSpy,
      },
    });
    await tick();

    const copyBtn = target.querySelector('button[title*="Discord"]') as HTMLButtonElement;
    expect(copyBtn).not.toBeNull();
    copyBtn.click();
    await tick();

    expect(writeTextMock).toHaveBeenCalled();
    const copiedText = writeTextMock.mock.calls[0][0];
    expect(copiedText).toContain('Wex Launcher — v1.0.0');
    expect(copiedText).toContain('Поддержка Fabric 1.21.1');
    expect(toastSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Скопировано',
      })
    );

    unmount(comp);
  });

  it('renders all 4 story types with indicator segments and navigation', async () => {
    const navSpy = vi.fn();

    const slides: StorySlideData[] = [
      {
        type: 'news',
        version: 'v1.0.0',
        items: [{ badgeText: 'Новое', categoryClass: 'feature', text: 'Чейнджлог' }],
      },
      {
        type: 'screenshot',
        fileName: '2026-11-08_19.00.00.png',
        filePath: 'C:/games/wex/screenshots/2026-11-08_19.00.00.png',
        createdAt: '19:00',
      },
      {
        type: 'achievements',
        unlockedCount: 4,
        totalCount: 14,
        latestAchievement: {
          id: 'first_launch',
          title: 'Первый старт',
          description: 'Успешный запуск Minecraft',
        },
      },
      {
        type: 'playtime',
        totalSeconds: 7200,
        formattedPlayTime: '2 ч. 0 мин.',
        totalLaunches: 5,
        lastPlayed: '08.11.2026',
      },
    ];

    const comp = mount(StoriesCarousel, {
      target,
      props: {
        slides,
        onnavigatescreen: navSpy,
      },
    });
    await tick();

    // 4 indicator buttons
    const indicators = target.querySelectorAll('.indicator-segment-btn');
    expect(indicators.length).toBe(4);

    // Initial slide: News
    expect(target.querySelector('[data-testid="story-news"]')).not.toBeNull();

    // Jump to slide 1: Screenshot
    (indicators[1] as HTMLButtonElement).click();
    await tick();
    expect(target.querySelector('[data-testid="story-screenshot"]')).not.toBeNull();
    expect(target.textContent).toContain('Последний скриншот');
    expect(target.textContent).toContain('2026-11-08_19.00.00.png');

    // Click on screenshot navigates to workshop
    const previewBtn = target.querySelector('.screenshot-preview-wrap') as HTMLButtonElement;
    expect(previewBtn).not.toBeNull();
    previewBtn.click();
    expect(navSpy).toHaveBeenCalledWith('workshop');

    // Jump to slide 2: Achievements
    (indicators[2] as HTMLButtonElement).click();
    await tick();
    expect(target.querySelector('[data-testid="story-achievements"]')).not.toBeNull();
    expect(target.textContent).toContain('Достижения');
    expect(target.textContent).toContain('4 из 14');
    expect(target.textContent).toContain('Первый старт');

    // Jump to slide 3: Playtime
    (indicators[3] as HTMLButtonElement).click();
    await tick();
    expect(target.querySelector('[data-testid="story-playtime"]')).not.toBeNull();
    expect(target.textContent).toContain('Время в игре');
    expect(target.textContent).toContain('2 ч. 0 мин.');
    expect(target.textContent).toContain('Запусков: 5');

    // Navigation arrow next wraps to slide 0
    const nextBtn = target.querySelector('.nav-next') as HTMLButtonElement;
    expect(nextBtn).not.toBeNull();
    nextBtn.click();
    await tick();
    expect(target.querySelector('[data-testid="story-news"]')).not.toBeNull();

    unmount(comp);
  });

  it('supports swipe gesture dragging to change slides', async () => {
    const slides: StorySlideData[] = [
      {
        type: 'news',
        version: '1.0',
        items: [{ badgeText: 'Инфо', categoryClass: 'general', text: 'Слайд 1' }],
      },
      {
        type: 'playtime',
        totalSeconds: 3600,
        formattedPlayTime: '1 ч. 0 мин.',
        totalLaunches: 1,
      },
    ];

    const comp = mount(StoriesCarousel, {
      target,
      props: { slides },
    });
    await tick();

    const card = target.querySelector('.stories-card') as HTMLElement;
    expect(card).not.toBeNull();

    // Swipe left (drag -70px) -> advance to next slide
    card.dispatchEvent(new MouseEvent('pointerdown', { clientX: 200, clientY: 100, bubbles: true } as any));
    card.dispatchEvent(new MouseEvent('pointermove', { clientX: 130, clientY: 100, bubbles: true } as any));
    card.dispatchEvent(new MouseEvent('pointerup', { clientX: 130, clientY: 100, bubbles: true } as any));
    await tick();

    expect(target.querySelector('[data-testid="story-playtime"]')).not.toBeNull();

    unmount(comp);
  });

  it('integrates seamlessly with PlayScreen and respects empty data exclusion', async () => {
    // When PlayScreen is mounted without news/screenshots/achievements/playtime
    const compEmpty = mount(PlayScreen, {
      target,
      props: {
        initialNews: [],
        initialScreenshots: [],
        initialAchievements: null,
        initialPlaytime: null,
      },
    });
    await tick();

    // Card without real data is NOT shown
    expect(target.querySelector('.updates-card')).toBeNull();
    unmount(compEmpty);
    target.innerHTML = '';

    // When PlayScreen has news and playtime
    const compWithData = mount(PlayScreen, {
      target,
      props: {
        initialNews: [
          {
            version: 'v1.1.0',
            date_text: 'Сегодня',
            short_title: 'Обновление лаунчера',
            items: [{ text: '[Новое] Инфокарточки' }],
          },
        ],
        initialPlaytime: {
          total_seconds: 5400,
          total_launches: 3,
          formatted_play_time: '1 ч. 30 мин.',
        },
      },
    });
    await tick();

    expect(target.querySelector('.updates-card')).not.toBeNull();
    expect(target.querySelector('.story-indicators')).not.toBeNull();
    expect(target.textContent).toContain('Инфокарточки');
    unmount(compWithData);
  });
});
