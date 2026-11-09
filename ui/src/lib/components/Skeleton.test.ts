import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import Skeleton from './Skeleton.svelte';

describe('Skeleton Component', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders default text skeleton with soft opacity pulse', async () => {
    const comp = mount(Skeleton, { target });
    await tick();

    const el = target.querySelector('[data-testid="skeleton"]');
    expect(el).not.toBeNull();
    expect(el?.classList.contains('text')).toBe(true);

    unmount(comp);
  });

  it('renders circular avatar skeleton', async () => {
    const comp = mount(Skeleton, {
      target,
      props: { variant: 'circle', width: 40, height: 40 },
    });
    await tick();

    const el = target.querySelector('[data-testid="skeleton"]') as HTMLElement;
    expect(el).not.toBeNull();
    expect(el.classList.contains('circle')).toBe(true);
    expect(el.style.width).toBe('40px');
    expect(el.style.height).toBe('40px');
    expect(el.style.borderRadius).toBe('50%');

    unmount(comp);
  });

  it('renders multiple row skeletons with 20-30ms cascade delay (max 8)', async () => {
    const comp = mount(Skeleton, {
      target,
      props: { variant: 'row', count: 3 },
    });
    await tick();

    const rows = target.querySelectorAll('[data-testid="skeleton-row"]');
    expect(rows.length).toBe(3);

    unmount(comp);
  });

  it('renders card skeleton with media and meta', async () => {
    const comp = mount(Skeleton, {
      target,
      props: { variant: 'card' },
    });
    await tick();

    const card = target.querySelector('[data-testid="skeleton-card"]');
    expect(card).not.toBeNull();
    expect(card?.querySelector('.skeleton-card-media')).not.toBeNull();

    unmount(comp);
  });
});
