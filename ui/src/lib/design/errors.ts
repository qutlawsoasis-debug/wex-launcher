/**
 * Wex Launcher — Unified Error Handling & Mapping System (Apple HIG)
 *
 * Guidelines:
 * - Apple HIG: Alerts (https://developer.apple.com/design/human-interface-guidelines/alerts)
 *   "Give an alert a clear, specific title stating what happened, with a message stating what to do."
 * - Apple HIG: Writing (https://developer.apple.com/design/human-interface-guidelines/writing)
 *   "Be concise, clear, and direct. Avoid technical jargon. Keep technical details secondary."
 *
 * Maps backend error codes (wex-ipc, core, OS) to player-facing text in the current
 * language: every title, description and action label is a dictionary key
 * (ru.ts / en.ts, "err.*"), resolved with t() when the error is formatted.
 * 1. What happened (title)
 * 2. What to do (description)
 * 3. Primary & secondary actions (retry, open folder, settings)
 * 4. Collapsible technical details for debugging/logs
 * Matching raw backend text to a code lives in errorPatterns.ts.
 */

import { ERROR_CODES } from '../ipc/generated/error';
import { t, type TranslationKey } from '../i18n';
import { inferErrorCodeFromPatterns } from './errorPatterns';

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

interface ErrorActionKey {
  label: TranslationKey;
  type: ErrorActionType;
}

/** Dictionary keys, resolved with t() in formatError so the language can change. */
interface ErrorMappingDefinition {
  title: TranslationKey;
  description: TranslationKey;
  primaryAction?: ErrorActionKey;
  secondaryAction?: ErrorActionKey;
}

const action = (a?: ErrorActionKey): ErrorAction | undefined => (a ? { label: t(a.label), type: a.type } : undefined);

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
    title: 'err.network_error.title',
    description: 'err.network_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.IO_TIMED_OUT]: {
    title: 'err.io_timed_out.title',
    description: 'err.io_timed_out.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.TIMEOUT]: {
    title: 'err.timeout.title',
    description: 'err.timeout.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_PING_TIMEOUT]: {
    title: 'err.net_ping_timeout.title',
    description: 'err.net_ping_timeout.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_DNS_ERROR]: {
    title: 'err.net_dns_error.title',
    description: 'err.net_dns_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_PROTOCOL_ERROR]: {
    title: 'err.net_protocol_error.title',
    description: 'err.net_protocol_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },

  // ==========================================================================
  // 2. Disk & Antivirus (нет места на диске / антивирус заблокировал файл)
  // ==========================================================================
  [ERROR_CODES.IO_DISK_FULL]: {
    title: 'err.io_disk_full.title',
    description: 'err.io_disk_full.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  IO_NO_SPACE: {
    title: 'err.io_no_space.title',
    description: 'err.io_no_space.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  ANTIVIRUS_BLOCKED: {
    title: 'err.antivirus_blocked.title',
    description: 'err.antivirus_blocked.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  [ERROR_CODES.IO_PERMISSION_DENIED]: {
    title: 'err.io_permission_denied.title',
    description: 'err.io_permission_denied.desc',
    primaryAction: { label: 'err.act.open_folder', type: 'folder' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.IO_NOT_FOUND]: {
    title: 'err.io_not_found.title',
    description: 'err.io_not_found.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  [ERROR_CODES.IO_ALREADY_EXISTS]: {
    title: 'err.io_already_exists.title',
    description: 'err.io_already_exists.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.IO_ERROR]: {
    title: 'err.io_error.title',
    description: 'err.io_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },

  // ==========================================================================
  // 3. Java Runtime (Java не запускается / Java не найдена)
  // ==========================================================================
  [ERROR_CODES.JAVA_NOT_FOUND]: {
    title: 'err.java_not_found.title',
    description: 'err.java_not_found.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  [ERROR_CODES.JAVA_WRONG_VERSION]: {
    title: 'err.java_wrong_version.title',
    description: 'err.java_wrong_version.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  [ERROR_CODES.JAVA_RUNTIME_ERROR]: {
    title: 'err.java_runtime_error.title',
    description: 'err.java_runtime_error.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },

  // ==========================================================================
  // 4. Game Launch & Crash (игра упала / мало памяти)
  // ==========================================================================
  [ERROR_CODES.LAUNCH_FAILED]: {
    title: 'err.launch_failed.title',
    description: 'err.launch_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  [ERROR_CODES.LAUNCH_PROCESS_FAILED]: {
    title: 'err.launch_process_failed.title',
    description: 'err.launch_process_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  [ERROR_CODES.LAUNCH_OUT_OF_MEMORY]: {
    title: 'err.launch_out_of_memory.title',
    description: 'err.launch_out_of_memory.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.LAUNCH_OFFLINE_FORBIDDEN]: {
    title: 'err.launch_offline_forbidden.title',
    description: 'err.launch_offline_forbidden.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.LAUNCH_ALREADY_RUNNING]: {
    title: 'err.launch_already_running.title',
    description: 'err.launch_already_running.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },

  // ==========================================================================
  // 5. Installation & Sync
  // ==========================================================================
  [ERROR_CODES.INSTALL_FAILED]: {
    title: 'err.install_failed.title',
    description: 'err.install_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_DOWNLOAD_FAILED]: {
    title: 'err.install_download_failed.title',
    description: 'err.install_download_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_HASH_MISMATCH]: {
    title: 'err.install_hash_mismatch.title',
    description: 'err.install_hash_mismatch.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_SIZE_MISMATCH]: {
    title: 'err.install_size_mismatch.title',
    description: 'err.install_size_mismatch.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.INSTALL_MANIFEST_ERROR]: {
    title: 'err.install_manifest_error.title',
    description: 'err.install_manifest_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.PACK_SYNC_FAILED]: {
    title: 'err.pack_sync_failed.title',
    description: 'err.pack_sync_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },

  // ==========================================================================
  // 6. Mods Management
  // ==========================================================================
  [ERROR_CODES.MODS_PROTECTED_PACK_MOD]: {
    title: 'err.mods_protected_pack_mod.title',
    description: 'err.mods_protected_pack_mod.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.MODS_INVALID_FILE]: {
    title: 'err.mods_invalid_file.title',
    description: 'err.mods_invalid_file.desc',
    primaryAction: { label: 'err.act.pick_other_file', type: 'custom' },
  },
  [ERROR_CODES.MODS_LOAD_FAILED]: {
    title: 'err.mods_load_failed.title',
    description: 'err.mods_load_failed.desc',
    primaryAction: { label: 'err.act.open_folder', type: 'folder' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.MODS_DELETE_FAILED]: {
    title: 'err.mods_delete_failed.title',
    description: 'err.mods_delete_failed.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },

  // ==========================================================================
  // 7. Workshop (Saves & Backups)
  // ==========================================================================
  [ERROR_CODES.WORKSHOP_NOT_FOUND]: {
    title: 'err.workshop_not_found.title',
    description: 'err.workshop_not_found.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  [ERROR_CODES.WORKSHOP_IO_ERROR]: {
    title: 'err.workshop_io_error.title',
    description: 'err.workshop_io_error.desc',
    primaryAction: { label: 'err.act.open_folder', type: 'folder' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.WORKSHOP_ZIP_ERROR]: {
    title: 'err.workshop_zip_error.title',
    description: 'err.workshop_zip_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.open_folder', type: 'folder' },
  },
  [ERROR_CODES.WORKSHOP_INVALID_LEVEL_DAT]: {
    title: 'err.workshop_invalid_level_dat.title',
    description: 'err.workshop_invalid_level_dat.desc',
    primaryAction: { label: 'err.act.open_backups', type: 'folder' },
  },

  // ==========================================================================
  // 8. Lobby & Multiplayer
  // ==========================================================================
  [ERROR_CODES.NET_LOBBY_NOT_FOUND]: {
    title: 'err.net_lobby_not_found.title',
    description: 'err.net_lobby_not_found.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_UNAUTHORIZED]: {
    title: 'err.net_lobby_unauthorized.title',
    description: 'err.net_lobby_unauthorized.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_API_ERROR]: {
    title: 'err.net_lobby_api_error.title',
    description: 'err.net_lobby_api_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_NETWORK_ERROR]: {
    title: 'err.net_lobby_network_error.title',
    description: 'err.net_lobby_network_error.desc',
    primaryAction: { label: 'err.act.try_again', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_JSON_ERROR]: {
    title: 'err.net_lobby_json_error.title',
    description: 'err.net_lobby_json_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NET_LOBBY_URL_ERROR]: {
    title: 'err.net_lobby_url_error.title',
    description: 'err.net_lobby_url_error.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.NET_LOBBY_KICKED]: {
    title: 'err.net_lobby_kicked.title',
    description: 'err.net_lobby_kicked.desc',
    primaryAction: { label: 'err.act.to_rooms', type: 'close' },
  },
  [ERROR_CODES.NET_TUNNEL_ERROR]: {
    title: 'err.net_tunnel_error.title',
    description: 'err.net_tunnel_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },

  // ==========================================================================
  // 9. Friends & Social
  // ==========================================================================
  [ERROR_CODES.SOCIAL_NOT_FOUND]: {
    title: 'err.social_not_found.title',
    description: 'err.social_not_found.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_UNAUTHORIZED]: {
    title: 'err.social_unauthorized.title',
    description: 'err.social_unauthorized.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  [ERROR_CODES.SOCIAL_API_ERROR]: {
    title: 'err.social_api_error.title',
    description: 'err.social_api_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_NETWORK_ERROR]: {
    title: 'err.social_network_error.title',
    description: 'err.social_network_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_JSON_ERROR]: {
    title: 'err.social_json_error.title',
    description: 'err.social_json_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.SOCIAL_ALREADY_EXISTS]: {
    title: 'err.social_already_exists.title',
    description: 'err.social_already_exists.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.ACHIEVEMENT_IO_ERROR]: {
    title: 'err.achievement_io_error.title',
    description: 'err.achievement_io_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.ACHIEVEMENT_SERIALIZE_ERROR]: {
    title: 'err.achievement_serialize_error.title',
    description: 'err.achievement_serialize_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },

  // ==========================================================================
  // 10. Updates & Extras
  // ==========================================================================
  [ERROR_CODES.UPDATE_NETWORK_ERROR]: {
    title: 'err.update_network_error.title',
    description: 'err.update_network_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.UPDATE_INVALID_MANIFEST]: {
    title: 'err.update_invalid_manifest.title',
    description: 'err.update_invalid_manifest.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.UPDATE_PLATFORM_NOT_FOUND]: {
    title: 'err.update_platform_not_found.title',
    description: 'err.update_platform_not_found.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.UPDATE_SIGNATURE_FAILED]: {
    title: 'err.update_signature_failed.title',
    description: 'err.update_signature_failed.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.SKIN_IMAGE_ERROR]: {
    title: 'err.skin_image_error.title',
    description: 'err.skin_image_error.desc',
    primaryAction: { label: 'err.act.pick_other_file', type: 'custom' },
  },
  [ERROR_CODES.SKIN_INVALID_PNG]: {
    title: 'err.skin_invalid_png.title',
    description: 'err.skin_invalid_png.desc',
    primaryAction: { label: 'err.act.pick_other_file', type: 'custom' },
  },
  [ERROR_CODES.SKIN_UNSUPPORTED_DIMENSIONS]: {
    title: 'err.skin_unsupported_dimensions.title',
    description: 'err.skin_unsupported_dimensions.desc',
    primaryAction: { label: 'err.act.pick_other_file', type: 'custom' },
  },
  [ERROR_CODES.SKIN_FILE_TOO_LARGE]: {
    title: 'err.skin_file_too_large.title',
    description: 'err.skin_file_too_large.desc',
    primaryAction: { label: 'err.act.pick_other_file', type: 'custom' },
  },
  [ERROR_CODES.OPERATION_CANCELLED]: {
    title: 'err.operation_cancelled.title',
    description: 'err.operation_cancelled.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.INTERNAL_ERROR]: {
    title: 'err.internal_error.title',
    description: 'err.internal_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.NOT_FOUND]: {
    title: 'err.not_found.title',
    description: 'err.not_found.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  [ERROR_CODES.INVALID_ARGUMENT]: {
    title: 'err.invalid_argument.title',
    description: 'err.invalid_argument.desc',
    primaryAction: { label: 'err.act.ok', type: 'close' },
  },
  [ERROR_CODES.SERIALIZATION_ERROR]: {
    title: 'err.serialization_error.title',
    description: 'err.serialization_error.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
};

/**
 * Fallback defaults by domain when error code is unknown or generic.
 */
const DOMAIN_FALLBACKS: Record<ErrorDomain, ErrorMappingDefinition> = {
  launch: {
    title: 'err.domain.launch.title',
    description: 'err.domain.launch.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
    secondaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  install: {
    title: 'err.domain.install.title',
    description: 'err.domain.install.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  pack: {
    title: 'err.domain.pack.title',
    description: 'err.domain.pack.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  java: {
    title: 'err.domain.java.title',
    description: 'err.domain.java.desc',
    primaryAction: { label: 'err.act.settings', type: 'settings' },
  },
  mods: {
    title: 'err.domain.mods.title',
    description: 'err.domain.mods.desc',
    primaryAction: { label: 'err.act.open_folder', type: 'folder' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  workshop: {
    title: 'err.domain.workshop.title',
    description: 'err.domain.workshop.desc',
    primaryAction: { label: 'err.act.open_folder', type: 'folder' },
    secondaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  lobby: {
    title: 'err.domain.lobby.title',
    description: 'err.domain.lobby.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  friends: {
    title: 'err.domain.friends.title',
    description: 'err.domain.friends.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  settings: {
    title: 'err.domain.settings.title',
    description: 'err.domain.settings.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  auth: {
    title: 'err.domain.auth.title',
    description: 'err.domain.auth.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
  },
  general: {
    title: 'err.domain.general.title',
    description: 'err.domain.general.desc',
    primaryAction: { label: 'err.act.retry', type: 'retry' },
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
    return { message: 'Unknown error' };
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
        title: t(mapping.title),
        description: t(mapping.description),
        primaryAction: action(mapping.primaryAction),
        secondaryAction: action(mapping.secondaryAction),
        technical: extracted.technical || rawText,
      };
    }
  }

  // Account errors: core already writes the player-facing text (wex_core::accounts).
  if (inferredCode?.startsWith('AUTH_') && extracted.message) {
    return {
      code: inferredCode,
      title: t(DOMAIN_FALLBACKS.auth.title),
      description: extracted.message,
      primaryAction: action(DOMAIN_FALLBACKS.auth.primaryAction),
      technical: extracted.technical || rawText,
    };
  }

  // Domain fallback if no dictionary match or unknown code
  const finalCode = inferredCode || ERROR_CODES.INTERNAL_ERROR;
  const fallback = DOMAIN_FALLBACKS[domain] || DOMAIN_FALLBACKS.general;
  return {
    code: finalCode,
    title: t(fallback.title),
    description: t(fallback.description),
    primaryAction: action(fallback.primaryAction),
    secondaryAction: action(fallback.secondaryAction),
    technical: extracted.technical || rawText,
  };
}

/** The backend error code of an error ("[CODE] message" strings, IpcError objects), if any. */
export function errorCodeOf(error: unknown): string | undefined {
  return extractRawError(error).code;
}
