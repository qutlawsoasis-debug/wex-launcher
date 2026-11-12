/**
 * The system file picker (tauri-plugin-dialog, open only). Returns real paths for
 * the core commands that take one (add_custom_mod, workshop_add_pack_file).
 * HIG › File management: use the system open panel, filter to the file types
 * that make sense, allow several files where adding several makes sense.
 */

import { open } from '@tauri-apps/plugin-dialog';
import { inTauri } from './ipc';

export interface PickOptions {
  title: string;
  /** Filter name shown in the picker, e.g. «Моды Minecraft». */
  filterName: string;
  extensions: string[];
  multiple?: boolean;
}

/** Paths the user picked; empty if they cancelled or the picker is unavailable. */
export async function pickFiles(o: PickOptions): Promise<string[]> {
  if (!inTauri()) return [];
  const picked = await open({
    title: o.title,
    multiple: o.multiple ?? true,
    directory: false,
    filters: [{ name: o.filterName, extensions: o.extensions }],
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}

/**
 * «Загрузить скин»: the WebView's own file chooser (the Windows open panel) for
 * one PNG. The skin commands take the image bytes (base64), not a path, and the
 * dialog plugin only returns paths, so this reads the file in the page instead.
 */
export function pickImageFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/png';
    input.addEventListener('change', () => resolve(input.files?.[0] ?? null), { once: true });
    input.addEventListener('cancel', () => resolve(null), { once: true });
    input.click();
  });
}
