import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));

import { builds } from '../../shell/builds.svelte';
import { launch } from '../../shell/launch.svelte';
import { toasts } from '../../shell/toast.svelte';
import { theme } from '../../design/theme.svelte';
import { i18n } from '../../i18n';
import SettingsPage from './SettingsPage.svelte';

let settings: Record<string, unknown>;
let oldLauncher: string | null;
type Acc = { id: string; kind: 'offline' | 'microsoft'; name: string; uuid: string; needs_sign_in: boolean; last_used_at: string | null };
let accs: { accounts: Acc[]; active_id: string | null; microsoft_available: boolean; token_storage: string };

beforeEach(async () => {
  mocks.invoke.mockReset();
  oldLauncher = null;
  settings = {
    selected_version: '1.21.1',
    player_name: 'Player',
    ram_mb: 6144,
    total_memory_mb: 16384,
    graphics_preset: 'Medium',
    appearance: 'auto',
    language: 'ru',
    reduce_motion: false,
    reduce_transparency: false,
    discord_rpc: true,
    fast_start_jvm: false,
    fast_launch: true,
    friend_notifications: true,
    minimize_to_tray: true,
    game_window_behavior: 'minimize',
  };
  accs = {
    accounts: [
      { id: 'offline-a', kind: 'offline', name: 'Player', uuid: 'a01e3843e5213998958af459800e4d11', needs_sign_in: false, last_used_at: null },
      { id: 'ms-b', kind: 'microsoft', name: 'Notch', uuid: '069a79f444e94726a5befca90e38aaf5', needs_sign_in: false, last_used_at: null },
    ],
    active_id: 'offline-a',
    microsoft_available: false,
    token_storage: 'dpapi-user',
  };
  mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'accounts_list':
        return structuredClone(accs);
      case 'accounts_switch':
        accs.active_id = args!.id as string;
        return structuredClone(accs);
      case 'accounts_sign_out':
        accs.accounts = accs.accounts.filter((a) => a.id !== args!.id);
        if (accs.active_id === args!.id) accs.active_id = accs.accounts[0]?.id ?? null;
        return structuredClone(accs);
      case 'accounts_add_offline': {
        const name = args!.name as string;
        accs.accounts.push({ id: `offline-${name}`, kind: 'offline', name, uuid: 'x', needs_sign_in: false, last_used_at: null });
        accs.active_id = `offline-${name}`;
        return structuredClone(accs);
      }
      case 'get_system_info':
        return { total_memory_mb: 16384, free_memory_mb: 12288 };
      case 'get_launcher_settings':
        return { ...settings };
      case 'save_launcher_settings':
        settings = { ...(args!.newSettings as Record<string, unknown>) };
        return null;
      case 'get_installed_versions':
        return [{ id: '1.21.1', name: '1.21.1', is_installed: true }];
      case 'get_storage_cache_info':
        return { total_bytes: 1288490188, formatted_size: '1,2 ГБ' };
      case 'clear_storage_cache':
        return { formatted_freed: '1,2 ГБ' };
      case 'get_data_dir':
        return 'C:\\Users\\P\\AppData\\Roaming\\Wex';
      case 'extras_detect_old_launcher':
        return oldLauncher;
      case 'startup_get_info':
        return { app_version: '0.9.2' };
      case 'updater_check':
        return { update_available: false, current_version: '0.9.2', latest_version: null };
      case 'reset_settings':
        settings = { ...settings, graphics_preset: 'Low', ram_mb: 4096, appearance: 'auto', reduce_motion: false };
        return { ...settings };
      default:
        return null;
    }
  });
  i18n.setLocale('ru');
  toasts.dismiss();
  launch.onStatus({ state: 'idle', message: '' });
  await builds.load();
});

let target: HTMLElement;
let comp: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  target = document.createElement('div');
  target.className = 'wx';
  document.body.appendChild(target);
});
afterEach(() => {
  if (comp) unmount(comp);
  comp = null;
  target.remove();
  theme.setAppearance('auto');
  theme.setReduceMotion(false);
  theme.setReduceTransparency(false);
  i18n.setLocale('ru');
});

async function open() {
  comp = mount(SettingsPage, { target });
  await vi.waitFor(() => expect(target.querySelector<HTMLButtonElement>('.switch')?.disabled).toBe(false));
  await vi.waitFor(() => expect(target.textContent).toContain('1,2 ГБ'));
  await tick();
}

const row = (label: string) => [...target.querySelectorAll<HTMLElement>('.srow')].find((r) => r.querySelector('.l')?.textContent === label)!;

describe('settings › accounts', () => {
  const accRow = (name: string) => [...target.querySelectorAll<HTMLElement>('.acc-row')].find((r) => r.querySelector('.l')?.textContent === name)!;
  const button = (label: string) => [...target.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === label)!;

  it('lists accounts, marks the active one with a checkmark and switches', async () => {
    await open();
    await vi.waitFor(() => expect(target.querySelectorAll('.acc-row').length).toBe(2));
    expect(accRow('Player').textContent).toContain('Без лицензии');
    expect(accRow('Player').querySelector('.acc-current')?.textContent).toContain('Выбран');
    expect(accRow('Notch').textContent).toContain('Microsoft · лицензия');
    [...accRow('Notch').querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent === 'Выбрать')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('accounts_switch', { id: 'ms-b' }));
    await vi.waitFor(() => expect(builds.playerName).toBe('Notch'));
    await vi.waitFor(() => expect(accRow('Notch').querySelector('.acc-current')).not.toBeNull());
  });

  it('sign-out asks twice and explains what is deleted', async () => {
    await open();
    await vi.waitFor(() => expect(target.querySelectorAll('.acc-row').length).toBe(2));
    const out = () => accRow('Notch').querySelector<HTMLButtonElement>('.btn.danger')!;
    out().click();
    flushSync();
    expect(mocks.invoke).not.toHaveBeenCalledWith('accounts_sign_out', expect.anything());
    expect(target.textContent).toContain('удалит сохранённый вход Microsoft');
    out().click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('accounts_sign_out', { id: 'ms-b' }));
    await vi.waitFor(() => expect(target.querySelectorAll('.acc-row').length).toBe(1));
  });

  it('sign-in sheet: Microsoft is unavailable without a client ID; offline validates the name', async () => {
    await open();
    await vi.waitFor(() => expect(target.querySelectorAll('.acc-row').length).toBe(2));
    button('Добавить…').click();
    await vi.waitFor(() => expect(target.querySelector('[role="dialog"]')).not.toBeNull());
    const ways = [...target.querySelectorAll<HTMLButtonElement>('.way')];
    expect(ways[0].textContent).toContain('Войти через Microsoft');
    expect(ways[0].disabled).toBe(true);
    expect(ways[0].textContent).toContain('WEX_MS_CLIENT_ID');
    ways[1].click();
    flushSync();
    const input = target.querySelector<HTMLInputElement>('#acc-nick')!;
    input.value = 'no';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    const submit = target.querySelector<HTMLButtonElement>('.offline button[type="submit"]')!;
    expect(submit.disabled).toBe(true);
    expect(target.querySelector('#acc-nick-hint')?.textContent).toContain('от 3 до 16');
    input.value = 'Friend_2';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(submit.disabled).toBe(false);
    submit.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('accounts_add_offline', { name: 'Friend_2' }));
    await vi.waitFor(() => expect(target.querySelector('[role="dialog"]')).toBeNull());
    expect(builds.playerName).toBe('Friend_2');
  });

  it('Microsoft sign-in shows the device code, opens the page and saves the account', async () => {
    accs.microsoft_available = true;
    let finish: () => void = () => {};
    const base = mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'accounts_ms_begin') return { flow_id: 'f1', user_code: 'ABCD-EFGH', verification_uri: 'https://www.microsoft.com/link', expires_in: 900 };
      if (cmd === 'accounts_ms_finish') {
        await new Promise<void>((r) => (finish = r));
        accs.active_id = 'ms-b';
        return structuredClone(accs);
      }
      return base(cmd, args);
    });
    await open();
    await vi.waitFor(() => expect(target.querySelectorAll('.acc-row').length).toBe(2));
    button('Добавить…').click();
    await vi.waitFor(() => expect(target.querySelector<HTMLButtonElement>('.way')?.disabled).toBe(false));
    target.querySelector<HTMLButtonElement>('.way')!.click();
    await vi.waitFor(() => expect(target.querySelector('.code-text')?.textContent).toBe('ABCD-EFGH'));
    expect(target.textContent).toContain('Ждём подтверждения');
    button('Открыть страницу Microsoft').click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('accounts_ms_open_page', { flowId: 'f1' }));
    finish();
    await vi.waitFor(() => expect(target.querySelector('[role="dialog"]')).toBeNull());
    expect(builds.playerName).toBe('Notch');
  });
});

describe('settings page', () => {
  it('has the reference sections and only settings core supports', async () => {
    await open();
    expect([...target.querySelectorAll('.sgroup h2')].map((h) => h.textContent)).toEqual(['Аккаунты', 'Игра', 'Графика', 'Внешний вид', 'Лаунчер', 'Данные', 'Клавиши']);
    expect([...target.querySelectorAll('.subnav a')].map((a) => a.textContent)).toEqual(['Аккаунты', 'Игра', 'Графика', 'Внешний вид', 'Лаунчер', 'Данные', 'Клавиши']);
    // Not in core, so not shown at all (owner rule).
    for (const missing of ['Окно игры', 'Полноэкранный', 'Аргументы Java', 'Дальность прорисовки', 'Ограничение кадров', 'Distant Horizons', 'Плотность', 'Цвет Wex', 'вместе с Windows', 'Канал обновлений', 'Одновременных загрузок', 'Изменить']) {
      expect(target.textContent).not.toContain(missing);
    }
    // Present and from core.
    expect(target.textContent).toContain('Язык');
    expect(target.textContent).toContain('Уменьшить прозрачность');
    expect(target.textContent).toContain('Статус в Discord');
    expect(target.textContent).toContain('Быстрый запуск игры');
    expect(row('Папка Wex').textContent).toContain('C:\\Users\\P\\AppData\\Roaming\\Wex');
    expect(row('Кэш загрузок').textContent).toContain('1,2 ГБ');
    expect(target.textContent).not.toContain('Перенести из Aura-Launcher');
  });

  it('memory: bar and legend from the real total, saved through core', async () => {
    await open();
    expect(target.querySelector('.ram-legend')?.textContent).toContain('Minecraft, 6 ГБ');
    // In use now = total − free from get_system_info, not an assumed 4 GB.
    expect(target.querySelector('.ram-legend')?.textContent).toContain('Сейчас занято Windows и программами, 4 ГБ');
    expect(target.querySelector('.ram-legend')?.textContent).toContain('Свободно, 6 ГБ');
    const range = target.querySelector<HTMLInputElement>('input[type="range"]')!;
    expect(range.max).toBe('14');
    range.value = '13';
    range.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(target.querySelector('.ram .mc')?.classList.contains('warn')).toBe(true);
    expect(target.textContent).toContain('Столько свободной памяти сейчас нет');
    await vi.waitFor(() => expect(settings.ram_mb).toBe(13 * 1024), { timeout: 2000 });
  });

  it('graphics preset, theme, language, animations and transparency are saved and applied', async () => {
    await open();
    const presets = target.querySelectorAll<HTMLButtonElement>('.preset');
    expect(presets[1].getAttribute('aria-checked')).toBe('true');
    // Preset contents live in core; the page does not repeat render distances.
    expect(presets[0].textContent).not.toMatch(/чанк/);
    presets[3].click();
    await vi.waitFor(() => expect(settings.graphics_preset).toBe('Ultra'));

    [...target.querySelectorAll<HTMLButtonElement>('.theme-card')].find((b) => b.textContent?.includes('Тёмная'))!.click();
    await vi.waitFor(() => expect(settings.appearance).toBe('dark'));
    expect(document.documentElement.dataset.theme).toBe('dark');

    row('Анимации').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.reduce_motion).toBe(true));
    expect(document.documentElement.dataset.reduceMotion).toBe('true');

    row('Уменьшить прозрачность').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.reduce_transparency).toBe(true));
    expect(document.documentElement.dataset.reduceTransparency).toBe('true');

    [...row('Язык').querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent === 'English')!.click();
    await vi.waitFor(() => expect(settings.language).toBe('en'));
    await vi.waitFor(() => expect(target.textContent).toContain('Graphics'));
    expect(settings.player_name).toBe('Player');
  });

  it('launcher switches and the after-launch choice write their own fields', async () => {
    await open();
    [...row('Когда игра запустилась').querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent === 'В трей')!.click();
    await vi.waitFor(() => expect(settings.game_window_behavior).toBe('tray'));
    row('Статус в Discord').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.discord_rpc).toBe(false));
    row('Быстрый запуск игры').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.fast_start_jvm).toBe(true));
    // «Готовить игру заранее» is fast_launch (prewarm); off stops it in core.
    expect(row('Готовить игру заранее').querySelector('.switch')?.getAttribute('aria-checked')).toBe('true');
    row('Готовить игру заранее').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.fast_launch).toBe(false));
    row('Сворачивать в трей при закрытии').querySelector<HTMLButtonElement>('.switch')!.click();
    await vi.waitFor(() => expect(settings.minimize_to_tray).toBe(false));
  });

  it('search hides rows and whole sections, and says when nothing matches', async () => {
    await open();
    const input = target.querySelector<HTMLInputElement>('.set-search input')!;
    input.value = 'тема';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(() => expect(target.querySelector<HTMLElement>('#sec-game')?.hidden).toBe(true));
    expect(target.querySelector<HTMLElement>('#sec-look')?.hidden).toBe(false);
    input.value = 'абракадабра';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(() => expect(target.textContent).toContain('Такой настройки нет.'));
  });

  it('cache clears through core; reset asks twice', async () => {
    await open();
    row('Кэш загрузок').querySelector<HTMLButtonElement>('.btn')!.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('clear_storage_cache', undefined));
    await vi.waitFor(() => expect(toasts.current?.title).toBe('Кэш очищен'));
    const reset = row('Сбросить настройки').querySelector<HTMLButtonElement>('.btn')!;
    reset.click();
    flushSync();
    expect(reset.textContent).toBe('Нажмите ещё раз');
    expect(mocks.invoke).not.toHaveBeenCalledWith('reset_settings', undefined);
    reset.click();
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('reset_settings', undefined));
    await vi.waitFor(() => expect(toasts.current?.title).toBe('Настройки сброшены'));
    expect(builds.ramMb).toBe(4096);
  });

  it('offers moving from Aura-Launcher only when it is found', async () => {
    oldLauncher = 'C:\\Users\\P\\AppData\\Roaming\\AuraLauncher';
    await open();
    await vi.waitFor(() => expect(target.textContent).toContain('Перенести из Aura-Launcher'));
  });

  it('update check reports the result from updater_check', async () => {
    await open();
    row('Обновления лаунчера').querySelector<HTMLButtonElement>('.btn')!.click();
    await vi.waitFor(() => expect(row('Обновления лаунчера').textContent).toContain('Установлена последняя версия'));
  });
});
