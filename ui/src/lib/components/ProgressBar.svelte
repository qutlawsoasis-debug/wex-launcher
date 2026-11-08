<!--
  Apple HIG Progress Bar Component
  Specification: tasks/apple/hig/progress-indicators.md
  Rules:
  - Determinate progress indicator fills from leading to trailing edge
  - Indeterminate animated activity mode when duration is unquantifiable
  - Accurate, smooth advancement reporting
  - Descriptive context on status and transfer speed
-->
<script lang="ts">
  interface Props {
    percentage?: number; // 0 to 100
    indeterminate?: boolean;
    label?: string;
    detail?: string;
    height?: number;
    color?: string;
  }

  let {
    percentage = 0,
    indeterminate = false,
    label,
    detail,
    height = 6,
    color = 'var(--color-system-blue)',
  }: Props = $props();

  let clampedPct = $derived(Math.max(0, Math.min(100, percentage)));
</script>

<div class="apple-progress-container" aria-label={label || 'Прогресс'}>
  {#if label || detail || !indeterminate}
    <div class="progress-info">
      <span class="progress-label">{label || (indeterminate ? 'Выполнение...' : 'Прогресс')}</span>
      <span class="progress-detail">
        {#if detail}
          {detail}
        {:else if !indeterminate}
          {Math.round(clampedPct)}%
        {/if}
      </span>
    </div>
  {/if}

  <div class="progress-track" style="height: {height}px;">
    {#if indeterminate}
      <div
        class="progress-indeterminate"
        style="background-color: {color};"
        role="progressbar"
        aria-busy="true"
        aria-label={label || 'Загрузка...'}
      ></div>
    {:else}
      <div
        class="progress-fill"
        style="width: {clampedPct}%; background-color: {color};"
        role="progressbar"
        aria-label={label || 'Прогресс'}
        aria-valuenow={Math.round(clampedPct)}
        aria-valuemin="0"
        aria-valuemax="100"
      ></div>
    {/if}
  </div>
</div>

<style>
  .apple-progress-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    user-select: none;
    font-family: var(--font-system);
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--text-footnote);
  }

  .progress-label {
    font-weight: 500;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .progress-detail {
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
    flex-shrink: 0;
  }

  .progress-track {
    position: relative;
    width: 100%;
    background-color: var(--color-quaternary-system-fill);
    border-radius: var(--radius-pill);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: var(--radius-pill);
    transition: width var(--motion-duration-short) ease-out;
  }

  .progress-indeterminate {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 40%;
    border-radius: var(--radius-pill);
    animation: indeterminateSlide 1.5s cubic-bezier(0.4, 0, 0.2, 1) infinite;
    will-change: transform;
  }

  @keyframes indeterminateSlide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .progress-fill {
      transition: none !important;
    }
    .progress-indeterminate {
      animation: none !important;
      left: 20% !important;
      width: 60% !important;
      opacity: 0.8;
    }
  }

  :global([data-effects="off"]) .progress-fill,
  :global([data-reduce-motion="true"]) .progress-fill {
    transition: none !important;
  }

  :global([data-effects="off"]) .progress-indeterminate,
  :global([data-reduce-motion="true"]) .progress-indeterminate {
    animation: none !important;
    left: 20% !important;
    width: 60% !important;
    opacity: 0.8;
  }
</style>
