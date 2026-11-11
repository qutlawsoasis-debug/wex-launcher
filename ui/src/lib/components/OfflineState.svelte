<!--
  Wex Launcher — Apple HIG Offline State Component
  Consistent empty/disconnected state for network-dependent screens (Servers, Lobby, Friends, Overview)
  Rules:
  - SF Pro Typography & Apple HIG spacing
  - System colors: var(--color-label), var(--color-secondary-label), var(--color-system-gray4)
  - Retry action triggering network.check() and optional onretry callback
-->
<script lang="ts">
  import { network } from '../design/network.svelte';
  import { t } from '../i18n';
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';

  interface Props {
    title?: string;
    subtitle?: string;
    onretry?: () => any;
    class?: string;
  }

  let {
    title,
    subtitle,
    onretry,
    class: className = '',
  }: Props = $props();

  let resolvedTitle = $derived(title ?? t('offline.title'));
  let resolvedSubtitle = $derived(subtitle ?? t('offline.subtitle'));

  let isRetrying = $state(false);

  async function handleRetry() {
    if (isRetrying) return;
    isRetrying = true;
    try {
      await network.check();
      if (onretry) {
        await onretry();
      }
    } finally {
      isRetrying = false;
    }
  }
</script>

<div class="offline-state {className}" role="status" aria-live="polite">
  <div class="offline-icon-bubble">
    <Icon name="wifi-off" size={32} color="var(--color-secondary-label)" />
  </div>

  <h3 class="offline-title">{resolvedTitle}</h3>
  <p class="offline-subtitle">{resolvedSubtitle}</p>

  <div class="offline-action">
    <Button
      variant="secondary"
      size="regular"
      onclick={handleRetry}
      loading={isRetrying || network.isChecking}
    >
      <Icon name="refresh" size={14} color="var(--color-label)" />
      <span>{t('offline.retry')}</span>
    </Button>
  </div>
</div>

<style>
  .offline-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
    min-height: 280px;
    width: 100%;
    animation: fadeIn var(--motion-duration-modal-enter) var(--motion-curve-spring-system);
  }

  .offline-icon-bubble {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background-color: var(--color-fill-quaternary, rgba(120, 120, 128, 0.12));
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
  }

  .offline-title {
    margin: 0 0 6px 0;
    font-size: 17px;
    font-weight: 600;
    color: var(--color-label);
    letter-spacing: -0.01em;
  }

  .offline-subtitle {
    margin: 0 0 20px 0;
    font-size: 13px;
    line-height: 1.4;
    color: var(--color-secondary-label);
    max-width: 320px;
  }

  .offline-action {
    display: flex;
    align-items: center;
    justify-content: center;
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
