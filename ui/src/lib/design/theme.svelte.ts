/**
 * Svelte 5 reactive theme state.
 *
 * appearance: 'auto' follows Windows (HIG › Dark Mode), 'light'/'dark' force one
 *   (aura-knowledge: Своя настройка темы, confirmed by the owner).
 * reduceMotion / reduceTransparency: the app's own switches, saved in the config.
 *   The system settings are tracked separately and always win (HIG › Accessibility,
 *   Motion: "Make motion optional").
 *
 * DOM contract read by wex.css:
 *   data-theme="light|dark"            resolved theme
 *   data-reduce-motion="true|false"    system OR app OR effects off
 *   data-reduce-transparency="true|false"
 */

import { type AppearanceMode, type AccentColor, AccentPresets } from './colors';
import { motionState, type EffectsMode } from '../motion';

class ThemeManager {
  appearance = $state<AppearanceMode>('auto');
  accent = $state<AccentColor>('blue');
  /** The app's Animations switch turned off (saved as reduce_motion). */
  reduceMotion = $state<boolean>(false);
  /** The app's Reduce Transparency switch (saved as reduce_transparency). */
  reduceTransparency = $state<boolean>(false);
  effectsMode = $state<EffectsMode>('full');
  systemIsDark = $state<boolean>(true);
  systemReduceMotion = $state<boolean>(false);
  systemReduceTransparency = $state<boolean>(false);

  constructor() {
    if (typeof window === 'undefined') return;
    try {
      const saved = localStorage.getItem('wex_theme');
      if (saved === 'dark' || saved === 'light' || saved === 'auto') {
        this.appearance = saved;
      }
    } catch {}

    if (typeof window.matchMedia === 'function') {
      const watch = (query: string, set: (v: boolean) => void) => {
        const mq = window.matchMedia(query);
        set(mq.matches);
        mq.addEventListener?.('change', (e) => {
          set(e.matches);
          this.applyDom();
        });
      };
      watch('(prefers-color-scheme: dark)', (v) => (this.systemIsDark = v));
      watch('(prefers-reduced-motion: reduce)', (v) => (this.systemReduceMotion = v));
      watch('(prefers-reduced-transparency: reduce)', (v) => (this.systemReduceTransparency = v));
    }
    this.applyDom();
  }

  get isDark(): boolean {
    return this.appearance === 'auto' ? this.systemIsDark : this.appearance === 'dark';
  }

  /** Motion is reduced by the system, by the app's switch, or by effects off. */
  get motionReduced(): boolean {
    return this.systemReduceMotion || this.reduceMotion || this.effectsMode === 'off';
  }

  /** Materials are solid: system setting or the app's switch. */
  get transparencyReduced(): boolean {
    return this.systemReduceTransparency || this.reduceTransparency;
  }

  get currentAccentHex(): string {
    const preset = AccentPresets[this.accent] || AccentPresets.blue;
    return this.isDark ? preset.dark : preset.light;
  }

  get currentOnAccentHex(): string {
    const preset = AccentPresets[this.accent] || AccentPresets.blue;
    return this.isDark ? preset.onAccentDark : preset.onAccentLight;
  }

  setAppearance(mode: AppearanceMode) {
    this.appearance = mode;
    try {
      localStorage.setItem('wex_theme', mode);
    } catch {}
    this.applyDom();
  }

  setAccent(accent: AccentColor) {
    this.accent = accent;
    this.applyDom();
  }

  setReduceMotion(enabled: boolean) {
    this.reduceMotion = enabled;
    this.applyDom();
  }

  setEffectsMode(mode: EffectsMode) {
    this.effectsMode = mode;
    motionState.setMode(mode);
    this.applyDom();
  }

  setReduceTransparency(enabled: boolean) {
    this.reduceTransparency = enabled;
    this.applyDom();
  }

  applyDom() {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    root.setAttribute('data-theme', this.isDark ? 'dark' : 'light');
    root.setAttribute('data-reduce-motion', String(this.motionReduced));
    root.setAttribute('data-reduce-transparency', String(this.transparencyReduced));
    motionState.systemReduceMotion = this.motionReduced;
  }
}

export const theme = new ThemeManager();
