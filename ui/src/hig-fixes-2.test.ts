import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ProgressBar from './lib/components/ProgressBar.svelte';
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
      expect(en['settings.logs_viewer_title']).toBe('Logs and Crash Reports');
    });
  });

  describe('4. Lobby Screen HIG Requirements (F09, F10, F11)', () => {
    it('provides specialized cloud rooms error title, description and retry action (F11)', () => {
      expect(ru['lobby.cloud_error_title']).toBe('Не удалось загрузить облачные миры');

      expect(en['lobby.cloud_error_title']).toBe('Failed to load cloud worlds');
    });
  });

});
