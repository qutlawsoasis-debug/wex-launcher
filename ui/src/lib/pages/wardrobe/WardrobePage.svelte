<!--
  Wardrobe (reference: V.wardrobe). A 3D model of the player made of CSS boxes
  textured from the real 64×64 skin: head with the hat layer, body, arms (wide or
  slim), legs, outer layers, cape. Drag to turn, ±90° buttons, auto-turn, poses
  (stands, walks, runs, waves), nickname above the head.
  Tabs: Skins (upload PNG 64×64 / 64×32, saved skins, arm model), Capes, Profile.
  Every change is put on through core (wardrobe_apply).
  HIG › Tab views; HIG › Segmented controls; HIG › Text fields; HIG › Motion.
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { builds } from '../../shell/builds.svelte';
  import { toasts } from '../../shell/toast.svelte';
  import { indicator } from '../../shell/indicator';
  import { pickImageFile } from '../../shell/filePick';
  import { formatError } from '../../design/errors';
  import { createDefaultSkinCanvas } from './defaultSkin';
  import { t } from '../../i18n';
  import { buildRig, drawThumb, loadImage, toSkinCanvas, isCapeSize, NICK_RE, type CapeTexture } from './skinRig';
  import { wardrobe, fileToBase64, looksSlim, safeName, type SavedSkin } from './wardrobe.svelte';
  import { call } from '../../shell/ipc';
  import { formatDate } from '../../shell/format';

  type Tab = 'skins' | 'capes' | 'profile';
  type Pose = 'idle' | 'walk' | 'run' | 'wave';

  let tab = $state<Tab>('skins');
  let pose = $state<Pose>('idle');
  let turn = $state(-24);
  let instant = $state(false);
  let auto = $state(false);
  let rigEl: HTMLElement | undefined = $state();
  let tex = $state<HTMLCanvasElement | null>(null);
  let cape = $state<CapeTexture | null>(null);
  let nick = $state('');
  let nickTouched = false;
  let nickOk = $state(true);
  let busy = $state(false);

  /** Achievements are kept locally by core (social_list_achievements): definitions and progress. */
  interface Achievement {
    id: string;
    title: string;
    description: string;
    target: number;
    current: number;
    unlocked: boolean;
    unlockedAt: string | null;
  }
  let achievements = $state<Achievement[]>([]);
  let showAch = $state(false);
  const unlocked = $derived(achievements.filter((a) => a.unlocked).length);

  async function loadAchievements() {
    try {
      const r = await call<{
        achievements: Array<{ id: string; title: string; description: string; target: number }>;
        progress: Array<{ id: string; current: number; unlocked: boolean; unlocked_at?: string | null }>;
      }>('social_list_achievements', {});
      const prog = new Map((r?.progress ?? []).map((p) => [p.id, p]));
      achievements = (r?.achievements ?? []).map((d) => {
        const p = prog.get(d.id);
        return { ...d, current: p?.current ?? 0, unlocked: p?.unlocked ?? false, unlockedAt: p?.unlocked_at ?? null };
      });
    } catch {
      achievements = [];
    }
  }

  const slim = $derived(wardrobe.state.model === 'Slim');
  const skinLabel = $derived(wardrobe.state.skin_base64 ? wardrobe.state.skin_name || t('wardrobe2.skin') : t('wardrobe2.default_skin'));

  onMount(() => {
    void loadAchievements();
    void wardrobe.load().then(() => {
      // Do not overwrite what the user has started typing.
      if (!nickTouched) nick = wardrobe.state.nickname || builds.playerName;
    });
  });

  // Texture from the current skin; the standard look when no skin is set.
  $effect(() => {
    const b64 = wardrobe.state.skin_base64;
    const model = slim ? 'slim' : 'default';
    if (!b64) {
      tex = createDefaultSkinCanvas(model);
      return;
    }
    let alive = true;
    loadImage(`data:image/png;base64,${b64}`)
      .then((img) => alive && (tex = toSkinCanvas(img)))
      .catch(() => alive && (tex = createDefaultSkinCanvas(model)));
    return () => (alive = false);
  });

  $effect(() => {
    const b64 = wardrobe.state.cape_id === 'custom' ? wardrobe.state.cape_base64 : null;
    if (!b64) {
      cape = null;
      return;
    }
    let alive = true;
    loadImage(`data:image/png;base64,${b64}`)
      .then((img) => alive && (cape = { canvas: img, scale: Math.max(1, Math.round(img.width / 64)) }))
      .catch(() => alive && (cape = null));
    return () => (alive = false);
  });

  $effect(() => {
    if (rigEl && tex) buildRig(rigEl, tex, slim, cape);
  });

  // Turning: drag, buttons, auto-turn (requestAnimationFrame, stops when off or the page closes).
  let drag: { x: number; base: number } | null = null;
  function down(e: PointerEvent) {
    if ((e.target as HTMLElement).closest('button')) return;
    drag = { x: e.clientX, base: turn };
    instant = true;
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (drag) turn = drag.base + (e.clientX - drag.x) * 0.6;
  }
  function up() {
    drag = null;
    instant = false;
  }
  function onStageKey(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') turn -= 15;
    else if (e.key === 'ArrowRight') turn += 15;
    else return;
    e.preventDefault();
  }

  let raf = 0;
  $effect(() => {
    if (!auto) return;
    let last = performance.now();
    instant = true;
    const step = (now: number) => {
      turn += (now - last) * 0.03;
      last = now;
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => {
      cancelAnimationFrame(raf);
      instant = false;
    };
  });
  onDestroy(() => cancelAnimationFrame(raf));

  /** Show the back so the cape is visible (reference). */
  function faceBack() {
    turn = Math.round((turn - 180) / 360) * 360 + 180;
  }

  async function put(patch: Parameters<typeof wardrobe.apply>[0], done?: string) {
    busy = true;
    try {
      await wardrobe.apply(patch);
      if (done) toasts.show(done);
    } catch (e) {
      toasts.show(t('wardrobe2.apply_failed'), formatError(e, 'general').description);
    } finally {
      busy = false;
    }
  }

  async function uploadSkin() {
    const file = await pickImageFile();
    if (!file) return;
    try {
      const raw = await fileToBase64(file);
      const img = await loadImage(`data:image/png;base64,${raw}`);
      if (img.width !== 64 || (img.height !== 64 && img.height !== 32)) {
        toasts.show(t('wardrobe2.bad_size', { w: img.width, h: img.height }));
        return;
      }
      const b64 = await wardrobe.prepareSkin(raw);
      const cv = toSkinCanvas(await loadImage(`data:image/png;base64,${b64}`));
      const ctx = cv.getContext('2d');
      const model = ctx && looksSlim(ctx) ? 'Slim' : 'Classic';
      await put({ skin_base64: b64, skin_name: safeName(file.name), model }, t('wardrobe2.skin_on'));
      void wardrobe.loadSaved();
    } catch (e) {
      toasts.show(t('wardrobe2.bad_skin'), formatError(e, 'general').description);
    }
  }

  function wearSaved(s: SavedSkin) {
    if (!s.base64) return;
    void put({ skin_base64: s.base64, skin_name: s.name, model: s.model }, t('wardrobe2.skin_on'));
  }

  async function uploadCape() {
    const file = await pickImageFile();
    if (!file) return;
    try {
      const b64 = await fileToBase64(file);
      const img = await loadImage(`data:image/png;base64,${b64}`);
      if (!isCapeSize(img.width, img.height)) {
        toasts.show(t('wardrobe2.bad_cape', { w: img.width, h: img.height }));
        return;
      }
      await put({ cape_id: 'custom', cape_name: safeName(file.name), cape_base64: b64 }, t('wardrobe2.cape_on'));
      faceBack();
    } catch (e) {
      toasts.show(t('wardrobe2.bad_cape_file'), formatError(e, 'general').description);
    }
  }

  function chooseCape(id: 'none' | 'custom') {
    if (id === 'custom') {
      if (wardrobe.state.cape_base64) {
        void put({ cape_id: 'custom' }).then(faceBack);
      } else void uploadCape();
    } else void put({ cape_id: 'none', cape_name: '' });
  }

  let nickTimer: ReturnType<typeof setTimeout> | null = null;
  function onNick(e: Event) {
    nickTouched = true;
    nick = (e.currentTarget as HTMLInputElement).value;
    nickOk = NICK_RE.test(nick);
    if (nickTimer) clearTimeout(nickTimer);
    if (!nickOk || nick === wardrobe.state.nickname) return;
    nickTimer = setTimeout(async () => {
      try {
        await wardrobe.apply({ nickname: nick });
        await builds.saveSettings({ player_name: nick });
      } catch (e) {
        toasts.show(t('wardrobe2.apply_failed'), formatError(e, 'settings').description);
      }
    }, 600);
  }

  function thumb(node: HTMLCanvasElement, s: { b64: string | null; slim: boolean }) {
    const paint = async (p: { b64: string | null; slim: boolean }) => {
      const img = p.b64 ? await loadImage(`data:image/png;base64,${p.b64}`).catch(() => null) : null;
      const src = img ? toSkinCanvas(img) : createDefaultSkinCanvas(p.slim ? 'slim' : 'default');
      drawThumb(node, src, p.slim);
    };
    void paint(s);
    return { update: (n: typeof s) => void paint(n) };
  }

  function capeSwatch(node: HTMLCanvasElement, b64: string | null) {
    const paint = async (v: string | null) => {
      const c = node.getContext('2d');
      node.width = 10;
      node.height = 16;
      if (!c) return;
      c.clearRect(0, 0, 10, 16);
      if (!v) return;
      const img = await loadImage(`data:image/png;base64,${v}`).catch(() => null);
      if (!img) return;
      const k = Math.max(1, Math.round(img.width / 64));
      c.imageSmoothingEnabled = false;
      c.drawImage(img, 1 * k, 1 * k, 10 * k, 16 * k, 0, 0, 10, 16);
    };
    void paint(b64);
    return { update: (v: string | null) => void paint(v) };
  }

  const POSES: Array<[Pose, string]> = [
    ['idle', 'wardrobe2.pose_idle'],
    ['walk', 'wardrobe2.pose_walk'],
    ['run', 'wardrobe2.pose_run'],
    ['wave', 'wardrobe2.pose_wave'],
  ];
  const TABS: Array<[Tab, string]> = [
    ['skins', 'wardrobe2.tab_skins'],
    ['capes', 'wardrobe2.tab_capes'],
    ['profile', 'wardrobe2.tab_profile'],
  ];
</script>

<div class="page-title">
  <h1>{t('shell.nav_wardrobe')}</h1>
  <p>{t('wardrobe2.subtitle')}</p>
</div>

<div class="wd2">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <section
    class="stage3d"
    aria-label={t('wardrobe2.stage')}
    tabindex="0"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={onStageKey}
  >
    <div class="nametag">{nickOk ? nick || wardrobe.state.nickname : wardrobe.state.nickname}</div>
    <div class="rig-wrap"><div class="rig pose-{pose}" class:now={instant} style:--turn="{turn}deg" bind:this={rigEl}></div></div>
    <div class="floor" aria-hidden="true"></div>
    <div class="stage-top">
      <b>{skinLabel}</b>
      <span>{slim ? t('wardrobe2.model_slim_long') : t('wardrobe2.model_wide_long')}{#if wardrobe.state.cape_id === 'custom'}, {t('wardrobe2.cape_named', { name: wardrobe.state.cape_name || t('wardrobe2.cape_own') })}{/if}</span>
    </div>
    <div class="stage-bar">
      <div class="seg" role="group" aria-label={t('wardrobe2.pose')} use:indicator={{ selector: '[aria-pressed="true"]', key: pose }}>
        {#each POSES as [id, label] (id)}
          <button type="button" aria-pressed={pose === id} onclick={() => (pose = id)}>{t(label)}</button>
        {/each}
      </div>
      <div class="stage-tools">
        <button type="button" class="icon-b" aria-label={t('wardrobe2.turn_left')} title={t('wardrobe2.turn_left')} onclick={() => (turn -= 90)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 4H2.5V1.5M2.8 4A6 6 0 1 1 2 8" /></svg>
        </button>
        <button type="button" class="icon-b" aria-label={t('wardrobe2.turn_right')} title={t('wardrobe2.turn_right')} onclick={() => (turn += 90)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M11 4h2.5V1.5M13.2 4A6 6 0 1 0 14 8" /></svg>
        </button>
        <button type="button" class="icon-b" aria-pressed={auto} aria-label={t('wardrobe2.auto_turn')} title={t('wardrobe2.auto_turn')} onclick={() => (auto = !auto)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="5.5" /><path d="M8 5v3l2 1.5" /></svg>
        </button>
      </div>
    </div>
  </section>

  <section class="wd-panel">
    <div class="tabs" role="tablist" aria-label={t('shell.nav_wardrobe')} use:indicator={{ selector: '[aria-selected="true"]', key: tab, kind: 'underline' }}>
      {#each TABS as [id, label] (id)}
        <button type="button" role="tab" aria-selected={tab === id} onclick={() => (tab = id)}>{t(label)}</button>
      {/each}
    </div>
    <div class="wd-body" role="tabpanel">
      {#if tab === 'skins'}
        <button type="button" class="drop" onclick={uploadSkin} disabled={busy}>
          <b>{t('wardrobe2.upload')}</b>
          <span>{t('wardrobe2.upload_hint')}</span>
        </button>
        <div class="skin-grid">
          {#if !wardrobe.saved.some((s) => s.base64 && s.base64 === wardrobe.state.skin_base64)}
            <button type="button" class="skin-card" aria-pressed="true">
              <canvas class="px" use:thumb={{ b64: wardrobe.state.skin_base64, slim }} aria-hidden="true"></canvas>
              <b>{skinLabel}</b>
              <span>{t('wardrobe2.worn')}</span>
            </button>
          {/if}
          {#each wardrobe.saved as s (s.id)}
            {@const on = !!s.base64 && s.base64 === wardrobe.state.skin_base64}
            <button type="button" class="skin-card" aria-pressed={on} disabled={!s.base64 || busy} onclick={() => wearSaved(s)}>
              <canvas class="px" use:thumb={{ b64: s.base64, slim: s.model === 'Slim' }} aria-hidden="true"></canvas>
              <b>{s.name}</b>
              <span>{on ? t('wardrobe2.worn') : s.model === 'Slim' ? t('wardrobe2.model_slim_long') : t('wardrobe2.model_wide_long')}</span>
            </button>
          {/each}
        </div>
        <div class="wrow">
          <div>
            <div class="l">{t('wardrobe2.arms')}</div>
            <div class="d">{t('wardrobe2.arms_d')}</div>
          </div>
          <div class="seg" role="group" aria-label={t('wardrobe2.arms')} use:indicator={{ selector: '[aria-pressed="true"]', key: slim }}>
            <button type="button" aria-pressed={!slim} disabled={busy} onclick={() => slim && put({ model: 'Classic' })}>{t('wardrobe2.model_wide')}</button>
            <button type="button" aria-pressed={slim} disabled={busy} onclick={() => !slim && put({ model: 'Slim' })}>{t('wardrobe2.model_slim')}</button>
          </div>
        </div>
      {:else if tab === 'capes'}
        <div class="cape-grid">
          <button type="button" class="cape-card" aria-pressed={wardrobe.state.cape_id === 'custom'} disabled={busy} onclick={() => chooseCape('custom')}>
            <canvas class="cape-sw px" use:capeSwatch={wardrobe.state.cape_base64} aria-hidden="true"></canvas>
            <b>{t('wardrobe2.cape_own')}</b>
            <span>{wardrobe.state.cape_base64 ? wardrobe.state.cape_name || t('wardrobe2.cape_loaded') : t('wardrobe2.cape_upload')}</span>
          </button>
          <button type="button" class="cape-card" aria-pressed={wardrobe.state.cape_id !== 'custom'} disabled={busy} onclick={() => chooseCape('none')}>
            <span class="cape-sw none" aria-hidden="true"></span>
            <b>{t('wardrobe2.cape_none')}</b>
            <span>{t('wardrobe2.cape_none_d')}</span>
          </button>
        </div>
        {#if wardrobe.state.cape_base64}
          <button type="button" class="btn" disabled={busy} onclick={uploadCape}>{t('wardrobe2.cape_replace')}</button>
        {/if}
        <p class="hint">{t('wardrobe2.cape_hint')}</p>
      {:else}
        <div class="wrow col">
          <label class="l" for="wd-nick">{t('wardrobe2.nick')}</label>
          <input id="wd-nick" class="field" class:bad={!nickOk} bind:value={nick} oninput={onNick} maxlength="16" autocomplete="off" spellcheck="false" aria-invalid={!nickOk} aria-describedby="wd-nick-hint" />
          <div class="d" class:bad={!nickOk} id="wd-nick-hint">{nickOk ? t('wardrobe2.nick_hint') : t('wardrobe2.nick_bad')}</div>
        </div>
        {#if achievements.length}
          <button type="button" class="wrow ach-row" aria-expanded={showAch} aria-controls="ach-list" onclick={() => (showAch = !showAch)}>
            <div>
              <div class="l">{t('wardrobe2.achievements')}</div>
              <div class="d">{t('wardrobe2.achievements_count', { n: unlocked, m: achievements.length })}</div>
            </div>
            <svg class="chev" class:open={showAch} viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3l5 5-5 5" /></svg>
          </button>
          {#if showAch}
            <div class="list" id="ach-list">
              {#each achievements as a (a.id)}
                <div class="lrow lrow-ach" class:done={a.unlocked}>
                  <span class="ach-ic" aria-hidden="true">
                    {#if a.unlocked}<svg viewBox="0 0 16 16"><path d="M3 8.5l3.2 3L13 4.5" /></svg>{/if}
                  </span>
                  <div>
                    <b>{a.title}</b>
                    <span>{a.description}</span>
                  </div>
                  {#if a.unlocked}
                    <span class="tag up">{a.unlockedAt ? t('wardrobe2.ach_got_on', { date: formatDate(a.unlockedAt) }) : t('wardrobe2.ach_got')}</span>
                  {:else}
                    <div class="ach-meter" role="progressbar" aria-label={a.title} aria-valuemin="0" aria-valuemax={a.target} aria-valuenow={Math.min(a.current, a.target)}>
                      <span>{t('wardrobe2.ach_progress', { n: Math.floor(Math.min(a.current, a.target)), m: a.target })}</span>
                      <i style:--w={(Math.min(a.current, a.target) / Math.max(1, a.target)).toFixed(3)}></i>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      {/if}
    </div>
  </section>
</div>

<style>
  .page-title h1 {
    font-size: var(--t-t1);
    font-weight: var(--w-display);
    letter-spacing: -0.02em;
  }
  .page-title p {
    margin: 4px 0 0;
    color: var(--label-2);
  }
  .wd2 {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(320px, 1fr);
    gap: 20px;
    align-items: start;
  }
  .stage3d {
    position: relative;
    height: 560px;
    border-radius: var(--r-l);
    overflow: hidden;
    background:
      radial-gradient(60% 50% at 50% 42%, var(--stage-a), transparent 70%),
      var(--card);
    box-shadow: inset 0 0 0 1px var(--edge);
    cursor: grab;
    touch-action: pan-y;
    user-select: none;
    perspective: 1400px;
  }
  .stage3d:active {
    cursor: grabbing;
  }
  .rig-wrap {
    position: absolute;
    left: 50%;
    top: 53%;
    transform-style: preserve-3d;
    transform: rotateX(-8deg);
  }
  .rig {
    position: relative;
    transform-style: preserve-3d;
    transform: rotateY(var(--turn, 0deg));
    transition: transform 700ms var(--spring);
  }
  .rig.now {
    transition: none;
  }
  .rig :global(.joint) {
    position: absolute;
    left: 0;
    top: 0;
    transform-style: preserve-3d;
  }
  .rig :global(.swing) {
    transform-style: preserve-3d;
    transform-origin: 0 0;
  }
  .rig :global(.box),
  .rig :global(.box-group) {
    position: absolute;
    left: 0;
    top: 0;
    transform-style: preserve-3d;
  }
  .rig :global(.box-group > .box) {
    transform: none !important;
  }
  .rig :global(.face) {
    position: absolute;
    left: 0;
    top: 0;
    image-rendering: pixelated;
    backface-visibility: hidden;
  }
  .rig :global(.overlay .face) {
    backface-visibility: visible;
  }
  /* Poses: joints swing around the shoulder and hip (reference). */
  .rig.pose-idle :global(.j-armR .swing) {
    animation: idleR 3.2s ease-in-out infinite;
  }
  .rig.pose-idle :global(.j-armL .swing) {
    animation: idleL 3.2s ease-in-out infinite;
  }
  .rig.pose-walk :global(.j-armR .swing),
  .rig.pose-walk :global(.j-legL .swing) {
    animation: swingA 1s ease-in-out infinite;
  }
  .rig.pose-walk :global(.j-armL .swing),
  .rig.pose-walk :global(.j-legR .swing) {
    animation: swingB 1s ease-in-out infinite;
  }
  .rig.pose-run :global(.j-armR .swing),
  .rig.pose-run :global(.j-legL .swing) {
    animation: swingA2 0.56s ease-in-out infinite;
  }
  .rig.pose-run :global(.j-armL .swing),
  .rig.pose-run :global(.j-legR .swing) {
    animation: swingB2 0.56s ease-in-out infinite;
  }
  .rig.pose-wave :global(.j-armR .swing) {
    animation: wave 1.1s ease-in-out infinite;
  }
  .rig :global(.j-cape .swing) {
    transition: transform 600ms var(--ease);
  }
  .rig.pose-idle :global(.j-cape .swing),
  .rig.pose-wave :global(.j-cape .swing) {
    transform: rotateX(-6deg);
  }
  .rig.pose-walk :global(.j-cape .swing) {
    transform: rotateX(-16deg);
  }
  .rig.pose-run :global(.j-cape .swing) {
    transform: rotateX(-38deg);
  }
  @keyframes idleR {
    50% {
      transform: rotateZ(4deg);
    }
  }
  @keyframes idleL {
    50% {
      transform: rotateZ(-4deg);
    }
  }
  @keyframes swingA {
    0%,
    100% {
      transform: rotateX(32deg);
    }
    50% {
      transform: rotateX(-32deg);
    }
  }
  @keyframes swingB {
    0%,
    100% {
      transform: rotateX(-32deg);
    }
    50% {
      transform: rotateX(32deg);
    }
  }
  @keyframes swingA2 {
    0%,
    100% {
      transform: rotateX(58deg);
    }
    50% {
      transform: rotateX(-58deg);
    }
  }
  @keyframes swingB2 {
    0%,
    100% {
      transform: rotateX(-58deg);
    }
    50% {
      transform: rotateX(58deg);
    }
  }
  @keyframes wave {
    0%,
    100% {
      transform: rotateZ(150deg);
    }
    50% {
      transform: rotateZ(118deg);
    }
  }
  .floor {
    position: absolute;
    left: 50%;
    top: calc(53% + 16 * 11px);
    width: 224px;
    height: 40px;
    margin-left: -112px;
    margin-top: -20px;
    border-radius: 50%;
    background: radial-gradient(var(--floor), transparent 70%);
    pointer-events: none;
  }
  .nametag {
    position: absolute;
    left: 50%;
    top: calc(53% - 16 * 11px - 48px);
    transform: translateX(-50%);
    padding: 4px 12px;
    border-radius: var(--r-xs);
    background: var(--veil-strong);
    box-shadow: inset 0 0 0 1px var(--edge);
    font-weight: var(--w-semibold);
    font-size: var(--t-sub);
    white-space: nowrap;
    z-index: 2;
  }
  .stage-top {
    position: absolute;
    left: 20px;
    top: 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stage-top b {
    font-weight: var(--w-bold);
  }
  .stage-top span {
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .stage-bar {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    z-index: 2;
  }
  .stage-bar .seg {
    background: var(--veil-strong);
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .stage-tools {
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: var(--r-s);
    background: var(--veil-strong);
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .icon-b {
    width: 32px;
    height: 32px;
    border-radius: var(--r-xs);
    display: grid;
    place-items: center;
    color: var(--label-2);
    transition:
      background-color var(--dur-micro),
      color var(--dur-micro);
  }
  .icon-b:hover {
    background: var(--fill-2);
    color: var(--label);
  }
  .icon-b[aria-pressed='true'] {
    background: var(--blue-tint);
    color: var(--blue-text);
  }
  .icon-b svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
  .wd-panel {
    border-radius: var(--r-l);
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--edge);
    padding: 4px 20px 20px;
    min-height: 560px;
  }
  .wd-panel .tabs {
    margin: 0 -4px;
  }
  .wd-body {
    padding-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .drop {
    text-align: left;
    cursor: pointer;
  }
  .drop:hover {
    border-color: var(--blue-text);
    background: var(--blue-tint);
  }
  .skin-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 8px;
  }
  /* Selection is a fill (AGENTS.md): the reference's 2 px ring is not used. */
  .skin-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 4px 8px;
    border-radius: var(--r-m);
    background: var(--fill-3);
    transition:
      background-color var(--dur-micro),
      transform var(--dur-short) var(--ease);
  }
  .skin-card canvas {
    width: 40px;
    height: 80px;
    margin-bottom: 4px;
    image-rendering: pixelated;
    transition: transform var(--dur-short) var(--spring);
  }
  .skin-card:hover {
    background: var(--fill-2);
  }
  .skin-card:hover canvas {
    transform: translateY(-3px);
  }
  .skin-card b {
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .skin-card span {
    font-size: var(--t-cap);
    color: var(--label-3);
  }
  .skin-card[aria-pressed='true'] {
    background: var(--blue-tint);
  }
  .skin-card[aria-pressed='true'] span {
    color: var(--blue-text);
    font-weight: var(--w-semibold);
  }
  .wrow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--separator);
  }
  .wrow.col {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    border-top: 0;
    padding-top: 0;
  }
  .wrow .l {
    font-weight: var(--w-medium);
  }
  .wrow .d,
  .hint {
    font-size: var(--t-foot);
    color: var(--label-2);
    margin: 0;
  }
  .d.bad {
    color: var(--red);
  }
  .ach-row {
    width: 100%;
    text-align: left;
    cursor: pointer;
  }
  .chev {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: var(--label-3);
    stroke-width: 2;
    transition: transform var(--dur-short) var(--ease);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .lrow-ach {
    grid-template-columns: 28px minmax(0, 1fr) auto;
    min-height: 56px;
  }
  .ach-ic {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--fill-2);
  }
  .done .ach-ic {
    background: var(--green);
  }
  .ach-ic svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: var(--cover-ink);
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .tag.up {
    background: var(--blue-tint);
    color: var(--blue-text);
  }
  .ach-meter {
    display: grid;
    gap: 6px;
    width: 96px;
    font-size: var(--t-cap);
    color: var(--label-2);
    text-align: right;
  }
  .ach-meter i {
    display: block;
    height: 4px;
    border-radius: 2px;
    background: var(--fill-2);
    overflow: hidden;
  }
  .ach-meter i::after {
    content: '';
    display: block;
    height: 100%;
    background: var(--blue-text);
    transform: scaleX(var(--w));
    transform-origin: left;
  }
  .cape-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .cape-card {
    display: grid;
    grid-template-columns: 32px 1fr;
    grid-template-rows: auto auto;
    column-gap: 12px;
    align-items: center;
    text-align: left;
    padding: 12px;
    border-radius: var(--r-m);
    background: var(--fill-3);
    transition: background-color var(--dur-micro);
  }
  .cape-card:hover {
    background: var(--fill-2);
  }
  .cape-card[aria-pressed='true'] {
    background: var(--blue-tint);
  }
  .cape-sw {
    grid-row: 1 / 3;
    width: 24px;
    height: 36px;
    border-radius: 3px;
    background: var(--fill);
    box-shadow: inset 0 0 0 1px var(--edge);
    image-rendering: pixelated;
  }
  .cape-sw.none {
    background: transparent;
  }
  .cape-card b {
    font-weight: var(--w-semibold);
    font-size: var(--t-sub);
  }
  .cape-card span:last-child {
    font-size: var(--t-cap);
    color: var(--label-3);
  }
  @media (max-width: 1100px) {
    .wd2 {
      grid-template-columns: 1fr;
    }
  }
</style>
