/**
 * Infers an error code from raw backend text when the backend sent no code.
 *
 * The Russian phrases below are not shown to anyone: they match text that the Rust
 * core, the OS or older backends emit, so an uncoded error still maps to the right
 * dictionary entry in errors.ts. That is why this file is a documented exception in
 * the i18n guard (lib/i18n/guard.ts › ALLOWED_EXCEPTIONS).
 */

import { ERROR_CODES } from '../ipc/generated/error';
import type { ErrorDomain } from './errors';

/**
 * Pattern-matches raw error text to infer a known error code if not explicitly given.
 */
export function inferErrorCodeFromPatterns(rawText: string, domain?: ErrorDomain): string | null {
  const lower = rawText.toLowerCase();

  // 1. Internet & Connectivity patterns
  if (
    lower.includes('no internet') ||
    lower.includes('network unreachable') ||
    lower.includes('failed to connect') ||
    lower.includes('econnrefused') ||
    lower.includes('enotfound') ||
    lower.includes('нет интернета') ||
    lower.includes('нет подключения к интернету') ||
    lower.includes('сетевая ошибка') ||
    lower.includes('не удалось подключиться к сети')
  ) {
    return ERROR_CODES.NETWORK_ERROR;
  }

  // 2. Timeout & Server Unresponsive patterns
  if (
    lower.includes('timed out') ||
    lower.includes('timeout') ||
    lower.includes('таймаут') ||
    lower.includes('время ожидания') ||
    lower.includes('сервер не отвечает') ||
    lower.includes('gateway timeout') ||
    lower.includes('connection timeout')
  ) {
    return ERROR_CODES.IO_TIMED_OUT;
  }

  // 3. Disk Full patterns
  if (
    lower.includes('not enough space') ||
    lower.includes('no space left') ||
    lower.includes('disk full') ||
    lower.includes('error 112') ||
    lower.includes('error 39') ||
    lower.includes('нет места на диске') ||
    lower.includes('недостаточно свободного места') ||
    lower.includes('недостаточно места') ||
    lower.includes('диск заполнен')
  ) {
    return 'IO_NO_SPACE';
  }

  // 4. Antivirus & Permission Denied patterns
  if (
    lower.includes('antivirus') ||
    lower.includes('антивирус') ||
    lower.includes('access is denied') ||
    lower.includes('permission denied') ||
    lower.includes('os error 5') ||
    lower.includes('operation not permitted') ||
    lower.includes('доступ запрещён') ||
    lower.includes('файл заблокирован') ||
    lower.includes('заблокировал файл')
  ) {
    return ERROR_CODES.IO_PERMISSION_DENIED;
  }

  // 5. Memory / OOM patterns
  if (
    lower.includes('could not reserve enough space') ||
    lower.includes('outofmemoryerror') ||
    lower.includes('out of memory') ||
    lower.includes('insufficient memory') ||
    lower.includes('java heap space') ||
    lower.includes('нехватка оперативной памяти') ||
    lower.includes('мало памяти') ||
    lower.includes('недостаточно памяти')
  ) {
    return ERROR_CODES.LAUNCH_OUT_OF_MEMORY;
  }

  // 6. Java patterns
  if (
    lower.includes('java binary not found') ||
    lower.includes('не найден исполняемый файл java') ||
    lower.includes('не найдена среда java') ||
    lower.includes('не найдена java') ||
    lower.includes('javaw.exe') ||
    lower.includes('cannot find java') ||
    lower.includes('java не найдена')
  ) {
    return ERROR_CODES.JAVA_NOT_FOUND;
  }
  if (
    lower.includes('unsupported major.minor') ||
    lower.includes('requires java 17') ||
    lower.includes('requires java 21') ||
    lower.includes('несовместимая версия java')
  ) {
    return ERROR_CODES.JAVA_WRONG_VERSION;
  }
  if (
    lower.includes('java runtime error') ||
    lower.includes('java не запускается') ||
    lower.includes('jvm')
  ) {
    return ERROR_CODES.JAVA_RUNTIME_ERROR;
  }

  // 7. Process exit / crash patterns
  if (
    lower.includes('process exited with code') ||
    lower.includes('exit code -1') ||
    lower.includes('exit code 1') ||
    lower.includes('игра упала') ||
    lower.includes('сбой игрового процесса') ||
    lower.includes('аварийно завершился') ||
    lower.includes('game crashed')
  ) {
    return ERROR_CODES.LAUNCH_PROCESS_FAILED;
  }

  // Offline / Installed version
  if (
    lower.includes('не установлена') &&
    (lower.includes('подключение к сети') || lower.includes('офлайн'))
  ) {
    return ERROR_CODES.LAUNCH_OFFLINE_FORBIDDEN;
  }

  // Already running
  if (lower.includes('игра уже запущена') || lower.includes('already running')) {
    return ERROR_CODES.LAUNCH_ALREADY_RUNNING;
  }

  // Hash / Size mismatch
  if (lower.includes('hash mismatch') || lower.includes('хеш-сумма') || lower.includes('контрольная сумма')) {
    return ERROR_CODES.INSTALL_HASH_MISMATCH;
  }
  if (lower.includes('size mismatch') || lower.includes('размер файла')) {
    return ERROR_CODES.INSTALL_SIZE_MISMATCH;
  }

  // Downloads / Manifests
  if (
    lower.includes('download error') ||
    lower.includes('failed to download') ||
    lower.includes('не удалось загрузить') ||
    lower.includes('не удалось скачать')
  ) {
    return ERROR_CODES.INSTALL_DOWNLOAD_FAILED;
  }
  if (lower.includes('manifest error') || lower.includes('манифест')) {
    return ERROR_CODES.INSTALL_MANIFEST_ERROR;
  }

  // Pack sync
  if (lower.includes('пак wex') || lower.includes('fabric loader') || lower.includes('fabric error')) {
    return ERROR_CODES.PACK_SYNC_FAILED;
  }

  // Mods
  if (lower.includes('защищены от удаления') || lower.includes('пак wex защищены')) {
    return ERROR_CODES.MODS_PROTECTED_PACK_MOD;
  }
  if (lower.includes('.jar') && lower.includes('расширением')) {
    return ERROR_CODES.MODS_INVALID_FILE;
  }
  if (domain === 'mods' && (lower.includes('delete') || lower.includes('удалить'))) {
    return ERROR_CODES.MODS_DELETE_FAILED;
  }

  // Workshop
  if (lower.includes('level.dat')) {
    return ERROR_CODES.WORKSHOP_INVALID_LEVEL_DAT;
  }
  if (domain === 'workshop' && (lower.includes('zip') || lower.includes('архив'))) {
    return ERROR_CODES.WORKSHOP_ZIP_ERROR;
  }

  // Lobby
  if (lower.includes('сервис недоступен') || lower.includes('service unavailable')) {
    return ERROR_CODES.NET_LOBBY_NETWORK_ERROR;
  }
  if (lower.includes('исключены из комнаты') || lower.includes('kicked')) {
    return ERROR_CODES.NET_LOBBY_KICKED;
  }
  if (domain === 'lobby' && (lower.includes('не найдена') || lower.includes('not found') || lower.includes('404'))) {
    return ERROR_CODES.NET_LOBBY_NOT_FOUND;
  }
  if (domain === 'lobby' && lower.includes('код комнаты')) {
    return ERROR_CODES.NET_LOBBY_NOT_FOUND;
  }
  if (lower.includes('tunnel') || lower.includes('playit')) {
    return ERROR_CODES.NET_TUNNEL_ERROR;
  }

  // Social / Friends
  if (lower.includes('уже в вашем списке друзей') || lower.includes('already friends')) {
    return ERROR_CODES.SOCIAL_ALREADY_EXISTS;
  }

  // Not found
  if (lower.includes('not found') || lower.includes('не найден')) {
    return ERROR_CODES.IO_NOT_FOUND;
  }

  return null;
}
