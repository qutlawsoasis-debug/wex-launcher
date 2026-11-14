import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const css = readFileSync(resolve(process.cwd(), 'src/lib/design/winter/winter.css'), 'utf8');
const code = css.replace(/\/\*[\s\S]*?\*\//g, '');

/** Every selector list in the file (at-rule headers left out). */
function selectors(source: string): string[] {
  const out: string[] = [];
  for (const m of source.matchAll(/([^{}]+)\{/g)) {
    const head = m[1].trim();
    if (!head || head.startsWith('@')) continue;
    for (const s of head.split(',')) out.push(s.trim());
  }
  return out;
}

describe('winter.css', () => {
  const all = selectors(code);

  it('has rules', () => {
    expect(all.length).toBeGreaterThan(10);
  });

  it('scopes every rule under html.winter', () => {
    const loose = all.filter((s) => !/^html\.winter(?![\w-])/.test(s));
    expect(loose).toEqual([]);
  });

  it('has a Reduce Transparency branch, app setting and system setting, with solid surfaces', () => {
    expect(all.some((s) => s.startsWith("html.winter[data-reduce-transparency='true']"))).toBe(true);
    const media = code.indexOf('@media (prefers-reduced-transparency: reduce)');
    expect(media).toBeGreaterThan(-1);
    const rt = code.slice(code.indexOf("html.winter[data-reduce-transparency='true']"));
    expect(rt).toMatch(/backdrop-filter:\s*none/);
  });

  it('keeps the snow caps decorative: no clicks, inline data: image only', () => {
    expect(code).toMatch(/::before[\s\S]*?pointer-events:\s*none/);
    const urls = [...code.matchAll(/url\(\s*["']?([^"')]+)/g)].map((m) => m[1]);
    expect(urls.length).toBeGreaterThan(0);
    for (const u of urls) expect(u.startsWith('data:image/svg+xml')).toBe(true);
  });

  it('does not style the build pair', () => {
    expect(all.some((s) => /\.(pair|pane)(?![\w-])/.test(s))).toBe(false);
  });

  it('does not animate (static under Reduce Motion)', () => {
    expect(code).not.toMatch(/animation|@keyframes|transition/);
  });
});
