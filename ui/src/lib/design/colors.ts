/**
 * Apple Design System Colors & Theme Engine.
 * Sourced strictly from Apple HIG (color, dark-mode, materials) and AppleColors.java
 */

export interface ColorDefinition {
  hex: string;
  rgba: string;
  r: number;
  g: number;
  b: number;
  a: number;
}

export type AppearanceMode = 'dark' | 'light' | 'auto';
export type AccentColor = 'blue' | 'indigo' | 'purple' | 'pink' | 'orange' | 'green' | 'graphite' | 'mono';

export const ApplePalette = {
  dark: {
    label: 'rgba(255, 255, 255, 1.0)',
    secondaryLabel: 'rgba(235, 235, 245, 0.60)',
    tertiaryLabel: 'rgba(235, 235, 245, 0.30)',
    quaternaryLabel: 'rgba(235, 235, 245, 0.18)',

    systemFill: 'rgba(120, 120, 128, 0.36)',
    secondarySystemFill: 'rgba(120, 120, 128, 0.32)',
    tertiarySystemFill: 'rgba(120, 120, 128, 0.24)',
    quaternarySystemFill: 'rgba(120, 120, 128, 0.18)',

    separator: 'rgba(84, 84, 88, 0.65)',
    opaqueSeparator: '#38383A',

    systemBlue: '#0091FF',
    systemBlueOnAccent: '#FFFFFF',
    systemGreen: '#30D158',
    systemGreenAccessible: '#32D74B',
    systemRed: '#FF453A',
    systemRedAccessible: '#FF6165',
    systemOrange: '#FF9F0A',
    systemYellow: '#FFD60A',
    systemCyan: '#64D2FF',
    systemGray: '#8E8E93',

    systemBackground: '#000000',
    secondarySystemBackground: '#1C1C1E',
    tertiarySystemBackground: '#2C2C2E',

    glassRegularBg: 'rgba(28, 28, 30, 0.75)',
    glassClearBg: 'rgba(28, 28, 30, 0.40)',
    glassBorder: 'rgba(255, 255, 255, 0.14)',
  },
  light: {
    label: 'rgba(0, 0, 0, 1.0)',
    secondaryLabel: 'rgba(60, 60, 67, 0.60)',
    tertiaryLabel: 'rgba(60, 60, 67, 0.30)',
    quaternaryLabel: 'rgba(60, 60, 67, 0.18)',

    systemFill: 'rgba(120, 120, 128, 0.20)',
    secondarySystemFill: 'rgba(120, 120, 128, 0.16)',
    tertiarySystemFill: 'rgba(120, 120, 128, 0.12)',
    quaternarySystemFill: 'rgba(120, 120, 128, 0.08)',

    separator: 'rgba(60, 60, 67, 0.29)',
    opaqueSeparator: '#C6C6C8',

    systemBlue: '#0088FF',
    systemBlueOnAccent: '#FFFFFF',
    systemGreen: '#34C759',
    systemGreenAccessible: '#0D5C1E',
    systemRed: '#FF3B30',
    systemRedAccessible: '#E9152D',
    systemOrange: '#FF9500',
    systemYellow: '#FFCC00',
    systemCyan: '#00C7BE',
    systemGray: '#8E8E93',

    systemBackground: '#FFFFFF',
    secondarySystemBackground: '#F2F2F7',
    tertiarySystemBackground: '#FFFFFF',

    glassRegularBg: 'rgba(245, 245, 247, 0.78)',
    glassClearBg: 'rgba(255, 255, 255, 0.45)',
    glassBorder: 'rgba(0, 0, 0, 0.09)',
  },
} as const;

export const AccentPresets = {
  blue: { title: 'Синий', dark: '#0091FF', light: '#0088FF', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  indigo: { title: 'Индиго', dark: '#6D7CFF', light: '#6155F5', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  purple: { title: 'Фиолетовый', dark: '#DB34F2', light: '#CB30E0', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  pink: { title: 'Розовый', dark: '#FF375F', light: '#FF2D55', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  orange: { title: 'Оранжевый', dark: '#FF9230', light: '#FF8D28', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  green: { title: 'Зелёный', dark: '#30D158', light: '#34C759', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  graphite: { title: 'Графит', dark: '#8E8E93', light: '#8E8E93', onAccentDark: '#FFFFFF', onAccentLight: '#FFFFFF' },
  mono: { title: 'Белый', dark: '#FFFFFF', light: '#000000', onAccentDark: '#000000', onAccentLight: '#FFFFFF' },
} as const;

/**
 * Computes the relative luminance of an sRGB color (0.0 to 1.0)
 */
export function computeRelativeLuminance(r: number, g: number, b: number): number {
  const rs = r / 255;
  const gs = g / 255;
  const bs = b / 255;

  const R = rs <= 0.03928 ? rs / 12.92 : Math.pow((rs + 0.055) / 1.055, 2.4);
  const G = gs <= 0.03928 ? gs / 12.92 : Math.pow((gs + 0.055) / 1.055, 2.4);
  const B = bs <= 0.03928 ? bs / 12.92 : Math.pow((bs + 0.055) / 1.055, 2.4);

  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

/**
 * Computes WCAG 2.1 contrast ratio between two luminance values (1.0 to 21.0)
 */
export function computeContrastRatio(l1: number, l2: number): number {
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Checks if a foreground and background meet WCAG AA standard (>= 4.5:1 for normal text)
 */
export function meetsWcagAA(
  fg: { r: number; g: number; b: number },
  bg: { r: number; g: number; b: number },
  isLargeText = false
): { ratio: number; pass: boolean } {
  const l1 = computeRelativeLuminance(fg.r, fg.g, fg.b);
  const l2 = computeRelativeLuminance(bg.r, bg.g, bg.b);
  const ratio = computeContrastRatio(l1, l2);
  const threshold = isLargeText ? 3.0 : 4.5;
  return { ratio, pass: ratio >= threshold };
}
