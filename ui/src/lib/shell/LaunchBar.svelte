<!--
  Launch bar (reference: .bar), always at the bottom.
  Ready → stage of the launch with progress (launch-status / install-progress)
  → game running with a timer → stopped. One button changes role:
  Play (systemBlue, the single accent) → Cancel → Stop (systemRed text).
  HIG › Buttons: one prominent action; HIG › Progress indicators: determinate
  bar when the amount is known; HIG › Materials for the bar background.
-->
<script lang="ts">
  import { launch, readiness } from './launch.svelte';
  import { builds } from './builds.svelte';
  import { t, i18n } from '../i18n';
  import BuildCover from './BuildCover.svelte';

  let now = $state(Date.now());
  let barEl: HTMLElement | undefined = $state();
  let playEl: HTMLButtonElement | undefined = $state();

  const shownId = $derived(launch.phase === 'idle' ? builds.selectedId : (launch.version ?? builds.selectedId));
  const build = $derived(builds.list.find((b) => b.id === shownId));

  $effect(() => {
    if (launch.phase !== 'running') return;
    now = Date.now();
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  function size(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    const fmt = (n: number, digits: number) =>
      new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: digits, minimumFractionDigits: digits }).format(n);
    return mb >= 1024 ? t('shell.gb', { n: fmt(mb / 1024, 1) }) : t('shell.mb', { n: fmt(mb, mb < 10 ? 1 : 0) });
  }

  function clock(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(s / 3600);
    const mm = String(Math.floor((s % 3600) / 60)).padStart(2, '0');
    const ss = String(s % 60).padStart(2, '0');
    return h ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }

  const sub = $derived.by(() => {
    if (launch.phase === 'launching') {
      const parts: string[] = [];
      // Sizes only when core reports real bytes; never "0,0 МБ из 0,0 МБ".
      if (launch.total >= 1024 * 1024 && launch.downloaded <= launch.total) {
        parts.push(t('shell.of', { done: size(launch.downloaded), total: size(launch.total) }));
      }
      if (launch.percent !== null) parts.push(t('shell.percent', { pct: Math.round(launch.percent) }));
      return parts.join(', ');
    }
    if (launch.phase === 'running') {
      const parts = [t('shell.build_name', { ver: shownId })];
      if (launch.startedAt) parts.push(clock(now - launch.startedAt));
      return parts.join(', ');
    }
    // Idle: always say whether Play can start and what it will do.
    if (!builds.loaded) return t('shell.builds_loading');
    if (!build) return t('shell.builds_unavailable');
    const parts: string[] = [launch.prewarm === 'prewarming' ? t('shell.prewarming') : readiness(build).text];
    if (builds.ramMb) parts.push(t('shell.memory_gb', { gb: Math.round(builds.ramMb / 1024) }));
    if (build.java !== null) parts.push(t('shell.java', { java: build.java }));
    return parts.join(', ');
  });

  /** Restart a CSS animation class on an element. */
  function replay(el: HTMLElement | undefined, cls: string) {
    if (!el) return;
    el.classList.remove(cls);
    void el.offsetWidth;
    el.classList.add(cls);
  }

  let lastPhase = launch.phase;
  $effect(() => {
    const phase = launch.phase;
    if (phase !== lastPhase) {
      lastPhase = phase;
      replay(playEl, 'swap');
    }
  });

  let lastStarts = launch.startCount;
  $effect(() => {
    const n = launch.startCount;
    if (n !== lastStarts) {
      lastStarts = n;
      replay(barEl, 'started');
    }
  });

  /** Core gave no builds: the button retries instead of standing disabled. */
  const retry = $derived(launch.phase === 'idle' && builds.loaded && !build);

  function press() {
    if (retry) void builds.load();
    else if (launch.phase === 'idle') void launch.start();
    else if (launch.phase === 'launching') void launch.cancel();
    else void launch.stop();
  }

</script>

<footer class="bar" bind:this={barEl}>
  <div class="status">
    {#key shownId}<BuildCover id={shownId} size="bar" />{/key}
    <div class="status-t">
      <div class="title" aria-live="polite">
        {#if launch.phase === 'running'}
          <b class="live"><span class="dot" aria-hidden="true"></span>{launch.server ? t('shell.running_on', { server: launch.server }) : t('shell.running')}</b>
        {:else if launch.phase === 'launching'}
          {#key launch.stage}<b class="tick">{launch.stage}</b>{/key}
        {:else}
          <b>{t('shell.build_name', { ver: shownId })}</b>
        {/if}
      </div>
      {#if sub}<span class="sub">{sub}</span>{/if}
      {#if launch.phase === 'launching'}
        <div
          class="track"
          role="progressbar"
          aria-label={t('shell.launch_progress')}
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={launch.percent === null ? undefined : Math.round(launch.percent)}
        >
          <i style:transform="scaleX({(launch.percent ?? 0) / 100})"></i>
        </div>
      {/if}
    </div>
  </div>
  <button
    type="button"
    class="play"
    class:cancel={launch.phase === 'launching'}
    class:stop={launch.phase === 'running'}
    bind:this={playEl}
    disabled={launch.phase === 'idle' && !builds.loaded}
    aria-label={retry
      ? t('common.retry')
      : launch.phase === 'idle'
      ? t('shell.play_label', { ver: shownId })
      : launch.phase === 'launching'
        ? t('shell.cancel_label')
        : t('shell.stop_label')}
    onclick={press}
  >
    {#if retry}
      {t('common.retry')}
    {:else if launch.phase === 'idle'}
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 1v8l7-4z" /></svg>{t('shell.play')}
    {:else if launch.phase === 'launching'}
      {t('shell.cancel')}
    {:else}
      {t('shell.stop')}
    {/if}
  </button>
</footer>

<style>
  .bar {
    position: relative;
    grid-column: 2;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--gutter);
    padding: 12px 16px 12px var(--gutter);
    background: var(--bar);
    backdrop-filter: blur(30px) saturate(160%);
    -webkit-backdrop-filter: blur(30px) saturate(160%);
    border-top: 1px solid var(--edge);
  }
  :global(.bar.started)::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: -1px;
    height: 2px;
    background: var(--green);
    transform-origin: left;
    animation: started var(--dur-started) var(--ease) both;
  }
  @keyframes started {
    from {
      transform: scaleX(0);
    }
    70% {
      opacity: 1;
    }
    to {
      transform: scaleX(1);
      opacity: 0;
    }
  }
  .status {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
  }
  .status-t {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title {
    min-width: 0;
    overflow: hidden;
  }
  .title b {
    display: block;
    font-weight: var(--w-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tick {
    animation: tick 320ms var(--ease);
  }
  @keyframes tick {
    from {
      transform: translateY(50%);
      opacity: 0;
    }
  }
  .live {
    display: inline-flex !important;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--green);
    flex: none;
  }
  .sub {
    font-size: var(--t-foot);
    color: var(--label-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .track {
    position: relative;
    height: 4px;
    border-radius: 2px;
    background: var(--fill-2);
    overflow: hidden;
    max-width: 520px;
  }
  .track i {
    display: block;
    height: 100%;
    background: var(--blue-text);
    transform-origin: left;
    transition: transform 200ms linear;
  }
  .track::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--sweep), transparent);
    transform: translateX(-100%);
    animation: sweep 1.4s var(--ease) infinite;
  }
  @keyframes sweep {
    to {
      transform: translateX(100%);
    }
  }
  :global(:root[data-reduce-motion='true']) .track::after {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .track::after {
      display: none;
    }
    .track i {
      transition: none;
    }
  }
  .play {
    height: var(--control-h-big);
    min-width: 180px;
    padding: 0 28px;
    border: 0;
    border-radius: var(--r-m);
    background: var(--blue);
    color: var(--blue-ink);
    font: inherit;
    font-weight: var(--w-bold);
    font-size: 17px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color var(--dur-micro),
      transform var(--dur-micro) var(--ease),
      opacity var(--dur-micro);
  }
  .play:hover {
    background: var(--blue-hi);
  }
  .play:active {
    transform: scale(var(--press-scale));
  }
  .play[disabled] {
    opacity: 0.5;
    cursor: default;
  }
  .play svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }
  .play.cancel {
    background: var(--fill-2);
    color: var(--label);
  }
  .play.cancel:hover,
  .play.stop:hover {
    background: var(--fill);
  }
  .play.stop {
    background: var(--fill-2);
    color: var(--red);
  }
  :global(.play.swap) {
    animation: swap var(--dur-swap) var(--spring);
  }
  @keyframes swap {
    0% {
      transform: scale(0.94);
      opacity: 0.6;
    }
  }
</style>
