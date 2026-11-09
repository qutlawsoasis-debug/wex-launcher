<!--
  Wex Launcher — Screen «Настройки»
  Specification: tasks/apple/hig/settings.md, tasks/apple/hig/sliders.md, tasks/apple/hig/toggles.md, tasks/apple/hig/segmented-controls.md
  Rules:
  - Java runtime: auto-detection (Java 17 for 1.20.1, Java 21 for 1.21.1) or custom executable path
  - RAM allocation: slider with 512 MB step, contextual Apple HIG hint based on system memory
  - Graphics presets: wex-extras presets (Low, Medium, High, Ultra)
  - Data directory: path display and open in Explorer
  - Discord RPC: systemGreen toggle with live status preview
  - Language: RU / EN selection
  - Appearance: Dark / Light / Auto & Accent picker
  - Accessibility: Reduce Motion and Reduce Transparency toggles
  - About Wex: authentic version and system telemetry
  - Bidirectional sync via get_launcher_settings / save_launcher_settings / reset_settings
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Slider from '../components/Slider.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Button from '../components/Button.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import LogsSheet from '../components/LogsSheet.svelte';
  import { theme } from '../design/theme.svelte';
  import { network } from '../design/network.svelte';
  import { AccentPresets, type AccentColor } from '../design/colors';
  import { i18n, t, type SupportedLocale } from '../i18n';
  import { friendsLive } from '../services/friendsLive';

  interface Props {
    onopenonboarding?: () => void;
    isGameRunning?: boolean;
    version?: string;
  }

  let {
    onopenonboarding,
    isGameRunning = false,
    version: propVersion = '1.21.1',
  }: Props = $props();

  let ipcGameRunning = $state(false);
  let isGameActive = $derived(isGameRunning || ipcGameRunning);

  // Storage section state (SPEC items 40–41: wex-verify-files)
  let cacheSizeFormatted = $state('0 Б');
  let isCheckingIntegrity = $state(false);
  let isCleaningCache = $state(false);
  let integrityStatus = $state<string | null>(null);
  let integrityDetail = $state<string | null>(null);
  let cleanupStatus = $state<string | null>(null);

  // System & configuration state
  let selectedVersion = $state('1.21.1');
  let logsSheetOpen = $state(false);
  let playerName = $state('Player');
  let totalMemoryMb = $state(16384);
  let ramMb = $state(4096);
  let gameDir = $state('');
  let graphicsPreset = $state('High');
  let javaMode = $state('auto');
  let customJavaPath = $state('');
  let detectedJava17 = $state<string | null>(null);
  let detectedJava21 = $state<string | null>(null);
  let discordRpc = $state(true);
  let friendNotifications = $state(true);
  let backgroundMusic = $state(true);
  let fastStartJvm = $state(false);
  let language = $state<string>(i18n.locale);
  let launcherVersion = $state('1.0.0');
  let osName = $state('windows');
  let minimizeToTray = $state(true);
  let gameWindowBehavior = $state('minimize');
  let updateChecking = $state(false);
  let updateStatusText = $state('');
  let updateAvailable = $state(false);
  let canAutoUpdateSettings = $state(false);
  let updateDownloadUrl = $state('');

  async function handleCheckUpdateSettings() {
    updateChecking = true;
    updateStatusText = 'Проверка обновлений…';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<any>('updater_check');
      if (res && res.update_available) {
        updateAvailable = true;
        canAutoUpdateSettings = Boolean(
          res.can_auto_update ?? (res.signature && res.signature.length >= 32)
        );
        updateDownloadUrl = res.download_url || '';
        updateStatusText = `Доступна версия ${res.latest_version || ''}${!canAutoUpdateSettings ? ' (неподписанная)' : ''}`;
      } else {
        updateAvailable = false;
        updateStatusText = 'У вас установлена последняя версия';
      }
    } catch {
      updateStatusText = 'Не удалось проверить обновления';
    } finally {
      updateChecking = false;
    }
  }

  async function handleApplyUpdateSettings() {
    if (!updateDownloadUrl) return;
    if (!canAutoUpdateSettings) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('updater_open_download_url', { url: updateDownloadUrl });
      } catch {
        if (typeof window !== 'undefined') window.open(updateDownloadUrl, '_blank');
      }
    } else {
      updateStatusText = 'Перейдите на экран «Играть» для установки';
    }
  }

  const gameWindowBehaviorOptions = [
    { value: 'keep', label: 'Оставить' },
    { value: 'minimize', label: 'Свернуть' },
    { value: 'tray', label: 'Скрыть в трей' },
  ];

  let graphicsOptions = $derived([
    { value: 'Low', label: t('settings.graphics_low') },
    { value: 'Medium', label: t('settings.graphics_medium') },
    { value: 'High', label: t('settings.graphics_high') },
    { value: 'Ultra', label: t('settings.graphics_ultra') },
  ]);

  let effectsOptions = $derived([
    { value: 'full', label: i18n.locale === 'en' ? 'Full' : 'Полные' },
    { value: 'low', label: i18n.locale === 'en' ? 'Low' : 'Умеренные' },
    { value: 'off', label: i18n.locale === 'en' ? 'Off' : 'Отключены' },
  ]);

  let javaModeOptions = $derived([
    { value: 'auto', label: t('settings.java_auto') },
    { value: 'custom', label: t('settings.java_custom') },
  ]);

  const languageOptions = [
    { value: 'ru', label: 'Русский' },
    { value: 'en', label: 'English' },
  ];

  let appearanceOptions = $derived([
    { value: 'dark', label: t('settings.theme_dark') },
    { value: 'light', label: t('settings.theme_light') },
    { value: 'auto', label: t('settings.theme_auto') },
  ]);

  // Discord Rich Presence preview state (reflects actual backend activity)
  let discordPreview = $state<{
    is_enabled: boolean;
    is_connected: boolean;
    title: string;
    details: string;
    state: string;
    small_text: string | null;
    has_timestamp: boolean;
  }>({
    is_enabled: true,
    is_connected: false,
    title: 'Wex Launcher',
    details: 'В главном меню',
    state: 'Выбирает режим игры',
    small_text: null,
    has_timestamp: false,
  });

  async function updateDiscordPreview() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const prev = await invoke<{
        is_enabled: boolean;
        is_connected: boolean;
        title: string;
        details: string;
        state: string;
        small_text: string | null;
        has_timestamp: boolean;
      }>('get_discord_rpc_preview');
      if (prev) {
        discordPreview = prev;
      }
    } catch {
      // Offline / unit test fallback
    }
  }

  const accents: { key: AccentColor; title: string; color: string }[] = Object.entries(
    AccentPresets
  ).map(([key, val]) => ({
    key: key as AccentColor,
    title: val.title,
    color: theme.isDark ? val.dark : val.light,
  }));

  function formatRam(val: number): string {
    const gb = (val / 1024).toFixed(1);
    const unitGb = i18n.locale === 'en' ? 'GB' : 'ГБ';
    const unitMb = i18n.locale === 'en' ? 'MB' : 'МБ';
    return `${gb} ${unitGb} (${val} ${unitMb})`;
  }

  let ramHint = $derived.by(() => {
    const totalGb = Math.round(totalMemoryMb / 1024);
    if (i18n.locale === 'en') {
      if (ramMb <= 4096) {
        return `For Minecraft and lightweight mods · System memory: ${totalGb} GB`;
      } else if (ramMb <= 8192) {
        return `For Wex Pack and Iris shaders · System memory: ${totalGb} GB`;
      } else if (ramMb <= totalMemoryMb * 0.75) {
        return `For large resource packs and longer render distances · System memory: ${totalGb} GB`;
      } else {
        return `Using more than 75% of system memory may slow down your computer · Total: ${totalGb} GB`;
      }
    }
    if (ramMb <= 4096) {
      return `Рекомендуется для базовой игры и лёгких модов • Всего в системе: ${totalGb} ГБ`;
    } else if (ramMb <= 8192) {
      return `Оптимально для сборки Wex, шейдеров Iris и стабильного FPS • Всего в системе: ${totalGb} ГБ`;
    } else if (ramMb <= totalMemoryMb * 0.75) {
      return `Для ресурс-паков высокого разрешения и дальней прорисовки • Всего в системе: ${totalGb} ГБ`;
    } else {
      return `Внимание: выделение более 75% ОЗУ может замедлить систему • Всего в системе: ${totalGb} ГБ`;
    }
  });

  let activeJavaDisplay = $derived.by(() => {
    if (javaMode === 'custom') {
      return customJavaPath ? customJavaPath : (i18n.locale === 'en' ? 'Choose a Java installation.' : 'Путь к javaw.exe не указан');
    }
    const detected = selectedVersion === '1.20.1' ? detectedJava17 : detectedJava21;
    if (detected) {
      return detected;
    }
    const autoSuffix = i18n.locale === 'en' ? 'Automatic' : 'Автоматически';
    return selectedVersion === '1.20.1'
      ? `Mojang JRE 17 • ${autoSuffix}`
      : `Mojang JRE 21 • ${autoSuffix}`;
  });

  async function loadConfig() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const sys = await invoke<{ total_memory_mb: number; os_name: string; launcher_version: string; data_dir?: string }>('get_system_info');
      if (sys) {
        if (sys.total_memory_mb) totalMemoryMb = sys.total_memory_mb;
        if (sys.os_name) osName = sys.os_name;
        if (sys.launcher_version) launcherVersion = sys.launcher_version;
        if (sys.data_dir) gameDir = sys.data_dir;
      }

      const cfg = await invoke<{
        selected_version?: string;
        player_name?: string;
        ram_mb?: number;
        total_memory_mb?: number;
        game_dir?: string;
        graphics_preset?: string;
        java_mode?: string;
        custom_java_path?: string | null;
        detected_java_17?: string | null;
        detected_java_21?: string | null;
        discord_rpc?: boolean;
        fast_start_jvm?: boolean;
        friend_notifications?: boolean;
        language?: string;
        appearance?: string;
        accent?: string;
        reduce_motion?: boolean;
        reduce_transparency?: boolean;
        minimize_to_tray?: boolean;
        game_window_behavior?: string;
      }>('get_launcher_settings');

      if (cfg) {
        if (cfg.selected_version) selectedVersion = cfg.selected_version;
        if (cfg.player_name) playerName = cfg.player_name;
        if (cfg.ram_mb) ramMb = cfg.ram_mb;
        if (cfg.total_memory_mb) totalMemoryMb = cfg.total_memory_mb;
        if (cfg.game_dir) gameDir = cfg.game_dir;
        if (cfg.graphics_preset) graphicsPreset = cfg.graphics_preset;
        if (cfg.java_mode) javaMode = cfg.java_mode;
        if (cfg.custom_java_path) customJavaPath = cfg.custom_java_path;
        if (cfg.detected_java_17 !== undefined) detectedJava17 = cfg.detected_java_17;
        if (cfg.detected_java_21 !== undefined) detectedJava21 = cfg.detected_java_21;
        if (cfg.discord_rpc !== undefined) discordRpc = cfg.discord_rpc;
        if (cfg.fast_start_jvm !== undefined) fastStartJvm = cfg.fast_start_jvm;
        if (cfg.friend_notifications !== undefined) {
          friendNotifications = cfg.friend_notifications;
        }
        friendsLive.setNotificationsEnabled(friendNotifications);
        if (cfg.language === 'ru' || cfg.language === 'en') {
          language = cfg.language;
          i18n.setLocale(cfg.language);
        }
        if (cfg.appearance) theme.setAppearance(cfg.appearance as 'dark' | 'light' | 'auto');
        if (cfg.accent) theme.setAccent(cfg.accent as AccentColor);
        if (cfg.minimize_to_tray !== undefined) minimizeToTray = cfg.minimize_to_tray;
        if (cfg.game_window_behavior) gameWindowBehavior = cfg.game_window_behavior;
        theme.setReduceMotion(cfg.reduce_motion ?? false);
        theme.setReduceTransparency(cfg.reduce_transparency ?? false);
      }
      const musicSettings = await invoke<{ enabled?: boolean }>('music_get_settings');
      if (musicSettings && musicSettings.enabled !== undefined) {
        backgroundMusic = musicSettings.enabled;
      }
      await updateDiscordPreview();
    } catch {
      // In-browser mock fallback
    }
  }

  async function saveMusicSettings() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('music_set_settings', {
        settings: {
          enabled: backgroundMusic,
          volume: 75,
          auto_pause_on_launch: true,
        },
      });
    } catch {}
  }

  async function handleOpenDataDir() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('open_data_dir');
    } catch {
      // Browser fallback
    }
  }

  async function saveConfig() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_launcher_settings', {
        newSettings: {
          selected_version: selectedVersion,
          player_name: playerName,
          ram_mb: ramMb,
          total_memory_mb: totalMemoryMb,
          game_dir: gameDir,
          graphics_preset: graphicsPreset,
          java_mode: javaMode,
          custom_java_path: customJavaPath ? customJavaPath : null,
          detected_java_17: detectedJava17,
          detected_java_21: detectedJava21,
          discord_rpc: discordRpc,
          fast_start_jvm: fastStartJvm,
          friend_notifications: friendNotifications,
          language,
          appearance: theme.appearance,
          accent: theme.accent,
          reduce_motion: theme.reduceMotion,
          reduce_transparency: theme.reduceTransparency,
          minimize_to_tray: minimizeToTray,
          game_window_behavior: gameWindowBehavior,
        },
      });
    } catch {
      // Ignored in browser preview
    }
  }

  async function handleResetDefaults() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const reset = await invoke<{
        ram_mb?: number;
        graphics_preset?: string;
        java_mode?: string;
        custom_java_path?: string | null;
        discord_rpc?: boolean;
        fast_start_jvm?: boolean;
        friend_notifications?: boolean;
        language?: string;
        minimize_to_tray?: boolean;
      }>('reset_settings');

      if (reset) {
        if (reset.ram_mb) ramMb = reset.ram_mb;
        if (reset.graphics_preset) graphicsPreset = reset.graphics_preset;
        if (reset.java_mode) javaMode = reset.java_mode;
        customJavaPath = '';
        if (reset.discord_rpc !== undefined) discordRpc = reset.discord_rpc;
        if (reset.fast_start_jvm !== undefined) fastStartJvm = reset.fast_start_jvm;
        if (reset.friend_notifications !== undefined) {
          friendNotifications = reset.friend_notifications;
        } else {
          friendNotifications = true;
        }
        friendsLive.setNotificationsEnabled(friendNotifications);
        if (reset.language === 'ru' || reset.language === 'en') {
          language = reset.language;
          i18n.setLocale(reset.language);
        } else {
          language = 'ru';
          i18n.setLocale('ru');
        }
        if (reset.minimize_to_tray !== undefined) minimizeToTray = reset.minimize_to_tray;
      }
    } catch {
      // Browser reset fallback
      ramMb = 4096;
      graphicsPreset = 'High';
      javaMode = 'auto';
      customJavaPath = '';
      discordRpc = true;
      fastStartJvm = false;
      friendNotifications = true;
      friendsLive.setNotificationsEnabled(true);
      language = 'ru';
      minimizeToTray = true;
      i18n.setLocale('ru');
    }

    theme.setAppearance('dark');
    theme.setAccent('blue');
    theme.setReduceMotion(false);
    theme.setReduceTransparency(false);
    gameWindowBehavior = 'minimize';
    saveConfig();
  }

  function handleGameWindowBehaviorChange(val: string) {
    gameWindowBehavior = val;
    saveConfig();
  }

  function handleRamChange(newVal: number) {
    ramMb = newVal;
    saveConfig();
  }

  function handlePresetChange(val: string) {
    graphicsPreset = val;
    saveConfig();
  }

  function handleJavaModeChange(val: string) {
    javaMode = val;
    saveConfig();
  }

  function handleLanguageChange(val: string) {
    language = val;
    if (val === 'ru' || val === 'en') {
      i18n.setLocale(val as SupportedLocale);
    }
    saveConfig();
  }

  // Aliases for compatibility
  let isVerifyingFiles = $derived(isCheckingIntegrity);
  let verifyStatus = $derived(
    integrityDetail
      ? integrityStatus
        ? `${integrityStatus} — ${integrityDetail}`
        : integrityDetail
      : integrityStatus
  );
  let cleanCacheStatus = $derived(cleanupStatus);

  async function loadCacheInfo() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{ total_bytes: number; formatted_size: string }>('get_storage_cache_info');
      if (res && res.formatted_size) {
        cacheSizeFormatted = res.formatted_size;
      }
    } catch {
      cacheSizeFormatted = '0 Б';
    }
  }

  async function checkGameRunningState() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const running = await invoke<boolean>('is_game_running');
      ipcGameRunning = Boolean(running);
    } catch {
      // In-browser mock
    }
  }

  async function handleVerifyFiles() {
    if (isGameActive || isCheckingIntegrity) return;
    isCheckingIntegrity = true;
    integrityStatus = 'Проверка...';
    integrityDetail = 'Проверка файлов по манифесту...';

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const ver = selectedVersion || propVersion;
      let res: any;
      try {
        res = await invoke('verify_instance_files', { version: ver });
      } catch (err: any) {
        if (err?.message?.includes('манифест') || (typeof err === 'string' && err.includes('манифест'))) {
          throw err;
        }
        res = await invoke('verify_game_files', { version: ver });
      }

      if (res) {
        integrityStatus = res.status || 'Проверка завершена';
        integrityDetail = res.message || res.status;
      }
    } catch (err: any) {
      integrityStatus = 'Ошибка проверки';
      const msg = typeof err === 'string' ? err : (err?.message || 'Не удалось проверить файлы игры');
      integrityDetail = msg.startsWith('Ошибка проверки: ') ? msg : `Ошибка проверки: ${msg}`;
    } finally {
      isCheckingIntegrity = false;
    }
  }

  async function handleClearCache() {
    if (isGameActive || isCleaningCache) return;
    isCleaningCache = true;
    cleanupStatus = 'Очистка кэша и логов...';

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const ver = selectedVersion || propVersion;
      let res: any;
      try {
        res = await invoke('cleanup_cache', { version: ver });
      } catch (err: any) {
        if (err?.message?.includes('заблокирован') || (typeof err === 'string' && err.includes('заблокирован'))) {
          throw err;
        }
        res = await invoke('clear_storage_cache');
      }

      if (res) {
        cleanupStatus = res.message || res.status_text || 'Кэш очищен';
        await loadCacheInfo();
      }
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : (err?.message || 'Ошибка очистки');
      cleanupStatus = msg.startsWith('Ошибка очистки: ') ? msg : `Ошибка очистки: ${msg}`;
    } finally {
      isCleaningCache = false;
      setTimeout(() => {
        cleanupStatus = null;
      }, 5000);
    }
  }

  const handleCleanCache = handleClearCache;

  let prevOnline = network.isOnline;
  $effect(() => {
    const currentOnline = network.isOnline;
    if (currentOnline && !prevOnline) {
      loadConfig();
    }
    prevOnline = currentOnline;
  });

  onMount(() => {
    loadConfig();
    loadCacheInfo();
    checkGameRunningState();

    let unlistenVerify: (() => void) | undefined;
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen<{ stage?: string; downloaded?: number; total?: number; percentage?: number; message?: string }>(
          'storage-verify-progress',
          (event) => {
            const p = event.payload;
            if (p.stage) {
              integrityStatus = p.stage;
            } else if (p.downloaded !== undefined && p.total !== undefined) {
              integrityStatus = `Проверка ${p.downloaded}/${p.total} (${Math.round(p.percentage || 0)}%)`;
            }
          }
        ).then((fn) => {
          unlistenVerify = fn;
        });
      }).catch(() => {});
    }

    return () => {
      unlistenVerify?.();
    };
  });
</script>

<div class="settings-screen">
  <div class="settings-content">
    <!-- Honest Offline State Banner -->
    {#if !network.isOnline}
      <div class="settings-offline-banner" role="status" aria-live="polite">
        <div class="offline-banner-left">
          <Icon name="wifi-off" size={16} color="var(--color-system-orange)" />
          <div class="offline-banner-text">
            <span class="offline-banner-title">{t('offline.title')}</span>
            <span class="offline-banner-desc">
              {i18n.locale === 'en' ? 'Local launcher settings are active. Network sync will resume when online.' : 'Локальные параметры сохранены. Сетевая синхронизация возобновится при подключении.'}
            </span>
          </div>
        </div>
        <Button
          variant="secondary"
          size="small"
          onclick={() => network.check()}
          title="Проверить подключение к сети"
        >
          <Icon name="refresh" size={12} color="var(--color-secondary-label)" />
          <span>{t('offline.retry')}</span>
        </Button>
      </div>
    {/if}

    <!-- Performance & Memory -->
    <List header={t('settings.header_perf')}>
      <Row
        title={t('settings.ram_title')}
        subtitle={ramHint}
      >
        <div class="slider-wrapper">
          <Slider
            value={ramMb}
            min={1024}
            max={totalMemoryMb}
            step={512}
            formatValue={formatRam}
            onchange={handleRamChange}
            label={t('settings.ram_label')}
          />
        </div>
      </Row>

      <Row
        title={t('settings.graphics_title')}
        subtitle={t('settings.graphics_subtitle')}
      >
        {#snippet trailing()}
          <div style="width: 320px;">
            <SegmentedControl
              options={graphicsOptions}
              value={graphicsPreset}
              onchange={handlePresetChange}
            />
          </div>
        {/snippet}
      </Row>

      <Row
        title="Быстрый запуск игры"
        subtitle="Экспериментальный предварительный прогрев виртуальной машины Java (PrewarmGame)"
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Быстрый запуск игры"
            checked={fastStartJvm}
            onchange={(checked) => {
              fastStartJvm = checked;
              saveConfig();
            }}
          />
        {/snippet}
      </Row>
    </List>

    <!-- Java Runtime -->
    <List header={t('settings.header_java')}>
      <Row
        title={t('settings.java_choice_title')}
        subtitle={t('settings.java_choice_subtitle')}
      >
        {#snippet trailing()}
          <div style="width: 280px;">
            <SegmentedControl
              options={javaModeOptions}
              value={javaMode}
              onchange={handleJavaModeChange}
              size="small"
            />
          </div>
        {/snippet}
      </Row>

      <Row
        title={javaMode === 'auto' ? t('settings.java_used_title') : t('settings.java_path_title')}
        subtitle={activeJavaDisplay}
      >
        {#if javaMode === 'custom'}
          <div class="custom-java-input-wrap">
            <input
              type="text"
              class="apple-text-field"
              placeholder={t('settings.java_placeholder')}
              bind:value={customJavaPath}
              onchange={() => saveConfig()}
              aria-label={t('settings.java_path_title')}
            />
          </div>
        {/if}
      </Row>
    </List>

    <!-- Data Directory & Network -->
    <List header={t('settings.header_dirs')}>
      <Row
        title="Подключение к сети"
        subtitle={network.isOnline
          ? (i18n.locale === 'en' ? 'Online • Connected to network services' : 'В сети • Подключение к сетевым сервисам активно')
          : (i18n.locale === 'en' ? 'Offline • Working in offline mode' : 'Нет подключения • Работа в офлайн-режиме')}
      >
        {#snippet trailing()}
          <div class="network-status-badge" class:online={network.isOnline} class:offline={!network.isOnline}>
            <span class="status-dot"></span>
            <span>{network.isOnline ? (i18n.locale === 'en' ? 'Online' : 'В сети') : t('offline.title')}</span>
          </div>
        {/snippet}
      </Row>

      <Row
        title={t('settings.gamedir_title')}
        subtitle={gameDir || t('settings.gamedir_undefined')}
      >
        {#snippet trailing()}
          <div class="dir-actions">
            <Button
              variant="secondary"
              size="small"
              onclick={handleOpenDataDir}
              title={t('settings.gamedir_open_tooltip')}
            >
              <Icon name="folder" size={13} color="var(--color-secondary-label)" />
              <span>{t('settings.gamedir_open')}</span>
            </Button>
          </div>
        {/snippet}
      </Row>

      <Row
        title="Журнал событий и краш-дампы"
        subtitle="Просмотр latest.log, crash-reports и hs_err_pid для диагностики сбоев"
      >
        {#snippet trailing()}
          <div class="dir-actions">
            <Button
              variant="secondary"
              size="small"
              onclick={() => (logsSheetOpen = true)}
              title="Открыть лист журнала"
            >
              <Icon name="info" size={13} color="var(--color-secondary-label)" />
              <span data-testid="btn-open-logs-sheet">Журнал</span>
            </Button>
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Storage Section -->
    <List header="Хранилище">
      <Row
        title="Проверить файлы игры"
        subtitle={integrityDetail ? `${integrityStatus ? integrityStatus + ' — ' : ''}${integrityDetail}` : (integrityStatus || (isGameActive ? 'Недоступно во время работы игры' : 'Сверка хэшей установленной версии и игровых файлов с манифестом'))}
      >
        {#snippet trailing()}
          <div class="storage-action-wrap">
            {#if isCheckingIntegrity}
              <span class="storage-status-badge checking">Проверка...</span>
            {:else if integrityStatus}
              <span class="storage-status-badge result">{integrityStatus}</span>
            {/if}
            <Button
              variant="secondary"
              size="small"
              disabled={isGameActive || isCheckingIntegrity}
              onclick={handleVerifyFiles}
              title={isGameActive ? 'Недоступно во время работы игры' : 'Проверить целостность файлов игры'}
            >
              <span>{isCheckingIntegrity ? 'Проверка...' : 'Проверить'}</span>
            </Button>
          </div>
        {/snippet}
      </Row>

      <Row
        title="Очистить кэш"
        subtitle={cleanupStatus ? cleanupStatus : `Размер кэша: ${cacheSizeFormatted}. Удаляются временные файлы загрузок, логи старше 7 дней и отчёты об ошибках.`}
      >
        {#snippet trailing()}
          <div class="storage-action-wrap">
            {#if isCleaningCache}
              <span class="storage-status-badge checking">Очистка...</span>
            {:else if cleanupStatus}
              <span class="storage-status-badge success">Готово</span>
            {/if}
            <Button
              variant="secondary"
              size="small"
              disabled={isGameActive || isCleaningCache}
              onclick={handleClearCache}
              title={isGameActive ? 'Недоступно во время работы игры' : 'Очистить кэш лаунчера'}
            >
              <span>{isCleaningCache ? 'Очистка...' : 'Очистить'}</span>
            </Button>
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Window and Tray behavior (SPEC wex-window) -->
    <List header="Окно и трей">
      <Row
        title="Сворачивать в трей при запуске игры"
        subtitle="Скрывать окно лаунчера во время игры и возвращать его на экран после закрытия"
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Сворачивать в трей при запуске игры"
            checked={minimizeToTray}
            onchange={(checked) => {
              minimizeToTray = checked;
              saveConfig();
            }}
          />
        {/snippet}
      </Row>
    </List>

    <!-- Integrations & Localization -->
    <List header={t('settings.header_integrations')}>
      <Row
        title={t('settings.discord_title')}
        subtitle={t('settings.discord_subtitle')}
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Discord Rich Presence"
            checked={discordRpc}
            onchange={(checked) => {
              discordRpc = checked;
              saveConfig();
              updateDiscordPreview();
            }}
          />
        {/snippet}
      </Row>

      <Row
        title="Превью статуса Discord"
        subtitle={discordRpc ? "Фактическое состояние активности профиля в Discord" : "Трансляция активности выключена в настройках"}
      >
        <div class="discord-preview-wrap">
          {#if discordRpc}
            <div class="discord-card">
              <div class="discord-card-header">
                <span class="discord-card-tag">Активность в Discord</span>
              </div>
              <div class="discord-card-body">
                <div class="discord-avatar-wrap">
                  <div class="discord-avatar">
                    <Icon name="server" size={26} color="var(--color-system-blue)" />
                  </div>
                  <span
                    class="discord-status-dot"
                    class:online={discordPreview.is_connected}
                    title={discordPreview.is_connected ? "Подключено к Discord" : "Ожидание подключения"}
                  ></span>
                </div>
                <div class="discord-content">
                  <div class="discord-app-title">{discordPreview.title || 'Wex Launcher'}</div>
                  <div class="discord-detail-title">{discordPreview.details}</div>
                  <div class="discord-state-text">{discordPreview.state}</div>
                  {#if discordPreview.small_text}
                    <div class="discord-meta-text">{discordPreview.small_text}</div>
                  {/if}
                  {#if !discordPreview.is_connected}
                    <div class="discord-offline-hint">Discord не запущен или ожидается подключение</div>
                  {/if}
                </div>
              </div>
            </div>
          {:else}
            <div class="discord-disabled-card">
              <div class="discord-disabled-badge">
                <Icon name="close" size={14} color="var(--color-secondary-label)" />
              </div>
              <div class="discord-disabled-info">
                <div class="disabled-title">Статус отключён</div>
                <div class="disabled-desc">
                  Лаунчер не передаёт статус в Discord и не использует сокет Rich Presence.
                </div>
              </div>
            </div>
          {/if}
        </div>
      </Row>

      <Row
        title={i18n.locale === 'en' ? 'Background Music' : 'Фоновая музыка'}
        subtitle={i18n.locale === 'en' ? 'Play atmospheric soundtracks from Wex/music' : 'Воспроизведение саундтрека из папки Wex/music во время работы лаунчера'}
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel={i18n.locale === 'en' ? 'Background music' : 'Фоновая музыка'}
            checked={backgroundMusic}
            onchange={(checked) => {
              backgroundMusic = checked;
              saveMusicSettings();
            }}
          />
        {/snippet}
      </Row>

      <Row
        title={i18n.locale === 'en' ? 'Friend Notifications' : 'Уведомления о друзьях'}
        subtitle={i18n.locale === 'en' ? 'Notify when friends join the game or invite you to a lobby' : 'Оповещать, когда друзья заходят в игру или приглашают в лобби'}
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel={i18n.locale === 'en' ? 'Friend notifications' : 'Уведомления о друзьях'}
            checked={friendNotifications}
            onchange={(checked) => {
              friendNotifications = checked;
              friendsLive.setNotificationsEnabled(checked);
              saveConfig();
            }}
          />
        {/snippet}
      </Row>

      <Row
        title={t('settings.lang_title')}
        subtitle={t('settings.lang_subtitle')}
      >
        {#snippet trailing()}
          <div style="width: 260px;">
            <SegmentedControl
              options={languageOptions}
              value={language}
              ariaLabel="Язык интерфейса"
              onchange={handleLanguageChange}
              size="small"
            />
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Appearance & Theme Section -->
    <List header={t('settings.header_appearance')}>
      <Row title={t('settings.theme_title')} subtitle={t('settings.theme_subtitle')}>
        {#snippet trailing()}
          <div style="width: 280px;">
            <SegmentedControl
              options={appearanceOptions}
              value={theme.appearance}
              ariaLabel="Тема оформления"
              onchange={(v) => {
                theme.setAppearance(v as 'dark' | 'light' | 'auto');
                saveConfig();
              }}
            />
          </div>
        {/snippet}
      </Row>

      <Row title={t('settings.accent_title')} subtitle={t('settings.accent_subtitle')}>
        {#snippet trailing()}
          <div class="accent-picker" role="radiogroup" aria-label="Цветовой акцент">
            {#each accents as acc (acc.key)}
              <button
                type="button"
                class="accent-dot"
                class:active={theme.accent === acc.key}
                style="background-color: {acc.color};"
                title={acc.title}
                role="radio"
                aria-checked={theme.accent === acc.key}
                onclick={() => {
                  theme.setAccent(acc.key);
                  saveConfig();
                }}
                aria-label={acc.title}
              >
                {#if theme.accent === acc.key}
                  <span class="active-check">✓</span>
                {/if}
              </button>
            {/each}
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Window Behavior During Game (SPEC 1.2 / Apple HIG) -->
    <List header="Поведение окна">
      <Row
        title="Во время игры"
        subtitle="Действие с окном лаунчера после старта игры: оставить открытым, свернуть или спрятать в область уведомлений"
      >
        {#snippet trailing()}
          <div style="width: 320px;">
            <SegmentedControl
              options={gameWindowBehaviorOptions}
              value={gameWindowBehavior}
              ariaLabel="Во время игры"
              onchange={handleGameWindowBehaviorChange}
              size="small"
            />
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Accessibility / Универсальный доступ -->
    <List header={t('settings.header_a11y')}>
      <Row
        title={i18n.locale === 'en' ? 'Interface Effects' : 'Эффекты интерфейса'}
        subtitle={i18n.locale === 'en' ? 'Animation fidelity and spring motion' : 'Степень плавности и пружинных анимаций'}
      >
        {#snippet trailing()}
          <div style="width: 320px;">
            <SegmentedControl
              options={effectsOptions}
              value={theme.effectsMode}
              onchange={(val) => {
                theme.setEffectsMode(val as any);
                saveConfig();
              }}
            />
          </div>
        {/snippet}
      </Row>

      <Row
        title={t('settings.reduce_motion_title')}
        subtitle={t('settings.reduce_motion_subtitle')}
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Уменьшение движения"
            checked={theme.reduceMotion}
            onchange={(checked) => {
              theme.setReduceMotion(checked);
              saveConfig();
            }}
          />
        {/snippet}
      </Row>

      <Row
        title={t('settings.reduce_transparency_title')}
        subtitle={t('settings.reduce_transparency_subtitle')}
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Уменьшение прозрачности"
            checked={theme.reduceTransparency}
            onchange={(checked) => {
              theme.setReduceTransparency(checked);
              saveConfig();
            }}
          />
        {/snippet}
      </Row>
    </List>

    <!-- About Section -->
    <List header={t('settings.header_about')}>
      <Row
        title={t('settings.about_title')}
        subtitle={t('settings.about_subtitle', { version: launcherVersion, os: osName })}
        trailingText="Wex Project"
      />
      <Row
        title="Самообновление лаунчера"
        subtitle={updateStatusText || `Проверка релизов по latest.json`}
      >
        {#snippet trailing()}
          {#if updateChecking}
            <span class="updating-text">Проверка…</span>
          {:else if updateAvailable}
            <Button
              variant="primary"
              size="small"
              onclick={handleApplyUpdateSettings}
            >
              {canAutoUpdateSettings ? 'Обновить' : 'Скачать'}
            </Button>
          {:else}
            <Button
              variant="secondary"
              size="small"
              onclick={handleCheckUpdateSettings}
            >
              Проверить
            </Button>
          {/if}
        {/snippet}
      </Row>
      <Row
        title={t('settings.about_stack_title')}
        subtitle={t('settings.about_stack_subtitle')}
        trailingText="v1.0.0"
      />
    </List>

    <!-- First Launch Onboarding Section -->
    <List header={i18n.locale === 'en' ? 'First Launch & Setup' : 'Первоначальная настройка'}>
      <Row
        title={i18n.locale === 'en' ? 'Show welcome screen again' : 'Показать приветствие снова'}
        subtitle={i18n.locale === 'en' ? 'Run initial setup wizard, choose version, nickname, and migrate data' : 'Пройти мастер первого запуска, настройку ника, выбор версии и перенос данных'}
      >
        {#snippet trailing()}
          <Button
            variant="secondary"
            size="small"
            onclick={() => {
              if (typeof localStorage !== 'undefined') {
                localStorage.removeItem('wex_onboarding_completed');
              }
              if (typeof window !== 'undefined') {
                window.dispatchEvent(new CustomEvent('wex-show-onboarding'));
              }
              onopenonboarding?.();
            }}
          >
            <Icon name="refresh" size={13} color="var(--color-secondary-label)" />
            <span>{i18n.locale === 'en' ? 'Start' : 'Показать'}</span>
          </Button>
        {/snippet}
      </Row>
    </List>

    <!-- Reset to defaults -->
    <div class="actions-section">
      <Button
        variant="secondary"
        onclick={handleResetDefaults}
      >
        {t('settings.reset_defaults')}
      </Button>
    </div>
  </div>

  <LogsSheet bind:open={logsSheetOpen} version={selectedVersion} />
</div>

<style>
  .settings-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    max-width: 860px;
    width: 100%;
    gap: 8px;
  }

  .settings-offline-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    background: var(--color-material-regular, rgba(30, 30, 35, 0.75));
    border: 1px solid rgba(255, 159, 10, 0.3);
    border-radius: var(--radius-large, 14px);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    animation: fadeIn var(--motion-duration-modal-enter) var(--motion-curve-spring-system);
    margin-bottom: 8px;
  }

  .network-status-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 12px;
    font-size: var(--text-caption-1, 11px);
    font-weight: 500;
  }

  .network-status-badge.online {
    background: rgba(48, 209, 88, 0.12);
    color: var(--color-system-green);
  }

  .network-status-badge.offline {
    background: rgba(255, 159, 10, 0.12);
    color: var(--color-system-orange);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .slider-wrapper {
    width: 100%;
    padding: 10px 0 4px;
  }

  .custom-java-input-wrap {
    width: 100%;
    padding: 8px 0 4px;
  }

  .apple-text-field {
    width: 100%;
    height: 32px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-separator);
    background-color: var(--color-secondary-system-grouped-background);
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    outline: none;
    box-sizing: border-box;
    transition: border-color var(--motion-duration-short) ease;
  }

  .apple-text-field:focus {
    border-color: var(--color-system-blue);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.25);
  }

  .dir-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .storage-action-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .storage-status-badge {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background-color: var(--color-tertiary-system-fill);
    white-space: nowrap;
  }

  .storage-status-badge.checking {
    color: var(--color-system-blue);
  }

  .storage-status-badge.success {
    color: var(--color-system-green);
  }

  .storage-status-badge.result {
    color: var(--color-secondary-label);
  }

  .accent-picker {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .accent-dot {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform var(--motion-duration-short) ease;
    outline: none;
    padding: 0;
  }

  .accent-dot::after {
    content: '';
    position: absolute;
    inset: -10px;
  }

  .accent-dot:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: 3px;
  }

  .accent-dot:hover {
    transform: scale(1.15);
  }

  @media (prefers-reduced-motion: reduce) {
    .accent-dot:hover {
      transform: none !important;
    }
  }

  :global([data-reduce-motion="true"]) .accent-dot:hover {
    transform: none !important;
  }

  .accent-dot.active {
    box-shadow: 0 0 0 2px var(--color-system-background), 0 0 0 4px var(--color-system-blue);
  }

  .active-check {
    font-size: 11px;
    font-weight: 800;
    color: #ffffff;
    line-height: 1;
  }

  .actions-section {
    display: flex;
    padding-top: 8px;
    margin-bottom: 24px;
  }

  .discord-preview-wrap {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    margin-top: 6px;
  }

  .discord-card {
    background: var(--color-fill-secondary);
    border: 1px solid var(--color-separator);
    border-radius: 12px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .discord-card-header {
    display: flex;
    align-items: center;
  }

  .discord-card-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-tertiary-label);
  }

  .discord-card-body {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }

  .discord-avatar-wrap {
    position: relative;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
  }

  .discord-avatar {
    width: 48px;
    height: 48px;
    border-radius: 10px;
    background: var(--color-fill-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-separator);
  }

  .discord-status-dot {
    position: absolute;
    bottom: -2px;
    right: -2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-system-gray);
    border: 2px solid var(--color-window-bg);
  }

  .discord-status-dot.online {
    background: var(--color-system-green);
  }

  .discord-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .discord-app-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--color-label);
    line-height: 1.2;
  }

  .discord-detail-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-secondary-label);
    line-height: 1.3;
  }

  .discord-state-text {
    font-size: 12px;
    color: var(--color-secondary-label);
    line-height: 1.3;
  }

  .discord-meta-text {
    font-size: 11px;
    color: var(--color-secondary-label);
    margin-top: 2px;
  }

  .discord-offline-hint {
    font-size: 11px;
    color: var(--color-tertiary-label);
    margin-top: 2px;
  }

  .discord-disabled-card {
    background: var(--color-fill-tertiary);
    border: 1px dashed var(--color-separator);
    border-radius: 12px;
    padding: 14px 16px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .discord-disabled-badge {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--color-fill-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .discord-disabled-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .disabled-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-secondary-label);
  }

  .disabled-desc {
    font-size: 12px;
    color: var(--color-tertiary-label);
  }

  .updating-text {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
  }
</style>
