<!--
  Apple HIG Skeleton Placeholder Component
  Specification:
  - Soft opacity pulse: 0.45 <-> 0.75
  - NO traveling glare / sheen (без бегущего блика)
  - Variants: text, circle, rect, row, card
  - Reduced-motion: static opacity, no animation
-->
<script lang="ts">
  import { theme } from '../design/theme.svelte';
  import { t } from '../i18n';

  interface Props {
    variant?: 'text' | 'rect' | 'circle' | 'row' | 'card';
    width?: string | number;
    height?: string | number;
    radius?: string | number;
    count?: number;
    class?: string;
  }

  let {
    variant = 'text',
    width,
    height,
    radius,
    count = 1,
    class: className = '',
  }: Props = $props();

  function formatDim(val: string | number | undefined, defaultVal: string): string {
    if (val === undefined) return defaultVal;
    return typeof val === 'number' ? `${val}px` : val;
  }

  const computedWidth = $derived(
    formatDim(
      width,
      variant === 'circle' ? '32px' : variant === 'text' ? '100%' : '100%'
    )
  );

  const computedHeight = $derived(
    formatDim(
      height,
      variant === 'circle' ? '32px' : variant === 'text' ? '14px' : '44px'
    )
  );

  const computedRadius = $derived(
    radius !== undefined
      ? (typeof radius === 'number' ? `${radius}px` : radius)
      : variant === 'circle'
        ? '50%'
        : variant === 'card'
          ? 'var(--radius-lg, 14px)'
          : variant === 'row'
            ? 'var(--radius-md, 10px)'
            : 'var(--radius-sm, 6px)'
  );

  const countArray = $derived(Array.from({ length: Math.max(1, count) }, (_, i) => i));
</script>

{#each countArray as index (index)}
  {#if variant === 'row'}
    <div
      class="apple-skeleton-row {className}"
      role="progressbar"
      aria-label={t('skeleton.loading_content')}
      style:--cascade-delay="{Math.min(index, 7) * 25}ms"
      data-testid="skeleton-row"
    >
      <div class="skeleton-pulse skeleton-circle-lead"></div>
      <div class="skeleton-row-body">
        <div class="skeleton-pulse skeleton-line skeleton-title"></div>
        <div class="skeleton-pulse skeleton-line skeleton-subtitle"></div>
      </div>
      <div class="skeleton-pulse skeleton-trailing"></div>
    </div>
  {:else if variant === 'card'}
    <div
      class="apple-skeleton-card {className}"
      role="progressbar"
      aria-label={t('skeleton.loading_card')}
      style:--cascade-delay="{Math.min(index, 7) * 25}ms"
      data-testid="skeleton-card"
    >
      <div class="skeleton-pulse skeleton-card-media"></div>
      <div class="skeleton-card-meta">
        <div class="skeleton-pulse skeleton-line skeleton-title"></div>
        <div class="skeleton-pulse skeleton-line skeleton-subtitle"></div>
      </div>
    </div>
  {:else}
    <div
      class="apple-skeleton {variant} {className}"
      class:reduce-motion={theme.reduceMotion}
      role="progressbar"
      aria-label={t('common.loading')}
      style:width={computedWidth}
      style:height={computedHeight}
      style:border-radius={computedRadius}
      data-testid="skeleton"
    ></div>
  {/if}
{/each}

<style>
  /* Soft Opacity Pulse: 0.45 <-> 0.75 without traveling glare */
  @keyframes skeletonOpacityPulse {
    0%, 100% {
      opacity: 0.45;
    }
    50% {
      opacity: 0.75;
    }
  }

  .apple-skeleton,
  .skeleton-pulse {
    background-color: var(--color-fill-secondary, rgba(120, 120, 128, 0.2));
    animation: skeletonOpacityPulse 1.8s ease-in-out infinite;
    display: inline-block;
    box-sizing: border-box;
    vertical-align: middle;
  }

  .apple-skeleton.circle {
    border-radius: 50%;
  }

  .apple-skeleton.rect {
    border-radius: var(--radius-sm, 6px);
  }

  .apple-skeleton.text {
    border-radius: var(--radius-xs, 4px);
    margin: 2px 0;
  }

  /* List Row Skeleton */
  .apple-skeleton-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    min-height: 48px;
    width: 100%;
    box-sizing: border-box;
    border-bottom: 1px solid var(--color-separator);
    animation: skeletonFadeIn 0.25s ease both;
    animation-delay: var(--cascade-delay, 0ms);
  }

  .apple-skeleton-row:last-child {
    border-bottom: none;
  }

  .skeleton-circle-lead {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .skeleton-row-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .skeleton-line {
    border-radius: 4px;
  }

  .skeleton-title {
    width: 48%;
    height: 13px;
  }

  .skeleton-subtitle {
    width: 32%;
    height: 10px;
  }

  .skeleton-trailing {
    width: 36px;
    height: 14px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  /* Card Skeleton */
  .apple-skeleton-card {
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-lg, 14px);
    background: var(--card-bg, rgba(255, 255, 255, 0.08));
    border: 1px solid var(--color-separator);
    overflow: hidden;
    padding: 12px;
    gap: 10px;
    box-sizing: border-box;
    animation: skeletonFadeIn 0.25s ease both;
    animation-delay: var(--cascade-delay, 0ms);
  }

  .skeleton-card-media {
    width: 100%;
    height: 88px;
    border-radius: var(--radius-md, 8px);
  }

  .skeleton-card-meta {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  @keyframes skeletonFadeIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Reduce Motion: Static Translucent Placeholder */
  @media (prefers-reduced-motion: reduce) {
    .apple-skeleton,
    .skeleton-pulse,
    .apple-skeleton-row,
    .apple-skeleton-card {
      animation: none !important;
      opacity: 0.6 !important;
    }
  }

  :global([data-reduce-motion='true']) .apple-skeleton,
  :global([data-reduce-motion='true']) .skeleton-pulse,
  :global([data-reduce-motion='true']) .apple-skeleton-row,
  :global([data-reduce-motion='true']) .apple-skeleton-card {
    animation: none !important;
    opacity: 0.6 !important;
  }
</style>
