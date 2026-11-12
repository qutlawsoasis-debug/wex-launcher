import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ProgressBar from './lib/components/ProgressBar.svelte';
import List from './lib/components/List.svelte';
import LobbyScreen from './lib/screens/LobbyScreen.svelte';
import FriendsScreen from './lib/screens/FriendsScreen.svelte';
import { theme } from './lib/design/theme.svelte';
import { ru } from './lib/i18n/locales/ru';
import { en } from './lib/i18n/locales/en';

let target: HTMLDivElement;

beforeEach(() => {
  target = document.createElement('div');
  document.body.appendChild(target);
  localStorage.clear();
  localStorage.setItem('wex_onboarding_completed', 'true');
});

afterEach(() => {
  target.remove();
  document.body.innerHTML = '';
});

describe('Apple HIG Fixes 2 Verification (Settings, Lobby, Friends)', () => {
  describe('1. ProgressBar hideInfo and Title Duplication (F12)', () => {
    it('suppresses .progress-info when hideInfo={true}', () => {
      const comp = mount(ProgressBar, {
        target,
        props: {
          percentage: 45,
          label: 'Тестовое достижение',
          hideInfo: true,
        },
      });

      expect(target.querySelector('.progress-info')).toBeNull();
      const track = target.querySelector('.progress-track');
      expect(track).not.toBeNull();
      const fill = target.querySelector('.progress-fill');
      expect(fill).not.toBeNull();
      expect(fill?.getAttribute('aria-label')).toBe('Тестовое достижение');

      unmount(comp);
    });

    it('renders .progress-info when hideInfo is false or omitted', () => {
      const comp = mount(ProgressBar, {
        target,
        props: {
          percentage: 45,
          label: 'Обычный прогресс',
        },
      });

      expect(target.querySelector('.progress-info')).not.toBeNull();
      expect(target.querySelector('.progress-label')?.textContent).toBe('Обычный прогресс');
      expect(target.querySelector('.progress-detail')?.textContent).toBe('45%');

      unmount(comp);
    });
  });

  describe('2. List Keyboard Navigation & Slider Independence (F19)', () => {
    it('does not hijack ArrowUp/ArrowDown when focus is inside a slider/range input', async () => {
      const comp = mount(List, {
        target,
        props: {
          header: 'Тестовый список',
        },
      });

      await tick();
      const listContainer = target.querySelector<HTMLDivElement>('.list-container');
      expect(listContainer).not.toBeNull();

      // Create a slider inside the list container
      const slider = document.createElement('input');
      slider.type = 'range';
      slider.min = '1024';
      slider.max = '8192';
      slider.step = '512';
      slider.value = '4096';
      listContainer?.appendChild(slider);
      slider.focus();

      const event = new KeyboardEvent('keydown', {
        key: 'ArrowUp',
        bubbles: true,
        cancelable: true,
      });

      slider.dispatchEvent(event);
      // Event must NOT be preventDefaulted, allowing native slider control
      expect(event.defaultPrevented).toBe(false);

      unmount(comp);
    });

    it('excludes elements with aria-hidden="true" or tabindex="-1" from navigation', async () => {
      const comp = mount(List, {
        target,
        props: {
          header: 'Навигация',
        },
      });

      await tick();
      const listContainer = target.querySelector<HTMLDivElement>('.list-container');
      expect(listContainer).not.toBeNull();

      const btn1 = document.createElement('button');
      btn1.textContent = 'Кнопка 1';
      const hiddenInput = document.createElement('input');
      hiddenInput.setAttribute('aria-hidden', 'true');
      const btn2 = document.createElement('button');
      btn2.textContent = 'Кнопка 2';

      listContainer?.appendChild(btn1);
      listContainer?.appendChild(hiddenInput);
      listContainer?.appendChild(btn2);

      btn1.focus();
      expect(document.activeElement).toBe(btn1);

      listContainer?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
      // Focus should jump over aria-hidden input directly to btn2
      expect(document.activeElement).toBe(btn2);

      unmount(comp);
    });
  });

  describe('3. Settings Screen HIG Requirements (F16, F17, F18, F20, F21)', () => {
    it('sets initial data-theme on documentElement at startup (F18)', () => {
      expect(document.documentElement.getAttribute('data-theme')).toMatch(/^(dark|light)$/);
    });

    it('removes technical jargon from user-facing descriptions (F21)', () => {
      // PrewarmGame is removed from fast launch subtitle
      expect(ru['settings.fast_launch_subtitle']).not.toContain('PrewarmGame');
      expect(ru['settings.fast_launch_subtitle']).toBe('Заранее подготавливать Java для запуска игры');
      expect(en['settings.fast_launch_subtitle']).toBe('Prewarm Java runtime for faster game startup');

      // Storage section explanations are friendly and without raw file names
      expect(ru['settings.logs_viewer_title']).toBe('Журнал и отчёты о сбоях');
      expect(ru['settings.logs_viewer_desc']).toBe('Откройте журнал игры и отчёты, чтобы разобраться в сбое.');
      expect(en['settings.logs_viewer_title']).toBe('Logs and Crash Reports');
      expect(en['settings.logs_viewer_desc']).toBe('Open game log and crash reports to diagnose issues.');

      // About section title does not have parenthetical English
      expect(ru['settings.about_title']).toBe('О программе');
      expect(ru['settings.about_title']).not.toContain('(Wex Launcher)');
    });
  });

  describe('4. Lobby Screen HIG Requirements (F09, F10, F11)', () => {
    it('displays honest empty callout when no local LAN worlds are present (F09)', () => {
      const comp = mount(LobbyScreen, { target });

      const text = target.textContent || '';
      // Fabricated worlds must NOT be present
      expect(text).not.toContain('Выживание в тайге');
      expect(text).not.toContain('Творческий полигон');
      expect(text).not.toContain('Redstone Experiments');

      // Empty state text must be present
      expect(text).toContain(ru['lobby.open_world_lan_empty']);

      // Create room button must be disabled when no world is selected
      const createBtn = target.querySelector<HTMLButtonElement>('[data-testid="create-room-btn"] button');
      expect(createBtn).not.toBeNull();
      expect(createBtn?.disabled).toBe(true);

      unmount(comp);
    });

    it('does not contain parenthetical English in LAN section header', () => {
      const comp = mount(LobbyScreen, { target });

      const header = target.querySelector('.section-title');
      expect(header).not.toBeNull();
      expect(header?.textContent).not.toContain('(LAN)');
      expect(ru['lobby.lan_section_title']).not.toContain('(LAN)');
      expect(en['lobby.lan_section_title']).not.toContain('(LAN)');

      unmount(comp);
    });

    it('provides specialized cloud rooms error title, description and retry action (F11)', () => {
      expect(ru['lobby.cloud_error_title']).toBe('Не удалось загрузить облачные миры');
      expect(ru['lobby.cloud_error_desc']).toBe('Проверьте подключение к интернету и повторите попытку.');
      expect(ru['lobby.cloud_error_retry']).toBe('Повторить');

      expect(en['lobby.cloud_error_title']).toBe('Failed to load cloud worlds');
      expect(en['lobby.cloud_error_desc']).toBe('Check your internet connection and try again.');
      expect(en['lobby.cloud_error_retry']).toBe('Retry');
    });
  });

  describe('5. Friends Screen HIG Requirements (F12)', () => {
    it('does not contain parenthetical English in add friend input', () => {
      const comp = mount(FriendsScreen, { target });

      const input = target.querySelector<HTMLInputElement>('.friend-code-input');
      expect(input).not.toBeNull();
      expect(input?.placeholder).not.toContain('(#A7X9)');
      expect(input?.getAttribute('aria-label')).not.toContain('(#A7X9)');

      unmount(comp);
    });

    it('achievement row suppresses duplicate title and percentage via hideInfo={true} (F12)', async () => {
      const comp = mount(FriendsScreen, {
        target,
        props: {
          tab: 'achievements',
        },
      });

      await new Promise((r) => setTimeout(r, 50));

      const achRows = Array.from(target.querySelectorAll('.achievement-row'));
      expect(achRows.length).toBeGreaterThan(0);

      for (const row of achRows) {
        const titleEl = row.querySelector('.ach-title');
        expect(titleEl).not.toBeNull();

        // Inside the achievement progress bar, .progress-info must NOT exist
        const progInfo = row.querySelector('.ach-progress-bar .progress-info');
        expect(progInfo).toBeNull();
      }

      unmount(comp);
    });
  });
});
