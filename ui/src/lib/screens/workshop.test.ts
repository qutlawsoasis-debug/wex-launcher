import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import WorkshopScreen from './WorkshopScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('WorkshopScreen Resource Packs and Shaders (SPEC 1.7)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
  });

  it('renders resource packs with Aura-Pack protection badge and toggling', async () => {
    const mockPacks = [
      {
        name: 'Aura-Pack 1.21.1',
        file_name: 'Aura-Pack.zip',
        file_path: 'C:/fake/resourcepacks/Aura-Pack.zip',
        size_bytes: 15728640,
        description: 'Official Aura resource pack',
        pack_format: 34,
        icon_data_url: null,
        is_enabled: true,
        is_from_pack: true,
      },
      {
        name: 'Faithful 32x',
        file_name: 'Faithful32.zip',
        file_path: 'C:/fake/resourcepacks/Faithful32.zip',
        size_bytes: 20971520,
        description: 'High definition textures',
        pack_format: 34,
        icon_data_url: null,
        is_enabled: false,
        is_from_pack: false,
      },
    ];

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_resource_packs') return Promise.resolve(mockPacks);
      if (cmd === 'workshop_toggle_resource_pack') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    // Switch to Resource Packs tab
    const buttons = Array.from(target.querySelectorAll('button'));
    const packsBtn = buttons.find((b) => b.textContent?.includes('Ресурс-паки'));
    expect(packsBtn).toBeDefined();
    packsBtn?.click();

    // Wait for async loadResourcePacks
    await new Promise((resolve) => setTimeout(resolve, 80));

    // Verify both packs are listed
    expect(target.textContent).toContain('Aura-Pack 1.21.1');
    expect(target.textContent).toContain('Faithful 32x');

    // Official pack must have «В сборке» badge and NO delete button
    const auraCard = target.querySelector('[data-testid="pack-row-Aura-Pack.zip"]');
    expect(auraCard).not.toBeNull();
    expect(auraCard?.textContent).toContain('В сборке');
    expect(auraCard?.textContent).toContain('Включён');
    expect(auraCard?.querySelector('button.destructive')).toBeNull();

    // User pack must have «Удалить» button
    const userCard = target.querySelector('[data-testid="pack-row-Faithful32.zip"]');
    expect(userCard).not.toBeNull();
    expect(userCard?.textContent).not.toContain('В сборке');
    const deleteBtn = userCard?.querySelector('button.destructive');
    expect(deleteBtn).not.toBeNull();
    expect(deleteBtn?.textContent).toContain('Удалить');

    // Test toggle click
    const toggle = userCard?.querySelector('[role="switch"]') as HTMLElement;
    expect(toggle).not.toBeNull();
    toggle?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    // Check that workshop_toggle_resource_pack was called
    expect(invokeMock).toHaveBeenCalledWith('workshop_toggle_resource_pack', {
      version: '1.21.1',
      fileName: 'Faithful32.zip',
      enabled: true,
    });

    // Test clicking Delete button opens modal sheet
    (deleteBtn as HTMLElement)?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(target.textContent).toContain('Удаление пака');
    expect(target.textContent).toContain('Faithful 32x');
    expect(target.textContent).toContain('Faithful32.zip будет безвозвратно удалён');

    unmount(comp);
  });

  it('renders shaders list, "Без шейдеров (OFF)" card, and shader selection', async () => {
    const mockShaders = [
      {
        name: 'Complementary Reimagined',
        file_name: 'ComplementaryReimagined.zip',
        file_path: 'C:/fake/shaderpacks/ComplementaryReimagined.zip',
        size_bytes: 8388608,
        description: 'Vibrant modern shaders with Iris support',
        icon_data_url: null,
        is_active: true,
        is_from_pack: false,
      },
    ];

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_shader_packs') return Promise.resolve(mockShaders);
      if (cmd === 'workshop_set_active_shader_pack') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    // Switch to Shaders tab
    const buttons = Array.from(target.querySelectorAll('button'));
    const shadersBtn = buttons.find((b) => b.textContent?.includes('Шейдеры'));
    expect(shadersBtn).toBeDefined();
    shadersBtn?.click();

    // Wait for async loadShaderPacks
    await new Promise((resolve) => setTimeout(resolve, 80));

    // Verify "Без шейдеров (OFF)" card is present
    expect(target.textContent).toContain('Без шейдеров (OFF)');
    expect(target.textContent).toContain('Стандартный быстрый рендеринг Minecraft без внешних шейдеров.');

    // Verify Complementary is rendered with active badge
    expect(target.textContent).toContain('Complementary Reimagined');
    const compCard = target.querySelector('[data-testid="shader-row-ComplementaryReimagined.zip"]');
    expect(compCard).not.toBeNull();
    expect(compCard?.textContent).toContain('Активен');

    // Click "Выбрать" on "Без шейдеров (OFF)" card
    const offCard = target.querySelector('.off-card');
    expect(offCard).not.toBeNull();
    const offSelectBtn = offCard?.querySelector('button');
    expect(offSelectBtn).not.toBeNull();
    offSelectBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    // Check that workshop_set_active_shader_pack was called with shaderFileName: null
    expect(invokeMock).toHaveBeenCalledWith('workshop_set_active_shader_pack', {
      version: '1.21.1',
      shaderFileName: null,
    });

    unmount(comp);
  });
});
