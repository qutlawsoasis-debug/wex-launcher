// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { build } from 'vite';
import * as path from 'node:path';

describe('20: production native notification module', () => {
  it('bundles notification commands without unresolved bare specifiers', async () => {
    const result=await build({root:path.resolve(__dirname,'../../..'),build:{write:false,minify:true}});
    const outputs=Array.isArray(result)?result:[result];
    const code=outputs.flatMap(o=>'output' in o?o.output:[]).filter(o=>o.type==='chunk').map(o=>o.code).join('\n');
    expect(code).not.toContain('@tauri-apps/plugin-notification');
    expect(code).toContain('plugin:notification|');
  },30000);
});
