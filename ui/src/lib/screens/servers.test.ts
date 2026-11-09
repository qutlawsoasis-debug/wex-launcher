import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ServersScreen from './ServersScreen.svelte';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('ServersScreen Component Tests', () => {
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
  it('renders authentic empty state when no servers exist in servers.dat', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_servers') return Promise.resolve([]);
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    expect(target.textContent).toContain('Серверы не найдены');
    expect(target.textContent).toContain('Список серверов пуст');

    unmount(comp);
  });

  // 2. Server List Populated from IPC Mock
  it('renders list of servers with name, address, and ping from list_servers and ping_server', async () => {
    const mockServers = [
      {
        id: 'srv-1',
        name: 'Hypixel Network',
        address: 'mc.hypixel.net',
        motd: 'Hypixel MMO and Minigames',
        version: '1.21.1',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Steve' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'ping_server') {
        return Promise.resolve({
          latency_ms: 45,
          online: 35000,
          max_players: 50000,
          motd: 'Hypixel MMO and Minigames',
          version: '1.21.1',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1', playerName: 'Steve' },
    });
    await new Promise((r) => setTimeout(r, 80));

    expect(target.textContent).toContain('Hypixel Network');
    expect(target.textContent).toContain('mc.hypixel.net');
    expect(target.textContent).toContain('45 мс');
    expect(target.textContent).toContain('35000/50000');
    expect(target.textContent).toContain('Играть');

    unmount(comp);
  });

  // 3. Ping Error State
  it('renders offline pill indicator when ping_server fails', async () => {
    const mockServers = [
      {
        id: 'srv-offline',
        name: 'Dead Server',
        address: 'offline.invalid.lan',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'ping_server') {
        return Promise.reject(new Error('Connection timed out'));
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 80));

    expect(target.textContent).toContain('Dead Server');
    const offlinePill = target.querySelector('.ping-pill.offline');
    expect(offlinePill).not.toBeNull();
    expect(offlinePill?.textContent).toContain('Офлайн');

    unmount(comp);
  });

  // 4. Add Server Action Calling add_server IPC
  it('opens Add Server sheet and calls add_server with name, address, and active version', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve([]);
      if (cmd === 'add_server') {
        return Promise.resolve({
          id: 'srv-new',
          name: 'Survival Server',
          address: 'mc.example.com',
        });
      }
      if (cmd === 'ping_server') {
        return Promise.resolve({
          latency_ms: 25,
          online: 10,
          max_players: 50,
          motd: 'Welcome',
          version: '1.21.1',
        });
      }
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 50));

    // Click «Добавить сервер» toolbar button
    const addBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('.toolbar-controls button')).find((b) =>
      b.textContent?.includes('Добавить сервер')
    );
    expect(addBtn).toBeDefined();
    addBtn?.click();
    await new Promise((r) => setTimeout(r, 40));

    // Fill sheet inputs
    const nameInput = target.querySelector('#new-srv-name') as HTMLInputElement;
    const addrInput = target.querySelector('#new-srv-addr') as HTMLInputElement;
    expect(nameInput).not.toBeNull();
    expect(addrInput).not.toBeNull();

    nameInput.value = 'Survival Server';
    nameInput.dispatchEvent(new Event('input', { bubbles: true }));
    addrInput.value = 'mc.example.com';
    addrInput.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();

    // Click Save in sheet
    const saveBtn = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Сохранить')
    );
    expect(saveBtn).toBeDefined();
    saveBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('add_server', {
      version: '1.21.1',
      name: 'Survival Server',
      address: 'mc.example.com',
    });

    unmount(comp);
  });

  // 5. Remove Server Action Calling remove_server IPC
  it('calls remove_server with address and version when delete button is clicked', async () => {
    const mockServers = [
      {
        id: 'srv-to-del',
        name: 'Server To Delete',
        address: 'del.example.com',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Player' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'remove_server') return Promise.resolve();
      if (cmd === 'ping_server') return Promise.resolve({ latency_ms: 10, online: 1, max_players: 10 });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const removeBtn = target.querySelector('.remove-btn') as HTMLButtonElement;
    expect(removeBtn).not.toBeNull();
    removeBtn.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('remove_server', {
      version: '1.21.1',
      address: 'del.example.com',
    });

    unmount(comp);
  });

  // 6. Connect / Play Server Calling launch_game IPC
  it('calls launch_game with server address, version, and player name when Play button is clicked', async () => {
    const mockServers = [
      {
        id: 'srv-play',
        name: 'Play Server',
        address: 'play.wex.lan:25565',
      },
    ];

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({ selected_version: '1.21.1', player_name: 'Alex' });
      if (cmd === 'list_servers') return Promise.resolve(mockServers);
      if (cmd === 'launch_game') return Promise.resolve({ success: true, pid: 1234 });
      if (cmd === 'ping_server') return Promise.resolve({ latency_ms: 15, online: 2, max_players: 20 });
      return Promise.resolve(null);
    });

    const comp = mount(ServersScreen, {
      target,
      props: { selectedVersion: '1.21.1', playerName: 'Alex' },
    });
    await new Promise((r) => setTimeout(r, 60));

    const playBtn = Array.from(target.querySelectorAll<HTMLButtonElement>('.server-metrics button')).find((b) =>
      b.textContent?.includes('Играть')
    );
    expect(playBtn).toBeDefined();
    playBtn?.click();

    await new Promise((r) => setTimeout(r, 60));

    expect(invokeMock).toHaveBeenCalledWith('launch_game', {
      version: '1.21.1',
      nickname: 'Alex',
      server: 'play.wex.lan:25565',
    });

    unmount(comp);
  });
});
