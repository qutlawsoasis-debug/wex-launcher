import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { i18n } from '../../i18n';
import { rooms, type LobbyRoom } from './rooms.svelte';

const room: LobbyRoom = {
  code: '#AB12',
  host_name: 'Host',
  world_name: 'Мир',
  tunnel_address: 'tun.example:25565',
  tunnel_active: true,
  tunnel_error: null,
  is_host: true,
  players: [{ name: 'Host', is_host: true, ping_ms: 0, mods_synced: true }],
  mod_mismatch_count: 0,
  total_mods: 0,
};

function statusFails(error: unknown) {
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_lobby_status') throw error;
    return null;
  });
}

beforeEach(() => {
  i18n.setLocale('ru');
  mocks.invoke.mockReset();
  rooms.active = { ...room };
  rooms.closedNotice = '';
});

describe('room status polling', () => {
  it('a transient error or no answer keeps the room and never closes it', async () => {
    statusFails({ kind: 'transient', message: 'Сервис лобби временно недоступен.' });
    await rooms.poll();
    expect(rooms.active?.code).toBe('#AB12');
    expect(rooms.closedNotice).toBe('');
    mocks.invoke.mockResolvedValue(null);
    await rooms.poll();
    expect(rooms.active?.code).toBe('#AB12');
    expect(mocks.invoke).not.toHaveBeenCalledWith('close_lobby_room', expect.anything());
  });

  it('closed ends the room with a notice; the host is told why', async () => {
    statusFails({ kind: 'closed', message: 'Комната закрыта организатором' });
    await rooms.poll();
    expect(rooms.active).toBeNull();
    expect(rooms.closedNotice).toContain('мир больше не открыт');
    rooms.active = { ...room, is_host: false };
    await rooms.poll();
    expect(rooms.closedNotice).toBe('Комната закрыта организатором');
  });

  it('kicked ends the room with the kick notice', async () => {
    rooms.active = { ...room, is_host: false };
    statusFails({ kind: 'kicked', message: 'Вы были исключены из комнаты хостом' });
    await rooms.poll();
    expect(rooms.active).toBeNull();
    expect(rooms.closedNotice).toBe('Вы были исключены из комнаты организатором');
  });
});
