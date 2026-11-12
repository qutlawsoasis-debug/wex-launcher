<!--
  Servers (reference: V.servers). The server list from core (list_servers) with
  live players and ping (ping_server_details), «Зайти» starts the selected build
  connected to the server. Add, rename and remove go through add_server /
  edit_server / remove_server.
  HIG › Lists and tables; Buttons (blue on the first server — the main action);
  Text fields (name and address); Alerts (remove asks twice).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { builds } from '../../shell/builds.svelte';
  import { launch } from '../../shell/launch.svelte';
  import { call } from '../../shell/ipc';
  import { toasts } from '../../shell/toast.svelte';
  import { formatError } from '../../design/errors';
  import { createIdleInterval } from '../../design/idleManager';
  import { t, tn } from '../../i18n';
  import Monogram from '../../shell/Monogram.svelte';

  interface Server {
    id?: string;
    name: string;
    address: string;
  }
  interface Ping {
    ok: boolean;
    motd?: string;
    online?: number;
    max?: number;
    ms?: number;
  }

  let list = $state<Server[] | null>(null);
  let pings = $state<Record<string, Ping>>({});
  let editing = $state<Server | 'new' | null>(null);
  let name = $state('');
  let address = $state('');
  let saving = $state(false);
  let removeArmed = $state(false);

  const version = $derived(builds.selectedId);

  async function load() {
    try {
      list = await call<Server[]>('list_servers', { version });
    } catch {
      list = [];
    }
    void pingAll();
  }

  async function pingAll() {
    const servers = list ?? [];
    await Promise.all(
      servers.map(async (s) => {
        try {
          const p = await call<{ success: boolean; latency_ms?: number | null; online?: number | null; max_players?: number | null; motd?: string | null }>('ping_server_details', {
            address: s.address,
            timeoutMs: null,
          });
          pings = { ...pings, [s.address]: p.success ? { ok: true, online: p.online ?? undefined, max: p.max_players ?? undefined, ms: p.latency_ms ?? undefined, motd: cleanMotd(p.motd) } : { ok: false } };
        } catch {
          pings = { ...pings, [s.address]: { ok: false } };
        }
      })
    );
  }

  onMount(() => {
    void load();
    return createIdleInterval(() => void pingAll(), 30_000);
  });

  /** Server description without Minecraft colour codes, on one line (reference: «Выживание и PvP, mc.auramc.net»). */
  function cleanMotd(m?: string | null): string | undefined {
    const text = (m ?? '').replace(/§./g, '').replace(/\s+/g, ' ').trim();
    return text || undefined;
  }

  function meter(p: Ping | undefined): string {
    if (!p) return t('servers2.pinging');
    if (!p.ok) return t('home.server_no_answer');
    const parts: string[] = [];
    if (p.online !== undefined && p.max !== undefined) parts.push(t('servers2.players', { on: p.online, max: p.max }));
    else if (p.online !== undefined) parts.push(tn('home.players_now', p.online));
    if (p.ms !== undefined) parts.push(t('home.ping_ms', { ms: p.ms }));
    return parts.join(', ');
  }

  function edit(s: Server | 'new') {
    editing = s;
    removeArmed = false;
    name = s === 'new' ? '' : s.name;
    address = s === 'new' ? '' : s.address;
  }

  const valid = $derived(name.trim().length > 0 && /^[A-Za-z0-9.-]+(:\d{1,5})?$/.test(address.trim()));

  async function save() {
    if (!valid || !editing) return;
    saving = true;
    try {
      if (editing === 'new') {
        await call('add_server', { version, name: name.trim(), address: address.trim() });
        toasts.show(t('servers2.added'));
      } else {
        await call('edit_server', { version, oldAddress: editing.address, name: name.trim(), newAddress: address.trim() });
        toasts.show(t('servers2.saved'));
      }
      editing = null;
      await load();
    } catch (e) {
      toasts.show(t('servers2.save_failed'), formatError(e, 'general').description);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!editing || editing === 'new') return;
    if (!removeArmed) {
      removeArmed = true;
      return;
    }
    try {
      await call('remove_server', { version, address: editing.address });
      toasts.show(t('servers2.removed'));
      editing = null;
      await load();
    } catch (e) {
      toasts.show(t('servers2.save_failed'), formatError(e, 'general').description);
    }
  }
</script>

<div class="page-title">
  <h1>{t('shell.nav_servers')}</h1>
  <p>{t('servers2.subtitle', { ver: version })}</p>
</div>

<div class="list">
  {#if list === null}
    <p class="empty" aria-busy="true">{t('build.loading')}</p>
  {:else if list.length}
    {#each list as s, i (s.address)}
      {@const p = pings[s.address]}
      <div class="lrow lrow-srv">
        <Monogram name={s.name} shape="square" tone={((i + 3) % 5) + 1} />
        <div>
          <b>{s.name}</b>
          <span>{[pings[s.address]?.motd, s.address].filter(Boolean).join(', ')}</span>
        </div>
        <div class="meter">
          <span>{meter(p)}</span>
          {#if p?.ok && p.online !== undefined && p.max}
            <i style:--w={Math.min(1, p.online / p.max).toFixed(3)}></i>
          {/if}
        </div>
        <div class="acts">
          <button type="button" class="mini" aria-label={t('servers2.edit_label', { name: s.name })} onclick={() => edit(s)}>{t('servers2.edit')}</button>
          <button type="button" class="btn" class:blue={i === 0} disabled={launch.busy} onclick={() => launch.start(s.address)}>{t('home.join')}</button>
        </div>
      </div>
    {/each}
  {:else}
    <p class="empty"><b>{t('servers2.empty')}</b> {t('servers2.empty_hint')}</p>
  {/if}
</div>

{#if editing}
  <form class="tile editor" onsubmit={(e) => { e.preventDefault(); void save(); }}>
    <b>{editing === 'new' ? t('servers2.add') : t('servers2.edit_title')}</b>
    <label>
      <span>{t('servers2.name')}</span>
      <input class="field" bind:value={name} maxlength="64" autocomplete="off" />
    </label>
    <label>
      <span>{t('servers2.address')}</span>
      <input class="field" bind:value={address} placeholder={t('servers2.address_placeholder')} autocomplete="off" spellcheck="false" aria-invalid={address.length > 0 && !valid} />
    </label>
    <div class="row">
      <button type="submit" class="btn blue" disabled={!valid || saving}>{editing === 'new' ? t('servers2.add_btn') : t('servers2.save_btn')}</button>
      <button type="button" class="btn" onclick={() => (editing = null)}>{t('servers2.cancel')}</button>
      {#if editing !== 'new'}
        <span class="grow"></span>
        <button type="button" class="btn danger" onclick={remove}>{removeArmed ? t('build.press_again') : t('servers2.remove')}</button>
      {/if}
    </div>
  </form>
{:else}
  <div><button type="button" class="btn" onclick={() => edit('new')}>{t('servers2.add')}</button></div>
{/if}

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
  .lrow-srv {
    grid-template-columns: 40px minmax(0, 1fr) 160px auto;
  }
  /* Reference: .meter */
  .meter {
    display: grid;
    gap: 8px;
    font-size: var(--t-foot);
    color: var(--label-2);
    min-width: 0;
  }
  .meter span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meter i {
    display: block;
    height: 4px;
    border-radius: 2px;
    background: var(--fill-2);
    overflow: hidden;
  }
  .meter i::after {
    content: '';
    display: block;
    height: 100%;
    background: var(--green);
    transform: scaleX(var(--w));
    transform-origin: left;
  }
  .acts {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .editor {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 560px;
  }
  .editor:hover {
    transform: none;
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .editor label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .editor .row {
    display: flex;
    gap: 8px;
  }
  .grow {
    flex: 1;
  }
</style>
