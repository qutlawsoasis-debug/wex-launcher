<!--
  Apple HIG Toast / Transient HUD Component
  Specification: tasks/apple/hig/alerts.md (macOS HUD feedback)
  Rules:
  - Transient, non-modal status confirmation
  - Distinct status icon and semantic color
  - High contrast on Liquid Glass pill
  - Auto-dismiss with pause on hover
-->
<script lang="ts">
  import Icon from './icons/Icon.svelte';

  export interface ToastMessage {
    id: string;
    type: 'info' | 'success' | 'warning' | 'error';
    title: string;
    description?: string;
    duration?: number;
    action?: {
      label: string;
      onClick: () => void;
    };
  }

  interface Props {
    toast: ToastMessage;
    ondismiss?: (id: string) => void;
  }

  let { toast, ondismiss }: Props = $props();

  let timer: number | undefined;

  $effect(() => {
    const dur = toast.duration ?? 3500;
    if (dur > 0) {
      timer = window.setTimeout(() => {
        ondismiss?.(toast.id);
      }, dur);
    }

    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  function pauseTimer() {
    if (timer) clearTimeout(timer);
  }

  function resumeTimer() {
    const dur = toast.duration ?? 3500;
    if (dur > 0) {
      timer = window.setTimeout(() => {
        ondismiss?.(toast.id);
      }, dur / 2);
    }
  }

  const iconName = $derived(
    toast.type === 'success'
      ? 'check'
      : toast.type === 'warning'
        ? 'warning'
        : toast.type === 'error'
          ? 'error'
          : 'info'
  );

  const iconColor = $derived(
    toast.type === 'success'
      ? 'var(--color-system-green)'
      : toast.type === 'warning'
        ? 'var(--color-system-orange)'
        : toast.type === 'error'
          ? 'var(--color-system-red-accessible)'
          : 'var(--color-system-blue)'
  );
</script>

<div
  class="apple-toast {toast.type}"
  role="alert"
  onmouseenter={pauseTimer}
  onmouseleave={resumeTimer}
>
  <div class="toast-icon">
    <Icon name={iconName} size={18} color={iconColor} />
  </div>

  <div class="toast-text">
    <span class="toast-title">{toast.title}</span>
    {#if toast.description}
      <span class="toast-description">{toast.description}</span>
    {/if}
  </div>

  {#if toast.action}
    <button
      type="button"
      class="toast-action-btn"
      onclick={() => {
        toast.action?.onClick();
        ondismiss?.(toast.id);
      }}
    >
      {toast.action.label}
    </button>
  {/if}

  <button
    type="button"
    class="dismiss-btn"
    title="Закрыть"
    onclick={() => ondismiss?.(toast.id)}
    aria-label="Закрыть уведомление"
  >
    <Icon name="close" size={12} color="var(--color-tertiary-label)" />
  </button>
</div>

<style>
  .apple-toast {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    background-color: var(--glass-regular-bg);
    backdrop-filter: blur(28px) saturate(190%);
    border: 1px solid var(--glass-border);
    box-shadow: var(--glass-shadow);
    font-family: var(--font-system);
    user-select: none;
    min-width: 280px;
    max-width: 440px;
    animation: toastSlideDown var(--motion-duration-normal) var(--motion-spring-ease) forwards;
  }

  .toast-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .toast-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .toast-title {
    font-size: var(--text-subheadline);
    font-weight: 600;
    color: var(--color-label);
  }

  .toast-description {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    line-height: var(--line-footnote);
  }

  .toast-action-btn {
    appearance: none;
    background: var(--color-system-blue);
    color: #ffffff;
    border: none;
    border-radius: var(--radius-md);
    padding: 4px 10px;
    font-size: var(--text-footnote);
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
    white-space: nowrap;
    transition: opacity var(--motion-duration-fast) ease;
  }

  .toast-action-btn:hover {
    opacity: 0.88;
  }

  .toast-action-btn:active {
    opacity: 0.75;
  }

  .dismiss-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    outline: none;
    transition:
      background-color var(--motion-duration-short) ease,
      box-shadow var(--motion-duration-short) ease;
    flex-shrink: 0;
  }

  .dismiss-btn:hover {
    background-color: var(--color-quaternary-system-fill);
  }

  .dismiss-btn:active {
    background-color: var(--color-tertiary-system-fill);
  }

  .dismiss-btn:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 1px;
  }

  @keyframes toastSlideDown {
    from {
      opacity: 0;
      transform: translateY(-16px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .apple-toast {
      animation: none !important;
      transform: none !important;
    }
  }

  :global([data-reduce-motion="true"]) .apple-toast {
    animation: none !important;
    transform: none !important;
  }
</style>
