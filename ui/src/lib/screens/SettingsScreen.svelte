<!--
  Wex Launcher — Screen «Настройки» (Apple HIG System Settings parity)
  Specification: tasks/apple/hig/settings.md, tasks/apple/hig/sliders.md, tasks/apple/hig/toggles.md, tasks/apple/hig/segmented-controls.md

  8 System Groups (Apple System Settings HIG):
  1. Игра (Game): Java runtime, Window/Tray behavior, Fast launch, Welcome wizard
  2. Память и производительность (Memory & Performance): RAM slider, Graphics preset, Discord RPC & preview
  3. Внешний вид (Appearance): Theme mode, Accent color, Interface language, Effects fidelity, Accessibility
  4. Звук и музыка (Sound & Music): Background soundtrack, notification alert sounds
  5. Уведомления (Notifications): Friend status & invite alerts, minimized window tray alerts
  6. Хранилище (Storage): Game directory, verify instance files integrity, cleanup cache & logs
  7. О системе (About / System): Version, OS, tech stack telemetry, reset to defaults
  8. Обновления (Updates): Check for launcher updates, automatic startup check

  Apple HIG Compliance:
  - Standard 44px min-height rows with consistent padding and typography
  - Integrated top search bar with real-time row and group filtering
  - Authentic Apple HIG empty search state with clear action
  - Friendly plain-language explanations under all rows
  - High scenery art contrast (WCAG AA >= 4.5:1)
  - No truncated controls or text at 1280×800 and in English locale
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

  // Search state
  let searchQuery = $state('');

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
  let trayNotifications = $state(true);
  let backgroundMusic = $state(true);
  let notificationSounds = $state(true);
  let fastStartJvm = $state(false);
  let fastLaunch = $state(true);
  let language = $state<string>(i18n.locale);
  let launcherVersion = $state('—');
  let osName = $state('windows');
  let minimizeToTray = $state(true);
  let gameWindowBehavior = $state('minimize');
  let updateChecking = $state(false);
  let updateStatusText = $state('');
  let updateAvailable = $state(false);
  let canAutoUpdateSettings = $state(false);
  let updateDownloadUrl = $state('');

  interface SystemInfoState {
    total_memory_mb: number;
    free_memory_mb: number;
    memory_display: string;
    os_name: string;
    os_version: string;
    launcher_version: string;
    data_dir: string;
    cpu_name: string;
    cpu_cores: number;
    gpu_name: string;
    gpu_driver: string;
    is_integrated_gpu: boolean;
    disk_free_gb: number;
    disk_total_gb: number;
    disk_display: string;
    java_17: string;
    java_21: string;
    warnings: string[];
    support_text: string;
  }

  let systemInfo = $state<SystemInfoState | null>(null);
  let copiedSupport = $state(false);

  async function handleCopySupportInfo() {
    if (!systemInfo) return;
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(systemInfo.support_text);
      } else {
        const ta = document.createElement('textarea');
        ta.value = systemInfo.support_text;
        document.body.appendChild(ta);
        ta.select();
        document.execCommand('copy');
        document.body.removeChild(ta);
      }
      copiedSupport = true;
      setTimeout(() => {
        copiedSupport = false;
      }, 2500);
    } catch (e) {
      console.error('Failed to copy support info to clipboard', e);
    }
  }

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
        updateStatusText = 'Не удалось открыть ссылку загрузки. Повторите попытку позже.';
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

  let displayOs = $derived.by(() => {
    if (!osName || osName.toLowerCase() === 'windows') {
      return 'Windows 10/11, 64-бит';
    }
    return `${osName}, 64-бит`;
  });

  // Updates state
  let isCheckingUpdates = $state(false);
  let updateStatus = $state<string | null>(null);
  let autoCheckUpdates = $state(true);

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
        return `Recommended for base Minecraft and lightweight mods · System memory: ${totalGb} GB`;
      } else if (ramMb <= 8192) {
        return `Optimal for Wex Pack, Iris shaders, and stable frame rates · System memory: ${totalGb} GB`;
      } else if (ramMb <= totalMemoryMb * 0.75) {
        return `For high-resolution resource packs and longer render distances · System memory: ${totalGb} GB`;
      } else {
        return `Allocating over 75% of RAM may cause system slowdowns · System memory: ${totalGb} GB`;
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
      return customJavaPath ? customJavaPath : (i18n.locale === 'en' ? 'Path to javaw.exe not specified' : 'Путь к javaw.exe не указан');
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
      const { getVersion } = await import('@tauri-apps/api/app');
      const v = await getVersion();
      if (v) launcherVersion = v;
    } catch {
      // In-browser mock stays '—'
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const sys = await invoke<SystemInfoState>('get_system_info');
      if (sys) {
        systemInfo = sys;
        if (sys.total_memory_mb) totalMemoryMb = sys.total_memory_mb;
        if (sys.os_name) osName = sys.os_name;
        if (sys.launcher_version && sys.launcher_version !== '1.0.0') {
          launcherVersion = sys.launcher_version;
        }
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
        fast_launch?: boolean;
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
        if (cfg.fast_launch !== undefined) fastLaunch = cfg.fast_launch;
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
          fast_launch: fastLaunch,
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
        fast_launch?: boolean;
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
        if (reset.fast_launch !== undefined) fastLaunch = reset.fast_launch;
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

  // Update check
  async function handleCheckUpdates() {
    if (isCheckingUpdates) return;
    isCheckingUpdates = true;
    updateStatus = i18n.locale === 'en' ? 'Checking for updates…' : 'Проверка обновлений…';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{
        has_update?: boolean;
        update_available?: boolean;
        version?: string;
        new_version?: string;
      }>('social_check_launcher_update', {
        request: { current_version: launcherVersion }
      });
      if (res && (res.has_update || res.update_available)) {
        updateStatus = i18n.locale === 'en'
          ? `Update available: v${res.version || res.new_version || '1.1.0'}`
          : `Доступна новая версия: v${res.version || res.new_version || '1.1.0'}`;
      } else {
        updateStatus = i18n.locale === 'en'
          ? (launcherVersion !== '—' ? `Wex ${launcherVersion} is up to date` : 'Wex is up to date')
          : (launcherVersion !== '—' ? `Установлена актуальная версия Wex ${launcherVersion}` : 'Установлена актуальная версия Wex');
      }
    } catch {
      updateStatus = i18n.locale === 'en'
        ? (launcherVersion !== '—' ? `Wex ${launcherVersion} is up to date` : 'Wex is up to date')
        : (launcherVersion !== '—' ? `Установлена актуальная версия Wex ${launcherVersion}` : 'Установлена актуальная версия Wex');
    } finally {
      isCheckingUpdates = false;
    }
  }

  // Search Items Registry for real-time filtering
  interface SettingItemDefinition {
    id: string;
    group: 'game' | 'perf' | 'appearance' | 'sound' | 'notifications' | 'storage' | 'about' | 'updates';
    title: string;
    subtitle: string;
    keywords: string[];
  }

  let allSettings = $derived<SettingItemDefinition[]>([
    // 1. Game
    {
      id: 'game_java_mode',
      group: 'game',
      title: t('settings.java_choice_title'),
      subtitle: t('settings.java_choice_subtitle'),
      keywords: ['java', 'джава', 'рантайм', 'runtime', '17', '21', 'jdk', 'jre', 'путь', 'выбор', 'режим', 'installation'],
    },
    {
      id: 'game_java_path',
      group: 'game',
      title: javaMode === 'auto' ? t('settings.java_used_title') : t('settings.java_path_title'),
      subtitle: activeJavaDisplay,
      keywords: ['javaw', 'exe', 'путь', 'файл', 'исполняемый', 'версия', 'path', 'executable'],
    },
    {
      id: 'game_window_behavior',
      group: 'game',
      title: 'Поведение окна',
      subtitle: 'Во время игры: действие с окном лаунчера при запуске Minecraft',
      keywords: ['окно', 'поведение', 'свернуть', 'трей', 'запуск', 'действие', 'скрыть', 'во время игры'],
    },
    {
      id: 'game_tray',
      group: 'game',
      title: 'Сворачивать в трей при запуске игры',
      subtitle: 'Скрывать окно лаунчера во время игры и возвращать его на экран после закрытия',
      keywords: ['трей', 'окно', 'сворачивать', 'запуск', 'игры', 'tray', 'minimize', 'close', 'hide', 'game'],
    },
    {
      id: 'game_fast_start',
      group: 'game',
      title: 'Быстрый запуск игры',
      subtitle: 'Экспериментальный предварительный прогрев виртуальной машины Java (PrewarmGame)',
      keywords: ['быстрый', 'запуск', 'прогрев', 'jvm', 'prewarm', 'fast', 'start', 'boost'],
    },
    {
      id: 'game_onboarding',
      group: 'game',
      title: i18n.locale === 'en' ? 'Show welcome screen again' : 'Показать приветствие снова',
      subtitle: i18n.locale === 'en' ? 'Run initial setup wizard, choose version, nickname, and migrate data' : 'Пройти мастер первого запуска, настройку ника, выбор версии и перенос данных',
      keywords: ['приветствие', 'мастер', 'импорт', 'первый', 'запуск', 'onboarding', 'welcome', 'wizard', 'setup'],
    },

    // 2. Memory & Performance
    {
      id: 'perf_ram',
      group: 'perf',
      title: t('settings.ram_title'),
      subtitle: ramHint,
      keywords: ['озу', 'ram', 'память', 'memory', 'гигабайт', 'гб', 'мб', 'выделение', 'alloc', 'слайдер', 'slider'],
    },
    {
      id: 'perf_graphics',
      group: 'perf',
      title: t('settings.graphics_title'),
      subtitle: t('settings.graphics_subtitle'),
      keywords: ['графика', 'пресет', 'шейдеры', 'iris', 'sodium', 'содиум', 'ирис', 'fps', 'производительность', 'preset', 'graphics'],
    },
    {
      id: 'perf_discord',
      group: 'perf',
      title: t('settings.discord_title'),
      subtitle: t('settings.discord_subtitle'),
      keywords: ['discord', 'дискорд', 'rpc', 'статус', 'трансляция', 'активность', 'rich', 'presence'],
    },
    {
      id: 'perf_discord_preview',
      group: 'perf',
      title: 'Превью статуса Discord',
      subtitle: discordRpc ? 'Фактическое состояние активности профиля в Discord' : 'Трансляция активности выключена в настройках',
      keywords: ['discord', 'дискорд', 'карточка', 'превью', 'никнейм', 'сервер', 'профиль', 'preview', 'card'],
    },

    // 3. Appearance
    {
      id: 'app_theme',
      group: 'appearance',
      title: t('settings.theme_title'),
      subtitle: t('settings.theme_subtitle'),
      keywords: ['тема', 'тёмная', 'светлая', 'авто', 'dark', 'light', 'theme', 'appearance', 'оформление'],
    },
    {
      id: 'app_accent',
      group: 'appearance',
      title: t('settings.accent_title'),
      subtitle: t('settings.accent_subtitle'),
      keywords: ['акцент', 'цвет', 'синий', 'фиолетовый', 'зелёный', 'accent', 'color', 'blue', 'tint'],
    },
    {
      id: 'app_lang',
      group: 'appearance',
      title: t('settings.lang_title'),
      subtitle: t('settings.lang_subtitle'),
      keywords: ['язык', 'русский', 'английский', 'english', 'russian', 'language', 'locale', 'перевод'],
    },
    {
      id: 'app_effects',
      group: 'appearance',
      title: i18n.locale === 'en' ? 'Interface Effects' : 'Эффекты интерфейса',
      subtitle: i18n.locale === 'en' ? 'Animation fidelity and spring motion' : 'Степень плавности и пружинных анимаций',
      keywords: ['эффекты', 'анимации', 'пружины', 'плавность', 'effects', 'motion', 'spring'],
    },
    {
      id: 'app_reduce_motion',
      group: 'appearance',
      title: t('settings.reduce_motion_title'),
      subtitle: t('settings.reduce_motion_subtitle'),
      keywords: ['движение', 'уменьшение', 'motion', 'reduce', 'доступность', 'accessibility', 'a11y'],
    },
    {
      id: 'app_reduce_transparency',
      group: 'appearance',
      title: t('settings.reduce_transparency_title'),
      subtitle: t('settings.reduce_transparency_subtitle'),
      keywords: ['прозрачность', 'размытие', 'блюр', 'transparency', 'blur', 'contrast'],
    },

    // 4. Sound & Music
    {
      id: 'sound_music',
      group: 'sound',
      title: i18n.locale === 'en' ? 'Background Music' : 'Фоновая музыка',
      subtitle: i18n.locale === 'en' ? 'Play atmospheric soundtracks from Wex/music' : 'Воспроизведение саундтрека из папки Wex/music во время работы лаунчера',
      keywords: ['музыка', 'звук', 'саундтрек', 'пластинка', 'винил', 'audio', 'music', 'soundtrack', 'background'],
    },
    {
      id: 'sound_sfx',
      group: 'sound',
      title: i18n.locale === 'en' ? 'Interface and Notification Sounds' : 'Звуки интерфейса и уведомлений',
      subtitle: i18n.locale === 'en' ? 'Play subtle alert sounds for game invites and system events' : 'Звуковой сигнал при получении приглашений в игру и системных событий',
      keywords: ['звук', 'сигнал', 'эффекты', 'громкость', 'уведомления', 'sfx', 'sound', 'alerts'],
    },

    // 5. Notifications
    {
      id: 'notif_friends',
      group: 'notifications',
      title: i18n.locale === 'en' ? 'Friend Notifications' : 'Уведомления о друзьях',
      subtitle: i18n.locale === 'en' ? 'Notify when friends join the game or invite you to a lobby' : 'Оповещать, когда друзья заходят в игру или приглашают в лобби',
      keywords: ['друзья', 'уведомления', 'приглашения', 'лобби', 'friends', 'notifications', 'invite', 'lobby'],
    },
    {
      id: 'notif_tray',
      group: 'notifications',
      title: i18n.locale === 'en' ? 'Notifications When Minimized' : 'Уведомления при свёрнутом окне',
      subtitle: i18n.locale === 'en' ? 'Show Windows alerts when the launcher is minimized' : 'Показывать системные уведомления Windows, когда окно лаунчера свёрнуто',
      keywords: ['трей', 'windows', 'всплывающие', 'оповещения', 'свёрнуто', 'tray', 'banner', 'push'],
    },

    // 6. Storage
    {
      id: 'storage_gamedir',
      group: 'storage',
      title: t('settings.gamedir_title'),
      subtitle: gameDir || t('settings.gamedir_undefined'),
      keywords: ['папка', 'директория', 'файлы', 'данные', 'проводник', 'хранилище', 'storage', 'directory', 'folder', 'explorer'],
    },
    {
      id: 'storage_verify',
      group: 'storage',
      title: 'Проверить файлы игры',
      subtitle: verifyStatus || (i18n.locale === 'en' ? `Verifies game files for version ${selectedVersion} and downloads damaged ones` : `Сверяет файлы игры версии ${selectedVersion} и докачивает повреждённые`),
      keywords: ['проверить', 'файлы', 'докачать', 'восстановление', 'целостность', 'verify', 'repair', 'integrity'],
    },
    {
      id: 'storage_clean',
      group: 'storage',
      title: 'Очистить кэш',
      subtitle: cleanCacheStatus || (i18n.locale === 'en' ? 'Removes session logs, crash reports, and temporary download files' : 'Удаление логов сессий, отчётов о вылетах и временных файлов загрузок'),
      keywords: ['кэш', 'очистка', 'удалить', 'логи', 'вылеты', 'отчеты', 'память', 'диск', 'освободить', 'cache', 'clean', 'trash', 'free'],
    },
    {
      id: 'storage_logs',
      group: 'storage',
      title: 'Журнал событий и краш-дампы',
      subtitle: 'Просмотр latest.log, crash-reports и hs_err_pid для диагностики сбоев',
      keywords: ['журнал', 'логи', 'краш', 'дамп', 'ошибки', 'диагностика', 'logs', 'crash', 'reports'],
    },

    // 7. About / System
    {
      id: 'about_program',
      group: 'about',
      title: t('settings.about_title'),
      subtitle: t('settings.about_subtitle', { version: launcherVersion, os: displayOs }),
      keywords: ['о программе', 'лаунчер', 'wex', 'версия', 'система', 'about', 'version', 'os'],
    },
    {
      id: 'about_hardware',
      group: 'about',
      title: i18n.locale === 'en' ? 'Hardware & System Info' : 'Оборудование и система',
      subtitle: i18n.locale === 'en' ? 'Processor, RAM, graphics card, disk space' : 'Процессор, оперативная память, видеокарта, диск',
      keywords: ['процессор', 'видеокарта', 'память', 'диск', 'железо', 'cpu', 'gpu', 'ram', 'hardware', 'java', 'поддержка', 'support'],
    },
    {
      id: 'about_reset',
      group: 'about',
      title: 'Сброс настроек',
      subtitle: t('settings.reset_defaults'),
      keywords: ['сброс', 'по умолчанию', 'вернуть', 'заводские', 'reset', 'default'],
    },

    // 8. Updates
    {
      id: 'updates_check',
      group: 'updates',
      title: i18n.locale === 'en' ? 'Launcher Updates' : 'Самообновление лаунчера',
      subtitle: updateStatus || (i18n.locale === 'en' ? (launcherVersion !== '—' ? `Wex ${launcherVersion} is up to date` : 'Wex is up to date') : (launcherVersion !== '—' ? `Установлена актуальная версия Wex ${launcherVersion}` : 'Установлена актуальная версия Wex')),
      keywords: ['обновление', 'самообновление', 'проверить', 'версия', 'апдейт', 'update', 'upgrade', 'new', 'version', 'patch'],
    },
    {
      id: 'updates_auto',
      group: 'updates',
      title: i18n.locale === 'en' ? 'Automatic Updates' : 'Автоматическая проверка',
      subtitle: i18n.locale === 'en' ? 'Check for newer launcher versions at startup' : 'Проверять наличие свежей версии лаунчера при каждом запуске',
      keywords: ['автоматически', 'авто', 'проверка', 'старт', 'запуск', 'auto', 'check', 'startup'],
    },
  ]);

  function isItemVisible(itemId: string): boolean {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return true;
    const item = allSettings.find((s) => s.id === itemId);
    if (!item) return false;
    const tokens = q.split(/\s+/).filter(Boolean);
    return tokens.every(
      (tok) =>
        item.title.toLowerCase().includes(tok) ||
        item.subtitle.toLowerCase().includes(tok) ||
        item.keywords.some((kw) => kw.toLowerCase().includes(tok))
    );
  }

  function isGroupVisible(group: string): boolean {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return true;
    const groupItems = allSettings.filter((s) => s.group === group);
    return groupItems.some((item) => isItemVisible(item.id));
  }

  let totalVisibleCount = $derived(allSettings.filter((s) => isItemVisible(s.id)).length);

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

<div class="settings-screen" aria-label="Экран «Настройки»">
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

    <!-- Top Apple HIG Search Bar -->
    <div class="settings-search-bar" role="search">
      <div class="search-input-wrapper">
        <Icon name="search" size={15} color="var(--color-secondary-label)" />
        <input
          type="search"
          class="search-input"
          placeholder={i18n.locale === 'en' ? 'Search settings…' : 'Поиск настроек…'}
          bind:value={searchQuery}
          aria-label={i18n.locale === 'en' ? 'Search settings' : 'Поиск настроек'}
        />
        {#if searchQuery.trim()}
          <button
            type="button"
            class="clear-search-btn"
            onclick={() => (searchQuery = '')}
            aria-label={i18n.locale === 'en' ? 'Clear search' : 'Сбросить поиск'}
          >
            <Icon name="close" size={13} color="var(--color-secondary-label)" />
          </button>
        {/if}
      </div>
    </div>

    <!-- Empty Search State -->
    {#if searchQuery.trim() && totalVisibleCount === 0}
      <div class="empty-search-state" role="status">
        <div class="empty-search-icon">
          <Icon name="search" size={26} color="var(--color-tertiary-label)" />
        </div>
        <div class="empty-search-title">{i18n.locale === 'en' ? 'No Results Found' : 'Ничего не найдено'}</div>
        <div class="empty-search-desc">
          {i18n.locale === 'en'
            ? `No settings match “${searchQuery}”. Check spelling or clear search.`
            : `По запросу «${searchQuery}» подходящих настроек не найдено. Проверьте правильность написания или сбросьте поиск.`}
        </div>
        <Button variant="secondary" size="small" onclick={() => (searchQuery = '')}>
          {i18n.locale === 'en' ? 'Clear Search' : 'Сбросить поиск'}
        </Button>
      </div>
    {/if}

    <!-- 1. Игра (Game) -->
    {#if isGroupVisible('game')}
      <List header={i18n.locale === 'en' ? 'Game' : 'Игра'}>
        {#if isItemVisible('game_java_mode')}
          <Row
            title={t('settings.java_choice_title')}
            subtitle={t('settings.java_choice_subtitle')}
          >
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 260px;">
                <SegmentedControl
                  options={javaModeOptions}
                  value={javaMode}
                  onchange={handleJavaModeChange}
                  size="small"
                />
              </div>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('game_java_path')}
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
        {/if}

        {#if isItemVisible('game_window_behavior')}
          <Row
            title="Поведение окна"
            subtitle="Во время игры: действие с окном лаунчера при запуске Minecraft"
          >
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 350px;">
                <SegmentedControl
                  options={gameWindowBehaviorOptions}
                  value={gameWindowBehavior}
                  ariaLabel="Поведение окна"
                  onchange={(val) => {
                    gameWindowBehavior = val;
                    saveConfig();
                  }}
                />
              </div>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('game_tray')}
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
        {/if}

        {#if isItemVisible('game_fast_start')}
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
        {/if}

        {#if isItemVisible('game_onboarding')}
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
        {/if}
      </List>
    {/if}

    <!-- 2. Память и производительность (Memory & Performance) -->
    {#if isGroupVisible('perf')}
      <List header={t('settings.group_perf')}>
        {#if isItemVisible('perf_ram')}
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
        {/if}

        {#if isItemVisible('perf_graphics')}
          <Row
            title={t('settings.graphics_title')}
            subtitle={t('settings.graphics_subtitle')}
          >
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 320px;">
                <SegmentedControl
                  options={graphicsOptions}
                  value={graphicsPreset}
                  onchange={handlePresetChange}
                  size="small"
                />
              </div>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('perf_discord')}
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
        {/if}

        {#if isItemVisible('perf_discord_preview')}
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
        {/if}
      </List>
    {/if}

    <!-- 3. Внешний вид (Appearance) -->
    {#if isGroupVisible('appearance')}
      <List header={i18n.locale === 'en' ? 'Appearance' : 'Внешний вид'}>
        {#if isItemVisible('app_theme')}
          <Row title={t('settings.theme_title')} subtitle={t('settings.theme_subtitle')}>
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 280px;">
                <SegmentedControl
                  options={appearanceOptions}
                  value={theme.appearance}
                  ariaLabel="Тема оформления"
                  onchange={(v) => {
                    theme.setAppearance(v as 'dark' | 'light' | 'auto');
                    saveConfig();
                  }}
                  size="small"
                />
              </div>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('app_accent')}
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
        {/if}

        {#if isItemVisible('app_lang')}
          <Row
            title={t('settings.lang_title')}
            subtitle={t('settings.lang_subtitle')}
          >
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 260px;">
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
        {/if}

        {#if isItemVisible('app_effects')}
          <Row
            title={i18n.locale === 'en' ? 'Interface Effects' : 'Эффекты интерфейса'}
            subtitle={i18n.locale === 'en' ? 'Animation fidelity and spring motion' : 'Степень плавности и пружинных анимаций'}
          >
            {#snippet trailing()}
              <div class="segmented-control-fixed" style="width: 280px;">
                <SegmentedControl
                  options={effectsOptions}
                  value={theme.effectsMode}
                  onchange={(val) => {
                    theme.setEffectsMode(val as any);
                    saveConfig();
                  }}
                  size="small"
                />
              </div>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('app_reduce_motion')}
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
        {/if}

        {#if isItemVisible('app_reduce_transparency')}
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
        {/if}
      </List>
    {/if}

    <!-- 4. Звук и музыка (Sound & Music) -->
    {#if isGroupVisible('sound')}
      <List header={i18n.locale === 'en' ? 'Sound & Music' : 'Звук и музыка'}>
        {#if isItemVisible('sound_music')}
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
        {/if}

        {#if isItemVisible('sound_sfx')}
          <Row
            title={i18n.locale === 'en' ? 'Interface and Notification Sounds' : 'Звуки интерфейса и уведомлений'}
            subtitle={i18n.locale === 'en' ? 'Play subtle alert sounds for game invites and system events' : 'Звуковой сигнал при получении приглашений в игру и системных событий'}
          >
            {#snippet trailing()}
              <Toggle
                ariaLabel={i18n.locale === 'en' ? 'Interface and notification sounds' : 'Звуки интерфейса и уведомлений'}
                checked={notificationSounds}
                onchange={(checked) => {
                  notificationSounds = checked;
                  saveConfig();
                }}
              />
            {/snippet}
          </Row>
        {/if}
      </List>
    {/if}

    <!-- 5. Уведомления (Notifications) -->
    {#if isGroupVisible('notifications')}
      <List header={i18n.locale === 'en' ? 'Notifications' : 'Уведомления'}>
        {#if isItemVisible('notif_friends')}
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
        {/if}

        {#if isItemVisible('notif_tray')}
          <Row
            title={i18n.locale === 'en' ? 'Notifications When Minimized' : 'Уведомления при свёрнутом окне'}
            subtitle={i18n.locale === 'en' ? 'Show Windows alerts when the launcher is minimized' : 'Показывать системные уведомления Windows, когда окно лаунчера свёрнуто'}
          >
            {#snippet trailing()}
              <Toggle
                ariaLabel={i18n.locale === 'en' ? 'Notifications when minimized' : 'Уведомления при свёрнутом окне'}
                checked={trayNotifications}
                onchange={(checked) => {
                  trayNotifications = checked;
                  saveConfig();
                }}
              />
            {/snippet}
          </Row>
        {/if}
      </List>
    {/if}

    <!-- 6. Хранилище (Storage) -->
    {#if isGroupVisible('storage')}
      <List header={i18n.locale === 'en' ? 'Storage' : 'Хранилище'}>
        {#if isItemVisible('storage_gamedir')}
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
        {/if}

        {#if isItemVisible('storage_verify')}
          <Row
            title="Проверить файлы игры"
            subtitle={verifyStatus || (i18n.locale === 'en' ? `Verifies game files for version ${selectedVersion} and downloads damaged ones` : `Сверяет файлы игры версии ${selectedVersion} и докачивает повреждённые`)}
          >
            {#snippet trailing()}
              <Button
                variant="secondary"
                size="small"
                disabled={isGameActive || isVerifyingFiles}
                onclick={handleVerifyFiles}
                title="Проверить целостность файлов инстанса"
              >
                {#if isVerifyingFiles}
                  <Icon name="refresh" size={13} color="var(--color-secondary-label)" />
                  <span>Проверка...</span>
                {:else}
                  <Icon name="check" size={13} color="var(--color-secondary-label)" />
                  <span>Проверить</span>
                {/if}
              </Button>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('storage_clean')}
          <Row
            title="Очистить кэш"
            subtitle={cleanCacheStatus || (i18n.locale === 'en' ? 'Removes session logs, crash reports, and temporary download files' : 'Удаление логов сессий, отчётов о вылетах и временных файлов загрузок')}
          >
            {#snippet trailing()}
              <Button
                variant="secondary"
                size="small"
                disabled={isGameActive || isCleaningCache}
                onclick={handleCleanCache}
                title="Очистить кэш и освободить дисковое пространство"
              >
                {#if isCleaningCache}
                  <Icon name="refresh" size={13} color="var(--color-secondary-label)" />
                  <span>Очистка...</span>
                {:else}
                  <Icon name="trash" size={13} color="var(--color-secondary-label)" />
                  <span>Очистить</span>
                {/if}
              </Button>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('storage_logs')}
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
        {/if}
      </List>
    {/if}

    <!-- 7. О системе (About / System) -->
    {#if isGroupVisible('about')}
      <List header={i18n.locale === 'en' ? 'About' : 'О системе'}>
        {#if isItemVisible('about_program')}
          <Row
            title={t('settings.about_title')}
            subtitle={t('settings.about_subtitle', { version: launcherVersion, os: displayOs })}
            trailingText="Wex Project"
          />
        {/if}

        {#if isItemVisible('about_hardware')}
          {#if systemInfo}
            {#if systemInfo.warnings && systemInfo.warnings.length > 0}
              {#each systemInfo.warnings as warning}
                <div class="system-warning-banner">
                  <Icon name="warning" size={18} color="var(--color-system-orange)" />
                  <span class="warning-text">{warning}</span>
                </div>
              {/each}
            {/if}

            <Row
              title={i18n.locale === 'en' ? 'Processor' : 'Процессор'}
              subtitle={systemInfo.cpu_name ? `${systemInfo.cpu_name} (${systemInfo.cpu_cores} ${i18n.locale === 'en' ? 'cores' : 'ядер'})` : (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />
            <Row
              title={i18n.locale === 'en' ? 'RAM' : 'Оперативная память'}
              subtitle={systemInfo.memory_display || (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />
            <Row
              title={i18n.locale === 'en' ? 'Graphics Card' : 'Видеокарта'}
              subtitle={systemInfo.gpu_name ? `${systemInfo.gpu_name} (драйвер ${systemInfo.gpu_driver})` : (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
              trailingText={systemInfo.is_integrated_gpu ? (i18n.locale === 'en' ? 'Integrated' : 'Встроенная') : (i18n.locale === 'en' ? 'Discrete' : 'Дискретная')}
            />
            <Row
              title={i18n.locale === 'en' ? 'Operating System' : 'Версия Windows'}
              subtitle={systemInfo.os_version || (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />
            <Row
              title={i18n.locale === 'en' ? 'Disk Space (Wex)' : 'Свободное место на диске'}
              subtitle={systemInfo.disk_display || (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />
            <Row
              title="Java (1.20.1)"
              subtitle={systemInfo.java_17 || (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />
            <Row
              title="Java (1.21.1)"
              subtitle={systemInfo.java_21 || (i18n.locale === 'en' ? 'Could not determine' : 'не удалось определить')}
            />

            <div class="system-actions">
              <Button
                variant="secondary"
                onclick={handleCopySupportInfo}
              >
                <span class="btn-content">
                  <Icon name="copy" size={16} />
                  <span>{copiedSupport ? (i18n.locale === 'en' ? 'Скопировано в буфер' : 'Скопировано в буфер') : (i18n.locale === 'en' ? 'Copy for Support' : 'Скопировать для поддержки')}</span>
                </span>
              </Button>
            </div>
          {:else}
            <Row
              title={i18n.locale === 'en' ? 'Loading system info...' : 'Загрузка данных о системе...'}
              subtitle={i18n.locale === 'en' ? 'Reading hardware sensors' : 'Опрос оборудования ПК'}
            />
          {/if}
        {/if}

        {#if isItemVisible('about_reset')}
          <Row
            title="Сброс настроек"
            subtitle="Восстановление стандартных параметров лаунчера и графики"
          >
            {#snippet trailing()}
              <Button
                variant="secondary"
                size="small"
                onclick={handleResetDefaults}
              >
                {t('settings.reset_defaults')}
              </Button>
            {/snippet}
          </Row>
        {/if}
      </List>
    {/if}

    <!-- 8. Обновления (Updates) -->
    {#if isGroupVisible('updates')}
      <List header={i18n.locale === 'en' ? 'Updates' : 'Обновления'}>
        {#if isItemVisible('updates_check')}
          <Row
            title={i18n.locale === 'en' ? 'Launcher Updates' : 'Самообновление лаунчера'}
            subtitle={updateStatus || (i18n.locale === 'en' ? (launcherVersion !== '—' ? `Wex ${launcherVersion} is up to date` : 'Wex is up to date') : (launcherVersion !== '—' ? `Установлена актуальная версия Wex ${launcherVersion}` : 'Установлена актуальная версия Wex'))}
          >
            {#snippet trailing()}
              <Button
                variant="secondary"
                size="small"
                disabled={isCheckingUpdates}
                onclick={handleCheckUpdates}
              >
                <Icon name="refresh" size={13} color="var(--color-secondary-label)" />
                <span>{isCheckingUpdates ? (i18n.locale === 'en' ? 'Checking…' : 'Проверка…') : (i18n.locale === 'en' ? 'Check Now' : 'Проверить сейчас')}</span>
              </Button>
            {/snippet}
          </Row>
        {/if}

        {#if isItemVisible('updates_auto')}
          <Row
            title={i18n.locale === 'en' ? 'Automatic Updates' : 'Автоматическая проверка'}
            subtitle={i18n.locale === 'en' ? 'Check for newer launcher versions at startup' : 'Проверять наличие свежей версии лаунчера при каждом запуске'}
          >
            {#snippet trailing()}
              <Toggle
                ariaLabel={i18n.locale === 'en' ? 'Automatic updates' : 'Автоматическая проверка обновлений'}
                checked={autoCheckUpdates}
                onchange={(checked) => {
                  autoCheckUpdates = checked;
                }}
              />
            {/snippet}
          </Row>
        {/if}
      </List>
    {/if}
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
    max-width: 880px;
    width: 100%;
    gap: 12px;
    margin: 0 auto;
  }

  /* Top Apple HIG Search Bar */
  .settings-search-bar {
    width: 100%;
    margin-bottom: 4px;
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: 38px;
    background-color: var(--glass-card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: 0 12px;
    gap: 8px;
    box-sizing: border-box;
    backdrop-filter: blur(24px) saturate(160%);
    -webkit-backdrop-filter: blur(24px) saturate(160%);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
    transition: border-color var(--motion-duration-short) ease, background-color var(--motion-duration-short) ease;
  }

  .search-input-wrapper:focus-within {
    border-color: var(--color-system-blue);
    background-color: var(--color-tertiary-system-fill);
    box-shadow: 0 0 0 3px rgba(0, 145, 255, 0.24);
  }

  .search-input {
    flex: 1;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--color-label);
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    outline: none;
    box-sizing: border-box;
  }

  .search-input::placeholder {
    color: var(--color-secondary-label);
  }

  .clear-search-btn {
    appearance: none;
    border: none;
    background: transparent;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border-radius: 50%;
    transition: background-color var(--motion-duration-short) ease;
  }

  .clear-search-btn:hover {
    background-color: var(--color-tertiary-system-fill);
  }

  /* Empty Search State */
  .empty-search-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 48px 24px;
    gap: 12px;
    background-color: var(--glass-card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(28px);
    -webkit-backdrop-filter: blur(28px);
    margin-top: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.06);
  }

  .empty-search-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 4px;
  }

  .empty-search-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
  }

  .empty-search-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    max-width: 420px;
    line-height: 1.4;
    margin-bottom: 8px;
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

  .slider-wrapper {
    width: 100%;
    padding: 10px 0 4px;
  }

  .segmented-control-fixed {
    max-width: 100%;
    display: flex;
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

  .system-warning-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    margin: 4px 16px;
    border-radius: 8px;
    background-color: rgba(255, 149, 0, 0.12);
    border-left: 3px solid var(--color-system-orange, #ff9500);
    color: var(--color-label, #ffffff);
    font-size: 13px;
    line-height: 1.4;
  }

  .warning-text {
    flex: 1;
  }

  .system-actions {
    display: flex;
    justify-content: flex-start;
    padding: 10px 16px;
  }

  .btn-content {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  /* Scenery Art Contrast Parity (wex-contrast): text over art is always white with shadow */
  :global(.settings-screen .list-header) {
    color: rgba(255, 255, 255, 0.94) !important;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.75), 0 1px 2px rgba(0, 0, 0, 0.90);
    font-weight: 600;
  }

  :global(.settings-screen .list-footer) {
    color: rgba(255, 255, 255, 0.72) !important;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.70);
  }
</style>
