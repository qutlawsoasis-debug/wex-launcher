<!--
  Apple HIG Deep Link Confirmation Modal
  Specification: tasks/inbox.md wex-deeplink & Apple HIG Alerts:
  https://developer.apple.com/design/human-interface-guidelines/alerts
  Rules:
  - Transparent dimmed backdrop scrim with backdrop blur
  - Dialog card centered with subtle shadow and rounded corners (16px)
  - Clear, concise Title and informative Subtitle explaining the action
  - Detail badge displaying the validated target (Lobby Code or Server Address)
  - Cancel button on the left, Primary Accent Confirm button on the right
  - Escape key dismisses without action, Enter key triggers confirmation
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';
  import type { DeepLinkActionDto } from '../ipc/generated/models';

  interface Props {
    open?: boolean;
    action?: DeepLinkActionDto | null;
    onconfirm?: (action: DeepLinkActionDto) => void;
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
        return 'Лобби P2P';
      case 'server':
        return 'Игровой сервер';
      case 'friend':
        return 'Друзья';
      default:
        return 'Ссылка протокола';
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
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) handleCancel();
    }}
  >
    <div
      class="deeplink-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="deeplink-title"
      aria-describedby="deeplink-desc"
      tabindex="-1"
      bind:this={dialogEl}
    >
      <div class="deeplink-icon-wrap">
        <div class="deeplink-icon-badge" class:server={action.kind === 'server'} class:join={action.kind === 'join'}>
          <Icon name={iconName} size={28} color="var(--color-accent)" />
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
              Код комнаты:
            {:else if action.kind === 'server'}
              Адрес подключения:
            {:else}
              Получатель:
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
          Отмена
        </Button>
        <Button variant="primary" onclick={handleConfirm}>
          {action.confirm_text || 'Подтвердить'}
        </Button>
      </div>
    </div>
  </div>
{/if}

<style>
  .deeplink-scrim {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: fadeIn 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .deeplink-dialog {
    width: 100%;
    max-width: 440px;
    background: var(--color-window-bg);
    border: 1px solid var(--color-separator);
    border-radius: 16px;
    padding: 24px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 16px;
    animation: popIn 0.22s cubic-bezier(0.16, 1, 0.3, 1);
    outline: none;
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
    background: var(--color-fill-secondary);
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
    background: var(--color-fill-tertiary);
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
    background: var(--color-fill-secondary);
    border: 1px solid var(--color-separator);
    border-radius: 10px;
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
    letter-spacing: 0.5px;
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

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes popIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
