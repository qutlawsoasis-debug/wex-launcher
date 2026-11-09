import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ModsScreen from './ModsScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('ModsScreen Component Tests', () => {
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

  // 1. Empty State
  it('renders authentic empty state when no mods are found in instance folder', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_mods') return Promise.resolve([]);
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, {
      target,
      props: { initialVersion: '1.20.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Моды не найдены');
    expect(target.textContent).toContain('В папке игры пока нет модов');

    unmount(comp);
  });

  // 2. Mod List Populated from IPC Mock
  it('renders list of mods from list_mods with pack badge for official mods', async () => {
    const mockMods = [
      {
        id: 'fabric-api',
        name: 'Fabric API',
        version: '0.102.0',
        description: 'Базовые библиотеки Fabric',
        file_name: 'fabric-api-0.102.0.jar',
        enabled: true,
        is_pack_mod: true,
      },
      {
        id: 'minimap',
        name: 'Xaero Minimap',
        version: '24.1.0',
        description: 'Миникарта в углу экрана',
        file_name: 'xaero-minimap-24.1.0.jar',
        enabled: false,
        is_pack_mod: false,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_mods') return Promise.resolve(mockMods);
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Fabric API');
    expect(target.textContent).toContain('0.102.0');
    expect(target.textContent).toContain('Пак Wex');
    expect(target.textContent).toContain('Xaero Minimap');
    expect(target.textContent).toContain('24.1.0');

    // Official mod has no delete button
    const packRow = Array.from(target.querySelectorAll('.apple-row')).find((el) =>
      el.textContent?.includes('Fabric API')
    );
    expect(packRow?.querySelector('.delete-btn')).toBeNull();

    // User mod has delete button
    const userRow = Array.from(target.querySelectorAll('.apple-row')).find((el) =>
      el.textContent?.includes('Xaero Minimap')
    );
    expect(userRow?.querySelector('.delete-btn')).not.toBeNull();

    unmount(comp);
  });

  // 3. Mod Toggle Action Calling toggle_mod IPC
  it('calls toggle_mod with version, fileName, and new state when switch is toggled', async () => {
    const mockMods = [
      {
        id: 'sodium',
        name: 'Sodium',
        version: '0.5.8',
        description: 'Modern rendering engine',
        file_name: 'sodium-fabric-0.5.8.jar',
        enabled: true,
        is_pack_mod: false,
      },
    ];

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'list_mods') return Promise.resolve(mockMods);
      if (cmd === 'toggle_mod') {
        return Promise.resolve({
          ...mockMods[0],
          enabled: args.enabled,
          file_name: args.enabled ? 'sodium-fabric-0.5.8.jar' : 'sodium-fabric-0.5.8.jar.disabled',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, {
      target,
      props: { initialVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const toggle = target.querySelector('[role="switch"]') as HTMLElement;
    expect(toggle).not.toBeNull();
    toggle.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('toggle_mod', {
      version: '1.21.1',
      fileName: 'sodium-fabric-0.5.8.jar',
      enabled: false,
    });

    unmount(comp);
  });

  // 4. Delete Custom Mod Action Calling delete_custom_mod IPC
  it('calls delete_custom_mod with version and fileName when delete button is clicked', async () => {
    const mockMods = [
      {
        id: 'voice-chat',
        name: 'Simple Voice Chat',
        version: '2.5.0',
        description: 'Proximity voice chat',
        file_name: 'voicechat-fabric-2.5.0.jar',
        enabled: true,
        is_pack_mod: false,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_mods') return Promise.resolve(mockMods);
      if (cmd === 'delete_custom_mod') return Promise.resolve();
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, {
      target,
      props: { initialVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const deleteBtn = target.querySelector('.delete-btn') as HTMLButtonElement;
    expect(deleteBtn).not.toBeNull();
    deleteBtn.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('delete_custom_mod', {
      version: '1.21.1',
      fileName: 'voicechat-fabric-2.5.0.jar',
    });

    unmount(comp);
  });

  // 5. Open Mods Folder Calling open_mods_dir IPC
  it('calls open_mods_dir with selected version when Folder button is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_mods') return Promise.resolve([]);
      if (cmd === 'open_mods_dir') return Promise.resolve();
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, {
      target,
      props: { initialVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 50));

    const folderBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('button')).find((b) =>
      b.textContent?.includes('Папка с модами')
    );
    expect(folderBtn).toBeDefined();
    folderBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('open_mods_dir', {
      version: '1.21.1',
    });

    unmount(comp);
  });

  // 6. Search Filter
  it('filters mod list dynamically based on search query', async () => {
    const mockMods = [
      {
        id: 'mod-iris',
        name: 'Iris Shaders',
        version: '1.7.0',
        description: 'Modern shaders for Minecraft',
        file_name: 'iris-1.7.0.jar',
        enabled: true,
        is_pack_mod: true,
      },
      {
        id: 'mod-sodium',
        name: 'Sodium',
        version: '0.5.8',
        description: 'Performance boost',
        file_name: 'sodium-0.5.8.jar',
        enabled: true,
        is_pack_mod: true,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_mods') return Promise.resolve(mockMods);
      return Promise.resolve(null);
    });

    const comp = mount(ModsScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Iris Shaders');
    expect(target.textContent).toContain('Sodium');

    // Type "iris" into search input
    const searchInput = target.querySelector('.search-wrap input') as HTMLInputElement;
    expect(searchInput).not.toBeNull();
    searchInput.value = 'iris';
    searchInput.dispatchEvent(new Event('input', { bubbles: true }));
    await new Promise((r) => setTimeout(r, 60));

    // Iris is visible, Sodium is filtered out
    expect(target.textContent).toContain('Iris Shaders');
    expect(target.textContent).not.toContain('Sodium');

    unmount(comp);
  });
});
