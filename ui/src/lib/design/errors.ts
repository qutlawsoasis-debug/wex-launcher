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
  | 'auth'
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
 * Sanitizes technical text and strips paths containing user directories
 * so that no user-specific folder paths or unformatted raw stacks leak into player-facing text.
 */
export function sanitizeUserMessage(text: string): string {
  if (!text) return '';
  return text
    // Replace Windows user home paths: C:\Users\<username>\... -> ...
    .replace(/[a-zA-Z]:\\Users\\[^\\]+\\/gi, '')
    // Replace Unix user home paths: /home/<username>/... or /Users/<username>/... -> ...
    .replace(/\/(?:home|Users)\/[^/]+\//gi, '')
    // Strip error codes like "[IO_ERROR]"
    .replace(/\[[A-Z0-9_]+\]\s*/g, '')
    // Strip technical prefixes like "I/O error: ", "Error: "
    .replace(/^(?:I\/O error|Error|Reqwest error|IPC error):\s*/i, '')
    // Strip raw OS error codes like "(os error 5)"
    .replace(/\s*\((?:os error|errno|code)\s*\d+\)/gi, '')
    .trim();
}

/**
 * Canonical dictionary mapping known error codes to Apple HIG compliant explanations.
 */
const ERROR_DICTIONARY: Record<string, ErrorMappingDefinition> = {
  // ==========================================================================
  // 1. Internet & Network (нет интернета / сервер не отвечает)
  // ==========================================================================
  [ERROR_CODES.NETWORK_ERROR]: {
    title: 'Нет подключения к интернету',
    description: 'Проверьте сетевое соединение или Wi-Fi и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.IO_TIMED_OUT]: {
    title: 'Сервер не отвечает',
    description: 'Сервер слишком долго отвечает на запрос. Проверьте подключение к интернету или попробуйте позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.TIMEOUT]: {
    title: 'Время ожидания истекло',
    description: 'Сервер или локальная операция не ответили вовремя. Повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_PING_TIMEOUT]: {
    title: 'Сервер не отвечает',
    description: 'Сервер не ответил на опрос. Проверьте адрес сервера и состояние сети.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_DNS_ERROR]: {
    title: 'Сервер не найден',
    description: 'Не удалось найти сервер по указанному адресу. Проверьте правильность написания адреса.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_PROTOCOL_ERROR]: {
    title: 'Неподдерживаемая версия сервера',
    description: 'Сервер вернул ответ в неизвестном формате. Проверьте версию сервера и моды.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // 2. Disk & Antivirus (нет места на диске / антивирус заблокировал файл)
  // ==========================================================================
  [ERROR_CODES.IO_DISK_FULL]: {
    title: 'Недостаточно места на диске',
    description: 'На накопителе закончилась свободная память. Освободите место на диске и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  IO_NO_SPACE: {
    title: 'Недостаточно места на диске',
    description: 'На накопителе закончилась свободная память. Освободите место на диске и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
  ANTIVIRUS_BLOCKED: {
    title: 'Антивирус заблокировал файл',
    description: 'Антивирус или система ограничили доступ к файлу игры. Добавьте лаунчер в исключения антивируса и повторите запуск.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },
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
  [ERROR_CODES.IO_ALREADY_EXISTS]: {
    title: 'Файл уже существует',
    description: 'Файл или папка с таким именем уже есть на диске. Выберите другое имя.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.IO_ERROR]: {
    title: 'Ошибка работы с диском',
    description: 'Не удалось прочитать или записать файлы. Проверьте свободное место и права доступа к папке лаунчера.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Открыть папку', type: 'folder' },
  },

  // ==========================================================================
  // 3. Java Runtime (Java не запускается / Java не найдена)
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
  // 4. Game Launch & Crash (игра упала / мало памяти)
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
  [ERROR_CODES.LAUNCH_OUT_OF_MEMORY]: {
    title: 'Недостаточно оперативной памяти',
    description: 'Системе не удалось выделить запрашиваемый объём памяти для Java. Уменьшите объём ОЗУ в настройках.',
    primaryAction: { label: 'Настройки', type: 'settings' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
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

  // ==========================================================================
  // 5. Installation & Sync
  // ==========================================================================
  [ERROR_CODES.INSTALL_FAILED]: {
    title: 'Ошибка установки клиента',
    description: 'Не удалось завершить установку файлов игры. Проверьте интернет-соединение и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_DOWNLOAD_FAILED]: {
    title: 'Не удалось скачать файлы игры',
    description: 'Произошёл сбой при загрузке компонентов игры. Проверьте интернет и повторите загрузку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_HASH_MISMATCH]: {
    title: 'Файл игры повреждён',
    description: 'Контрольная сумма скачанного файла не совпадает. Повторите загрузку для автоматического исправления файла.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_SIZE_MISMATCH]: {
    title: 'Загрузка не завершена',
    description: 'Размер скачанного файла меньше ожидаемого. Повторите загрузку для скачивания целого файла.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_MANIFEST_ERROR]: {
    title: 'Ошибка списка файлов',
    description: 'Не удалось прочитать список файлов игры. Обновите список версий или попробуйте позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.PACK_SYNC_FAILED]: {
    title: 'Сбой синхронизации сборки',
    description: 'Не удалось обновить набор модов. Проверьте сеть или запустите игру без обновления.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // 6. Mods Management
  // ==========================================================================
  [ERROR_CODES.MODS_PROTECTED_PACK_MOD]: {
    title: 'Мод защищён от удаления',
    description: 'Этот мод входит в обязательный пак сборки. Вы можете временно выключить его переключателем.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.MODS_INVALID_FILE]: {
    title: 'Неподдерживаемый файл мода',
    description: 'Лаунчер поддерживает только файлы модификаций с расширением .jar.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.MODS_LOAD_FAILED]: {
    title: 'Не удалось загрузить моды',
    description: 'Произошла ошибка при чтении каталога модов. Проверьте доступность папки mods.',
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
  // 7. Workshop (Saves & Backups)
  // ==========================================================================
  [ERROR_CODES.WORKSHOP_NOT_FOUND]: {
    title: 'Файл мира не найден',
    description: 'Папка с сохранением была перемещена или удалена. Обновите список сохранений в Мастерской.',
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
  // 8. Lobby & Multiplayer
  // ==========================================================================
  [ERROR_CODES.NET_LOBBY_NOT_FOUND]: {
    title: 'Комната не найдена',
    description: 'Проверьте правильность кода комнаты или создайте новую сессию.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_UNAUTHORIZED]: {
    title: 'Доступ ограничен',
    description: 'Недостаточно прав для подключения к выбранной комнате. Попросите организатора отправить новое приглашение.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_API_ERROR]: {
    title: 'Сервер комнат недоступен',
    description: 'Не удалось связаться с сервером сетевых комнат. Проверьте интернет или повторите попытку позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_NETWORK_ERROR]: {
    title: 'Сервис недоступен',
    description: 'Не удалось связаться с сервером сетевых комнат. Проверьте подключение к интернету или повторите попытку позже.',
    primaryAction: { label: 'Повторить попытку', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_JSON_ERROR]: {
    title: 'Сбой ответа сервера комнат',
    description: 'Сервер комнат прислал некорректный ответ. Попробуйте повторить попытку позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_URL_ERROR]: {
    title: 'Неверный адрес сервера комнат',
    description: 'Указан некорректный адрес подключения. Проверьте настройки сети.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.NET_LOBBY_KICKED]: {
    title: 'Вы исключены из комнаты',
    description: 'Организатор закрыл комнату или удалил вас из текущей сессии.',
    primaryAction: { label: 'К списку комнат', type: 'close' },
  },
  [ERROR_CODES.NET_TUNNEL_ERROR]: {
    title: 'Сбой сетевого туннеля',
    description: 'Не удалось установить прямое сетевое соединение. Проверьте брандмауэр и антивирус или повторите подключение.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // 9. Friends & Social
  // ==========================================================================
  [ERROR_CODES.SOCIAL_NOT_FOUND]: {
    title: 'Игрок не найден',
    description: 'Проверьте правильность кода друга или никнейма и попробуйте снова.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_UNAUTHORIZED]: {
    title: 'Требуется авторизация',
    description: 'Срок действия сессии профиля истёк. Войдите в профиль заново в настройках.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  [ERROR_CODES.SOCIAL_API_ERROR]: {
    title: 'Служба друзей временно недоступна',
    description: 'Не удалось обновить список друзей. Попробуйте повторить попытку позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_NETWORK_ERROR]: {
    title: 'Нет подключения к интернету',
    description: 'Не удалось связаться со службой друзей. Проверьте сетевое подключение и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_JSON_ERROR]: {
    title: 'Ошибка данных службы друзей',
    description: 'Сервер друзей вернул некорректные данные. Попробуйте повторить попытку позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_ALREADY_EXISTS]: {
    title: 'Заявка уже отправлена',
    description: 'Этот игрок уже в вашем списке друзей или вы ранее отправили ему приглашение.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.ACHIEVEMENT_IO_ERROR]: {
    title: 'Ошибка сохранения достижений',
    description: 'Не удалось сохранить прогресс достижений на диск. Проверьте свободное место на диске.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.ACHIEVEMENT_SERIALIZE_ERROR]: {
    title: 'Ошибка файла достижений',
    description: 'Файл прогресса достижений повреждён. Лаунчер обновит список достижений.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },

  // ==========================================================================
  // 10. Updates & Extras
  // ==========================================================================
  [ERROR_CODES.UPDATE_NETWORK_ERROR]: {
    title: 'Не удалось проверить обновления',
    description: 'Сервер обновлений временно недоступен. Лаунчер продолжит работу в штатном режиме.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.UPDATE_INVALID_MANIFEST]: {
    title: 'Ошибка данных обновления',
    description: 'Сервер обновлений вернул повреждённые данные. Попробуйте проверить обновления позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.UPDATE_PLATFORM_NOT_FOUND]: {
    title: 'Обновление не найдено',
    description: 'Для вашей операционной системы нет подходящей версии обновления. Обратитесь к разработчикам.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.UPDATE_SIGNATURE_FAILED]: {
    title: 'Ошибка проверки обновления',
    description: 'Цифровая подпись обновления не подтверждена. Загрузка отменена в целях безопасности.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.SKIN_IMAGE_ERROR]: {
    title: 'Неверный формат изображения',
    description: 'Не удалось открыть файл скина. Выберите файл в формате PNG.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.SKIN_INVALID_PNG]: {
    title: 'Повреждён файл скина',
    description: 'Файл скина повреждён или имеет неверный формат. Выберите другой PNG-файл.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.SKIN_UNSUPPORTED_DIMENSIONS]: {
    title: 'Неподдерживаемый размер скина',
    description: 'Используйте изображение скина размером 64x64 или 64x32 пикселей.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.SKIN_FILE_TOO_LARGE]: {
    title: 'Файл скина слишком большой',
    description: 'Размер файла скина превышает 2 МБ. Выберите файл меньшего размера.',
    primaryAction: { label: 'Выбрать другой файл', type: 'custom' },
  },
  [ERROR_CODES.OPERATION_CANCELLED]: {
    title: 'Операция отменена',
    description: 'Действие было прервано. Вы можете запустить его снова в любое время.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.INTERNAL_ERROR]: {
    title: 'Произошла ошибка',
    description: 'Операция не была завершена. Попробуйте повторить действие позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.NOT_FOUND]: {
    title: 'Объект не найден',
    description: 'Запрошенный файл или ресурс не найден. Обновите список и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  [ERROR_CODES.INVALID_ARGUMENT]: {
    title: 'Неверные параметры запроса',
    description: 'Проверьте введённые данные и повторите попытку.',
    primaryAction: { label: 'Понятно', type: 'close' },
  },
  [ERROR_CODES.SERIALIZATION_ERROR]: {
    title: 'Ошибка чтения данных',
    description: 'Файл параметров повреждён. Лаунчер восстановит настройки по умолчанию.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
};

/**
 * Fallback defaults by domain when error code is unknown or generic.
 */
const DOMAIN_FALLBACKS: Record<ErrorDomain, ErrorMappingDefinition> = {
  launch: {
    title: 'Сбой при запуске игры',
    description: 'Проверьте параметры Java и моды в настройках, затем повторите запуск.',
    primaryAction: { label: 'Повторить', type: 'retry' },
    secondaryAction: { label: 'Настройки', type: 'settings' },
  },
  install: {
    title: 'Ошибка установки файлов',
    description: 'Проверьте сетевое подключение и свободное место на диске, затем повторите установку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  pack: {
    title: 'Сбой синхронизации сборки',
    description: 'Не удалось получить актуальные файлы сборки модов. Проверьте сеть и повторите попытку.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  java: {
    title: 'Среда Java не найдена',
    description: 'Проверьте настройки Java и убедитесь, что установлена подходящая версия Java.',
    primaryAction: { label: 'Настройки', type: 'settings' },
  },
  mods: {
    title: 'Ошибка работы с модами',
    description: 'Не удалось выполнить действие с файлом мода. Проверьте папку mods на диске.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  workshop: {
    title: 'Ошибка сохранения мира',
    description: 'Не удалось завершить операцию с файлами сохранения. Проверьте свободное место на диске.',
    primaryAction: { label: 'Открыть папку', type: 'folder' },
    secondaryAction: { label: 'Повторить', type: 'retry' },
  },
  lobby: {
    title: 'Ошибка в комнате',
    description: 'Не удалось подключиться к комнате. Проверьте код комнаты или создайте новую.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  friends: {
    title: 'Ошибка социальной службы',
    description: 'Не удалось выполнить операцию с профилем или списком друзей. Попробуйте повторить попытку позже.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  settings: {
    title: 'Ошибка сохранения настроек',
    description: 'Не удалось записать файл настроек на диск. Проверьте права доступа к папке лаунчера.',
    primaryAction: { label: 'Повторить', type: 'retry' },
  },
  auth: {
    title: 'Не удалось войти',
    description: 'Вход не завершён. Повторите попытку.',
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

  // Dictionary lookup only when a specific code was inferred or provided
  if (inferredCode) {
    const mapping = ERROR_DICTIONARY[inferredCode];
    if (mapping) {
      return {
        code: inferredCode,
        title: mapping.title,
        description: mapping.description,
        primaryAction: mapping.primaryAction,
        secondaryAction: mapping.secondaryAction,
        technical: extracted.technical || rawText,
      };
    }
  }

  // Account errors: core already writes the player-facing text (wex_core::accounts).
  if (inferredCode?.startsWith('AUTH_') && extracted.message) {
    return {
      code: inferredCode,
      title: DOMAIN_FALLBACKS.auth.title,
      description: extracted.message,
      primaryAction: DOMAIN_FALLBACKS.auth.primaryAction,
      technical: extracted.technical || rawText,
    };
  }

  // Domain fallback if no dictionary match or unknown code
  const finalCode = inferredCode || ERROR_CODES.INTERNAL_ERROR;
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
