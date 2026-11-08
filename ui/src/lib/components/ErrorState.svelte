<!--
  Wex Launcher — Apple HIG Error State Component
  Specification:
  - Apple HIG: Alerts (https://developer.apple.com/design/human-interface-guidelines/alerts)
    Short clear title stating what happened + subtitle stating what to do + primary action button.
  - Apple HIG: Writing (https://developer.apple.com/design/human-interface-guidelines/writing)
    Plain Russian, no jargon or raw English stack traces in primary text.
  - Collapsible «Подробнее» disclosure container with «Копировать» button for logs/support.
-->
<script lang="ts">
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';
  import {
    formatError,
    type ErrorDomain,
    type FormattedError,
    type ErrorActionType,
  } from '../design/errors';

  interface Props {
    error?: unknown;
    domain?: ErrorDomain;
    title?: string;
    description?: string;
    technical?: string | null;
    actionText?: string;
    actionType?: ErrorActionType;
    onaction?: () => any;
    secondaryActionText?: string;
    onsecondaryaction?: () => any;
    onclose?: () => any;
    compact?: boolean;
    class?: string;
  }

  let {
    error,
    domain = 'general',
    title: propTitle,
    description: propDescription,
    technical: propTechnical,
    actionText: propActionText,
    actionType: propActionType,
    onaction,
    secondaryActionText: propSecondaryActionText,
    onsecondaryaction,
    onclose,
    compact = false,
    class: className = '',
  }: Props = $props();

  // Derived formatted error
  let formatted = $derived<FormattedError>(formatError(error, domain));

  // Resolved titles and descriptions
  let displayTitle = $derived(propTitle || formatted.title);
  let displayDescription = $derived(propDescription || formatted.description);
  let displayTechnical = $derived(
    propTechnical !== undefined ? propTechnical : formatted.technical
  );

  // Resolved actions
  let resolvedPrimaryLabel = $derived(
    propActionText || formatted.primaryAction?.label || 'Повторить'
  );
  let resolvedPrimaryType = $derived(
    propActionType || formatted.primaryAction?.type || 'retry'
  );
  let resolvedSecondaryLabel = $derived(
    propSecondaryActionText || formatted.secondaryAction?.label
  );

  // Collapsible technical details state
  let isDetailsOpen = $state(false);
  let isCopied = $state(false);
  let copyTimeoutId = $state<number | null>(null);

  // Resolve action icon
  type IconName =
    | 'refresh'
    | 'folder'
    | 'settings'
    | 'error'
    | 'check'
    | 'copy'
    | 'close';

  let primaryActionIcon = $derived.by<IconName | undefined>(() => {
    switch (resolvedPrimaryType) {
      case 'retry':
        return 'refresh';
      case 'folder':
        return 'folder';
      case 'settings':
        return 'settings';
      default:
        return undefined;
    }
  });

  async function handleCopyTechnical() {
    if (!displayTechnical) return;
    try {
      await navigator.clipboard.writeText(displayTechnical);
      isCopied = true;
      if (copyTimeoutId) clearTimeout(copyTimeoutId);
      copyTimeoutId = window.setTimeout(() => {
        isCopied = false;
      }, 2000);
    } catch {
      // Clipboard fallback
    }
  }

  function handlePrimaryAction() {
    if (onaction) {
      onaction();
    }
  }

  function handleSecondaryAction() {
    if (onsecondaryaction) {
      onsecondaryaction();
    }
  }
</script>

<div
  class="error-state-card {compact ? 'compact' : ''} {className}"
  role="alert"
  aria-live="assertive"
>
  {#if onclose}
    <div class="dismiss-button-row">
      <button
        type="button"
        class="dismiss-btn"
        onclick={onclose}
        aria-label="Закрыть уведомление"
      >
        <Icon name="close" size={14} color="var(--color-secondary-label)" />
      </button>
    </div>
  {/if}

  <div class="error-header-row">
    <div class="error-icon-bubble" aria-hidden="true">
      <Icon name="error" size={compact ? 24 : 32} color="var(--color-system-red, #ff453a)" />
    </div>

    <div class="error-text-block">
      <h3 class="error-title">{displayTitle}</h3>
      <p class="error-description">{displayDescription}</p>
    </div>
  </div>

  <!-- Action Buttons Row -->
  <div class="error-actions-row">
    <Button
      variant="primary"
      size={compact ? 'small' : 'regular'}
      onclick={handlePrimaryAction}
    >
      {#if primaryActionIcon}
        <Icon name={primaryActionIcon} size={14} color="#ffffff" />
      {/if}
      <span>{resolvedPrimaryLabel}</span>
    </Button>

    {#if resolvedSecondaryLabel && onsecondaryaction}
      <Button
        variant="secondary"
        size={compact ? 'small' : 'regular'}
        onclick={handleSecondaryAction}
      >
        <span>{resolvedSecondaryLabel}</span>
      </Button>
    {/if}
  </div>

  <!-- Collapsible Technical Details (Apple HIG: Secondary & Non-intrusive) -->
  {#if displayTechnical}
    <div class="error-disclosure-section">
      <button
        type="button"
        class="disclosure-toggle-btn"
        onclick={() => (isDetailsOpen = !isDetailsOpen)}
        aria-expanded={isDetailsOpen}
      >
        <div class="disclosure-chevron" class:open={isDetailsOpen}>
          <Icon name="chevron-right" size={12} color="var(--color-secondary-label)" />
        </div>
        <span class="disclosure-label">
          {isDetailsOpen ? 'Скрыть подробности' : 'Подробнее'}
        </span>
      </button>

      {#if isDetailsOpen}
        <div class="technical-details-box">
          <div class="details-top-bar">
            <span class="details-code-badge">{formatted.code}</span>
            <button
              type="button"
              class="copy-btn"
              onclick={handleCopyTechnical}
              title="Скопировать сведения об ошибке"
            >
              <Icon
                name={isCopied ? 'check' : 'copy'}
                size={12}
                color={isCopied ? 'var(--color-system-green, #30d158)' : 'var(--color-secondary-label)'}
              />
              <span class="copy-label">{isCopied ? 'Скопировано' : 'Копировать'}</span>
            </button>
          </div>

          <pre class="technical-pre"><code>{displayTechnical}</code></pre>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .error-state-card {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 24px;
    background-color: var(--color-bg-secondary, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.1));
    border-radius: 14px;
    text-align: left;
    width: 100%;
    box-sizing: border-box;
    animation: fadeIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .error-state-card.compact {
    padding: 16px;
    border-radius: 10px;
  }

  .dismiss-button-row {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 2;
  }

  .dismiss-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-secondary-label);
    transition: background-color 0.15s ease;
  }

  .dismiss-btn:hover {
    background-color: var(--color-fill-quaternary, rgba(120, 120, 128, 0.12));
  }

  .error-header-row {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 16px;
  }

  .error-icon-bubble {
    width: 48px;
    height: 48px;
    min-width: 48px;
    border-radius: 50%;
    background-color: rgba(255, 69, 58, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .compact .error-icon-bubble {
    width: 36px;
    height: 36px;
    min-width: 36px;
  }

  .error-text-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .error-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--color-label);
    letter-spacing: -0.01em;
    line-height: 1.3;
  }

  .compact .error-title {
    font-size: 14px;
  }

  .error-description {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--color-secondary-label);
  }

  .compact .error-description {
    font-size: 12px;
  }

  .error-actions-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    margin-bottom: 4px;
  }

  .error-disclosure-section {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
  }

  .disclosure-toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-secondary-label);
    font-family: inherit;
    transition: color 0.15s ease;
  }

  .disclosure-toggle-btn:hover {
    color: var(--color-label);
  }

  .disclosure-chevron {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .disclosure-chevron.open {
    transform: rotate(90deg);
  }

  .technical-details-box {
    margin-top: 10px;
    background-color: var(--color-fill-quaternary, rgba(120, 120, 128, 0.1));
    border-radius: 8px;
    padding: 10px 12px;
    box-sizing: border-box;
  }

  .details-top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .details-code-badge {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background-color: var(--color-fill-tertiary, rgba(120, 120, 128, 0.18));
    color: var(--color-secondary-label);
    font-weight: 500;
  }

  .copy-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
    color: var(--color-secondary-label);
    font-size: 11px;
    font-family: inherit;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .copy-btn:hover {
    background-color: var(--color-fill-tertiary, rgba(120, 120, 128, 0.15));
    color: var(--color-label);
  }

  .copy-label {
    font-size: 11px;
  }

  .technical-pre {
    margin: 0;
    max-height: 140px;
    overflow-y: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-label);
    white-space: pre-wrap;
    word-break: break-all;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
