/**
 * Page routing with back/forward history (HIG › Navigation: the toolbar's back
 * and forward move through visited pages). Routes live in the URL hash so a
 * deep link can open a page, but the history stack is our own.
 */

import { tick } from 'svelte';
import { motionReduced } from '../design/wex-tokens';

export type Section = 'home' | 'servers' | 'friends' | 'wardrobe' | 'options';
export type Route = Section | `build-${string}`;

export const SECTIONS: readonly Section[] = ['home', 'servers', 'friends', 'wardrobe', 'options'];

export function buildRoute(versionId: string): Route {
  return `build-${versionId}` as Route;
}

export function buildIdOf(route: Route): string | null {
  return route.startsWith('build-') ? route.slice('build-'.length) : null;
}

export function isRoute(value: string, buildIds: readonly string[]): value is Route {
  if ((SECTIONS as readonly string[]).includes(value)) return true;
  const id = buildIdOf(value as Route);
  return id !== null && buildIds.includes(id);
}

class Router {
  route = $state<Route>('home');
  back = $state<Route[]>([]);
  forward = $state<Route[]>([]);
  /** Bumped on every change; pages use it to replay their entrance. */
  version = $state(0);

  get canBack(): boolean {
    return this.back.length > 0;
  }

  get canForward(): boolean {
    return this.forward.length > 0;
  }

  go(next: Route, push = true): void {
    if (next === this.route) return;
    if (push) {
      this.back = [...this.back, this.route];
      this.forward = [];
    }
    const apply = () => {
      this.route = next;
      this.version++;
      this.writeHash();
    };
    // The old page fades out through View Transitions (reference: vt-out); the new
    // one rises block by block (stagger). Skipped under Reduce Motion.
    const doc = typeof document !== 'undefined' ? (document as Document & { startViewTransition?: (cb: () => Promise<void>) => unknown }) : null;
    if (doc?.startViewTransition && !motionReduced()) {
      doc.startViewTransition(async () => {
        apply();
        await tick();
      });
    } else {
      apply();
    }
  }

  goBack(): void {
    const prev = this.back.at(-1);
    if (!prev) return;
    this.back = this.back.slice(0, -1);
    this.forward = [...this.forward, this.route];
    this.go(prev, false);
  }

  goForward(): void {
    const next = this.forward.at(-1);
    if (!next) return;
    this.forward = this.forward.slice(0, -1);
    this.back = [...this.back, this.route];
    this.go(next, false);
  }

  /** Open the same page again from the start (e.g. with a new intent). */
  reopen(): void {
    this.version++;
  }

  /** Reset to a clean state (tests, sign-out). */
  reset(route: Route = 'home'): void {
    this.route = route;
    this.back = [];
    this.forward = [];
    this.version = 0;
  }

  private writeHash(): void {
    if (typeof window === 'undefined') return;
    const hash = this.route === 'home' ? '' : `#${this.route}`;
    if (window.location.hash === hash) return;
    try {
      history.replaceState(null, '', hash || window.location.pathname + window.location.search);
    } catch {}
  }
}

export const router = new Router();
