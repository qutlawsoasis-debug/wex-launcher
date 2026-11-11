/**
 * Floating selection indicator (reference: .ind). One shape per group glides to
 * the selected item with the spring curve; selection stays a fill, never an
 * outline (AGENTS.md; HIG › Sidebars, Segmented controls). Under Reduce Motion
 * --dur-ind is 0, so it jumps.
 *
 * Usage: <nav use:indicator={{ selector: '[aria-current="page"]', key: router.route }}>
 */

import { tick } from 'svelte';

export interface IndicatorOptions {
  /** Selector of the selected child inside the group. */
  selector: string;
  /** Anything that changes when the selection changes. */
  key?: unknown;
  /** 'fill' covers the item (sidebar, segments); 'underline' sits under it (tabs). */
  kind?: 'fill' | 'underline';
}

export function indicator(node: HTMLElement, options: IndicatorOptions) {
  let opts = options;
  const ind = document.createElement('span');
  ind.className = `ind ind-${opts.kind ?? 'fill'}`;
  ind.setAttribute('aria-hidden', 'true');
  node.prepend(ind);
  let placed = false;

  function place(animate: boolean) {
    const target = node.querySelector<HTMLElement>(opts.selector);
    if (!target || target.offsetParent === null) {
      ind.style.opacity = '0';
      placed = false;
      return;
    }
    const x = target.offsetLeft;
    const y = opts.kind === 'underline' ? 0 : target.offsetTop;
    if (!animate || !placed) ind.style.transition = 'none';
    ind.style.width = `${target.offsetWidth}px`;
    if (opts.kind !== 'underline') ind.style.height = `${target.offsetHeight}px`;
    ind.style.transform = `translate(${x}px, ${y}px)`;
    ind.style.opacity = '1';
    if (!animate || !placed) {
      void ind.offsetWidth;
      ind.style.transition = '';
    }
    placed = true;
  }

  const resize = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(() => place(false)) : null;
  resize?.observe(node);
  void tick().then(() => place(false));
  void document.fonts?.ready.then(() => place(false));

  return {
    update(next: IndicatorOptions) {
      opts = next;
      void tick().then(() => place(true));
    },
    destroy() {
      resize?.disconnect();
      ind.remove();
    },
  };
}
