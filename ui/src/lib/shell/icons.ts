/** Line icons from the reference (20×20 viewBox, stroke = currentColor). */
import type { Section } from './router.svelte';

export const NAV_ICONS: Record<Section, string> = {
  home: '<path d="M3.5 9.5L10 4l6.5 5.5V16a1 1 0 0 1-1 1H12v-4.5H8V17H4.5a1 1 0 0 1-1-1z"/>',
  servers:
    '<rect x="3" y="3.5" width="14" height="5.5" rx="1.5"/><rect x="3" y="11" width="14" height="5.5" rx="1.5"/><path d="M6.5 6.25h.01M6.5 13.75h.01"/>',
  friends:
    '<circle cx="7.5" cy="7" r="3"/><path d="M2.5 16.5c0-2.8 2.2-5 5-5s5 2.2 5 5"/><path d="M13 4.3a3 3 0 0 1 0 5.4M14.8 11.8c1.6.8 2.7 2.5 2.7 4.7"/>',
  wardrobe: '<path d="M7 3.5L3.5 6l1.7 3.2 1.6-.8v8.1h6.4V8.4l1.6.8L16.5 6 13 3.5a3 3 0 0 1-6 0z"/>',
  options:
    '<circle cx="10" cy="10" r="2.6"/><path d="M10 2.5v2.2M10 15.3v2.2M3.5 6.2l1.9 1.1M14.6 12.7l1.9 1.1M3.5 13.8l1.9-1.1M14.6 7.3l1.9-1.1"/>',
};
