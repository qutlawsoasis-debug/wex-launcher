/**
 * Rooms (the former Lobby screen, now a Friends tab — owner decision). Same core
 * flow as before:
 *   join by code     prepare_lobby_join (+ lobby-join-progress events) → start the game on the tunnel address
 *   open a world     LAN discovery (net_lan_start/stop_discovery, list_lan_worlds) → publish_lan_world
 *   LAN connect      connect_lan_world
 *   open rooms       list_open_rooms
 *   in a room        get_lobby_status polling, retry_lobby_tunnel, kick_lobby_player, close_lobby_room,
 *                    social_send_lobby_invite
 */

import { call, on } from '../../shell/ipc';
import { builds } from '../../shell/builds.svelte';
import { launch } from '../../shell/launch.svelte';
import { t } from '../../i18n';
import type { LobbyRoom, OpenRoom } from './lobbyTypes';

export type { LobbyRoom, OpenRoom };

export interface LanWorld {
  motd: string;
  address: string;
  port: number;
}

export type JoinStep = 'idle' | 'checking_mods' | 'downloading_mods' | 'connecting' | 'error';

function message(e: unknown, fallback: string): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error && e.message) return e.message;
  return fallback;
}

class Rooms {
  active = $state<LobbyRoom | null>(null);
  lan = $state<LanWorld[]>([]);
  open = $state<OpenRoom[] | null>(null);
  openError = $state<string | null>(null);
  error = $state('');
  closedNotice = $state('');
  joining = $state(false);
  joinStep = $state<JoinStep>('idle');
  joinText = $state('');
  joinPercent = $state<number | null>(null);
  modsDiffer = $state(false);
  publishing = $state<number | null>(null);
  retrying = $state(false);
  invited = $state<string[]>([]);

  async refreshLan(): Promise<void> {
    try {
      this.lan = (await call<LanWorld[]>('list_lan_worlds')) ?? [];
    } catch {
      this.lan = [];
    }
  }

  async startDiscovery(): Promise<void> {
    try {
      await call('net_lan_start_discovery');
    } catch (e) {
      this.error = message(e, t('lobby.lan_discover_failed'));
    }
  }

  async stopDiscovery(): Promise<void> {
    try {
      await call('net_lan_stop_discovery');
    } catch {}
  }

  async refreshOpen(): Promise<void> {
    this.openError = null;
    try {
      this.open = (await call<OpenRoom[]>('list_open_rooms', { playerName: builds.playerName })) ?? [];
    } catch (e) {
      this.open = [];
      this.openError = message(e, t('lobby.service_unavailable'));
    }
  }

  async publish(world: LanWorld): Promise<void> {
    if (this.publishing !== null) return;
    this.publishing = world.port;
    this.error = '';
    this.closedNotice = '';
    try {
      this.active = await call<LobbyRoom>('publish_lan_world', {
        motd: world.motd,
        port: world.port,
        playerName: builds.playerName,
        version: builds.selectedId,
      });
      this.invited = [];
    } catch (e) {
      this.error = message(e, t('lobby.create_room_failed'));
    } finally {
      this.publishing = null;
    }
  }

  async connectLan(address: string): Promise<string> {
    this.error = '';
    try {
      return (await call<string>('connect_lan_world', { address, version: builds.selectedId, nickname: builds.playerName })) || t('lobby.lan_connecting');
    } catch (e) {
      this.error = message(e, t('lobby.lan_connect_failed'));
      return '';
    }
  }

  async join(code: string, repair = false): Promise<void> {
    const clean = code.trim().replace(/^#/, '').toUpperCase();
    if (!clean || this.joining) return;
    this.joining = true;
    this.error = '';
    this.closedNotice = '';
    this.joinStep = 'idle';
    this.joinPercent = null;
    this.joinText = t('lobby.waiting_room_response');
    const stops = [
      on<{ code: string; stage: JoinStep; message: string }>('lobby-join-progress', (p) => {
        if (p.code !== clean) return;
        this.joinStep = p.stage;
        this.joinText = p.message;
        this.joinPercent = null;
      }),
      on<{ stage: string; percentage: number; detail: string }>('install-progress', (p) => {
        if (this.joinStep !== 'downloading_mods') return;
        this.joinPercent = Number.isFinite(p.percentage) ? p.percentage : null;
        this.joinText = p.detail || p.stage;
      }),
    ];
    try {
      const room = await call<LobbyRoom>('prepare_lobby_join', { code: clean, playerName: builds.playerName, version: builds.selectedId, repair });
      if (!room || room.mod_mismatch_count > 0) {
        this.modsDiffer = Boolean(room && room.mod_mismatch_count > 0);
        throw new Error(t('lobby.mods_differ_hint'));
      }
      if (!room.tunnel_active || !room.tunnel_address) throw new Error(room.tunnel_error || t('lobby.host_not_opened'));
      this.active = room;
      this.modsDiffer = false;
      this.joinStep = 'idle';
      void launch.start(room.tunnel_address);
    } catch (e) {
      this.error = message(e, t('lobby.join_room_failed'));
      this.modsDiffer = this.modsDiffer || this.error.includes(t('lobby.mods_differ_hint'));
      this.joinStep = 'error';
    } finally {
      for (const stop of stops) stop();
      this.joining = false;
    }
  }

  /** Poll the room; the host closing it or kicking us ends it with a notice. */
  async poll(): Promise<void> {
    const current = this.active;
    if (!current) return;
    try {
      const updated = await call<LobbyRoom | null>('get_lobby_status', { code: current.code, playerName: builds.playerName });
      if (!updated) {
        this.active = null;
        this.closedNotice = t('lobby.room_closed_by_host');
        return;
      }
      this.active = updated;
    } catch (e) {
      const msg = message(e, '');
      this.active = null;
      this.closedNotice = msg.includes('исключены') ? t('lobby.kicked_by_host') : t('lobby.room_closed_by_host');
    }
  }

  async retryTunnel(): Promise<void> {
    const room = this.active;
    if (!room) return;
    this.retrying = true;
    try {
      const updated = await call<LobbyRoom>('retry_lobby_tunnel', { code: room.code });
      if (updated) this.active = updated;
    } catch (e) {
      this.active = { ...room, tunnel_active: false, tunnel_error: message(e, t('lobby.err_tunnel_failed')) };
    } finally {
      this.retrying = false;
    }
  }

  async kick(name: string): Promise<void> {
    const room = this.active;
    if (!room?.is_host) return;
    try {
      await call('kick_lobby_player', { code: room.code, playerName: name });
      this.active = { ...room, players: room.players.filter((p) => p.name !== name) };
    } catch (e) {
      this.error = message(e, t('lobby.kick_player_failed'));
    }
  }

  async leave(): Promise<void> {
    const room = this.active;
    if (room) {
      try {
        await call('close_lobby_room', { code: room.code });
      } catch {}
    }
    this.active = null;
  }

  async invite(friendId: string): Promise<void> {
    const room = this.active;
    if (!room) return;
    try {
      await call('social_send_lobby_invite', { lobbyCode: room.code, targetUserId: friendId, auth: null });
      this.invited = [...this.invited, friendId];
    } catch (e) {
      this.error = message(e, t('rooms.invite_failed'));
    }
  }
}

export const rooms = new Rooms();
