import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import { formatError, sanitizeUserMessage, type FormattedError } from './errors';
import ErrorState from '../components/ErrorState.svelte';
import { ERROR_CODES, type IpcError } from '../ipc/generated/error';

const BANNED_TECHNICAL_WORDS = [
  'reqwest',
  'ipc',
  'os error',
  'exit code',
  'status code',
  'exception',
  'errno',
  'nullpointer',
  'stacktrace',
  'failed to',
  'c:\\users\\',
  '/home/',
  '/users/',
];

function assertNoBannedWordsOrPaths(text: string, context: string) {
  const lower = text.toLowerCase();
  for (const word of BANNED_TECHNICAL_WORDS) {
    expect(
      lower.includes(word),
      `[${context}] contains banned technical word or path "${word}": "${text}"`
    ).toBe(false);
  }
  // Must contain Russian Cyrillic characters
  const hasCyrillic = /[а-яёА-ЯЁ]/.test(text);
  expect(hasCyrillic, `[${context}] must be localized in Russian: "${text}"`).toBe(true);
}

describe('Error Mapping (formatError)', () => {
  it('maps typed IpcError object into clean Russian title and description', () => {
    const ipcErr: IpcError = {
      code: ERROR_CODES.JAVA_NOT_FOUND,
      message: 'Java not found at C:\\bin\\java.exe',
      technical: 'Exit code 1',
    };
    const formatted = formatError(ipcErr);
    expect(formatted.code).toBe(ERROR_CODES.JAVA_NOT_FOUND);
    expect(formatted.title).toBe('Не найдена среда Java');
    expect(formatted.description).toContain('Укажите путь к Java в настройках');
    expect(formatted.primaryAction?.type).toBe('settings');
    expect(formatted.primaryAction?.label).toBe('Настройки');
    expect(formatted.technical).toBe('Exit code 1');
  });

  it('parses JSON string representation of IpcError', () => {
    const jsonStr = JSON.stringify({
      code: ERROR_CODES.INSTALL_HASH_MISMATCH,
      message: 'Hash mismatch for client.jar',
      technical: 'expected abc, actual def',
    });
    const formatted = formatError(jsonStr);
    expect(formatted.code).toBe(ERROR_CODES.INSTALL_HASH_MISMATCH);
    expect(formatted.title).toBe('Файл игры повреждён');
    expect(formatted.description).toContain('Контрольная сумма');
    expect(formatted.primaryAction?.label).toBe('Повторить');
    expect(formatted.technical).toBe('expected abc, actual def');
  });

  it('parses Rust formatted string e.g. [CODE] message (technical)', () => {
    const rustStr = `[${ERROR_CODES.IO_PERMISSION_DENIED}] Access is denied. (os error 5)`;
    const formatted = formatError(rustStr);
    expect(formatted.code).toBe(ERROR_CODES.IO_PERMISSION_DENIED);
    expect(formatted.title).toBe('Доступ к файлам запрещён');
    expect(formatted.description).toContain('Операционная система ограничила доступ');
    expect(formatted.primaryAction?.type).toBe('folder');
    expect(formatted.technical).toBe('os error 5');
  });

  it('maps Java missing error from raw string pattern', () => {
    const raw = 'Java runtime error: java binary not found in PATH';
    const formatted = formatError(raw, 'launch');
    expect(formatted.code).toBe(ERROR_CODES.JAVA_NOT_FOUND);
    expect(formatted.title).toBe('Не найдена среда Java');
    expect(formatted.primaryAction?.type).toBe('settings');
  });

  it('maps Java wrong version error from raw string pattern', () => {
    const raw = 'Unsupported major.minor version 65.0, requires Java 21';
    const formatted = formatError(raw, 'java');
    expect(formatted.code).toBe(ERROR_CODES.JAVA_WRONG_VERSION);
    expect(formatted.title).toBe('Несовместимая версия Java');
    expect(formatted.primaryAction?.type).toBe('settings');
  });

  it('maps Out of Memory errors with guidance to adjust RAM in settings', () => {
    const raw = 'Could not reserve enough space for 8388608KB object heap';
    const formatted = formatError(raw, 'launch');
    expect(formatted.code).toBe('LAUNCH_OUT_OF_MEMORY');
    expect(formatted.title).toBe('Недостаточно оперативной памяти');
    expect(formatted.description).toContain('Уменьшите объём ОЗУ в настройках');
    expect(formatted.primaryAction?.type).toBe('settings');
  });

  it('maps offline launch forbidden error', () => {
    const raw = 'Версия 1.21.1 не установлена. Требуется подключение к сети для загрузки.';
    const formatted = formatError(raw, 'launch');
    expect(formatted.code).toBe(ERROR_CODES.LAUNCH_OFFLINE_FORBIDDEN);
    expect(formatted.title).toBe('Версия игры не установлена');
    expect(formatted.primaryAction?.label).toBe('Повторить');
  });

  it('maps game crash / process failed error', () => {
    const raw = 'Process exited with code 1';
    const formatted = formatError(raw, 'launch');
    expect(formatted.code).toBe(ERROR_CODES.LAUNCH_PROCESS_FAILED);
    expect(formatted.title).toBe('Сбой игрового процесса');
    expect(formatted.secondaryAction?.type).toBe('settings');
  });

  it('maps protected pack mod error', () => {
    const raw = 'Моды из официального пака Wex защищены от удаления.';
    const formatted = formatError(raw, 'mods');
    expect(formatted.code).toBe(ERROR_CODES.MODS_PROTECTED_PACK_MOD);
    expect(formatted.title).toBe('Мод защищён от удаления');
    expect(formatted.primaryAction?.label).toBe('Понятно');
  });

  it('maps invalid mod file format error', () => {
    const raw = 'Пожалуйста, выберите файл с расширением .jar';
    const formatted = formatError(raw, 'mods');
    expect(formatted.code).toBe(ERROR_CODES.MODS_INVALID_FILE);
    expect(formatted.title).toBe('Неподдерживаемый файл мода');
  });

  it('maps workshop world corrupted level.dat error', () => {
    const raw = 'Файл level.dat повреждён или имеет неподдерживаемый формат';
    const formatted = formatError(raw, 'workshop');
    expect(formatted.code).toBe(ERROR_CODES.WORKSHOP_INVALID_LEVEL_DAT);
    expect(formatted.title).toBe('Повреждён файл сохранения');
  });

  it('maps lobby kicked error', () => {
    const raw = 'Вы были исключены из комнаты организатором';
    const formatted = formatError(raw, 'lobby');
    expect(formatted.code).toBe('NET_LOBBY_KICKED');
    expect(formatted.title).toBe('Вы исключены из комнаты');
  });

  it('maps disk full error', () => {
    const raw = 'There is not enough space on the disk.';
    const formatted = formatError(raw, 'workshop');
    expect(formatted.code).toBe('IO_NO_SPACE');
    expect(formatted.title).toBe('Недостаточно места на диске');
    expect(formatted.primaryAction?.type).toBe('retry');
  });

  it('uses domain fallback when completely unknown error is provided', () => {
    const formatted = formatError('Some weird esoteric error 0x8892', 'friends');
    expect(formatted.title).toBe('Ошибка социальной службы');
    expect(formatted.description).toContain('Не удалось выполнить операцию');
    expect(formatted.technical).toBe('Some weird esoteric error 0x8892');
  });
});

describe('ErrorState Component', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders title, description and primary action button', async () => {
    let actionFired = false;
    const component = mount(ErrorState, {
      target,
      props: {
        error: {
          code: ERROR_CODES.LAUNCH_FAILED,
          message: 'Failed to launch game',
          technical: 'Stack trace line 1',
        },
        domain: 'launch',
        onaction: () => {
          actionFired = true;
        },
      },
    });

    await tick();

    const titleEl = target.querySelector('.error-title');
    expect(titleEl?.textContent).toBe('Не удалось запустить игру');

    const descEl = target.querySelector('.error-description');
    expect(descEl?.textContent).toContain('Проверьте целостность файлов');

    const primaryBtn = target.querySelector('.error-actions-row button') as HTMLButtonElement;
    expect(primaryBtn?.textContent).toContain('Повторить');

    primaryBtn.click();
    expect(actionFired).toBe(true);

    unmount(component);
  });

  it('toggles collapsible technical details section and provides copy button', async () => {
    // Mock navigator.clipboard
    let copiedText = '';
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn(async (txt: string) => {
          copiedText = txt;
        }),
      },
    });

    const component = mount(ErrorState, {
      target,
      props: {
        title: 'Тестовая ошибка',
        description: 'Описание действия',
        technical: 'Detailed technical error log (os error 13)',
      },
    });

    await tick();

    // Details should initially be collapsed
    expect(target.querySelector('.technical-details-box')).toBeNull();

    // Click toggle button
    const toggleBtn = target.querySelector('.disclosure-toggle-btn') as HTMLButtonElement;
    expect(toggleBtn?.textContent).toContain('Подробнее');
    toggleBtn.click();

    await tick();

    // Details box should now be visible
    const detailsBox = target.querySelector('.technical-details-box');
    expect(detailsBox).not.toBeNull();
    expect(detailsBox?.textContent).toContain('Detailed technical error log (os error 13)');

    // Click copy button
    const copyBtn = target.querySelector('.copy-btn') as HTMLButtonElement;
    copyBtn.click();

    await tick();
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      'Detailed technical error log (os error 13)'
    );

    unmount(component);
  });
});

describe('Strict Player-Facing Error Texts (wex-error-texts requirements)', () => {
  it('covers Scenario 1: нет интернета (no internet)', () => {
    const rawPatterns = [
      'нет интернета',
      'No internet connection detected',
      'network unreachable (os error 101)',
      'failed to connect to host',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'install');
      expect(formatted.code).toBe(ERROR_CODES.NETWORK_ERROR);
      expect(formatted.title).toBe('Нет подключения к интернету');
      assertNoBannedWordsOrPaths(formatted.title, 'no-internet-title');
      assertNoBannedWordsOrPaths(formatted.description, 'no-internet-desc');
    }

    const typedErr = formatError({ code: ERROR_CODES.NETWORK_ERROR });
    expect(typedErr.title).toBe('Нет подключения к интернету');
    assertNoBannedWordsOrPaths(typedErr.title, 'no-internet-typed-title');
    assertNoBannedWordsOrPaths(typedErr.description, 'no-internet-typed-desc');
  });

  it('covers Scenario 2: сервер не отвечает (server timeout / unresponsive)', () => {
    const rawPatterns = [
      'timed out',
      'Connection timed out after 3000ms',
      'сервер не отвечает на запрос',
      'gateway timeout 504',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'lobby');
      expect(formatted.code).toBe(ERROR_CODES.IO_TIMED_OUT);
      expect(formatted.title).toBe('Сервер не отвечает');
      assertNoBannedWordsOrPaths(formatted.title, 'server-timeout-title');
      assertNoBannedWordsOrPaths(formatted.description, 'server-timeout-desc');
    }

    const typedErr = formatError({ code: ERROR_CODES.IO_TIMED_OUT });
    expect(typedErr.title).toBe('Сервер не отвечает');
    assertNoBannedWordsOrPaths(typedErr.title, 'server-timeout-typed-title');
    assertNoBannedWordsOrPaths(typedErr.description, 'server-timeout-typed-desc');
  });

  it('covers Scenario 3: нет места на диске (disk full / out of space)', () => {
    const rawPatterns = [
      'There is not enough space on the disk.',
      'no space left on device',
      'диск заполнен: освободите место',
      'error 112: disk full',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'workshop');
      expect(formatted.title).toBe('Недостаточно места на диске');
      assertNoBannedWordsOrPaths(formatted.title, 'disk-full-title');
      assertNoBannedWordsOrPaths(formatted.description, 'disk-full-desc');
    }

    const typedErr = formatError({ code: ERROR_CODES.IO_DISK_FULL });
    expect(typedErr.title).toBe('Недостаточно места на диске');
    assertNoBannedWordsOrPaths(typedErr.title, 'disk-full-typed-title');
    assertNoBannedWordsOrPaths(typedErr.description, 'disk-full-typed-desc');
  });

  it('covers Scenario 4: антивирус заблокировал файл (antivirus / access denied)', () => {
    const rawPatterns = [
      'Антивирус заблокировал файл client.jar',
      'Access is denied (os error 5)',
      'Permission denied: operation not permitted by antivirus',
      'Файл заблокирован антивирусом',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'launch');
      assertNoBannedWordsOrPaths(formatted.title, 'antivirus-title');
      assertNoBannedWordsOrPaths(formatted.description, 'antivirus-desc');
      // Description must mention antivirus or system access
      expect(
        formatted.description.includes('антивирус') || formatted.description.includes('Операционная система')
      ).toBe(true);
    }

    const typedErr = formatError({ code: 'ANTIVIRUS_BLOCKED' });
    expect(typedErr.title).toBe('Антивирус заблокировал файл');
    assertNoBannedWordsOrPaths(typedErr.title, 'antivirus-typed-title');
    assertNoBannedWordsOrPaths(typedErr.description, 'antivirus-typed-desc');
  });

  it('covers Scenario 5: Java не запускается (Java missing / launch failure)', () => {
    const rawMissing = 'cannot find java at C:\\Program Files\\Java';
    const formattedMissing = formatError(rawMissing, 'java');
    expect(formattedMissing.code).toBe(ERROR_CODES.JAVA_NOT_FOUND);
    assertNoBannedWordsOrPaths(formattedMissing.title, 'java-missing-title');
    assertNoBannedWordsOrPaths(formattedMissing.description, 'java-missing-desc');
    expect(formattedMissing.primaryAction?.type).toBe('settings');

    const rawRuntime = 'Java runtime error: failed to launch JVM';
    const formattedRuntime = formatError(rawRuntime, 'java');
    expect(formattedRuntime.code).toBe(ERROR_CODES.JAVA_RUNTIME_ERROR);
    expect(formattedRuntime.title).toBe('Ошибка среды выполнения Java');
    assertNoBannedWordsOrPaths(formattedRuntime.title, 'java-runtime-title');
    assertNoBannedWordsOrPaths(formattedRuntime.description, 'java-runtime-desc');
    expect(formattedRuntime.primaryAction?.type).toBe('settings');
  });

  it('covers Scenario 6: игра упала (game crashed / process exit)', () => {
    const rawPatterns = [
      'Process exited with code 1',
      'Process exited with code -1',
      'Игра упала при загрузке мира',
      'Game crashed due to exception',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'launch');
      expect(formatted.code).toBe(ERROR_CODES.LAUNCH_PROCESS_FAILED);
      expect(formatted.title).toBe('Сбой игрового процесса');
      assertNoBannedWordsOrPaths(formatted.title, 'game-crash-title');
      assertNoBannedWordsOrPaths(formatted.description, 'game-crash-desc');
      expect(formatted.secondaryAction?.type).toBe('settings');
    }
  });

  it('covers Scenario 7: мало памяти (out of memory)', () => {
    const rawPatterns = [
      'Could not reserve enough space for 8388608KB object heap',
      'OutOfMemoryError: Java heap space',
      'нехватка оперативной памяти',
    ];

    for (const raw of rawPatterns) {
      const formatted = formatError(raw, 'launch');
      expect(formatted.code).toBe(ERROR_CODES.LAUNCH_OUT_OF_MEMORY);
      expect(formatted.title).toBe('Недостаточно оперативной памяти');
      assertNoBannedWordsOrPaths(formatted.title, 'oom-title');
      assertNoBannedWordsOrPaths(formatted.description, 'oom-desc');
      expect(formatted.primaryAction?.type).toBe('settings');
    }
  });

  it('verifies that ALL error codes produce clean Russian without banned technical jargon or paths', () => {
    for (const [codeKey, codeVal] of Object.entries(ERROR_CODES)) {
      const formatted = formatError({ code: codeVal });
      assertNoBannedWordsOrPaths(formatted.title, `code-${codeKey}-title`);
      assertNoBannedWordsOrPaths(formatted.description, `code-${codeKey}-desc`);
      expect(formatted.title.length).toBeGreaterThan(3);
      expect(formatted.description.length).toBeGreaterThan(10);
    }
  });

  it('sanitizes user paths and technical prefixes from raw error strings', () => {
    const rawWithPath = 'C:\\Users\\JohnDoe\\AppData\\Local\\Wex\\launcher.exe (os error 5)';
    const sanitized = sanitizeUserMessage(rawWithPath);
    expect(sanitized.includes('JohnDoe')).toBe(false);
    expect(sanitized.includes('os error')).toBe(false);
    expect(sanitized).toBe('AppData\\Local\\Wex\\launcher.exe');

    const unixPath = '/home/username/wex/repo/error.log [IO_ERROR]';
    const sanitizedUnix = sanitizeUserMessage(unixPath);
    expect(sanitizedUnix.includes('username')).toBe(false);
    expect(sanitizedUnix.includes('/home/')).toBe(false);
    expect(sanitizedUnix).toBe('wex/repo/error.log');
  });
});
