import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import OnboardingModal from './OnboardingModal.svelte';

const persist = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => persist(...args) }));

describe('OnboardingModal Component (Apple HIG Onboarding)', () => {
  let target: HTMLElement;
  let comp: any = null;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    localStorage.clear();
    vi.clearAllMocks();
    persist.mockResolvedValue(undefined);
  });

  afterEach(() => {
    if (comp) {
      unmount(comp);
      comp = null;
    }
    target.remove();
  });

  it('renders welcome step with emblem, highlights, and prewarm indicator', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'welcome',
      },
    });

    await tick();

    expect(target.textContent).toContain('Добро пожаловать в Wex');
    expect(target.textContent).toContain('Оптимальная производительность');
    expect(target.textContent).toContain('Друзья и локальная сеть');
    expect(target.textContent).toContain('Начать настройку');
    expect(target.textContent).toContain('Пропустить');
  });

  it('saves wex_onboarding_completed to localStorage on skip', async () => {
    let skipped = false;
    let closed = false;

    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        onskip: () => { skipped = true; },
        onclose: () => { closed = true; },
      },
    });

    await tick();

    const skipBtn = target.querySelector('.skip-action') as HTMLButtonElement;
    expect(skipBtn).toBeTruthy();
    skipBtn.click();
    await tick();

    expect(skipped).toBe(true);
    expect(closed).toBe(true);
    expect(localStorage.getItem('wex_onboarding_completed')).toBe('true');
  });

  it('skips import step when no old Aura-Launcher is detected and proceeds to nickname', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'welcome',
      },
    });

    await tick();

    const nextBtn = target.querySelector('.footer-actions button') as HTMLButtonElement;
    expect(nextBtn.textContent).toContain('Начать настройку');
    nextBtn.click();
    await tick();

    // With no aura detected, advances directly to nickname
    expect(target.textContent).toContain('Как вас зовут в игре?');
    expect(target.textContent).toContain('Игровой ник');
  });

  it('validates nickname input allowing only 3-16 valid characters', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'nickname',
      },
    });

    await tick();

    const input = target.querySelector('#onboarding-nickname-input') as HTMLInputElement;
    expect(input).toBeTruthy();

    // Empty or short nickname
    input.value = 'ab';
    input.dispatchEvent(new Event('input'));
    await tick();

    const buttons = Array.from(target.querySelectorAll('.footer-actions button')) as HTMLButtonElement[];
    const nextBtn = buttons.find((b) => b.textContent?.includes('Далее'));
    expect(nextBtn?.disabled).toBe(true);

    // Valid nickname
    input.value = 'CraftMaster_99';
    input.dispatchEvent(new Event('input'));
    await tick();

    expect(nextBtn?.disabled).toBe(false);
  });

  it('allows version selection between Minecraft 1.21.1 and 1.20.1', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'version',
      },
    });

    await tick();

    expect(target.textContent).toContain('Выберите версию игры');
    const versionCards = Array.from(target.querySelectorAll('.version-card')) as HTMLButtonElement[];
    expect(versionCards.length).toBe(2);

    expect(versionCards[0].textContent).toContain('Minecraft 1.21.1');
    expect(versionCards[0].textContent).toContain('Рекомендуется');
    expect(versionCards[1].textContent).toContain('Minecraft 1.20.1');
    expect(versionCards[1].textContent).toContain('Классика');

    // Click 1.20.1
    versionCards[1].click();
    await tick();
    expect(versionCards[1].classList.contains('selected')).toBe(true);
  });

  it('displays RAM slider and contextual HIG explanation on memory step', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'memory',
      },
    });

    await tick();

    expect(target.textContent).toContain('Выделение оперативной памяти');
    expect(target.textContent).toContain('Всего в ПК');
    expect(target.querySelector('.ram-explanation-box')).toBeTruthy();
  });

  it('displays import step when Aura-Launcher is detected and calls extras_apply_import on executeImport', async () => {
    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'welcome',
      },
    });

    await tick();

    // Trigger step 'import' directly
    comp.goToStep('import');
    await tick();

    expect(target.textContent).toContain('Найден Aura-Launcher');
    expect(target.textContent).toContain('Настроить с нуля');
    expect(target.textContent).toContain('Перенести всё');

    // Clicking "Настроить с нуля" navigates to nickname step
    const buttons = Array.from(target.querySelectorAll('.footer-actions button')) as HTMLButtonElement[];
    const declineBtn = buttons.find((b) => b.textContent?.includes('Настроить с нуля'));
    expect(declineBtn).toBeTruthy();
    declineBtn?.click();
    await tick();

    expect(target.textContent).toContain('Как вас зовут в игре?');
  });

  it('completes onboarding and emits oncomplete with selected configuration', async () => {
    let completedData: any = null;

    comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
        initialStep: 'ready',
        oncomplete: (data) => {
          completedData = data;
        },
      },
    });

    await tick();

    expect(target.textContent).toContain('Всё готово к игре');
    const launchBtn = Array.from(target.querySelectorAll('.footer-actions button')).find(
      (b) => b.textContent?.includes('Начать игру')
    ) as HTMLButtonElement;

    expect(launchBtn).toBeTruthy();
    launchBtn.click();
    await tick();

    await vi.waitFor(() => expect(completedData).toBeTruthy());
    expect(completedData.launchImmediately).toBe(true);
    expect(localStorage.getItem('wex_onboarding_completed')).toBe('true');
  });
  it('review_16_waits_for_persistence_before_launching', async () => {
    let release!: () => void;
    const complete = vi.fn();
    persist.mockImplementation((cmd) => cmd === 'onboarding_complete' ? new Promise<void>(r => { release = r; }) : Promise.resolve(null));
    comp = mount(OnboardingModal, { target, props: { open: true, initialStep: 'ready', oncomplete: complete } });
    await tick();
    const finishing = comp.finishOnboarding(true);
    expect(complete).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(persist).toHaveBeenCalledWith('onboarding_complete', expect.anything()));
    expect(localStorage.getItem('wex_onboarding_completed')).toBeNull();
    release();
    await finishing;
    await vi.waitFor(() => expect(complete).toHaveBeenCalledOnce());
  });

  it('keeps onboarding open and retryable when saving fails', async () => {
    const complete = vi.fn();
    persist.mockImplementation((cmd) => cmd === 'onboarding_complete' ? Promise.reject(new Error('Диск недоступен')) : Promise.resolve(null));
    comp = mount(OnboardingModal, { target, props: { open: true, initialStep: 'ready', oncomplete: complete } });
    await tick();
    await comp.finishOnboarding(true); await tick();
    expect(complete).not.toHaveBeenCalled();
    expect(localStorage.getItem('wex_onboarding_completed')).toBeNull();
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('Диск недоступен');
    persist.mockResolvedValue(undefined);
    await comp.finishOnboarding(false); await tick();
    expect(complete).toHaveBeenCalledOnce();
  });

});
