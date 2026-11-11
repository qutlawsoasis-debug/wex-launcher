import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import WardrobeScreen from './WardrobeScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('WardrobeScreen Component Tests', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  // 1. Initial State & Render
  it('renders WardrobeScreen with character preview, model switcher, and cape options', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') {
        return Promise.resolve({
          skin_name: 'Custom Alex',
          skin_base64: null,
          model: 'Slim',
          cape_id: 'none',
          custom_cape_base64: null,
        });
      }
      if (cmd === 'extras_list_skin_history') {
        return Promise.resolve([]);
      }
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, {
      target,
      props: { playerName: 'Steve' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Custom Alex');
    expect(target.textContent).toContain('Тонкая модель (Alex)');
    expect(target.textContent).toContain('Без плаща');
    expect(target.textContent).toContain('Свой файл');
    expect(target.textContent).toContain('Применить к игре');

    unmount(comp);
  });

  // 2. Skin History List Populated from IPC Mock
  it('renders skins library list from extras_list_skin_history IPC command', async () => {
    const mockHistory = [
      {
        id: 'skin-001',
        name: 'Cool Knight',
        model: 'Classic',
        created_utc: '2026-11-08T12:00:00Z',
        relative_path: 'skins/cool_knight.png',
        hash: 'hash-001',
        item_type: 'skin',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') return Promise.resolve(null);
      if (cmd === 'extras_list_skin_history') return Promise.resolve(mockHistory);
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, { target });
    for (let i = 0; i < 20 && !target.textContent?.includes('Cool Knight'); i++) {
      await new Promise((r) => setTimeout(r, 20));
    }

    expect(target.textContent).toContain('Cool Knight');
    expect(target.textContent).toContain('Классическая');

    unmount(comp);
  });

  // 3. Selection of Skin from History Calling wardrobe_get_skin
  it('calls wardrobe_get_skin and updates active skin when history item is selected', async () => {
    const mockHistory = [
      {
        id: 'skin-002',
        name: 'Wizard Blue',
        model: 'Classic',
        created_utc: '2026-11-08T12:00:00Z',
        relative_path: 'skins/wizard_blue.png',
        hash: 'hash-002',
        item_type: 'skin',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') return Promise.resolve(null);
      if (cmd === 'extras_list_skin_history') return Promise.resolve(mockHistory);
      if (cmd === 'wardrobe_get_skin') {
        return Promise.resolve({
          id: 'skin-002',
          name: 'Wizard Blue',
          model: 'Classic',
          data_base64: 'new-active-base64',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, { target });
    for (let i = 0; i < 20 && !target.textContent?.includes('Wizard Blue'); i++) {
      await new Promise((r) => setTimeout(r, 20));
    }

    // Find select button «Выбрать» in history list
    const selectBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('.history-actions button')).find((el) =>
      el.textContent?.includes('Выбрать')
    );
    expect(selectBtn).toBeDefined();
    selectBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('wardrobe_get_skin', {
      request: {
        history_dir: '',
        item_id: 'skin-002',
      },
    });
    expect(target.textContent).toContain('Wizard Blue');

    unmount(comp);
  });

  // 4. Deleting Skin from History Calling wardrobe_delete_history
  it('calls wardrobe_delete_history with item id when delete icon is clicked', async () => {
    const mockHistory = [
      {
        id: 'skin-to-del',
        name: 'Skin To Delete',
        model: 'Slim',
        created_utc: '2026-11-08T12:00:00Z',
        relative_path: 'skins/del.png',
        hash: 'hash-del',
        item_type: 'skin',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') return Promise.resolve(null);
      if (cmd === 'extras_list_skin_history') return Promise.resolve(mockHistory);
      if (cmd === 'wardrobe_delete_history') return Promise.resolve();
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, { target });
    for (let i = 0; i < 20 && !target.textContent?.includes('Wizard Blue'); i++) {
      await new Promise((r) => setTimeout(r, 20));
    }

    const delBtn = target.querySelector('.history-actions button[aria-label="Удалить из истории"]') as HTMLButtonElement;
    expect(delBtn).not.toBeNull();
    delBtn.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('wardrobe_delete_history', {
      request: {
        history_dir: '',
        item_id: 'skin-to-del',
      },
    });

    unmount(comp);
  });

  // 5. Model Switcher & Apply Wardrobe Calling wardrobe_apply IPC
  it('switches model and calls wardrobe_apply with full payload when Apply button is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') {
        return Promise.resolve({
          skin_name: 'Hero Steve',
          skin_base64: 'hero-base64',
          model: 'Classic',
          cape_id: 'none',
        });
      }
      if (cmd === 'extras_list_skin_history') return Promise.resolve([]);
      if (cmd === 'wardrobe_apply') {
        return Promise.resolve({ success: true, message: 'Применено' });
      }
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, {
      target,
      props: { playerName: 'AlexPro' },
    });
    await new Promise((r) => setTimeout(r, 60));

    // Switch model to Slim (Alex)
    const slimBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find((b) =>
      b.textContent?.includes('Тонкая (3 пикселя)')
    );
    expect(slimBtn).toBeDefined();
    slimBtn?.click();
    await tick();

    // Click «Применить к игре»
    const applyBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find((b) =>
      b.textContent?.includes('Применить к игре')
    );
    expect(applyBtn).toBeDefined();
    applyBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('wardrobe_apply', {
      request: {
        nickname: 'AlexPro',
        skin_name: 'Hero Steve',
        skin_base64: 'hero-base64',
        model: 'Slim',
        cape_id: 'none',
        cape_name: 'Без плаща',
        cape_base64: null,
      },
    });

    unmount(comp);
  });

  // 6. Error Banner when Wardrobe Apply Fails
  it('displays error banner when wardrobe_apply backend call fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'wardrobe_get_state') return Promise.resolve(null);
      if (cmd === 'extras_list_skin_history') return Promise.resolve([]);
      if (cmd === 'wardrobe_apply') {
        return Promise.reject(new Error('Не удалось записать конфигурацию CSL'));
      }
      return Promise.resolve(null);
    });

    const comp = mount(WardrobeScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const applyBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find((b) =>
      b.textContent?.includes('Применить к игре')
    );
    expect(applyBtn).toBeDefined();
    applyBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    const errBanner = target.querySelector('.error-banner');
    expect(errBanner).not.toBeNull();
    expect(target.textContent).toContain('Не удалось записать конфигурацию CSL');

    unmount(comp);
  });
});
