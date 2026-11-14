<!--
  Screenshots of the build (workshop tab): date and size, copy the image to the
  clipboard, show the file in Explorer. Thumbnails are not shown: the app has no
  way to load local images yet (asset protocol is not enabled in core).
  HIG › Lists and tables.
-->
<script lang="ts">
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { formatDate, formatSize } from '../../shell/format';
  import { t } from '../../i18n';
  import { build, type ShotItem } from './build.svelte';

  async function copy(s: ShotItem) {
    try {
      await call('workshop_copy_screenshot_image', { path: s.file_path });
      toasts.show(t('build.shot_copied'));
    } catch (e) {
      toasts.show(t('build.shot_copy_failed'), formatError(e, 'workshop').description);
    }
  }

  async function reveal(path: string) {
    try {
      await call('workshop_open_path', { path });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }

  async function openFolder() {
    try {
      await call('workshop_open_screenshots_folder', { version: build.version });
    } catch (e) {
      toasts.show(t('build.open_failed'), formatError(e, 'workshop').description);
    }
  }
</script>

<div class="tools">
  <span class="grow"></span>
  <button type="button" class="btn" onclick={openFolder}>{t('build.shots_folder')}</button>
</div>

<div class="list">
  {#if build.shots === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else if build.shots.length}
    {#each build.shots as s (s.file_path)}
      <div class="lrow lrow-shot">
        <div>
          <b>{s.file_name}</b>
          <span>{[formatDate(s.created_at), formatSize(s.size_bytes)].filter(Boolean).join(', ')}</span>
        </div>
        <button type="button" class="mini" onclick={() => copy(s)}>{t('build.shot_copy')}</button>
        <button type="button" class="mini" onclick={() => reveal(s.file_path)}>{t('build.show_in_folder')}</button>
      </div>
    {/each}
  {:else}
    <p class="empty"><b>{t('build.shots_empty')}</b> {t('build.shots_empty_hint')}</p>
  {/if}
</div>

<style>
  .grow {
    flex: 1;
  }
  .lrow-shot {
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 8px;
  }
</style>
