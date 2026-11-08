/**
 * Supported Locales and Type Definitions for Wex Launcher i18n
 */

export type SupportedLocale = 'ru' | 'en';

export interface LocaleOption {
  value: SupportedLocale;
  label: string;
}

export const SUPPORTED_LOCALES: LocaleOption[] = [
  { value: 'ru', label: 'Русский' },
  { value: 'en', label: 'English' },
];
