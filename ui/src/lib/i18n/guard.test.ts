import { describe, it, expect } from 'vitest';
import * as path from 'node:path';
import {
  scanSvelteContent,
  scanAllSvelteFiles,
  ALLOWED_EXCEPTIONS,
  type GuardException,
} from './guard';

describe('i18n Guard & Enforcement Engine (Apple HIG)', () => {
  describe('Rule Enforcement Unit Tests', () => {
    it('fails when hardcoded Russian text is present in text nodes', () => {
      const svelteSample = `
        <script lang="ts">
          import Button from './Button.svelte';
        </script>

        <div>
          <span>Запустить игру</span>
        </div>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(violations.some((v) => v.type === 'cyrillic_hardcoded')).toBe(true);
      expect(violations[0].text).toContain('Запустить игру');
    });

    it('fails when hardcoded Russian text is present in user-facing attributes', () => {
      const svelteSample = `
        <script lang="ts"></script>
        <button title="Закрыть окно">
          <Icon name="close" />
        </button>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(violations.some((v) => v.type === 'cyrillic_hardcoded')).toBe(true);
    });

    it('fails when hardcoded Russian string literal is present in <script>', () => {
      const svelteSample = `
        <script lang="ts">
          let status = $state('Клиент готов к запуску');
        </script>
        <div>{status}</div>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(violations.some((v) => v.type === 'cyrillic_hardcoded')).toBe(true);
      expect(violations[0].text).toBe('Клиент готов к запуску');
    });

    it('fails when hardcoded English text is present in text nodes', () => {
      const svelteSample = `
        <script lang="ts"></script>
        <div>
          <h1>Welcome to Launcher</h1>
        </div>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(violations.some((v) => v.type === 'english_text_node')).toBe(true);
      expect(violations[0].text).toContain('Welcome to Launcher');
    });

    it('fails when hardcoded English text is present in user-facing attributes', () => {
      const svelteSample = `
        <script lang="ts"></script>
        <input placeholder="Search installed modifications..." />
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(violations.some((v) => v.type === 'english_attribute')).toBe(true);
      expect(violations[0].text).toContain('placeholder="Search installed modifications..."');
    });

    it('fails when hardcoded English/Russian inline ternary is present', () => {
      const svelteSample = `
        <script lang="ts"></script>
        <span>{isSlim ? 'Slim' : 'Тонкая'}</span>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations.length).toBeGreaterThan(0);
      expect(
        violations.some((v) => v.type === 'cyrillic_hardcoded' || v.type === 'english_inline')
      ).toBe(true);
    });

    it('passes cleanly when user-facing text is dispatched through t(...)', () => {
      const svelteSample = `
        <script lang="ts">
          import { t } from '../i18n';
          let status = $derived(t('play.client_ready'));
        </script>

        <div>
          <span>{t('play.launch')}</span>
          <button title={t('common.close')} aria-label={t('common.close')}>
            {status}
          </button>
        </div>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations).toEqual([]);
    });

    it('passes when universal brand name or technical exception is used', () => {
      const svelteSample = `
        <script lang="ts"></script>
        <div>
          <span class="brand">Wex</span>
          <span class="game">Minecraft</span>
          <span class="loader">Fabric</span>
        </div>
      `;

      const violations = scanSvelteContent('TestComponent.svelte', svelteSample);
      expect(violations).toEqual([]);
    });
  });

  describe('Exception List Audit', () => {
    it('verifies that every entry in ALLOWED_EXCEPTIONS has a documented reason', () => {
      expect(ALLOWED_EXCEPTIONS.length).toBeGreaterThan(0);

      for (const ex of ALLOWED_EXCEPTIONS) {
        expect(ex.reason, `Exception for "${ex.file}" / "${ex.text}" must have a reason`).toBeDefined();
        expect(typeof ex.reason).toBe('string');
        expect(
          ex.reason.trim().length,
          `Exception reason for "${ex.text || ex.file}" must be descriptive (> 15 chars)`
        ).toBeGreaterThan(15);
      }
    });
  });

  describe('Repository-Wide Svelte Verification', () => {
    it('verifies all ui/src/**/*.svelte files adhere to i18n dictionary rules', () => {
      const srcDir = path.resolve(__dirname, '../../');
      const violations = scanAllSvelteFiles(srcDir, ALLOWED_EXCEPTIONS);

      if (violations.length > 0) {
        const report = violations
          .slice(0, 30)
          .map((v) => `  [${v.type}] ${v.file}:${v.line}:${v.column} -> "${v.text}" (Context: ${v.context})`)
          .join('\n');

        const extra = violations.length > 30 ? `\n  ... and ${violations.length - 30} more violations` : '';

        expect.fail(
          `Found ${violations.length} unexcepted hardcoded strings in .svelte files outside dictionaries:\n${report}${extra}\n\nPlease dispatch these strings through ui/src/lib/i18n/locales/ru.ts and en.ts using t('...'), or document valid exceptions in ALLOWED_EXCEPTIONS with reasons.`
        );
      }

      expect(violations).toEqual([]);
    });
  });
});
