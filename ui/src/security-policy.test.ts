// @vitest-environment node
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const appDir = resolve(process.cwd(), '../app/src-tauri');
const read = (name: string) => readFileSync(resolve(appDir, name), 'utf8');

describe('bundled WebView security boundary', () => {
  it('enforces a CSP without remote scripts, eval, framing or arbitrary connections', () => {
    const policy = JSON.parse(read('tauri.conf.json')).app.security.csp;
    expect(typeof policy).toBe('string');
    expect(policy).toContain("default-src 'none'");
    expect(policy).toContain("script-src 'self'");
    expect(policy).not.toMatch(/unsafe-eval|script-src[^;]*(?:https:|unsafe-inline|\*)/);
    expect(policy).toContain("frame-src 'none'");
    expect(policy).toContain("object-src 'none'");
    expect(policy).toContain("base-uri 'none'");
    expect(policy).toContain("form-action 'none'");
    expect(policy).not.toMatch(/connect-src[^;]*(?:https:;|\*)/);
  });

  it('enrolls every registered custom command in the Tauri ACL', () => {
    const handler = read('src/lib.rs').split('.invoke_handler(')[1].split('.run(')[0];
    const commands = [...handler.matchAll(/commands::\w+::(\w+)/g)].map(m => m[1]);
    const build = read('build.rs');
    expect(build).toContain('AppManifest::new().commands(');
    const manifest = read('src/ipc_commands.rs');
    for (const command of commands) expect(manifest).toContain(`"${command}"`);
    const capability = JSON.parse(read('capabilities/default.json'));
    expect(capability.windows).toEqual(['main']);
    expect(capability.remote).toBeUndefined();
    expect(capability.permissions).toContain('launcher-ui');
    const permissions = read('permissions/launcher-ui.toml');
    expect(permissions).not.toMatch(/"(?:net_tunnel_start|extras_generate_csl_config|extras_save_skin)"/);
  });
});
