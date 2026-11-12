import { describe, it, expect, vi, afterEach } from 'vitest';
import { initScrollIndicators, SCROLLING_CLASS, SCROLL_INDICATOR_HIDE_MS } from './scrollIndicators';

describe('scroll indicator', () => {
  afterEach(() => vi.useRealTimers());

  it('shows while an element scrolls and hides 1.2 s after the last scroll', () => {
    vi.useFakeTimers();
    const stop = initScrollIndicators();
    const el = document.createElement('div');
    document.body.append(el);
    el.dispatchEvent(new Event('scroll'));
    expect(el.classList.contains(SCROLLING_CLASS)).toBe(true);
    vi.advanceTimersByTime(SCROLL_INDICATOR_HIDE_MS - 100);
    el.dispatchEvent(new Event('scroll'));
    vi.advanceTimersByTime(SCROLL_INDICATOR_HIDE_MS - 100);
    expect(el.classList.contains(SCROLLING_CLASS), 'a new scroll restarts the wait').toBe(true);
    vi.advanceTimersByTime(200);
    expect(el.classList.contains(SCROLLING_CLASS)).toBe(false);
    stop();
    el.remove();
  });

  it('stops listening and clears the class when removed', () => {
    vi.useFakeTimers();
    const stop = initScrollIndicators();
    const el = document.createElement('div');
    document.body.append(el);
    el.dispatchEvent(new Event('scroll'));
    stop();
    expect(el.classList.contains(SCROLLING_CLASS)).toBe(false);
    el.dispatchEvent(new Event('scroll'));
    expect(el.classList.contains(SCROLLING_CLASS)).toBe(false);
    el.remove();
  });

  it('uses the Aura timing', () => {
    expect(SCROLL_INDICATOR_HIDE_MS).toBe(1200);
  });
});
