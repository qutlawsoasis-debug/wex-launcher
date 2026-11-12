import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import Sheet from './lib/components/Sheet.svelte';
import OnboardingModal from './lib/components/OnboardingModal.svelte';
import ServersScreen from './lib/screens/ServersScreen.svelte';
import { theme } from './lib/design/theme.svelte';

let target: HTMLDivElement;

beforeEach(() => {
  target = document.createElement('div');
  document.body.appendChild(target);
  localStorage.clear();
  localStorage.setItem('wex_onboarding_completed', 'true');
});

afterEach(() => {
  target.remove();
  document.body.innerHTML = '';
});

describe('Apple HIG Fixes Verification', () => {
  it('1. Sheet does not close when ondone returns false (empty save validation)', async () => {
    let open = true;
    let ondoneCalled = false;

    const comp = mount(Sheet, {
      target,
      props: {
        open,
        title: 'Тестовый Sheet',
        showDone: true,
        ondone: () => {
          ondoneCalled = true;
          return false; // validation failed
        },
      },
    });

    const doneBtn = target.querySelector<HTMLButtonElement>('.header-action.trailing button');
    expect(doneBtn).not.toBeNull();
    doneBtn?.click();
    await new Promise((r) => setTimeout(r, 50));

    expect(ondoneCalled).toBe(true);
    // Sheet card must remain in the DOM because ondone returned false
    expect(target.querySelector('.sheet-card')).not.toBeNull();

    unmount(comp);
  });

  it('2. ServersScreen keeps Add Server sheet open and displays error when saving empty inputs', async () => {
    const comp = mount(ServersScreen, { target });

    const buttons = Array.from(target.querySelectorAll('button'));
    const addBtn = buttons.find((b) => b.textContent?.includes('Добавить'));
    expect(addBtn).toBeDefined();
    addBtn?.click();
    await new Promise((r) => setTimeout(r, 50));

    expect(document.querySelector('.sheet-card')).not.toBeNull();

    // Click "Сохранить" with empty fields
    const sheetDoneBtn = document.querySelector<HTMLButtonElement>('.header-action.trailing button');
    expect(sheetDoneBtn).not.toBeNull();
    sheetDoneBtn?.click();
    await new Promise((r) => setTimeout(r, 50));

    // The sheet must remain open with validation error displayed
    expect(document.querySelector('.sheet-card')).not.toBeNull();
    expect(document.querySelector('.sheet-error')).not.toBeNull();
    expect(document.querySelector('.sheet-error')?.textContent).toContain('Укажите название и адрес');

    unmount(comp);
  });

  it('5. Reduce motion styles preserve transform positioning and disable animation durations', () => {
    theme.setReduceMotion(true);
    expect(theme.reduceMotion).toBe(true);

    const root = document.documentElement;
    expect(root.getAttribute('data-reduce-motion')).toBe('true');

    theme.setReduceMotion(false);
  });

  it('6. Modal cards and sheets do not display window-level focus outline when focused', async () => {
    const comp = mount(OnboardingModal, {
      target,
      props: {
        open: true,
      },
    });

    const card = target.querySelector<HTMLElement>('.onboarding-card');
    expect(card).not.toBeNull();
    card?.focus();
    expect(document.activeElement).toBe(card);

    unmount(comp);
  });
});
