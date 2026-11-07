<!--
  Wex Launcher — Component Showcase
  Comprehensive gallery of all 12 Apple HIG components for visual QA & automated verification.
-->
<script lang="ts">
  import Button from '../components/Button.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Slider from '../components/Slider.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Toolbar from '../components/Toolbar.svelte';
  import SearchField from '../components/SearchField.svelte';
  import Sheet from '../components/Sheet.svelte';
  import Popover from '../components/Popover.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';
  import Toast, { type ToastMessage } from '../components/Toast.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import { onMount } from 'svelte';

  let toggleState = $state(true);
  let sliderValue = $state(65);
  let segmentValue = $state('option1');
  let searchInput = $state('Текст поиска');
  let sheetOpen = $state(false);
  let popoverOpen = $state(false);

  onMount(() => {
    if (typeof window === 'undefined') return;
    function checkHash() {
      const hash = window.location.hash;
      if (hash.includes('sheet=true')) {
        sheetOpen = true;
      }
      if (hash.includes('popover=true')) {
        popoverOpen = true;
      }
    }
    window.addEventListener('hashchange', checkHash);
    checkHash();
  });

  const sampleToast: ToastMessage = {
    id: 'demo-1',
    type: 'success',
    title: 'Aura Client синхронизирован',
    description: 'Все 128 файлов проверены, хеши совпадают.',
    duration: 0, // don't auto-dismiss during showcase
  };
</script>

<div class="showcase-screen">
  <div class="showcase-header">
    <h2 class="section-title">Apple HIG Design System Gallery</h2>
    <p class="section-desc">Коллекция всех 12 компонентов лаунчера Wex в соответствии с Apple HIG.</p>
  </div>

  <div class="components-grid">
    <!-- 1. Buttons -->
    <div class="component-card">
      <h3 class="card-title">1. Button (Кнопка — primary / secondary / destructive)</h3>
      <div class="card-content row-gap">
        <Button variant="primary">Primary</Button>
        <Button variant="secondary">Secondary</Button>
        <Button variant="destructive">Destructive</Button>
        <Button variant="ghost">Ghost</Button>
        <Button variant="primary" loading>Loading</Button>
        <Button variant="secondary" disabled>Disabled</Button>
      </div>
    </div>

    <!-- 2. Toggle -->
    <div class="component-card">
      <h3 class="card-title">2. Toggle (Переключатель)</h3>
      <div class="card-content row-gap">
        <Toggle bind:checked={toggleState} label="Включено (ВКЛ)" />
        <Toggle checked={false} label="Выключено (ВЫКЛ)" />
        <Toggle checked={true} disabled label="Отключённый (Disabled)" />
      </div>
    </div>

    <!-- 3. Slider -->
    <div class="component-card">
      <h3 class="card-title">3. Slider (Ползунок)</h3>
      <div class="card-content">
        <Slider
          bind:value={sliderValue}
          min={0}
          max={100}
          unit="%"
          label="Громкость аудиоэффектов"
        />
      </div>
    </div>

    <!-- 4. Segmented Control -->
    <div class="component-card">
      <h3 class="card-title">4. SegmentedControl (Вкладки)</h3>
      <div class="card-content">
        <SegmentedControl
          options={[
            { value: 'option1', label: '1.21.1 Core' },
            { value: 'option2', label: '1.20.1 Pack' },
            { value: 'option3', label: 'Vanilla' },
          ]}
          bind:value={segmentValue}
        />
      </div>
    </div>

    <!-- 5. List & Row -->
    <div class="component-card">
      <h3 class="card-title">5. List & Row (Список и строки)</h3>
      <div class="card-content">
        <List header="Пример группы">
          <Row title="Основная строка списка" subtitle="Вторичное описание строки" clickable showChevron>
            {#snippet leading()}
              <Icon name="server" size={16} color="var(--color-system-blue)" />
            {/snippet}
          </Row>
          <Row title="Строка с переключателем">
            {#snippet trailing()}
              <Toggle checked={true} />
            {/snippet}
          </Row>
        </List>
      </div>
    </div>

    <!-- 6. Toolbar & Window Controls -->
    <div class="component-card">
      <h3 class="card-title">6. Toolbar (Панель инструментов)</h3>
      <div class="card-content">
        <Toolbar title="Заголовок окна" showWindowControls={true} />
      </div>
    </div>

    <!-- 7. Search Field -->
    <div class="component-card">
      <h3 class="card-title">7. SearchField (Поле поиска)</h3>
      <div class="card-content row-gap">
        <SearchField placeholder="Пустое поле поиска..." />
        <SearchField bind:value={searchInput} />
      </div>
    </div>

    <!-- 8. Progress Bar -->
    <div class="component-card">
      <h3 class="card-title">8. ProgressBar (Индикатор прогресса)</h3>
      <div class="card-content col-gap">
        <ProgressBar
          percentage={68}
          label="Загрузка модов..."
          detail="15.4 МБ/с • 68%"
        />
        <ProgressBar indeterminate label="Проверка контрольных сумм SHA-256..." />
      </div>
    </div>

    <!-- 9. Toast / Transient HUD -->
    <div class="component-card">
      <h3 class="card-title">9. Toast (Уведомление / баннер)</h3>
      <div class="card-content">
        <Toast toast={sampleToast} />
      </div>
    </div>

    <!-- 10. Sheet & Popover Triggers -->
    <div class="component-card">
      <h3 class="card-title">10. Sheet & Popover (Модальные окна)</h3>
      <div class="card-content row-gap">
        <Button variant="primary" onclick={() => (sheetOpen = true)}>
          Открыть Sheet
        </Button>

        <Popover bind:open={popoverOpen} title="Всплывающее меню">
          {#snippet trigger()}
            <Button variant="secondary">
              Показать Popover
            </Button>
          {/snippet}
          <div style="padding: 4px 0;">
            <p style="margin-bottom: 8px;">Быстрые действия над выбранным элементом.</p>
            <Button variant="primary" size="small" onclick={() => (popoverOpen = false)}>
              Применить
            </Button>
          </div>
        </Popover>
      </div>
    </div>
  </div>
</div>

<Sheet bind:open={sheetOpen} title="Демонстрационный Sheet">
  <div style="padding: 10px 0;">
    <p style="margin-bottom: 12px; color: var(--color-secondary-label);">
      Sheet открывается как модальная карточка над затемнённым окном родительского интерфейса.
    </p>
    <Button variant="primary" fullWidth onclick={() => (sheetOpen = false)}>
      Понятно
    </Button>
  </div>
</Sheet>

<style>
  .showcase-screen {
    display: flex;
    flex-direction: column;
    padding: 24px;
    gap: 20px;
    overflow-y: auto;
    height: 100%;
    font-family: var(--font-system);
  }

  .showcase-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .section-title {
    font-size: var(--text-title-2);
    font-weight: 700;
    letter-spacing: var(--tracking-title-2);
    color: var(--color-label);
  }

  .section-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
  }

  .components-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 16px;
  }

  .component-card {
    background-color: var(--card-bg);
    backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.05);
  }

  .card-title {
    font-size: var(--text-footnote);
    font-weight: 600;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .card-content {
    display: flex;
  }

  .card-content.row-gap {
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .card-content.col-gap {
    flex-direction: column;
    gap: 12px;
  }
</style>
