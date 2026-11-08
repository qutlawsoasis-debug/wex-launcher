<!--
  Wex Launcher — Screen «Настройки»
  Specification: tasks/apple/hig/settings.md, tasks/apple/hig/sliders.md, tasks/apple/hig/toggles.md
  Rules:
  - RAM allocation: slider with 512 MB step, upper bound based on system memory
  - Game data directory selector
  - Graphics preset: Low / Medium / High / Ultra
  - Appearance: Dark / Light / Auto & Accent picker
  - Accessibility: Reduce Motion and Reduce Transparency toggles
  - About Wex section
  - Bidirectional sync via get_launcher_config / save_launcher_config
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
  let totalMemoryMb = $state(16384);
  let ramMb = $state(4096);
  let gameDir = $state('C:\\Users\\Player\\AppData\\Roaming\\.minecraft');
  let graphicsPreset = $state('High');
  let launcherVersion = $state('1.0.0');
  let osName = $state('windows');

  const graphicsOptions = [
    { value: 'Low', label: 'Низкий' },
    { value: 'Medium', label: 'Средний' },
    { value: 'High', label: 'Высокий' },
    { value: 'Ultra', label: 'Ультра' },
  ];

  const appearanceOptions = [
    { value: 'dark', label: 'Тёмная' },
    { value: 'light', label: 'Светлая' },
    { value: 'auto', label: 'Авто' },
  ];

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

  async function loadConfig() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const sys = await invoke<{ total_memory_mb: number; os_name: string; launcher_version: string }>('get_system_info');
      if (sys && sys.total_memory_mb) {
        totalMemoryMb = sys.total_memory_mb;
        osName = sys.os_name;
        launcherVersion = sys.launcher_version;
      }

      const cfg = await invoke<{
        ram_mb: number;
        game_dir: string;
        graphics_preset: string;
        appearance: string;
        accent: string;
        reduce_motion: boolean;
        reduce_transparency: boolean;
      }>('get_launcher_config');

      if (cfg) {
        ramMb = cfg.ram_mb || 4096;
        if (cfg.game_dir) gameDir = cfg.game_dir;
        if (cfg.graphics_preset) graphicsPreset = cfg.graphics_preset;
        if (cfg.appearance) theme.setAppearance(cfg.appearance as 'dark' | 'light' | 'auto');
        if (cfg.accent) theme.setAccent(cfg.accent as AccentColor);
        theme.setReduceMotion(cfg.reduce_motion ?? false);
        theme.setReduceTransparency(cfg.reduce_transparency ?? false);
      }
    } catch {
      // In-browser mock fallback
    }
  }

  async function saveConfig() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_launcher_config', {
        newConfig: {
          selected_version: '1.21.1',
          player_name: 'Player',
          ram_mb: ramMb,
          game_dir: gameDir,
          graphics_preset: graphicsPreset,
          appearance: theme.appearance,
          accent: theme.accent,
          reduce_motion: theme.reduceMotion,
          reduce_transparency: theme.reduceTransparency,
        },
      });
    } catch {
      // Ignored
    }
  }

  function handleRamChange(newVal: number) {
    ramMb = newVal;
    saveConfig();
  }

  function handlePresetChange(val: string) {
    graphicsPreset = val;
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
        subtitle="Шаг 512 МБ • Верхняя граница ограничена памятью системы ({Math.round(totalMemoryMb / 1024)} ГБ)"
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
        subtitle="Настройки шейдеров Iris и оптимизатора Sodium"
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

    <!-- Data Directory -->
    <List header="Папки и файлы данных">
      <Row
        title="Рабочая папка игры"
        subtitle={gameDir}
      >
        {#snippet trailing()}
          <div class="dir-actions">
            <Button
              variant="secondary"
              size="small"
              title="Открыть папку данных в Проводнике"
            >
              <Icon name="restore" size={13} color="var(--color-secondary-label)" />
              <span>Открыть</span>
            </Button>
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
          <div class="accent-picker">
            {#each accents as acc}
              <button
                type="button"
                class="accent-dot"
                class:active={theme.accent === acc.key}
                style="background-color: {acc.color};"
                title={acc.title}
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
        subtitle="Версия {launcherVersion} • ОС: {osName} (64-bit)"
        trailingText="Aura Project"
      />
      <Row
        title="Технологический стек"
        subtitle="Tauri 2 • Svelte 5 • TypeScript 5.8 • Rust (core/wex-net)"
        trailingText="v1.0.0"
      />
    </List>

    <!-- Reset to defaults -->
    <div class="actions-section">
      <Button
        variant="secondary"
        onclick={() => {
          theme.setAppearance('dark');
          theme.setAccent('blue');
          theme.setReduceMotion(false);
          theme.setReduceTransparency(false);
          ramMb = 4096;
          graphicsPreset = 'High';
          saveConfig();
        }}
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

  .dir-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .accent-picker {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .accent-dot {
    width: 22px;
    height: 22px;
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

  .accent-dot:hover {
    transform: scale(1.15);
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
</style>
