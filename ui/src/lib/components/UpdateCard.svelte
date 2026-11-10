<!--
  Launcher update card, shown at the bottom of the sidebar on every screen.
  Apple HIG:
  - Buttons: one prominent (systemBlue) action per view — "Обновить" / "Перезапуск".
  - Progress indicators: determinate bar while the download size is known.
  - Apple does not define an in-sidebar update notice; placement is a project decision.
  Lifecycle: 'available' -> 'downloading' -> 'ready_to_restart' | 'error'.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from './Button.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import Icon from './icons/Icon.svelte';

  export interface UpdateInfo {
    update_available: boolean;
    current_version: string;
    latest_version?: string | null;
    notes?: string | null;
    download_url?: string | null;
    signature?: string | null;
    is_signed?: boolean;
    can_auto_update?: boolean;
    mode?: 'auto' | 'notify_only';
  }

  interface Props {
    /** Preloaded update info (tests). When null the card asks the updater itself. */
    initialInfo?: UpdateInfo | null;
  }

  let { initialInfo = null }: Props = $props();

  let fetchedInfo = $state<UpdateInfo | null>(null);
  let updateInfo = $derived(initialInfo !== null ? initialInfo : fetchedInfo);
  let dismissed = $state(false);
  let visible = $derived(Boolean(updateInfo?.update_available) && !dismissed);

  let updateStage = $state<'available' | 'downloading' | 'ready_to_restart' | 'error'>('available');
  let updateProgressPct = $state(0);
  let updateProgressText = $state('');
  let updateErrorMessage = $state('');

  let canAutoUpdate = $derived.by(() => {
    if (!updateInfo) return false;
    if (typeof updateInfo.can_auto_update === 'boolean') return updateInfo.can_auto_update;
    if (typeof updateInfo.is_signed === 'boolean') return updateInfo.is_signed;
    return Boolean(updateInfo.signature && updateInfo.signature.trim().length >= 32);
  });

  async function handleStartUpdate() {
    if (!updateInfo) return;
    updateStage = 'downloading';
    updateProgressPct = 0;
    updateProgressText = 'Подключение…';
    updateErrorMessage = '';

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{ status: string; message: string }>('updater_install', {
        downloadUrl: updateInfo.download_url || '',
        signature: updateInfo.signature || null,
        version: updateInfo.latest_version || 'latest',
      });
      if (res && res.status === 'ready_to_restart') {
        updateStage = 'ready_to_restart';
      } else {
        updateStage = 'error';
        updateErrorMessage = res?.message || 'Не удалось завершить обновление';
      }
    } catch (err: any) {
      updateStage = 'error';
      const msg = typeof err === 'string' ? err : err?.message || 'Ошибка загрузки обновления';
      updateErrorMessage = msg.includes('Прежняя версия') ? msg : `${msg}. Прежняя версия сохранена.`;
    }
  }

  async function handleRestart() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('updater_restart');
    } catch {
      if (typeof window !== 'undefined') window.location.reload();
    }
  }

  async function handleOpenDownloadUrl() {
    const url =
      updateInfo?.download_url || 'https://github.com/qutlawsoasis-debug/wex-launcher/releases';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('updater_open_download_url', { url });
    } catch {
      if (typeof window !== 'undefined') window.open(url, '_blank');
    }
  }

  onMount(() => {
    let unlistenProgress: (() => void) | undefined;
    let destroyed = false;

    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const un = await listen<any>('updater-progress', (event) => {
          if (event.payload) {
            updateProgressPct = event.payload.percentage ?? 0;
            updateProgressText = event.payload.status_text ?? '';
          }
        });
        if (destroyed) un();
        else unlistenProgress = un;
      } catch {}

      if (initialInfo === null) {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const res = await invoke<UpdateInfo>('updater_check').catch(() =>
            invoke<UpdateInfo>('overview_check_update')
          );
          if (!destroyed && res && res.update_available) fetchedInfo = res;
        } catch {}
      }
    })();

    return () => {
      destroyed = true;
      if (unlistenProgress) unlistenProgress();
    };
  });
</script>

{#if visible && updateInfo}
  <div class="update-banner" role="status" aria-live="polite">
    <div class="update-banner-top">
      <div class="update-banner-icon {updateStage}">
        {#if updateStage === 'ready_to_restart'}
          <Icon name="check" size={13} color="var(--color-system-green)" />
        {:else if updateStage === 'error'}
          <Icon name="error" size={13} color="var(--color-system-red)" />
        {:else}
          <Icon name="refresh" size={13} color="var(--color-system-blue)" />
        {/if}
      </div>
      <span class="update-banner-headline" class:update-banner-headline-error={updateStage === 'error'}>
        {#if updateStage === 'available'}
          Доступна новая версия Wex {updateInfo.latest_version || ''}
        {:else if updateStage === 'downloading'}
          Загрузка обновления Wex {updateInfo.latest_version || ''}
        {:else if updateStage === 'ready_to_restart'}
          Обновление готово к установке
        {:else}
          Не удалось обновить лаунчер
        {/if}
      </span>
      {#if updateStage !== 'downloading'}
        <div class="update-banner-close">
          <button
            type="button"
            class="close-btn"
            onclick={() => (dismissed = true)}
            title={updateStage === 'ready_to_restart' ? 'Отложить перезапуск' : 'Закрыть уведомление'}
            aria-label={updateStage === 'ready_to_restart' ? 'Отложить перезапуск' : 'Закрыть уведомление'}
          >
            <Icon name="close" size={12} color="var(--color-secondary-label)" />
          </button>
        </div>
      {/if}
    </div>

    {#if updateStage === 'available'}
      {#if updateInfo.notes}
        <span class="update-banner-subtext" title={updateInfo.notes}>{updateInfo.notes}</span>
      {/if}
      {#if !canAutoUpdate}
        <span class="update-banner-subtext">Неподписанная сборка — только ручная загрузка.</span>
      {/if}
    {:else if updateStage === 'downloading'}
      <span class="update-banner-subtext">
        {updateProgressText || `Скачиваем обновление… ${updateProgressPct}%`}
      </span>
      <ProgressBar percentage={updateProgressPct} height={4} />
    {:else if updateStage === 'ready_to_restart'}
      <span class="update-banner-subtext">
        Лаунчер закроется, обновится и откроется сам.
      </span>
    {:else}
      <span class="update-banner-subtext" title={updateErrorMessage}>
        {updateErrorMessage || 'Ошибка при загрузке. Прежняя версия сохранена и готова к работе.'}
      </span>
    {/if}

    {#if updateStage === 'available'}
      {#if canAutoUpdate}
        <Button variant="primary" size="small" fullWidth onclick={handleStartUpdate} title="Скачать и установить обновление">
          <span>Обновить</span>
        </Button>
      {:else}
        <Button variant="secondary" size="small" fullWidth onclick={handleOpenDownloadUrl} title="Перейти к загрузке установщика">
          <span>Перейти к загрузке</span>
        </Button>
      {/if}
    {:else if updateStage === 'ready_to_restart'}
      <Button variant="primary" size="small" fullWidth onclick={handleRestart} title="Перезапустить лаунчер сейчас">
        <span>Перезапуск</span>
      </Button>
    {:else if updateStage === 'error'}
      <Button variant="secondary" size="small" fullWidth onclick={handleStartUpdate} title="Повторить попытку обновления">
        <span>Повторить</span>
      </Button>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--color-tertiary-system-fill);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md);
    min-width: 0;
  }

  .update-banner-top {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
  }

  .update-banner-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: rgba(0, 145, 255, 0.14);
    flex-shrink: 0;
  }

  .update-banner-icon.ready_to_restart {
    background: rgba(48, 209, 88, 0.16);
  }

  .update-banner-icon.error {
    background: rgba(255, 69, 58, 0.16);
  }

  .update-banner-headline {
    flex: 1;
    min-width: 0;
    font-size: var(--text-footnote);
    font-weight: 600;
    line-height: 1.3;
    color: var(--color-label);
    overflow-wrap: anywhere;
  }

  .update-banner-headline-error {
    color: var(--color-system-red);
  }

  .update-banner-subtext {
    font-size: var(--text-caption-1);
    line-height: 1.35;
    color: var(--color-secondary-label);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .update-banner-close {
    flex-shrink: 0;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    margin: -3px -4px 0 0;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
  }

  .close-btn:hover {
    background: var(--color-quaternary-system-fill);
  }
</style>
