/**
 * Window keyboard shortcuts (reference: Settings › Горячие клавиши):
 *   Ctrl K       search and commands
 *   Ctrl 1 / 2   select Wex 1.21.1 / 1.20.1
 *   Alt ← / →    back / forward
 *   Enter        play the selected build (when nothing else has focus)
 *   Esc          close what is on top
 * HIG › Keyboards: standard modifiers, no shortcut steals typing in a text field.
 */

import { builds } from './builds.svelte';
import { launch } from './launch.svelte';
import { router } from './router.svelte';
import { palette, selectBuild } from './commands.svelte';

export interface HotkeyContext {
  /** True while a sheet, alert or the onboarding is open: only Esc and Ctrl K apply. */
  overlayOpen: boolean;
  /** Close the topmost overlay; return true if something was closed. */
  closeOverlay: () => boolean;
}

function typing(el: Element | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  return el.isContentEditable || el.matches('input, textarea, select');
}

/** Enter should play only when focus is not on something Enter already activates. */
function enterIsFree(el: Element | null): boolean {
  if (!(el instanceof HTMLElement) || el === document.body) return true;
  return !el.closest('button, a[href], input, textarea, select, [role="button"], [role="tab"], [role="option"], [role="switch"], [role="radio"], [tabindex]:not([tabindex="-1"])');
}

/** Handle a keydown on the window. Returns true when the key was used. */
export function handleHotkey(e: KeyboardEvent, ctx: HotkeyContext): boolean {
  const mod = e.ctrlKey || e.metaKey;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;

  if (mod && !e.altKey && !e.shiftKey && (key === 'k' || e.code === 'KeyK')) {
    if (palette.open) palette.hide();
    else palette.show();
    return true;
  }
  if (e.key === 'Escape') {
    if (palette.open) {
      palette.hide();
      return true;
    }
    return ctx.closeOverlay();
  }
  if (palette.open || ctx.overlayOpen) return false;

  if (mod && !e.altKey && !e.shiftKey && (key === '1' || key === '2')) {
    const b = builds.list[Number(key) - 1];
    if (b) void selectBuild(b.id);
    return true;
  }
  if (e.altKey && !mod && e.key === 'ArrowLeft') {
    router.goBack();
    return true;
  }
  if (e.altKey && !mod && e.key === 'ArrowRight') {
    router.goForward();
    return true;
  }
  if (e.key === 'Enter' && !mod && !e.altKey && !e.shiftKey && !e.repeat) {
    const el = document.activeElement;
    if (typing(el) || !enterIsFree(el)) return false;
    if (launch.phase === 'idle' && builds.selected) {
      void launch.start();
      return true;
    }
  }
  return false;
}
