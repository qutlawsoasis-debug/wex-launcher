/**
 * Shows an element's scroll indicator while it scrolls and hides it after
 * 1.2 s without scrolling (Aura «Полоса прокрутки», from aura-client ScrollList).
 * The look is in wex.css; this only toggles `.is-scrolling` on the element.
 */

export const SCROLL_INDICATOR_HIDE_MS = 1200;
export const SCROLLING_CLASS = 'is-scrolling';

export function initScrollIndicators(root: Document = document): () => void {
  const timers = new Map<Element, ReturnType<typeof setTimeout>>();

  function onScroll(e: Event) {
    const el = e.target === root ? root.scrollingElement : e.target;
    if (!(el instanceof Element)) return;
    el.classList.add(SCROLLING_CLASS);
    clearTimeout(timers.get(el));
    timers.set(
      el,
      setTimeout(() => {
        el.classList.remove(SCROLLING_CLASS);
        timers.delete(el);
      }, SCROLL_INDICATOR_HIDE_MS)
    );
  }

  // Scroll events do not bubble; capture sees every scrolling element.
  root.addEventListener('scroll', onScroll, { capture: true, passive: true });
  return () => {
    root.removeEventListener('scroll', onScroll, { capture: true });
    for (const [el, t] of timers) {
      clearTimeout(t);
      el.classList.remove(SCROLLING_CLASS);
    }
    timers.clear();
  };
}
