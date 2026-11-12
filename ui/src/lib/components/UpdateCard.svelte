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
  import { t } from '../i18n';

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
    updateProgressText = t('update_card.connecting');
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
        updateErrorMessage = res?.message || t('update_card.err_complete');
      }
    } catch (err: any) {
      updateStage = 'error';
      const msg = typeof err === 'string' ? err : err?.message || t('update_card.err_download');
      updateErrorMessage = msg.includes(t('play.update_prev_version_keyword'))
        ? msg
        : `${msg}. ${t('update_card.prev_version_preserved')}`;
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
    // Only the address the updater reported; no built-in fallback page.
    const url = updateInfo?.download_url;
    if (!url) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('updater_open_download_url', { url });
    } catch {
      // No window.open fallback: links must go through the validated backend opener.
      updateStage = 'error';
      updateErrorMessage = t('update_card.link_error');
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
          <Icon name="check" size={13} color="var(--green)" />
        {:else if updateStage === 'error'}
          <Icon name="error" size={13} color="var(--red)" />
        {:else}
          <Icon name="refresh" size={13} color="var(--blue)" />
        {/if}
      </div>
      <span class="update-banner-headline" class:update-banner-headline-error={updateStage === 'error'}>
        {#if updateStage === 'available'}
          {t('update_card.title_available', { version: updateInfo.latest_version || '' })}
        {:else if updateStage === 'downloading'}
          {t('update_card.title_downloading', { version: updateInfo.latest_version || '' })}
        {:else if updateStage === 'ready_to_restart'}
          {t('update_card.title_ready')}
        {:else}
          {t('update_card.title_error')}
        {/if}
      </span>
      {#if updateStage !== 'downloading'}
        <div class="update-banner-close">
          <button
            type="button"
            class="close-btn"
            onclick={() => (dismissed = true)}
            title={updateStage === 'ready_to_restart' ? t('update_card.postpone_restart') : t('update_card.close')}
            aria-label={updateStage === 'ready_to_restart' ? t('update_card.postpone_restart') : t('update_card.close')}
          >
            <Icon name="close" size={12} color="var(--label-2)" />
          </button>
        </div>
      {/if}
    </div>

    {#if updateStage === 'available'}
      {#if updateInfo.notes}
        <span class="update-banner-subtext" title={updateInfo.notes}>{updateInfo.notes}</span>
      {/if}
      {#if !canAutoUpdate}
        <span class="update-banner-subtext">{t('update_card.unsigned')}</span>
      {/if}
    {:else if updateStage === 'downloading'}
      <span class="update-banner-subtext">
        {updateProgressText || t('update_card.downloading_pct', { pct: updateProgressPct })}
      </span>
      <ProgressBar percentage={updateProgressPct} height={4} />
    {:else if updateStage === 'ready_to_restart'}
      <span class="update-banner-subtext">
        {t('update_card.ready_desc')}
      </span>
    {:else}
      <span class="update-banner-subtext" title={updateErrorMessage}>
        {updateErrorMessage || t('update_card.error_desc')}
      </span>
    {/if}

    {#if updateStage === 'available'}
      {#if canAutoUpdate}
        <Button variant="primary" size="small" fullWidth onclick={handleStartUpdate} title={t('update_card.update_title')}>
          <span>{t('update_card.update')}</span>
        </Button>
      {:else if updateInfo?.download_url}
        <Button variant="secondary" size="small" fullWidth onclick={handleOpenDownloadUrl} title={t('update_card.open_download_title')}>
          <span>{t('update_card.open_download')}</span>
        </Button>
      {/if}
    {:else if updateStage === 'ready_to_restart'}
      <Button variant="primary" size="small" fullWidth onclick={handleRestart} title={t('update_card.restart_title')}>
        <span>{t('update_card.restart')}</span>
      </Button>
    {:else if updateStage === 'error'}
      <Button variant="secondary" size="small" fullWidth onclick={handleStartUpdate} title={t('update_card.retry_title')}>
        <span>{t('update_card.retry')}</span>
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
    background: var(--fill-3);
    border: 1px solid var(--separator);
    border-radius: var(--r-m);
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
    font-size: var(--t-foot);
    font-weight: 600;
    line-height: 1.3;
    color: var(--label);
    overflow-wrap: anywhere;
  }

  .update-banner-headline-error {
    color: var(--red);
  }

  .update-banner-subtext {
    font-size: var(--t-cap);
    line-height: 1.35;
    color: var(--label-2);
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
    background: var(--fill-3);
  }
</style>
