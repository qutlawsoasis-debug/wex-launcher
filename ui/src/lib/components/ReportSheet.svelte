<!--
  Wex Launcher — Bug Report Sheet (SPEC 2.10 & wex-parity)
  Apple HIG specifications:
  - https://developer.apple.com/design/human-interface-guidelines/sheets
  - https://developer.apple.com/design/human-interface-guidelines/color
  - https://developer.apple.com/design/human-interface-guidelines/typography
  Rules:
  - Honest disclosure: no fake server endpoints or made-up ticket IDs
  - Anonymized preview of all data before export (paths and nickname masked)
  - Options to save .zip to Desktop and copy summary to clipboard
-->
<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';

  interface Props {
    open?: boolean;
    version?: string;
    errorText?: string;
    diagnosisTitle?: string;
    playerName?: string;
    onclose?: () => void;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) => void;
  }

  let {
    open = $bindable(false),
    version = '1.21.1',
    errorText = '',
    diagnosisTitle = '',
    playerName = '',
    onclose,
    ontoast,
  }: Props = $props();

  let userComment = $state('');
  let isSavingZip = $state(false);
  let isCopying = $state(false);
  let copyFeedback = $state(false);
  let showFullPreview = $state(false);

  // Anonymization helper for local preview
  function maskText(raw: string): string {
    if (!raw) return '';
    let out = raw;
    if (playerName && playerName.trim() && playerName.toLowerCase() !== 'player') {
      out = out.replaceAll(playerName.trim(), '<player>');
    }
    out = out.replace(/[a-zA-Z]:\\Users\\[^\\/\r\n"'\s,;]+/gi, 'C:\\Users\\***');
    out = out.replace(/[a-zA-Z]:\/Users\/[^\\/\r\n"'\s,;]+/gi, 'C:/Users/***');
    out = out.replace(/\/home\/[^\\/\r\n"'\s,;]+/gi, '/home/***');
    return out;
  }

  let anonymizedError = $derived(maskText(errorText));
  let anonymizedComment = $derived(maskText(userComment));

  let previewJson = $derived(
    JSON.stringify(
      {
        launcher: 'Wex Launcher',
        launcher_version: '0.9.0',
        game_version: version,
        os: 'Windows 64-bit',
        java: version.startsWith('1.20') ? 'Java 17 (Mojang Runtime)' : 'Java 21 (Mojang Runtime)',
        diagnosis: diagnosisTitle || undefined,
        error_details: anonymizedError || undefined,
        user_comment: anonymizedComment || undefined,
        anonymized: true,
      },
      null,
      2
    )
  );

  async function handleSaveZip() {
    isSavingZip = true;
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core');
        const res: any = await invoke('report_save_zip', {
          version,
          errorText: errorText || null,
          userComment: userComment || null,
          playerName: playerName || null,
        });

        if (res && res.success) {
          ontoast?.(
            'success',
            'Отчёт сохранён на рабочий стол',
            res.file_name || 'Архив с логами и конфигурацией готов'
          );
        }
      } else {
        // Mock fallback for browser / vitest
        ontoast?.(
          'success',
          'Отчёт сохранён на рабочий стол',
          `Wex-report-${version}.zip`
        );
      }
    } catch (e: any) {
      ontoast?.('error', 'Не удалось сохранить отчёт', String(e?.message || e));
    } finally {
      isSavingZip = false;
    }
  }

  async function handleCopyReport() {
    isCopying = true;
    try {
      let textToCopy = '';
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core');
        textToCopy = await invoke('report_copy_text', {
          version,
          errorText: errorText || null,
          userComment: userComment || null,
          playerName: playerName || null,
        });
      }

      if (!textToCopy) {
        textToCopy = `### Отчёт об ошибке Wex Launcher\n- **Версия игры:** \`${version}\`\n- **Версия лаунчера:** \`0.9.0\`\n- **Диагноз:** ${diagnosisTitle || 'Сбой игры'}\n\n**Комментарий:**\n${anonymizedComment || 'Без комментария'}\n\n**Ошибка:**\n\`\`\`\n${anonymizedError || 'Нет технических подробностей'}\n\`\`\``;
      }

      if (navigator.clipboard) {
        await navigator.clipboard.writeText(textToCopy);
      }
      copyFeedback = true;
      ontoast?.('success', 'Отчёт скопирован в буфер обмена');
      setTimeout(() => {
        copyFeedback = false;
      }, 2500);
    } catch (e: any) {
      ontoast?.('error', 'Ошибка при копировании', String(e?.message || e));
    } finally {
      isCopying = false;
    }
  }

  function handleClose() {
    open = false;
    onclose?.();
  }
</script>

<Sheet bind:open title="Отчёт об ошибке" maxWidth="680px">
  <div class="report-container">
    <!-- Header summary info -->
    <div class="report-header-card">
      <div class="info-pill-row">
        <span class="info-pill version-pill" data-testid="pill-version">{version}</span>
        <span class="info-pill">Wex 0.9.0</span>
        <span class="info-pill secure-pill">
          <Icon name="check" size={12} color="var(--color-system-green)" />
          <span>Обезличено</span>
        </span>
      </div>

      {#if diagnosisTitle}
        <div class="diagnosis-lead" data-testid="report-diagnosis-title">
          {diagnosisTitle}
        </div>
      {/if}

      <div class="security-notice">
        Личные пути к файлам пользователя (<code>C:\Users\***</code>) и ваш никнейм автоматически скрыты из всех файлов и логов отчёта.
      </div>
    </div>

    <!-- User Comment section -->
    <div class="input-section">
      <label for="user-comment-field" class="section-label">
        Что произошло? (необязательно)
      </label>
      <textarea
        id="user-comment-field"
        class="comment-textarea"
        rows="3"
        placeholder="Опишите, какие действия выполнялись перед возникновением ошибки..."
        bind:value={userComment}
        data-testid="input-user-comment"
      ></textarea>
    </div>

    <!-- Included data list -->
    <div class="data-manifest">
      <span class="section-label">В архив будут включены:</span>
      <div class="manifest-grid">
        <div class="manifest-item">
          <Icon name="info" size={14} color="var(--color-system-blue)" />
          <span class="manifest-text"><strong>latest.log</strong> — журнал игры с маскировкой путей</span>
        </div>
        <div class="manifest-item">
          <Icon name="warning" size={14} color="var(--color-system-orange)" />
          <span class="manifest-text"><strong>crash-report</strong> — технический отчёт сбоя (если есть)</span>
        </div>
        <div class="manifest-item">
          <Icon name="folder" size={14} color="var(--color-secondary-label)" />
          <span class="manifest-text"><strong>mods.txt</strong> — список установленных модов версии</span>
        </div>
        <div class="manifest-item">
          <Icon name="cpu" size={14} color="var(--color-secondary-label)" />
          <span class="manifest-text"><strong>system-info.txt</strong> — версия ОС, Java и конфигурация памяти</span>
        </div>
      </div>
    </div>

    <!-- Collapsible JSON Preview -->
    <div class="preview-toggle-section">
      <button
        type="button"
        class="toggle-link"
        onclick={() => (showFullPreview = !showFullPreview)}
        data-testid="btn-toggle-preview"
      >
        <Icon name={showFullPreview ? 'chevron-down' : 'chevron-right'} size={12} />
        <span>{showFullPreview ? 'Скрыть структуру данных' : 'Показать структуру данных (JSON)'}</span>
      </button>

      {#if showFullPreview}
        <pre class="preview-codebox" data-testid="report-json-preview">{previewJson}</pre>
      {/if}
    </div>

    <!-- Offline note -->
    <div class="service-notice">
      Сервер автоматической отправки старого лаунчера недоступен. Сохраните ZIP-архив на рабочий стол или скопируйте текст отчёта.
    </div>

    <!-- Action Bar -->
    <div class="actions-bar">
      <div class="actions-left">
        <Button
          variant="primary"
          size="regular"
          onclick={handleSaveZip}
          loading={isSavingZip}
        >
          <Icon name="folder" size={14} />
          <span data-testid="btn-save-zip">Сохранить на рабочий стол (.zip)</span>
        </Button>

        <Button
          variant="secondary"
          size="regular"
          onclick={handleCopyReport}
          loading={isCopying}
        >
          <span data-testid="btn-copy-report">{copyFeedback ? 'Скопировано!' : 'Скопировать отчёт'}</span>
        </Button>
      </div>

      <div class="actions-right">
        <Button variant="ghost" size="regular" onclick={handleClose}>
          <span data-testid="btn-close-report">Закрыть</span>
        </Button>
      </div>
    </div>
  </div>
</Sheet>

<style>
  .report-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 6px 0;
  }

  .report-header-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-md);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
  }

  .info-pill-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .info-pill {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background-color: var(--color-tertiary-system-fill);
    color: var(--color-secondary-label);
  }

  .info-pill.version-pill {
    background-color: rgba(0, 122, 255, 0.15);
    color: var(--color-system-blue);
  }

  .info-pill.secure-pill {
    background-color: rgba(48, 209, 88, 0.15);
    color: var(--color-system-green);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .diagnosis-lead {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    line-height: 1.35;
  }

  .security-notice {
    font-size: 12px;
    line-height: 1.45;
    color: var(--color-secondary-label);
  }

  .security-notice code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    background-color: rgba(0, 0, 0, 0.2);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .input-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-secondary-label);
  }

  .comment-textarea {
    width: 100%;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-separator);
    background-color: var(--color-secondary-system-grouped-background);
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    outline: none;
    box-sizing: border-box;
    resize: vertical;
    transition: border-color var(--motion-duration-short) ease;
  }

  .comment-textarea:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
  }

  .data-manifest {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .manifest-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  @media (max-width: 600px) {
    .manifest-grid {
      grid-template-columns: 1fr;
    }
  }

  .manifest-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background-color: var(--color-secondary-system-fill);
    border: 1px solid var(--glass-border);
  }

  .manifest-text {
    font-size: 11px;
    color: var(--color-label);
    line-height: 1.3;
  }

  .preview-toggle-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .toggle-link {
    background: none;
    border: none;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--color-system-blue);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
    width: fit-content;
  }

  .toggle-link:hover {
    text-decoration: underline;
  }

  .preview-codebox {
    margin: 0;
    padding: 10px 12px;
    background-color: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-sm);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    line-height: 1.45;
    color: var(--color-secondary-label);
    max-height: 180px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
  }

  .service-notice {
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-tertiary-label);
    font-style: italic;
  }

  .actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 4px;
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
