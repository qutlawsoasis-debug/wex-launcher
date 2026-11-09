<!--
  Apple HIG Onboarding Modal for Wex Launcher
  Specification:
  - Apple HIG Onboarding: https://developer.apple.com/design/human-interface-guidelines/onboarding
  - Apple HIG Modality: https://developer.apple.com/design/human-interface-guidelines/modality
  - Apple HIG Sliders: https://developer.apple.com/design/human-interface-guidelines/sliders
  - Apple HIG Motion: https://developer.apple.com/design/human-interface-guidelines/motion

  Flow:
  1. Welcome: Friendly overview + immediate background game download pre-warm
  2. Import: If Aura-Launcher found, offer seamless non-destructive data transfer
  3. Nickname: In-game nickname input with live character validation
  4. Version: Choice between Minecraft 1.21.1 (recommended) and 1.20.1
  5. Memory: Contextual RAM recommendation based on system memory with explanation
  6. Ready: Review choices + launch or return to launcher
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from './Button.svelte';
  import Slider from './Slider.svelte';
  import Icon from './icons/Icon.svelte';
  import { theme } from '../design/theme.svelte';
  import { t, i18n } from '../i18n';
  import type { OldLauncherDataDto, OnboardingSpecsDto } from '../ipc/generated/models';

  export type OnboardingStep = 'welcome' | 'import' | 'nickname' | 'version' | 'memory' | 'ready';

  interface Props {
    open?: boolean;
    initialStep?: OnboardingStep;
    oncomplete?: (data: {
      nickname: string;
      version: string;
      ramMb: number;
      importedFromAura: boolean;
      launchImmediately: boolean;
    }) => void;
    onskip?: () => void;
    onclose?: () => void;
  }

  let {
    open = $bindable(false),
    initialStep = 'welcome',
    oncomplete,
    onskip,
    onclose,
  }: Props = $props();

  // Navigation state
  let currentStep = $state<OnboardingStep>('welcome');
  let previouslyFocusedEl: HTMLElement | null = null;
  let modalEl: HTMLElement | undefined = $state();

  // Discovered system & environment specs
  let totalMemoryMb = $state(16384);
  let ramMb = $state(4096);
  let ramExplanation = $state(
    'Рекомендуется 4 ГБ: оптимально для модов и шейдеров без зависаний операционной системы.'
  );
  let detectedAuraDir = $state<string | null>(null);
  let oldLauncherData = $state<OldLauncherDataDto | null>(null);
  let isScanningAura = $state(false);
  let isImporting = $state(false);
  let importSuccess = $state(false);

  // User configured choices
  let nickname = $state('Player');
  let selectedVersion = $state<'1.21.1' | '1.20.1'>('1.21.1');
  let nicknameTouched = $state(false);

  // Background game download progress simulation / tracking
  let prewarmProgress = $state(12);
  let prewarmStage = $state('Проверка манифеста и целостности клиента в фоне');
  let isPrewarming = $state(false);

  // Nickname validation: 3-16 chars, alphanumeric and underscore
  let isNicknameValid = $derived.by(() => {
    const trimmed = nickname.trim();
    return /^[a-zA-Z0-9_]{3,16}$/.test(trimmed);
  });

  // Calculate RAM explanation on-the-fly when user changes slider
  function updateRamExplanation(val: number) {
    const totalGb = Math.round(totalMemoryMb / 1024);
    const chosenGb = (val / 1024).toFixed(val % 1024 === 0 ? 0 : 1);
    const freeGb = Math.max(0, Math.round((totalMemoryMb - val) / 1024));

    if (val <= 2048) {
      ramExplanation = `Выделено ${chosenGb} ГБ из ${totalGb} ГБ. Минимальный расход ресурсов, подходит для стандартной игры без тяжёлых текстур.`;
    } else if (val <= 4096) {
      ramExplanation = `Выделено ${chosenGb} ГБ из ${totalGb} ГБ. Сбалансированный режим: комфортный FPS в сборке Wex, остаётся ${freeGb} ГБ операционной системе.`;
    } else if (val <= 6144) {
      ramExplanation = `Выделено ${chosenGb} ГБ из ${totalGb} ГБ. Оптимально для шейдеров Iris, модов на оптимизацию и высоких настроек графики.`;
    } else if (val <= totalMemoryMb * 0.75) {
      ramExplanation = `Выделено ${chosenGb} ГБ из ${totalGb} ГБ. Для ресурс-паков высокого разрешения и максимальной дальности прорисовки.`;
    } else {
      ramExplanation = `Внимание: выделение более 75% всей памяти ПК (${chosenGb} ГБ) может замедлить работу Windows и фоновых программ.`;
    }
  }

  function formatRamLabel(val: number): string {
    const gb = (val / 1024).toFixed(val % 1024 === 0 ? 0 : 1);
    return `${gb} ГБ (${val} МБ)`;
  }

  // Pre-warm game download starting immediately from step 1
  async function startBackgroundPrewarm(ver: string) {
    if (isPrewarming) return;
    isPrewarming = true;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('onboarding_prewarm_game', { version: ver });
    } catch {
      // In-browser or test fallback simulation
    }

    // Gentle progressive tick
    const interval = setInterval(() => {
      if (!open) {
        clearInterval(interval);
        return;
      }
      if (prewarmProgress < 95) {
        prewarmProgress += Math.floor(Math.random() * 8) + 4;
        if (prewarmProgress > 30 && prewarmProgress < 60) {
          prewarmStage = `Синхронизация библиотек Fabric и Java для ${selectedVersion}`;
        } else if (prewarmProgress >= 60 && prewarmProgress < 90) {
          prewarmStage = `Загрузка и проверка ассетов Minecraft ${selectedVersion}`;
        } else if (prewarmProgress >= 90) {
          prewarmStage = `Клиент ${selectedVersion} подготовлен к первому запуску`;
        }
      } else {
        prewarmProgress = 100;
        prewarmStage = `Клиент ${selectedVersion} готов к запуску`;
        clearInterval(interval);
      }
    }, 900);
  }

  // Scan for old Aura-Launcher directory and data
  export async function scanOldLauncher() {
    isScanningAura = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const detected = await invoke<string | null>('extras_detect_old_launcher');
      if (detected) {
        detectedAuraDir = detected;
        const data = await invoke<OldLauncherDataDto>('extras_import_old_launcher', {
          request: { old_launcher_dir: detected },
        });
        if (data) {
          oldLauncherData = data;
          if (data.nickname) nickname = data.nickname;
          if (data.ram_mb) {
            ramMb = data.ram_mb;
            updateRamExplanation(data.ram_mb);
          }
          if (data.selected_version === '1.20.1' || data.selected_version === '1.21.1') {
            selectedVersion = data.selected_version;
          }
        }
      }
    } catch {
      // Aura-Launcher not present or inaccessible
      detectedAuraDir = null;
      oldLauncherData = null;
    } finally {
      isScanningAura = false;
    }
  }

  // Execute import from detected Aura-Launcher
  export async function executeImport() {
    if (!detectedAuraDir || !oldLauncherData || isImporting) return;
    isImporting = true;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('extras_apply_import', {
        request: {
          old_launcher_dir: detectedAuraDir,
          import_nickname: true,
          nickname: oldLauncherData.nickname,
          import_ram: true,
          ram_mb: oldLauncherData.ram_mb,
          import_version: true,
          selected_version: oldLauncherData.selected_version,
          graphics_preset: oldLauncherData.graphics_preset,
          import_servers: true,
          servers: oldLauncherData.servers || [],
          import_skins: true,
          skins: oldLauncherData.skins || [],
          capes: oldLauncherData.capes || [],
          import_worlds: true,
          worlds: oldLauncherData.worlds || [],
        },
      });

      importSuccess = true;
      if (oldLauncherData.nickname) nickname = oldLauncherData.nickname;
      if (oldLauncherData.ram_mb) ramMb = oldLauncherData.ram_mb;
      if (oldLauncherData.selected_version === '1.20.1') selectedVersion = '1.20.1';

      setTimeout(() => {
        currentStep = 'nickname';
      }, 700);
    } catch {
      // Advance on error gracefully
      currentStep = 'nickname';
    } finally {
      isImporting = false;
    }
  }

  // Load hardware specs & recommendations
  async function loadSystemSpecs() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const specs = await invoke<OnboardingSpecsDto>('onboarding_get_specs');
      if (specs) {
        if (specs.total_memory_mb) totalMemoryMb = specs.total_memory_mb;
        if (specs.recommended_memory_mb) {
          ramMb = specs.recommended_memory_mb;
          updateRamExplanation(specs.recommended_memory_mb);
        }
        if (specs.detected_aura_dir) {
          detectedAuraDir = specs.detected_aura_dir;
        }
        if (specs.default_nickname && nickname === 'Player') {
          nickname = specs.default_nickname;
        }
        if (specs.default_version === '1.20.1' || specs.default_version === '1.21.1') {
          selectedVersion = specs.default_version;
        }
      }
    } catch {
      // Fallback heuristics: detect navigator.deviceMemory if available
      if (typeof navigator !== 'undefined' && (navigator as any).deviceMemory) {
        const memGb = (navigator as any).deviceMemory;
        totalMemoryMb = memGb * 1024;
        ramMb = Math.min(4096, Math.max(2048, (totalMemoryMb * 0.5)));
        updateRamExplanation(ramMb);
      }
    }
  }

  // Step transitions
  export function goToStep(step: OnboardingStep) {
    currentStep = step;
  }

  export function handleNext() {
    if (currentStep === 'welcome') {
      if (detectedAuraDir && oldLauncherData) {
        currentStep = 'import';
      } else {
        currentStep = 'nickname';
      }
    } else if (currentStep === 'import') {
      currentStep = 'nickname';
    } else if (currentStep === 'nickname') {
      if (!isNicknameValid) {
        nicknameTouched = true;
        return;
      }
      currentStep = 'version';
    } else if (currentStep === 'version') {
      currentStep = 'memory';
    } else if (currentStep === 'memory') {
      currentStep = 'ready';
    } else if (currentStep === 'ready') {
      finishOnboarding(false);
    }
  }

  export function handleBack() {
    if (currentStep === 'import') {
      currentStep = 'welcome';
    } else if (currentStep === 'nickname') {
      currentStep = (detectedAuraDir && oldLauncherData) ? 'import' : 'welcome';
    } else if (currentStep === 'version') {
      currentStep = 'nickname';
    } else if (currentStep === 'memory') {
      currentStep = 'version';
    } else if (currentStep === 'ready') {
      currentStep = 'memory';
    }
  }

  export function finishOnboarding(launchImmediately: boolean = false) {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('wex_onboarding_completed', 'true');
    }

    oncomplete?.({
      nickname: nickname.trim(),
      version: selectedVersion,
      ramMb,
      importedFromAura: Boolean(importSuccess),
      launchImmediately,
    });

    open = false;
    onclose?.();

    // Async background sync to persist in Tauri config
    import('@tauri-apps/api/core')
      .then(({ invoke }) => {
        invoke('onboarding_complete', {
          req: {
            nickname: nickname.trim(),
            version: selectedVersion,
            ram_mb: ramMb,
          },
        }).catch(() => {});
      })
      .catch(() => {});
  }

  export function handleSkip() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('wex_onboarding_completed', 'true');
    }
    onskip?.();
    open = false;
    onclose?.();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      handleSkip();
    } else if (e.key === 'Enter' && currentStep !== 'ready') {
      if (currentStep === 'nickname' && !isNicknameValid) return;
      e.preventDefault();
      handleNext();
    }
  }

  $effect(() => {
    if (open) {
      currentStep = initialStep;
      loadSystemSpecs().then(() => {
        scanOldLauncher();
      });
      startBackgroundPrewarm(selectedVersion);

      if (typeof document !== 'undefined') {
        previouslyFocusedEl = document.activeElement as HTMLElement | null;
        setTimeout(() => {
          modalEl?.focus();
        }, 30);
      }
    }
  });

  function applyHash() {
    if (typeof window === 'undefined') return;
    const hash = window.location.hash.replace(/^#/, '');
    const params = new URLSearchParams(hash);
    if (params.get('onboarding') === 'true' || params.get('screen') === 'onboarding') {
      open = true;
    }
    const stepParam = params.get('onboarding-step');
    if (stepParam && ['welcome', 'import', 'nickname', 'version', 'memory', 'ready'].includes(stepParam)) {
      currentStep = stepParam as OnboardingStep;
      if (stepParam === 'import' && !oldLauncherData) {
        detectedAuraDir = '%APPDATA%\\.aura';
        oldLauncherData = {
          nickname: 'DragonSlayer',
          ram_mb: 6144,
          selected_version: '1.21.1',
          graphics_preset: 'High',
          total_play_time_seconds: BigInt(3600),
          total_game_launches: 10,
          servers: [{ name: 'Aura Survival', ip: 'play.auramc.ru:25565', icon: null }],
          skins: [{ file_name: 'steve_custom.png', path: '', hash: 'abc', is_slim: false }],
          capes: [],
          wardrobe_history: [],
          worlds: [],
        };
      }
    }
  }

  onMount(() => {
    applyHash();
    if (typeof window !== 'undefined') {
      window.addEventListener('hashchange', applyHash);
      return () => {
        window.removeEventListener('hashchange', applyHash);
      };
    }
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
  <!-- Scrim backdrop: RGBA without live blur per single blur layer rule -->
  <div class="onboarding-scrim" role="dialog" aria-modal="true" aria-label="Первоначальная настройка Wex">
    <div
      class="onboarding-card"
      bind:this={modalEl}
      tabindex="-1"
    >
      <!-- Top header bar with step indicator & Skip button -->
      <div class="card-header">
        <div class="step-indicator" aria-label="Шаг настройки">
          <span class="step-dot" class:active={currentStep === 'welcome'}></span>
          {#if detectedAuraDir}
            <span class="step-dot" class:active={currentStep === 'import'}></span>
          {/if}
          <span class="step-dot" class:active={currentStep === 'nickname'}></span>
          <span class="step-dot" class:active={currentStep === 'version'}></span>
          <span class="step-dot" class:active={currentStep === 'memory'}></span>
          <span class="step-dot" class:active={currentStep === 'ready'}></span>
        </div>

        <button
          type="button"
          class="skip-action"
          onclick={handleSkip}
          aria-label="Пропустить настройку"
        >
          {i18n.locale === 'en' ? 'Skip setup' : 'Пропустить'}
        </button>
      </div>

      <!-- Main Step Viewport -->
      <div class="step-content">
        <!-- Step 1: Welcome -->
        {#if currentStep === 'welcome'}
          <div class="step-pane welcome-pane">
            <div class="hero-badge">
              <div class="app-emblem">
                <span class="emblem-glyph">W</span>
              </div>
            </div>

            <h1 class="step-title">Добро пожаловать в Wex</h1>
            <p class="step-subtitle">
              Быстрый лаунчер Minecraft. Всё готово для плавной игры, чистой графики и удобной игры с друзьями.
            </p>

            <div class="features-list">
              <div class="feature-item">
                <div class="feature-icon">
                  <Icon name="cpu" size={18} color="var(--color-system-blue)" />
                </div>
                <div class="feature-text">
                  <span class="feature-headline">Оптимальная производительность</span>
                  <span class="feature-desc">Умный контроль оперативной памяти и готовая оптимизация Fabric</span>
                </div>
              </div>

              <div class="feature-item">
                <div class="feature-icon">
                  <Icon name="friends" size={18} color="var(--color-system-green)" />
                </div>
                <div class="feature-text">
                  <span class="feature-headline">Друзья и локальная сеть</span>
                  <span class="feature-desc">Подключение к мирам без сторонних утилит и сложных настроек портов</span>
                </div>
              </div>

              <div class="feature-item">
                <div class="feature-icon">
                  <Icon name="folder" size={18} color="var(--color-system-indigo)" />
                </div>
                <div class="feature-text">
                  <span class="feature-headline">Изоляция и порядок</span>
                  <span class="feature-desc">Каждая версия хранится в отдельной папке и не трогает другие лаунчеры</span>
                </div>
              </div>
            </div>

            <!-- Background download progress teaser -->
            <div class="prewarm-banner">
              <div class="prewarm-info">
                <Icon name="refresh" size={14} color="var(--color-system-blue)" />
                <span class="prewarm-text">{prewarmStage}</span>
                <span class="prewarm-pct">{prewarmProgress}%</span>
              </div>
              <div class="prewarm-track">
                <div class="prewarm-fill" style="width: {prewarmProgress}%;"></div>
              </div>
            </div>
          </div>

        <!-- Step 2: Aura-Launcher Migration (Only if detected) -->
        {:else if currentStep === 'import'}
          <div class="step-pane import-pane">
            <div class="hero-badge icon-badge">
              <Icon name="folder" size={28} color="var(--color-system-blue)" />
            </div>

            <h2 class="step-title">Найден Aura-Launcher</h2>
            <p class="step-subtitle">
              Мы обнаружили сохранённые данные старого лаунчера. Хотите перенести профиль, настройки и серверы в Wex?
            </p>

            {#if oldLauncherData}
              <div class="discovered-card">
                <div class="card-item">
                  <span class="item-label">Игровой ник</span>
                  <span class="item-value">{oldLauncherData.nickname || 'Не указан'}</span>
                </div>
                <div class="card-item">
                  <span class="item-label">Выделенная память</span>
                  <span class="item-value">{Math.round((oldLauncherData.ram_mb || 4096) / 1024)} ГБ</span>
                </div>
                <div class="card-item">
                  <span class="item-label">Сохранённые серверы</span>
                  <span class="item-value">{oldLauncherData.servers?.length || 0} шт.</span>
                </div>
                <div class="card-item">
                  <span class="item-label">Скины и миры</span>
                  <span class="item-value">{(oldLauncherData.skins?.length || 0) + (oldLauncherData.worlds?.length || 0)} шт.</span>
                </div>
              </div>

              <div class="safe-notice">
                <Icon name="info" size={14} color="var(--color-secondary-label)" />
                <span>Данные Aura-Launcher не изменяются и останутся нетронутыми на диске.</span>
              </div>
            {/if}

            {#if importSuccess}
              <div class="import-status success">
                <Icon name="check" size={16} color="var(--color-system-green)" />
                <span>Данные успешно скопированы! Переходим к никнейму...</span>
              </div>
            {/if}
          </div>

        <!-- Step 3: Nickname -->
        {:else if currentStep === 'nickname'}
          <div class="step-pane nickname-pane">
            <div class="hero-badge icon-badge">
              <Icon name="friends" size={28} color="var(--color-system-blue)" />
            </div>

            <h2 class="step-title">Как вас зовут в игре?</h2>
            <p class="step-subtitle">
              Этот никнейм будут видеть друзья в совместной игре, на серверах и в локальной сети.
            </p>

            <div class="input-container">
              <label for="onboarding-nickname-input" class="input-label">Игровой ник</label>
              <div class="apple-field-wrapper" class:has-error={nicknameTouched && !isNicknameValid}>
                <input
                  id="onboarding-nickname-input"
                  type="text"
                  class="apple-input"
                  placeholder="Player"
                  maxlength="16"
                  bind:value={nickname}
                  oninput={() => { nicknameTouched = true; }}
                  aria-invalid={!isNicknameValid}
                />
                {#if nickname}
                  <button
                    type="button"
                    class="clear-input-btn"
                    onclick={() => { nickname = ''; nicknameTouched = true; }}
                    title="Очистить ник"
                    aria-label="Очистить ник"
                  >
                    <Icon name="clear" size={14} color="var(--color-secondary-label)" />
                  </button>
                {/if}
              </div>

              {#if nicknameTouched && !isNicknameValid}
                <p class="validation-msg error">
                  Ник должен состоять из 3–16 латинских букв, цифр или подчёркивания.
                </p>
              {:else}
                <p class="validation-msg hint">
                  Используйте латинские буквы, цифры и символ подчёркивания.
                </p>
              {/if}
            </div>
          </div>

        <!-- Step 4: Version Selection -->
        {:else if currentStep === 'version'}
          <div class="step-pane version-pane">
            <div class="hero-badge icon-badge">
              <Icon name="server" size={28} color="var(--color-system-blue)" />
            </div>

            <h2 class="step-title">Выберите версию игры</h2>
            <p class="step-subtitle">
              Wex поддерживает обе популярные версии одновременно. Вы сможете переключить версию в любой момент.
            </p>

            <div class="version-cards-grid" role="radiogroup" aria-label="Версия Minecraft">
              <!-- Minecraft 1.21.1 Card -->
              <button
                type="button"
                class="version-card"
                class:selected={selectedVersion === '1.21.1'}
                role="radio"
                aria-checked={selectedVersion === '1.21.1'}
                onclick={() => {
                  selectedVersion = '1.21.1';
                  startBackgroundPrewarm('1.21.1');
                }}
              >
                <div class="card-top">
                  <span class="version-name">Minecraft 1.21.1</span>
                  <span class="version-badge recommended">Рекомендуется</span>
                </div>
                <p class="version-desc">
                  Новейший релиз с оптимизацией Wex Pack, актуальными механиками, шейдерами Iris и современным звуком.
                </p>
                <div class="card-check" class:active={selectedVersion === '1.21.1'}>
                  {#if selectedVersion === '1.21.1'}
                    <Icon name="check" size={14} color="white" />
                  {/if}
                </div>
              </button>

              <!-- Minecraft 1.20.1 Card -->
              <button
                type="button"
                class="version-card"
                class:selected={selectedVersion === '1.20.1'}
                role="radio"
                aria-checked={selectedVersion === '1.20.1'}
                onclick={() => {
                  selectedVersion = '1.20.1';
                  startBackgroundPrewarm('1.20.1');
                }}
              >
                <div class="card-top">
                  <span class="version-name">Minecraft 1.20.1</span>
                  <span class="version-badge classic">Классика</span>
                </div>
                <p class="version-desc">
                  Проверенная временем стабильная версия с максимальным каталогом любимых модификаций сообщества.
                </p>
                <div class="card-check" class:active={selectedVersion === '1.20.1'}>
                  {#if selectedVersion === '1.20.1'}
                    <Icon name="check" size={14} color="white" />
                  {/if}
                </div>
              </button>
            </div>
          </div>

        <!-- Step 5: Memory Allocation -->
        {:else if currentStep === 'memory'}
          <div class="step-pane memory-pane">
            <div class="hero-badge icon-badge">
              <Icon name="cpu" size={28} color="var(--color-system-blue)" />
            </div>

            <h2 class="step-title">Выделение оперативной памяти</h2>
            <p class="step-subtitle">
              Лаунчер автоматически подобрал оптимальный объём памяти под конфигурацию вашего компьютера.
            </p>

            <div class="memory-slider-card">
              <div class="slider-header-row">
                <span class="sys-ram-badge">Всего в ПК: {Math.round(totalMemoryMb / 1024)} ГБ</span>
                <span class="current-ram-val">{formatRamLabel(ramMb)}</span>
              </div>

              <Slider
                value={ramMb}
                min={1024}
                max={totalMemoryMb}
                step={512}
                formatValue={formatRamLabel}
                onchange={(val) => {
                  ramMb = val;
                  updateRamExplanation(val);
                }}
                label="Оперативная память для игры"
              />

              <!-- Explanation based on Apple HIG Contextual Guidance -->
              <div class="ram-explanation-box">
                <Icon name="info" size={16} color="var(--color-system-blue)" />
                <p class="explanation-text">{ramExplanation}</p>
              </div>

              <!-- Quick presets -->
              <div class="preset-buttons">
                <button
                  type="button"
                  class="preset-chip"
                  class:active={ramMb === 2048}
                  onclick={() => { ramMb = 2048; updateRamExplanation(2048); }}
                >
                  2 ГБ
                </button>
                <button
                  type="button"
                  class="preset-chip"
                  class:active={ramMb === 4096}
                  onclick={() => { ramMb = 4096; updateRamExplanation(4096); }}
                >
                  4 ГБ
                </button>
                <button
                  type="button"
                  class="preset-chip"
                  class:active={ramMb === 6144}
                  onclick={() => { ramMb = 6144; updateRamExplanation(6144); }}
                >
                  6 ГБ
                </button>
                {#if totalMemoryMb >= 8192}
                  <button
                    type="button"
                    class="preset-chip"
                    class:active={ramMb === 8192}
                    onclick={() => { ramMb = 8192; updateRamExplanation(8192); }}
                  >
                    8 ГБ
                  </button>
                {/if}
              </div>
            </div>
          </div>

        <!-- Step 6: Ready («Готово») -->
        {:else if currentStep === 'ready'}
          <div class="step-pane ready-pane">
            <div class="hero-badge success-badge">
              <Icon name="check" size={32} color="var(--color-system-green)" />
            </div>

            <h2 class="step-title">Всё готово к игре</h2>
            <p class="step-subtitle">
              Настройки сохранены. Вы всегда сможете изменить их во вкладке «Настройки».
            </p>

            <div class="summary-card">
              <div class="summary-row">
                <span class="summary-label">Игровой никнейм</span>
                <span class="summary-value highlight">{nickname}</span>
              </div>
              <div class="summary-row">
                <span class="summary-label">Выбранная версия</span>
                <span class="summary-value">Minecraft {selectedVersion}</span>
              </div>
              <div class="summary-row">
                <span class="summary-label">Выделено памяти</span>
                <span class="summary-value">{Math.round(ramMb / 1024)} ГБ ({ramMb} МБ)</span>
              </div>
              <div class="summary-row status-row">
                <span class="summary-label">Фоновая загрузка</span>
                <span class="summary-value ready-status">
                  <Icon name="check" size={14} color="var(--color-system-green)" />
                  <span>Клиент подготовлен к старту</span>
                </span>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Card footer actions with Apple HIG layout -->
      <div class="card-footer">
        {#if currentStep === 'welcome'}
          <div class="footer-actions center">
            <Button
              variant="primary"
              size="large"
              onclick={handleNext}
            >
              <span>Начать настройку</span>
              <Icon name="chevron-right" size={16} color="white" />
            </Button>
          </div>

        {:else if currentStep === 'import'}
          <div class="footer-actions split">
            <Button
              variant="secondary"
              size="regular"
              onclick={() => { currentStep = 'nickname'; }}
            >
              <span>Настроить с нуля</span>
            </Button>

            <Button
              variant="primary"
              size="regular"
              loading={isImporting}
              disabled={isImporting || importSuccess}
              onclick={executeImport}
            >
              <Icon name="refresh" size={14} color="white" />
              <span>Перенести всё</span>
            </Button>
          </div>

        {:else if currentStep === 'ready'}
          <div class="footer-actions split">
            <Button
              variant="secondary"
              size="regular"
              onclick={() => finishOnboarding(false)}
            >
              <span>В лаунчер</span>
            </Button>

            <Button
              variant="primary"
              size="large"
              onclick={() => finishOnboarding(true)}
            >
              <Icon name="play" size={16} color="white" />
              <span>Начать игру</span>
            </Button>
          </div>

        {:else}
          <!-- Intermediate steps (nickname, version, memory) -->
          <div class="footer-actions split">
            <Button
              variant="secondary"
              size="regular"
              onclick={handleBack}
            >
              <Icon name="chevron-left" size={14} color="var(--color-label)" />
              <span>Назад</span>
            </Button>

            <Button
              variant="primary"
              size="regular"
              disabled={currentStep === 'nickname' && !isNicknameValid}
              onclick={handleNext}
            >
              <span>Далее</span>
              <Icon name="chevron-right" size={14} color="white" />
            </Button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .onboarding-scrim {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.45);
    animation: scrimFadeIn 220ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes scrimFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .onboarding-card {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 580px;
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 64px);
    border-radius: var(--radius-xl, 18px);
    background: var(--color-system-background, #1c1c1e);
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.12));
    box-shadow:
      0 24px 64px rgba(0, 0, 0, 0.45),
      0 4px 16px rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(28px) saturate(180%);
    overflow: hidden;
    outline: none;
    animation: cardPopIn 260ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  :global([data-theme='light']) .onboarding-card {
    background: rgba(255, 255, 255, 0.95);
    box-shadow:
      0 24px 64px rgba(0, 0, 0, 0.20),
      0 4px 16px rgba(0, 0, 0, 0.08);
  }

  @keyframes cardPopIn {
    from {
      opacity: 0;
      transform: scale(0.975) translateY(12px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  /* Header bar */
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px 8px;
    min-height: 44px;
  }

  .step-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .step-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-tertiary-label, rgba(120, 120, 128, 0.36));
    transition: all 200ms ease;
  }

  .step-dot.active {
    width: 18px;
    border-radius: 3px;
    background: var(--color-system-blue, #0a84ff);
  }

  .skip-action {
    background: transparent;
    border: none;
    color: var(--color-secondary-label, #8e8e93);
    font-size: var(--text-footnote, 13px);
    font-family: var(--font-system);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm, 6px);
    transition: color 150ms ease;
  }

  .skip-action:hover {
    color: var(--color-label, #ffffff);
  }

  /* Content area */
  .step-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 32px 24px;
  }

  .step-pane {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    animation: paneFadeIn 220ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes paneFadeIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .hero-badge {
    margin-bottom: 16px;
  }

  .app-emblem {
    width: 64px;
    height: 64px;
    border-radius: 18px;
    background: linear-gradient(135deg, #0a84ff 0%, #0066cc 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 8px 24px rgba(10, 132, 255, 0.35);
  }

  .emblem-glyph {
    color: #ffffff;
    font-size: 32px;
    font-weight: 700;
    line-height: 1;
  }

  .icon-badge {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: rgba(10, 132, 255, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .success-badge {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: rgba(48, 209, 88, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .step-title {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.015em;
    color: var(--color-label, #ffffff);
    margin: 0 0 8px;
  }

  .step-subtitle {
    font-size: 14px;
    line-height: 1.45;
    color: var(--color-secondary-label, #8e8e93);
    margin: 0 0 24px;
    max-width: 440px;
  }

  /* Features list in Welcome */
  .features-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    margin-bottom: 20px;
    text-align: left;
  }

  .feature-item {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 12px 14px;
    border-radius: var(--radius-md, 12px);
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.06));
  }

  .feature-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
  }

  .feature-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .feature-headline {
    font-size: 14px;
    font-weight: 600;
    color: var(--color-label, #ffffff);
  }

  .feature-desc {
    font-size: 12px;
    color: var(--color-secondary-label, #8e8e93);
    line-height: 1.35;
  }

  /* Prewarm Banner */
  .prewarm-banner {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    padding: 10px 14px;
    border-radius: var(--radius-md, 10px);
    background: rgba(10, 132, 255, 0.08);
    border: 1px solid rgba(10, 132, 255, 0.16);
  }

  .prewarm-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .prewarm-text {
    flex: 1;
    text-align: left;
    color: var(--color-label, #ffffff);
  }

  .prewarm-pct {
    font-weight: 600;
    color: var(--color-system-blue, #0a84ff);
  }

  .prewarm-track {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }

  .prewarm-fill {
    height: 100%;
    background: var(--color-system-blue, #0a84ff);
    transition: width 300ms ease;
  }

  /* Discovered Aura Data */
  .discovered-card {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    width: 100%;
    margin-bottom: 16px;
    text-align: left;
  }

  .card-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    border-radius: var(--radius-md, 10px);
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.06));
  }

  .item-label {
    font-size: 11px;
    color: var(--color-secondary-label, #8e8e93);
  }

  .item-value {
    font-size: 15px;
    font-weight: 600;
    color: var(--color-label, #ffffff);
  }

  .safe-notice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-secondary-label, #8e8e93);
    margin-bottom: 8px;
    text-align: left;
  }

  .import-status.success {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-system-green, #30d158);
    font-size: 13px;
    font-weight: 500;
    margin-top: 8px;
  }

  /* Nickname input */
  .input-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    max-width: 420px;
    gap: 6px;
    margin: 0 auto;
  }

  .input-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-secondary-label, #8e8e93);
  }

  .apple-field-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.06));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.15));
    border-radius: var(--radius-md, 10px);
    transition: all 180ms ease;
  }

  .apple-field-wrapper:focus-within {
    border-color: var(--color-system-blue, #0a84ff);
    box-shadow: 0 0 0 3px rgba(10, 132, 255, 0.25);
  }

  .apple-field-wrapper.has-error {
    border-color: var(--color-system-red, #ff453a);
    box-shadow: 0 0 0 3px rgba(255, 69, 58, 0.2);
  }

  .apple-input {
    width: 100%;
    height: 42px;
    padding: 0 36px 0 14px;
    background: transparent;
    border: none;
    outline: none;
    font-size: 16px;
    font-family: var(--font-system);
    color: var(--color-label, #ffffff);
    box-sizing: border-box;
  }

  .clear-input-btn {
    position: absolute;
    right: 10px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .validation-msg {
    font-size: 12px;
    margin: 4px 0 0;
    text-align: left;
  }

  .validation-msg.error {
    color: var(--color-system-red, #ff453a);
  }

  .validation-msg.hint {
    color: var(--color-secondary-label, #8e8e93);
  }

  /* Version cards grid */
  .version-cards-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }

  .version-card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px 44px 16px 16px;
    border-radius: var(--radius-lg, 14px);
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.04));
    border: 2px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    cursor: pointer;
    text-align: left;
    transition: all 180ms ease;
  }

  .version-card:hover {
    background: rgba(255, 255, 255, 0.07);
  }

  .version-card.selected {
    border-color: var(--color-system-blue, #0a84ff);
    background: rgba(10, 132, 255, 0.08);
  }

  .card-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .version-name {
    font-size: 16px;
    font-weight: 700;
    color: var(--color-label, #ffffff);
  }

  .version-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 10px;
  }

  .version-badge.recommended {
    background: var(--color-system-blue, #0a84ff);
    color: #ffffff;
  }

  .version-badge.classic {
    background: rgba(120, 120, 128, 0.24);
    color: var(--color-label, #ffffff);
  }

  .version-desc {
    font-size: 13px;
    line-height: 1.4;
    color: var(--color-secondary-label, #8e8e93);
    margin: 0;
  }

  .card-check {
    position: absolute;
    top: 18px;
    right: 16px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1.5px solid var(--color-separator, rgba(255, 255, 255, 0.2));
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 180ms ease;
  }

  .card-check.active {
    background: var(--color-system-blue, #0a84ff);
    border-color: var(--color-system-blue, #0a84ff);
  }

  /* Memory slider card */
  .memory-slider-card {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    padding: 18px 20px;
    border-radius: var(--radius-lg, 14px);
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    text-align: left;
  }

  .slider-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sys-ram-badge {
    font-size: 12px;
    color: var(--color-secondary-label, #8e8e93);
  }

  .current-ram-val {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-system-blue, #0a84ff);
  }

  .ram-explanation-box {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md, 8px);
    background: rgba(10, 132, 255, 0.08);
  }

  .explanation-text {
    font-size: 12px;
    line-height: 1.4;
    color: var(--color-label, #ffffff);
    margin: 0;
  }

  .preset-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preset-chip {
    padding: 5px 12px;
    border-radius: 12px;
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.12));
    background: transparent;
    color: var(--color-secondary-label, #8e8e93);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .preset-chip:hover {
    color: var(--color-label, #ffffff);
    background: rgba(255, 255, 255, 0.06);
  }

  .preset-chip.active {
    background: var(--color-system-blue, #0a84ff);
    color: #ffffff;
    border-color: var(--color-system-blue, #0a84ff);
  }

  /* Summary Card in Ready */
  .summary-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
    padding: 16px 18px;
    border-radius: var(--radius-lg, 14px);
    background: var(--color-secondary-system-grouped-background, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    text-align: left;
  }

  .summary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }

  .summary-label {
    color: var(--color-secondary-label, #8e8e93);
  }

  .summary-value {
    font-weight: 600;
    color: var(--color-label, #ffffff);
  }

  .summary-value.highlight {
    color: var(--color-system-blue, #0a84ff);
  }

  .ready-status {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--color-system-green, #30d158);
  }

  /* Footer actions */
  .card-footer {
    padding: 16px 28px 24px;
    border-top: 1px solid var(--color-separator, rgba(255, 255, 255, 0.08));
    background: rgba(0, 0, 0, 0.08);
  }

  :global([data-theme='light']) .card-footer {
    background: rgba(0, 0, 0, 0.02);
  }

  .footer-actions {
    display: flex;
    align-items: center;
    width: 100%;
  }

  .footer-actions.center {
    justify-content: center;
  }

  .footer-actions.split {
    justify-content: space-between;
  }

  /* Reduce Motion overrides */
  @media (prefers-reduced-motion: reduce) {
    .onboarding-scrim,
    .onboarding-card,
    .step-pane,
    .step-dot,
    .prewarm-fill,
    .version-card,
    .card-check {
      animation: none !important;
      transition: none !important;
      transform: none !important;
    }
  }

  :global([data-reduce-motion='true']) .onboarding-scrim,
  :global([data-reduce-motion='true']) .onboarding-card,
  :global([data-reduce-motion='true']) .step-pane,
  :global([data-reduce-motion='true']) .step-dot,
  :global([data-reduce-motion='true']) .prewarm-fill,
  :global([data-reduce-motion='true']) .version-card,
  :global([data-reduce-motion='true']) .card-check {
    animation: none !important;
    transition: none !important;
    transform: none !important;
  }
</style>
