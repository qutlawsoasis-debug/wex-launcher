import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import axe from 'axe-core';

import Sheet from './components/Sheet.svelte';
import SegmentedControl from './components/SegmentedControl.svelte';
import Row from './components/Row.svelte';
import Toggle from './components/Toggle.svelte';
import { theme } from './design/theme.svelte';
import HomePage from './pages/home/HomePage.svelte';
import BuildPage from './pages/build/BuildPage.svelte';
import ServersPage from './pages/servers/ServersPage.svelte';
import FriendsPage from './pages/friends/FriendsPage.svelte';
import WardrobePage from './pages/wardrobe/WardrobePage.svelte';
import SettingsPage from './pages/settings/SettingsPage.svelte';
import Sidebar from './shell/Sidebar.svelte';
import TopLine from './shell/TopLine.svelte';
import LaunchBar from './shell/LaunchBar.svelte';

// Mock canvas getContext in jsdom to suppress warnings
if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = (() => ({
    fillRect: () => {},
    clearRect: () => {},
    getImageData: () => ({ data: new Uint8ClampedArray(4) }),
    putImageData: () => {},
    createImageData: () => [],
    setTransform: () => {},
    drawImage: () => {},
    save: () => {},
    fillText: () => {},
    restore: () => {},
    beginPath: () => {},
    moveTo: () => {},
    lineTo: () => {},
    closePath: () => {},
    stroke: () => {},
    translate: () => {},
    scale: () => {},
    rotate: () => {},
    arc: () => {},
    ellipse: () => {},
    fill: () => {},
    measureText: () => ({ width: 0 }),
    transform: () => {},
    rect: () => {},
    clip: () => {},
  })) as any;
}

const invokeMock = vi.fn(async (cmd: string, args?: any) => {
  if (cmd === 'list_servers') return [];
  if (cmd === 'list_open_rooms') return [];
  if (cmd === 'list_lan_worlds') return [];
  if (cmd === 'social_sync') return { friends: [], requests: [] };
  if (cmd === 'social_list_achievements') return { achievements: [], progress: [] };
  if (cmd === 'wardrobe_get_state') return null;
  if (cmd === 'extras_list_skin_history') return [];
  if (cmd === 'workshop_list_worlds') return [];
  if (cmd === 'workshop_list_screenshots') return [];
  if (cmd === 'list_mods') return [];
  if (cmd === 'get_launcher_settings') return { selected_version: '1.21.1', player_name: 'Player' };
  return null;
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args as [string, any?]),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
}));

describe('A11y Automated Axe-Core Audit Across All 8 Sections', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: any) => invokeMock(cmd, args),
      transformCallback: (fn: any) => fn,
    };
    vi.clearAllMocks();
  });

  afterEach(() => {
    target.remove();
    delete (window as any).__TAURI_INTERNALS__;
  });

  const screens: Array<{ name: string; component: unknown; props: Record<string, unknown> }> = [
    { name: 'Sidebar', component: Sidebar, props: { online: true } },
    { name: 'TopLine', component: TopLine, props: {} },
    { name: 'LaunchBar', component: LaunchBar, props: {} },
    { name: 'HomePage', component: HomePage, props: {} },
    { name: 'BuildPage', component: BuildPage, props: { version: '1.21.1' } },
    { name: 'ServersPage', component: ServersPage, props: {} },
    { name: 'FriendsPage', component: FriendsPage, props: {} },
    { name: 'WardrobePage', component: WardrobePage, props: {} },
    { name: 'SettingsPage', component: SettingsPage, props: {} },
  ];

  for (const { name, component, props } of screens) {
    for (const appearance of ['dark', 'light'] as const) {
      // Axe traverses the full screen DOM; allow shared CI CPU contention without skipping checks.
      it(`evaluates ${name} accessibility in ${appearance} theme`, async () => {
        theme.setAppearance(appearance);
        target.setAttribute('data-theme', appearance);

        const comp = mount(component as any, { target, props });
        await tick();
        await new Promise((r) => setTimeout(r, 60));

        const results = await axe.run(target, {
          rules: {
            'color-contrast': { enabled: false },
          },
        });

        const seriousOrCritical = results.violations.filter(
          (v: any) => v.impact === 'serious' || v.impact === 'critical'
        );

        if (seriousOrCritical.length > 0) {
          console.log(
            `[AXE VIOLATIONS] ${name} (${appearance}):`,
            seriousOrCritical.map((v: any) => ({
              id: v.id,
              impact: v.impact,
              description: v.description,
              nodes: v.nodes.map((n: any) => n.html),
            }))
          );
        }

        expect(seriousOrCritical).toEqual([]);
        unmount(comp);
      }, 15_000);
    }
  }

  describe('Keyboard accessibility interactions', () => {
    it('closes Sheet modal on Escape key', async () => {
      let open = true;
      const comp = mount(Sheet as any, {
        target,
        props: {
          open,
          title: 'Тестовое окно',
          onclose: () => {
            open = false;
          },
        },
      });

      await tick();
      expect(document.querySelector('.sheet-card')).not.toBeNull();

      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await tick();
      expect(open).toBe(false);

      unmount(comp);
    });

    it('navigates SegmentedControl options with Arrow keys', async () => {
      let selected = 'opt1';
      const comp = mount(SegmentedControl as any, {
        target,
        props: {
          options: [
            { value: 'opt1', label: 'Опция 1' },
            { value: 'opt2', label: 'Опция 2' },
            { value: 'opt3', label: 'Опция 3' },
          ],
          value: selected,
          onchange: (val: string) => {
            selected = val;
          },
        },
      });

      await tick();
      const buttons = target.querySelectorAll<HTMLButtonElement>('.segment-button');
      expect(buttons.length).toBe(3);

      buttons[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
      await tick();
      expect(selected).toBe('opt2');

      buttons[1].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
      await tick();
      expect(selected).toBe('opt1');

      unmount(comp);
    });

    it('toggles switch state on Space or Enter key', async () => {
      let checked = false;
      const comp = mount(Toggle as any, {
        target,
        props: {
          checked,
          label: 'Турбо-режим',
          onchange: (val: boolean) => {
            checked = val;
          },
        },
      });

      await tick();
      const toggleEl = target.querySelector<HTMLElement>('.apple-toggle');
      expect(toggleEl).not.toBeNull();
      expect(toggleEl?.getAttribute('role')).toBe('switch');

      toggleEl?.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
      await tick();
      expect(checked).toBe(true);

      toggleEl?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      await tick();
      expect(checked).toBe(false);

      unmount(comp);
    });

  });
});
