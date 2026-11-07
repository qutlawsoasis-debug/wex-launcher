/**
 * Apple Liquid Glass & Standard Materials Specifications.
 * Sourced from tasks/apple/hig/materials.md and adopting-liquid-glass.md
 */

export interface MaterialStyle {
  variant: 'regular' | 'clear';
  backdropFilter: string;
  background: string;
  border: string;
  boxShadow: string;
}

export const Materials = {
  liquidGlassRegular: {
    variant: 'regular',
    backdropFilter: 'blur(24px) saturate(190%)',
    background: 'var(--glass-regular-bg)',
    border: '1px solid var(--glass-border)',
    boxShadow: 'var(--glass-shadow)',
  },
  liquidGlassClear: {
    variant: 'clear',
    backdropFilter: 'blur(16px) saturate(180%)',
    background: 'var(--glass-clear-bg)',
    border: '1px solid var(--glass-border)',
    boxShadow: 'var(--glass-shadow)',
  },
  card: {
    variant: 'regular',
    backdropFilter: 'blur(12px)',
    background: 'var(--card-bg)',
    border: '1px solid var(--glass-border)',
    boxShadow: '0 4px 16px rgba(0, 0, 0, 0.06)',
  },
} as const;
