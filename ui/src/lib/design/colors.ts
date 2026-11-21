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

/**
 * The one palette of Wex. wex.css writes the same values as CSS variables and
 * palette.test.ts parses wex.css and checks that every variable equals the entry
 * here (see CssPaletteMap), so the two cannot drift apart.
 *
 * Sources (aura-knowledge, status verified):
 *   label, secondaryLabel, tertiaryLabel, quaternaryLabel: «Подписи Aura»
 *   systemFill … quaternarySystemFill (#787880 at 20/16/12/8 %, dark 36/32/24/18 %): «Системные заливки Aura»
 *   separator: «Сепаратор Aura»
 *   systemBlue: «Акцент Aura», «Акцент Blue вместо Cyan», HIG card «Синий»
 *   systemRed, systemGreen: «Красный и зелёный Aura», HIG cards «Красный», «Зелёный»
 *   systemOrange, systemGray, systemGray4–6: HIG cards «Оранжевый», «Серый», «Серый 4/5/6»
 *   onAccent on Blue is black in both themes: «Текст на акценте Aura» (WCAG 4.5:1)
 * System backgrounds: HIG › Color names them but gives no RGB ("Apple не указывает");
 * the values are the matching grays: secondarySystemBackground = systemGray6,
 * tertiarySystemBackground = white / systemGray5 (dark).
 */
export const ApplePalette = {
  dark: {
    label: '#FFFFFF',
    secondaryLabel: 'rgba(235, 235, 245, 0.60)',
    tertiaryLabel: 'rgba(235, 235, 245, 0.30)',
    quaternaryLabel: 'rgba(235, 235, 245, 0.18)',

    systemFill: 'rgba(120, 120, 128, 0.36)',
    secondarySystemFill: 'rgba(120, 120, 128, 0.32)',
    tertiarySystemFill: 'rgba(120, 120, 128, 0.24)',
    quaternarySystemFill: 'rgba(120, 120, 128, 0.18)',

    separator: 'rgba(84, 84, 88, 0.65)',

    systemBlue: '#0091FF',
    systemBlueOnAccent: '#000000',
    systemGreen: '#30D158',
    systemRed: '#FF4245',
    systemOrange: '#FF9230',
    systemGray: '#8E8E93',
    systemGray4: '#3A3A3C',
    systemGray5: '#2C2C2E',
    systemGray6: '#1C1C1E',

    systemBackground: '#000000',
    secondarySystemBackground: '#1C1C1E',
    tertiarySystemBackground: '#2C2C2E',
  },
  light: {
    label: '#000000',
    secondaryLabel: 'rgba(60, 60, 67, 0.60)',
    tertiaryLabel: 'rgba(60, 60, 67, 0.30)',
    quaternaryLabel: 'rgba(60, 60, 67, 0.18)',

    systemFill: 'rgba(120, 120, 128, 0.20)',
    secondarySystemFill: 'rgba(120, 120, 128, 0.16)',
    tertiarySystemFill: 'rgba(120, 120, 128, 0.12)',
    quaternarySystemFill: 'rgba(120, 120, 128, 0.08)',

    separator: 'rgba(60, 60, 67, 0.29)',

    systemBlue: '#0088FF',
    systemBlueOnAccent: '#000000',
    systemGreen: '#34C759',
    systemRed: '#FF383C',
    systemOrange: '#FF8D28',
    systemGray: '#8E8E93',
    systemGray4: '#D1D1D6',
    systemGray5: '#E5E5EA',
    systemGray6: '#F2F2F7',

    systemBackground: '#FFFFFF',
    secondarySystemBackground: '#F2F2F7',
    tertiarySystemBackground: '#FFFFFF',
  },
} as const;

export type PaletteKey = keyof typeof ApplePalette.light;

/**
 * Which wex.css variable carries which palette entry. Every colour variable of
 * the light block and of both dark blocks must equal ApplePalette[mode][key].
 */
export const CssPaletteMap = {
  blue: 'systemBlue',
  'blue-ink': 'systemBlueOnAccent',
  green: 'systemGreen',
  red: 'systemRed',
  orange: 'systemOrange',
  gray: 'systemGray',
  label: 'label',
  'label-2': 'secondaryLabel',
  'label-3': 'tertiaryLabel',
  'label-4': 'quaternaryLabel',
  fill: 'systemFill',
  'fill-2': 'secondarySystemFill',
  'fill-3': 'tertiarySystemFill',
  'fill-4': 'quaternarySystemFill',
  separator: 'separator',
  base: 'secondarySystemBackground',
  card: 'tertiarySystemBackground',
} as const satisfies Record<string, PaletteKey>;

/** Accent presets («Акцент Aura»); text on each accent from «Текст на акценте Aura». */
export const AccentPresets = {
  blue: { title: 'Blue', dark: '#0091FF', light: '#0088FF', onAccentDark: '#000000', onAccentLight: '#000000' },
  indigo: { title: 'Indigo', dark: '#6D7CFF', light: '#6155F5', onAccentDark: '#000000', onAccentLight: '#FFFFFF' },
  purple: { title: 'Purple', dark: '#DB34F2', light: '#CB30E0', onAccentDark: '#000000', onAccentLight: '#000000' },
  pink: { title: 'Pink', dark: '#FF375F', light: '#FF2D55', onAccentDark: '#000000', onAccentLight: '#000000' },
  orange: { title: 'Orange', dark: '#FF9230', light: '#FF8D28', onAccentDark: '#000000', onAccentLight: '#000000' },
  green: { title: 'Green', dark: '#30D158', light: '#34C759', onAccentDark: '#000000', onAccentLight: '#000000' },
  graphite: { title: 'Graphite', dark: '#8E8E93', light: '#8E8E93', onAccentDark: '#000000', onAccentLight: '#000000' },
  mono: { title: 'Mono', dark: '#FFFFFF', light: '#000000', onAccentDark: '#000000', onAccentLight: '#FFFFFF' },
} as const;

/** Parses '#RRGGBB' or 'rgb(a)(r, g, b[, a])' into channels; alpha defaults to 1. */
export function parseColor(value: string): { r: number; g: number; b: number; a: number } | null {
  const v = value.trim().toLowerCase();
  const hex = v.match(/^#([0-9a-f]{6})$/);
  if (hex) {
    const n = parseInt(hex[1], 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a: 1 };
  }
  const fn = v.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)$/);
  if (fn) return { r: +fn[1], g: +fn[2], b: +fn[3], a: fn[4] === undefined ? 1 : +fn[4] };
  return null;
}

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
