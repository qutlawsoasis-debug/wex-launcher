/**
 * Svelte 5 Reactive Internationalization (i18n) Manager
 *
 * Rules:
 * - Reactive language switching without application reload
 * - High-speed dictionary lookup with fallback to Russian and key
 * - Parameter substitution: {param}
 * - Syncs with document.documentElement.lang and localStorage
 */

import { ru, type TranslationKey } from './locales/ru';
import { en } from './locales/en';
import type { SupportedLocale } from './types';

export class I18nManager {
  locale = $state<SupportedLocale>('ru');

  constructor() {
    if (typeof window !== 'undefined') {
      try {
        const stored = localStorage.getItem('wex-locale');
        if (stored === 'en' || stored === 'ru') {
          this.locale = stored;
        }
      } catch {
        // Fallback for sandboxed storage
      }
    }
  }

  setLocale(newLocale: SupportedLocale) {
    if (this.locale === newLocale) return;
    this.locale = newLocale;

    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem('wex-locale', newLocale);
      } catch {
        // Storage unavailable
      }
    }

    if (typeof document !== 'undefined') {
      document.documentElement.lang = newLocale;
    }
  }

  t(key: TranslationKey | (string & {}), params?: Record<string, string | number>): string {
    // Access this.locale to register Svelte 5 fine-grained dependency
    const currentLocale = this.locale;
    const dictionary = currentLocale === 'en' ? en : ru;
    let template = (dictionary as Record<string, string>)[key];

    // Fallback to Russian dictionary if missing in target locale
    if (template === undefined) {
      template = (ru as Record<string, string>)[key];
    }

    // Fallback to key itself if missing completely
    if (template === undefined) {
      return key;
    }

    // Parameter interpolation
    if (params) {
      for (const [paramKey, paramVal] of Object.entries(params)) {
        template = template.replaceAll(`{${paramKey}}`, String(paramVal));
      }
    }

    return template;
  }
}

export const i18n = new I18nManager();

/**
 * Convenient reactive translation helper
 */
export const t = (
  key: TranslationKey | (string & {}),
  params?: Record<string, string | number>
): string => i18n.t(key, params);
