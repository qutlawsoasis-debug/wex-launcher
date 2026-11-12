/**
 * Wardrobe data, all through core:
 *   wardrobe_get_state      current nickname, skin (base64), arm model, cape
 *   extras_list_skin_history + wardrobe_get_skin   saved skins with their images
 *   extras_validate_skin / extras_convert_skin     check a PNG, turn 64×32 into 64×64
 *   wardrobe_apply          put on skin, model, cape, nickname (writes CustomSkinLoader for both builds)
 */

import { call } from '../../shell/ipc';

export type ArmModel = 'Classic' | 'Slim';

export interface WardrobeState {
  nickname: string;
  skin_name: string;
  skin_base64: string | null;
  model: ArmModel;
  cape_id: string;
  cape_name: string;
  cape_base64: string | null;
}

export interface SavedSkin {
  id: string;
  name: string;
  model: ArmModel;
  base64: string | null;
}

interface HistoryItem {
  id: string;
  name: string;
  model: string;
}

const DEFAULT: WardrobeState = {
  nickname: 'Player',
  skin_name: '',
  skin_base64: null,
  model: 'Classic',
  cape_id: 'none',
  cape_name: '',
  cape_base64: null,
};

/** Names reach the file system; keep letters, digits, spaces, dot, dash and underscore. */
export function safeName(name: string): string {
  const cleaned = name.replace(/\.png$/i, '').replace(/[^\p{L}\p{N} ._-]/gu, '').replace(/^[.\s]+/, '').trim().slice(0, 32);
  return cleaned || 'skin';
}

/** Slim (3 px) arms leave the outer column of the right arm empty. */
export function looksSlim(c: CanvasRenderingContext2D): boolean {
  try {
    return c.getImageData(55, 20, 1, 1).data[3] === 0 && c.getImageData(54, 20, 1, 1).data[3] === 0;
  } catch {
    return false;
  }
}

export function fileToBase64(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(String(r.result).replace(/^data:[^,]*,/, ''));
    r.onerror = () => reject(r.error);
    r.readAsDataURL(file);
  });
}

class Wardrobe {
  state = $state<WardrobeState>({ ...DEFAULT });
  saved = $state<SavedSkin[]>([]);
  loaded = $state(false);

  async load(): Promise<void> {
    try {
      const s = await call<Partial<WardrobeState>>('wardrobe_get_state');
      this.state = { ...DEFAULT, ...s, model: s.model === 'Slim' ? 'Slim' : 'Classic' } as WardrobeState;
    } catch {
      this.state = { ...DEFAULT };
    }
    this.loaded = true;
    await this.loadSaved();
  }

  async loadSaved(): Promise<void> {
    let items: HistoryItem[] = [];
    try {
      const res = await call<{ items: HistoryItem[] } | HistoryItem[]>('extras_list_skin_history', { request: { history_dir: '' } });
      items = Array.isArray(res) ? res : (res?.items ?? []);
    } catch {
      items = [];
    }
    const withImages = await Promise.all(
      items.map(async (it) => {
        let base64: string | null = null;
        try {
          const r = await call<{ skin_bytes_base64: string }>('wardrobe_get_skin', { request: { history_dir: '', item_id: it.id } });
          base64 = r.skin_bytes_base64 || null;
        } catch {}
        return { id: it.id, name: it.name, model: (it.model?.toLowerCase() === 'slim' ? 'Slim' : 'Classic') as ArmModel, base64 };
      })
    );
    this.saved = withImages;
  }

  /** Put on changes through core. The previous state comes back if core refuses. */
  async apply(patch: Partial<WardrobeState>): Promise<void> {
    const prev = this.state;
    const next = { ...prev, ...patch };
    this.state = next;
    try {
      const res = await call<{ success: boolean; message: string }>('wardrobe_apply', {
        request: {
          nickname: next.nickname,
          skin_name: next.skin_name || 'skin',
          skin_base64: next.skin_base64,
          model: next.model,
          cape_id: next.cape_id,
          cape_name: next.cape_name || next.cape_id,
          cape_base64: next.cape_id === 'custom' ? next.cape_base64 : null,
        },
      });
      if (res && res.success === false) throw new Error(res.message);
    } catch (e) {
      this.state = prev;
      throw e;
    }
  }

  /** Check a PNG with core; 64×32 is converted to 64×64. Returns the skin as base64. */
  async prepareSkin(base64: string): Promise<string> {
    const v = await call<{ is_valid: boolean; error?: string | null; width: number; height: number }>('extras_validate_skin', {
      request: { skin_bytes_base64: base64 },
    });
    if (!v.is_valid) throw new Error(v.error || 'invalid');
    if (v.height === 32) {
      const c = await call<{ converted_bytes_base64: string }>('extras_convert_skin', { request: { skin_bytes_base64: base64 } });
      return c.converted_bytes_base64;
    }
    return base64;
  }
}

export const wardrobe = new Wardrobe();
