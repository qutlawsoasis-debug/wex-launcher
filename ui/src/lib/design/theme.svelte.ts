/**
 * Svelte 5 Reactive Theme & Design System State.
 * Controls Appearance (Dark/Light/Auto), Accent color, Reduce Motion, and Reduce Transparency.
 */

import { type AppearanceMode, type AccentColor, AccentPresets } from './colors';

class ThemeManager {
  appearance = $state<AppearanceMode>('dark');
  accent = $state<AccentColor>('blue');
  reduceMotion = $state<boolean>(false);
  reduceTransparency = $state<boolean>(false);
  systemIsDark = $state<boolean>(true);

  constructor() {
    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
      this.systemIsDark = darkQuery.matches;
      darkQuery.addEventListener('change', (e) => {
        this.systemIsDark = e.matches;
        this.applyDom();
      });

      const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
      this.reduceMotion = motionQuery.matches;
      motionQuery.addEventListener('change', (e) => {
        this.reduceMotion = e.matches;
        this.applyDom();
      });
    }
  }

  get isDark(): boolean {
    if (this.appearance === 'auto') {
      return this.systemIsDark;
    }
    return this.appearance === 'dark';
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

  setReduceTransparency(enabled: boolean) {
    this.reduceTransparency = enabled;
    this.applyDom();
  }

  applyDom() {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    root.setAttribute('data-theme', this.isDark ? 'dark' : 'light');
    root.setAttribute('data-accent', this.accent);
    root.setAttribute('data-reduce-motion', String(this.reduceMotion));
    root.setAttribute('data-reduce-transparency', String(this.reduceTransparency));

    // Update dynamic accent CSS variable
    root.style.setProperty('--color-accent', this.currentAccentHex);
    root.style.setProperty('--color-on-accent', this.currentOnAccentHex);
  }
}

export const theme = new ThemeManager();
