/**
 * Apple HIG Typography Specifications.
 * Sourced from tasks/apple/hig/typography.md
 */

export interface TextStyle {
  size: number;
  lineHeight: number;
  weight: number;
  letterSpacing?: string;
  css: string;
}

export const TypographyScale = {
  largeTitle: {
    size: 34,
    lineHeight: 41,
    weight: 700,
    letterSpacing: '-0.4px',
    css: 'font-size: 34px; line-height: 41px; font-weight: 700; letter-spacing: -0.4px;',
  },
  title1: {
    size: 28,
    lineHeight: 34,
    weight: 700,
    letterSpacing: '-0.3px',
    css: 'font-size: 28px; line-height: 34px; font-weight: 700; letter-spacing: -0.3px;',
  },
  title2: {
    size: 22,
    lineHeight: 28,
    weight: 700,
    letterSpacing: '-0.2px',
    css: 'font-size: 22px; line-height: 28px; font-weight: 700; letter-spacing: -0.2px;',
  },
  title3: {
    size: 20,
    lineHeight: 25,
    weight: 600,
    letterSpacing: '-0.15px',
    css: 'font-size: 20px; line-height: 25px; font-weight: 600; letter-spacing: -0.15px;',
  },
  headline: {
    size: 17,
    lineHeight: 22,
    weight: 600,
    letterSpacing: '-0.1px',
    css: 'font-size: 17px; line-height: 22px; font-weight: 600; letter-spacing: -0.1px;',
  },
  body: {
    size: 17,
    lineHeight: 22,
    weight: 400,
    css: 'font-size: 17px; line-height: 22px; font-weight: 400;',
  },
  callout: {
    size: 16,
    lineHeight: 21,
    weight: 400,
    css: 'font-size: 16px; line-height: 21px; font-weight: 400;',
  },
  subheadline: {
    size: 15,
    lineHeight: 20,
    weight: 400,
    css: 'font-size: 15px; line-height: 20px; font-weight: 400;',
  },
  footnote: {
    size: 13,
    lineHeight: 18,
    weight: 400,
    css: 'font-size: 13px; line-height: 18px; font-weight: 400;',
  },
  caption1: {
    size: 12,
    lineHeight: 16,
    weight: 400,
    css: 'font-size: 12px; line-height: 16px; font-weight: 400;',
  },
  caption2: {
    size: 11,
    lineHeight: 13,
    weight: 400,
    css: 'font-size: 11px; line-height: 13px; font-weight: 400;',
  },
} as const;
