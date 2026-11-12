/**
 * Wex Launcher — i18n Guard & Enforcement Engine (Apple HIG)
 *
 * Rules:
 * - Detects hardcoded Russian and English natural language text in Svelte files
 * - Ensures all user-facing strings are dispatched through dictionaries (`t('key')` / `ru.ts` / `en.ts`)
 * - Maintains an explicit allowlist of exceptions with mandatory documented reasons
 * - Fail-closed: any unexcepted natural language triggers an automated test failure
 */

import * as fs from 'node:fs';
import * as path from 'node:path';

export interface GuardException {
  /** File path relative to `ui/src`, or '*' for universal tokens */
  file: string;
  /** Specific substring or token allowed */
  text?: string;
  /** Regular expression allowed */
  pattern?: RegExp;
  /** Mandatory documented reason why this exception is permitted outside dictionaries */
  reason: string;
}

export interface GuardViolation {
  file: string;
  line: number;
  column: number;
  text: string;
  type: 'cyrillic_hardcoded' | 'english_text_node' | 'english_attribute' | 'english_inline';
  context: string;
}

/**
 * Explicit allowlist of permissible exceptions with required architectural reasons.
 */
export const ALLOWED_EXCEPTIONS: GuardException[] = [
  // 2. Universal Proper Names and Brand Terms (invariant across all languages)
  {
    file: '*',
    text: 'Wex',
    reason: 'Universal product brand name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Minecraft',
    reason: 'Universal game brand name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Discord Rich Presence',
    reason: 'Universal third-party integration protocol and product brand feature name',
  },
  {
    file: '*',
    text: 'Русский',
    reason: 'Universal autonym / endonym for the Russian language option in language selector',
  },
  {
    file: '*',
    text: 'English',
    reason: 'Universal autonym / endonym for the English language option in language selector',
  },
  {
    file: '*',
    text: 'Fabric',
    reason: 'Universal mod loader platform name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Forge',
    reason: 'Universal mod loader platform name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'NeoForge',
    reason: 'Universal mod loader platform name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Quilt',
    reason: 'Universal mod loader platform name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Java',
    reason: 'Universal runtime platform name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'OpenJDK',
    reason: 'Universal runtime name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'HotSpot',
    reason: 'Universal JVM implementation name that remains identical in all languages',
  },
  {
    file: '*',
    text: 'Microsoft',
    reason: 'Universal platform and account provider name',
  },
  {
    file: '*',
    text: 'Playit',
    reason: 'Universal network tunneling provider name',
  },
  {
    file: '*',
    text: 'Iris',
    reason: 'Universal shader mod name',
  },
  {
    file: '*',
    text: 'Sodium',
    reason: 'Universal rendering mod name',
  },
  {
    file: '*',
    text: 'Steve',
    reason: 'Standard Minecraft character model name',
  },
  {
    file: '*',
    text: 'Alex',
    reason: 'Standard Minecraft character model name',
  },

  // 3. Technical File Extensions, Formats, and Architecture Identifiers
  {
    file: '*',
    text: 'PNG',
    reason: 'Standard image file format abbreviation',
  },
  {
    file: '*',
    text: 'ZIP',
    reason: 'Standard archive file format abbreviation',
  },
  {
    file: '*',
    text: 'JAR',
    reason: 'Standard Java archive file format abbreviation',
  },
  {
    file: '*',
    text: 'RAM',
    reason: 'Standard hardware memory abbreviation',
  },
  {
    file: '*',
    text: 'LAN',
    reason: 'Standard network protocol and domain abbreviation',
  },
  {
    file: '*',
    text: 'FPS',
    reason: 'Standard frame rate technical metric',
  },
  {
    file: '*',
    text: 'PID',
    reason: 'Standard process identifier abbreviation',
  },
  {
    file: '*',
    text: 'UUID',
    reason: 'Standard universally unique identifier abbreviation',
  },
  {
    file: '*',
    text: 'MOTD',
    reason: 'Standard Minecraft server message of the day protocol term',
  },
  {
    file: '*',
    text: 'URL',
    reason: 'Standard uniform resource locator abbreviation',
  },
  {
    file: '*',
    text: 'GL',
    reason: 'Standard OpenGL abbreviation',
  },
  {
    file: '*',
    text: 'P2P',
    reason: 'Standard peer-to-peer networking abbreviation',
  },
  {
    file: '*',
    text: 'OFF',
    reason: 'Standard toggle state label (e.g. Shaders OFF)',
  },
  {
    file: '*',
    text: '3D',
    reason: 'Universal three-dimensional abbreviation',
  },
  {
    file: '*',
    text: '2D',
    reason: 'Universal two-dimensional abbreviation',
  },

  // 4. Units & Punctuation
  {
    file: '*',
    text: 'MB/s',
    reason: 'Standard international transfer speed unit',
  },
  {
    file: '*',
    text: 'GB',
    reason: 'Standard memory and storage unit',
  },
  {
    file: '*',
    text: 'MB',
    reason: 'Standard memory and storage unit',
  },
  {
    file: '*',
    text: 'KB',
    reason: 'Standard storage unit',
  },
  {
    file: '*',
    text: 'ms',
    reason: 'Standard latency millisecond unit',
  },
  {
    file: 'WorkshopScreen.svelte',
    text: 'C:\\Downloads\\Pack.zip',
    reason: 'Local file system path pattern illustration used in input field placeholder',
  },
  {
    file: 'Logo.svelte',
    text: 'ex',
    reason: 'Stylized brand logo accent letters "ex" in "Wex"',
  },
  {
    file: 'ReportSheet.svelte',
    text: 'latest.log',
    reason: 'Standard Minecraft log file name listed in diagnostic export contents',
  },
  {
    file: 'ReportSheet.svelte',
    text: 'crash-report',
    reason: 'Standard Minecraft crash log folder and identifier',
  },
  {
    file: 'ReportSheet.svelte',
    text: 'mods.txt',
    reason: 'Diagnostic exported mod list file name',
  },
  {
    file: 'ReportSheet.svelte',
    text: 'system-info.txt',
    reason: 'Diagnostic exported system info file name',
  },
  {
    file: 'ReportSheet.svelte',
    text: 'C:\\Users\\***',
    reason: 'Path anonymization mask illustration shown in privacy badge explanation',
  },
  {
    file: 'App.svelte',
    text: 'Игра догружается…',
    reason: 'Backend launch status message match emitted from Rust commands/launch.rs for prewarming state',
  },
  {
    file: 'ServersScreen.svelte',
    pattern: /нет интернета|неверный адрес/,
    reason: 'Error message substring matching in network fault diagnosis',
  },
  {
    file: 'LobbyScreen.svelte',
    pattern: /закрыта|исключены/,
    reason: 'Heuristic keyword substring matching for error messages returned from remote lobby server',
  },
  {
    file: 'SettingsScreen.svelte',
    pattern: /манифест|заблокирован/,
    reason: 'Heuristic keyword substring matching for storage integrity check and cache cleanup error inspection',
  },
  {
    file: '*',
    text: '•',
    reason: 'Standard typography divider dot',
  },
  {
    file: '*',
    text: '—',
    reason: 'Standard typography em-dash',
  },
  {
    file: '*',
    text: '…',
    reason: 'Standard typography ellipsis',
  },
  {
    file: '*',
    text: '×',
    reason: 'Standard multiplication / resolution symbol (e.g. 64×64)',
  },
];

/** Attributes that present user-facing text and must be localized */
const USER_FACING_ATTRIBUTES = [
  'title',
  'aria-label',
  'ariaLabel',
  'placeholder',
  'header',
  'footer',
  'label',
  'subtitle',
  'description',
  'badge',
  'doneLabel',
  'cancelLabel',
  'actionText',
  'secondaryActionText',
];

/** Regular expression matching Cyrillic characters */
const CYRILLIC_REGEX = /[\u0400-\u04FF]/;

/**
 * Checks whether an exception matches a given token in a file.
 */
function isException(
  relFile: string,
  token: string,
  exceptions: GuardException[]
): boolean {
  const normRel = relFile.replace(/\\/g, '/');
  const cleanToken = token.trim();

  for (const ex of exceptions) {
    if (ex.file !== '*') {
      const exFile = ex.file.replace(/\\/g, '/');
      if (!normRel.endsWith(exFile) && !exFile.endsWith(normRel)) {
        continue;
      }
    }
    if (ex.text && (cleanToken === ex.text || cleanToken.toLowerCase() === ex.text.toLowerCase())) {
      return true;
    }
    if (ex.pattern && ex.pattern.test(cleanToken)) {
      return true;
    }
  }

  // Pure punctuation, digits, or technical formatting tokens (e.g. "#A7X9", "1.21.1")
  if (/^[\d\s.,:;!?#@$%\^&*()\-_+=\[\]{}<>\/\\|~`'"]+$/.test(cleanToken)) {
    return true;
  }

  // Version numbers like 1.21.1 or 2.4
  if (/^v?\d+(\.\d+)*(-[\w.]+)?$/.test(cleanToken)) {
    return true;
  }

  return false;
}

/**
 * Strips `<style>` blocks from Svelte content while preserving line breaks.
 */
function stripStyleBlocks(content: string): string {
  return content.replace(/<style\b[^>]*>([\s\S]*?)<\/style>/gi, (match) => {
    return match.replace(/[^\n]/g, ' ');
  });
}

/**
 * Strips HTML comments `<!-- ... -->` from Svelte content while preserving line breaks.
 */
function stripHtmlComments(content: string): string {
  return content.replace(/<!--([\s\S]*?)-->/g, (match) => {
    return match.replace(/[^\n]/g, ' ');
  });
}

/**
 * Strips JS block comments `/* ... * /` from code while preserving line breaks.
 */
function stripJsComments(content: string): string {
  return content.replace(/\/\*([\s\S]*?)\*\//g, (match) => {
    return match.replace(/[^\n]/g, ' ');
  });
}

/**
 * Strips balanced mustache `{ ... }` from text, returning raw text outside expressions.
 */
function stripMustacheExpressions(text: string): string {
  let result = '';
  let depth = 0;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '{') {
      depth++;
    } else if (ch === '}') {
      if (depth > 0) depth--;
    } else if (depth === 0) {
      result += ch;
    }
  }
  return result;
}

/**
 * Parses Svelte template into tags and text nodes, correctly handling
 * expressions `{ (e) => ... }` and quotes inside tags so that `>` in arrow
 * functions or expressions does not prematurely end the tag.
 */
interface ParsedTag {
  tagName: string;
  attrsString: string;
  startOffset: number;
  endOffset: number;
}

interface ParsedTextNode {
  text: string;
  startOffset: number;
  endOffset: number;
}

function parseSvelteTemplate(template: string): {
  tags: ParsedTag[];
  textNodes: ParsedTextNode[];
} {
  const tags: ParsedTag[] = [];
  const textNodes: ParsedTextNode[] = [];

  let i = 0;
  const len = template.length;
  let textStart = 0;

  while (i < len) {
    if (template[i] === '<') {
      // Possible tag start
      // Save any preceding text node
      if (i > textStart) {
        textNodes.push({
          text: template.slice(textStart, i),
          startOffset: textStart,
          endOffset: i,
        });
      }

      const tagStart = i;
      i++; // skip '<'

      // Check if closing tag </tag> or comment <!-- (already stripped)
      let inQuotes: string | null = null;
      let braceDepth = 0;

      while (i < len) {
        const ch = template[i];

        if (inQuotes) {
          if (ch === '\\') {
            i += 2; // skip escape
            continue;
          }
          if (ch === inQuotes) {
            inQuotes = null;
          }
        } else if (ch === '"' || ch === "'") {
          inQuotes = ch;
        } else if (ch === '{') {
          braceDepth++;
        } else if (ch === '}') {
          if (braceDepth > 0) braceDepth--;
        } else if (ch === '>' && braceDepth === 0 && !inQuotes) {
          // Found tag end!
          const fullTag = template.slice(tagStart, i + 1);
          const match = /^<\/?([a-zA-Z0-9.:_-]+)([\s\S]*?)>$/.exec(fullTag);
          if (match) {
            tags.push({
              tagName: match[1],
              attrsString: match[2],
              startOffset: tagStart,
              endOffset: i + 1,
            });
          }
          i++; // skip '>'
          textStart = i;
          break;
        }
        i++;
      }
    } else {
      i++;
    }
  }

  // Trailing text node
  if (textStart < len) {
    textNodes.push({
      text: template.slice(textStart, len),
      startOffset: textStart,
      endOffset: len,
    });
  }

  return { tags, textNodes };
}

/**
 * Scans a single Svelte file's content for i18n violations.
 */
export function scanSvelteContent(
  relFilePath: string,
  content: string,
  exceptions: GuardException[] = ALLOWED_EXCEPTIONS
): GuardViolation[] {
  const normRel = relFilePath.replace(/\\/g, '/');

  // Check if entire file is excepted
  for (const ex of exceptions) {
    if (ex.file !== '*') {
      const exFile = ex.file.replace(/\\/g, '/');
      if ((normRel.endsWith(exFile) || exFile.endsWith(normRel)) && !ex.text && !ex.pattern) {
        return [];
      }
    }
  }

  const violations: GuardViolation[] = [];
  const lines = content.split('\n');

  // 1. Strip styles and HTML comments
  let cleaned = stripStyleBlocks(content);
  cleaned = stripHtmlComments(cleaned);

  // 2. Separate <script> blocks and scan them
  const scriptRegex = /<script\b[^>]*>([\s\S]*?)<\/script>/gi;
  let scriptMatch: RegExpExecArray | null;

  while ((scriptMatch = scriptRegex.exec(cleaned)) !== null) {
    const scriptBody = scriptMatch[1];
    const scriptStartPos = scriptMatch.index + scriptMatch[0].indexOf(scriptBody);
    const beforeScript = cleaned.slice(0, scriptStartPos);
    const startLine = beforeScript.split('\n').length;

    // Remove JS block comments
    const noComments = stripJsComments(scriptBody);
    const scriptLines = noComments.split('\n');

    for (let l = 0; l < scriptLines.length; l++) {
      let lineText = scriptLines[l];
      // Strip single-line comments
      const commentIdx = lineText.indexOf('//');
      if (commentIdx !== -1) {
        lineText = lineText.slice(0, commentIdx);
      }

      // Check for hardcoded Cyrillic string literals in script
      if (CYRILLIC_REGEX.test(lineText)) {
        // Extract string literals and regex literals
        const strRegex = /(["'`])((?:\\.|(?!\1)[^\\])*)\1|\/((?:\\.|[^\/\\])+)\/[gimsuy]*/g;
        let match: RegExpExecArray | null;
        let foundCyrillicToken = false;

        while ((match = strRegex.exec(lineText)) !== null) {
          const literalVal = match[2] !== undefined ? match[2] : match[3];
          if (literalVal && CYRILLIC_REGEX.test(literalVal)) {
            foundCyrillicToken = true;
            if (!isException(relFilePath, literalVal, exceptions)) {
              violations.push({
                file: relFilePath,
                line: startLine + l,
                column: match.index + 1,
                text: literalVal,
                type: 'cyrillic_hardcoded',
                context: lineText.trim(),
              });
            }
          }
        }

        // If not caught by quotes or regexes but line contains Cyrillic (e.g. object property or template)
        if (!foundCyrillicToken && !isException(relFilePath, lineText, exceptions)) {
          violations.push({
            file: relFilePath,
            line: startLine + l,
            column: 1,
            text: lineText.trim(),
            type: 'cyrillic_hardcoded',
            context: lineText.trim(),
          });
        }
      }
    }
  }

  // 3. Scan template using the syntax-aware template parser
  const templateOnly = cleaned.replace(/<script\b[^>]*>([\s\S]*?)<\/script>/gi, (m) =>
    m.replace(/[^\n]/g, ' ')
  );

  const { tags, textNodes } = parseSvelteTemplate(templateOnly);

  // 3A. Scan attributes on tags: <TagName ... attr="literal" ... >
  for (const tag of tags) {
    for (const attr of USER_FACING_ATTRIBUTES) {
      // Matches attr="Value" or attr='Value', but NOT attr={...} and avoids prefix overlap like aria-label / label
      const attrPattern = new RegExp(`(?:^|\\s)${attr}\\s*=\\s*(["'])([^"'{]+?)\\1`, 'g');
      let attrMatch: RegExpExecArray | null;

      while ((attrMatch = attrPattern.exec(tag.attrsString)) !== null) {
        const val = attrMatch[2].trim();
        const attrOffset = tag.startOffset + tag.attrsString.indexOf(attrMatch[0]);
        const line = templateOnly.slice(0, attrOffset).split('\n').length;
        const column = attrOffset - templateOnly.slice(0, attrOffset).lastIndexOf('\n');

        const hasCyrillic = CYRILLIC_REGEX.test(val);
        const hasEnglish = /[a-zA-Z]{2,}/.test(val);

        if (hasCyrillic || hasEnglish) {
          if (!isException(relFilePath, val, exceptions)) {
            violations.push({
              file: relFilePath,
              line,
              column,
              text: `${attr}="${val}"`,
              type: hasCyrillic ? 'cyrillic_hardcoded' : 'english_attribute',
              context: `<${tag.tagName} ... ${attr}="${val}">`,
            });
          }
        }
      }
    }
  }

  // 3B. Scan text nodes between tags
  for (const node of textNodes) {
    // Strip mustache expressions: {...}
    const pureText = stripMustacheExpressions(node.text).trim();
    if (!pureText) continue;

    const line = templateOnly.slice(0, node.startOffset).split('\n').length;
    const column = node.startOffset - templateOnly.slice(0, node.startOffset).lastIndexOf('\n');

    // Check for Cyrillic in text node
    if (CYRILLIC_REGEX.test(pureText)) {
      if (!isException(relFilePath, pureText, exceptions)) {
        violations.push({
          file: relFilePath,
          line,
          column,
          text: pureText,
          type: 'cyrillic_hardcoded',
          context: pureText,
        });
        continue;
      }
    }

    // Check for English words in text node
    if (/[a-zA-Z]{2,}/.test(pureText)) {
      const words = pureText.split(/\s+/).filter((w) => /[a-zA-Z]{2,}/.test(w));
      const unexcepted = words.filter((w) => !isException(relFilePath, w, exceptions));

      if (unexcepted.length > 0 && !isException(relFilePath, pureText, exceptions)) {
        violations.push({
          file: relFilePath,
          line,
          column,
          text: pureText,
          type: 'english_text_node',
          context: pureText,
        });
      }
    }
  }

  // 3C. Check inline ternaries for hardcoded English strings: e.g. ? 'English' : 'Русский'
  const inlineTernaryRegex = /\?\s*(["'])([A-Z][a-zA-Z0-9\s.,!?'-]{2,})\1\s*:\s*(["'])([^"']+)\3/g;
  let ternaryMatch: RegExpExecArray | null;

  while ((ternaryMatch = inlineTernaryRegex.exec(templateOnly)) !== null) {
    const branch1 = ternaryMatch[2].trim();
    const branch2 = ternaryMatch[4].trim();
    const offset = ternaryMatch.index;
    const line = templateOnly.slice(0, offset).split('\n').length;
    const column = offset - templateOnly.slice(0, offset).lastIndexOf('\n');

    if (!isException(relFilePath, branch1, exceptions) || !isException(relFilePath, branch2, exceptions)) {
      violations.push({
        file: relFilePath,
        line,
        column,
        text: `? '${branch1}' : '${branch2}'`,
        type: 'english_inline',
        context: ternaryMatch[0],
      });
    }
  }

  return violations;
}

/**
 * Recursively scans all `.svelte` files under a given directory.
 */
export function scanAllSvelteFiles(
  rootDir: string,
  exceptions: GuardException[] = ALLOWED_EXCEPTIONS
): GuardViolation[] {
  const violations: GuardViolation[] = [];

  function walk(dir: string) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.isFile() && entry.name.endsWith('.svelte')) {
        const relPath = path.relative(rootDir, fullPath);
        const content = fs.readFileSync(fullPath, 'utf8');
        const fileViolations = scanSvelteContent(relPath, content, exceptions);
        violations.push(...fileViolations);
      }
    }
  }

  walk(rootDir);
  return violations;
}
