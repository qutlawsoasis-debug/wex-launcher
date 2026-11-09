<!--
  Apple HIG Liquid Glass Segmented Control Component
  Reference benchmark: C:\Users\Public\agy-multiuser\status\segmented-liquid-glass.html
  Specifications:
  - Track: 40px height (small: 32px), border-radius 20px (small: 16px)
  - Dark: background rgba(120,120,128,.22), border 0.5px rgba(255,255,255,.16), blur 20px saturate(180%)
  - Light: background rgba(118,118,128,.14), border 0.5px rgba(0,0,0,.08)
  - Drop: 34px height (small: 26px), border-radius 17px (small: 13px), 3px padding from track edges
  - Dark drop: rgba(255,255,255,.13), on-press .20, inset highlight & top gradient
  - Light drop: #ffffff, shadow 0 1px 3px rgba(0,0,0,.12), on-press #f5f5f7
  - Physics: spring v += (goal - x) * 0.05; v *= 0.8; stretch = min(|v|*1.2, 14) px
  - Dual-layer text counter-translated lens (.lens-in) with progressive opacity fading (no double rendering)
  - Pointer dragging & closest-segment snapping, keyboard arrow navigation, reduced-motion 150ms
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export interface Option {
    value: string;
    label: string;
    icon?: string;
  }

  interface Props {
    options: Option[] | string[];
    value?: string;
    disabled?: boolean;
    size?: 'small' | 'regular';
    ariaLabel?: string;
    onchange?: (val: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    disabled = false,
    size = 'regular',
    ariaLabel,
    onchange,
  }: Props = $props();

  const P = 3; // 3px padding on all sides

  const normalizedOptions = $derived<Option[]>(
    options.map((opt) => (typeof opt === 'string' ? { value: opt, label: opt } : opt))
  );

  let activeIndex = $derived.by(() => {
    const idx = normalizedOptions.findIndex((opt) => opt.value === value);
    return idx >= 0 ? idx : 0;
  });

  let trackEl: HTMLDivElement | null = $state(null);
  let dropEl: HTMLDivElement | null = $state(null);
  let lensEl: HTMLDivElement | null = $state(null);
  let labelsEl: HTMLDivElement | null = $state(null);
  let baseSpans: HTMLSpanElement[] = $state([]);
  let buttonEls: HTMLButtonElement[] = $state([]);

  let S = $state(0);
  let minX = $state(P);
  let maxX = $state(P);
  let x = P;
  let v = 0;
  let target = P;
  let goal = P;
  let pressed = false;
  let moved = false;
  let downX = 0;
  let raf: number | null = null;
  let resizeObserver: ResizeObserver | null = null;

  function isReducedMotion(): boolean {
    if (typeof window === 'undefined') return false;
    if (document.documentElement.getAttribute('data-reduce-motion') === 'true') return true;
    return window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches ?? false;
  }

  function applyFrame(curX: number, velocity: number) {
    if (!dropEl || !lensEl || S <= 0) return;
    const reduced = isReducedMotion();
    const st = reduced ? 0 : Math.min(Math.abs(velocity) * 1.2, 14);
    let left = curX - st / 2;
    let w = S + st;

    // Constrain drop strictly inside track
    if (left < minX) {
      w -= minX - left;
      left = minX;
    }
    if (left + w > maxX + S) {
      w = maxX + S - left;
    }

    left = Math.round(left);
    w = Math.round(w);

    dropEl.style.width = `${w}px`;
    dropEl.style.transform = `translateX(${left}px)`;
    lensEl.style.transform = `translateX(${-left}px)`;

    // Fade underlying labels proportionally to drop coverage
    baseSpans.forEach((sp, i) => {
      if (!sp) return;
      const a = P + i * S;
      const b = a + S;
      const ov = Math.max(0, Math.min(b, left + w) - Math.max(a, left)) / S;
      sp.style.opacity = (1 - Math.min(1, ov * 1.2)).toFixed(3);
    });
  }

  function frame() {
    if (isReducedMotion()) {
      x = target;
      goal = target;
      v = 0;
      applyFrame(target, 0);
      raf = null;
      return;
    }

    goal += (target - goal) * (pressed ? 0.2 : 1);
    v += (goal - x) * 0.05;
    v *= 0.8;
    x += v;
    applyFrame(x, v);

    if (
      Math.abs(v) > 0.01 ||
      Math.abs(goal - x) > 0.05 ||
      Math.abs(target - goal) > 0.05 ||
      pressed
    ) {
      raf = requestAnimationFrame(frame);
    } else {
      raf = null;
    }
  }

  function kick() {
    if (isReducedMotion()) {
      if (dropEl && lensEl) {
        dropEl.style.transition = 'transform 150ms cubic-bezier(0.2, 0.9, 0.3, 1), width 150ms ease';
        lensEl.style.transition = 'transform 150ms cubic-bezier(0.2, 0.9, 0.3, 1)';
      }
      applyFrame(target, 0);
      return;
    }

    if (dropEl && lensEl) {
      dropEl.style.transition = 'background-color 200ms ease';
      lensEl.style.transition = 'none';
    }

    if (!raf) {
      raf = requestAnimationFrame(frame);
    }
  }

  function measure() {
    if (!trackEl) return;
    const n = normalizedOptions.length;
    if (n === 0) return;
    const W = trackEl.clientWidth > 0 ? trackEl.clientWidth : n * 80 + 2 * P;
    S = Math.round((W - 2 * P) / n);
    minX = P;
    maxX = P + S * (n - 1);

    if (labelsEl) {
      labelsEl.style.width = `${P * 2 + S * n}px`;
      labelsEl.style.gridTemplateColumns = `${P}px repeat(${n}, ${S}px) ${P}px`;
    }
    if (lensEl) {
      lensEl.style.width = `${P * 2 + S * n}px`;
      lensEl.style.gridTemplateColumns = `${P}px repeat(${n}, ${S}px) ${P}px`;
    }

    target = P + activeIndex * S;
    goal = target;
    x = target;
    applyFrame(x, 0);
  }

  function fromX(clientX: number): number {
    if (!trackEl) return minX;
    const r = trackEl.getBoundingClientRect();
    return Math.max(minX, Math.min(maxX, clientX - r.left - S / 2));
  }

  function select(optValue: string, index?: number) {
    if (disabled) return;
    value = optValue;
    onchange?.(optValue);

    const targetIdx = index ?? normalizedOptions.findIndex((opt) => opt.value === optValue);
    if (targetIdx >= 0) {
      target = P + targetIdx * S;
      kick();
      if (buttonEls[targetIdx]) {
        buttonEls[targetIdx].focus();
      }
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    pressed = true;
    moved = false;
    downX = e.clientX;
    dropEl?.classList.add('on');
    try {
      trackEl?.setPointerCapture(e.pointerId);
    } catch {
      // Ignored in headless/test environments
    }
    kick();
  }

  function handlePointerMove(e: PointerEvent) {
    if (!pressed || disabled) return;
    if (Math.abs(e.clientX - downX) > 4) {
      moved = true;
    }
    if (moved) {
      target = fromX(e.clientX);
      kick();
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!pressed) return;
    pressed = false;
    dropEl?.classList.remove('on');

    const n = normalizedOptions.length;
    let i: number;
    if (!moved) {
      const r = trackEl?.getBoundingClientRect();
      const rectLeft = r ? r.left : 0;
      i = Math.floor((e.clientX - rectLeft - P) / (S || 1));
    } else {
      i = Math.round((target - P) / (S || 1));
    }

    i = Math.max(0, Math.min(n - 1, i));
    const chosen = normalizedOptions[i];
    if (chosen) {
      select(chosen.value, i);
    }
  }

  function handlePointerCancel() {
    if (!pressed) return;
    pressed = false;
    dropEl?.classList.remove('on');
    target = P + activeIndex * S;
    kick();
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (disabled) return;
    const n = normalizedOptions.length;
    if (n === 0) return;

    let nextIdx = -1;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      event.preventDefault();
      nextIdx = (activeIndex + 1) % n;
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      event.preventDefault();
      nextIdx = (activeIndex - 1 + n) % n;
    } else if (event.key === 'Home') {
      event.preventDefault();
      nextIdx = 0;
    } else if (event.key === 'End') {
      event.preventDefault();
      nextIdx = n - 1;
    }

    if (nextIdx >= 0) {
      select(normalizedOptions[nextIdx].value, nextIdx);
    }
  }

  $effect(() => {
    // React to external value or options array changes
    const curIdx = activeIndex;
    if (S > 0) {
      target = P + curIdx * S;
      kick();
    }
  });

  onMount(() => {
    measure();
    if (typeof ResizeObserver !== 'undefined' && trackEl) {
      resizeObserver = new ResizeObserver(() => {
        measure();
      });
      resizeObserver.observe(trackEl);
    }
  });

  onDestroy(() => {
    if (raf) cancelAnimationFrame(raf);
    resizeObserver?.disconnect();
  });
</script>

<!-- Outer Liquid Glass Track -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={trackEl}
  class="apple-segmented-control liquid-glass {size}"
  class:disabled
  role="tablist"
  tabindex={disabled ? -1 : 0}
  aria-label={ariaLabel}
  aria-orientation="horizontal"
  data-n={normalizedOptions.length}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
>
  <!-- Base Labels Layer (under drop) -->
  <div bind:this={labelsEl} class="labels" aria-hidden="true">
    <i></i>
    {#each normalizedOptions as opt, i (opt.value)}
      <span bind:this={baseSpans[i]}>{opt.label}</span>
    {/each}
    <i></i>
  </div>

  <!-- Morphing / Stretching Liquid Drop -->
  <div bind:this={dropEl} class="drop" aria-hidden="true">
    <div bind:this={lensEl} class="lens-in">
      <i></i>
      {#each normalizedOptions as opt, i (opt.value)}
        <span>{opt.label}</span>
      {/each}
      <i></i>
    </div>
  </div>

  <!-- Interactive / Accessible Tab Buttons Layer -->
  <div
    class="buttons-layer"
    style:grid-template-columns="{P}px repeat({normalizedOptions.length}, {S > 0 ? S : 80}px) {P}px"
  >
    <i aria-hidden="true"></i>
    {#each normalizedOptions as opt, i (opt.value)}
      {@const isSelected = i === activeIndex}
      <button
        type="button"
        role="tab"
        class="segment-button"
        class:selected={isSelected}
        aria-selected={isSelected}
        tabindex={isSelected ? 0 : -1}
        bind:this={buttonEls[i]}
        onclick={(e) => {
          e.stopPropagation();
          select(opt.value, i);
        }}
        onkeydown={handleKeyDown}
      >
        <span class="sr-only">{opt.label}</span>
      </button>
    {/each}
    <i></i>
  </div>
</div>

<style>
  .apple-segmented-control.liquid-glass {
    position: relative;
    width: 100%;
    max-width: 640px;
    height: 40px;
    border-radius: 20px;
    background: rgba(120, 120, 128, 0.22);
    border: 0.5px solid rgba(255, 255, 255, 0.16);
    touch-action: none;
    user-select: none;
    cursor: pointer;
    overflow: hidden;
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-sizing: border-box;
    display: block;
  }

  .apple-segmented-control.liquid-glass.small {
    height: 32px;
    border-radius: 16px;
  }

  .apple-segmented-control.liquid-glass.disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  /* Light Theme */
  :global([data-theme="light"]) .apple-segmented-control.liquid-glass {
    background: rgba(118, 118, 128, 0.14);
    border: 0.5px solid rgba(0, 0, 0, 0.08);
  }

  /* Labels layers (Base and Lens) */
  .labels,
  .lens-in {
    position: absolute;
    top: 0;
    left: 0;
    height: 40px;
    display: grid;
    grid-auto-flow: column;
    align-items: center;
    text-align: center;
    font-family: var(--font-family, Onest), system-ui, -apple-system, sans-serif;
    font-weight: 500;
    font-size: 15px;
    line-height: 1;
    pointer-events: none;
    box-sizing: border-box;
  }

  .apple-segmented-control.liquid-glass.small .labels,
  .apple-segmented-control.liquid-glass.small .lens-in {
    height: 32px;
    font-size: 13px;
  }

  .labels span {
    color: rgba(255, 255, 255, 0.62);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 4px;
    will-change: opacity;
  }

  :global([data-theme="light"]) .labels span {
    color: rgba(0, 0, 0, 0.55);
  }

  /* Drop */
  .drop {
    position: absolute;
    top: 3px;
    left: 0;
    height: 34px;
    border-radius: 17px;
    overflow: hidden;
    pointer-events: none;
    will-change: transform, width;
    background: rgba(255, 255, 255, 0.13);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.5),
      inset 0 -1px 1px rgba(255, 255, 255, 0.1);
    transition: background-color 200ms ease;
    box-sizing: border-box;
  }

  .apple-segmented-control.liquid-glass.small .drop {
    height: 26px;
    border-radius: 13px;
  }

  .drop.on {
    background: rgba(255, 255, 255, 0.2);
  }

  .drop::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.22),
      rgba(255, 255, 255, 0) 50%
    );
    pointer-events: none;
  }

  /* Light Theme Drop */
  :global([data-theme="light"]) .drop {
    background: #ffffff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  }

  :global([data-theme="light"]) .drop.on {
    background: #f5f5f7;
  }

  :global([data-theme="light"]) .drop::after {
    display: none;
  }

  /* Inside Drop Lens Text */
  .lens-in {
    top: -3px;
    will-change: transform;
  }

  .lens-in span {
    color: #ffffff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 4px;
  }

  :global([data-theme="light"]) .lens-in span {
    color: #000000;
  }

  /* Accessible Interactive Tab Buttons Layer */
  .buttons-layer {
    position: absolute;
    inset: 0;
    display: grid;
    grid-auto-flow: column;
    align-items: stretch;
    pointer-events: auto;
    z-index: 3;
  }

  .segment-button {
    width: 100%;
    height: 100%;
    border: none;
    background: transparent;
    cursor: pointer;
    outline: none;
    padding: 0;
    margin: 0;
  }

  .segment-button:focus-visible {
    outline: 2px solid var(--color-system-blue, #007aff);
    outline-offset: -2px;
    border-radius: 17px;
  }

  .apple-segmented-control.liquid-glass.small .segment-button:focus-visible {
    border-radius: 13px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* Reduced Motion */
  @media (prefers-reduced-motion: reduce) {
    .drop,
    .lens-in {
      transition: transform 150ms cubic-bezier(0.2, 0.9, 0.3, 1), width 150ms ease !important;
    }
  }

  :global([data-reduce-motion="true"]) .drop,
  :global([data-reduce-motion="true"]) .lens-in {
    transition: transform 150ms cubic-bezier(0.2, 0.9, 0.3, 1), width 150ms ease !important;
  }
</style>
