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
  import { theme } from '../design/theme.svelte';
  import { AccentPresets, type AccentColor } from '../design/colors';
  import { i18n, t, type SupportedLocale } from '../i18n';
  import { friendsLive } from '../services/friendsLive';

  // System & configuration state
  let selectedVersion = $state('1.21.1');
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

  // Maintenance: Verify instance files & Cleanup cache
  let isVerifyingFiles = $state(false);
  let verifyStatus = $state<string | null>(null);
  let isCleaningCache = $state(false);
  let cleanCacheStatus = $state<string | null>(null);

  async function handleVerifyFiles() {
    if (isVerifyingFiles) return;
    isVerifyingFiles = true;
    verifyStatus = 'Проверка файлов по манифесту...';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{
        success: boolean;
        total_files: number;
        verified_files: number;
        repaired_files: number;
        corrupted_files: number;
        errors: string[];
        message: string;
      }>('verify_instance_files', { version: selectedVersion });

      if (res) {
        verifyStatus = res.message;
      } else {
        verifyStatus = 'Проверка завершена';
      }
    } catch (e: any) {
      verifyStatus = `Ошибка проверки: ${e?.message || e || 'неизвестная ошибка'}`;
    } finally {
      isVerifyingFiles = false;
    }
  }

  async function handleCleanCache() {
    if (isCleaningCache) return;
    isCleaningCache = true;
    cleanCacheStatus = 'Очистка кэша и логов...';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{
        success: boolean;
        freed_bytes: number;
        freed_formatted: string;
        deleted_files_count: number;
        message: string;
      }>('cleanup_cache', { version: selectedVersion });

      if (res) {
        cleanCacheStatus = res.message;
      } else {
        cleanCacheStatus = 'Очистка завершена';
      }
    } catch (e: any) {
      cleanCacheStatus = `Ошибка очистки: ${e?.message || e || 'неизвестная ошибка'}`;
    } finally {
      isCleaningCache = false;
    }
  }

  onMount(() => {
    loadConfig();
  });
</script>

<div class="settings-screen">
  <div class="settings-content">
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
        title="Проверить файлы игры"
        subtitle={verifyStatus || `Проверка SHA-256 файлов клиента версии ${selectedVersion} по манифесту пака и восстановление`}
      >
        {#snippet trailing()}
          <Button
            variant="secondary"
            size="small"
            disabled={isVerifyingFiles}
            onclick={handleVerifyFiles}
            title="Проверить SHA-256 файлов инстанса"
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

      <Row
        title="Очистить кэш"
        subtitle={cleanCacheStatus || 'Удаление логов сессий, crash-reports и временных файлов загрузок'}
      >
        {#snippet trailing()}
          <Button
            variant="secondary"
            size="small"
            disabled={isCleaningCache}
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
        title={t('settings.about_stack_title')}
        subtitle={t('settings.about_stack_subtitle')}
        trailingText="v1.0.0"
      />
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
</style>
