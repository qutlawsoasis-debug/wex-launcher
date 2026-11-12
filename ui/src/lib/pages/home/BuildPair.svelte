<!--
  The pair of builds (reference: .pair / .pane). The selected build is open,
  the other stands as a column; choosing it makes the pair flow with a spring.
  HIG › Buttons: the open pane's Play is its one prominent action.
  HIG › Motion: the flow answers the user's choice and is instant under Reduce Motion.
-->
<script lang="ts">
  import { builds, coverVar, type Build } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { router, buildRoute } from '../../shell/router.svelte';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { motionReduced } from '../../design/wex-tokens';
  import { home } from './home.svelte';
  import { t, tn } from '../../i18n';

  async function choose(b: Build) {
    if (launch.busy || b.id === builds.selectedId) return;
    try {
      await builds.select(b.id);
      toasts.show(t('shell.build_selected', { ver: b.id }));
    } catch (e) {
      toasts.show(t('shell.select_failed'), formatError(e, 'settings').description);
    }
  }

  async function play(b: Build) {
    if (launch.busy) return;
    if (b.id !== builds.selectedId) await choose(b);
    if (builds.selectedId === b.id) void launch.start();
  }

  function onKey(e: KeyboardEvent, b: Build) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      void choose(b);
    }
  }

  function follow(e: PointerEvent) {
    if (motionReduced()) return;
    const pane = (e.currentTarget as HTMLElement);
    const r = pane.getBoundingClientRect();
    pane.style.setProperty('--px', ((e.clientX - r.left) / r.width - 0.5).toFixed(3));
    pane.style.setProperty('--py', ((e.clientY - r.top) / r.height - 0.5).toFixed(3));
  }

  /** "132 мода на Fabric, Java 21. 3 мира." — only facts the backend knows. */
  function summary(b: Build): string {
    const parts: string[] = [];
    const head = b.mods !== null ? t('home.mods_on_loader', { mods: tn('shell.mods', b.mods), java: b.java }) : t('home.loader_java', { java: b.java });
    parts.push(head + '.');
    const w = home.worlds[b.id];
    if (w !== undefined) parts.push(w > 0 ? tn('home.worlds', w) + '.' : t('home.no_worlds'));
    return parts.join(' ');
  }

  /** Readiness from what core reports: fast launch prepared, game files present, or not yet. */
  function readiness(b: Build): { ok: boolean; text: string } {
    if (b.id === builds.selectedId && launch.prewarm === 'ready') return { ok: true, text: t('home.ready_fast') };
    if (b.installed) return { ok: true, text: t('home.ready_installed') };
    return { ok: false, text: t('home.ready_first_download') };
  }
</script>

<section class="pair" aria-label={t('shell.builds')}>
  {#each builds.list as b (b.id)}
    {@const open = b.id === builds.selectedId}
    {@const ready = readiness(b)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="pane"
      class:open
      style:--hero-c={coverVar(b.id)}
      role={open ? undefined : 'button'}
      tabindex={open ? undefined : 0}
      onclick={() => !open && choose(b)}
      onkeydown={(e) => !open && onKey(e, b)}
      onpointermove={open ? follow : undefined}
    >
      <span class="pane-ver" aria-hidden="true">{b.id}</span>
      <!-- The visible text is the accessible name; the hint says what pressing does. -->
      <div class="pane-side" aria-hidden={open}>
        <b>{b.id}</b>
        {#if b.mods !== null}<span>{tn('shell.mods', b.mods)}</span>{/if}
        {#if !open}<span class="sr">{t('home.choose_hint')}</span>{/if}
      </div>
      {#if open}
        <div class="pane-body">
          <div class="pane-top">
            <span class="pane-chip">{t('shell.selected_for_launch')}</span>
            <a
              class="pane-link"
              href="#{buildRoute(b.id)}"
              onclick={(e) => {
                e.preventDefault();
                router.go(buildRoute(b.id));
              }}>{t('home.open_build')}</a
            >
          </div>
          <div>
            <h1>{t('shell.build_name', { ver: b.id })}</h1>
            <p>{summary(b)}</p>
          </div>
          <div class="pane-actions">
            <button type="button" class="btn big pane-play" disabled={launch.busy} onclick={() => play(b)}>
              <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 1v8l7-4z" /></svg>{t('shell.play')}
            </button>
            <span class="ready" class:upd={!ready.ok}><i aria-hidden="true"></i>{ready.text}</span>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</section>

<style>
  .pair {
    display: flex;
    gap: 12px;
    height: 380px;
  }
  .pane {
    position: relative;
    flex: 0 0 104px;
    border-radius: var(--r-l);
    overflow: hidden;
    isolation: isolate;
    background: var(--hero-c);
    color: var(--cover-ink);
    cursor: pointer;
    transition:
      flex-grow var(--dur-pair) var(--spring),
      filter var(--dur-short);
  }
  .pane.open {
    flex-grow: 1;
    cursor: default;
  }
  .pane:not(.open):hover {
    filter: brightness(1.06);
  }
  .pane-ver {
    position: absolute;
    right: -0.04em;
    bottom: -0.2em;
    z-index: -1;
    font-size: clamp(160px, 22vw, 300px);
    font-weight: var(--w-display);
    letter-spacing: -0.06em;
    line-height: 1;
    opacity: 0.16;
    pointer-events: none;
    user-select: none;
    white-space: nowrap;
    transform: translate(calc(var(--px, 0) * -18px), calc(var(--py, 0) * -10px));
    transition:
      opacity var(--dur-short),
      transform 900ms var(--ease);
  }
  .pane:not(.open) .pane-ver {
    opacity: 0;
  }
  .pane.open .pane-ver {
    animation: ver-in 900ms var(--ease) both;
  }
  @keyframes ver-in {
    from {
      opacity: 0;
      translate: 60px 0;
    }
  }
  .pane-side {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 24px 0;
    transition: opacity var(--dur-short);
  }
  .pane-side b {
    order: 2;
    font-size: 40px;
    font-weight: var(--w-display);
    letter-spacing: -0.03em;
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    transition: transform var(--dur-short) var(--spring);
  }
  .pane:not(.open):hover .pane-side b {
    transform: rotate(180deg) translateY(4px);
  }
  .pane-side span {
    order: 1;
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    opacity: 0.8;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .pane.open .pane-side {
    opacity: 0;
    pointer-events: none;
  }
  .pane-body {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 28px 32px;
    min-width: 520px;
  }
  .pane-body > * {
    animation: rise 520ms var(--ease) both;
  }
  .pane-body > :nth-child(2) {
    animation-delay: 70ms;
  }
  .pane-body > :nth-child(3) {
    animation-delay: 140ms;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(14px);
    }
  }
  .pane-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .pane-chip {
    font-size: var(--t-foot);
    font-weight: var(--w-semibold);
    padding: 4px 12px;
    border-radius: var(--r-pill);
    background: var(--hero-btn);
  }
  .pane-link {
    font-size: var(--t-sub);
    font-weight: var(--w-semibold);
    opacity: 0.9;
  }
  .pane-link:hover {
    text-decoration: underline;
  }
  h1 {
    font-size: var(--t-hero);
    font-weight: var(--w-display);
    letter-spacing: -0.03em;
    line-height: 1;
  }
  p {
    margin: 8px 0 0;
    opacity: 0.85;
    max-width: 52ch;
  }
  .pane-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .pane .pane-play {
    background: var(--cover-ink);
    color: var(--hero-c);
  }
  .pane .pane-play:hover {
    opacity: 0.92;
    background: var(--cover-ink);
  }
  .ready {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--t-sub);
    font-weight: var(--w-medium);
  }
  .ready i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: block;
    background: var(--cover-ink);
  }
  .ready.upd i {
    background: var(--orange);
  }
  :global(:root[data-reduce-motion='true']) .pane-body > *,
  :global(:root[data-reduce-motion='true']) .pane.open .pane-ver {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .pane-body > *,
    .pane.open .pane-ver {
      animation: none;
    }
  }
</style>
