<!--
  Apple HIG Deep Link Confirmation Modal (Окно подтверждения wex://)
  Specification: tasks/inbox.md wex-motion-details & Apple HIG Alerts:
  https://developer.apple.com/design/human-interface-guidelines/alerts
  Rules:
  - Transparent dimmed backdrop scrim without live blur (anti-slop single live blur rule)
  - Dialog card centered with Apple spring entrance (scale 0.92 -> 1, translateY 12px -> 0)
  - Clear, concise Title and informative Subtitle explaining the incoming deep link action
  - Detail badge displaying the validated target (Lobby Code, Server Address, or Friend Code)
  - Cancel button on the left, Primary Accent Confirm button on the right
  - Escape key dismisses without action, Enter key triggers confirmation
  - Strict Reduce Motion support: instant opacity entrance without scale/transform
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';
  import { theme } from '../design/theme.svelte';
  import { t } from '../i18n';

  export interface DeepLinkAction {
    kind: 'join' | 'server' | 'friend' | 'custom';
    title: string;
    subtitle: string;
    target: string;
    raw_uri?: string;
    confirm_text?: string;
  }

  interface Props {
    open?: boolean;
    action?: DeepLinkAction | null;
    onconfirm?: (action: DeepLinkAction) => void;
    oncancel?: () => void;
  }

  let {
    open = $bindable(false),
    action = null,
    onconfirm,
    oncancel,
  }: Props = $props();

  let dialogEl: HTMLElement | undefined = $state();

  function handleCancel() {
    open = false;
    oncancel?.();
  }

  function handleConfirm() {
    if (!action) return;
    const act = action;
    open = false;
    onconfirm?.(act);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    }
  }

  $effect(() => {
    if (open && typeof window !== 'undefined') {
      dialogEl?.focus();
    }
  });

  const kindLabel = $derived.by(() => {
    switch (action?.kind) {
      case 'join':
        return t('deeplink.badge_lobby');
      case 'server':
        return t('deeplink.badge_server');
      case 'friend':
        return t('deeplink.badge_friend');
      default:
        return t('deeplink.badge_generic');
    }
  });

  const iconName = $derived.by(() => {
    switch (action?.kind) {
      case 'join':
        return 'lobby';
      case 'server':
        return 'server';
      case 'friend':
        return 'friends';
      default:
        return 'info';
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open && action}
  <div
    class="deeplink-scrim"
    class:reduce-motion={theme.reduceMotion}
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) handleCancel();
    }}
    data-testid="deeplink-modal"
  >
    <div
      class="deeplink-dialog"
      class:reduce-motion={theme.reduceMotion}
      role="dialog"
      aria-modal="true"
      aria-labelledby="deeplink-title"
      aria-describedby="deeplink-desc"
      tabindex="-1"
      bind:this={dialogEl}
    >
      <div class="deeplink-icon-wrap">
        <div class="deeplink-icon-badge" class:server={action.kind === 'server'} class:join={action.kind === 'join'}>
          <Icon name={iconName} size={28} color="var(--color-system-blue)" />
        </div>
      </div>

      <div class="deeplink-header">
        <div class="deeplink-kind-badge">{kindLabel}</div>
        <h2 id="deeplink-title" class="deeplink-title">{action.title}</h2>
        <p id="deeplink-desc" class="deeplink-desc">{action.subtitle}</p>
      </div>

      <div class="deeplink-card">
        <div class="deeplink-target-row">
          <span class="target-label">
            {#if action.kind === 'join'}
              {t('deeplink.lbl_room_code')}
            {:else if action.kind === 'server'}
              {t('deeplink.lbl_address')}
            {:else}
              {t('deeplink.lbl_player')}
            {/if}
          </span>
          <span class="target-value">{action.target}</span>
        </div>
        {#if action.raw_uri}
          <div class="raw-uri-row" title={action.raw_uri}>
            {action.raw_uri}
          </div>
        {/if}
      </div>

      <div class="deeplink-actions">
        <Button variant="secondary" onclick={handleCancel}>
          {t('deeplink.btn_cancel')}
        </Button>
        <Button variant="primary" onclick={handleConfirm}>
          {action.confirm_text || t('deeplink.btn_confirm_default')}
        </Button>
      </div>
    </div>
  </div>
{/if}

<style>
  @keyframes scrimFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes modalSpringIn {
    0% {
      opacity: 0;
      transform: scale(0.92) translateY(12px);
    }
    100% {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .deeplink-scrim {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: scrimFadeIn var(--motion-duration-fast, 180ms) cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .deeplink-dialog {
    width: 100%;
    max-width: 440px;
    background: var(--card-bg, rgba(30, 30, 32, 0.85));
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl, 16px);
    padding: 24px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 16px;
    animation: modalSpringIn var(--motion-duration-normal, 240ms) var(--motion-curve-spring-gentle, cubic-bezier(0.16, 1, 0.3, 1)) both;
    outline: none;
    box-sizing: border-box;
  }

  .deeplink-icon-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .deeplink-icon-badge {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    background: var(--color-quaternary-system-fill, rgba(120, 120, 128, 0.12));
    border: 1px solid var(--color-separator);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .deeplink-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .deeplink-kind-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-secondary-label);
    background: var(--color-quaternary-system-fill, rgba(120, 120, 128, 0.12));
    border: 1px solid var(--color-separator);
    padding: 2px 8px;
    border-radius: 6px;
  }

  .deeplink-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--color-label);
    margin: 0;
    line-height: 1.25;
  }

  .deeplink-desc {
    font-size: 13px;
    color: var(--color-secondary-label);
    margin: 0;
    line-height: 1.4;
    max-width: 380px;
  }

  .deeplink-card {
    width: 100%;
    background: var(--color-quaternary-system-fill, rgba(120, 120, 128, 0.08));
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md, 10px);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-sizing: border-box;
  }

  .deeplink-target-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }

  .target-label {
    color: var(--color-secondary-label);
  }

  .target-value {
    font-family: var(--font-family-mono, monospace);
    font-weight: 700;
    color: var(--color-label);
  }

  .raw-uri-row {
    font-size: 11px;
    color: var(--color-tertiary-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .deeplink-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    width: 100%;
    padding-top: 4px;
  }

  /* Reduce Motion Parity */
  @media (prefers-reduced-motion: reduce) {
    .deeplink-scrim,
    .deeplink-dialog {
      animation: none !important;
      transform: none !important;
    }
  }

  :global([data-reduce-motion='true']) .deeplink-scrim,
  :global([data-reduce-motion='true']) .deeplink-dialog {
    animation: none !important;
    transform: none !important;
  }
</style>
