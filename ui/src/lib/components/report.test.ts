import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import ReportSheet from './ReportSheet.svelte';

describe('ReportSheet Component (SPEC 2.10 & wex-parity)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    return () => {
      target.remove();
    };
  });

  it('renders ReportSheet with anonymized paths and version badge', () => {
    const comp = mount(ReportSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        diagnosisTitle: 'Нехватка оперативной памяти (OutOfMemoryError)',
        errorText: 'java.lang.OutOfMemoryError at C:\\Users\\Administrator\\AppData\\.wex\\instances\\1.21.1\\options.txt',
        playerName: 'SuperGamer',
      },
    });

    expect(target.textContent).toContain('Отчёт об ошибке');
    expect(target.textContent).toContain('1.21.1');
    expect(target.textContent).toContain('Нехватка оперативной памяти');
    expect(target.textContent).toContain('Обезличено');
    expect(target.textContent).toContain('C:\\Users\\***');
    expect(target.textContent).toContain('Сохранить на рабочий стол (.zip)');
    expect(target.textContent).toContain('Скопировать отчёт');
    unmount(comp);
  });

  it('allows expanding and viewing anonymized JSON preview', async () => {
    const comp = mount(ReportSheet, {
      target,
      props: {
        open: true,
        version: '1.20.1',
        errorText: 'Crash in C:\\Users\\SecretUser\\test.jar with player SecretUser',
        playerName: 'SecretUser',
      },
    });

    const toggleBtn = target.querySelector('[data-testid="btn-toggle-preview"]') as HTMLButtonElement;
    expect(toggleBtn).not.toBeNull();
    toggleBtn.click();
    await tick();

    // Check JSON preview is visible and anonymized
    const codebox = target.querySelector('[data-testid="report-json-preview"]');
    expect(codebox).not.toBeNull();
    expect(codebox?.textContent).not.toContain('SecretUser');
    expect(codebox?.textContent).toContain('C:\\\\Users\\\\***');
    expect(codebox?.textContent).toContain('<player>');
    unmount(comp);
  });

  it('handles save zip action and triggers success notification', async () => {
    let toastType = '';
    let toastTitle = '';

    const comp = mount(ReportSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        ontoast: (type, title) => {
          toastType = type;
          toastTitle = title;
        },
      },
    });

    const saveBtn = target.querySelector('[data-testid="btn-save-zip"]') as HTMLElement;
    expect(saveBtn).not.toBeNull();
    saveBtn.click();
    await tick();
    await new Promise((r) => setTimeout(r, 10));

    expect(toastType).toBe('success');
    expect(toastTitle).toContain('Отчёт сохранён на рабочий стол');
    unmount(comp);
  });

  it('handles copy report action to clipboard', async () => {
    let toastTitle = '';
    let copiedText = '';

    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockImplementation((text: string) => {
          copiedText = text;
          return Promise.resolve();
        }),
      },
    });

    const comp = mount(ReportSheet, {
      target,
      props: {
        open: true,
        version: '1.21.1',
        diagnosisTitle: 'Ошибка шейдера',
        errorText: 'OpenGL error in C:\\Users\\MyUser\\shaders.zip',
        playerName: 'MyUser',
        ontoast: (_type, title) => {
          toastTitle = title;
        },
      },
    });

    const copyBtn = target.querySelector('[data-testid="btn-copy-report"]') as HTMLElement;
    expect(copyBtn).not.toBeNull();
    copyBtn.click();
    await tick();
    // allow promise resolution
    await new Promise((r) => setTimeout(r, 10));

    expect(toastTitle).toContain('Отчёт скопирован в буфер обмена');
    unmount(comp);
  });
});
