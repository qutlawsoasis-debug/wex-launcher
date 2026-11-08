<!--
  Wex Launcher — Wardrobe Screen (SPEC 1.6 & 2.8)
  Apple Human Interface Guidelines:
  - developer.apple.com/design/human-interface-guidelines/segmented-controls (Model & View selection)
  - developer.apple.com/design/human-interface-guidelines/buttons (systemBlue primary action, neutral secondary)
  - developer.apple.com/design/human-interface-guidelines/lists-and-tables (Inset grouped history & cape selection)
  - developer.apple.com/design/human-interface-guidelines/color (Apple semantic colors, no neon glow or gradient slogans)
  - developer.apple.com/design/human-interface-guidelines/selection (Fill selection instead of outlines)
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../components/Button.svelte';
  import SegmentedControl from '../components/SegmentedControl.svelte';
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
  import Icon from '../components/icons/Icon.svelte';
  import { COMMANDS } from '../ipc/generated/commands';
  import type { WardrobeItemDto, WardrobeStateDto, SkinValidationResultDto } from '../ipc/generated';

  interface Props {
    playerName?: string;
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, desc?: string) => void;
  }

  let { playerName = 'Player', ontoast }: Props = $props();

  // Model & View states
  let selectedModel = $state<'default' | 'slim'>('default');
  let selectedView = $state<'front' | 'back'>('front');
  let selectedCapeId = $state<string>('none');

  // Active Skin state
  let skinName = $state('Steve (Классический)');
  let skinBase64 = $state<string | null>(null);
  let customCapeBase64 = $state<string | null>(null);

  // Status & history
  let historyItems = $state<WardrobeItemDto[]>([]);
  let isApplying = $state(false);
  let isValidating = $state(false);
  let errorMessage = $state<string | null>(null);

  let fileInputRef: HTMLInputElement | null = null;
  let capeInputRef: HTMLInputElement | null = null;
  let canvasRef: HTMLCanvasElement | null = null;

  const modelOptions = [
    { value: 'default', label: 'Классическая (4 px)' },
    { value: 'slim', label: 'Тонкая (3 px)' },
  ];

  const viewOptions = [
    { value: 'front', label: 'Спереди' },
    { value: 'back', label: 'Сзади' },
  ];

  const capeOptions = [
    { id: 'none', name: 'Без плаща', desc: 'Стандартный вид без накидки' },
    { id: 'cyan', name: 'Aura Cyan', desc: 'Официальный лазурный плащ Aura' },
    { id: 'silver', name: 'Aura Silver', desc: 'Официальный серебристый плащ Aura' },
    { id: 'custom', name: 'Пользовательский', desc: 'Загрузить файл накидки (PNG)' },
  ];

  async function callTauri<T>(command: string, args: Record<string, any> = {}): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(command, args);
    } catch (e: any) {
      throw new Error(e?.message || String(e));
    }
  }

  onMount(() => {
    loadActiveState();
    loadHistory();
    renderPreview();
  });

  $effect(() => {
    // Re-render preview whenever skin, cape, model, or view changes
    const _skin = skinBase64;
    const _cape = selectedCapeId;
    const _model = selectedModel;
    const _view = selectedView;
    renderPreview();
  });

  async function loadActiveState() {
    try {
      const state = await callTauri<WardrobeStateDto>(COMMANDS.WARDROBE_GET_STATE);
      if (state) {
        skinName = state.skin_name || 'Steve (Классический)';
        skinBase64 = state.skin_base64 || null;
        selectedModel = state.model === 'Slim' ? 'slim' : 'default';
        selectedCapeId = state.cape_id || 'none';
        customCapeBase64 = state.cape_base64 || null;
      }
    } catch {
      // Offline fallback: Steve default
    }
  }

  async function loadHistory() {
    try {
      const res = await callTauri<{ items: WardrobeItemDto[] }>(COMMANDS.EXTRAS_LIST_SKIN_HISTORY, {
        request: { history_dir: '' },
      });
      if (res?.items) {
        historyItems = res.items;
      }
    } catch {
      historyItems = [];
    }
  }

  function renderPreview() {
    if (!canvasRef) return;
    const ctx = canvasRef.getContext('2d');
    if (!ctx) return;

    const width = 160;
    const height = 320;
    canvasRef.width = width;
    canvasRef.height = height;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, width, height);

    const isSlim = selectedModel === 'slim';
    const isBack = selectedView === 'back';
    const armW = isSlim ? 3 : 4;

    // Draw background subtle pedestal
    ctx.fillStyle = 'rgba(0, 0, 0, 0.04)';
    ctx.beginPath();
    ctx.ellipse(width / 2, height - 12, 54, 12, 0, 0, Math.PI * 2);
    ctx.fill();

    const scale = 10;
    const originX = (width - 16 * scale) / 2;
    const originY = 20;

    if (skinBase64) {
      const img = new Image();
      img.onload = () => {
        ctx.clearRect(0, 0, width, height);

        // Pedestal
        ctx.fillStyle = 'rgba(0, 0, 0, 0.04)';
        ctx.beginPath();
        ctx.ellipse(width / 2, height - 12, 54, 12, 0, 0, Math.PI * 2);
        ctx.fill();

        // 1. Cape behind character (if back view)
        if (isBack && selectedCapeId !== 'none') {
          drawCape(ctx, originX + 3 * scale, originY + 8 * scale, 10 * scale, 16 * scale);
        }

        // 2. Head
        if (!isBack) {
          // Front Head (8, 8, 8, 8) -> Canvas (4, 0, 8, 8)
          drawPixelPart(ctx, img, 8, 8, 8, 8, originX + 4 * scale, originY, 8 * scale, 8 * scale);
          // Hat (40, 8, 8, 8)
          drawPixelPart(ctx, img, 40, 8, 8, 8, originX + 4 * scale, originY, 8 * scale, 8 * scale);
        } else {
          // Back Head (24, 8, 8, 8) -> Canvas (4, 0, 8, 8)
          drawPixelPart(ctx, img, 24, 8, 8, 8, originX + 4 * scale, originY, 8 * scale, 8 * scale);
          // Hat Back (56, 8, 8, 8)
          drawPixelPart(ctx, img, 56, 8, 8, 8, originX + 4 * scale, originY, 8 * scale, 8 * scale);
        }

        // 3. Torso
        if (!isBack) {
          drawPixelPart(ctx, img, 20, 20, 8, 12, originX + 4 * scale, originY + 8 * scale, 8 * scale, 12 * scale);
          drawPixelPart(ctx, img, 20, 36, 8, 12, originX + 4 * scale, originY + 8 * scale, 8 * scale, 12 * scale);
        } else {
          drawPixelPart(ctx, img, 32, 20, 8, 12, originX + 4 * scale, originY + 8 * scale, 8 * scale, 12 * scale);
          drawPixelPart(ctx, img, 32, 36, 8, 12, originX + 4 * scale, originY + 8 * scale, 8 * scale, 12 * scale);
        }

        // 4. Arms
        const armCanvasXLeft = isSlim ? originX + 1 * scale : originX;
        const armCanvasXRight = isSlim ? originX + 12 * scale : originX + 12 * scale;

        if (!isBack) {
          // Right arm on screen left
          drawPixelPart(ctx, img, 44, 20, armW, 12, armCanvasXLeft, originY + 8 * scale, armW * scale, 12 * scale);
          drawPixelPart(ctx, img, 44, 36, armW, 12, armCanvasXLeft, originY + 8 * scale, armW * scale, 12 * scale);
          // Left arm on screen right
          drawPixelPart(ctx, img, 36, 52, armW, 12, armCanvasXRight, originY + 8 * scale, armW * scale, 12 * scale);
          drawPixelPart(ctx, img, 52, 52, armW, 12, armCanvasXRight, originY + 8 * scale, armW * scale, 12 * scale);
        } else {
          // Back right arm on screen right
          drawPixelPart(ctx, img, isSlim ? 51 : 52, 20, armW, 12, armCanvasXRight, originY + 8 * scale, armW * scale, 12 * scale);
          drawPixelPart(ctx, img, isSlim ? 51 : 52, 36, armW, 12, armCanvasXRight, originY + 8 * scale, armW * scale, 12 * scale);
          // Back left arm on screen left
          drawPixelPart(ctx, img, isSlim ? 43 : 44, 52, armW, 12, armCanvasXLeft, originY + 8 * scale, armW * scale, 12 * scale);
          drawPixelPart(ctx, img, isSlim ? 59 : 60, 52, armW, 12, armCanvasXLeft, originY + 8 * scale, armW * scale, 12 * scale);
        }

        // 5. Legs
        if (!isBack) {
          // Right leg
          drawPixelPart(ctx, img, 4, 20, 4, 12, originX + 4 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          drawPixelPart(ctx, img, 4, 36, 4, 12, originX + 4 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          // Left leg
          drawPixelPart(ctx, img, 20, 52, 4, 12, originX + 8 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          drawPixelPart(ctx, img, 4, 52, 4, 12, originX + 8 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
        } else {
          // Back legs
          drawPixelPart(ctx, img, 12, 20, 4, 12, originX + 8 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          drawPixelPart(ctx, img, 12, 36, 4, 12, originX + 8 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          drawPixelPart(ctx, img, 28, 52, 4, 12, originX + 4 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
          drawPixelPart(ctx, img, 12, 52, 4, 12, originX + 4 * scale, originY + 20 * scale, 4 * scale, 12 * scale);
        }
      };
      img.src = skinBase64.startsWith('data:') ? skinBase64 : `data:image/png;base64,${skinBase64}`;
    } else {
      // Default Steve rendering directly
      drawDefaultSteve(ctx, originX, originY, scale, isSlim, isBack);
    }
  }

  function drawPixelPart(
    ctx: CanvasRenderingContext2D,
    img: HTMLImageElement,
    sx: number,
    sy: number,
    sw: number,
    sh: number,
    dx: number,
    dy: number,
    dw: number,
    dh: number
  ) {
    try {
      ctx.drawImage(img, sx, sy, sw, sh, dx, dy, dw, dh);
    } catch {}
  }

  function drawCape(ctx: CanvasRenderingContext2D, dx: number, dy: number, dw: number, dh: number) {
    if (selectedCapeId === 'cyan') {
      ctx.fillStyle = '#0A84FF'; // Apple systemBlue / Aura Cyan
      ctx.fillRect(dx, dy, dw, dh);
      ctx.fillStyle = '#FFFFFF';
      ctx.fillRect(dx + 30, dy + 50, 40, 10);
      ctx.fillStyle = '#005BB5';
      ctx.fillRect(dx, dy + dh - 10, dw, 10);
    } else if (selectedCapeId === 'silver') {
      ctx.fillStyle = '#8E8E93'; // Apple systemGray / Aura Silver
      ctx.fillRect(dx, dy, dw, dh);
      ctx.fillStyle = '#FFFFFF';
      ctx.fillRect(dx + 30, dy + 50, 40, 10);
      ctx.fillStyle = '#3A3A3C';
      ctx.fillRect(dx, dy + dh - 10, dw, 10);
    } else if (selectedCapeId === 'custom' && customCapeBase64) {
      const capeImg = new Image();
      capeImg.onload = () => {
        try {
          ctx.drawImage(capeImg, 1, 1, 10, 16, dx, dy, dw, dh);
        } catch {}
      };
      capeImg.src = customCapeBase64.startsWith('data:')
        ? customCapeBase64
        : `data:image/png;base64,${customCapeBase64}`;
    }
  }

  function drawDefaultSteve(
    ctx: CanvasRenderingContext2D,
    ox: number,
    oy: number,
    s: number,
    isSlim: boolean,
    isBack: boolean
  ) {
    const armW = isSlim ? 3 : 4;

    // Cape behind if back
    if (isBack && selectedCapeId !== 'none') {
      drawCape(ctx, ox + 3 * s, oy + 8 * s, 10 * s, 16 * s);
    }

    // Head
    ctx.fillStyle = '#A77D5A'; // Skin tone
    ctx.fillRect(ox + 4 * s, oy, 8 * s, 8 * s);
    ctx.fillStyle = '#4A321E'; // Hair
    ctx.fillRect(ox + 4 * s, oy, 8 * s, 2 * s);
    if (!isBack) {
      ctx.fillStyle = '#FFFFFF'; // Eyes
      ctx.fillRect(ox + 5 * s, oy + 4 * s, 2 * s, 1 * s);
      ctx.fillRect(ox + 9 * s, oy + 4 * s, 2 * s, 1 * s);
      ctx.fillStyle = '#3C4DA2'; // Pupils
      ctx.fillRect(ox + 6 * s, oy + 4 * s, 1 * s, 1 * s);
      ctx.fillRect(ox + 9 * s, oy + 4 * s, 1 * s, 1 * s);
      ctx.fillStyle = '#6E452B'; // Beard/mouth
      ctx.fillRect(ox + 7 * s, oy + 6 * s, 2 * s, 1 * s);
    }

    // Torso (Cyan shirt)
    ctx.fillStyle = '#00AAAA';
    ctx.fillRect(ox + 4 * s, oy + 8 * s, 8 * s, 12 * s);

    // Arms
    const armLeftX = isSlim ? ox + 1 * s : ox;
    const armRightX = isSlim ? ox + 12 * s : ox + 12 * s;
    ctx.fillStyle = '#00AAAA'; // Sleeves
    ctx.fillRect(armLeftX, oy + 8 * s, armW * s, 4 * s);
    ctx.fillRect(armRightX, oy + 8 * s, armW * s, 4 * s);
    ctx.fillStyle = '#A77D5A'; // Bare arms
    ctx.fillRect(armLeftX, oy + 12 * s, armW * s, 8 * s);
    ctx.fillRect(armRightX, oy + 12 * s, armW * s, 8 * s);

    // Legs (Blue pants + gray shoes)
    ctx.fillStyle = '#26348C'; // Blue pants
    ctx.fillRect(ox + 4 * s, oy + 20 * s, 4 * s, 10 * s);
    ctx.fillRect(ox + 8 * s, oy + 20 * s, 4 * s, 10 * s);
    ctx.fillStyle = '#5A5A5A'; // Shoes
    ctx.fillRect(ox + 4 * s, oy + 30 * s, 4 * s, 2 * s);
    ctx.fillRect(ox + 8 * s, oy + 30 * s, 4 * s, 2 * s);
  }

  async function handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    input.value = ''; // Reset input for re-selecting same file
    errorMessage = null;

    if (!file.name.toLowerCase().endsWith('.png')) {
      errorMessage = 'Файл должен быть в формате PNG';
      ontoast?.('error', 'Ошибка формата', errorMessage);
      return;
    }

    if (file.size > 2 * 1024 * 1024) {
      errorMessage = 'Размер файла скина не должен превышать 2 МБ';
      ontoast?.('error', 'Файл слишком велик', errorMessage);
      return;
    }

    isValidating = true;
    try {
      const reader = new FileReader();
      reader.onload = async () => {
        try {
          const dataUrl = reader.result as string;
          const base64Data = dataUrl.split(',')[1] || dataUrl;

          // Validate via IPC
          const valRes = await callTauri<SkinValidationResultDto>(COMMANDS.EXTRAS_VALIDATE_SKIN, {
            request: { skin_bytes_base64: base64Data },
          });

          if (!valRes.is_valid) {
            errorMessage = valRes.error || 'Некорректное изображение скина';
            ontoast?.('error', 'Недопустимый скин', errorMessage);
            isValidating = false;
            return;
          }

          let finalBase64 = base64Data;

          // Auto-convert 64x32 legacy skins to modern 64x64
          if (valRes.height === 32) {
            const conv = await callTauri<{ converted_bytes_base64: string }>(
              COMMANDS.EXTRAS_CONVERT_SKIN,
              { request: { skin_bytes_base64: base64Data } }
            );
            finalBase64 = conv.converted_bytes_base64;
            ontoast?.('info', 'Формат обновлён', 'Старый скин 64×32 сконвертирован в 64×64');
          }

          skinBase64 = finalBase64;
          skinName = file.name.replace(/\.[^/.]+$/, '');

          // Detect slim model (Alex) by checking transparent pixels
          detectModelFromBase64(finalBase64);

          ontoast?.('success', 'Скин загружен', file.name);
        } catch (err: any) {
          errorMessage = err?.message || 'Ошибка обработки файла';
          ontoast?.('error', 'Ошибка загрузки', errorMessage ?? undefined);
        } finally {
          isValidating = false;
        }
      };
      reader.readAsDataURL(file);
    } catch (err: any) {
      errorMessage = err?.message || 'Не удалось прочитать файл';
      isValidating = false;
    }
  }

  function detectModelFromBase64(b64: string) {
    const img = new Image();
    img.onload = () => {
      const testCanvas = document.createElement('canvas');
      testCanvas.width = 64;
      testCanvas.height = 64;
      const tCtx = testCanvas.getContext('2d');
      if (!tCtx) return;
      tCtx.drawImage(img, 0, 0);

      // Inspect column 55 at y=20..31
      let isSlim = true;
      for (let y = 20; y < 32; y++) {
        const px = tCtx.getImageData(55, y, 1, 1).data;
        if (px[3] > 0) {
          isSlim = false;
          break;
        }
      }
      selectedModel = isSlim ? 'slim' : 'default';
    };
    img.src = b64.startsWith('data:') ? b64 : `data:image/png;base64,${b64}`;
  }

  async function handleCapeFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    input.value = '';

    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      customCapeBase64 = dataUrl.split(',')[1] || dataUrl;
      selectedCapeId = 'custom';
      ontoast?.('success', 'Плащ загружен', file.name);
    };
    reader.readAsDataURL(file);
  }

  function handleResetDefault() {
    skinBase64 = null;
    skinName = 'Steve (Классический)';
    selectedModel = 'default';
    selectedCapeId = 'none';
    customCapeBase64 = null;
    errorMessage = null;
    ontoast?.('info', 'Сброс выполнен', 'Восстановлен стандартный скин Steve');
  }

  function selectHistorySkin(item: WardrobeItemDto) {
    skinName = item.name;
    selectedModel = item.model === 'slim' ? 'slim' : 'default';
    // If we have hash or data, we can reload or keep
    errorMessage = null;
    ontoast?.('info', 'Выбран скин из истории', item.name);
  }

  async function deleteHistorySkin(item: WardrobeItemDto, e: MouseEvent) {
    e.stopPropagation();
    try {
      await callTauri(COMMANDS.WARDROBE_DELETE_HISTORY, {
        request: { history_dir: '', item_id: item.id },
      });
      historyItems = historyItems.filter((i) => i.id !== item.id);
      ontoast?.('info', 'Удалено', `Скин «${item.name}» удалён из истории`);
    } catch (err: any) {
      ontoast?.('error', 'Ошибка удаления', err.message);
    }
  }

  async function applyWardrobe() {
    if (isApplying) return;
    isApplying = true;
    errorMessage = null;

    try {
      const activeCape = capeOptions.find((c) => c.id === selectedCapeId);
      const res = await callTauri<{ success: boolean; message: string }>(COMMANDS.WARDROBE_APPLY, {
        request: {
          nickname: playerName || 'Player',
          skin_name: skinName,
          skin_base64: skinBase64,
          model: selectedModel === 'slim' ? 'Slim' : 'Classic',
          cape_id: selectedCapeId,
          cape_name: activeCape?.name || 'Без плаща',
          cape_base64: selectedCapeId === 'custom' ? customCapeBase64 : null,
        },
      });

      if (res?.success) {
        ontoast?.('success', 'Гардероб применён', 'Скин и плащ успешно обновлены в клиенте игры');
        loadHistory();
      } else {
        ontoast?.('warning', 'Применение', res?.message || 'Готово');
      }
    } catch (err: any) {
      errorMessage = err?.message || 'Ошибка применения настроек';
      ontoast?.('error', 'Ошибка применения', errorMessage ?? undefined);
    } finally {
      isApplying = false;
    }
  }
</script>

<div class="wardrobe-screen" aria-label="Экран «Гардероб»">
  <!-- Hidden File Pickers -->
  <input
    type="file"
    accept="image/png"
    bind:this={fileInputRef}
    onchange={handleFileSelect}
    style="display: none"
  />
  <input
    type="file"
    accept="image/png"
    bind:this={capeInputRef}
    onchange={handleCapeFileSelect}
    style="display: none"
  />

  <div class="wardrobe-layout">
    <!-- Left Column: Character 2D Viewport -->
    <div class="preview-column">
      <div class="preview-card">
        <div class="preview-header">
          <SegmentedControl
            options={viewOptions}
            bind:value={selectedView}
            size="small"
          />
        </div>

        <div class="canvas-container">
          <canvas
            bind:this={canvasRef}
            class="character-canvas"
            aria-label="2D предпросмотр персонажа"
          ></canvas>
        </div>

        <div class="preview-status">
          <div class="status-title">{skinName}</div>
          <div class="status-subtitle">
            {selectedModel === 'slim' ? 'Тонкая модель (Alex)' : 'Классическая модель (Steve)'}
            {#if selectedCapeId !== 'none'}
              • {capeOptions.find((c) => c.id === selectedCapeId)?.name}
            {/if}
          </div>
        </div>

        <div class="preview-actions">
          <Button
            variant="secondary"
            size="small"
            onclick={() => fileInputRef?.click()}
            disabled={isValidating}
          >
            <Icon name="folder" size={14} color="var(--color-label)" />
            <span>Загрузить скин (PNG)</span>
          </Button>

          <Button
            variant="ghost"
            size="small"
            onclick={handleResetDefault}
          >
            <span>Сбросить на стандартный</span>
          </Button>
        </div>
      </div>
    </div>

    <!-- Right Column: Model, Capes, and History Settings -->
    <div class="settings-column">
      {#if errorMessage}
        <div class="error-banner" role="alert">
          <Icon name="error" size={16} color="var(--color-system-red)" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      <!-- Model Type Segmented Control -->
      <List header="Тип модели персонажа">
        <div class="list-control-wrap">
          <SegmentedControl
            options={modelOptions}
            bind:value={selectedModel}
            size="regular"
          />
        </div>
      </List>

      <!-- Capes Selection Section -->
      <List header="Плащ игрока">
        {#each capeOptions as cape}
          {@const isSelected = selectedCapeId === cape.id}
          <Row
            title={cape.name}
            subtitle={cape.desc}
            clickable
            onclick={() => {
              selectedCapeId = cape.id;
              if (cape.id === 'custom' && !customCapeBase64) {
                capeInputRef?.click();
              }
            }}
          >
            {#snippet trailing()}
              <div class="cape-row-accessory">
                {#if cape.id === 'cyan'}
                  <span class="cape-chip cyan" title="Лазурный плащ Aura"></span>
                {:else if cape.id === 'silver'}
                  <span class="cape-chip silver" title="Серебристый плащ Aura"></span>
                {:else if cape.id === 'custom'}
                  <button
                    type="button"
                    class="upload-cape-link"
                    onclick={(e) => {
                      e.stopPropagation();
                      capeInputRef?.click();
                    }}
                  >
                    Обзор...
                  </button>
                {/if}

                {#if isSelected}
                  <span class="check-mark" aria-label="Выбрано">
                    <Icon name="check" size={16} color="var(--color-system-blue)" />
                  </span>
                {/if}
              </div>
            {/snippet}
          </Row>
        {/each}
      </List>

      <!-- History of Skins Section -->
      <List header="История скинов" footer="Ранее использованные скины сохраняются локально в профиле лаунчера.">
        {#if historyItems.length === 0}
          <div class="empty-history">
            <span class="empty-text">История гардероба пуста</span>
          </div>
        {:else}
          {#each historyItems as item}
            <Row
              title={item.name}
              subtitle={`${item.model === 'slim' ? 'Тонкая' : 'Классическая'} • ${item.created_utc.slice(0, 10)}`}
              clickable
              onclick={() => selectHistorySkin(item)}
            >
              {#snippet trailing()}
                <div class="history-actions">
                  <button
                    type="button"
                    class="delete-item-btn"
                    title="Удалить из истории"
                    onclick={(e) => deleteHistorySkin(item, e)}
                  >
                    <Icon name="close" size={14} color="var(--color-secondary-label)" />
                  </button>
                </div>
              {/snippet}
            </Row>
          {/each}
        {/if}
      </List>

      <!-- Apply to Game Bottom Action -->
      <div class="apply-bar">
        <Button
          variant="primary"
          size="regular"
          onclick={applyWardrobe}
          disabled={isApplying}
        >
          <span>{isApplying ? 'Применение...' : 'Применить к игре'}</span>
        </Button>
      </div>
    </div>
  </div>
</div>

<style>
  .wardrobe-screen {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    padding: 24px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }

  .wardrobe-layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 24px;
    max-width: 960px;
    margin: 0 auto;
    width: 100%;
  }

  .preview-column {
    display: flex;
    flex-direction: column;
  }

  .preview-card {
    background-color: var(--glass-card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
  }

  .preview-header {
    width: 100%;
    display: flex;
    justify-content: center;
    margin-bottom: 12px;
  }

  .canvas-container {
    width: 160px;
    height: 320px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .character-canvas {
    image-rendering: pixelated;
    image-rendering: crisp-edges;
  }

  .preview-status {
    margin-top: 16px;
    text-align: center;
    width: 100%;
  }

  .status-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status-subtitle {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    margin-top: 2px;
  }

  .preview-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    margin-top: 16px;
  }

  .settings-column {
    display: flex;
    flex-direction: column;
  }

  .list-control-wrap {
    padding: 12px 16px;
    background-color: var(--glass-card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    margin-bottom: 8px;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    background-color: rgba(255, 69, 58, 0.12);
    border: 1px solid rgba(255, 69, 58, 0.28);
    border-radius: var(--radius-md);
    padding: 10px 14px;
    font-size: var(--text-footnote);
    color: var(--color-system-red);
    margin-bottom: 16px;
  }

  .cape-row-accessory {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cape-chip {
    width: 14px;
    height: 20px;
    border-radius: 2px;
    display: inline-block;
  }

  .cape-chip.cyan {
    background-color: var(--color-system-blue);
    border: 1px solid rgba(0, 0, 0, 0.15);
  }

  .cape-chip.silver {
    background-color: var(--color-system-gray);
    border: 1px solid rgba(0, 0, 0, 0.15);
  }

  .upload-cape-link {
    font-size: var(--text-caption-1);
    color: var(--color-system-blue);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .upload-cape-link:hover {
    text-decoration: underline;
  }

  .check-mark {
    display: flex;
    align-items: center;
  }

  .empty-history {
    padding: 24px 16px;
    text-align: center;
    background-color: var(--glass-card-bg);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
  }

  .empty-text {
    font-size: var(--text-subheadline);
    color: var(--color-tertiary-label);
  }

  .history-actions {
    display: flex;
    align-items: center;
  }

  .delete-item-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .delete-item-btn:hover {
    background-color: var(--color-quaternary-fill);
  }

  .apply-bar {
    margin-top: 8px;
    display: flex;
    justify-content: flex-end;
  }
</style>
