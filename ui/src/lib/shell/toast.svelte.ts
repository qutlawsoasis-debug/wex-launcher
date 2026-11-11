/**
 * One short notice at a time above the launch bar (reference: .toast).
 * HIG › Notifications / Feedback: brief, non-blocking, announced to assistive
 * technology through role="status". Errors get a second line with the detail.
 */

import { WexMotion } from '../design/wex-tokens';

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastItem {
  id: number;
  title: string;
  detail?: string;
  action?: ToastAction;
}

class Toasts {
  current = $state<ToastItem | null>(null);
  private seq = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;

  show(title: string, detail?: string, action?: ToastAction): void {
    this.current = { id: ++this.seq, title, detail: detail || undefined, action };
    if (this.timer) clearTimeout(this.timer);
    // Longer when there is more to read or something to press.
    const ms = WexMotion.ms.toast + (detail ? 2400 : 0) + (action ? 3000 : 0);
    this.timer = setTimeout(() => this.dismiss(), ms);
  }

  dismiss(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.current = null;
  }
}

export const toasts = new Toasts();
