import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import WorkshopScreen from './WorkshopScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('WorkshopScreen Component Tests (SPEC 1.7)', () => {
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

  // 1. Empty States
  it('renders authentic empty state when worlds list is empty', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Сохранённых миров нет');
    expect(target.textContent).toContain('Создайте одиночный мир');

    unmount(comp);
  });

  it('renders authentic empty state when switched to screenshots tab with no screenshots', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_screenshots') return Promise.resolve([]);
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 40));

    const shotBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Скриншоты')
    );
    shotBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Снимков экрана пока нет');
    expect(target.textContent).toContain('Нажмите клавишу F2 во время игры');

    unmount(comp);
  });

  // 2. Loading State
  it('displays loading state on refresh button while content is loading', async () => {
    let resolveWorlds: (val: any) => void = () => {};
    const pendingPromise = new Promise((resolve) => {
      resolveWorlds = resolve;
    });

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return pendingPromise;
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 20));

    // Refresh button should show loading state
    const refreshBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Обновить')
    );
    expect(refreshBtn?.getAttribute('aria-busy')).toBe('true');

    resolveWorlds([]);
    await new Promise((r) => setTimeout(r, 40));
    unmount(comp);
  });

  // 3. Lists Populated from IPC Mock
  it('renders list of worlds from workshop_list_worlds IPC command', async () => {
    const mockWorlds = [
      {
        folder_name: 'SurvivalWorld',
        folder_path: 'C:/games/wex/instances/1.21.1/saves/SurvivalWorld',
        display_name: 'Выживание в тайге',
        game_mode: 'Survival',
        last_played: 1731110000,
        size_bytes: 148897792,
        icon_path: null,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve(mockWorlds);
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Выживание в тайге');
    expect(target.textContent).toContain('Выживание');
    expect(target.textContent).toContain('142.0 МБ');
    expect(target.textContent).toContain('SurvivalWorld');

    unmount(comp);
  });

  it('renders list of screenshots from workshop_list_screenshots IPC command', async () => {
    const mockScreenshots = [
      {
        file_name: '2026-11-08_15.30.00.png',
        file_path: 'C:/games/wex/instances/1.21.1/screenshots/2026-11-08_15.30.00.png',
        size_bytes: 2097152,
        created_at: '2026-11-08T15:30:00Z',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_screenshots') return Promise.resolve(mockScreenshots);
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 40));

    const shotBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Скриншоты')
    );
    shotBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('2026-11-08_15.30.00.png');
    expect(target.textContent).toContain('2.0 МБ');

    unmount(comp);
  });

  // 4. Button Clicks Call Expected IPC Commands With Arguments
  it('calls workshop_create_world_backup with worldFolder and version when Backup is clicked', async () => {
    const mockWorlds = [
      {
        folder_name: 'SurvivalWorld',
        folder_path: 'C:/saves/SurvivalWorld',
        display_name: 'Выживание в тайге',
        game_mode: 'Survival',
        last_played: 1731110000,
        size_bytes: 50000000,
        icon_path: null,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve(mockWorlds);
      if (cmd === 'workshop_create_world_backup') {
        return Promise.resolve({
          file_name: 'SurvivalWorld_backup.zip',
          file_path: 'C:/backups/SurvivalWorld_backup.zip',
          size_bytes: 30000000,
          created_at: '2026-11-08T18:00:00Z',
        });
      }
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const backupBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Резервная копия')
    );
    expect(backupBtn).toBeDefined();
    backupBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('workshop_create_world_backup', {
      worldFolder: 'C:/saves/SurvivalWorld',
      version: '1.21.1',
    });

    unmount(comp);
  });

  it('calls workshop_delete_world with worldFolder when Delete confirmation is accepted', async () => {
    const mockWorlds = [
      {
        folder_name: 'TrashWorld',
        folder_path: 'C:/saves/TrashWorld',
        display_name: 'Старый тестовый мир',
        game_mode: 'Creative',
        last_played: 1731110000,
        size_bytes: 10000000,
        icon_path: null,
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve(mockWorlds);
      if (cmd === 'workshop_delete_world') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    // Click Delete button on world item
    const deleteBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('Удалить') && b.closest('.world-actions')
    );
    expect(deleteBtn).toBeDefined();
    deleteBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    // Modal confirmation opens
    expect(target.textContent).toContain('Удаление мира');
    expect(target.textContent).toContain('Старый тестовый мир');

    // Click confirm «Удалить мир»
    const confirmBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Удалить мир')
    );
    expect(confirmBtn).toBeDefined();
    confirmBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('workshop_delete_world', {
      worldFolder: 'C:/saves/TrashWorld',
    });

    unmount(comp);
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

    invokeMock.mockImplementation((cmd: string) => {
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

    await new Promise((resolve) => setTimeout(resolve, 80));

    expect(target.textContent).toContain('Aura-Pack 1.21.1');
    expect(target.textContent).toContain('Faithful 32x');

    const auraCard = target.querySelector('[data-testid="pack-row-Aura-Pack.zip"]');
    expect(auraCard).not.toBeNull();
    expect(auraCard?.textContent).toContain('В сборке');
    expect(auraCard?.textContent).toContain('Включён');

    const userCard = target.querySelector('[data-testid="pack-row-Faithful32.zip"]');
    expect(userCard).not.toBeNull();
    const toggle = userCard?.querySelector('[role="switch"]') as HTMLElement;
    toggle?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(invokeMock).toHaveBeenCalledWith('workshop_toggle_resource_pack', {
      version: '1.21.1',
      fileName: 'Faithful32.zip',
      enabled: true,
    });

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

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_shader_packs') return Promise.resolve(mockShaders);
      if (cmd === 'workshop_set_active_shader_pack') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    const buttons = Array.from(target.querySelectorAll('button'));
    const shadersBtn = buttons.find((b) => b.textContent?.includes('Шейдеры'));
    expect(shadersBtn).toBeDefined();
    shadersBtn?.click();

    await new Promise((resolve) => setTimeout(resolve, 80));

    expect(target.textContent).toContain('Без шейдеров (OFF)');
    expect(target.textContent).toContain('Complementary Reimagined');

    const offCard = target.querySelector('.off-card');
    const offSelectBtn = offCard?.querySelector('button');
    offSelectBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(invokeMock).toHaveBeenCalledWith('workshop_set_active_shader_pack', {
      version: '1.21.1',
      shaderFileName: null,
    });

    unmount(comp);
  });

  it('calls workshop_open_packs_folder when open folder button is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_resource_packs') return Promise.resolve([]);
      if (cmd === 'workshop_open_packs_folder') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    const packsTabBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Ресурс-паки')
    );
    packsTabBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    const openFolderBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Папка паков')
    );
    expect(openFolderBtn).toBeDefined();
    openFolderBtn?.click();

    await new Promise((r) => setTimeout(r, 50));

    expect(invokeMock).toHaveBeenCalledWith('workshop_open_packs_folder', {
      packType: 'resourcepack',
      version: '1.21.1',
    });

    unmount(comp);
  });

  it('calls workshop_copy_screenshot_image and workshop_open_path from screenshot actions', async () => {
    const mockScreenshots = [
      {
        file_name: 'test_shot.png',
        file_path: 'C:/screenshots/test_shot.png',
        size_bytes: 1048576,
        created_at: '2026-11-08T12:00:00Z',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_screenshots') return Promise.resolve(mockScreenshots);
      if (cmd === 'workshop_copy_screenshot_image') return Promise.resolve();
      if (cmd === 'workshop_open_path') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 40));

    const shotBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Скриншоты')
    );
    shotBtn?.click();
    await new Promise((r) => setTimeout(r, 60));

    // Find copy button
    const copyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Копия')
    );
    expect(copyBtn).toBeDefined();
    copyBtn?.click();
    await new Promise((r) => setTimeout(r, 40));

    expect(invokeMock).toHaveBeenCalledWith('workshop_copy_screenshot_image', {
      path: 'C:/screenshots/test_shot.png',
    });

    // Find file folder reveal button
    const fileBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Файл')
    );
    expect(fileBtn).toBeDefined();
    fileBtn?.click();
    await new Promise((r) => setTimeout(r, 40));

    expect(invokeMock).toHaveBeenCalledWith('workshop_open_path', {
      path: 'C:/screenshots/test_shot.png',
    });

    unmount(comp);
  });
});
