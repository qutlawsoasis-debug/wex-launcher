<!--
  «Свой сервер» tab: a dedicated Fabric server for friends, run on this PC.
  Create, install, accept the EULA, start/stop, status and players, the address
  to copy for friends, console, settings, «Добавить себе в Серверы», delete.
  HIG › Buttons (one accent button: the next step; destructive actions are red),
  Toggles (systemGreen switch in a form row), Segmented controls (game mode,
  difficulty, version), Text fields (hint + label, validated on save),
  Sliders (memory, with the exact value shown), Lists and tables (players,
  whitelist, servers), Layout › Grouping and Progressive disclosure.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createIdleInterval } from '../../design/idleManager';
  import { indicator } from '../../shell/indicator';
  import { toasts } from '../../shell/toast.svelte';
  import { call } from '../../shell/ipc';
  import { t } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';
  import { ownServers, defaultSettings, statusLabel, VERSIONS, type ServerSettings, type OwnServer } from './ownServer.svelte';

  let creating = $state(false);
  let newVersion = $state<string>('1.21.1');
  let newName = $state(defaultSettings().name);
  let draft = $state<ServerSettings | null>(null);
  let draftFor = $state<string | null>(null);
  let command = $state('');
  let wlName = $state('');
  let confirmDelete = $state(false);
  let logEl: HTMLElement | undefined = $state();

  const s = $derived(ownServers.selected);
  const live = $derived(!!s && (s.status === 'running' || s.status === 'starting' || s.status === 'stopping'));
  const showCreate = $derived(creating || (ownServers.list !== null && ownServers.list.length === 0));
  const dirty = $derived(!!s && !!draft && JSON.stringify(draft) !== JSON.stringify(s.settings));

  onMount(() => {
    const stopListen = ownServers.listen();
    void ownServers.refresh();
    const stopPoll = createIdleInterval(() => void ownServers.poll(), 1500);
    return () => {
      stopListen();
      stopPoll();
    };
  });

  // The settings form edits a copy; it resets when another server is selected.
  $effect(() => {
    if (s && draftFor !== s.id) {
      draft = structuredClone($state.snapshot(s.settings));
      draftFor = s.id;
      confirmDelete = false;
    }
  });

  $effect(() => {
    void ownServers.log.length;
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
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
    const ok = await ownServers.create(newVersion, { ...defaultSettings(), name: newName.trim() });
    if (ok) creating = false;
  }

  async function save() {
    if (!draft) return;
    const view = await ownServers.save({ ...draft, publicAddress: draft.publicAddress?.trim() || null });
    if (view) {
      draft = structuredClone($state.snapshot(view.settings));
      toasts.show(t('own.saved'), live ? t('own.saved_restart') : undefined);
    }
  }

  async function sendCommand() {
    if (await ownServers.send(command)) command = '';
  }

  async function whitelist(name: string, add: boolean) {
    if (await ownServers.whitelist(name, add)) {
      if (add) wlName = '';
      toasts.show(add ? t('own.wl_added', { name }) : t('own.wl_removed', { name }));
    }
  }

  async function addToServers() {
    if (await ownServers.addToServers()) toasts.show(t('own.added_to_servers'));
  }

  async function openEula() {
    try {
      await call('own_server_open_eula');
    } catch {
      toasts.show(t('own.eula_link_failed'));
    }
  }

  function primary(srv: OwnServer): { label: string; run: () => unknown; disabled: boolean } {
    if (srv.installing || ownServers.busy === 'install') return { label: t('own.installing'), run: () => {}, disabled: true };
    if (!srv.installed) return { label: t('own.install'), run: () => ownServers.install(), disabled: !!ownServers.busy };
    if (srv.status === 'starting') return { label: t('own.starting'), run: () => {}, disabled: true };
    if (srv.status === 'stopping') return { label: t('own.stopping'), run: () => {}, disabled: true };
    return { label: t('own.start'), run: () => ownServers.start(), disabled: !srv.eulaAccepted || !!ownServers.busy };
  }

  const MODES = [
    ['survival', 'own.mode_survival'],
    ['creative', 'own.mode_creative'],
    ['adventure', 'own.mode_adventure'],
  ] as const;
  const DIFFS = [
    ['peaceful', 'own.diff_peaceful'],
    ['easy', 'own.diff_easy'],
    ['normal', 'own.diff_normal'],
    ['hard', 'own.diff_hard'],
  ] as const;
</script>

{#if ownServers.list === null}
  <p class="empty" aria-busy="true">{t('build.loading')}</p>
{:else if showCreate}
  <section>
    <div class="h-row"><h2>{t('own.create_title')}</h2></div>
    <p class="lead">{t('own.intro')}</p>
    <div class="form">
      <div class="frow">
        <div><label class="l" for="own-new-name">{t('own.name')}</label><div class="d">{t('own.name_d')}</div></div>
        <input id="own-new-name" class="field" bind:value={newName} maxlength="59" placeholder={t('own.default_name')} />
      </div>
      <div class="frow">
        <div class="l" id="own-new-version">{t('own.version')}</div>
        <div class="seg" role="group" aria-labelledby="own-new-version" use:indicator={{ selector: '[aria-pressed="true"]', key: newVersion }}>
          {#each VERSIONS as v (v)}
            <button type="button" aria-pressed={newVersion === v} onclick={() => (newVersion = v)}>{v}</button>
          {/each}
        </div>
      </div>
    </div>
    <div class="acts">
      <button type="button" class="btn blue" disabled={!newName.trim() || ownServers.busy === 'create'} onclick={create}>{t('own.create')}</button>
      {#if ownServers.list.length}
        <button type="button" class="btn" onclick={() => (creating = false)}>{t('common.cancel')}</button>
      {/if}
    </div>
  </section>
{:else if s && draft}
  {#if ownServers.list.length > 1}
    <section>
      <div class="h-row">
        <h2>{t('own.my_servers')}</h2>
        <button type="button" onclick={() => (creating = true)}>{t('own.new_server')}</button>
      </div>
      <div class="list" role="listbox" aria-label={t('own.my_servers')}>
        {#each ownServers.list as srv (srv.id)}
          <div class="lrow pick" role="option" tabindex="0" aria-selected={srv.id === s.id} onclick={() => ownServers.select(srv.id)} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && ownServers.select(srv.id)}>
            <Monogram name={srv.settings.name} shape="square" tone={3} />
            <div><b>{srv.settings.name}</b><span>{srv.version}, {statusLabel(srv.status)}</span></div>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {@const p = primary(s)}
  <section class="tile head">
    <div class="head-row">
      <Monogram name={s.settings.name} shape="large" tone={3} />
      <div class="head-t">
        <b>{s.settings.name}</b>
        <span>
          {#if s.status === 'running'}<span class="dot" aria-hidden="true"></span>{/if}
          {t('own.head_d', { version: s.version, status: statusLabel(s.status) })}{s.status === 'running' ? `, ${t('own.players_count', { n: s.players.length, max: s.settings.maxPlayers })}` : ''}
        </span>
      </div>
      <div class="acts">
        {#if s.status === 'running'}
          <button type="button" class="btn" disabled={!!ownServers.busy} onclick={() => ownServers.stop()}>{t('own.stop')}</button>
        {:else}
          <button type="button" class="btn blue" disabled={p.disabled} onclick={p.run}>{p.label}</button>
        {/if}
      </div>
    </div>
    {#if ownServers.progress && ownServers.progress.id === s.id}
      <p class="d" aria-live="polite">{ownServers.progress.stage}{ownServers.progress.total ? ` — ${Math.round((ownServers.progress.current / ownServers.progress.total) * 100)}%` : ''}</p>
    {/if}
    {#if s.installed}
      <p class="d">{s.installed.modSource === 'manifest' ? t('own.mods_manifest', { n: s.installed.mods.length }) : s.installed.modSource === 'fabricModJson' ? t('own.mods_fabric', { n: s.installed.mods.length }) : t('own.mods_none')}</p>
    {:else if !s.installing}
      <p class="d">{t('own.install_d')}</p>
    {/if}
    {#if s.problem}<p class="bad" role="alert">{s.problem}</p>{/if}
    {#if s.status === 'crashed' && !s.problem}<p class="bad" role="alert">{t('own.crashed_d')}</p>{/if}
  </section>

  {#if !s.eulaAccepted}
    <section>
      <div class="form">
        <div class="frow">
          <div>
            <div class="l" id="own-eula-l">{t('own.eula')}</div>
            <div class="d">{t('own.eula_d')} <button type="button" class="link" onclick={openEula}>{t('own.eula_read')}</button></div>
          </div>
          <button type="button" class="switch" role="switch" aria-checked="false" aria-labelledby="own-eula-l" disabled={ownServers.busy === 'eula'} onclick={() => ownServers.acceptEula()}></button>
        </div>
      </div>
    </section>
  {/if}

  <section>
    <div class="h-row"><h2>{t('own.address_title')}</h2></div>
    <div class="form">
      <div class="frow">
        <div><div class="l">{t('own.addr_lan')}</div><div class="d code">{s.lanAddress ?? t('own.addr_lan_none')}</div></div>
        {#if s.lanAddress}<button type="button" class="mini" onclick={() => copy(s.lanAddress ?? '')}>{t('own.copy_for_friends')}</button>{/if}
      </div>
      <div class="frow">
        {#if s.publicAddress}
          <div><div class="l">{t('own.addr_public')}</div><div class="d code">{s.publicAddress}</div></div>
          <button type="button" class="mini" onclick={() => copy(s.publicAddress ?? '')}>{t('own.copy_for_friends')}</button>
        {:else}
          <div><div class="l">{t('own.addr_public')}</div><div class="d">{t('own.addr_public_hint', { port: s.settings.port, lan: s.lanAddress?.split(':')[0] ?? t('own.this_pc') })}</div></div>
          <span></span>
        {/if}
      </div>
      <div class="frow">
        <div><div class="l">{t('own.addr_self')}</div><div class="d code">{s.localAddress}</div></div>
        <button type="button" class="mini" onclick={addToServers}>{t('own.add_to_servers')}</button>
      </div>
    </div>
    <p class="foot">{t('own.uptime_note')}</p>
  </section>

  <section>
    <div class="h-row"><h2>{t('own.players_title', { n: s.players.length, max: s.settings.maxPlayers })}</h2></div>
    <div class="panel">
      {#each s.players as name (name)}
        <div class="prow"><Monogram {name} /><div><b>{name}</b></div><span></span></div>
      {:else}
        <p class="empty">{s.status === 'running' ? t('own.players_none') : t('own.players_offline')}</p>
      {/each}
    </div>
  </section>

  {#if s.settings.whitelist}
    <section>
      <div class="h-row"><h2>{t('own.wl_title')}</h2></div>
      <div class="panel">
        {#each s.whitelist as name (name)}
          <div class="prow">
            <Monogram {name} />
            <div><b>{name}</b></div>
            <button type="button" class="mini danger" disabled={s.status !== 'running'} onclick={() => whitelist(name, false)}>{t('own.wl_remove')}</button>
          </div>
        {:else}
          <p class="empty">{t('own.wl_empty')}</p>
        {/each}
      </div>
      <div class="inline">
        <label class="sr" for="own-wl">{t('own.wl_add_label')}</label>
        <input id="own-wl" class="field" bind:value={wlName} maxlength="16" placeholder={t('own.wl_placeholder')} autocomplete="off" spellcheck="false" disabled={s.status !== 'running'} onkeydown={(e) => e.key === 'Enter' && whitelist(wlName.trim(), true)} />
        <button type="button" class="btn" disabled={s.status !== 'running' || !wlName.trim()} onclick={() => whitelist(wlName.trim(), true)}>{t('own.wl_add')}</button>
      </div>
      {#if s.status !== 'running'}<p class="foot">{t('own.wl_needs_running')}</p>{/if}
    </section>
  {/if}

  <section>
    <div class="h-row"><h2>{t('own.console')}</h2></div>
    <!-- Scrollable log must be reachable by keyboard (axe: scrollable-region-focusable). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="console" role="log" aria-label={t('own.console')} tabindex="0" bind:this={logEl}>
      {#each ownServers.log as line (line.seq)}<div>{line.text}</div>{:else}<div class="muted">{live ? t('own.console_wait') : t('own.console_empty')}</div>{/each}
    </div>
    <div class="inline">
      <label class="sr" for="own-cmd">{t('own.command')}</label>
      <input id="own-cmd" class="field mono" bind:value={command} maxlength="256" placeholder={t('own.command_placeholder')} autocomplete="off" spellcheck="false" disabled={s.status !== 'running'} onkeydown={(e) => e.key === 'Enter' && sendCommand()} />
      <button type="button" class="btn" disabled={s.status !== 'running' || !command.trim()} onclick={sendCommand}>{t('own.send')}</button>
    </div>
  </section>

  <section>
    <div class="h-row"><h2>{t('own.settings')}</h2></div>
    <div class="form">
      <div class="frow">
        <div><label class="l" for="own-name">{t('own.name')}</label><div class="d">{t('own.name_d')}</div></div>
        <input id="own-name" class="field" bind:value={draft.name} maxlength="59" />
      </div>
      <div class="frow">
        <div><label class="l" for="own-slots">{t('own.slots')}</label></div>
        <input id="own-slots" class="field num" type="number" min="1" max="100" step="1" bind:value={draft.maxPlayers} />
      </div>
      <div class="frow">
        <div class="l" id="own-mode">{t('own.mode')}</div>
        <div class="seg" role="group" aria-labelledby="own-mode" use:indicator={{ selector: '[aria-pressed="true"]', key: draft.gamemode }}>
          {#each MODES as [v, label] (v)}
            <button type="button" aria-pressed={draft.gamemode === v} onclick={() => draft && (draft.gamemode = v)}>{t(label)}</button>
          {/each}
        </div>
      </div>
      <div class="frow">
        <div class="l" id="own-diff">{t('own.difficulty')}</div>
        <div class="seg" role="group" aria-labelledby="own-diff" use:indicator={{ selector: '[aria-pressed="true"]', key: draft.difficulty }}>
          {#each DIFFS as [v, label] (v)}
            <button type="button" aria-pressed={draft.difficulty === v} onclick={() => draft && (draft.difficulty = v)}>{t(label)}</button>
          {/each}
        </div>
      </div>
      <div class="frow">
        <div><div class="l" id="own-wl-l">{t('own.wl_toggle')}</div><div class="d">{t('own.wl_toggle_d')}</div></div>
        <button type="button" class="switch" role="switch" aria-checked={draft.whitelist} aria-labelledby="own-wl-l" onclick={() => draft && (draft.whitelist = !draft.whitelist)}></button>
      </div>
      <div class="frow">
        <div><div class="l" id="own-online-l">{t('own.online_mode')}</div><div class="d">{t('own.online_mode_d')}</div></div>
        <button type="button" class="switch" role="switch" aria-checked={draft.onlineMode} aria-labelledby="own-online-l" onclick={() => draft && (draft.onlineMode = !draft.onlineMode)}></button>
      </div>
      <div class="frow">
        <div class="l" id="own-pvp-l">{t('own.pvp')}</div>
        <button type="button" class="switch" role="switch" aria-checked={draft.pvp} aria-labelledby="own-pvp-l" onclick={() => draft && (draft.pvp = !draft.pvp)}></button>
      </div>
      <div class="frow full">
        <div class="mem-h"><label class="l" for="own-mem">{t('own.memory')}</label><span class="d">{t('own.memory_value', { gb: (draft.memoryMb / 1024).toFixed(1) })}</span></div>
        <input id="own-mem" type="range" min="1024" max="16384" step="512" bind:value={draft.memoryMb} />
      </div>
      <div class="frow">
        <div><label class="l" for="own-port">{t('own.port')}</label><div class="d">{t('own.port_d')}</div></div>
        <input id="own-port" class="field num" type="number" min="1024" max="65535" step="1" bind:value={draft.port} />
      </div>
      <div class="frow">
        <div><label class="l" for="own-public">{t('own.public_address')}</label><div class="d">{t('own.public_address_d')}</div></div>
        <input id="own-public" class="field" placeholder={t('own.public_placeholder')} autocomplete="off" spellcheck="false" value={draft.publicAddress ?? ''} oninput={(e) => draft && (draft.publicAddress = e.currentTarget.value || null)} />
      </div>
    </div>
    <div class="acts">
      <button type="button" class="btn" disabled={!dirty || ownServers.busy === 'save'} onclick={save}>{t('own.save')}</button>
      {#if live && dirty}<span class="foot">{t('own.saved_restart')}</span>{/if}
    </div>
  </section>

  <section>
    <div class="acts">
      {#if confirmDelete}
        <span class="warn">{t('own.delete_confirm')}</span>
        <button type="button" class="btn danger" disabled={ownServers.busy === 'delete'} onclick={async () => { if (await ownServers.remove()) confirmDelete = false; }}>{t('own.delete')}</button>
        <button type="button" class="btn" onclick={() => (confirmDelete = false)}>{t('common.cancel')}</button>
      {:else}
        <button type="button" class="btn danger" disabled={live || s.installing} onclick={() => (confirmDelete = true)}>{t('own.delete_server')}</button>
        {#if ownServers.list.length === 1}
          <button type="button" class="btn" onclick={() => (creating = true)}>{t('own.new_server')}</button>
        {/if}
      {/if}
    </div>
  </section>
{/if}

{#if ownServers.error}
  <p class="bad" role="alert">{ownServers.error}</p>
{/if}

<style>
  section + section {
    margin-top: 28px;
  }
  .lead {
    margin: 0 0 12px;
    color: var(--label-2);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }
  .head-row .acts {
    margin-top: 0;
  }
  .head {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head:hover {
    transform: none;
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .head-row {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
  }
  .head-t b {
    display: block;
    font-size: var(--t-t3);
    font-weight: var(--w-bold);
  }
  .head-t span,
  .head .d {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--label-2);
    font-size: var(--t-sub);
    margin: 0;
  }
  .bad {
    margin: 8px 0 0;
    color: var(--red);
    font-size: var(--t-foot);
  }
  .warn {
    font-weight: var(--w-medium);
  }
  .foot {
    margin: 8px 0 0;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .code,
  .mono,
  .console {
    font-family: var(--font-mono);
  }
  .link {
    padding: 0;
    color: var(--blue);
    font-size: inherit;
  }
  .num {
    width: 96px;
  }
  .frow .field:not(.num) {
    width: min(280px, 100%);
  }
  .mem-h {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .pick {
    grid-template-columns: 40px minmax(0, 1fr);
    cursor: pointer;
  }
  .pick[aria-selected='true'] {
    background: var(--fill-2);
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
  .mini.danger {
    color: var(--red);
  }
  .inline {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    margin-top: 8px;
  }
  .console {
    height: 240px;
    overflow: auto;
    padding: 12px;
    border-radius: var(--r-m);
    background: var(--fill-3);
    box-shadow: inset 0 0 0 1px var(--edge);
    font-size: var(--t-foot);
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .console .muted {
    color: var(--label-2);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
