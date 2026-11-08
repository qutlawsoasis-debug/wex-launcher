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
  let language = $state('ru');
  let launcherVersion = $state('1.0.0');
  let osName = $state('windows');

  const graphicsOptions = [
    { value: 'Low', label: 'Низкий' },
    { value: 'Medium', label: 'Средний' },
    { value: 'High', label: 'Высокий' },
    { value: 'Ultra', label: 'Ультра' },
  ];

  const javaModeOptions = [
    { value: 'auto', label: 'Автоматически' },
    { value: 'custom', label: 'Свой путь' },
  ];

  const languageOptions = [
    { value: 'ru', label: 'Русский' },
    { value: 'en', label: 'English' },
  ];

  const appearanceOptions = [
    { value: 'dark', label: 'Тёмная' },
    { value: 'light', label: 'Светлая' },
    { value: 'auto', label: 'Авто' },
  ];

  // Discord Rich Presence preview state
  let discordPreviewMode = $state<'launcher' | 'installing' | 'playing' | 'server' | 'lobby'>('launcher');

  const discordPreviewOptions = [
    { value: 'launcher', label: 'В лаунчере' },
    { value: 'installing', label: 'Установка' },
    { value: 'playing', label: 'В игре' },
    { value: 'server', label: 'На сервере' },
    { value: 'lobby', label: 'В лобби' },
  ];

  const discordPreviewData = $derived.by(() => {
    switch (discordPreviewMode) {
      case 'launcher':
        return {
          title: 'В главном меню',
          state: 'Выбирает режим игры',
          badge: null,
          time: null,
        };
      case 'installing':
        return {
          title: `Установка Minecraft ${selectedVersion}`,
          state: 'Этап: Загрузка библиотек',
          badge: `Minecraft ${selectedVersion}`,
          time: null,
        };
      case 'playing':
        return {
          title: `В мире Minecraft (${selectedVersion})`,
          state: 'Сборка Wex • Модов: 142',
          badge: `Minecraft ${selectedVersion}`,
          time: '00:18:42 прошло',
        };
      case 'server':
        return {
          title: 'На сервере: mc.auramc.net',
          state: 'Сборка Wex • Выживание',
          badge: `Minecraft ${selectedVersion}`,
          time: '01:05:12 прошло',
        };
      case 'lobby':
        return {
          title: 'В лобби #WEX77',
          state: `Игроков: 4/8 • Хост: ${playerName || 'Player'}`,
          badge: 'P2P Multiplayer',
          time: null,
        };
    }
  });

  const accents: { key: AccentColor; title: string; color: string }[] = Object.entries(
    AccentPresets
  ).map(([key, val]) => ({
    key: key as AccentColor,
    title: val.title,
    color: theme.isDark ? val.dark : val.light,
  }));

  function formatRam(val: number): string {
    const gb = (val / 1024).toFixed(1);
    return `${gb} ГБ (${val} МБ)`;
  }

  let ramHint = $derived.by(() => {
    const totalGb = Math.round(totalMemoryMb / 1024);
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
      return customJavaPath ? customJavaPath : 'Путь к javaw.exe не указан';
    }
    const detected = selectedVersion === '1.20.1' ? detectedJava17 : detectedJava21;
    if (detected) {
      return detected;
    }
    return selectedVersion === '1.20.1'
      ? 'Mojang JRE 17 • Автоматически'
      : 'Mojang JRE 21 • Автоматически';
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
        language?: string;
        appearance?: string;
        accent?: string;
        reduce_motion?: boolean;
        reduce_transparency?: boolean;
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
        if (cfg.language) language = cfg.language;
        if (cfg.appearance) theme.setAppearance(cfg.appearance as 'dark' | 'light' | 'auto');
        if (cfg.accent) theme.setAccent(cfg.accent as AccentColor);
        theme.setReduceMotion(cfg.reduce_motion ?? false);
        theme.setReduceTransparency(cfg.reduce_transparency ?? false);
      }
    } catch {
      // In-browser mock fallback
    }
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
          language,
          appearance: theme.appearance,
          accent: theme.accent,
          reduce_motion: theme.reduceMotion,
          reduce_transparency: theme.reduceTransparency,
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
        language?: string;
      }>('reset_settings');

      if (reset) {
        if (reset.ram_mb) ramMb = reset.ram_mb;
        if (reset.graphics_preset) graphicsPreset = reset.graphics_preset;
        if (reset.java_mode) javaMode = reset.java_mode;
        customJavaPath = '';
        if (reset.discord_rpc !== undefined) discordRpc = reset.discord_rpc;
        if (reset.language) language = reset.language;
      }
    } catch {
      // Browser reset fallback
      ramMb = 4096;
      graphicsPreset = 'High';
      javaMode = 'auto';
      customJavaPath = '';
      discordRpc = true;
      language = 'ru';
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
    saveConfig();
  }

  onMount(() => {
    loadConfig();
  });
</script>

<div class="settings-screen">
  <div class="settings-content">
    <!-- Performance & Memory -->
    <List header="Производительность и память">
      <Row
        title="Выделение оперативной памяти"
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
            label="ОЗУ для Minecraft"
          />
        </div>
      </Row>

      <Row
        title="Графический пресет"
        subtitle="Оптимизация шейдеров Iris и настроек Sodium для вашей видеокарты"
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
    </List>

    <!-- Java Runtime -->
    <List header="Среда выполнения Java">
      <Row
        title="Выбор исполняемого файла Java"
        subtitle="1.20.1 требует Java 17, 1.21.1 требует Java 21"
      >
        {#snippet trailing()}
          <div style="width: 260px;">
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
        title={javaMode === 'auto' ? 'Используемый рантайм Java' : 'Путь к javaw.exe'}
        subtitle={activeJavaDisplay}
      >
        {#if javaMode === 'custom'}
          <div class="custom-java-input-wrap">
            <input
              type="text"
              class="apple-text-field"
              placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe"
              bind:value={customJavaPath}
              onchange={() => saveConfig()}
              aria-label="Путь к javaw.exe"
            />
          </div>
        {/if}
      </Row>
    </List>

    <!-- Data Directory & Network -->
    <List header="Папки и файлы данных">
      <Row
        title="Рабочая папка игры"
        subtitle={gameDir || 'Не определена'}
      >
        {#snippet trailing()}
          <div class="dir-actions">
            <Button
              variant="secondary"
              size="small"
              onclick={handleOpenDataDir}
              title="Открыть папку данных в Проводнике"
            >
              <Icon name="folder" size={13} color="var(--color-secondary-label)" />
              <span>Открыть</span>
            </Button>
          </div>
        {/snippet}
      </Row>
    </List>

    <!-- Integrations & Localization -->
    <List header="Интеграции и язык">
      <Row
        title="Discord Rich Presence"
        subtitle="Отображение текущей версии Wex Client, статуса игры и длительности сессии в Discord"
      >
        {#snippet trailing()}
          <Toggle
            ariaLabel="Discord Rich Presence"
            checked={discordRpc}
            onchange={(checked) => {
              discordRpc = checked;
              saveConfig();
            }}
          />
        {/snippet}
      </Row>

      <Row
        title="Превью статуса Discord"
        subtitle={discordRpc ? "Интерактивный предварительный просмотр для профиля Discord" : "Трансляция активности выключена в настройках"}
      >
        <div class="discord-preview-wrap">
          {#if discordRpc}
            <div class="discord-mode-nav">
              <SegmentedControl
                options={discordPreviewOptions}
                value={discordPreviewMode}
                onchange={(val) => {
                  discordPreviewMode = val as any;
                }}
              />
            </div>

            <div class="discord-card">
              <div class="discord-card-header">
                <span class="discord-card-tag">ИГРАЕТ В ИГРУ</span>
              </div>
              <div class="discord-card-body">
                <div class="discord-avatar-wrap">
                  <div class="discord-avatar">
                    <Icon name="server" size={26} color="var(--color-system-blue)" />
                  </div>
                  <span class="discord-status-dot" title="В сети"></span>
                </div>
                <div class="discord-content">
                  <div class="discord-app-title">Wex Launcher</div>
                  <div class="discord-detail-title">{discordPreviewData.title}</div>
                  <div class="discord-state-text">{discordPreviewData.state}</div>
                  {#if discordPreviewData.time}
                    <div class="discord-time-text">{discordPreviewData.time}</div>
                  {/if}
                </div>
              </div>
              <div class="discord-card-footer">
                <div class="discord-mock-btn">Скачать Wex Launcher</div>
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
        title="Язык интерфейса"
        subtitle="Локализация меню лаунчера и игровых подсказок"
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
    <List header="Оформление">
      <Row title="Тема оформления" subtitle="Тёмная, светлая или системная тема">
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

      <Row title="Цветовой акцент" subtitle="Основной цвет элементов интерфейса">
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
    <List header="Универсальный доступ">
      <Row
        title="Уменьшение движения"
        subtitle="Отключение пружинных анимаций и динамических сдвигов"
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
        title="Уменьшение прозрачности"
        subtitle="Замена эффектов размытия фона на контрастные непрозрачные заливки"
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
    <List header="О программе">
      <Row
        title="Лаунчер Wex"
        subtitle="Версия {launcherVersion} • ОС: {osName} • 64-бит"
        trailingText="Wex Project"
      />
      <Row
        title="Технологический стек"
        subtitle="Tauri 2 • Svelte 5 • TypeScript 5.8 • Rust (core/wex-core, wex-extras)"
        trailingText="v1.0.0"
      />
    </List>

    <!-- Reset to defaults -->
    <div class="actions-section">
      <Button
        variant="secondary"
        onclick={handleResetDefaults}
      >
        Сбросить настройки по умолчанию
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

  .discord-mode-nav {
    display: flex;
    width: 100%;
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
    font-weight: 700;
    letter-spacing: 0.5px;
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
    background: var(--color-system-green);
    border: 2px solid var(--color-window-bg);
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

  .discord-time-text {
    font-size: 11px;
    color: var(--color-tertiary-label);
    margin-top: 2px;
  }

  .discord-card-footer {
    display: flex;
    padding-top: 4px;
  }

  .discord-mock-btn {
    background: var(--color-fill-tertiary);
    border: 1px solid var(--color-separator);
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-label);
    text-align: center;
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
