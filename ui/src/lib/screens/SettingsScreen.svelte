<!--
  Wex Launcher — Screen «Настройки»
  Rules:
  - Appearance: Dark / Light / Auto (SegmentedControl)
  - Accent Color picker with WCAG contrast verification
  - RAM allocation (Slider)
  - Accessibility: Reduce Motion and Reduce Transparency toggles
  - Settings mapped directly to theme state
-->
<script lang="ts">
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Slider from '../components/Slider.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import Button from '../components/Button.svelte';
  import { theme } from '../design/theme.svelte';
  import { AccentPresets, type AccentColor } from '../design/colors';

  let ramAllocation = $state(4);
  let fullscreenOnLaunch = $state(false);
  let jvmArguments = $state('-XX:+UseG1GC -XX:+ParallelRefProcEnabled');

  const appearanceOptions = [
    { value: 'dark', label: 'Тёмная' },
    { value: 'light', label: 'Светлая' },
    { value: 'auto', label: 'Авто (Системная)' },
  ];

  const accents: { key: AccentColor; title: string; color: string }[] = Object.entries(
    AccentPresets
  ).map(([key, val]) => ({
    key: key as AccentColor,
    title: val.title,
    color: theme.isDark ? val.dark : val.light,
  }));
</script>

<div class="settings-screen">
  <div class="settings-content">
    <!-- Appearance & Theme Section -->
    <List header="Внешний вид и стиль Apple">
      <Row title="Тема оформления" subtitle="Выбор системного режима (Dark Mode / Light Mode)">
        {#snippet trailing()}
          <div style="width: 280px;">
            <SegmentedControl
              options={appearanceOptions}
              value={theme.appearance}
              onchange={(v) => theme.setAppearance(v as 'dark' | 'light' | 'auto')}
            />
          </div>
        {/snippet}
      </Row>

      <Row title="Цветовой акцент" subtitle="Основной цвет кнопок, ползунков и индикаторов">
        {#snippet trailing()}
          <div class="accent-picker">
            {#each accents as acc}
              <button
                type="button"
                class="accent-dot"
                class:active={theme.accent === acc.key}
                style="background-color: {acc.color};"
                title={acc.title}
                onclick={() => theme.setAccent(acc.key)}
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
    <List header="Универсальный доступ (Accessibility)">
      <Row
        title="Уменьшение движения (Reduce Motion)"
        subtitle="Отключение физики пружин и плавных сдвигов, мгновенные переходы"
      >
        {#snippet trailing()}
          <Toggle
            checked={theme.reduceMotion}
            onchange={(checked) => theme.setReduceMotion(checked)}
          />
        {/snippet}
      </Row>

      <Row
        title="Уменьшение прозрачности (Reduce Transparency)"
        subtitle="Замена эффектов размытия Liquid Glass на плотные системные заливки"
      >
        {#snippet trailing()}
          <Toggle
            checked={theme.reduceTransparency}
            onchange={(checked) => theme.setReduceTransparency(checked)}
          />
        {/snippet}
      </Row>
    </List>

    <!-- Java & Game Performance -->
    <List header="Производительность и память">
      <Row title="Выделение оперативной памяти" subtitle="Рекомендуется от 4 до 8 ГБ для Aura-Pack">
        <div style="width: 100%; padding-top: 10px;">
          <Slider
            bind:value={ramAllocation}
            min={2}
            max={16}
            step={1}
            unit="ГБ"
            label="Оперативная память (RAM)"
          />
        </div>
      </Row>

      <Row title="Полноэкранный режим" subtitle="Запускать Minecraft сразу на весь экран">
        {#snippet trailing()}
          <Toggle bind:checked={fullscreenOnLaunch} />
        {/snippet}
      </Row>

      <Row title="Параметры запуска JVM">
        <div style="width: 100%; padding-top: 8px;">
          <input
            type="text"
            bind:value={jvmArguments}
            class="jvm-input"
            placeholder="-XX:+UseG1GC"
          />
        </div>
      </Row>
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
          ramAllocation = 4;
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
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    max-width: 800px;
    width: 100%;
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

  .jvm-input {
    width: 100%;
    height: 36px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--glass-border);
    background-color: var(--color-quaternary-system-fill);
    color: var(--color-label);
    font-family: var(--font-mono);
    font-size: var(--text-footnote);
    outline: none;
  }

  .jvm-input:focus {
    border-color: var(--color-system-blue);
    background-color: var(--card-bg);
  }

  .actions-section {
    display: flex;
    padding-top: 8px;
    margin-bottom: 24px;
  }
</style>
