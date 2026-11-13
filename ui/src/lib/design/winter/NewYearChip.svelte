<!--
  The New Year countdown on the build tile (winter look): «До Нового года 48 дней»,
  on the last day a clock to midnight, in the first week of January a greeting.
  Pressing it sets off a short firework show (Fireworks.svelte). A secondary
  glass pill (HIG › Buttons: Play stays the one prominent action); the label
  says what it is, the time is announced politely only on the last minute.
  Apple has no guidance on seasonal decoration.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { newYearState, pad2, type NewYearState } from './newyear';
  import { theme } from '../theme.svelte';
  import { t, tn } from '../../i18n';

  let state = $state<NewYearState>(newYearState(new Date()));
  onMount(() => {
    const id = setInterval(() => (state = newYearState(new Date())), 1000);
    return () => clearInterval(id);
  });

  const label = $derived(
    state.kind === 'days'
      ? tn('ny.days', state.days)
      : state.kind === 'clock'
        ? t('ny.clock', { time: `${pad2(state.h)}:${pad2(state.m)}:${pad2(state.s)}` })
        : state.kind === 'greeting'
          ? t('ny.greeting', { year: state.year })
          : ''
  );

  function celebrate() {
    window.dispatchEvent(new CustomEvent('wex-fireworks', { detail: { seconds: 6 } }));
  }
</script>

{#if state.kind !== 'none'}
  <button type="button" class="ny" class:final={state.kind === 'clock'} title={theme.motionReduced ? undefined : t('ny.fireworks')} onclick={celebrate}>
    <svg viewBox="0 0 16 16" aria-hidden="true"
      ><path d="M8 1.5 9.6 6l4.9.2-3.8 3 1.3 4.8L8 11.2 4 14l1.3-4.8-3.8-3L6.4 6z" /></svg
    >
    <span aria-live={state.kind === 'clock' && state.h === 0 && state.m === 0 ? 'polite' : 'off'}>{label}</span>
  </button>
{/if}

<style>
  .ny {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-pill);
    background: rgb(255 255 255 / 0.16);
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.22);
    color: #fff;
    font: inherit;
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    font-variant-numeric: tabular-nums;
    cursor: default;
    transition: background-color var(--dur-micro);
  }
  .ny:hover {
    background: rgb(255 255 255 / 0.26);
  }
  .ny svg {
    width: 13px;
    height: 13px;
    fill: #ffd60a;
  }
  .ny.final {
    background: rgb(255 214 10 / 0.22);
  }
  :global(html[data-reduce-transparency='true']) .ny {
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
    background: rgb(0 0 0 / 0.35);
  }
</style>
