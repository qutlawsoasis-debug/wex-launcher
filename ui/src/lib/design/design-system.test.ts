import { describe, it, expect } from 'vitest';
import {
  ApplePalette,
  AccentPresets,
  computeRelativeLuminance,
  computeContrastRatio,
  meetsWcagAA,
} from './colors';
import { MotionCurves } from './motion';
import { theme } from './theme.svelte';

describe('Apple Design System Tokens', () => {
  it('defines dark and light system colors adhering to AppleColors.java', () => {
    expect(ApplePalette.dark.systemBlue).toBe('#0091FF');
    expect(ApplePalette.light.systemBlue).toBe('#0088FF');
    expect(ApplePalette.dark.systemGreen).toBe('#30D158');
    expect(ApplePalette.light.systemGreen).toBe('#34C759');
    expect(ApplePalette.dark.systemRed).toBe('#FF4245');
    expect(ApplePalette.light.systemRed).toBe('#FF383C');
    expect(ApplePalette.dark.label).toBe('#FFFFFF');
    expect(ApplePalette.light.label).toBe('#000000');
    // «Акцент Blue вместо Cyan»: #64D2FF is never part of the palette.
    for (const mode of ['light', 'dark'] as const) {
      expect(Object.values(ApplePalette[mode]).map((v) => v.toUpperCase())).not.toContain('#64D2FF');
    }
  });

  it('guarantees WCAG 2.1 AA contrast ratio >= 4.5:1 for text labels on backgrounds', () => {
    // White text on dark background (#000000)
    const whiteOnDark = meetsWcagAA({ r: 255, g: 255, b: 255 }, { r: 0, g: 0, b: 0 });
    expect(whiteOnDark.ratio).toBe(21);
    expect(whiteOnDark.pass).toBe(true);

    // Black text on light background (#FFFFFF)
    const blackOnLight = meetsWcagAA({ r: 0, g: 0, b: 0 }, { r: 255, g: 255, b: 255 });
    expect(blackOnLight.ratio).toBe(21);
    expect(blackOnLight.pass).toBe(true);

    // Primary button text on systemBlue: black per «Текст на акценте Aura», 4.5:1 for normal text.
    const onAccentDark = meetsWcagAA({ r: 0, g: 0, b: 0 }, { r: 0, g: 145, b: 255 });
    expect(onAccentDark.ratio).toBeGreaterThan(4.5);
    expect(onAccentDark.pass).toBe(true);

    const onAccentLight = meetsWcagAA({ r: 0, g: 0, b: 0 }, { r: 0, g: 136, b: 255 });
    expect(onAccentLight.ratio).toBeGreaterThan(4.5);
    expect(onAccentLight.pass).toBe(true);
  });

  it('provides all 8 Apple accent color presets with valid onAccent contrast', () => {
    const keys = ['blue', 'indigo', 'purple', 'pink', 'orange', 'green', 'graphite', 'mono'] as const;
    for (const key of keys) {
      const preset = AccentPresets[key];
      expect(preset).toBeDefined();
      expect(preset.dark).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(preset.light).toMatch(/^#[0-9A-Fa-f]{6}$/);
      // The text colour on every accent passes WCAG AA (the card's 4.5:1 rule).
      for (const [bg, fg] of [[preset.dark, preset.onAccentDark], [preset.light, preset.onAccentLight]]) {
        const c = (h: string) => ({ r: parseInt(h.slice(1, 3), 16), g: parseInt(h.slice(3, 5), 16), b: parseInt(h.slice(5, 7), 16) });
        expect(meetsWcagAA(c(fg), c(bg)).pass, `${key} ${fg} on ${bg}`).toBe(true);
      }
    }
  });

  it('defines Apple spring and interactive motion curves', () => {
    expect(MotionCurves.springInteractive.cssBezier).toContain('cubic-bezier');
    expect(MotionCurves.springInteractive.durationMs).toBe(250);
    expect(MotionCurves.sheetPresentation.durationMs).toBe(320);
    expect(MotionCurves.pressFeedback.durationMs).toBe(120);
  });

  it('manages theme modes and accessibility settings reactively', () => {
    theme.setAppearance('dark');
    expect(theme.appearance).toBe('dark');
    expect(theme.isDark).toBe(true);

    theme.setAppearance('light');
    expect(theme.appearance).toBe('light');
    expect(theme.isDark).toBe(false);

    theme.setAccent('purple');
    expect(theme.accent).toBe('purple');
    expect(theme.currentAccentHex).toBe(AccentPresets.purple.light);

    theme.setReduceMotion(true);
    expect(theme.reduceMotion).toBe(true);

    theme.setReduceTransparency(true);
    expect(theme.reduceTransparency).toBe(true);

    // Reset back to defaults
    theme.setAppearance('dark');
    theme.setAccent('blue');
    theme.setReduceMotion(false);
    theme.setReduceTransparency(false);
  });
});
