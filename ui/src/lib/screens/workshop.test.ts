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
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
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
    expect(auraCard?.textContent).toContain('В паке');
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

  it('(wex-workshop-polish) opens world in lobby with single click, calling create_lobby_room and notifying listener', async () => {
    const mockWorlds = [
      {
        folder_name: 'world_survival_1',
        display_name: 'Тайга и Горы',
        game_mode: 'Survival',
        last_played: 1731110000000,
        size_bytes: 52428800,
        folder_path: 'C:/games/wex/instances/1.21.1/saves/world_survival_1',
        icon_path: null,
      },
    ];

    let createdLobbyRoom: any = null;
    let onOpenLobbyPayload: any = null;

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve(mockWorlds);
      if (cmd === 'create_lobby_room') {
        createdLobbyRoom = {
          code: 'W7K2',
          host_name: 'Alex',
          world_name: args.worldName,
          tunnel_address: 'tunnel.playit.gg:25565',
          tunnel_active: true,
          tunnel_error: null,
          is_host: true,
          players: [],
          mod_mismatch_count: 0,
          total_mods: 100,
        };
        return Promise.resolve(createdLobbyRoom);
      }
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: {
        selectedVersion: '1.21.1',
        playerName: 'Alex',
        onopenlobby: (room: any) => {
          onOpenLobbyPayload = room;
        },
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 80));

    expect(target.textContent).toContain('Тайга и Горы');
    const openLobbyBtn = target.querySelector('[data-testid="open-in-lobby-btn-world_survival_1"] button') as HTMLButtonElement;
    expect(openLobbyBtn).not.toBeNull();
    expect(openLobbyBtn.textContent).toContain('Открыть в лобби');

    // Trigger single-click Open in Lobby
    openLobbyBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 60));

    expect(invokeMock).toHaveBeenCalledWith('create_lobby_room', {
      worldName: 'Тайга и Горы',
      playerName: 'Alex',
    });
    expect(onOpenLobbyPayload).not.toBeNull();
    expect(onOpenLobbyPayload.code).toBe('W7K2');

    unmount(comp);
  });

  it('(wex-workshop-polish) prompts confirmation before restoring world and creates safety backup if requested', async () => {
    const mockWorlds = [
      {
        folder_name: 'world_survival_1',
        display_name: 'Тайга и Горы',
        game_mode: 'Survival',
        last_played: 1731110000000,
        size_bytes: 52428800,
        folder_path: 'C:/games/wex/instances/1.21.1/saves/world_survival_1',
        icon_path: null,
      },
    ];

    const mockBackups = [
      {
        file_name: 'world_survival_1_2026-11-08.zip',
        file_path: 'C:/games/wex/instances/1.21.1/backups/world_survival_1_2026-11-08.zip',
        size_bytes: 41943040,
        created_at: '2026-11-08T12:00:00Z',
      },
    ];

    let backupCreated = false;
    let backupRestored = false;

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve(mockWorlds);
      if (cmd === 'workshop_list_world_backups') return Promise.resolve(mockBackups);
      if (cmd === 'workshop_create_world_backup') {
        backupCreated = true;
        return Promise.resolve({
          file_name: 'safety_backup.zip',
          file_path: 'C:/safety_backup.zip',
          size_bytes: 40000000,
          created_at: '2026-11-09T03:00:00Z',
        });
      }
      if (cmd === 'workshop_restore_world_backup') {
        backupRestored = true;
        return Promise.resolve();
      }
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    await new Promise((resolve) => setTimeout(resolve, 80));

    // Open Backups sheet
    const restoreSheetBtn = target.querySelector('[data-testid="restore-world-btn-world_survival_1"] button') as HTMLButtonElement;
    expect(restoreSheetBtn).not.toBeNull();
    restoreSheetBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 60));

    // Backups sheet is visible
    expect(target.querySelector('[data-testid="backups-modal"]')).not.toBeNull();
    const restoreBtn = target.querySelector('[data-testid="restore-backup-btn-world_survival_1_2026-11-08.zip"] button') as HTMLButtonElement;
    expect(restoreBtn).not.toBeNull();

    // Click Restore -> opens Confirm Restore dialog
    restoreBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 60));

    const confirmModal = target.querySelector('[data-testid="restore-confirm-modal"]');
    expect(confirmModal).not.toBeNull();
    expect(confirmModal?.textContent).toContain('Восстановить мир');
    expect(confirmModal?.textContent).toContain('Создать резервную копию текущего мира перед восстановлением');

    const checkbox = target.querySelector('input[data-testid="backup-before-restore-checkbox"]') as HTMLInputElement;
    expect(checkbox).not.toBeNull();
    expect(checkbox.checked).toBe(true);

    // Confirm restore with safety backup checked
    const confirmBtn = target.querySelector('[data-testid="confirm-restore-btn-wrap"] button') as HTMLButtonElement;
    expect(confirmBtn).not.toBeNull();
    confirmBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 60));

    // Verify both commands were called in order: backup THEN restore
    expect(backupCreated).toBe(true);
    expect(backupRestored).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith('workshop_create_world_backup', {
      worldFolder: 'C:/games/wex/instances/1.21.1/saves/world_survival_1',
      version: '1.21.1',
    });
    expect(invokeMock).toHaveBeenCalledWith('workshop_restore_world_backup', {
      backupPath: 'C:/games/wex/instances/1.21.1/backups/world_survival_1_2026-11-08.zip',
      targetWorldFolder: 'C:/games/wex/instances/1.21.1/saves/world_survival_1',
      version: '1.21.1',
    });

    unmount(comp);
  });

  it('(wex-workshop-polish) screenshots copy image and open folder via dedicated buttons', async () => {
    const mockScreenshots = [
      {
        file_name: '2026-11-08_15.30.00.png',
        file_path: 'C:/games/wex/instances/1.21.1/screenshots/2026-11-08_15.30.00.png',
        size_bytes: 1048576,
        created_at: '2026-11-08T15:30:00Z',
      },
    ];

    invokeMock.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_screenshots') return Promise.resolve(mockScreenshots);
      if (cmd === 'workshop_copy_screenshot_image') return Promise.resolve();
      if (cmd === 'workshop_open_screenshots_folder') return Promise.resolve();
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1', initialTab: 'screenshots' },
    });

    await new Promise((resolve) => setTimeout(resolve, 80));

    // Screenshots grid is rendered
    expect(target.querySelector('[data-testid="screenshots-grid"]')).not.toBeNull();
    expect(target.textContent).toContain('2026-11-08_15.30.00.png');

    // Click «Открыть папку» in toolbar
    const toolbarFolderBtn = target.querySelector('[data-testid="open-screenshots-folder-wrap"] button') as HTMLButtonElement;
    expect(toolbarFolderBtn).not.toBeNull();
    toolbarFolderBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(invokeMock).toHaveBeenCalledWith('workshop_open_screenshots_folder', {
      version: '1.21.1',
    });

    // Click «Скопировать» on screenshot item
    const copyBtn = target.querySelector('button[data-testid="copy-screenshot-btn"]') as HTMLButtonElement;
    expect(copyBtn).not.toBeNull();
    copyBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(invokeMock).toHaveBeenCalledWith('workshop_copy_screenshot_image', {
      path: 'C:/games/wex/instances/1.21.1/screenshots/2026-11-08_15.30.00.png',
    });

    // Click «Открыть папку» on screenshot card
    const cardFolderBtn = target.querySelector('button[data-testid="open-screenshot-folder-btn"]') as HTMLButtonElement;
    expect(cardFolderBtn).not.toBeNull();
    cardFolderBtn.click();
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(invokeMock).toHaveBeenCalledWith('workshop_open_screenshots_folder', {
      version: '1.21.1',
    });

    unmount(comp);
  });

  it('(wex-workshop-polish) displays plain-words empty states across tabs', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'workshop_list_worlds') return Promise.resolve([]);
      if (cmd === 'workshop_list_world_backups') return Promise.resolve([]);
      if (cmd === 'workshop_list_resource_packs') return Promise.resolve([]);
      if (cmd === 'workshop_list_shader_packs') return Promise.resolve([]);
      if (cmd === 'workshop_list_screenshots') return Promise.resolve([]);
      return Promise.resolve();
    });

    const comp = mount(WorkshopScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });

    await new Promise((resolve) => setTimeout(resolve, 80));

    // Tab 1: Worlds empty state
    const emptyWorlds = target.querySelector('[data-testid="empty-worlds"]');
    expect(emptyWorlds).not.toBeNull();
    expect(emptyWorlds?.textContent).toContain('Миров пока нет');
    expect(emptyWorlds?.textContent).toContain('Создайте мир в одиночной игре');

    // Switch to Resource packs
    const buttons = Array.from(target.querySelectorAll('button'));
    const packsBtn = buttons.find((b) => b.textContent?.includes('Ресурс-паки'));
    packsBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 80));

    const emptyPacks = target.querySelector('[data-testid="empty-resourcepacks"]');
    expect(emptyPacks).not.toBeNull();
    expect(emptyPacks?.textContent).toContain('Ресурс-паков пока нет');

    // Switch to Shaders
    const shadersBtn = Array.from(target.querySelectorAll('button')).find((b) => b.textContent?.includes('Шейдеры'));
    shadersBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 80));

    const emptyShaders = target.querySelector('[data-testid="empty-shaders"]');
    expect(emptyShaders).not.toBeNull();
    expect(emptyShaders?.textContent).toContain('Шейдер-паков пока нет');

    // Switch to Screenshots
    const screensBtn = Array.from(target.querySelectorAll('button')).find((b) => b.textContent?.includes('Скриншоты'));
    screensBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 80));

    const emptyScreens = target.querySelector('[data-testid="empty-screenshots"]');
    expect(emptyScreens).not.toBeNull();
    expect(emptyScreens?.textContent).toContain('Скриншотов пока нет');
    expect(emptyScreens?.textContent).toContain('Нажмите F2 во время игры');

    unmount(comp);
  });
});
