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
  import Button from '../components/Button.svelte';
  import { theme } from '../design/theme.svelte';
  import { isSystemReduceMotion } from '../design/motion';
  import { t } from '../i18n';

  interface Props {
    initialVersion?: string;
    autoStart?: boolean;
    oncomplete?: () => void;
  }

  let {
    initialVersion = '',
    autoStart = true,
    oncomplete,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let appVersion = $state(initialVersion);
  let statusText = $state(t('splash.status_init_dirs'));
  let hasNetworkError = $state(false);
  let errorMessage = $state('');
  let isRetrying = $state(false);

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
    statusText = t('splash.status_init_dirs');

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
    // No update check runs here (the updater banner does it later), so the
    // splash does not claim one.
    hasNetworkError = false;

    // Launcher updates are checked by the real updater (`updater_check`) and
    // offered as a banner on the Play screen; the splash never blocks on them.

    // In-browser or up-to-date transition
    statusText = t('splash.status_ready');
    await waitDelay(200);
    finishStartup();
  }

  function handleNetworkFailure(msg: string) {
    hasNetworkError = true;
    errorMessage = msg || t('splash.err_unreachable');
    statusText = t('splash.network_err');
  }

  export async function retryCheck() {
    if (isRetrying) return;
    isRetrying = true;
    hasNetworkError = false;
    statusText = t('splash.retrying');
    await waitDelay(250);
    isRetrying = false;
    await checkUpdates();
  }

  export function skipOffline() {
    statusText = t('splash.entering_offline');
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
  aria-label={t('splash.aria_splash')}
>
  <div class="splash-card">
    <!-- Brand Wordmark Logo (Modular, easily replaceable) -->
    <div class="logo-wrapper">
      <Logo size="large" />
    </div>

    <!-- Launcher Version Tag -->
    <div class="version-tag">
      {#if appVersion}<span class="version-badge">{appVersion}</span>{/if}
    </div>

    <!-- Dynamic Status or Progress -->
    <div class="status-section">
      <p class="status-message" class:error={hasNetworkError}>
        {statusText}
      </p>
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
            {t('splash.btn_retry')}
          </Button>
          <Button
            variant="secondary"
            size="small"
            onclick={skipOffline}
          >
            {t('splash.btn_skip')}
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
    background-color: var(--base);
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
    border-radius: var(--r-l);
    background: var(--card);
    border: 1px solid var(--separator);
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
    border-radius: var(--r-pill);
    background-color: var(--fill);
    font-size: var(--t-foot);
    font-weight: 500;
    color: var(--label-2);
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
    font-size: var(--t-sub);
    color: var(--label-2);
    line-height: 1.4;
    transition: color 150ms ease;
  }

  .status-message.error {
    color: var(--red);
    font-weight: 500;
  }

  /* Network Error Box & Action Buttons */
  .network-error-box {
    margin-top: 1.25rem;
    width: 100%;
    padding: 0.85rem;
    border-radius: var(--r-m);
    background-color: var(--fill);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    animation: fadeIn 200ms ease;
  }

  .network-error-desc {
    margin: 0;
    font-size: var(--t-cap);
    color: var(--label);
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
