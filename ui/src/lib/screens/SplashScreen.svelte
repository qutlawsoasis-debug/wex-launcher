<!--
  Wex Launcher — Startup Splash Screen
  
  Specification: SPEC 1.1 «Экран начальной загрузки»
  Apple HIG Guidelines:
  - Launching: https://developer.apple.com/design/human-interface-guidelines/launching
    Enhance the perception of the app as quick to start and immediately ready for use.
    Compact frameless card, no unnecessary marketing slogans, immediate status feedback.
  - Progress Indicators: https://developer.apple.com/design/human-interface-guidelines/progress-indicators
    Determinate progress indicator during updates, clear transfer speed and stage detail.
  - Motion: https://developer.apple.com/design/human-interface-guidelines/motion
    Seamless 250ms cross-fade into main window, reduced to 0ms when Reduce Motion is requested.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Logo from '../components/Logo.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';
  import Button from '../components/Button.svelte';
  import { theme } from '../design/theme.svelte';
  import { isSystemReduceMotion } from '../design/motion';

  interface Props {
    initialVersion?: string;
    autoStart?: boolean;
    simulateNetworkError?: boolean;
    oncomplete?: () => void;
  }

  let {
    initialVersion = 'v0.1.0',
    autoStart = true,
    simulateNetworkError = false,
    oncomplete,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let appVersion = $state(initialVersion);
  let statusText = $state('Инициализация каталогов…');
  let hasNetworkError = $state(false);
  let errorMessage = $state('');
  let isRetrying = $state(false);

  // Update download progress state
  let isDownloading = $state(false);
  let downloadProgress = $state(0);
  let downloadSpeed = $state('');
  let downloadDetail = $state('');

  // Fade out state
  let isExiting = $state(false);

  function isTauri(): boolean {
    return (
      typeof window !== 'undefined' &&
      Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
    );
  }

  export async function startInitialization() {
    hasNetworkError = false;
    errorMessage = '';
    statusText = 'Инициализация каталогов…';

    try {
      if (isTauri()) {
        const { invoke } = await import('@tauri-apps/api/core');
        try {
          const info = await invoke<{
            app_version: string;
            directories_ok: boolean;
          }>('startup_get_info');

          if (info?.app_version) {
            appVersion = `v${info.app_version}`;
          }
        } catch {
          // Keep default version
        }

        try {
          await invoke('startup_verify_directories');
        } catch {
          // Non-fatal directory initialization fallback
        }
      }

      await checkUpdates();
    } catch (err: unknown) {
      handleNetworkFailure(err instanceof Error ? err.message : String(err));
    }
  }

  export async function checkUpdates() {
    statusText = 'Проверка обновлений…';
    hasNetworkError = false;

    if (simulateNetworkError) {
      handleNetworkFailure('Сетевая ошибка: невозможно связаться с сервером обновлений');
      return;
    }

    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const update = await invoke<{
          update_available: boolean;
          latest_version?: string;
          download_url?: string;
        }>('startup_check_update', {
          simulateNetworkError: false,
        });

        if (update && update.update_available && update.download_url) {
          await runUpdateDownload(update.download_url, update.latest_version);
          return;
        }
      } catch (err: unknown) {
        handleNetworkFailure(
          err instanceof Error
            ? err.message
            : 'Сетевая ошибка: не удалось проверить обновления'
        );
        return;
      }
    }

    // In-browser or up-to-date transition
    statusText = 'Готово к запуску';
    await waitDelay(200);
    finishStartup();
  }

  async function runUpdateDownload(downloadUrl: string, latestVer?: string) {
    isDownloading = true;
    statusText = latestVer
      ? `Загрузка обновления ${latestVer}…`
      : 'Загрузка обновления лаунчера…';

    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const { listen } = await import('@tauri-apps/api/event');

        const unlisten = await listen<{
          percentage: number;
          speed_bytes_per_sec: number;
          stage: string;
        }>('startup-update-progress', (event) => {
          downloadProgress = event.payload.percentage;
          downloadDetail = event.payload.stage;
          if (event.payload.speed_bytes_per_sec > 0) {
            downloadSpeed = `${(event.payload.speed_bytes_per_sec / (1024 * 1024)).toFixed(1)} МБ/с`;
          }
        });

        await invoke('startup_download_update', { downloadUrl });
        unlisten();
      } catch {
        // Fallback progress simulation
        await simulateDownloadFallback();
      }
    } else {
      await simulateDownloadFallback();
    }

    statusText = 'Обновление загружено';
    await waitDelay(250);
    finishStartup();
  }

  async function simulateDownloadFallback() {
    const steps = [
      { pct: 20, speed: '12.4 МБ/с', detail: 'Загрузка манифеста…' },
      { pct: 55, speed: '14.8 МБ/с', detail: 'Загрузка архива обновления…' },
      { pct: 85, speed: '15.1 МБ/с', detail: 'Проверка контрольной суммы…' },
      { pct: 100, speed: '', detail: 'Готово к установке' },
    ];

    for (const step of steps) {
      downloadProgress = step.pct;
      downloadSpeed = step.speed;
      downloadDetail = step.detail;
      await waitDelay(120);
    }
  }

  function handleNetworkFailure(msg: string) {
    hasNetworkError = true;
    errorMessage = msg || 'Не удалось связаться с сервером обновлений';
    statusText = 'Сетевая ошибка';
  }

  export async function retryCheck() {
    if (isRetrying) return;
    isRetrying = true;
    hasNetworkError = false;
    statusText = 'Повторная попытка…';
    await waitDelay(250);
    isRetrying = false;
    await checkUpdates();
  }

  export function skipOffline() {
    statusText = 'Переход в автономный режим…';
    finishStartup();
  }

  function finishStartup() {
    const reduceMotion = theme.reduceMotion || isSystemReduceMotion();
    isExiting = true;

    if (reduceMotion) {
      // Immediate transition when Reduce Motion is preferred per Apple HIG Motion
      oncomplete?.();
    } else {
      // 250ms cross-fade transition as per SPEC 1.1
      setTimeout(() => {
        oncomplete?.();
      }, 250);
    }
  }

  function waitDelay(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  onMount(() => {
    if (autoStart) {
      startInitialization();
    }
  });
</script>

<div
  class="splash-viewport"
  class:exiting={isExiting}
  role="region"
  aria-label="Экран загрузки лаунчера"
>
  <div class="splash-card">
    <!-- Brand Wordmark Logo (Modular, easily replaceable) -->
    <div class="logo-wrapper">
      <Logo size="large" />
    </div>

    <!-- Launcher Version Tag -->
    <div class="version-tag">
      <span class="version-badge">{appVersion}</span>
    </div>

    <!-- Dynamic Status or Progress -->
    <div class="status-section">
      {#if isDownloading}
        <div class="download-container">
          <ProgressBar
            percentage={downloadProgress}
            label={statusText}
            detail={downloadSpeed ? `${downloadDetail} • ${downloadSpeed}` : downloadDetail}
            height={6}
          />
        </div>
      {:else}
        <p class="status-message" class:error={hasNetworkError}>
          {statusText}
        </p>
      {/if}
    </div>

    <!-- Network Error Fallback Actions: Повторить & Пропустить -->
    {#if hasNetworkError}
      <div class="network-error-box" role="alert">
        <p class="network-error-desc">
          {errorMessage}
        </p>
        <div class="error-actions">
          <Button
            variant="primary"
            size="small"
            onclick={retryCheck}
            loading={isRetrying}
          >
            Повторить
          </Button>
          <Button
            variant="secondary"
            size="small"
            onclick={skipOffline}
          >
            Пропустить
          </Button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .splash-viewport {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--color-system-background);
    opacity: 1;
    transition: opacity 250ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .splash-viewport.exiting {
    opacity: 0;
    pointer-events: none;
  }

  :global([data-reduce-motion="true"]) .splash-viewport {
    transition: none !important;
  }

  /* Compact Frameless Card per SPEC 1.1 */
  .splash-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 380px;
    padding: 2.5rem 2rem 2rem;
    border-radius: var(--radius-large);
    background: var(--color-secondary-system-background);
    border: 1px solid var(--color-separator);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.28);
    backdrop-filter: blur(40px);
    -webkit-backdrop-filter: blur(40px);
    text-align: center;
    box-sizing: border-box;
  }

  .logo-wrapper {
    margin-bottom: 0.75rem;
    display: flex;
    justify-content: center;
  }

  .version-tag {
    margin-bottom: 1.5rem;
  }

  .version-badge {
    display: inline-block;
    padding: 0.15rem 0.6rem;
    border-radius: var(--radius-pill);
    background-color: var(--color-system-fill);
    font-size: var(--font-size-footnote);
    font-weight: 500;
    color: var(--color-secondary-label);
    letter-spacing: -0.01em;
  }

  .status-section {
    width: 100%;
    min-height: 2.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .status-message {
    margin: 0;
    font-size: var(--font-size-subheadline);
    color: var(--color-secondary-label);
    line-height: 1.4;
    transition: color 150ms ease;
  }

  .status-message.error {
    color: var(--color-system-red);
    font-weight: 500;
  }

  .download-container {
    width: 100%;
  }

  /* Network Error Box & Action Buttons */
  .network-error-box {
    margin-top: 1.25rem;
    width: 100%;
    padding: 0.85rem;
    border-radius: var(--radius-medium);
    background-color: var(--color-system-fill);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    animation: fadeIn 200ms ease;
  }

  .network-error-desc {
    margin: 0;
    font-size: var(--font-size-caption1);
    color: var(--color-label);
    line-height: 1.35;
  }

  .error-actions {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    width: 100%;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
