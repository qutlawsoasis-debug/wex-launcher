<!--
  The single notice above the launch bar (reference: .toast). role="status" so
  screen readers announce it without moving focus (HIG › Accessibility).
-->
<script lang="ts">
  import { toasts } from './toast.svelte';
</script>

<div class="toast" class:show={!!toasts.current} class:multi={!!toasts.current?.detail} role="status" aria-live="polite">
  {#if toasts.current}
    {#key toasts.current.id}
      <div class="body">
        <b>{toasts.current.title}</b>
        {#if toasts.current.detail}<span>{toasts.current.detail}</span>{/if}
      </div>
      {#if toasts.current.action}
        {@const action = toasts.current.action}
        <button type="button" onclick={() => { action.run(); toasts.dismiss(); }}>{action.label}</button>
      {/if}
    {/key}
  {/if}
</div>

<style>
  .toast {
    position: absolute;
    left: 50%;
    bottom: 96px;
    z-index: 9;
    display: flex;
    align-items: center;
    gap: 16px;
    transform: translateX(-50%);
    padding: 12px 16px;
    border-radius: var(--r-m);
    background: var(--veil-strong);
    box-shadow:
      inset 0 0 0 1px var(--edge),
      0 12px 30px var(--shade);
    font-size: var(--t-sub);
    max-width: min(560px, calc(100% - 32px));
    transition:
      opacity var(--dur-short),
      transform var(--dur-ind) var(--spring);
  }
  .toast:not(.show) {
    opacity: 0;
    transform: translate(-50%, 8px);
    pointer-events: none;
  }
  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  b {
    font-weight: var(--w-medium);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .multi b {
    font-weight: var(--w-semibold);
  }
  span {
    color: var(--label-2);
    font-size: var(--t-foot);
  }
  button {
    flex: none;
    height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-s);
    background: var(--fill-2);
    color: var(--label);
    font: inherit;
    font-weight: var(--w-semibold);
    cursor: pointer;
  }
  button:hover {
    background: var(--fill);
  }
</style>
