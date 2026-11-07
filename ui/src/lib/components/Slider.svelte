<!--
  Apple HIG Slider Component
  Specification: tasks/apple/hig/sliders.md
  Rules:
  - Track fills with accent color from leading edge to thumb
  - Smooth round thumb with subtle shadow
  - Live value feedback display
  - Keyboard accessible (left/right arrows)
  - Strict alignment: thumb position corresponds exactly to value
-->
<script lang="ts">
  interface Props {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    label?: string;
    unit?: string;
    showValue?: boolean;
    formatValue?: (val: number) => string;
    onchange?: (val: number) => void;
  }

  let {
    value = $bindable(50),
    min = 0,
    max = 100,
    step = 1,
    disabled = false,
    label,
    unit = '',
    showValue = true,
    formatValue,
    onchange,
  }: Props = $props();

  let percentage = $derived(
    max === min ? 0 : Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100))
  );

  let formatted = $derived(
    formatValue ? formatValue(value) : `${value}${unit ? ' ' + unit : ''}`
  );

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    const num = parseFloat(target.value);
    value = num;
    onchange?.(num);
  }
</script>

<div class="apple-slider-container" class:disabled>
  {#if label || showValue}
    <div class="slider-header">
      {#if label}
        <span class="slider-label">{label}</span>
      {/if}
      {#if showValue}
        <span class="slider-value">{formatted}</span>
      {/if}
    </div>
  {/if}

  <div class="slider-track-area">
    <div class="track-bg"></div>
    <div class="track-fill" style="width: {percentage}%;"></div>
    <div
      class="slider-thumb"
      data-testid="slider-thumb"
      data-percentage={percentage.toFixed(1)}
      style="left: calc({percentage}% - {percentage * 0.2}px);"
    ></div>
    <input
      type="range"
      {min}
      {max}
      {step}
      {value}
      {disabled}
      oninput={handleInput}
      class="range-input"
      aria-label={label || 'Slider'}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={formatted}
    />
  </div>
</div>

<style>
  .apple-slider-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    user-select: none;
    font-family: var(--font-system);
  }

  .apple-slider-container.disabled {
    opacity: 0.42;
    pointer-events: none;
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--text-subheadline);
  }

  .slider-label {
    color: var(--color-label);
    font-weight: 500;
  }

  .slider-value {
    color: var(--color-secondary-label);
    font-variant-numeric: tabular-nums;
    font-size: var(--text-footnote);
    font-weight: 600;
  }

  .slider-track-area {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
  }

  .track-bg {
    position: absolute;
    left: 0;
    right: 0;
    height: 4px;
    border-radius: var(--radius-pill);
    background-color: var(--color-quaternary-system-fill);
  }

  .track-fill {
    position: absolute;
    left: 0;
    height: 4px;
    border-radius: var(--radius-pill);
    background-color: var(--color-system-blue);
    pointer-events: none;
    transition: width var(--motion-duration-short) ease;
  }

  .slider-thumb {
    position: absolute;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background-color: #ffffff;
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.28),
      0 0 1px rgba(0, 0, 0, 0.18);
    pointer-events: none;
    transition:
      left var(--motion-duration-short) ease,
      transform var(--motion-duration-short) ease;
  }

  .range-input {
    position: absolute;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
    z-index: 2;
    -webkit-appearance: none;
    appearance: none;
    background: transparent;
  }
</style>
