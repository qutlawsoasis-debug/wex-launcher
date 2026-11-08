import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import CrashSheet from './CrashSheet.svelte';
import type { CrashDiagnosisDto } from '../ipc/generated/models';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('CrashSheet (SPEC 2.10: Apple HIG Game Crash Diagnostics)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    vi.clearAllMocks();

    // Mock navigator.clipboard
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(() => {
    target.remove();
  });

  it('renders OutOfMemory diagnosis with RAM badge and "Увеличить память" action', async () => {
    const oomDiagnosis: CrashDiagnosisDto = {
      kind: 'OutOfMemory',
      title: 'Нехватка оперативной памяти (ОЗУ)',
      user_advice:
        'Увеличьте выделенный объём памяти в Настройках лаунчера (рекомендуется 6–8 ГБ) или закройте фоновые приложения.',
      technical_summary:
        'Exception in thread "main" java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Main.main(Main.java:123)',
    };

    let navigatedTab = '';
    let navigatedHighlight = '';

    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        diagnosis: oomDiagnosis,
        version: '1.21.1',
        onnavigate: (tab, highlight) => {
          navigatedTab = tab;
          navigatedHighlight = highlight || '';
        },
      },
    });

    expect(target.textContent).toContain('Нехватка оперативной памяти (ОЗУ)');
    expect(target.textContent).toContain('Оперативная память');
    expect(target.textContent).toContain('Увеличьте выделенный объём памяти в Настройках лаунчера');
    expect(target.textContent).toContain('Java heap space');

    // Primary action button must be «Увеличить память»
    const ramBtn = target.querySelector('[data-testid="btn-increase-ram"]');
    expect(ramBtn).not.toBeNull();
    (ramBtn as HTMLElement)?.click();

    expect(navigatedTab).toBe('settings');
    expect(navigatedHighlight).toBe('ram');

    unmount(comp);
  });

  it('renders mod dependency diagnosis with mod badge and "Отключить мод" action', async () => {
    const modDiagnosis: CrashDiagnosisDto = {
      kind: {
        MissingFabricDependency: {
          mod_name: 'iris',
          required_dep: 'sodium',
        },
      },
      title: 'Отсутствует зависимость для «iris»',
      user_advice:
        'Мод «iris» требует обязательной установки «sodium». Установите недостающую библиотеку или удалите мод во вкладке Мастерская.',
      technical_summary:
        "net.fabricmc.loader.impl.FormattedException: Mod 'iris' (1.7.0) requires 'sodium' (0.5.8), but only 0.5.2 is present!",
    };

    invokeMock.mockResolvedValue('iris-mc1.21-1.7.0.jar.disabled');

    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        diagnosis: modDiagnosis,
        version: '1.21.1',
      },
    });

    expect(target.textContent).toContain('Отсутствует зависимость для «iris»');
    expect(target.textContent).toContain('Зависимость мода');
    expect(target.textContent).toContain('Мод «iris» требует обязательной установки «sodium»');

    // Destructive action button must be «Отключить мод»
    const disableBtn = target.querySelector('[data-testid="btn-disable-mod"]');
    expect(disableBtn).not.toBeNull();
    (disableBtn as HTMLElement)?.click();

    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(invokeMock).toHaveBeenCalledWith('crash_disable_mod', {
      modIdentifier: 'iris',
      version: '1.21.1',
    });

    // Advice changes to success notice
    expect(target.textContent).toContain('Мод успешно отключён (.disabled)');

    unmount(comp);
  });

  it('renders invalid Java version diagnosis with Java badge and "Выбрать Java" action', async () => {
    const javaDiagnosis: CrashDiagnosisDto = {
      kind: {
        InvalidJavaVersion: {
          required_java: 'Java 21 (class 65.0)',
          current_java: 'Java 17 (class 61.0)',
        },
      },
      title: 'Несовместимая версия Java',
      user_advice:
        'Игра или установленный мод требуют Java 21 (ваша текущая версия устарела). Выберите нужную версию Java в Настройках лаунчера.',
      technical_summary:
        'java.lang.UnsupportedClassVersionError: net/minecraft/client/Main has been compiled by a more recent version of the Java Runtime (class file version 65.0)',
    };

    let navigatedTab = '';
    let navigatedHighlight = '';

    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        diagnosis: javaDiagnosis,
        version: '1.21.1',
        onnavigate: (tab, highlight) => {
          navigatedTab = tab;
          navigatedHighlight = highlight || '';
        },
      },
    });

    expect(target.textContent).toContain('Несовместимая версия Java');
    expect(target.textContent).toContain('Среда Java');
    expect(target.textContent).toContain('Выберите нужную версию Java в Настройках');

    const javaBtn = target.querySelector('[data-testid="btn-select-java"]');
    expect(javaBtn).not.toBeNull();
    (javaBtn as HTMLElement)?.click();

    expect(navigatedTab).toBe('settings');
    expect(navigatedHighlight).toBe('java');

    unmount(comp);
  });

  it('renders unknown crash honestly with "Причина не определена", zero fake data, and technical log', async () => {
    const unknownDiagnosis: CrashDiagnosisDto = {
      kind: {
        Unknown: {
          exit_code: -1073740791,
        },
      },
      title: 'Игра завершилась с ошибкой (код -1073740791)',
      user_advice:
        'Нажмите «Показать лог», чтобы просмотреть технические детали, или обратитесь за поддержкой к разработчикам.',
      technical_summary:
        'ExitCode: -1073740791\n[17:00:00] [Render thread/INFO]: Game crashed with unknown native stack',
    };

    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        diagnosis: unknownDiagnosis,
        version: '1.20.1',
      },
    });

    // Must show honest cause: «Причина не определена»
    expect(target.textContent).toContain('Причина не определена');
    expect(target.textContent).toContain('-1073740791');
    expect(target.textContent).toContain('Game crashed with unknown native stack');

    // Zero fabricated causes or marketing text
    expect(target.textContent).not.toContain('Виноват антивирус');
    expect(target.textContent).not.toContain('Переустановите Windows');

    // Contextual fix button should NOT be shown for unknown cause
    expect(target.querySelector('[data-testid="btn-increase-ram"]')).toBeNull();
    expect(target.querySelector('[data-testid="btn-disable-mod"]')).toBeNull();

    // Standard secondary actions must be present
    expect(target.querySelector('[data-testid="btn-copy-report"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="btn-open-log"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="btn-close-crash"]')).not.toBeNull();

    unmount(comp);
  });

  it('allows copying full crash report to clipboard and opening log file', async () => {
    const oomDiagnosis: CrashDiagnosisDto = {
      kind: 'OutOfMemory',
      title: 'Нехватка оперативной памяти (ОЗУ)',
      user_advice: 'Увеличьте выделенный объём памяти в Настройках лаунчера.',
      technical_summary: 'OutOfMemoryError: Java heap space',
    };

    invokeMock.mockResolvedValue('C:/Games/Wex/instances/1.21.1/logs/latest.log');

    let toastTitle = '';
    const comp = mount(CrashSheet, {
      target,
      props: {
        open: true,
        diagnosis: oomDiagnosis,
        version: '1.21.1',
        ontoast: (type, title) => {
          toastTitle = title;
        },
      },
    });

    // Test Copy Report
    const copyBtn = target.querySelector('[data-testid="btn-copy-report"]');
    (copyBtn as HTMLElement)?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(navigator.clipboard.writeText).toHaveBeenCalled();
    expect(toastTitle).toContain('буфер');

    // Test Open Log
    const openLogBtn = target.querySelector('[data-testid="btn-open-log"]');
    (openLogBtn as HTMLElement)?.click();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(invokeMock).toHaveBeenCalledWith('crash_open_log_file', {
      version: '1.21.1',
    });

    unmount(comp);
  });
});
