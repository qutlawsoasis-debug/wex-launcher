<!--
  Build settings (reference: Настройки сборки). Memory and Java are saved to the
  launcher config through core; in core they are shared by both builds, so the
  page says so instead of pretending they are per build. Folder opens the
  build's instance folder; Verify checks and repairs files (verify_game_files)
  with progress from storage-verify-progress.
  HIG › Sliders; HIG › Pop-up buttons; HIG › Text fields.
-->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { builds, type LauncherSettings } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { call, on } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { t } from '../../i18n';
  import { build } from './build.svelte';

  interface Props {
    version: string;
  }

  let { version }: Props = $props();

  let settings = $state<LauncherSettings | null>(null);
  let memGb = $state(6);
  let javaMode = $state<'auto' | 'custom'>('auto');
  let javaPath = $state('');
  let verifying = $state(false);
  let verifyText = $state('');

  const b = $derived(builds.list.find((x) => x.id === version));
  const totalGb = $derived(settings && typeof settings.total_memory_mb === 'number' ? Math.floor(Number(settings.total_memory_mb) / 1024) : 16);
  const maxGb = $derived(Math.max(2, Math.min(16, totalGb)));

  $effect(() => {
    void (async () => {
      try {
        const s = await call<LauncherSettings>('get_launcher_settings');
        settings = s;
        if (typeof s.ram_mb === 'number') memGb = Math.max(2, Math.round(s.ram_mb / 1024));
        javaMode = s.java_mode === 'custom' ? 'custom' : 'auto';
        javaPath = typeof s.custom_java_path === 'string' ? s.custom_java_path : '';
      } catch {
        settings = {};
      }
    })();
  });

  /** Save fields into the launcher settings without touching the others. */
  async function save(patch: Record<string, unknown>) {
    try {
      const current = await call<LauncherSettings>('get_launcher_settings');
      const next = { ...current, ...patch };
      await call('save_launcher_settings', { newSettings: next });
      settings = next;
      builds.applySettings(next);
    } catch (e) {
      toasts.show(t('build.save_failed'), formatError(e, 'settings').description);
    }
  }

  let memTimer: ReturnType<typeof setTimeout> | null = null;
  function onMem(e: Event) {
    memGb = Number((e.currentTarget as HTMLInputElement).value);
    if (memTimer) clearTimeout(memTimer);
    memTimer = setTimeout(() => void save({ ram_mb: memGb * 1024 }), 400);
  }

  function onJava(e: Event) {
    javaMode = (e.currentTarget as HTMLSelectElement).value === 'custom' ? 'custom' : 'auto';
    void save({ java_mode: javaMode, custom_java_path: javaMode === 'custom' && javaPath.trim() ? javaPath.trim() : null });
  }

  function onJavaPath() {
    void save({ java_mode: 'custom', custom_java_path: javaPath.trim() || null });
  }

  async function openFolder() {
    const dir = build.instanceDir();
    if (!dir) return;
    try {
      await call('workshop_open_path', { path: dir });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }

  let stopProgress: (() => void) | null = null;
  async function verify() {
    if (verifying) return;
    verifying = true;
    verifyText = t('build.verify_running');
    stopProgress = on<{ message?: string; percentage?: number }>('storage-verify-progress', (p) => {
      const pct = typeof p.percentage === 'number' ? ` ${Math.round(p.percentage)}%` : '';
      verifyText = (p.message || t('build.verify_running')) + pct;
    });
    try {
      const r = await call<{ status_message: string }>('verify_game_files', { version });
      toasts.show(t('build.verify_done'), r.status_message);
    } catch (e) {
      const f = formatError(e, 'install');
      toasts.show(f.title, f.description);
    } finally {
      stopProgress?.();
      stopProgress = null;
      verifying = false;
      verifyText = '';
    }
  }

  onDestroy(() => {
    stopProgress?.();
    if (memTimer) clearTimeout(memTimer);
  });
</script>

<div class="form">
  <div class="frow full">
    <div>
      <div class="l">{t('build.memory')}</div>
      <div class="d">{t('build.memory_shared')}</div>
    </div>
    <div class="mem">
      <input type="range" min="2" max={maxGb} step="1" value={memGb} aria-label={t('build.memory')} disabled={!settings} oninput={onMem} />
      <b>{t('shell.gb', { n: memGb })}</b>
    </div>
  </div>
  <div class="frow">
    <div>
      <div class="l">{t('build.java')}</div>
      <div class="d">{t('build.java_shared')}</div>
    </div>
    <select aria-label={t('build.java')} value={javaMode} onchange={onJava} disabled={!settings}>
      <option value="auto">{t('build.java_builtin', { java: b?.java ?? '' })}</option>
      <option value="custom">{t('build.java_custom')}</option>
    </select>
    {#if javaMode === 'custom'}
      <input
        class="field path"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder={t('build.java_path_placeholder')}
        aria-label={t('build.java_path')}
        bind:value={javaPath}
        onchange={onJavaPath}
      />
    {/if}
  </div>
  <div class="frow">
    <div class="l">{t('build.folder')}</div>
    <button type="button" class="btn" disabled={!build.instanceDir()} onclick={openFolder}>{t('build.open')}</button>
  </div>
  <div class="frow">
    <div>
      <div class="l">{t('build.verify')}</div>
      <div class="d" aria-live="polite">{verifying ? verifyText : t('build.verify_hint')}</div>
    </div>
    <button type="button" class="btn" disabled={verifying || launch.busy} onclick={verify}>{t('build.verify_action')}</button>
  </div>
</div>

<style>
  .mem {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px 16px;
    align-items: center;
  }
  .mem b {
    font-size: var(--t-t2);
    font-weight: var(--w-bold);
    min-width: 5ch;
    text-align: right;
  }
  .path {
    grid-column: 1 / -1;
    width: 100%;
    margin-top: 8px;
  }
</style>
