<!--
  World backups (workshop tab): pick a world, make a copy, restore one.
  Restoring replaces the world, so it asks twice (reference pattern «Нажмите ещё
  раз», as for Reset). HIG › Alerts: confirm actions that destroy data; the
  destructive button is red. Restoring is blocked while the game runs.
-->
<script lang="ts">
  import { launch } from '../../shell/launch.svelte';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { formatDate, formatSize } from '../../shell/format';
  import { t } from '../../i18n';
  import { build, type BackupItem } from './build.svelte';

  let creating = $state(false);
  let armed = $state<string | null>(null);
  let armTimer: ReturnType<typeof setTimeout> | null = null;

  async function create() {
    creating = true;
    try {
      const b = await build.createBackup();
      if (b) toasts.show(t('build.backup_created'), b.file_name);
    } catch (e) {
      const f = formatError(e, 'workshop');
      toasts.show(f.title, f.description);
    } finally {
      creating = false;
    }
  }

  async function restore(b: BackupItem) {
    if (armed !== b.file_path) {
      armed = b.file_path;
      if (armTimer) clearTimeout(armTimer);
      armTimer = setTimeout(() => (armed = null), 3000);
      return;
    }
    armed = null;
    try {
      await build.restoreBackup(b);
      toasts.show(t('build.backup_restored'));
    } catch (e) {
      const f = formatError(e, 'workshop');
      toasts.show(f.title, f.description);
    }
  }

  function pickWorld(e: Event) {
    build.backupWorld = (e.currentTarget as HTMLSelectElement).value;
    build.backups = null;
    void build.loadBackups();
  }
</script>

{#if build.worlds && build.worlds.length}
  <div class="tools">
    <label class="pick">
      <span>{t('build.backup_world')}</span>
      <select value={build.backupWorld} onchange={pickWorld} aria-label={t('build.backup_world')}>
        {#each build.worlds as w (w.folder_name)}
          <option value={w.folder_name}>{w.display_name}</option>
        {/each}
      </select>
    </label>
    <span class="grow"></span>
    <button type="button" class="btn" disabled={creating} onclick={create}>{t('build.backup_create')}</button>
  </div>
  <div class="list">
    {#if build.backups === null}
      <p class="empty" aria-busy="true">{t('build.loading')}</p>
    {:else if build.backups.length}
      {#each build.backups as b (b.file_path)}
        <div class="lrow lrow-backup">
          <div>
            <b>{formatDate(b.created_at) || b.file_name}</b>
            <span>{[b.file_name, formatSize(b.size_bytes)].join(', ')}</span>
          </div>
          <button type="button" class="mini" class:danger={armed === b.file_path} disabled={launch.phase === 'running'} onclick={() => restore(b)}>
            {armed === b.file_path ? t('build.press_again') : t('build.backup_restore')}
          </button>
        </div>
      {/each}
    {:else}
      <p class="empty"><b>{t('build.backups_empty')}</b> {t('build.backups_empty_hint')}</p>
    {/if}
  </div>
{:else if build.worlds}
  <div class="list"><p class="empty"><b>{t('build.worlds_empty')}</b> {t('build.backups_need_world')}</p></div>
{:else}
  <div class="list"><p class="empty" aria-busy="true">{t('build.loading')}</p></div>
{/if}

<style>
  .grow {
    flex: 1;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    color: var(--label-2);
    font-size: var(--t-sub);
  }
  .pick select {
    color: var(--label);
  }
  .lrow-backup {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .mini.danger {
    color: var(--red);
  }
</style>
