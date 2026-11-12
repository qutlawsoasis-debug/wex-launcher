/**
 * Writes a line to the launcher's log file (`logs\launcher.log`) through `ui_log`,
 * so what the interface received and decided can be read back after a run.
 * Outside Tauri, and if the call fails, it does nothing.
 */
import { call, inTauri } from './ipc';

export function uiLog(level: 'info' | 'warn' | 'error', message: string): void {
  if (!inTauri()) return;
  call('ui_log', { level, message }).catch(() => {});
}
