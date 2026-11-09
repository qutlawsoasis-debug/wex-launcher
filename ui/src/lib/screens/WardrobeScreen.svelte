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
  import { network } from '../design/network.svelte';
  import { t, i18n } from '../i18n';
  import { COMMANDS } from '../ipc/generated/commands';
  import type { WardrobeItemDto, WardrobeStateDto, SkinValidationResultDto } from '../ipc/generated';

  interface Props {
    playerName?: string;
    initialHistory?: WardrobeItemDto[];
    ontoast?: (type: 'info' | 'success' | 'warning' | 'error', title: string, desc?: string) => void;
  }

  let { playerName = 'Player', initialHistory, ontoast }: Props = $props();

  // Model & View states
  let selectedModel = $state<'default' | 'slim'>('default');
  let selectedView = $state<'front' | 'back'>('front');
  let selectedCapeId = $state<string>('none');

  // Active Skin state
  let skinName = $state('Стандартный скин');
  let skinBase64 = $state<string | null>(null);
  let customCapeBase64 = $state<string | null>(null);

  // Status & history
  let historyItems = $state<WardrobeItemDto[]>([]);

  $effect(() => {
    if (initialHistory) {
      historyItems = initialHistory;
    }
  });
  let isApplying = $state(false);
  let isValidating = $state(false);
  let errorMessage = $state<string | null>(null);

  // 3D / 2D Canvas References & State
  let fileInputRef: HTMLInputElement | null = null;
  let capeInputRef: HTMLInputElement | null = null;
  let canvas2dRef: HTMLCanvasElement | null = null;
  let canvas3dRef: HTMLCanvasElement | null = null;
  let renderer3d: Skin3DRenderer | null = null;
  let hasWebGL = $state(false);
  let displayMode = $state<'3d' | '2d'>('3d');
  let selectedAnimation = $state<'idle' | 'walking'>('idle');

  import {
    Skin3DRenderer,
    isWebGLSupported,
    createDefaultSkinCanvas,
  } from '../design/skin3d';

  let displayModeOptions = $derived([
    { value: '3d', label: i18n.locale === 'en' ? '3D Model' : '3D модель' },
    { value: '2d', label: i18n.locale === 'en' ? '2D View' : '2D вид' },
  ]);

  let animationOptions = $derived([
    { value: 'idle', label: i18n.locale === 'en' ? 'Idle' : 'Покой' },
    { value: 'walking', label: i18n.locale === 'en' ? 'Walking' : 'Ходьба' },
  ]);

  let modelOptions = $derived([
    { value: 'default', label: t('wardrobe.model_default') },
    { value: 'slim', label: t('wardrobe.model_slim') },
  ]);

  let viewOptions = $derived([
    { value: 'front', label: t('wardrobe.view_front') },
    { value: 'back', label: t('wardrobe.view_back') },
  ]);

  let capeOptions = $derived([
    { id: 'none', name: t('wardrobe.cape_none'), desc: t('wardrobe.cape_none_desc') },
    { id: 'custom', name: t('wardrobe.cape_custom'), desc: t('wardrobe.cape_custom_desc') },
  ]);

  async function callTauri<T>(command: string, args: Record<string, any> = {}): Promise<T> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(command, args);
    } catch (e: any) {
      throw new Error(e?.message || String(e));
    }
  }

  let prevOnline = network.isOnline;
  $effect(() => {
    const currentOnline = network.isOnline;
    if (currentOnline && !prevOnline) {
      loadActiveState();
      loadHistory();
    }
    prevOnline = currentOnline;
  });

  onMount(() => {
    loadActiveState();
    loadHistory();

    hasWebGL = isWebGLSupported(canvas3dRef);
    if (hasWebGL && canvas3dRef) {
      try {
        renderer3d = new Skin3DRenderer({
          canvas: canvas3dRef,
          model: selectedModel,
          animation: selectedAnimation,
        });
        update3DTexture();
      } catch {
        hasWebGL = false;
        displayMode = '2d';
      }
    } else {
      displayMode = '2d';
    }

    render2DPreview();

    return () => {
      renderer3d?.destroy();
      renderer3d = null;
    };
  });

  $effect(() => {
    // Re-render preview whenever skin, cape, model, view, mode, or animation changes
    const _skin = skinBase64;
    const _cape = selectedCapeId;
    const _model = selectedModel;
    const _view = selectedView;
    const _customCape = customCapeBase64;
    const _mode = displayMode;
    const _anim = selectedAnimation;

    render2DPreview();
    if (hasWebGL && renderer3d) {
      if (_mode === '3d') {
        renderer3d.resume();
        renderer3d.setAnimation(_anim);
        update3DTexture();
      } else {
        renderer3d.pause();
      }
    }
  });

  function update3DTexture() {
    if (!renderer3d) return;

    renderer3d.updateModelGeometry(selectedModel);

    // 1. Skin Texture
    if (skinBase64) {
      const img = new Image();
      img.onload = () => {
        renderer3d?.setSkinImage(img);
      };
      img.src = skinBase64.startsWith('data:') ? skinBase64 : `data:image/png;base64,${skinBase64}`;
    } else {
      const defSkinCanvas = createDefaultSkinCanvas(selectedModel);
      renderer3d.setSkinImage(defSkinCanvas);
    }

    // 2. Cape Texture
    if (selectedCapeId === 'none') {
      renderer3d.setHasCape(false);
    } else if (selectedCapeId === 'custom' && customCapeBase64) {
      const capeImg = new Image();
      capeImg.onload = () => {
        renderer3d?.setCapeImage(capeImg);
      };
      capeImg.src = customCapeBase64.startsWith('data:')
        ? customCapeBase64
        : `data:image/png;base64,${customCapeBase64}`;
    } else {
      renderer3d.setHasCape(false);
    }
  }

  async function loadActiveState() {
    try {
      const state = await callTauri<WardrobeStateDto>(COMMANDS.WARDROBE_GET_STATE);
      if (state) {
        skinName =
          state.skin_name && state.skin_name !== 'Steve (Классический)'
            ? state.skin_name
            : 'Стандартный скин';
        skinBase64 = state.skin_base64 || null;
        selectedModel = state.model === 'Slim' ? 'slim' : 'default';
        selectedCapeId = state.cape_id || 'none';
        customCapeBase64 = state.cape_base64 || null;
      }
    } catch {
      // Offline fallback: Standard skin
      skinName = 'Стандартный скин';
    }
  }

  async function loadHistory() {
    if (initialHistory !== undefined) return;
    try {
      const res = await callTauri<{ items: WardrobeItemDto[] } | WardrobeItemDto[]>(
        COMMANDS.EXTRAS_LIST_SKIN_HISTORY,
        { request: { history_dir: '' } }
      );
      if (Array.isArray(res)) {
        historyItems = res;
      } else if (res && Array.isArray(res.items)) {
        historyItems = res.items;
      }
    } catch {
      historyItems = [];
    }
  }

  function render2DPreview() {
    if (!canvas2dRef) return;
    const ctx = canvas2dRef.getContext('2d');
    if (!ctx) return;

    const width = 160;
    const height = 320;
    canvas2dRef.width = width;
    canvas2dRef.height = height;
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
    if (selectedCapeId === 'custom' && customCapeBase64) {
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
      errorMessage = 'Файл не в формате PNG. Скин должен быть в формате .png';
      ontoast?.('error', 'Ошибка формата', errorMessage);
      return;
    }

    if (file.size > 2 * 1024 * 1024) {
      errorMessage = 'Размер файла скина превышает 2 МБ';
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

          // Client-side quick validation of image resolution
          const testImg = new Image();
          testImg.onerror = () => {
            errorMessage = 'Файл повреждён или не является изображением PNG';
            ontoast?.('error', 'Ошибка файла', errorMessage);
            isValidating = false;
          };
          testImg.onload = async () => {
            const w = testImg.naturalWidth;
            const h = testImg.naturalHeight;
            if (!((w === 64 && h === 64) || (w === 64 && h === 32))) {
              errorMessage = `Размер скина ${w}×${h} не поддерживается. Разрешение должно быть ровно 64×64 или 64×32 пикселя`;
              ontoast?.('error', 'Неверный размер', errorMessage);
              isValidating = false;
              return;
            }

            try {
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
              if (valRes.height === 32 || h === 32) {
                try {
                  const conv = await callTauri<{ converted_bytes_base64: string }>(
                    COMMANDS.EXTRAS_CONVERT_SKIN,
                    { request: { skin_bytes_base64: base64Data } }
                  );
                  finalBase64 = conv.converted_bytes_base64;
                  ontoast?.('info', 'Формат обновлён', 'Старый скин 64×32 сконвертирован в 64×64');
                } catch {
                  // Keep original if conversion fails
                }
              }

              skinBase64 = finalBase64;
              skinName = file.name.replace(/\.[^/.]+$/, '');

              // Detect slim model (Alex) by checking transparent pixels
              detectModelFromBase64(finalBase64);

              ontoast?.('success', 'Скин загружен', file.name);
            } catch (err: any) {
              // Fallback for offline / headless testing: allow validated 64x64/64x32
              skinBase64 = base64Data;
              skinName = file.name.replace(/\.[^/.]+$/, '');
              detectModelFromBase64(base64Data);
              ontoast?.('success', 'Скин загружен', file.name);
            } finally {
              isValidating = false;
            }
          };
          testImg.src = dataUrl;
        } catch (err: any) {
          errorMessage = err?.message || 'Ошибка обработки файла';
          ontoast?.('error', 'Ошибка загрузки', errorMessage ?? undefined);
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
    skinName = 'Стандартный скин';
    selectedModel = 'default';
    selectedCapeId = 'none';
    customCapeBase64 = null;
    errorMessage = null;
    ontoast?.('info', 'Сброс выполнен', 'Восстановлен стандартный скин');
  }

  async function selectHistorySkin(item: WardrobeItemDto) {
    try {
      skinName = item.name;
      selectedModel = item.model === 'slim' ? 'slim' : 'default';
      errorMessage = null;

      const res = await callTauri<{ item: WardrobeItemDto; skin_bytes_base64: string }>(
        COMMANDS.WARDROBE_GET_SKIN,
        { request: { history_dir: '', item_id: item.id } }
      );

      if (res?.skin_bytes_base64) {
        skinBase64 = res.skin_bytes_base64;
      }
      ontoast?.('info', 'Скин выбран', item.name);
    } catch {
      // Offline fallback: keep metadata
      ontoast?.('info', 'Выбран скин', item.name);
    }
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
  <!-- Honest Offline State Banner -->
  {#if !network.isOnline}
    <div class="wardrobe-offline-banner" role="status" aria-live="polite">
      <div class="offline-banner-left">
        <Icon name="wifi-off" size={16} color="var(--color-system-orange)" />
        <div class="offline-banner-text">
          <span class="offline-banner-title">{t('offline.title')}</span>
          <span class="offline-banner-desc">
            {i18n.locale === 'en' ? 'Local skins and 3D preview are available. Changes will sync when online.' : 'Локальные скины и 3D-просмотр работают офлайн. Синхронизация произойдёт при подключении.'}
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
          {#if hasWebGL}
            <div class="mode-control-row">
              <SegmentedControl
                options={displayModeOptions}
                bind:value={displayMode}
                size="small"
              />
            </div>
          {/if}

          {#if displayMode === '3d' && hasWebGL}
            <div class="animation-control-row">
              <div class="animation-control-wrap">
                <SegmentedControl
                  options={animationOptions}
                  bind:value={selectedAnimation}
                  size="small"
                />
              </div>
              <button
                class="reset-angle-btn"
                onclick={() => renderer3d?.resetOrientation()}
                title="Сбросить угол обзора"
                type="button"
              >
                Сброс
              </button>
            </div>
          {:else}
            <div class="view-control-row">
              <SegmentedControl
                options={viewOptions}
                bind:value={selectedView}
                size="small"
              />
            </div>
          {/if}
        </div>

        <div class="canvas-container">
          <canvas
            bind:this={canvas3dRef}
            width="240"
            height="320"
            class="character-canvas character-canvas-3d"
            class:hidden={!hasWebGL || displayMode !== '3d'}
            aria-label="3D интерактивная модель персонажа"
          ></canvas>
          <canvas
            bind:this={canvas2dRef}
            width="160"
            height="320"
            class="character-canvas character-canvas-2d"
            class:hidden={hasWebGL && displayMode === '3d'}
            aria-label="2D предпросмотр персонажа"
          ></canvas>

          {#if hasWebGL && displayMode === '3d'}
            <span class="rotation-hint">Потяните мышью для вращения</span>
          {/if}
        </div>

        <div class="preview-status">
          <div class="status-title visually-hidden">{skinName}</div>
          <div class="skin-name-field">
            <input
              type="text"
              class="skin-name-input"
              bind:value={skinName}
              placeholder="Имя скина"
              aria-label="Имя скина"
            />
          </div>
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
            <span>{t('wardrobe.upload_skin')}</span>
          </Button>

          <Button
            variant="ghost"
            size="small"
            onclick={handleResetDefault}
          >
            <span>{t('wardrobe.reset_default')}</span>
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
      <List header={i18n.locale === 'en' ? 'Character Model' : 'Тип модели персонажа'}>
        <div class="list-control-wrap">
          <SegmentedControl
            options={modelOptions}
            bind:value={selectedModel}
            size="regular"
          />
        </div>
      </List>

      <!-- Capes Selection Section -->
      <List header={i18n.locale === 'en' ? 'Player Cape' : 'Плащ игрока'}>
        {#each capeOptions as cape (cape.id)}
          {@const isSelected = selectedCapeId === cape.id}
          {#if cape.id === 'custom'}
            <Row
              title={cape.name}
              subtitle={cape.desc}
            >
              {#snippet trailing()}
                <div class="cape-row-accessory">
                  <Button
                    variant="ghost"
                    size="small"
                    onclick={() => {
                      capeInputRef?.click();
                    }}
                  >
                    {customCapeBase64
                      ? (i18n.locale === 'en' ? 'Change…' : 'Сменить...')
                      : (i18n.locale === 'en' ? 'Browse…' : 'Обзор...')}
                  </Button>

                  <Button
                    variant={isSelected ? 'primary' : 'secondary'}
                    size="small"
                    role="radio"
                    ariaChecked={isSelected}
                    ariaLabel={`${i18n.locale === 'en' ? 'Select' : 'Выбрать'} ${cape.name}`}
                    onclick={() => {
                      selectedCapeId = cape.id;
                      if (!customCapeBase64) {
                        capeInputRef?.click();
                      }
                    }}
                  >
                    {isSelected ? (i18n.locale === 'en' ? 'Selected' : 'Выбрано') : (i18n.locale === 'en' ? 'Select' : 'Выбрать')}
                  </Button>
                </div>
              {/snippet}
            </Row>
          {:else}
            <Row
              title={cape.name}
              subtitle={cape.desc}
              clickable
              role="radio"
              ariaChecked={isSelected}
              onclick={() => {
                selectedCapeId = cape.id;
              }}
            >
              {#snippet trailing()}
                <div class="cape-row-accessory">
                  {#if isSelected}
                    <span class="check-mark" aria-label={i18n.locale === 'en' ? 'Selected' : 'Выбрано'}>
                      <Icon name="check" size={16} color="var(--color-system-blue)" />
                    </span>
                  {/if}
                </div>
              {/snippet}
            </Row>
          {/if}
        {/each}
      </List>

      <!-- Library of Skins Section -->
      <List header={i18n.locale === 'en' ? 'Skins Library' : 'Библиотека скинов'} footer={i18n.locale === 'en' ? 'Previously applied skins are stored locally in the Wex directory.' : 'Ранее загруженные скины сохраняются локально в папке Wex.'}>
        {#if historyItems.length === 0}
          <div class="empty-history">
            <span class="empty-text">{t('wardrobe.history_empty')}</span>
          </div>
        {:else}
          {#each historyItems as item (item.id || item.created_utc)}
            {@const isSelected = skinName === item.name}
            <Row
              title={item.name}
              subtitle={`${item.model === 'slim' ? (i18n.locale === 'en' ? 'Slim' : 'Тонкая') : (i18n.locale === 'en' ? 'Classic' : 'Классическая')}${item.created_utc ? ` • ${item.created_utc.slice(0, 10)}` : ''}`}
            >
              {#snippet trailing()}
                <div class="history-actions">
                  {#if isSelected}
                    <span class="library-check-mark" aria-label="Выбран">
                      <Icon name="check" size={14} color="var(--color-system-blue)" />
                    </span>
                  {/if}
                  <Button
                    variant={isSelected ? 'primary' : 'secondary'}
                    size="small"
                    onclick={() => selectHistorySkin(item)}
                  >
                    {isSelected ? (i18n.locale === 'en' ? 'Selected' : 'Выбран') : (i18n.locale === 'en' ? 'Select' : 'Выбрать')}
                  </Button>
                  <Button
                    variant="ghost"
                    size="small"
                    title={i18n.locale === 'en' ? 'Remove from Library' : 'Удалить скин'}
                    ariaLabel={i18n.locale === 'en' ? 'Remove from Library' : 'Удалить из истории'}
                    onclick={(e) => deleteHistorySkin(item, e)}
                  >
                    <Icon name="close" size={14} color="var(--color-secondary-label)" />
                  </Button>
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
          <span>{isApplying ? t('wardrobe.applying') : t('wardrobe.apply_to_game')}</span>
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
    gap: 16px;
  }

  .wardrobe-offline-banner {
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
    max-width: 960px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .offline-banner-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .offline-banner-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .offline-banner-title {
    font-size: var(--text-footnote, 13px);
    font-weight: 600;
    color: var(--color-label);
  }

  .offline-banner-desc {
    font-size: var(--text-caption-1, 11px);
    color: var(--color-secondary-label);
  }

  .wardrobe-layout {
    display: grid;
    grid-template-columns: 310px 1fr;
    gap: 24px;
    max-width: 1000px;
    margin: 0 auto;
    width: 100%;
  }

  .preview-column {
    display: flex;
    flex-direction: column;
  }

  .preview-card {
    background-color: var(--glass-card-bg);
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
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
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    margin-bottom: 12px;
  }

  .mode-control-row,
  .view-control-row {
    width: 100%;
  }

  .animation-control-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .animation-control-wrap {
    flex: 1;
  }

  .canvas-container {
    width: 100%;
    min-height: 320px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .character-canvas {
    width: 240px;
    height: 320px;
    image-rendering: pixelated;
    image-rendering: crisp-edges;
    touch-action: none;
    user-select: none;
  }

  .character-canvas-2d {
    width: 160px;
    height: 320px;
  }

  .character-canvas-3d {
    cursor: grab;
  }

  .character-canvas-3d:active {
    cursor: grabbing;
  }

  .character-canvas.hidden {
    display: none;
  }

  .rotation-hint {
    font-size: 11px;
    color: var(--color-tertiary-label);
    margin-top: 6px;
    letter-spacing: -0.01em;
  }

  .reset-angle-btn {
    appearance: none;
    background: transparent;
    border: none;
    font-size: 12px;
    color: var(--color-system-blue);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-weight: 500;
    transition: background var(--motion-duration-short) ease;
  }

  .reset-angle-btn:hover {
    background: var(--color-fill-secondary);
  }

  .preview-status {
    margin-top: 16px;
    text-align: center;
    width: 100%;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .skin-name-field {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .skin-name-input {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    text-align: center;
    padding: 2px 8px;
    width: 100%;
    max-width: 220px;
    transition: border-color var(--duration-fast), background-color var(--duration-fast);
  }

  .skin-name-input:hover {
    border-color: var(--color-quaternary-label);
    background-color: var(--color-tertiary-fill);
  }

  .skin-name-input:focus {
    outline: none;
    border-color: var(--color-system-blue);
    background-color: var(--color-secondary-fill);
  }

  .status-subtitle {
    font-size: var(--text-footnote);
    color: var(--color-secondary-label);
    font-weight: 500;
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
    gap: 8px;
  }

  .check-mark {
    display: flex;
    align-items: center;
  }

  .empty-history {
    padding: 24px 16px;
    text-align: center;
    background-color: var(--card-bg);
    backdrop-filter: blur(20px) saturate(160%);
    -webkit-backdrop-filter: blur(20px) saturate(160%);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
  }

  .empty-text {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    font-weight: 500;
  }

  .history-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .library-check-mark {
    display: flex;
    align-items: center;
  }


  .apply-bar {
    margin-top: 8px;
    display: flex;
    justify-content: flex-end;
  }

  /* Scenery Art Contrast Parity (wex-contrast): text over art is always white with shadow */
  :global(.wardrobe-screen .list-header) {
    color: rgba(255, 255, 255, 0.94) !important;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.75), 0 1px 2px rgba(0, 0, 0, 0.90);
    font-weight: 600;
  }

  :global(.wardrobe-screen .list-footer) {
    color: rgba(255, 255, 255, 0.72) !important;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.70);
  }
</style>
