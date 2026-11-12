<!--
  Settings (reference: V.options). Everything applies at once and is saved to the
  launcher config through core (save_launcher_settings); nothing is kept only in
  the page. Search filters rows, the contents on the left follow the scroll.
  Only settings core supports are shown (owner rule); the rest are listed in the
  stage report with a plan.
  HIG › Settings: group related options, apply immediately, explain each option.
-->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { builds, type LauncherSettings } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { indicator } from '../../shell/indicator';
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { applyLook } from '../../shell/look';
  import { formatError } from '../../design/errors';
  import { motionReduced } from '../../design/wex-tokens';
  import { friendsLive } from '../../services/friendsLive';
  import { t } from '../../i18n';
  import LogsSheet from '../../components/LogsSheet.svelte';
  import ImportWizardSheet from '../../components/ImportWizardSheet.svelte';

  interface Props {
    onopenonboarding?: () => void;
  }

  let { onopenonboarding }: Props = $props();

  type Sec = 'game' | 'gfx' | 'look' | 'launcher' | 'data' | 'keys';
  const SECTIONS: Array<[Sec, string]> = [
    ['game', 'settings2.sec_game'],
    ['gfx', 'settings2.sec_gfx'],
    ['look', 'settings2.sec_look'],
    ['launcher', 'settings2.sec_launcher'],
    ['data', 'settings2.sec_data'],
    ['keys', 'settings2.sec_keys'],
  ];

  let s = $state<LauncherSettings | null>(null);
  let memGb = $state(6);
  let q = $state('');
  let current = $state<Sec>('game');
  let root: HTMLElement | undefined = $state();
  let cacheText = $state('');
  let cacheBytes = $state(0);
  let dataDir = $state('');
  let oldLauncher = $state<string | null>(null);
  let version = $state('');
  let updateText = $state('');
  let checking = $state(false);
  let cleaning = $state(false);
  let resetArmed = $state(false);
  let logsOpen = $state(false);
  let importOpen = $state(false);
  let visibleCount = $state(1);

  const totalGb = $derived(s && Number(s.total_memory_mb) > 0 ? Math.round(Number(s.total_memory_mb) / 1024) : 16);
  const maxGb = $derived(Math.max(2, Math.min(totalGb - 2, 32)));
  /** Memory kept for Windows and apps, as the reference shows it (and its advice: leave 4 GB). */
  const SYSTEM_GB = 4;
  const freeGb = $derived(Math.max(0, totalGb - SYSTEM_GB - memGb));
  const memWarn = $derived(memGb > totalGb - SYSTEM_GB);

  /** Graphics presets as core applies them (wex-extras presets.rs: render distance per tier). */
  const PRESETS: Array<[string, string, number]> = [
    ['Low', 'settings2.preset_low', 6],
    ['Medium', 'settings2.preset_medium', 8],
    ['High', 'settings2.preset_high', 8],
    ['Ultra', 'settings2.preset_ultra', 10],
  ];

  /** Shortcuts handled in shell/hotkeys.ts. Key names come from the dictionary (Ctrl, Alt, Enter, Esc). */
  const HOTKEYS: Array<[string, string]> = [
    ['settings2.key_search', 'settings2.kbd_search'],
    ['settings2.key_play', 'settings2.kbd_play'],
    ['settings2.key_build1', 'settings2.kbd_build1'],
    ['settings2.key_build2', 'settings2.kbd_build2'],
    ['settings2.key_back', 'settings2.kbd_back'],
    ['settings2.key_forward', 'settings2.kbd_forward'],
    ['settings2.key_close', 'settings2.kbd_close'],
  ];

  async function load() {
    try {
      s = await call<LauncherSettings>('get_launcher_settings');
      if (typeof s.ram_mb === 'number') memGb = Math.max(2, Math.round(s.ram_mb / 1024));
    } catch {
      s = {};
    }
  }

  async function loadExtras() {
    const [cache, dir, old, upd] = await Promise.all([
      call<{ total_bytes: number; formatted_size: string }>('get_storage_cache_info').catch(() => null),
      call<string>('get_data_dir').catch(() => ''),
      call<string | null>('extras_detect_old_launcher').catch(() => null),
      call<{ app_version: string }>('startup_get_info').catch(() => null),
    ]);
    if (cache) {
      cacheBytes = Number(cache.total_bytes) || 0;
      cacheText = cache.formatted_size;
    }
    dataDir = dir;
    oldLauncher = old;
    if (upd?.app_version) version = upd.app_version;
  }

  onMount(() => {
    void load();
    void loadExtras();
  });

  /** Save fields through core; apply the look right away. */
  async function save(patch: LauncherSettings) {
    try {
      s = await builds.saveSettings(patch);
      applyLook(s);
    } catch (e) {
      toasts.show(t('build.save_failed'), formatError(e, 'settings').description);
    }
  }

  function flip(key: string) {
    if (!s) return;
    const now = !!s[key];
    void save({ [key]: !now });
    if (key === 'friend_notifications') friendsLive.setNotificationsEnabled(!now);
  }

  let memTimer: ReturnType<typeof setTimeout> | null = null;
  function onMem(e: Event) {
    memGb = Number((e.currentTarget as HTMLInputElement).value);
    if (memTimer) clearTimeout(memTimer);
    memTimer = setTimeout(() => void save({ ram_mb: memGb * 1024 }), 400);
  }

  async function openDataDir() {
    try {
      await call('open_data_dir');
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'settings').description);
    }
  }

  async function clearCache() {
    cleaning = true;
    try {
      const r = await call<{ formatted_freed?: string }>('clear_storage_cache');
      toasts.show(t('settings2.cache_cleared'), r?.formatted_freed ? t('settings2.cache_freed', { size: r.formatted_freed }) : undefined);
      await loadExtras();
    } catch (e) {
      toasts.show(t('settings2.cache_failed'), formatError(e, 'settings').description);
    } finally {
      cleaning = false;
    }
  }

  async function checkUpdates() {
    checking = true;
    try {
      const r = await call<{ update_available: boolean; current_version: string; latest_version?: string | null }>('updater_check', { manifestUrl: null });
      version = r.current_version || version;
      updateText = r.update_available ? t('settings2.update_available', { version: r.latest_version ?? '' }) : t('settings2.update_latest');
    } catch (e) {
      updateText = formatError(e, 'general').title;
    } finally {
      checking = false;
    }
  }

  let resetTimer: ReturnType<typeof setTimeout> | null = null;
  async function reset() {
    if (!resetArmed) {
      resetArmed = true;
      if (resetTimer) clearTimeout(resetTimer);
      resetTimer = setTimeout(() => (resetArmed = false), 3000);
      return;
    }
    resetArmed = false;
    try {
      const fresh = await call<LauncherSettings>('reset_settings');
      s = fresh && typeof fresh === 'object' ? fresh : await call<LauncherSettings>('get_launcher_settings');
      builds.applySettings(s);
      applyLook(s);
      if (typeof s.ram_mb === 'number') memGb = Math.round(s.ram_mb / 1024);
      toasts.show(t('settings2.reset_done'));
    } catch (e) {
      toasts.show(t('settings2.reset_failed'), formatError(e, 'settings').description);
    }
  }

  // Search: hide rows that do not match; a section with no rows hides too.
  $effect(() => {
    const needle = q.trim().toLowerCase();
    void s;
    void tick().then(() => {
      if (!root) return;
      let any = 0;
      root.querySelectorAll<HTMLElement>('.sgroup').forEach((g) => {
        const title = g.querySelector('h2')?.textContent?.toLowerCase() ?? '';
        let n = 0;
        g.querySelectorAll<HTMLElement>('[data-k]').forEach((r) => {
          const hit = !needle || (r.dataset.k ?? '').toLowerCase().includes(needle) || (r.textContent ?? '').toLowerCase().includes(needle) || title.includes(needle);
          r.hidden = !hit;
          if (hit) n++;
        });
        g.hidden = n === 0;
        any += n;
      });
      visibleCount = any;
    });
  });

  // Contents follow the scroll (reference: spy).
  onMount(() => {
    const main = root?.closest('.main');
    if (!main) return;
    const spy = () => {
      let cur: Sec = SECTIONS[0][0];
      for (const [id] of SECTIONS) {
        const el = root?.querySelector<HTMLElement>(`#sec-${id}`);
        if (el && !el.hidden && el.getBoundingClientRect().top < 220) cur = id;
      }
      current = cur;
    };
    main.addEventListener('scroll', spy, { passive: true });
    return () => main.removeEventListener('scroll', spy);
  });

  function jump(e: MouseEvent, id: Sec) {
    e.preventDefault();
    current = id;
    root?.querySelector(`#sec-${id}`)?.scrollIntoView({ behavior: motionReduced() ? 'auto' : 'smooth', block: 'start' });
  }

  const on = (k: string) => !!s?.[k];
</script>

<div class="page-title">
  <h1>{t('shell.nav_options')}</h1>
  <p>{t('settings2.subtitle')}</p>
</div>

<div class="set2" bind:this={root}>
  <nav class="subnav" aria-label={t('settings2.contents')} use:indicator={{ selector: '[aria-current="true"]', key: current }}>
    {#each SECTIONS as [id, label] (id)}
      <a href="#options" aria-current={current === id ? 'true' : 'false'} onclick={(e) => jump(e, id)}>{t(label)}</a>
    {/each}
  </nav>

  <div class="set-main">
    <label class="search set-search">
      <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.5" /><path d="M10.5 10.5L14 14" /></svg>
      <input type="search" placeholder={t('settings2.search')} aria-label={t('settings2.search')} bind:value={q} />
    </label>
    {#if !visibleCount}
      <p class="empty"><b>{t('settings2.nothing')}</b> {t('settings2.nothing_hint')}</p>
    {/if}

    <section class="sgroup" id="sec-game">
      <h2>{t('settings2.sec_game')}</h2>
      <div class="form">
        <div class="srow col" data-k={t('settings2.k_memory')}>
          <div class="srow-h">
            <div>
              <div class="l">{t('build.memory')}</div>
              <div class="d">{memWarn ? t('settings2.memory_warn') : t('build.memory_shared')}</div>
            </div>
            <b class="val">{t('shell.gb', { n: memGb })}</b>
          </div>
          <div class="ram" aria-hidden="true">
            <i class="sys" style:--f={SYSTEM_GB / totalGb}></i>
            <i class="mc" class:warn={memWarn} style:--f={memGb / totalGb}></i>
            <i class="free" style:--f={freeGb / totalGb}></i>
          </div>
          <div class="ram-legend">
            <span><i class="sys"></i>{t('settings2.ram_rest', { gb: SYSTEM_GB })}</span>
            <span><i class="mc"></i>{t('settings2.ram_minecraft', { gb: memGb })}</span>
            <span><i class="free"></i>{t('settings2.ram_free', { gb: freeGb })}</span>
          </div>
          <input type="range" min="2" max={maxGb} step="1" value={memGb} aria-label={t('build.memory')} disabled={!s} oninput={onMem} />
        </div>
      </div>
    </section>

    <section class="sgroup" id="sec-gfx">
      <h2>{t('settings2.sec_gfx')}</h2>
      <div class="presets" data-k={t('settings2.k_graphics')} role="radiogroup" aria-label={t('settings2.sec_gfx')}>
        {#each PRESETS as [value, label, dist], i (value)}
          <button type="button" class="preset" role="radio" aria-checked={s?.graphics_preset === value} disabled={!s} onclick={() => save({ graphics_preset: value })}>
            <span class="bars" aria-hidden="true">{#each [0, 1, 2, 3] as n (n)}<i class:on={n <= i}></i>{/each}</span>
            <b>{t(label)}</b>
            <span>{t('settings2.preset_dist', { n: dist })}</span>
          </button>
        {/each}
      </div>
      <p class="hint" data-k={t('settings2.k_graphics')}>{t('settings2.preset_hint')}</p>
    </section>

    <section class="sgroup" id="sec-look">
      <h2>{t('settings2.sec_look')}</h2>
      <div class="form">
        <div class="srow col" data-k={t('settings2.k_theme')}>
          <div class="l">{t('settings2.theme')}</div>
          <div class="themes" role="radiogroup" aria-label={t('settings2.theme')}>
            {#each [['auto', 'system', 'settings2.theme_system'], ['light', 'light', 'settings2.theme_light'], ['dark', 'dark', 'settings2.theme_dark']] as [value, cls, label] (value)}
              <button type="button" class="theme-card {cls}" role="radio" aria-checked={(s?.appearance ?? 'auto') === value} disabled={!s} onclick={() => save({ appearance: value })}>
                <span class="mini-win" aria-hidden="true"><i></i><i></i><i></i></span>
                <b>{t(label)}</b>
              </button>
            {/each}
          </div>
        </div>
        <div class="srow" data-k={t('settings2.k_language')}>
          <div class="l">{t('settings2.language')}</div>
          <div class="seg" role="group" aria-label={t('settings2.language')} use:indicator={{ selector: '[aria-pressed="true"]', key: s?.language }}>
            <button type="button" aria-pressed={(s?.language ?? 'ru') === 'ru'} onclick={() => save({ language: 'ru' })}>{t('settings2.lang_ru')}</button>
            <button type="button" aria-pressed={s?.language === 'en'} onclick={() => save({ language: 'en' })}>{t('settings2.lang_en')}</button>
          </div>
        </div>
        <div class="srow" data-k={t('settings2.k_motion')}>
          <div><div class="l">{t('settings2.motion')}</div><div class="d">{t('settings2.motion_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={!!s && !s.reduce_motion} aria-label={t('settings2.motion')} disabled={!s} onclick={() => save({ reduce_motion: !s?.reduce_motion })}></button>
        </div>
        <div class="srow" data-k={t('settings2.k_transparency')}>
          <div><div class="l">{t('settings2.transparency')}</div><div class="d">{t('settings2.transparency_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('reduce_transparency')} aria-label={t('settings2.transparency')} disabled={!s} onclick={() => flip('reduce_transparency')}></button>
        </div>
      </div>
    </section>

    <section class="sgroup" id="sec-launcher">
      <h2>{t('settings2.sec_launcher')}</h2>
      <div class="form">
        <div class="srow" data-k={t('settings2.k_after')}>
          <div class="l">{t('settings2.after_launch')}</div>
          <div class="seg" role="group" aria-label={t('settings2.after_launch')} use:indicator={{ selector: '[aria-pressed="true"]', key: s?.game_window_behavior }}>
            {#each [['minimize', 'settings2.after_minimize'], ['tray', 'settings2.after_tray'], ['keep', 'settings2.after_keep']] as [value, label] (value)}
              <button type="button" aria-pressed={(s?.game_window_behavior ?? 'minimize') === value} onclick={() => save({ game_window_behavior: value })}>{t(label)}</button>
            {/each}
          </div>
        </div>
        <div class="srow" data-k={t('settings2.k_tray')}>
          <div><div class="l">{t('settings2.tray')}</div><div class="d">{t('settings2.tray_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('minimize_to_tray')} aria-label={t('settings2.tray')} disabled={!s} onclick={() => flip('minimize_to_tray')}></button>
        </div>
        <div class="srow" data-k={t('settings2.k_fast')}>
          <div><div class="l">{t('settings.fast_launch_title')}</div><div class="d">{t('settings.fast_launch_subtitle')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('fast_start_jvm')} aria-label={t('settings.fast_launch_title')} disabled={!s} onclick={() => flip('fast_start_jvm')}></button>
        </div>
        <!-- fast_launch: core starts Minecraft in advance (prewarm). Turning it off stops the running one at once. -->
        <div class="srow" data-k={t('settings2.k_prewarm')}>
          <div><div class="l">{t('settings2.prewarm')}</div><div class="d">{t('settings2.prewarm_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('fast_launch')} aria-label={t('settings2.prewarm')} disabled={!s} onclick={() => flip('fast_launch')}></button>
        </div>
        <div class="srow" data-k={t('settings2.k_discord')}>
          <div><div class="l">{t('settings2.discord')}</div><div class="d">{t('settings2.discord_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('discord_rpc')} aria-label={t('settings2.discord')} disabled={!s} onclick={() => flip('discord_rpc')}></button>
        </div>
        <div class="srow" data-k={t('settings2.k_friends')}>
          <div><div class="l">{t('settings.notify_friends_title')}</div><div class="d">{t('settings2.friends_d')}</div></div>
          <button type="button" class="switch" role="switch" aria-checked={on('friend_notifications')} aria-label={t('settings.notify_friends_title')} disabled={!s} onclick={() => flip('friend_notifications')}></button>
        </div>
        <div class="srow" data-k={t('settings2.k_updates')}>
          <div>
            <div class="l">{t('settings2.updates')}</div>
            <div class="d" aria-live="polite">{updateText || (version ? t('settings2.version', { version }) : t('settings2.updates_d'))}</div>
          </div>
          <button type="button" class="btn" disabled={checking} onclick={checkUpdates}>{t('settings2.check')}</button>
        </div>
      </div>
    </section>

    <section class="sgroup" id="sec-data">
      <h2>{t('settings2.sec_data')}</h2>
      <div class="form">
        <div class="srow" data-k={t('settings2.k_folder')}>
          <div><div class="l">{t('settings2.folder')}</div>{#if dataDir}<div class="d code">{dataDir}</div>{/if}</div>
          <button type="button" class="btn" onclick={openDataDir}>{t('build.open')}</button>
        </div>
        <div class="srow" data-k={t('settings2.k_cache')}>
          <div><div class="l">{t('settings2.cache')}</div><div class="d">{cacheBytes > 0 ? t('settings2.cache_d', { size: cacheText }) : t('settings2.cache_empty')}</div></div>
          <button type="button" class="btn" disabled={cleaning || cacheBytes <= 0 || launch.busy} onclick={clearCache}>{t('settings2.clear')}</button>
        </div>
        <div class="srow" data-k={t('settings2.k_logs')}>
          <div><div class="l">{t('settings.logs_viewer_title')}</div><div class="d">{t('settings2.logs_d')}</div></div>
          <button type="button" class="btn" onclick={() => (logsOpen = true)}>{t('build.open')}</button>
        </div>
        {#if oldLauncher}
          <div class="srow" data-k={t('settings2.k_import')}>
            <div><div class="l">{t('settings2.import')}</div><div class="d">{t('settings2.import_d')}</div></div>
            <button type="button" class="btn" onclick={() => (importOpen = true)}>{t('settings2.import_btn')}</button>
          </div>
        {/if}
        <div class="srow" data-k={t('settings2.k_onboarding')}>
          <div><div class="l">{t('settings.onboarding_replay_title')}</div><div class="d">{t('settings2.onboarding_d')}</div></div>
          <button type="button" class="btn" onclick={() => onopenonboarding?.()}>{t('settings.onboarding_replay_btn')}</button>
        </div>
        <div class="srow" data-k={t('settings2.k_reset')}>
          <div><div class="l">{t('settings2.reset')}</div><div class="d">{t('settings2.reset_d')}</div></div>
          <button type="button" class="btn danger" onclick={reset}>{resetArmed ? t('build.press_again') : t('settings2.reset_btn')}</button>
        </div>
      </div>
    </section>

    <section class="sgroup" id="sec-keys">
      <h2>{t('settings2.sec_keys')}</h2>
      <div class="form">
        {#each HOTKEYS as [label, keys] (label)}
          <div class="srow" data-k={t('settings2.k_keys')}>
            <div class="l">{t(label)}</div>
            <span class="keys">{#each t(keys).split(' ') as k (k)}<kbd>{k}</kbd>{/each}</span>
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>

{#if logsOpen}
  <LogsSheet bind:open={logsOpen} version={builds.selectedId} ontoast={(_t, title, d) => toasts.show(title, d)} />
{/if}
{#if importOpen && oldLauncher}
  <ImportWizardSheet
    bind:open={importOpen}
    initialDir={oldLauncher}
    onimported={async () => {
      await builds.load();
      await load();
    }}
  />
{/if}

<style>
  .page-title h1 {
    font-size: var(--t-t1);
    font-weight: var(--w-display);
    letter-spacing: -0.02em;
  }
  .page-title p {
    margin: 4px 0 0;
    color: var(--label-2);
  }
  .set2 {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    gap: 24px;
    align-items: start;
  }
  .subnav {
    position: sticky;
    top: 64px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .subnav a {
    position: relative;
    z-index: 1;
    height: 36px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-radius: var(--r-s);
    color: var(--label-2);
    font-weight: var(--w-medium);
    font-size: var(--t-sub);
    transition: color var(--dur-micro);
  }
  .subnav a:hover,
  .subnav a[aria-current='true'] {
    color: var(--label);
  }
  .subnav :global(> .ind) {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 0;
    pointer-events: none;
    border-radius: var(--r-s);
    background: var(--fill);
    transition:
      transform var(--dur-ind) var(--spring),
      opacity var(--dur-short);
  }
  .set-main {
    display: flex;
    flex-direction: column;
    gap: 28px;
    min-width: 0;
  }
  .set-search {
    flex: none;
    position: sticky;
    top: 64px;
    z-index: 2;
    background: var(--card);
    box-shadow:
      inset 0 0 0 1px var(--edge),
      0 8px 24px var(--shade);
  }
  .sgroup {
    scroll-margin-top: 120px;
  }
  .sgroup h2 {
    font-size: var(--t-t3);
    font-weight: var(--w-bold);
    margin: 0 0 12px 4px;
  }
  .srow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px 24px;
    align-items: center;
    min-height: 64px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--separator);
  }
  .srow:last-child {
    border-bottom: 0;
  }
  .srow.col {
    grid-template-columns: 1fr;
  }
  .srow-h {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
  }
  .srow .l {
    font-weight: var(--w-medium);
  }
  .srow .d {
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .code {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .val {
    font-size: var(--t-t3);
    font-weight: var(--w-bold);
    white-space: nowrap;
  }
  .ram {
    display: flex;
    height: 12px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--fill-3);
    gap: 2px;
  }
  .ram i {
    display: block;
    flex: 0 0 auto;
    width: calc(var(--f) * 100%);
    transition: width var(--dur-short) var(--ease);
  }
  .ram .sys,
  .ram-legend .sys {
    background: var(--label-3);
  }
  .ram .mc,
  .ram-legend .mc {
    background: var(--blue);
  }
  .ram .mc.warn {
    background: var(--orange);
  }
  .ram .free,
  .ram-legend .free {
    background: var(--fill);
  }
  .ram-legend {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .ram-legend span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .ram-legend i {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    display: block;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 12px;
  }
  /* Selection is a fill (AGENTS.md), so the reference's 2 px ring is not used. */
  .preset {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 16px;
    border-radius: var(--r-m);
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--edge);
    text-align: left;
    transition:
      background-color var(--dur-micro),
      transform var(--dur-short) var(--ease);
  }
  .preset:hover {
    transform: translateY(var(--hover-lift));
  }
  .preset b {
    font-weight: var(--w-bold);
  }
  .preset span:last-child {
    font-size: var(--t-cap);
    color: var(--label-2);
  }
  .preset[aria-checked='true'] {
    background: var(--blue-tint);
  }
  .bars {
    display: flex;
    gap: 4px;
    align-items: flex-end;
    height: 16px;
    margin-bottom: 4px;
  }
  .bars i {
    width: 5px;
    border-radius: 2px;
    background: var(--fill);
    display: block;
  }
  .bars i:nth-child(1) {
    height: 6px;
  }
  .bars i:nth-child(2) {
    height: 9px;
  }
  .bars i:nth-child(3) {
    height: 12px;
  }
  .bars i:nth-child(4) {
    height: 16px;
  }
  .bars i.on {
    background: var(--blue-text);
  }
  .hint {
    margin: 0 0 0 4px;
    font-size: var(--t-foot);
    color: var(--label-3);
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .theme-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 8px 12px;
    border-radius: var(--r-m);
    background: var(--fill-3);
    text-align: left;
    transition: background-color var(--dur-micro);
  }
  .theme-card b {
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    padding-left: 4px;
  }
  .theme-card[aria-checked='true'] {
    background: var(--blue-tint);
  }
  .mini-win {
    display: grid;
    grid-template-columns: 30% 1fr;
    grid-template-rows: 1fr 22%;
    gap: 4px;
    height: 64px;
    padding: 4px;
    border-radius: var(--r-xs);
    background: var(--mw-bg);
  }
  .mini-win i {
    border-radius: 3px;
    background: var(--mw-a);
    display: block;
  }
  .mini-win i:first-child {
    grid-row: 1 / 3;
    background: var(--mw-b);
  }
  .mini-win i:last-child {
    background: var(--mw-c);
  }
  .theme-card.light {
    --mw-bg: var(--mwl-bg);
    --mw-a: var(--mwl-a);
    --mw-b: var(--mwl-b);
    --mw-c: var(--mwl-c);
  }
  .theme-card.dark {
    --mw-bg: var(--mwd-bg);
    --mw-a: var(--mwd-a);
    --mw-b: var(--mwd-b);
    --mw-c: var(--mwd-c);
  }
  .theme-card.system {
    --mw-bg: linear-gradient(90deg, var(--mwl-bg) 50%, var(--mwd-bg) 50%);
    --mw-a: var(--mws-a);
    --mw-b: var(--mws-b);
    --mw-c: var(--mwd-c);
  }
  .keys {
    display: inline-flex;
    gap: 4px;
  }
  .keys kbd {
    min-width: 28px;
    height: 28px;
    padding: 0 8px;
    display: inline-grid;
    place-items: center;
    border-radius: var(--r-xs);
    background: var(--fill-2);
    box-shadow: inset 0 -1px 0 var(--edge);
    font: 600 var(--t-foot) var(--font);
    color: var(--label);
  }
  @media (max-width: 1100px) {
    .set2 {
      grid-template-columns: 1fr;
    }
    .subnav {
      position: static;
      flex-direction: row;
      flex-wrap: wrap;
    }
    .presets {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
