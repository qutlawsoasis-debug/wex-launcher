<!--
  Sign-in sheet: choose a way to sign in, then finish it.
  HIG › Sheets: a scoped task in a modal sheet; Cancel dismisses without saving,
  no Done/Back trio. HIG › Managing accounts: every button names its method
  («Войти через Microsoft»), explain what each way gives, refer only to methods
  available now (Microsoft is shown disabled with the reason when core has no
  client ID). HIG › Buttons: one primary (systemBlue) action per step, a waiting
  action shows its activity in the button. HIG › Text fields: label, hint,
  validation before moving on. HIG › Accessibility: no time-boxed UI, the code
  stays until the player cancels or it expires.
-->
<script lang="ts">
  import Sheet from './Sheet.svelte';
  import { accounts, OFFLINE_NAME_RE, type DeviceCode } from '../shell/accounts.svelte';
  import { formatError } from '../design/errors';
  import { toasts } from '../shell/toast.svelte';
  import { t } from '../i18n';

  interface Props {
    open?: boolean;
    /** Start on a step directly (the sign-in-again action opens Microsoft). */
    start?: Step;
  }

  type Step = 'choose' | 'offline' | 'microsoft';

  let { open = $bindable(false), start = 'choose' }: Props = $props();

  let step = $state<Step>('choose');
  let name = $state('');
  let nameTouched = $state(false);
  let busy = $state(false);
  let error = $state('');
  let code = $state<DeviceCode | null>(null);
  let waiting = $state(false);

  const nameOk = $derived(OFFLINE_NAME_RE.test(name.trim()));

  $effect(() => {
    if (open) {
      step = start;
      error = '';
      if (start === 'microsoft') void beginMicrosoft();
    }
  });

  function describe(e: unknown): string {
    return formatError(e, 'auth').description;
  }

  async function addOffline(e?: Event) {
    e?.preventDefault();
    nameTouched = true;
    if (!nameOk || busy) return;
    busy = true;
    error = '';
    try {
      await accounts.addOffline(name);
      toasts.show(t('acc.signed_in', { name: name.trim() }));
      close();
    } catch (err) {
      error = describe(err);
    } finally {
      busy = false;
    }
  }

  async function beginMicrosoft() {
    step = 'microsoft';
    error = '';
    code = null;
    busy = true;
    try {
      code = await accounts.beginMicrosoft();
    } catch (err) {
      error = describe(err);
      busy = false;
      return;
    }
    busy = false;
    waiting = true;
    const flow = code.flow_id;
    try {
      await accounts.finishMicrosoft(flow);
      if (code?.flow_id === flow) {
        toasts.show(t('acc.signed_in', { name: accounts.active?.name ?? '' }));
        close();
      }
    } catch (err) {
      // Cancelled by the player: nothing to say.
      if (code?.flow_id === flow && formatError(err, 'auth').code !== 'AUTH_CANCELLED') error = describe(err);
    } finally {
      if (code?.flow_id === flow) waiting = false;
    }
  }

  async function openPage() {
    if (!code) return;
    try {
      await accounts.openMicrosoftPage(code.flow_id);
    } catch (err) {
      error = describe(err);
    }
  }

  async function copyCode() {
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code.user_code);
      toasts.show(t('common.copied'));
    } catch {
      // Clipboard can be unavailable; the code stays on screen to type by hand.
    }
  }

  function cancelFlow() {
    if (code) void accounts.cancelMicrosoft(code.flow_id).catch(() => {});
    code = null;
    waiting = false;
  }

  function close() {
    cancelFlow();
    open = false;
  }
</script>

<Sheet bind:open title={t('acc.sheet_title')} showDone={false} cancelLabel={t('common.cancel')} onclose={cancelFlow} maxWidth="480px">
  <div class="acc-sheet">
    {#if step === 'choose'}
      <p class="lead">{t('acc.choose_lead')}</p>
      <div class="ways">
        <button type="button" class="way" disabled={!accounts.microsoftAvailable} aria-describedby="way-ms-d" onclick={beginMicrosoft}>
          <b>{t('acc.way_microsoft')}</b>
          <span id="way-ms-d">{accounts.microsoftAvailable ? t('acc.way_microsoft_d') : t('acc.way_microsoft_off')}</span>
        </button>
        <button type="button" class="way" aria-describedby="way-off-d" onclick={() => (step = 'offline')}>
          <b>{t('acc.way_offline')}</b>
          <span id="way-off-d">{t('acc.way_offline_d')}</span>
        </button>
      </div>
    {:else if step === 'offline'}
      <form class="offline" onsubmit={addOffline} novalidate>
        <label class="l" for="acc-nick">{t('acc.nick_label')}</label>
        <input
          id="acc-nick"
          class="field"
          class:bad={nameTouched && !nameOk}
          bind:value={name}
          oninput={() => (nameTouched = true)}
          placeholder={t('acc.nick_placeholder')}
          maxlength="16"
          autocomplete="off"
          spellcheck="false"
          aria-invalid={nameTouched && !nameOk}
          aria-describedby="acc-nick-hint"
        />
        <div class="d" class:bad={nameTouched && !nameOk} id="acc-nick-hint">{nameTouched && !nameOk ? t('acc.nick_bad') : t('acc.nick_hint')}</div>
        <div class="actions">
          <button type="button" class="btn" onclick={() => (step = 'choose')}>{t('common.back')}</button>
          <button type="submit" class="btn blue" disabled={busy || !nameOk} aria-busy={busy}>{t('acc.add_offline')}</button>
        </div>
      </form>
    {:else}
      <div class="ms">
        {#if code}
          <p class="lead">{t('acc.ms_lead')}</p>
          <div class="code" aria-label={t('acc.ms_code_label')}>
            <span class="code-text" aria-live="polite">{code.user_code}</span>
            <button type="button" class="mini" onclick={copyCode}>{t('acc.copy_code')}</button>
          </div>
          <div class="d url">{code.verification_uri}</div>
          <button type="button" class="btn blue big" onclick={openPage}>{t('acc.open_ms_page')}</button>
          {#if waiting}
            <p class="status" role="status"><span class="spin" aria-hidden="true"></span>{t('acc.ms_waiting')}</p>
          {/if}
        {:else if busy}
          <p class="status" role="status"><span class="spin" aria-hidden="true"></span>{t('acc.ms_starting')}</p>
        {/if}
        {#if error}
          <div class="actions">
            <button type="button" class="btn" onclick={() => { cancelFlow(); step = 'choose'; error = ''; }}>{t('common.back')}</button>
            <button type="button" class="btn blue" onclick={beginMicrosoft}>{t('acc.retry')}</button>
          </div>
        {/if}
      </div>
    {/if}
    {#if error}
      <p class="err" role="alert">{error}</p>
    {/if}
    <p class="foot">{t('acc.storage_note')}</p>
  </div>
</Sheet>

<style>
  .acc-sheet {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 4px 4px 8px;
  }
  .lead {
    margin: 0;
    color: var(--label-2);
  }
  .ways {
    display: flex;
    flex-direction: column;
    border-radius: var(--r-l);
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--edge);
    overflow: hidden;
  }
  .way {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 12px 16px;
    min-height: 64px;
    text-align: left;
    border-bottom: 1px solid var(--separator);
    transition: background-color var(--dur-micro);
  }
  .way:last-child {
    border-bottom: 0;
  }
  .way:hover:not([disabled]) {
    background: var(--fill-3);
  }
  .way[disabled] {
    cursor: default;
  }
  .way[disabled] b {
    color: var(--label-3);
  }
  .way b {
    font-weight: var(--w-semibold);
  }
  .way span,
  .d {
    font-size: var(--t-foot);
    color: var(--label-2);
  }
  .offline {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .l {
    font-weight: var(--w-medium);
  }
  .d.bad,
  .err {
    color: var(--red);
  }
  .err {
    margin: 0;
    font-size: var(--t-foot);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .ms {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .code {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--r-m);
    background: var(--fill-3);
  }
  .code-text {
    font-size: var(--t-t2, 22px);
    font-weight: var(--w-semibold);
    letter-spacing: 0.08em;
    font-variant-numeric: tabular-nums;
    user-select: all;
  }
  .url {
    word-break: break-all;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--label-2);
    font-size: var(--t-foot);
  }
  .spin {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--fill);
    border-top-color: var(--label-2);
    animation: spin 0.9s linear infinite;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .foot {
    margin: 4px 0 0;
    font-size: var(--t-cap);
    color: var(--label-3);
  }
</style>
