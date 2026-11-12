<!--
  Build page (reference: V.build). Header in the build's colour with the version
  set large, facts, tabs: Mods, Worlds, the workshop (resource packs, shaders,
  screenshots, backups — owner decision) and Build settings.
  HIG › Tab views: tabs switch sections of one place, the selected one is marked
  by a gliding underline; arrow keys move between tabs.
-->
<script lang="ts">
  import { builds, coverVar } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { toasts } from '../../shell/toast.svelte';
  import { indicator } from '../../shell/indicator';
  import { formatError } from '../../design/errors';
  import { motionReduced, WexMotion } from '../../design/wex-tokens';
  import { t, tn } from '../../i18n';
  import { build, BUILD_TABS, type BuildTab } from './build.svelte';
  import ModsTab from './ModsTab.svelte';
  import WorldsTab from './WorldsTab.svelte';
  import PacksTab from './PacksTab.svelte';
  import ShadersTab from './ShadersTab.svelte';
  import ShotsTab from './ShotsTab.svelte';
  import BackupsTab from './BackupsTab.svelte';
  import SettingsTab from './SettingsTab.svelte';

  interface Props {
    version: string;
  }

  let { version }: Props = $props();
  let tab = $state<BuildTab>('mods');

  const b = $derived(builds.list.find((x) => x.id === version));
  const selected = $derived(builds.selectedId === version);

  const TAB_LABEL: Record<BuildTab, string> = {
    mods: 'build.tab_mods',
    worlds: 'build.tab_worlds',
    packs: 'build.tab_packs',
    shaders: 'build.tab_shaders',
    shots: 'build.tab_shots',
    backups: 'build.tab_backups',
    settings: 'build.tab_settings',
  };

  $effect(() => {
    build.open(version);
    void build.loadBasics();
  });

  $effect(() => {
    const current = tab;
    void build.version;
    void build.loadTab(current);
  });

  async function select() {
    try {
      await builds.select(version);
      toasts.show(t('shell.build_selected', { ver: version }));
    } catch (e) {
      toasts.show(t('shell.select_failed'), formatError(e, 'settings').description);
    }
  }

  async function play() {
    if (launch.busy) return;
    if (!selected) await select();
    if (builds.selectedId === version) void launch.start();
  }

  function onTabKey(e: KeyboardEvent) {
    const i = BUILD_TABS.indexOf(tab);
    let next = -1;
    if (e.key === 'ArrowRight') next = (i + 1) % BUILD_TABS.length;
    else if (e.key === 'ArrowLeft') next = (i - 1 + BUILD_TABS.length) % BUILD_TABS.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = BUILD_TABS.length - 1;
    if (next < 0) return;
    e.preventDefault();
    tab = BUILD_TABS[next];
    (e.currentTarget as HTMLElement).querySelector<HTMLButtonElement>(`#btab-${tab}`)?.focus();
  }

  /** Numbers count up once when the page opens (reference: data-count), not under Reduce Motion. */
  function countUp(node: HTMLElement, to: number) {
    if (motionReduced() || !to) return;
    const t0 = performance.now();
    let raf = 0;
    const step = (now: number) => {
      const k = Math.min(1, (now - t0) / WexMotion.ms.countUp);
      node.textContent = String(Math.round(to * (1 - Math.pow(1 - k, 3))));
      if (k < 1) raf = requestAnimationFrame(step);
    };
    node.textContent = '0';
    raf = requestAnimationFrame(step);
    return { destroy: () => cancelAnimationFrame(raf) };
  }
</script>

<section class="build-head" style:--hero-c={coverVar(version)}>
  <span class="big-ver" aria-hidden="true">{version}</span>
  <div class="hero-in">
    <div>
      <h1>{t('shell.build_name', { ver: version })}</h1>
      {#if b}<p>{t('build.head_meta', { java: b.java })}</p>{/if}
    </div>
    <div class="hero-actions">
      {#if !selected}
        <button type="button" class="btn" disabled={launch.busy} onclick={select}>{t('build.select_for_launch')}</button>
      {/if}
      <button type="button" class="btn blue big" disabled={launch.busy} onclick={play}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 1v8l7-4z" /></svg>{t('shell.play')}
      </button>
    </div>
  </div>
</section>

<div class="facts">
  {#if build.mods}
    <div class="fact"><b use:countUp={build.mods.length}>{build.mods.length}</b><span>{tn('build.fact_mods', build.mods.length)}</span></div>
  {/if}
  {#if build.worlds}
    <div class="fact"><b use:countUp={build.worlds.length}>{build.worlds.length}</b><span>{tn('build.fact_worlds', build.worlds.length)}</span></div>
  {/if}
  {#if builds.ramMb}
    <div class="fact"><b>{t('shell.gb', { n: Math.round(builds.ramMb / 1024) })}</b><span>{t('build.fact_memory')}</span></div>
  {/if}
</div>

<div>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="tabs" role="tablist" aria-label={t('shell.build_tabs')} tabindex="-1" onkeydown={onTabKey} use:indicator={{ selector: '[aria-selected="true"]', key: tab, kind: 'underline' }}>
    {#each BUILD_TABS as id (id)}
      <button
        type="button"
        role="tab"
        id="btab-{id}"
        aria-selected={tab === id}
        aria-controls="bpanel"
        tabindex={tab === id ? 0 : -1}
        onclick={() => (tab = id)}>{t(TAB_LABEL[id])}</button
      >
    {/each}
  </div>
  <div class="panel-body" id="bpanel" role="tabpanel" aria-labelledby="btab-{tab}">
    {#if tab === 'mods'}<ModsTab />
    {:else if tab === 'worlds'}<WorldsTab />
    {:else if tab === 'packs'}<PacksTab />
    {:else if tab === 'shaders'}<ShadersTab />
    {:else if tab === 'shots'}<ShotsTab />
    {:else if tab === 'backups'}<BackupsTab />
    {:else}<SettingsTab {version} />{/if}
  </div>
</div>

<style>
  .build-head {
    position: relative;
    min-height: 260px;
    display: flex;
    align-items: flex-end;
    isolation: isolate;
    border-radius: var(--r-l);
    overflow: hidden;
    background: var(--hero-c);
    color: var(--cover-ink);
  }
  .big-ver {
    position: absolute;
    right: -0.04em;
    bottom: -0.2em;
    z-index: -1;
    font-size: clamp(160px, 22vw, 300px);
    font-weight: var(--w-display);
    letter-spacing: -0.06em;
    line-height: 1;
    opacity: 0.16;
    pointer-events: none;
    user-select: none;
    white-space: nowrap;
  }
  .hero-in {
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    padding: 28px 32px;
    flex-wrap: wrap;
  }
  h1 {
    font-size: var(--t-hero);
    font-weight: var(--w-display);
    letter-spacing: -0.03em;
    line-height: 1;
  }
  .hero-in p {
    margin: 8px 0 0;
    color: var(--cover-ink);
    opacity: 0.82;
    max-width: 54ch;
  }
  .hero-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .build-head :global(.btn:not(.blue)) {
    background: var(--hero-btn);
    color: var(--cover-ink);
  }
  .build-head :global(.btn:not(.blue):hover) {
    background: var(--hero-btn-hi);
  }
  .build-head :global(.btn.blue) {
    background: var(--cover-ink);
    color: var(--hero-c);
  }
  .build-head :global(.btn.blue:hover) {
    opacity: 0.92;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  .fact {
    padding: 16px;
    border-radius: var(--r-m);
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--edge);
    transition:
      background-color var(--dur-short),
      transform var(--dur-short) var(--ease);
  }
  .fact b {
    display: block;
    font-size: var(--t-t2);
    font-weight: var(--w-bold);
  }
  .fact span {
    display: block;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .panel-body {
    margin-top: 16px;
  }
  @media (max-width: 1100px) {
    .facts {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
