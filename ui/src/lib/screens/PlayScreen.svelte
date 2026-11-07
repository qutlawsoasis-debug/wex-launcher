<!--
  Wex Launcher — Screen «Играть»
  Supports 3 design mockups per task directive 08.11 00:05 — wex-design:
  - Mockup 1: «Иммерсивный горизонт» (Apple Arcade / Full-Bleed Atmosphere)
  - Mockup 2: «Редакционный обзор» (macOS App Store / Split Editorial)
  - Mockup 3: «Сфокусированный лаунчер» (Apple Music / Media Player Minimalist)
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Mockup1 from './mockups/Mockup1.svelte';
  import Mockup2 from './mockups/Mockup2.svelte';
  import Mockup3 from './mockups/Mockup3.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';

  interface Props {
    selectedVersion?: string;
    playerName?: string;
    clientStatus?: string;
    isLaunching?: boolean;
    progressPercentage?: number;
    progressStage?: string;
    progressDetail?: string;
    progressSpeed?: string;
    onlaunch?: () => void;
    oncancel?: () => void;
  }

  let {
    selectedVersion = $bindable('1.21.1'),
    playerName = $bindable('Player'),
    clientStatus = '',
    isLaunching = false,
    progressPercentage = 0,
    progressStage = 'Готов к запуску',
    progressDetail = 'Нажмите «Играть» для старта',
    progressSpeed = '',
    onlaunch,
    oncancel,
  }: Props = $props();

  let activeMockup = $state('1');

  const mockupOptions = [
    { value: '1', label: '1: Иммерсивный' },
    { value: '2', label: '2: Редакционный' },
    { value: '3', label: '3: Минималистичный' },
  ];

  onMount(() => {
    function checkHashMockup() {
      const hash = window.location.hash.replace(/^#/, '');
      const params = new URLSearchParams(hash);
      const m = params.get('mockup');
      if (m && ['1', '2', '3'].includes(m)) {
        activeMockup = m;
      }
    }

    checkHashMockup();
    window.addEventListener('hashchange', checkHashMockup);
    return () => window.removeEventListener('hashchange', checkHashMockup);
  });
</script>

<div class="play-screen-wrapper">
  <!-- Mockup Switcher Pill for User Review -->
  <div class="mockup-selector-floating" data-testid="mockup-selector">
    <span class="selector-hint">Вариант:</span>
    <SegmentedControl
      options={mockupOptions}
      bind:value={activeMockup}
    />
  </div>

  <!-- Active Mockup Body -->
  <div class="mockup-viewport">
    {#if activeMockup === '1'}
      <Mockup1
        bind:selectedVersion
        bind:playerName
        {clientStatus}
        {isLaunching}
        {progressPercentage}
        {progressStage}
        {progressDetail}
        {progressSpeed}
        {onlaunch}
        {oncancel}
      />
    {:else if activeMockup === '2'}
      <Mockup2
        bind:selectedVersion
        bind:playerName
        {clientStatus}
        {isLaunching}
        {progressPercentage}
        {progressStage}
        {progressDetail}
        {progressSpeed}
        {onlaunch}
        {oncancel}
      />
    {:else}
      <Mockup3
        bind:selectedVersion
        bind:playerName
        {clientStatus}
        {isLaunching}
        {progressPercentage}
        {progressStage}
        {progressDetail}
        {progressSpeed}
        {onlaunch}
        {oncancel}
      />
    {/if}
  </div>
</div>

<style>
  .play-screen-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .mockup-selector-floating {
    position: absolute;
    top: 14px;
    right: 24px;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-system-ultra-thin-material);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-xl);
    padding: 4px 10px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }

  :global([data-theme='light']) .mockup-selector-floating {
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
  }

  .selector-hint {
    font-size: var(--text-caption-2);
    font-weight: 600;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .mockup-viewport {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
