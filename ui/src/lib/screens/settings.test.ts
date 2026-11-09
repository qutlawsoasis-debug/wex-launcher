import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import SettingsScreen from './SettingsScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe('SettingsScreen Component Tests', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: Record<string, unknown>) => invokeMock(cmd, args),
      transformCallback: () => 1,
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  // 1. Default / Fallback State
  it('renders standard settings defaults when backend returns null', async () => {
    invokeMock.mockImplementation(() => Promise.resolve(null));

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Выделение оперативной памяти');
    expect(target.textContent).toContain('Среда выполнения Java');
    expect(target.textContent).toContain('Папки и файлы данных');
    expect(target.textContent).toContain('Тема оформления');

    unmount(comp);
  });

  // 2. Loading States
  it('displays loading state during verify_instance_files operation', async () => {
    let resolveVerify: (val: any) => void = () => {};
    const pendingPromise = new Promise((resolve) => {
      resolveVerify = resolve;
    });

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'verify_instance_files') return pendingPromise;
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const verifyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Проверить')
    );
    expect(verifyBtn).toBeDefined();
    verifyBtn?.click();

    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('Проверка...');
    expect(target.textContent).toContain('Проверка файлов по манифесту...');

    resolveVerify({
      success: true,
      total_files: 100,
      verified_files: 100,
      repaired_files: 0,
      corrupted_files: 0,
      errors: [],
      message: 'Все 100 файлов в порядке',
    });
    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Все 100 файлов в порядке');
    unmount(comp);
  });

  it('displays loading state during cleanup_cache operation', async () => {
    let resolveClean: (val: any) => void = () => {};
    const pendingPromise = new Promise((resolve) => {
      resolveClean = resolve;
    });

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'cleanup_cache') return pendingPromise;
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const cleanBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Очистить')
    );
    expect(cleanBtn).toBeDefined();
    cleanBtn?.click();

    await new Promise((r) => setTimeout(r, 20));

    expect(target.textContent).toContain('Очистка...');
    expect(target.textContent).toContain('Очистка кэша и логов...');

    resolveClean({
      success: true,
      freed_bytes: 10485760,
      freed_formatted: '10 МБ',
      deleted_files_count: 5,
      message: 'Кэш очищен: освобождено 10 МБ',
    });
    await new Promise((r) => setTimeout(r, 50));

    expect(target.textContent).toContain('Кэш очищен: освобождено 10 МБ');
    unmount(comp);
  });

  // 3. Error States
  it('displays error message when verify_instance_files fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'verify_instance_files') {
        return Promise.reject(new Error('Сетевой сбой манифеста'));
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const verifyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Проверить')
    );
    verifyBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Ошибка проверки: Сетевой сбой манифеста');
    unmount(comp);
  });

  it('displays error message when cleanup_cache fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'cleanup_cache') {
        return Promise.reject(new Error('Доступ к файлу заблокирован игрой'));
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const cleanBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Очистить')
    );
    cleanBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Ошибка очистки: Доступ к файлу заблокирован игрой');
    unmount(comp);
  });

  // 4. Populated from IPC Mock
  it('renders settings populated from get_system_info and get_launcher_settings IPC commands', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_system_info') {
        return Promise.resolve({
          total_memory_mb: 32768,
          os_name: 'Windows 11 Pro',
          launcher_version: '2.5.0',
          data_dir: 'D:\\Games\\WexLauncher',
        });
      }
      if (cmd === 'get_launcher_settings') {
        return Promise.resolve({
          selected_version: '1.21.1',
          player_name: 'ProGamer',
          ram_mb: 8192,
          total_memory_mb: 32768,
          game_dir: 'D:\\Games\\WexLauncher',
          graphics_preset: 'Ultra',
          java_mode: 'auto',
          custom_java_path: null,
          discord_rpc: true,
          fast_start_jvm: true,
          language: 'ru',
        });
      }
      if (cmd === 'music_get_settings') {
        return Promise.resolve({ enabled: true });
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('D:\\Games\\WexLauncher');
    expect(target.textContent).toContain('Windows 11 Pro');
    expect(target.textContent).toContain('2.5.0');
    expect(target.textContent).toContain('Фоновая музыка');

    unmount(comp);
  });

  // 5. Button Clicks Call Expected IPC Commands With Arguments
  it('calls verify_instance_files with selected version when clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_launcher_settings') {
        return Promise.resolve({ selected_version: '1.20.1' });
      }
      if (cmd === 'verify_instance_files') {
        return Promise.resolve({
          success: true,
          total_files: 50,
          verified_files: 50,
          repaired_files: 0,
          corrupted_files: 0,
          errors: [],
          message: 'Все 50 файлов проверены',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 60));

    const verifyBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Проверить')
    );
    expect(verifyBtn).toBeDefined();
    verifyBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('verify_instance_files', {
      version: '1.20.1',
    });

    unmount(comp);
  });

  it('calls cleanup_cache with selected version when clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'cleanup_cache') {
        return Promise.resolve({
          success: true,
          freed_bytes: 5242880,
          freed_formatted: '5.0 МБ',
          deleted_files_count: 2,
          message: 'Кэш очищен',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const cleanBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Очистить')
    );
    expect(cleanBtn).toBeDefined();
    cleanBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('cleanup_cache', {
      version: '1.21.1',
    });

    unmount(comp);
  });

  it('calls open_data_dir when Open data dir button is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'open_data_dir') return Promise.resolve();
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const openBtn = Array.from(target.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === 'Открыть'
    );
    expect(openBtn).toBeDefined();
    openBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('open_data_dir');

    unmount(comp);
  });

  it('calls reset_settings when Reset Defaults button is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'reset_settings') {
        return Promise.resolve({
          ram_mb: 4096,
          graphics_preset: 'High',
          java_mode: 'auto',
          custom_java_path: null,
          discord_rpc: true,
          fast_start_jvm: false,
          language: 'ru',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(SettingsScreen, { target });
    await new Promise((r) => setTimeout(r, 50));

    const resetBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Сбросить настройки')
    );
    expect(resetBtn).toBeDefined();
    resetBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('reset_settings');

    unmount(comp);
  });
});
