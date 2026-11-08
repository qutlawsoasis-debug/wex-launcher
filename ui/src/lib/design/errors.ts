/**
 * Wex Launcher — Unified Error Handling & Mapping System (Apple HIG)
 *
 * Guidelines:
 * - Apple HIG: Alerts (https://developer.apple.com/design/human-interface-guidelines/alerts)
 *   "Give an alert a clear, specific title stating what happened, with a message stating what to do."
 * - Apple HIG: Writing (https://developer.apple.com/design/human-interface-guidelines/writing)
 *   "Be concise, clear, and direct. Avoid technical jargon. Keep technical details secondary."
 *
 * Provides typed mapping of backend error codes (wex-ipc, core, OS) into user-friendly Russian:
 * 1. «Что случилось» (title)
 * 2. «Что сделать» (description)
 * 3. Primary & secondary action buttons (Повторить, Открыть папку, Настройки)
 * 4. Collapsible technical details for debugging/logs
 */

import { ERROR_CODES, type IpcError } from '../ipc/generated/error';

export type ErrorDomain =
  | 'launch'
  | 'install'
  | 'pack'
  | 'java'
  | 'mods'
  | 'workshop'
  | 'lobby'
  | 'friends'
  | 'settings'
  | 'general';

export type ErrorActionType = 'retry' | 'folder' | 'settings' | 'close' | 'custom';

export interface ErrorAction {
  label: string;
  type: ErrorActionType;
  handler?: () => any;
}

export interface FormattedError {
  code: string;
  title: string;
  description: string;
  primaryAction?: ErrorAction;
  secondaryAction?: ErrorAction;
  technical?: string | null;
}

interface ErrorMappingDefinition {
  title: string;
  description: string;
  primaryAction?: ErrorAction;
  secondaryAction?: ErrorAction;
}

/**
 * Canonical dictionary mapping known error codes to Apple HIG compliant explanations.
 */
const ERROR_DICTIONARY: Record<string, ErrorMappingDefinition> = {
  // ==========================================================================
  // Launch & Runtime Errors
  // ==========================================================================
  [ERROR_CODES.LAUNCH_FAILED]: {
    title: 'Не удалось запустить игру',
    description: 'Проверьте целостность файлов и повторите запуск.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  [ERROR_CODES.LAUNCH_PROCESS_FAILED]: {
    title: 'Сбой игрового процесса',
    description: 'Minecraft неожиданно завершил работу. Проверьте отчёт о сбое или параметры памяти в настройках.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Настройки', type: 'settings' },
  },
  [ERROR_CODES.LAUNCH_OFFLINE_FORBIDDEN]: {
    title: 'Версия игры не установлена',
    description: 'Для первого запуска требуется подключение к интернету, чтобы загрузить необходимые файлы.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.LAUNCH_ALREADY_RUNNING]: {
    title: 'Игра уже запущена',
    description: 'Дождитесь закрытия текущего процесса Minecraft перед повторным стартом.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  LAUNCH_OUT_OF_MEMORY: {
    title: 'Недостаточно оперативной памяти',
    description: 'Системе не удалось выделить запрашиваемый объём памяти для Java. Уменьшите объём ОЗУ в настройках.',
    primaryAction: { label: 'Настройки', type: 'settings' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // Java Runtime Errors
  // ==========================================================================
  [ERROR_CODES.JAVA_NOT_FOUND]: {
    title: 'Не найдена среда Java',
    description: 'Для запуска Minecraft требуется Java. Укажите путь к Java в настройках или установите подходящую версию.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  [ERROR_CODES.JAVA_WRONG_VERSION]: {
    title: 'Несовместимая версия Java',
    description: 'Для этой версии Minecraft требуется более новая среда Java (Java 17 или 21). Измените среду в настройках.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  [ERROR_CODES.JAVA_RUNTIME_ERROR]: {
    title: 'Ошибка среды выполнения Java',
    description: 'Сбой при инициализации виртуальной машины Java. Проверьте аргументы запуска в настройках.',
    primaryAction: { label: 'Настройки', type: 'settings' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // Installation & Pack Sync Errors
  // ==========================================================================
  [ERROR_CODES.INSTALL_FAILED]: {
    title: 'Ошибка установки клиента',
    description: 'Не удалось завершить установку файлов игры. Проверьте соединение и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_DOWNLOAD_FAILED]: {
    title: 'Не удалось загрузить файлы игры',
    description: 'Произошёл сбой при скачивании компонентов с сервера. Проверьте интернет и повторите загрузку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_HASH_MISMATCH]: {
    title: 'Файл игры повреждён',
    description: 'Контрольная сумма скачанного файла не совпадает. Повторите загрузку для автоматического исправления.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_SIZE_MISMATCH]: {
    title: 'Загрузка не завершена',
    description: 'Размер скачанного файла меньше ожидаемого. Повторите загрузку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_MANIFEST_ERROR]: {
    title: 'Ошибка манифеста версии',
    description: 'Сервер вернул повреждённую структуру версии. Попробуйте обновить список версий.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.PACK_SYNC_FAILED]: {
    title: 'Не удалось синхронизировать пак Wex',
    description: 'Сетевой сбой при обновлении набора модов. Повторите попытку или запустите игру без обновления.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // Mods Management
  // ==========================================================================
  [ERROR_CODES.MODS_PROTECTED_PACK_MOD]: {
    title: 'Мод защищён от удаления',
    description: 'Этот мод входит в обязательный пак Wex. Вы можете временно выключить его переключателем, не удаляя с диска.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.MODS_INVALID_FILE]: {
    title: 'Неподдерживаемый файл мода',
    description: 'Лаунчер поддерживает только файлы модификаций с расширением .jar.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.MODS_LOAD_FAILED]: {
    title: 'Не удалось загрузить моды',
    description: 'Произошла ошибка при чтении каталога модов. Проверьте доступность папки.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.MODS_DELETE_FAILED]: {
    title: 'Не удалось удалить мод',
    description: 'Файл мода занят другим процессом или защищён от записи. Закройте игру и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },

  // ==========================================================================
  // Workshop (Saves & Backups)
  // ==========================================================================
  [ERROR_CODES.WORKSHOP_NOT_FOUND]: {
    title: 'Файл мира не найден',
    description: 'Папка с сохранением была перемещена или удалена. Обновите список миров.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  [ERROR_CODES.WORKSHOP_IO_ERROR]: {
    title: 'Ошибка доступа к миру',
    description: 'Не удалось прочитать или записать файлы сохранения. Проверьте права доступа к папке.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.WORKSHOP_ZIP_ERROR]: {
    title: 'Ошибка архивации мира',
    description: 'Не удалось упаковать или распаковать резервную копию мира. Убедитесь, что на диске достаточно места.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  [ERROR_CODES.WORKSHOP_INVALID_LEVEL_DAT]: {
    title: 'Повреждён файл сохранения',
    description: 'Файл level.dat повреждён или имеет неподдерживаемый формат. Восстановите мир из предыдущей резервной копии.',
    primaryAction: { label: 'Открыть резервные копии', type: 'folder' },
  },

  // ==========================================================================
  // Lobby & Multiplayer
  // ==========================================================================
  [ERROR_CODES.NET_LOBBY_NOT_FOUND]: {
    title: 'Комната не найдена',
    description: 'Проверьте правильность кода комнаты или создайте новую сессию.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_UNAUTHORIZED]: {
    title: 'Доступ ограничен',
    description: 'Недостаточно прав для подключения к выбранной комнате.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_NETWORK_ERROR]: {
    title: 'Сервис недоступен',
    description: 'Не удалось связаться с сервером сетевых комнат. Проверьте подключение к интернету или повторите попытку позже.',
    primaryAction: { label: 'Повторить попытку', type: 'retry' },
  },
  NET_LOBBY_KICKED: {
    title: 'Вы исключены из комнаты',
    description: 'Организатор закрыл комнату или удалил вас из текущей сессии.',
    primaryAction: { label: 'К списку комнат', type: 'close' },
  },
  NET_TUNNEL_ERROR: {
    title: 'Сбой сетевого туннеля',
    description: 'Не удалось установить прямое P2P-соединение. Проверьте брандмауэр или повторите подключение.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // Friends & Social
  // ==========================================================================
  [ERROR_CODES.SOCIAL_NOT_FOUND]: {
    title: 'Игрок не найден',
    description: 'Проверьте правильность кода друга или никнейма и попробуйте снова.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_UNAUTHORIZED]: {
    title: 'Требуется авторизация',
    description: 'Срок действия сессии профиля истёк. Перезайдите в аккаунт.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  [ERROR_CODES.SOCIAL_NETWORK_ERROR]: {
    title: 'Служба друзей недоступна',
    description: 'Не удалось получить данные социальной сети. Проверьте сетевое подключение.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  SOCIAL_ALREADY_EXISTS: {
    title: 'Заявка уже отправлена',
    description: 'Этот игрок уже в вашем списке друзей или вы ранее отправили ему приглашение.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },

  // ==========================================================================
  // File System & OS Errors
  // ==========================================================================
  [ERROR_CODES.IO_PERMISSION_DENIED]: {
    title: 'Доступ к файлам запрещён',
    description: 'Операционная система ограничила доступ к папке лаунчера. Проверьте права или запустите лаунчер от имени администратора.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.IO_NOT_FOUND]: {
    title: 'Файл или папка не найдены',
    description: 'Запрошенный файл был перемещён или удалён другой программой.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  [ERROR_CODES.IO_TIMED_OUT]: {
    title: 'Истекло время ожидания диска',
    description: 'Накопитель слишком долго не отвечает на операцию чтения или записи. Проверьте состояние диска.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  IO_NO_SPACE: {
    title: 'Недостаточно места на диске',
    description: 'На накопителе закончилась свободная память. Освободите место и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  [ERROR_CODES.TIMEOUT]: {
    title: 'Время ожидания истекло',
    description: 'Сервер или локальная операция не ответили вовремя. Повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // Launcher Updates
  // ==========================================================================
  [ERROR_CODES.UPDATE_NETWORK_ERROR]: {
    title: 'Не удалось проверить обновления',
    description: 'Сервер обновлений Wex временно недоступен. Лаунчер продолжит работу в штатном режиме.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.UPDATE_SIGNATURE_FAILED]: {
    title: 'Ошибка проверки обновления',
    description: 'Цифровая подпись пакета обновления не прошла проверку подлинности. Загрузка отменена в целях безопасности.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
};

/**
 * Fallback defaults by domain when error code is unknown or generic.
 */
const DOMAIN_FALLBACKS: Record<ErrorDomain, ErrorMappingDefinition> = {
  launch: {
    title: 'Сбой при запуске игры',
    description: 'Проверьте установленные моды и параметры Java в настройках, затем повторите запуск.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Настройки', type: 'settings' },
  },
  install: {
    title: 'Ошибка установки файлов',
    description: 'Проверьте интернет-соединение и доступность диска, затем повторите установку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  pack: {
    title: 'Сбой синхронизации пака',
    description: 'Не удалось получить актуальные файлы пака модов. Повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  java: {
    title: 'Ошибка среды Java',
    description: 'Проверьте настройки Java и убедитесь, что установлена поддерживаемая версия.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  mods: {
    title: 'Ошибка управления модами',
    description: 'Не удалось выполнить действие с файлом мода. Проверьте папку mods.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  workshop: {
    title: 'Ошибка мастерской',
    description: 'Не удалось завершить операцию с файлами мира или резервной копией.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  lobby: {
    title: 'Ошибка в лобби',
    description: 'Не удалось подключиться к комнате или обновить статус игроков.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  friends: {
    title: 'Ошибка социальной службы',
    description: 'Не удалось выполнить операцию с профилем или списком друзей.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  settings: {
    title: 'Ошибка сохранения настроек',
    description: 'Не удалось записать файл конфигурации. Проверьте права доступа к папке лаунчера.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  general: {
    title: 'Произошла ошибка',
    description: 'Операция не была завершена. Попробуйте повторить действие позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
};

/**
 * Parses raw input into a structured IpcError candidate.
 */
function extractRawError(error: unknown): {
  code?: string;
  message?: string;
  technical?: string | null;
} {
  if (!error) {
    return { message: 'Неизвестная ошибка' };
  }

  // If already an object resembling IpcError
  if (typeof error === 'object') {
    const errObj = error as Record<string, unknown>;
    // Direct code/message/technical
    if (typeof errObj.code === 'string') {
      return {
        code: errObj.code,
        message: typeof errObj.message === 'string' ? errObj.message : undefined,
        technical: typeof errObj.technical === 'string' ? errObj.technical : null,
      };
    }
    // Result object { Err: { code, message, technical } }
    if (errObj.Err && typeof errObj.Err === 'object') {
      const nested = errObj.Err as Record<string, unknown>;
      return {
        code: typeof nested.code === 'string' ? nested.code : undefined,
        message: typeof nested.message === 'string' ? nested.message : undefined,
        technical: typeof nested.technical === 'string' ? nested.technical : null,
      };
    }
    // JavaScript Error instance
    if (typeof errObj.message === 'string') {
      return parseRawString(errObj.message);
    }
  }

  if (typeof error === 'string') {
    // Try JSON parsing
    const trimmed = error.trim();
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      try {
        const parsed = JSON.parse(trimmed);
        return extractRawError(parsed);
      } catch {
        // Not valid JSON, continue with string heuristics
      }
    }
    return parseRawString(error);
  }

  return { message: String(error) };
}

/**
 * Parses formatted Rust error string e.g. "[CODE] Message (Technical details)"
 */
function parseRawString(raw: string): {
  code?: string;
  message?: string;
  technical?: string | null;
} {
  const bracketMatch = raw.match(/^\[([A-Z0-9_]+)\]\s*(.*)$/);
  if (bracketMatch) {
    const code = bracketMatch[1];
    let rest = bracketMatch[2];
    let technical: string | null = null;
    const techMatch = rest.match(/^(.*?)\s*\((.*)\)$/);
    if (techMatch) {
      rest = techMatch[1];
      technical = techMatch[2];
    }
    return { code, message: rest.trim(), technical };
  }

  return { message: raw, technical: raw };
}

/**
 * Pattern-matches raw error text to infer a known error code if not explicitly given.
 */
function inferErrorCodeFromPatterns(rawText: string, domain?: ErrorDomain): string | null {
  const lower = rawText.toLowerCase();

  // Java patterns
  if (
    lower.includes('java binary not found') ||
    lower.includes('не найден исполняемый файл java') ||
    lower.includes('javaw.exe') ||
    lower.includes('cannot find java')
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
  if (lower.includes('java runtime error') || lower.includes('jvm')) {
    return ERROR_CODES.JAVA_RUNTIME_ERROR;
  }

  // Memory patterns
  if (
    lower.includes('could not reserve enough space') ||
    lower.includes('outofmemoryerror') ||
    lower.includes('out of memory')
  ) {
    return 'LAUNCH_OUT_OF_MEMORY';
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

  // Process exit / crash
  if (
    lower.includes('process exited with code') ||
    lower.includes('exit code -1') ||
    lower.includes('exit code 1') ||
    lower.includes('аварийно завершился')
  ) {
    return ERROR_CODES.LAUNCH_PROCESS_FAILED;
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
    lower.includes('не удалось загрузить')
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
    return 'NET_LOBBY_KICKED';
  }
  if (domain === 'lobby' && (lower.includes('не найдена') || lower.includes('not found') || lower.includes('404'))) {
    return ERROR_CODES.NET_LOBBY_NOT_FOUND;
  }
  if (domain === 'lobby' && lower.includes('код комнаты')) {
    return ERROR_CODES.NET_LOBBY_NOT_FOUND;
  }
  if (lower.includes('tunnel') || lower.includes('playit')) {
    return 'NET_TUNNEL_ERROR';
  }

  // Social / Friends
  if (lower.includes('уже в вашем списке друзей') || lower.includes('already friends')) {
    return 'SOCIAL_ALREADY_EXISTS';
  }

  // Disk / IO
  if (lower.includes('access is denied') || lower.includes('permission denied') || lower.includes('доступ запрещён')) {
    return ERROR_CODES.IO_PERMISSION_DENIED;
  }
  if (lower.includes('not enough space') || lower.includes('no space left') || lower.includes('недостаточно места')) {
    return 'IO_NO_SPACE';
  }
  if (lower.includes('not found') || lower.includes('не найден')) {
    return ERROR_CODES.IO_NOT_FOUND;
  }
  if (lower.includes('timed out') || lower.includes('таймаут') || lower.includes('время ожидания')) {
    return ERROR_CODES.IO_TIMED_OUT;
  }

  return null;
}

/**
 * Primary error formatter function.
 * Transforms any backend error or exception into a clean, Apple HIG compliant FormattedError.
 *
 * @param error - Raw error object, string, or IpcError from Tauri backend
 * @param domain - Optional contextual domain (launch, mods, workshop, etc.)
 * @returns FormattedError ready for presentation in ErrorState or dialogs
 */
export function formatError(error: unknown, domain: ErrorDomain = 'general'): FormattedError {
  const extracted = extractRawError(error);
  const rawText = extracted.message || String(error);

  // Inferred or explicit code
  const inferredCode = extracted.code || inferErrorCodeFromPatterns(rawText, domain);
  const finalCode = inferredCode || ERROR_CODES.INTERNAL_ERROR;

  // Dictionary lookup
  const mapping = ERROR_DICTIONARY[finalCode];
  if (mapping) {
    return {
      code: finalCode,
      title: mapping.title,
      description: mapping.description,
      primaryAction: mapping.primaryAction,
      secondaryAction: mapping.secondaryAction,
      technical: extracted.technical || rawText,
    };
  }

  // Domain fallback if no dictionary match
  const fallback = DOMAIN_FALLBACKS[domain] || DOMAIN_FALLBACKS.general;
  return {
    code: finalCode,
    title: fallback.title,
    description: fallback.description,
    primaryAction: fallback.primaryAction,
    secondaryAction: fallback.secondaryAction,
    technical: extracted.technical || rawText,
  };
}
