import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import LogsSheet from './LogsSheet.svelte';
import { builds } from '../shell/builds.svelte';
import type { LogFileInfo, LogReadResponse } from './LogsSheet.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('LogsSheet (SPEC 2.10: Apple HIG Game Logs Viewer)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    // The version switch lists the builds core reported.
    builds.list = [
      { id: '1.21.1', java: null, installed: true, mods: null },
      { id: '1.20.1', java: null, installed: true, mods: null },
    ];
    target = document.createElement('div');
    document.body.appendChild(target);
    vi.clearAllMocks();

    // Mock navigator.clipboard
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(() => {
    target.remove();
  });

  it('renders LogsSheet with toolbar controls, version selector, and file dropdown', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'logs_list_files') return Promise.resolve([]);
      if (cmd === 'logs_read_file') return Promise.resolve(null);
      return Promise.resolve();
    });

    const comp = mount(LogsSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
      },
    });

    // Header title
    expect(target.textContent).toContain('Журнал Minecraft');
    expect(target.textContent).toContain('1.21.1');
    expect(target.textContent).toContain('1.20.1');

    // Controls
    expect(target.querySelector('[data-testid="log-file-select"]')).not.toBeNull();
    expect(target.textContent).toContain('Все');
    expect(target.textContent).toContain('Ошибки');
    expect(target.textContent).toContain('Предупреждения');
    expect(target.textContent).toContain('Инфо');

    // Empty state without IPC
    expect(target.textContent).toContain('Журнал событий пуст');

    unmount(comp);
  });

  it('renders parsed log entries with ERROR and WARN highlights and metrics', async () => {
    const mockFiles: LogFileInfo[] = [
      {
        file_name: 'latest.log',
        relative_path: 'logs/latest.log',
        file_type: 'latest',
        size_bytes: 4096,
        modified_timestamp: 1700000000,
        is_crash: false,
      },
      {
        file_name: 'crash-2026-11-08-client.txt',
        relative_path: 'crash-reports/crash-2026-11-08-client.txt',
        file_type: 'crash_report',
        size_bytes: 8192,
        modified_timestamp: 1700000500,
        is_crash: true,
      },
    ];

    const mockResponse: LogReadResponse = {
      file_name: 'latest.log',
      total_lines: 3,
      returned_lines: 3,
      has_more: false,
      error_count: 1,
      warn_count: 1,
      entries: [
        {
          line_number: 1,
          timestamp: '12:00:01',
          level: 'INFO',
          thread: 'main',
          message: 'Loading mods...',
        },
        {
          line_number: 2,
          timestamp: '12:00:02',
          level: 'WARN',
          thread: 'Render thread',
          message: 'Mod sodium has slow chunk meshing',
        },
        {
          line_number: 3,
          timestamp: '12:00:03',
          level: 'ERROR',
          thread: 'Render thread',
          message: 'Critical rendering pipeline failure',
        },
      ],
    };

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'logs_list_files') return Promise.resolve(mockFiles);
      if (cmd === 'logs_read_file') return Promise.resolve(mockResponse);
      return Promise.resolve();
    });

    const comp = mount(LogsSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 60));

    // Metrics pills
    expect(target.textContent).toContain('1 ошибка');
    expect(target.textContent).toContain('1 предупреждение');
    expect(target.textContent).toContain('3 строк');

    // Lines content
    expect(target.textContent).toContain('Loading mods...');
    expect(target.textContent).toContain('Mod sodium has slow chunk meshing');
    expect(target.textContent).toContain('Critical rendering pipeline failure');

    // Error and Warn row styles
    const errorRow = target.querySelector('.error-row');
    expect(errorRow).not.toBeNull();
    expect(errorRow?.textContent).toContain('ERROR');

    const warnRow = target.querySelector('.warn-row');
    expect(warnRow).not.toBeNull();
    expect(warnRow?.textContent).toContain('WARN');

    unmount(comp);
  });

  it('filters log by level when user clicks Ошибки or Предупреждения', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'logs_list_files') return Promise.resolve([]);
      if (cmd === 'logs_read_file') return Promise.resolve({
        file_name: 'latest.log',
        total_lines: 0,
        returned_lines: 0,
        has_more: false,
        error_count: 0,
        warn_count: 0,
        entries: [],
      });
      return Promise.resolve();
    });

    const comp = mount(LogsSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 60));

    // Find and click «Ошибки» tab
    const buttons = Array.from(target.querySelectorAll('button'));
    const errorTabBtn = buttons.find((b) => b.textContent?.trim() === 'Ошибки');
    expect(errorTabBtn).toBeDefined();

    errorTabBtn?.click();
    await new Promise((resolve) => setTimeout(resolve, 60));

    expect(invokeMock).toHaveBeenCalledWith('logs_read_file', expect.objectContaining({
      filterLevel: 'ERROR',
    }));

    unmount(comp);
  });

  it('copies log content to clipboard and triggers folder opening', async () => {
    const mockResponse: LogReadResponse = {
      file_name: 'latest.log',
      total_lines: 1,
      returned_lines: 1,
      has_more: false,
      error_count: 0,
      warn_count: 0,
      entries: [
        {
          line_number: 1,
          timestamp: '15:30:00',
          level: 'INFO',
          thread: 'main',
          message: 'Server connection established',
        },
      ],
    };

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'logs_list_files') return Promise.resolve([]);
      if (cmd === 'logs_read_file') return Promise.resolve(mockResponse);
      if (cmd === 'logs_open_folder') return Promise.resolve('C:/Wex/logs');
      return Promise.resolve();
    });

    let toastTitle = '';
    const comp = mount(LogsSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        ontoast: (type, title) => {
          toastTitle = title;
        },
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 60));

    // Test Copy
    const copyBtn = target.querySelector('[data-testid="btn-copy-log"]');
    (copyBtn as HTMLElement)?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(navigator.clipboard.writeText).toHaveBeenCalled();
    expect(toastTitle).toContain('буфер');

    // Test Open Logs Folder
    const openLogsBtn = target.querySelector('[data-testid="btn-open-logs-folder"]');
    (openLogsBtn as HTMLElement)?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(invokeMock).toHaveBeenCalledWith('logs_open_folder', {
      version: '1.21.1',
      folderType: 'logs',
    });

    unmount(comp);
  });
});
