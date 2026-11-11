/**
 * Page entrance: direct children rise one after another, 60 ms apart
 * (reference: .page.stagger). Durations come from tokens, so Reduce Motion
 * makes it instant. Use on the page wrapper inside a {#key route} block.
 */
export function stagger(node: HTMLElement) {
  [...node.children].forEach((el, i) => (el as HTMLElement).style.setProperty('--i', String(i)));
  node.classList.add('stagger');
}
