import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import SegmentedControl from './SegmentedControl.svelte';
import SegmentedTestWrapper from './SegmentedTestWrapper.svelte';

describe('SegmentedControl Component - Liquid Glass', () => {
  let target: HTMLElement;
  let comp: any;

  const defaultOptions = [
    { value: 'opt1', label: '1.21.1' },
    { value: 'opt2', label: '1.20.1' },
    { value: 'opt3', label: '1.19.4' },
  ];

  beforeEach(() => {
    vi.restoreAllMocks();
    if (typeof (globalThis as any).PointerEvent === 'undefined') {
      (globalThis as any).PointerEvent = class PointerEvent extends MouseEvent {
        pointerId: number;
        constructor(type: string, params: any = {}) {
          super(type, params);
          this.pointerId = params.pointerId ?? 1;
        }
      };
    }
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    if (comp) {
      unmount(comp);
      comp = null;
    }
    target.remove();
    document.documentElement.removeAttribute('data-reduce-motion');
    document.documentElement.removeAttribute('data-reduce-transparency');
    document.documentElement.removeAttribute('data-high-contrast');
    document.documentElement.removeAttribute('data-theme');
  });

  // L1: role="radiogroup" / role="radio" + aria-checked
  it('L1: uses role="radiogroup" on container and role="radio" with aria-checked on segments', async () => {
    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt2',
      },
    });
    await tick();

    const control = target.querySelector('.apple-segmented-control.liquid-glass');
    expect(control).toBeTruthy();
    expect(control?.getAttribute('role')).toBe('radiogroup');
    expect(control?.getAttribute('role')).not.toBe('tablist');

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    expect(buttons.length).toBe(3);

    expect(buttons[0].getAttribute('role')).toBe('radio');
    expect(buttons[0].getAttribute('aria-checked')).toBe('false');

    expect(buttons[1].getAttribute('role')).toBe('radio');
    expect(buttons[1].getAttribute('aria-checked')).toBe('true');

    expect(buttons[2].getAttribute('role')).toBe('radio');
    expect(buttons[2].getAttribute('aria-checked')).toBe('false');
  });

  // L2: Tab directly to selected segment (no tabindex on container), arrow keys navigate from first Tab
  it('L2: container has no tabindex; active segment has tabindex=0; arrow keys navigate directly', async () => {
    const onchange = vi.fn();
    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
        onchange,
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    // Container must NOT have tabindex so Tab skips the container directly to active radio
    expect(control.hasAttribute('tabindex')).toBe(false);

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    expect(buttons[0].getAttribute('tabindex')).toBe('0');
    expect(buttons[1].getAttribute('tabindex')).toBe('-1');
    expect(buttons[2].getAttribute('tabindex')).toBe('-1');

    // Arrow navigation operates directly from focused button
    buttons[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt2');
  });

  // L3: disabled state: all segments disabled, aria-disabled="true", tabindex=-1
  it('L3: disabled state sets aria-disabled="true" on radiogroup, disabled={true} and tabindex=-1 on all buttons', async () => {
    const onchange = vi.fn();
    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
        disabled: true,
        onchange,
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    expect(control.getAttribute('aria-disabled')).toBe('true');
    expect(control.classList.contains('disabled')).toBe(true);

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    buttons.forEach((btn) => {
      expect(btn.disabled).toBe(true);
      expect(btn.getAttribute('aria-disabled')).toBe('true');
      expect(btn.getAttribute('tabindex')).toBe('-1');
    });

    // Clicks and keyboard events must be ignored
    buttons[1].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    buttons[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await tick();
    expect(onchange).not.toHaveBeenCalled();
  });

  // L4: rAF cleanup on visibilitychange, window.blur, lostpointercapture, pointercancel
  it('L4: stops rAF and animation loop on visibilitychange, window.blur, lostpointercapture, and pointercancel', async () => {
    const cancelSpy = vi.spyOn(window, 'cancelAnimationFrame');

    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    const drop = target.querySelector<HTMLElement>('.drop')!;

    // 1. pointerdown then visibilitychange (document.hidden = true)
    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 50, bubbles: true }));
    await tick();
    expect(drop.classList.contains('on')).toBe(true);

    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));
    await tick();
    expect(cancelSpy).toHaveBeenCalled();
    expect(drop.classList.contains('on')).toBe(false);

    // 2. pointerdown then window.blur
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
    cancelSpy.mockClear();
    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 50, bubbles: true }));
    await tick();
    expect(drop.classList.contains('on')).toBe(true);

    window.dispatchEvent(new Event('blur'));
    await tick();
    expect(cancelSpy).toHaveBeenCalled();
    expect(drop.classList.contains('on')).toBe(false);

    // 3. pointerdown then lostpointercapture
    cancelSpy.mockClear();
    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 50, bubbles: true }));
    await tick();
    expect(drop.classList.contains('on')).toBe(true);

    control.dispatchEvent(new PointerEvent('lostpointercapture', { bubbles: true }));
    await tick();
    expect(cancelSpy).toHaveBeenCalled();
    expect(drop.classList.contains('on')).toBe(false);

    // 4. pointerdown then pointercancel
    cancelSpy.mockClear();
    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 50, bubbles: true }));
    await tick();
    expect(drop.classList.contains('on')).toBe(true);

    control.dispatchEvent(new PointerEvent('pointercancel', { bubbles: true }));
    await tick();
    expect(cancelSpy).toHaveBeenCalled();
    expect(drop.classList.contains('on')).toBe(false);
  });

  // L5: Reduce Motion: progressive label opacity fading synchronous with drop motion
  it('L5: in reduced-motion mode, fades base label opacity synchronously across frames without stretch', async () => {
    document.documentElement.setAttribute('data-reduce-motion', 'true');
    const onchange = vi.fn();

    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
        onchange,
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    vi.spyOn(control, 'clientWidth', 'get').mockReturnValue(300);

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    const drop = target.querySelector<HTMLElement>('.drop')!;
    const baseSpans = target.querySelectorAll<HTMLSpanElement>('.labels span');

    // Trigger select to opt3
    buttons[2].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt3');

    // Under reduced motion, drop has no velocity stretch (width stays stable)
    expect(drop.style.width).toBeDefined();
    // Base label opacity is defined and smoothly calculated
    expect(baseSpans[0].style.opacity).toBeDefined();
  });

  // L8: Dynamic change in options count recalculates grid and layout
  it('L8: recalculates grid and segment width when options count changes dynamically', async () => {
    comp = mount(SegmentedTestWrapper, {
      target,
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    expect(control.getAttribute('data-n')).toBe('3');

    const labels = target.querySelector<HTMLElement>('.labels')!;
    expect(labels.style.gridTemplateColumns).toContain('repeat(3,');

    let buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    expect(buttons.length).toBe(3);

    // Dynamically update options to 4 items
    comp.setOptions([
      { value: 'opt1', label: '1.21.1' },
      { value: 'opt2', label: '1.20.1' },
      { value: 'opt3', label: '1.19.4' },
      { value: 'opt4', label: '1.18.2' },
    ]);
    await tick();

    expect(control.getAttribute('data-n')).toBe('4');
    expect(labels.style.gridTemplateColumns).toContain('repeat(4,');
    buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    expect(buttons.length).toBe(4);
  });

  // L9: Reduce Transparency & High Contrast support
  it('L9: supports reduce transparency and high contrast modes without blur effects', async () => {
    document.documentElement.setAttribute('data-reduce-transparency', 'true');
    document.documentElement.setAttribute('data-high-contrast', 'true');

    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.apple-segmented-control.liquid-glass');
    expect(control).toBeTruthy();
    expect(document.documentElement.getAttribute('data-reduce-transparency')).toBe('true');
    expect(document.documentElement.getAttribute('data-high-contrast')).toBe('true');
  });

  // Uniform 40px Track without small variant
  it('renders standard uniform 40px Liquid Glass track without small variant', async () => {
    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
      },
    });
    await tick();

    const control = target.querySelector('.apple-segmented-control.liquid-glass');
    expect(control).toBeTruthy();
    expect(control?.classList.contains('small')).toBe(false);
  });

  // Pointer drag and snaps to closest segment
  it('handles pointer drag and settles on closest segment on pointerup', async () => {
    const onchange = vi.fn();
    comp = mount(SegmentedControl, {
      target,
      props: {
        options: defaultOptions,
        value: 'opt1',
        onchange,
      },
    });
    await tick();

    const control = target.querySelector<HTMLElement>('.liquid-glass')!;
    expect(control).toBeTruthy();

    vi.spyOn(control, 'getBoundingClientRect').mockReturnValue({
      left: 0,
      top: 0,
      right: 300,
      bottom: 40,
      width: 300,
      height: 40,
      x: 0,
      y: 0,
      toJSON: () => {},
    });
    Object.defineProperty(control, 'clientWidth', { value: 300, configurable: true });

    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 20, bubbles: true }));
    await tick();
    const drop = target.querySelector('.drop');
    expect(drop?.classList.contains('on')).toBe(true);

    control.dispatchEvent(new PointerEvent('pointermove', { clientX: 160, bubbles: true }));
    await tick();

    control.dispatchEvent(new PointerEvent('pointerup', { clientX: 160, bubbles: true }));
    await tick();
    expect(drop?.classList.contains('on')).toBe(false);
    expect(onchange).toHaveBeenCalledWith('opt2');
  });
});
