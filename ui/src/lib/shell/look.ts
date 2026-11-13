/** Apply the look stored in the launcher settings (theme, motion, transparency, language). */
import { theme } from '../design/theme.svelte';
import { i18n } from '../i18n';
import { season } from '../design/season.svelte';
import type { LauncherSettings } from './builds.svelte';

export function applyLook(s: LauncherSettings): void {
  const a = s.appearance;
  if (a === 'auto' || a === 'light' || a === 'dark') theme.setAppearance(a);
  if (typeof s.reduce_motion === 'boolean') theme.setReduceMotion(s.reduce_motion);
  if (typeof s.reduce_transparency === 'boolean') theme.setReduceTransparency(s.reduce_transparency);
  if (s.language === 'ru' || s.language === 'en') i18n.setLocale(s.language);
  season.setEnabled(s.winter_look !== false);
}
