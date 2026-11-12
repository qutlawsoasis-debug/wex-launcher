<!--
  Wex Launcher — Crash Report Sheet (SPEC 2.10)
  Apple HIG specifications:
  - https://developer.apple.com/design/human-interface-guidelines/sheets
  - https://developer.apple.com/design/human-interface-guidelines/alerts
  Rules:
  - Honest diagnosis without fabricated metrics or fake causes
  - Clear natural Russian phrasing
  - Contextual action buttons:
    * "Увеличить память" for OutOfMemory
    * "Отключить мод" for Mod/Mixin/Dependency conflicts
    * "Выбрать Java" for Java runtime mismatch
    * "Открыть лог" and "Скопировать отчёт" for all crashes
  - Monospace snippet viewer for technical log
-->
<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';
  import { t } from '../i18n';
  import ReportSheet from './ReportSheet.svelte';
  import type { CrashDiagnosisDto, CrashKindDto } from '../ipc/generated/models';

  interface Props {
    open?: boolean;
    diagnosis?: CrashDiagnosisDto | null;
    version?: string;
    onclose?: () => void;
    onnavigate?: (tab: string, highlight?: string) => void;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) => void;
  }

  let {
    open = $bindable(false),
    diagnosis = null,
    version = '1.21.1',
    onclose,
    onnavigate,
    ontoast,
  }: Props = $props();

  let isDisablingMod = $state(false);
  let isOpeningLog = $state(false);
  let copyFeedback = $state(false);
  let modDisabledSuccess = $state(false);
  let isReportSheetOpen = $state(false);

  interface ParsedCategory {
    badge: string;
    badgeType: 'oom' | 'mod' | 'java' | 'gpu' | 'corruption' | 'unknown';
    actionType: 'ram' | 'mod' | 'java' | 'shader' | 'world' | 'none';
    modName?: string;
  }

  function parseKind(kind: CrashKindDto | undefined): ParsedCategory {
    if (!kind) {
      return {
        badge: t('crash.badge_unknown'),
        badgeType: 'unknown',
        actionType: 'none',
      };
    }

    if (kind === 'OutOfMemory') {
      return {
        badge: t('crash.badge_oom'),
        badgeType: 'oom',
        actionType: 'ram',
      };
    }

    if (typeof kind === 'object') {
      if ('MissingFabricDependency' in kind) {
        return {
          badge: t('crash.badge_mod_dep'),
          badgeType: 'mod',
          actionType: 'mod',
          modName: kind.MissingFabricDependency.mod_name,
        };
      }
      if ('IncompatibleMod' in kind) {
        return {
          badge: t('crash.badge_mod_incompat'),
          badgeType: 'mod',
          actionType: 'mod',
          modName: kind.IncompatibleMod.mod_name,
        };
      }
      if ('MixinConflict' in kind) {
        const firstMod = kind.MixinConflict.mod_names[0];
        return {
          badge: t('crash.badge_mixin'),
          badgeType: 'mod',
          actionType: 'mod',
          modName: firstMod,
        };
      }
      if ('InvalidJavaVersion' in kind) {
        return {
          badge: t('crash.badge_java'),
          badgeType: 'java',
          actionType: 'java',
        };
      }
      if ('GraphicsDriverOrShader' in kind) {
        return {
          badge: t('crash.badge_gpu'),
          badgeType: 'gpu',
          actionType: 'shader',
        };
      }
      if ('CorruptedFile' in kind) {
        return {
          badge: t('crash.badge_files'),
          badgeType: 'corruption',
          actionType: 'world',
        };
      }
      if ('Unknown' in kind) {
        return {
          badge: t('crash.badge_unknown'),
          badgeType: 'unknown',
          actionType: 'none',
        };
      }
    }

    return {
      badge: t('crash.badge_unknown'),
      badgeType: 'unknown',
      actionType: 'none',
    };
  }

  let category = $derived(parseKind(diagnosis?.kind));

  let displayTitle = $derived.by(() => {
    if (!diagnosis) return t('crash.default_title');
    if (category.badgeType === 'unknown') {
      return diagnosis.title || t('crash.badge_unknown');
    }
    return diagnosis.title;
  });

  let displayAdvice = $derived.by(() => {
    if (modDisabledSuccess) {
      return t('crash.mod_disabled_msg');
    }
    if (!diagnosis) {
      return t('crash.unknown_advice');
    }
    if (category.badgeType === 'unknown') {
      return t('crash.unknown_advice');
    }
    return diagnosis.user_advice;
  });

  let technicalSnippet = $derived(diagnosis?.technical_summary || t('crash.empty_technical_log'));

  function notify(type: 'info' | 'success' | 'warning' | 'error', title: string, desc?: string) {
    if (ontoast) {
      ontoast(type, title, desc);
    }
  }

  async function handleCopyReport() {
    const reportText = [
      t('crash.report_header', { version }),
      t('crash.diagnosis_label', { title: displayTitle }),
      t('crash.category_label', { badge: category.badge }),
      t('crash.recommendation_label', { advice: displayAdvice }),
      '',
      t('crash.tech_log_separator'),
      technicalSnippet,
    ].join('\n');

    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(reportText);
      }
      copyFeedback = true;
      notify('success', t('crash.report_copied'));
      setTimeout(() => {
        copyFeedback = false;
      }, 2500);
    } catch {
      notify('info', t('crash.report_copy_failed'));
    }
  }

  async function handleOpenLog() {
    isOpeningLog = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const openedPath = await invoke<string>('crash_open_log_file', { version });
      notify('info', t('crash.log_opened'), openedPath);
    } catch {
      notify('info', t('crash.opening_logs_folder'), t('crash.instance_logs_desc', { version }));
    } finally {
      isOpeningLog = false;
    }
  }

  async function handleDisableMod() {
    if (!category.modName) return;
    isDisablingMod = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const disabledFile = await invoke<string>('crash_disable_mod', {
        modIdentifier: category.modName,
        version,
      });
      modDisabledSuccess = true;
      notify('success', t('crash.mod_disabled_toast', { name: category.modName }), disabledFile);
    } catch (e: any) {
      notify('error', t('crash.mod_disable_failed'), e?.message || String(e));
    } finally {
      isDisablingMod = false;
    }
  }

  function handleGoToSettingsRam() {
    open = false;
    onclose?.();
    if (onnavigate) {
      onnavigate('settings', 'ram');
    }
  }

  function handleGoToSettingsJava() {
    open = false;
    onclose?.();
    if (onnavigate) {
      onnavigate('settings', 'java');
    }
  }

  function handleGoToWorkshopShaders() {
    open = false;
    onclose?.();
    if (onnavigate) {
      onnavigate('workshop', 'shaders');
    }
  }

  function handleClose() {
    open = false;
    onclose?.();
  }
</script>

<Sheet
  bind:open
  title={t('crash.sheet_title')}
  maxWidth="560px"
  showCancel={false}
  showDone={false}
  onclose={handleClose}
>
  <div class="crash-sheet-content" data-testid="crash-sheet">
    <!-- Header alert area -->
    <div class="crash-header">
      <div class="crash-icon-badge" class:warning={category.badgeType === 'unknown'}>
        <Icon name={category.badgeType === 'unknown' ? 'warning' : 'error'} size={28} color="#ffffff" />
      </div>

      <div class="crash-title-group">
        <div class="crash-badge-row">
          <span class="category-badge {category.badgeType}" data-testid="crash-badge">
            {category.badge}
          </span>
          <span class="version-tag">{t('crash.version_tag', { version })}</span>
        </div>
        <h2 class="crash-title" data-testid="crash-title">{displayTitle}</h2>
      </div>
    </div>

    <!-- Advice Card -->
    <div class="advice-card" class:success-state={modDisabledSuccess} data-testid="crash-advice">
      <p class="advice-text">{displayAdvice}</p>
    </div>

    <!-- Technical Log Snippet -->
    <div class="log-section">
      <div class="log-header">
        <span class="log-label">{t('crash.log_label')}</span>
        <button type="button" class="copy-link" onclick={handleCopyReport}>
          {copyFeedback ? t('crash.copied') : t('crash.copy')}
        </button>
      </div>

      <pre class="log-code-box" data-testid="crash-log">{technicalSnippet}</pre>
    </div>

    <!-- Actions Bar -->
    <div class="actions-bar">
      <div class="actions-left">
        {#if !modDisabledSuccess}
          {#if category.actionType === 'ram'}
            <Button
              variant="primary"
              size="regular"
              onclick={handleGoToSettingsRam}
            >
              <span data-testid="btn-increase-ram">{t('crash.btn_increase_ram')}</span>
            </Button>
          {:else if category.actionType === 'mod' && category.modName}
            <Button
              variant="destructive"
              size="regular"
              onclick={handleDisableMod}
              loading={isDisablingMod}
            >
              <span data-testid="btn-disable-mod">{t('crash.btn_disable_mod')}</span>
            </Button>
          {:else if category.actionType === 'java'}
            <Button
              variant="primary"
              size="regular"
              onclick={handleGoToSettingsJava}
            >
              <span data-testid="btn-select-java">{t('crash.btn_select_java')}</span>
            </Button>
          {:else if category.actionType === 'shader'}
            <Button
              variant="primary"
              size="regular"
              onclick={handleGoToWorkshopShaders}
            >
              <span data-testid="btn-open-shaders">{t('crash.btn_open_shaders')}</span>
            </Button>
          {/if}
        {/if}

        <Button
          variant="secondary"
          size="regular"
          onclick={handleCopyReport}
        >
          <span data-testid="btn-copy-report">{copyFeedback ? t('crash.btn_report_copied') : t('crash.btn_copy_report')}</span>
        </Button>

        <Button
          variant="secondary"
          size="regular"
          onclick={handleOpenLog}
          loading={isOpeningLog}
        >
          <span data-testid="btn-open-log">{t('crash.btn_open_log')}</span>
        </Button>

        <Button
          variant="secondary"
          size="regular"
          onclick={() => (isReportSheetOpen = true)}
        >
          <span data-testid="btn-open-report">{t('crash.btn_send_report')}</span>
        </Button>
      </div>

      <div class="actions-right">
        <Button variant="ghost" size="regular" onclick={handleClose}>
          <span data-testid="btn-close-crash">{t('crash.btn_close')}</span>
        </Button>
      </div>
    </div>
  </div>

  <ReportSheet
    bind:open={isReportSheetOpen}
    {version}
    errorText={technicalSnippet}
    diagnosisTitle={displayTitle}
    {ontoast}
  />
</Sheet>

<style>
  .crash-sheet-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 0;
  }

  .crash-header {
    display: flex;
    align-items: flex-start;
    gap: 16px;
  }

  .crash-icon-badge {
    width: 48px;
    height: 48px;
    border-radius: var(--r-m);
    background-color: var(--red);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: 0 4px 12px rgba(255, 59, 48, 0.25);
  }

  .crash-icon-badge.warning {
    background-color: #ff9500;
    box-shadow: 0 4px 12px rgba(255, 149, 0, 0.25);
  }

  .crash-title-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .crash-badge-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .category-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--r-s);
  }

  .category-badge.oom {
    background-color: rgba(255, 59, 48, 0.15);
    color: var(--red);
  }

  .category-badge.mod {
    background-color: rgba(175, 82, 222, 0.15);
    color: #af52de;
  }

  .category-badge.java {
    background-color: rgba(10, 132, 255, 0.15);
    color: var(--blue-text);
  }

  .category-badge.gpu {
    background-color: rgba(255, 149, 0, 0.15);
    color: #ff9500;
  }

  .category-badge.corruption {
    background-color: rgba(255, 69, 58, 0.15);
    color: #ff453a;
  }

  .category-badge.unknown {
    background-color: var(--fill-2);
    color: var(--label-2);
  }

  .version-tag {
    font-size: 11px;
    color: var(--label-2);
  }

  .crash-title {
    font-size: var(--t-body);
    font-weight: 600;
    margin: 0;
    color: var(--label);
    line-height: 1.3;
  }

  .advice-card {
    padding: 14px 16px;
    border-radius: var(--r-m);
    background-color: var(--fill-2);
    border: 1px solid var(--edge);
  }

  .advice-card.success-state {
    background-color: rgba(48, 209, 88, 0.12);
    border-color: rgba(48, 209, 88, 0.3);
  }

  .advice-text {
    font-size: var(--t-sub);
    line-height: 1.5;
    margin: 0;
    color: var(--label);
  }

  .log-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .log-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--label-2);
  }

  .copy-link {
    background: none;
    border: none;
    font-size: 12px;
    color: var(--blue-text);
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }

  .copy-link:hover {
    text-decoration: underline;
  }

  .log-code-box {
    margin: 0;
    padding: 12px;
    background-color: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--edge);
    border-radius: var(--r-s);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    line-height: 1.45;
    color: var(--label-2);
    max-height: 160px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
  }

  .actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 8px;
    flex-wrap: wrap;
  }

  .actions-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .actions-right {
    display: flex;
    align-items: center;
  }
</style>
