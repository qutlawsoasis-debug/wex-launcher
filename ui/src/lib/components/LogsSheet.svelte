<!--
  Wex Launcher — Logs Viewer Sheet «Журнал» (SPEC 2.10)
  Apple HIG specifications:
  - https://developer.apple.com/design/human-interface-guidelines/sheets
  - https://developer.apple.com/design/human-interface-guidelines/search-fields
  - https://developer.apple.com/design/human-interface-guidelines/segmented-controls
  - https://developer.apple.com/design/human-interface-guidelines/buttons
  - https://developer.apple.com/design/human-interface-guidelines/color
  Rules:
  - Streaming / safe chunked reading for large log files (no UI freeze)
  - Color highlighting for ERROR (systemRed), WARN (systemOrange), and INFO
  - Real-time search filter and level filter
  - Quick actions: Copy log, Reload, Open OS folders
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Sheet from './Sheet.svelte';
  import Button from './Button.svelte';
  import SearchField from './SearchField.svelte';
  import SegmentedControl from './SegmentedControl.svelte';
  import Icon from './icons/Icon.svelte';
  import { t } from '../i18n';
  import { builds } from '../shell/builds.svelte';

  export interface LogFileInfo {
    file_name: string;
    relative_path: string;
    file_type: string;
    size_bytes: number | bigint;
    modified_timestamp: number | bigint;
    is_crash: boolean;
  }

  export interface LogEntry {
    line_number: number;
    timestamp: string | null;
    level: string;
    thread: string | null;
    message: string;
  }

  export interface LogReadResponse {
    file_name: string;
    total_lines: number;
    returned_lines: number;
    entries: LogEntry[];
    has_more: boolean;
    error_count: number;
    warn_count: number;
  }

  interface Props {
    open?: boolean;
    version?: string;
    initialFileName?: string;
    onclose?: () => void;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, description?: string) => void;
  }

  let {
    open = $bindable(false),
    version = builds.selectedId,
    initialFileName = 'latest.log',
    onclose,
    ontoast,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let currentVersion = $state(version);
  let selectedFileName = $state('latest.log');
  let searchQuery = $state('');
  let levelFilter = $state('ALL'); // 'ALL' | 'ERROR' | 'WARN' | 'INFO'

  let availableFiles = $state<LogFileInfo[]>([]);
  let logData = $state<LogReadResponse | null>(null);
  let isLoading = $state(false);
  let isCopying = $state(false);
  let copyFeedback = $state(false);

  $effect(() => {
    if (version) {
      currentVersion = version;
    }
  });

  $effect(() => {
    if (open) {
      loadFilesList();
    }
  });

  function notify(type: 'info' | 'success' | 'warning' | 'error', title: string, desc?: string) {
    if (ontoast) {
      ontoast(type, title, desc);
    }
  }

  async function loadFilesList() {
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const files = await invoke<LogFileInfo[]>('logs_list_files', {
        version: currentVersion,
      });
      availableFiles = files || [];
      if (availableFiles.length > 0) {
        if (!availableFiles.some((f) => f.file_name === selectedFileName)) {
          selectedFileName = availableFiles[0].file_name;
        }
      }
      await loadLogContent();
    } catch {
      availableFiles = [];
      logData = null;
    } finally {
      isLoading = false;
    }
  }

  async function loadLogContent() {
    if (!selectedFileName) return;
    isLoading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<LogReadResponse>('logs_read_file', {
        fileName: selectedFileName,
        version: currentVersion,
        maxLines: 2500,
        filterLevel: levelFilter === 'ALL' ? null : levelFilter,
        searchQuery: searchQuery.trim() || null,
      });
      logData = res;
    } catch (e: any) {
      logData = null;
      notify('error', t('logs.toast_read_error'), e?.message || String(e));
    } finally {
      isLoading = false;
    }
  }

  function handleVersionChange(v: string) {
    currentVersion = v;
    loadFilesList();
  }

  function handleFileSelect(fname: string) {
    selectedFileName = fname;
    loadLogContent();
  }

  function handleLevelChange(lvl: string) {
    levelFilter = lvl;
    loadLogContent();
  }

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  function handleSearchInput(val: string) {
    searchQuery = val;
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      loadLogContent();
    }, 250);
  }

  async function handleCopyLog() {
    if (!logData || logData.entries.length === 0) {
      notify('info', t('logs.toast_empty'));
      return;
    }

    isCopying = true;
    const lines = logData.entries.map((e) => {
      const timeStr = e.timestamp ? `[${e.timestamp}] ` : '';
      const threadStr = e.thread ? `[${e.thread}/${e.level}]: ` : `[${e.level}]: `;
      return `${timeStr}${threadStr}${e.message}`;
    });

    const fullText = lines.join('\n');

    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(fullText);
      }
      copyFeedback = true;
      notify('success', t('logs.toast_copied'), t('logs.toast_copied_desc', { count: logData.entries.length }));
      setTimeout(() => {
        copyFeedback = false;
        isCopying = false;
      }, 2000);
    } catch {
      notify('info', t('logs.toast_copy_failed'));
      isCopying = false;
    }
  }

  async function handleOpenFolder(type: 'logs' | 'crash_reports') {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const opened = await invoke<string>('logs_open_folder', {
        version: currentVersion,
        folderType: type,
      });
      notify('info', t('logs.toast_folder_opened'), opened);
    } catch {
      notify('info', t('logs.toast_opening_explorer'), t('logs.folder_type_label', { type }));
    }
  }

  function handleClose() {
    open = false;
    onclose?.();
  }
</script>

<Sheet
  bind:open
  title={t('logs.sheet_title')}
  maxWidth="920px"
  showCancel={false}
  showDone={false}
  onclose={handleClose}
>
  <div class="logs-sheet-container" data-testid="logs-sheet">
    <!-- Top Filter & Control Toolbar -->
    <div class="logs-toolbar">
      <div class="toolbar-top-row">
        <!-- Version Selector -->
        <div class="version-control">
          <SegmentedControl
            options={builds.list.map((b) => ({ value: b.id, label: b.id }))}
            bind:value={currentVersion}
            onchange={handleVersionChange}
          />
        </div>

        <!-- File Picker Selector -->
        <div class="file-picker-wrap">
          <label for="log-file-select" class="picker-label">{t('logs.file_label')}</label>
          <select
            id="log-file-select"
            class="file-select-dropdown"
            bind:value={selectedFileName}
            onchange={(e) => handleFileSelect((e.target as HTMLSelectElement).value)}
            data-testid="log-file-select"
          >
            {#if availableFiles.length === 0}
              <option value="latest.log">{t('logs.default_file_suffix')}</option>
            {:else}
              {#each availableFiles as file (file.file_name)}
                <option value={file.file_name}>
                  {file.file_name} {file.is_crash ? t('logs.crash_badge') : ''}
                </option>
              {/each}
            {/if}
          </select>
        </div>

        <!-- Telemetry Stats Badges -->
        <div class="metrics-row" data-testid="log-metrics">
          {#if logData}
            {#if logData.error_count > 0}
              <span class="metric-pill error-pill" title={t('logs.title_errors')}>
                {logData.error_count === 1 ? t('logs.metric_error_one', { count: logData.error_count }) : t('logs.metric_error_other', { count: logData.error_count })}
              </span>
            {/if}
            {#if logData.warn_count > 0}
              <span class="metric-pill warn-pill" title={t('logs.title_warns')}>
                {logData.warn_count === 1 ? t('logs.metric_warn_one', { count: logData.warn_count }) : t('logs.metric_warn_other', { count: logData.warn_count })}
              </span>
            {/if}
            <span class="metric-pill lines-pill">
              {logData.total_lines === 1 ? t('logs.metric_lines_one', { count: logData.total_lines }) : t('logs.metric_lines_other', { count: logData.total_lines })}
            </span>
          {/if}
        </div>
      </div>

      <div class="toolbar-bottom-row">
        <!-- Search Field -->
        <div class="search-field-wrap">
          <SearchField
            bind:value={searchQuery}
            placeholder={t('logs.search_placeholder')}
            oninput={handleSearchInput}
            onclear={() => {
              searchQuery = '';
              loadLogContent();
            }}
          />
        </div>

        <!-- Level Filter Tabs -->
        <div class="level-control-wrap">
          <SegmentedControl
            options={[
              { value: 'ALL', label: t('logs.level_all') },
              { value: 'ERROR', label: t('logs.level_error') },
              { value: 'WARN', label: t('logs.level_warn') },
              { value: 'INFO', label: t('logs.level_info') },
            ]}
            bind:value={levelFilter}
            onchange={handleLevelChange}
          />
        </div>
      </div>
    </div>

    <!-- Main Terminal View Container -->
    <div class="terminal-view-card" data-testid="log-terminal">
      {#if isLoading}
        <div class="loading-state">
          <div class="loading-indicator"></div>
          <span class="loading-text">{t('logs.loading_file', { name: selectedFileName })}</span>
        </div>
      {:else if !logData || logData.entries.length === 0}
        <div class="empty-log-state" data-testid="log-empty-state">
          <div class="empty-icon-wrap">
            <Icon name="info" size={32} color="var(--label-3)" />
          </div>
          <h4 class="empty-title">
            {searchQuery || levelFilter !== 'ALL'
              ? t('logs.empty_filtered_title')
              : t('logs.empty_title')}
          </h4>
          <p class="empty-sub">
            {searchQuery || levelFilter !== 'ALL'
              ? t('logs.empty_filtered_sub')
              : t('logs.empty_sub', { name: selectedFileName })}
          </p>
        </div>
      {:else}
        <div class="log-lines-stream">
          {#each logData.entries as entry (entry.line_number)}
            <div
              class="log-row"
              class:error-row={entry.level === 'ERROR'}
              class:warn-row={entry.level === 'WARN'}
              class:debug-row={entry.level === 'DEBUG'}
            >
              <span class="line-num">{entry.line_number}</span>

              {#if entry.timestamp}
                <span class="line-time">[{entry.timestamp}]</span>
              {/if}

              <span class="level-tag {entry.level.toLowerCase()}">
                {entry.level}
              </span>

              {#if entry.thread}
                <span class="line-thread">[{entry.thread}]</span>
              {/if}

              <span class="line-message">{entry.message}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer Actions Bar -->
    <div class="logs-footer-actions">
      <div class="actions-left">
        <Button
          variant="secondary"
          size="regular"
          onclick={handleCopyLog}
          loading={isCopying}
        >
          <span data-testid="btn-copy-log">
            {copyFeedback ? t('logs.btn_copied') : t('logs.btn_copy')}
          </span>
        </Button>

        <Button
          variant="ghost"
          size="regular"
          onclick={loadLogContent}
          loading={isLoading}
        >
          <Icon name="refresh" size={14} color="var(--label-2)" />
          <span data-testid="btn-reload-log">{t('logs.btn_reload')}</span>
        </Button>

        <Button
          variant="ghost"
          size="regular"
          onclick={() => handleOpenFolder('logs')}
        >
          <Icon name="folder" size={14} color="var(--label-2)" />
          <span data-testid="btn-open-logs-folder">{t('logs.btn_logs_folder')}</span>
        </Button>

        <Button
          variant="ghost"
          size="regular"
          onclick={() => handleOpenFolder('crash_reports')}
        >
          <Icon name="warning" size={14} color="var(--label-2)" />
          <span data-testid="btn-open-crashes-folder">{t('logs.btn_crashes_folder')}</span>
        </Button>
      </div>

      <div class="actions-right">
        <Button
          variant="secondary"
          size="regular"
          onclick={handleClose}
        >
          <span data-testid="btn-close-logs">{t('logs.btn_close')}</span>
        </Button>
      </div>
    </div>
  </div>
</Sheet>

<style>
  .logs-sheet-container {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 72vh;
    max-height: 720px;
    min-height: 480px;
  }

  .logs-toolbar {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background-color: var(--fill-2);
    padding: 12px;
    border-radius: var(--r-m);
    border: 1px solid var(--edge);
  }

  .toolbar-top-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .file-picker-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 220px;
  }

  .picker-label {
    font-size: var(--t-sub);
    font-weight: 500;
    color: var(--label-2);
  }

  .file-select-dropdown {
    flex: 1;
    height: 32px;
    background-color: var(--veil-strong);
    border: 1px solid var(--edge);
    border-radius: var(--r-s);
    color: var(--label);
    font-family: inherit;
    font-size: 13px;
    padding: 0 8px;
    outline: none;
    cursor: pointer;
  }

  .file-select-dropdown:focus {
    border-color: var(--blue-text);
  }

  .metrics-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .metric-pill {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--r-s);
  }

  .metric-pill.error-pill {
    background-color: rgba(255, 59, 48, 0.15);
    color: var(--red);
    border: 1px solid rgba(255, 59, 48, 0.25);
  }

  .metric-pill.warn-pill {
    background-color: rgba(255, 149, 0, 0.15);
    color: #ff9500;
    border: 1px solid rgba(255, 149, 0, 0.25);
  }

  .metric-pill.lines-pill {
    background-color: var(--fill);
    color: var(--label-2);
  }

  .toolbar-bottom-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .search-field-wrap {
    flex: 1;
    min-width: 200px;
  }

  .terminal-view-card {
    flex: 1;
    background-color: rgba(12, 14, 18, 0.88);
    border: 1px solid var(--edge);
    border-radius: var(--r-m);
    overflow-y: auto;
    overflow-x: auto;
    padding: 8px 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11.5px;
    line-height: 1.5;
    position: relative;
    user-select: text;
  }

  .loading-state,
  .empty-log-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 240px;
    text-align: center;
    padding: 24px;
  }

  .empty-icon-wrap {
    margin-bottom: 12px;
    opacity: 0.8;
  }

  .empty-title {
    font-size: var(--t-body);
    font-weight: 600;
    margin: 0 0 6px;
    color: var(--label);
  }

  .empty-sub {
    font-size: var(--t-sub);
    color: var(--label-2);
    margin: 0;
    max-width: 400px;
  }

  .log-lines-stream {
    display: flex;
    flex-direction: column;
  }

  .log-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 2px 12px;
    white-space: pre-wrap;
    word-break: break-all;
    border-left: 2px solid transparent;
  }

  .log-row:hover {
    background-color: rgba(255, 255, 255, 0.04);
  }

  .log-row.error-row {
    background-color: rgba(255, 59, 48, 0.08);
    border-left-color: var(--red);
  }

  .log-row.warn-row {
    background-color: rgba(255, 149, 0, 0.06);
    border-left-color: #ff9500;
  }

  .log-row.debug-row {
    opacity: 0.7;
  }

  .line-num {
    width: 38px;
    text-align: right;
    color: var(--label-3);
    user-select: none;
    flex-shrink: 0;
    font-size: 10.5px;
  }

  .line-time {
    color: var(--label-2);
    flex-shrink: 0;
  }

  .level-tag {
    font-weight: 700;
    font-size: 10px;
    padding: 1px 4px;
    border-radius: var(--r-s);
    flex-shrink: 0;
  }

  .level-tag.error {
    background-color: rgba(255, 59, 48, 0.22);
    color: var(--red);
  }

  .level-tag.warn {
    background-color: rgba(255, 149, 0, 0.22);
    color: #ff9500;
  }

  .level-tag.info {
    color: var(--label-2);
  }

  .level-tag.debug {
    color: var(--label-3);
  }

  .line-thread {
    color: #64d2ff;
    flex-shrink: 0;
  }

  .line-message {
    color: var(--label);
    flex: 1;
  }

  .log-row.error-row .line-message {
    color: #ff453a;
    font-weight: 500;
  }

  .log-row.warn-row .line-message {
    color: #ffd60a;
  }

  .logs-footer-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding-top: 4px;
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
