/**
 * Files dropped onto the window from Explorer. Tauri intercepts drag and drop
 * (dragDropEnabled), so paths come from the webview's drag-drop event — the only
 * way the UI can get a real file path without a file dialog (core has none).
 */

import { getCurrentWebview } from '@tauri-apps/api/webview';
import { inTauri } from './ipc';

export interface DropHandlers {
  /** Called with the dropped paths. */
  drop: (paths: string[]) => void;
  /** Called with true while files hover over the window. */
  hover?: (over: boolean) => void;
}

export function onFileDrop(handlers: DropHandlers): () => void {
  if (!inTauri()) return () => {};
  let unlisten: (() => void) | null = null;
  let stopped = false;
  // getCurrentWebview() throws synchronously when the window metadata is missing;
  // file drop is optional, so that never breaks the page.
  let webview: ReturnType<typeof getCurrentWebview>;
  try {
    webview = getCurrentWebview();
  } catch {
    return () => {};
  }
  webview
    .onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter' || p.type === 'over') handlers.hover?.(true);
      else if (p.type === 'leave') handlers.hover?.(false);
      else if (p.type === 'drop') {
        handlers.hover?.(false);
        if (p.paths.length) handlers.drop(p.paths);
      }
    })
    .then((fn) => {
      if (stopped) fn();
      else unlisten = fn;
    })
    .catch(() => {});
  return () => {
    stopped = true;
    unlisten?.();
  };
}
