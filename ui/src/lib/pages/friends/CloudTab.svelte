<!--
  «Облачные миры» tab: friends' worlds that run 24/7 on the Aura server.
  My worlds and worlds shared with me (state, version, players), create (name,
  version, optional seed), start/stop with the manager's stage text and
  progress, share by nickname, delete with confirmation, Join and copy address.
  HIG › Buttons (one accent button per world: its next step; delete is red and
  asks first), Segmented controls (version), Text fields (seed is optional and
  says so; nickname validated before sending), Progress indicators
  (determinate bar once the manager reports a percentage, with its stage text),
  Lists and tables (worlds, players with access), Layout › Progressive
  disclosure (the create form and the access list open on demand),
  Color › never colour alone (state is always written out).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createIdleInterval } from '../../design/idleManager';
  import { indicator } from '../../shell/indicator';
  import { toasts } from '../../shell/toast.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { builds } from '../../shell/builds.svelte';
  import { accounts } from '../../shell/accounts.svelte';
  import { friendsLive } from '../../services/friendsLive';
  import { formatError } from '../../design/errors';
  import { t } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import ProgressBar from '../../components/ProgressBar.svelte';
  import { cloudWorlds, stateLabel, inTransit, CLOUD_VERSIONS, FAST_POLL_MS, PLAYER_RE, MAX_NAME, MAX_SEED, type CloudWorld } from './cloud.svelte';

  let creating = $state(false);
  let newName = $state('');
  let newVersion = $state<string>('1.21.1');
  let newSeed = $state('');
  let accessFor = $state<string | null>(null);
  let confirmDelete = $state<string | null>(null);
  let shareName = $state('');

  const st = $derived(cloudWorlds.status);
  const list = $derived(cloudWorlds.list);
  const showCreate = $derived(creating || (list !== null && cloudWorlds.mine.length === 0));
  const shareValid = $derived(PLAYER_RE.test(shareName.trim()));
  const friendNicks = $derived(friendsLive.friends.map((f) => f.nick).filter((n) => PLAYER_RE.test(n)));

  onMount(() => {
    void cloudWorlds.load();
    const stop = createIdleInterval(() => void cloudWorlds.tick(), FAST_POLL_MS);
    return stop;
  });

  // The active account is the owner: reload when it changes.
  let seenAccount: string | null | undefined = undefined;
  $effect(() => {
    const id = accounts.activeId;
    if (seenAccount !== undefined && id !== seenAccount) void cloudWorlds.load();
    seenAccount = id;
  });

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toasts.show(t('lobby.copied'));
    } catch {
      toasts.show(t('rooms.copy_failed'));
    }
  }

  async function create() {
    if (await cloudWorlds.create(newName, newVersion, newSeed)) {
      toasts.show(t('cloud.created', { name: newName.trim() }));
      creating = false;
      newName = '';
      newSeed = '';
    }
  }

  async function join(w: CloudWorld) {
    if (!w.address) return;
    try {
      if (builds.selectedId !== w.gameVersion) await builds.select(w.gameVersion);
      await launch.start(w.address);
    } catch (e) {
      const f = formatError(e, 'launch');
      toasts.show(f.title, f.description);
    }
  }

  async function share(w: CloudWorld, name: string, add: boolean) {
    if (await cloudWorlds.share(w, name, add)) {
      if (add) shareName = '';
      toasts.show(add ? t('cloud.shared', { name }) : t('cloud.unshared', { name }));
    }
  }

  async function remove(w: CloudWorld) {
    if (await cloudWorlds.remove(w)) {
      confirmDelete = null;
      toasts.show(t('cloud.deleted', { name: w.name }));
    }
  }

  function toggleAccess(id: string) {
    accessFor = accessFor === id ? null : id;
    shareName = '';
  }

  function line(w: CloudWorld): string {
    const parts = [stateLabel(w)];
    if (w.state === 'running') parts.push(t('cloud.players', { n: w.onlinePlayers.length, max: w.maxPlayers }));
    if (!w.isOwner) parts.push(t('cloud.owner', { name: w.owner }));
    return parts.join(' · ');
  }
</script>

{#snippet worldRow(w: CloudWorld)}
  {@const busy = cloudWorlds.busy[w.id]}
  <div class="tile world">
    <div class="w-row">
      <Monogram name={w.name} shape="square" tone={2} />
      <div class="w-t">
        <b>{w.name}<span class="vtag" style:--c="var(--cover-{w.gameVersion.replace(/\D/g, '')}, var(--fill))">{w.gameVersion}</span></b>
        <span>{#if w.state === 'running'}<span class="dot" aria-hidden="true"></span>{/if}{line(w)}</span>
      </div>
      <div class="acts">
        {#if w.state === 'running'}
          <button type="button" class="btn blue" disabled={launch.busy || !w.address} onclick={() => join(w)}>{t('home.join')}</button>
          <button type="button" class="btn" disabled={!!busy} onclick={() => cloudWorlds.stop(w)}>{t('cloud.stop')}</button>
        {:else if w.state === 'starting'}
          <button type="button" class="btn blue" disabled>{t('cloud.starting')}</button>
        {:else if w.state === 'stopping'}
          <button type="button" class="btn" disabled>{t('cloud.stopping')}</button>
        {:else}
          <button type="button" class="btn blue" disabled={!!busy} onclick={() => cloudWorlds.start(w)}>{t('cloud.start')}</button>
        {/if}
      </div>
    </div>

    {#if inTransit(w)}
      <div class="prog" aria-live="polite">
        <ProgressBar
          percentage={w.progress}
          indeterminate={w.state === 'stopping' || w.progress <= 0}
          label={w.stageText ?? (w.state === 'stopping' ? t('cloud.stage_saving') : t('cloud.stage_starting'))}
        />
      </div>
    {/if}

    {#if w.address && w.state === 'running'}
      <div class="addr">
        <span class="d">{t('cloud.address')} <span class="code">{w.address}</span></span>
        <button type="button" class="mini" onclick={() => copy(w.address ?? '')}>{t('cloud.copy_address')}</button>
      </div>
    {/if}

    {#if w.isOwner}
      <div class="owner-acts">
        <button type="button" class="mini" aria-expanded={accessFor === w.id} onclick={() => toggleAccess(w.id)}>
          {t('cloud.access', { n: w.allowedPlayers.length })}
        </button>
        {#if confirmDelete === w.id}
          <span class="warn">{t('cloud.delete_confirm')}</span>
          <button type="button" class="mini danger" disabled={busy === 'delete'} onclick={() => remove(w)}>{t('cloud.delete')}</button>
          <button type="button" class="mini" onclick={() => (confirmDelete = null)}>{t('common.cancel')}</button>
        {:else}
          <button type="button" class="mini danger" disabled={w.state !== 'sleeping' || !!busy} onclick={() => (confirmDelete = w.id)}>{t('cloud.delete_world')}</button>
          {#if w.state !== 'sleeping'}<span class="foot">{t('cloud.delete_needs_stop')}</span>{/if}
        {/if}
      </div>

      {#if accessFor === w.id}
        <div class="access">
          <div class="panel">
            {#each w.allowedPlayers as name (name)}
              <div class="prow">
                <Monogram {name} />
                <div><b>{name}</b></div>
                <button type="button" class="mini danger" disabled={busy === 'share'} onclick={() => share(w, name, false)}>{t('cloud.unshare')}</button>
              </div>
            {:else}
              <p class="empty">{t('cloud.access_empty')}</p>
            {/each}
          </div>
          <div class="inline">
            <label class="sr" for="cloud-share-{w.id}">{t('cloud.share_label')}</label>
            <input
              id="cloud-share-{w.id}"
              class="field"
              bind:value={shareName}
              maxlength="16"
              list="cloud-friends"
              placeholder={t('cloud.share_placeholder')}
              autocomplete="off"
              spellcheck="false"
              aria-invalid={shareName.trim() !== '' && !shareValid}
              onkeydown={(e) => e.key === 'Enter' && shareValid && share(w, shareName.trim(), true)}
            />
            <button type="button" class="btn" disabled={!shareValid || busy === 'share'} onclick={() => share(w, shareName.trim(), true)}>{t('cloud.share')}</button>
          </div>
          {#if shareName.trim() !== '' && !shareValid}<p class="foot">{t('cloud.share_invalid')}</p>{/if}
        </div>
      {/if}
    {/if}
  </div>
{/snippet}

{#if st === null}
  <p class="empty" aria-busy="true">{t('build.loading')}</p>
{:else if !st.available}
  <div class="panel">
    <p class="empty"><b>{t('cloud.unavailable')}</b> {st.reason ?? ''}</p>
  </div>
  <p class="foot">{t('cloud.unavailable_d')}</p>
{:else}
  <p class="lead">{t('cloud.intro', { owner: st.owner ?? '' })}</p>
  {#if list && list.maxActive > 0}
    <p class="foot quota">{t('cloud.quota', { n: list.activeCount, max: list.maxActive })}</p>
  {/if}

  {#if list === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else}
    {#if showCreate}
      <section>
        <div class="h-row"><h2>{t('cloud.create_title')}</h2></div>
        <div class="form">
          <div class="frow">
            <div><label class="l" for="cloud-name">{t('cloud.name')}</label></div>
            <input id="cloud-name" class="field" bind:value={newName} maxlength={MAX_NAME} placeholder={t('cloud.name_placeholder')} />
          </div>
          <div class="frow">
            <div class="l" id="cloud-version">{t('cloud.version')}</div>
            <div class="seg" role="group" aria-labelledby="cloud-version" use:indicator={{ selector: '[aria-pressed="true"]', key: newVersion }}>
              {#each CLOUD_VERSIONS as v (v)}
                <button type="button" aria-pressed={newVersion === v} onclick={() => (newVersion = v)}>{v}</button>
              {/each}
            </div>
          </div>
          <div class="frow">
            <div><label class="l" for="cloud-seed">{t('cloud.seed')}</label><div class="d">{t('cloud.seed_d')}</div></div>
            <input id="cloud-seed" class="field" bind:value={newSeed} maxlength={MAX_SEED} placeholder={t('cloud.seed_placeholder')} autocomplete="off" spellcheck="false" />
          </div>
        </div>
        <div class="acts">
          <button type="button" class="btn blue" disabled={!newName.trim() || !!cloudWorlds.busy.create} onclick={create}>
            {cloudWorlds.busy.create ? t('cloud.creating') : t('cloud.create')}
          </button>
          {#if cloudWorlds.mine.length}
            <button type="button" class="btn" onclick={() => (creating = false)}>{t('common.cancel')}</button>
          {/if}
        </div>
      </section>
    {/if}

    {#if cloudWorlds.mine.length}
      <section>
        <div class="h-row">
          <h2>{t('cloud.mine')}</h2>
          {#if !showCreate}<button type="button" onclick={() => (creating = true)}>{t('cloud.new_world')}</button>{/if}
        </div>
        <div class="worlds">
          {#each cloudWorlds.mine as w (w.id)}{@render worldRow(w)}{/each}
        </div>
      </section>
    {/if}

    <section>
      <div class="h-row"><h2>{t('cloud.shared_title')}</h2></div>
      {#if cloudWorlds.shared.length}
        <div class="worlds">
          {#each cloudWorlds.shared as w (w.id)}{@render worldRow(w)}{/each}
        </div>
      {:else}
        <div class="panel"><p class="empty">{t('cloud.shared_empty')}</p></div>
      {/if}
    </section>
  {/if}
{/if}

<datalist id="cloud-friends">
  {#each friendNicks as n (n)}<option value={n}></option>{/each}
</datalist>

{#if cloudWorlds.error}
  <p class="bad" role="alert">{cloudWorlds.error}</p>
{/if}

<style>
  section + section,
  .lead + section,
  .quota + section {
    margin-top: 28px;
  }
  .lead {
    margin: 0 0 4px;
    color: var(--label-2);
  }
  .worlds {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .world {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .world:hover {
    transform: none;
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .w-row {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
  }
  .w-t {
    min-width: 0;
  }
  .w-t b {
    display: block;
    font-weight: var(--w-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .w-t > span {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .vtag {
    display: inline-block;
    vertical-align: 1px;
    margin-left: 8px;
    padding: 0 8px;
    border-radius: var(--r-pill);
    font-size: 11px;
    font-weight: var(--w-bold);
    line-height: 18px;
    color: var(--cover-ink);
    background: var(--c);
  }
  .acts,
  .owner-acts,
  .addr {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  section > .acts {
    margin-top: 12px;
  }
  .addr {
    justify-content: space-between;
  }
  .d {
    color: var(--label-2);
    font-size: var(--t-foot);
  }
  .code {
    font-family: var(--font-mono);
    color: var(--label);
  }
  .warn {
    font-weight: var(--w-medium);
  }
  .foot {
    margin: 8px 0 0;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .owner-acts .foot {
    margin: 0;
  }
  .bad {
    margin: 12px 0 0;
    color: var(--red);
    font-size: var(--t-foot);
  }
  .mini.danger {
    color: var(--red);
  }
  .prow {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 8px;
    border-radius: var(--r-m);
  }
  .prow b {
    font-weight: var(--w-semibold);
  }
  .inline {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    margin-top: 8px;
  }
  .frow .field {
    width: min(280px, 100%);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
