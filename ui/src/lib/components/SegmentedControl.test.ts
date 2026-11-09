import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import SegmentedControl from './SegmentedControl.svelte';

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
    document.documentElement.removeAttribute('data-theme');
  });

  it('renders segments with dual-layer text (base labels and lens-in)', async () => {
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

    const labels = target.querySelector('.labels');
    expect(labels).toBeTruthy();
    expect(labels?.textContent).toContain('1.21.1');
    expect(labels?.textContent).toContain('1.20.1');
    expect(labels?.textContent).toContain('1.19.4');

    const lensIn = target.querySelector('.drop .lens-in');
    expect(lensIn).toBeTruthy();
    expect(lensIn?.textContent).toContain('1.21.1');
  });

  it('handles click to select segment and triggers onchange', async () => {
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

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    expect(buttons.length).toBe(3);

    // Click 2nd segment button
    buttons[1].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt2');
  });

  it('navigates with keyboard arrow keys (ArrowRight, ArrowLeft, Home, End)', async () => {
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

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');

    // ArrowRight advances to opt2
    buttons[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt2');

    // ArrowRight advances to opt3
    buttons[1].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt3');

    // ArrowLeft goes back to opt2
    buttons[2].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt2');

    // Home jumps to first segment
    buttons[1].dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt1');

    // End jumps to last segment
    buttons[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }));
    await tick();
    expect(onchange).toHaveBeenCalledWith('opt3');
  });

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

    // Mock bounding rect: width 300px, 3 segments -> 100px each
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

    // Pointer down at start
    control.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 20, bubbles: true }));
    await tick();
    const drop = target.querySelector('.drop');
    expect(drop?.classList.contains('on')).toBe(true);

    // Pointer move dragging past segment 2 threshold (e.g. clientX = 160px)
    control.dispatchEvent(new PointerEvent('pointermove', { clientX: 160, bubbles: true }));
    await tick();

    // Pointer up settles on 2nd option (opt2)
    control.dispatchEvent(new PointerEvent('pointerup', { clientX: 160, bubbles: true }));
    await tick();
    expect(drop?.classList.contains('on')).toBe(false);
    expect(onchange).toHaveBeenCalledWith('opt2');
  });

  it('respects reduced-motion setting without stretching', async () => {
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

    const drop = target.querySelector<HTMLElement>('.drop');
    expect(drop).toBeTruthy();

    const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
    buttons[2].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await tick();

    expect(onchange).toHaveBeenCalledWith('opt3');
    // Reduced motion applies 150ms transition
    expect(drop?.style.transition).toContain('150ms');
  });

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
});
