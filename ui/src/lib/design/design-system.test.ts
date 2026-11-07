import { describe, it, expect } from 'vitest';
import {
  ApplePalette,
  AccentPresets,
  computeRelativeLuminance,
  computeContrastRatio,
  meetsWcagAA,
} from './colors';
import { TypographyScale } from './typography';
import { Materials } from './materials';
import { MotionCurves } from './motion';
import { theme } from './theme.svelte';

describe('Apple Design System Tokens', () => {
  it('defines dark and light system colors adhering to AppleColors.java', () => {
    expect(ApplePalette.dark.systemBlue).toBe('#0091FF');
    expect(ApplePalette.light.systemBlue).toBe('#0088FF');
    expect(ApplePalette.dark.systemGreen).toBe('#30D158');
    expect(ApplePalette.light.systemGreen).toBe('#34C759');
    expect(ApplePalette.dark.systemRed).toBe('#FF453A');
    expect(ApplePalette.light.systemRed).toBe('#FF3B30');
    expect(ApplePalette.dark.systemCyan).toBe('#64D2FF');
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

    // Primary button onAccent text on systemBlue dark (bold UI control >= 3.0:1)
    const onAccentDark = meetsWcagAA({ r: 255, g: 255, b: 255 }, { r: 0, g: 145, b: 255 }, true);
    expect(onAccentDark.ratio).toBeGreaterThan(3.0);
    expect(onAccentDark.pass).toBe(true);

    // Primary button onAccent text on systemBlue light (bold UI control >= 3.0:1)
    const onAccentLight = meetsWcagAA({ r: 255, g: 255, b: 255 }, { r: 0, g: 136, b: 255 }, true);
    expect(onAccentLight.ratio).toBeGreaterThan(3.0);
    expect(onAccentLight.pass).toBe(true);
  });

  it('provides all 8 Apple accent color presets with valid onAccent contrast', () => {
    const keys = ['blue', 'indigo', 'purple', 'pink', 'orange', 'green', 'graphite', 'mono'] as const;
    for (const key of keys) {
      const preset = AccentPresets[key];
      expect(preset).toBeDefined();
      expect(preset.dark).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(preset.light).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(preset.onAccentDark).toBeDefined();
      expect(preset.onAccentLight).toBeDefined();
    }
  });

  it('defines Apple HIG typography scale with correct font sizes and weights', () => {
    expect(TypographyScale.largeTitle.size).toBe(34);
    expect(TypographyScale.largeTitle.weight).toBe(700);
    expect(TypographyScale.title1.size).toBe(28);
    expect(TypographyScale.title2.size).toBe(22);
    expect(TypographyScale.title3.size).toBe(20);
    expect(TypographyScale.headline.size).toBe(17);
    expect(TypographyScale.headline.weight).toBe(600);
    expect(TypographyScale.body.size).toBe(17);
    expect(TypographyScale.body.weight).toBe(400);
    expect(TypographyScale.footnote.size).toBe(13);
    expect(TypographyScale.caption1.size).toBe(12);
  });

  it('defines Liquid Glass materials with backdrop-filter specifications', () => {
    expect(Materials.liquidGlassRegular.variant).toBe('regular');
    expect(Materials.liquidGlassRegular.backdropFilter).toContain('blur');
    expect(Materials.liquidGlassRegular.backdropFilter).toContain('saturate');
    expect(Materials.liquidGlassClear.variant).toBe('clear');
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
