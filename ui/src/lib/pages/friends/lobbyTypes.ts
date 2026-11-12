/** Room data returned by the lobby commands (core: LobbyRoomModel, OpenRoom). */

export interface LobbyPlayer {
  name: string;
  is_host: boolean;
  ping_ms: number;
  mods_synced: boolean;
}

export interface LobbyRoom {
  code: string;
  host_name: string;
  world_name: string;
  tunnel_address?: string | null;
  tunnel_active: boolean;
  tunnel_error?: string | null;
  is_host: boolean;
  players: LobbyPlayer[];
  mod_mismatch_count: number;
  total_mods: number;
}

export interface OpenRoom {
  id: string;
  name: string;
  host_name: string;
  game_version: string;
  player_count: number;
  max_players: number;
  address: string | null;
  status: string;
}
