/**
 * Wex Launcher — Network Connectivity State & Supervisor (Svelte 5)
 *
 * Rules:
 * - Single source of truth for online/offline across all screens
 * - Listens to backend Tauri 'network-status' event for auto-reconnect
 * - Listens to window 'online' / 'offline' events
 * - Provides retry / manual refresh check
 */

export interface NetworkStatusPayload {
  is_online: boolean;
}

class NetworkManager {
  isOnline = $state<boolean>(true);
  isChecking = $state<boolean>(false);
  private unlistenTauri: (() => void) | null = null;

  constructor() {
    if (typeof window !== 'undefined') {
      if (typeof navigator !== 'undefined' && 'onLine' in navigator) {
        this.isOnline = navigator.onLine;
      }

      window.addEventListener('online', () => {
        this.isOnline = true;
      });

      window.addEventListener('offline', () => {
        this.isOnline = false;
      });

      this.initTauriListener();
    }
  }

  private async initTauriListener() {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<NetworkStatusPayload>('network-status', (event) => {
        this.isOnline = event.payload.is_online;
      });
      this.unlistenTauri = unlisten;

      await this.check();
    } catch {
      // In tests or non-tauri environments, fall back gracefully
    }
  }

  async check(): Promise<boolean> {
    this.isChecking = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<NetworkStatusPayload>('get_network_status');
      this.isOnline = res.is_online;
      return res.is_online;
    } catch {
      if (typeof navigator !== 'undefined' && 'onLine' in navigator) {
        this.isOnline = navigator.onLine;
      }
      return this.isOnline;
    } finally {
      this.isChecking = false;
    }
  }

  setOnlineForTesting(online: boolean) {
    this.isOnline = online;
  }

  destroy() {
    if (this.unlistenTauri) {
      this.unlistenTauri();
      this.unlistenTauri = null;
    }
  }
}

export const network = new NetworkManager();
