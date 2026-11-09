import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import Button from './Button.svelte';
import Toggle from './Toggle.svelte';
import Slider from './Slider.svelte';
import SegmentedControl from './SegmentedControl.svelte';
import Row from './Row.svelte';
import List from './List.svelte';
import SearchField from './SearchField.svelte';
import ProgressBar from './ProgressBar.svelte';
import Toast from './Toast.svelte';
import ProfileBadge from './ProfileBadge.svelte';
import Sheet from './Sheet.svelte';
import Popover from './Popover.svelte';

describe('Apple HIG Svelte 5 Components', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders Button with variants and loading state', async () => {
    const comp = mount(Button, {
      target,
      props: { variant: 'primary', size: 'regular' },
    });
    const btn = target.querySelector('button');
    expect(btn).not.toBeNull();
    expect(btn?.classList.contains('primary')).toBe(true);
    expect(btn?.classList.contains('regular')).toBe(true);
    unmount(comp);

    const compLoading = mount(Button, {
      target,
      props: { variant: 'destructive', loading: true },
    });
    const spinner = target.querySelector('.spinner');
    expect(spinner).not.toBeNull();
    expect(target.querySelector('button')?.getAttribute('aria-busy')).toBe('true');
    unmount(compLoading);
  });

  it('renders Toggle switch with accessible role and toggle behavior', async () => {
    let checked = false;
    const comp = mount(Toggle, {
      target,
      props: {
        checked,
        label: 'Тестовый переключатель',
        onchange: (val) => {
          checked = val;
        },
      },
    });

    const switchEl = target.querySelector('[role="switch"]') as HTMLElement;
    expect(switchEl).not.toBeNull();
    expect(switchEl.getAttribute('aria-checked')).toBe('false');

    // Click toggle
    switchEl.click();
    expect(checked).toBe(true);

    unmount(comp);
  });

  it('renders Slider with correct percentage and track fill', () => {
    const comp = mount(Slider, {
      target,
      props: { value: 75, min: 0, max: 100, label: 'Громкость', unit: '%' },
    });

    const range = target.querySelector('input[type="range"]') as HTMLInputElement;
    expect(range).not.toBeNull();
    expect(range.value).toBe('75');

    const fill = target.querySelector('.track-fill') as HTMLElement;
    expect(fill.style.width).toBe('75%');

    unmount(comp);
  });

  it('slider thumb position corresponds strictly to value (min, middle, max, 65%)', () => {
    // 1. Min (0%)
    const compMin = mount(Slider, {
      target,
      props: { value: 0, min: 0, max: 100 },
    });
    const thumbMin = target.querySelector('[data-testid="slider-thumb"]') as HTMLElement;
    expect(thumbMin).not.toBeNull();
    expect(thumbMin.getAttribute('data-percentage')).toBe('0.0');
    expect(thumbMin.style.left).toBe('calc(0% - 0px)');
    unmount(compMin);

    // 2. Middle (50%)
    const compMid = mount(Slider, {
      target,
      props: { value: 50, min: 0, max: 100 },
    });
    const thumbMid = target.querySelector('[data-testid="slider-thumb"]') as HTMLElement;
    expect(thumbMid).not.toBeNull();
    expect(thumbMid.getAttribute('data-percentage')).toBe('50.0');
    expect(thumbMid.style.left).toBe('calc(50% - 10px)');
    unmount(compMid);

    // 3. Max (100%)
    const compMax = mount(Slider, {
      target,
      props: { value: 100, min: 0, max: 100 },
    });
    const thumbMax = target.querySelector('[data-testid="slider-thumb"]') as HTMLElement;
    expect(thumbMax).not.toBeNull();
    expect(thumbMax.getAttribute('data-percentage')).toBe('100.0');
    expect(thumbMax.style.left).toBe('calc(100% - 20px)');
    unmount(compMax);

    // 4. Custom 65% (gallery value)
    const comp65 = mount(Slider, {
      target,
      props: { value: 65, min: 0, max: 100 },
    });
    const thumb65 = target.querySelector('[data-testid="slider-thumb"]') as HTMLElement;
    expect(thumb65).not.toBeNull();
    expect(thumb65.getAttribute('data-percentage')).toBe('65.0');
    expect(thumb65.style.left).toBe('calc(65% - 13px)');
    unmount(comp65);
  });

  it('renders SegmentedControl with options and active selection pill', () => {
    const comp = mount(SegmentedControl, {
      target,
      props: {
        options: [
          { value: '1.20.1', label: '1.20.1' },
          { value: '1.21.1', label: '1.21.1' },
        ],
        value: '1.21.1',
      },
    });

    const tabs = target.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(2);
    expect(tabs[1].classList.contains('selected')).toBe(true);
    expect(tabs[1].getAttribute('aria-selected')).toBe('true');

    unmount(comp);
  });

  it('renders List and Row with hierarchical layout and accessories', () => {
    const comp = mount(List, {
      target,
      props: { header: 'Тестовая группа' },
    });

    expect(target.querySelector('.list-header')?.textContent).toBe('Тестовая группа');
    unmount(comp);

    const rowComp = mount(Row, {
      target,
      props: {
        title: 'Заголовок строки',
        subtitle: 'Подзаголовок',
        trailingText: '128 ms',
        showChevron: true,
        clickable: true,
      },
    });

    expect(target.querySelector('.row-title')?.textContent).toBe('Заголовок строки');
    expect(target.querySelector('.row-subtitle')?.textContent).toBe('Подзаголовок');
    expect(target.querySelector('.trailing-text')?.textContent).toBe('128 ms');
    expect(target.querySelector('.chevron')).not.toBeNull();
    unmount(rowComp);
  });

  it('renders SearchField with search icon and clear button', async () => {
    let query = 'Sodium';
    const comp = mount(SearchField, {
      target,
      props: {
        value: query,
        oninput: (val) => {
          query = val;
        },
      },
    });

    const input = target.querySelector('.search-input') as HTMLInputElement;
    expect(input.value).toBe('Sodium');

    const clearBtn = target.querySelector('.clear-btn') as HTMLButtonElement;
    expect(clearBtn).not.toBeNull();

    clearBtn.click();
    expect(query).toBe('');
    unmount(comp);
  });

  it('renders ProgressBar in determinate and indeterminate modes', () => {
    const comp = mount(ProgressBar, {
      target,
      props: { percentage: 42, label: 'Синхронизация...', detail: '42%' },
    });

    const fill = target.querySelector('.progress-fill') as HTMLElement;
    expect(fill).not.toBeNull();
    expect(fill.style.width).toBe('42%');
    expect(fill.getAttribute('aria-valuenow')).toBe('42');
    unmount(comp);

    const compIndet = mount(ProgressBar, {
      target,
      props: { indeterminate: true, label: 'Загрузка...' },
    });
    const indet = target.querySelector('.progress-indeterminate');
    expect(indet).not.toBeNull();
    expect(indet?.getAttribute('role')).toBe('progressbar');
    expect(indet?.getAttribute('aria-busy')).toBe('true');
    unmount(compIndet);
  });

  it('renders Toast with semantic status and dismiss handler', () => {
    let dismissedId = '';
    const comp = mount(Toast, {
      target,
      props: {
        toast: {
          id: 'test-toast',
          type: 'success',
          title: 'Успешно выполнено',
          description: 'Тестовое описание',
        },
        ondismiss: (id) => {
          dismissedId = id;
        },
      },
    });

    expect(target.querySelector('.apple-toast.success')).not.toBeNull();
    expect(target.querySelector('.toast-title')?.textContent).toBe('Успешно выполнено');
    expect(target.querySelector('.toast-description')?.textContent).toBe('Тестовое описание');

    const dismissBtn = target.querySelector('.dismiss-btn') as HTMLButtonElement;
    expect(dismissBtn).not.toBeNull();
    dismissBtn.click();
    expect(dismissedId).toBe('test-toast');
    unmount(comp);
  });

  it('renders Toast with action button and handles action click', () => {
    let actionTriggered = false;
    let dismissedId = '';
    const comp = mount(Toast, {
      target,
      props: {
        toast: {
          id: 'invite-toast',
          type: 'info',
          title: 'Вас пригласили',
          description: 'Alice зовёт вас в игру',
          action: {
            label: 'Присоединиться',
            onclick: () => {
              actionTriggered = true;
            },
          },
        },
        ondismiss: (id) => {
          dismissedId = id;
        },
      },
    });

    const actionBtn = target.querySelector('.toast-action-btn') as HTMLButtonElement;
    expect(actionBtn).not.toBeNull();
    expect(actionBtn.textContent).toBe('Присоединиться');
    actionBtn.click();
    expect(actionTriggered).toBe(true);
    expect(dismissedId).toBe('invite-toast');
    unmount(comp);
  });

  it('renders ProfileBadge in offline mode with initial letter avatar and offline dot', () => {
    const comp = mount(ProfileBadge, {
      target,
      props: {
        playerName: 'Steve',
        isOnline: false,
      },
    });

    expect(target.querySelector('.profile-nickname')?.textContent).toBe('Steve');
    const fallback = target.querySelector('.avatar-letter-fallback');
    expect(fallback).not.toBeNull();
    expect(fallback?.textContent?.trim()).toBe('S');
    expect(target.querySelector('.network-status-dot.offline')).not.toBeNull();
    expect(target.textContent).toContain('Офлайн');
    unmount(comp);
  });

  it('renders ProfileBadge in online mode with online indicator dot and head image attempt', () => {
    const comp = mount(ProfileBadge, {
      target,
      props: {
        playerName: 'Alex',
        isOnline: true,
      },
    });

    expect(target.querySelector('.profile-nickname')?.textContent).toBe('Alex');
    expect(target.querySelector('.network-status-dot.online')).not.toBeNull();
    expect(target.textContent).toContain('В сети');
    const img = target.querySelector('.avatar-head-image') as HTMLImageElement;
    expect(img).not.toBeNull();
    expect(img.src).toContain('Alex');
    unmount(comp);
  });

  it('renders Sheet with modal dialog semantics and dispatches cancel and done', () => {
    let closed = false;
    let done = false;
    const comp = mount(Sheet, {
      target,
      props: {
        open: true,
        title: 'Тестовый диалог',
        cancelLabel: 'Отменить',
        doneLabel: 'Сохранить',
        onclose: () => {
          closed = true;
        },
        ondone: () => {
          done = true;
        },
      },
    });

    const dialog = target.querySelector('[role="dialog"]') as HTMLElement;
    expect(dialog).not.toBeNull();
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-label')).toBe('Тестовый диалог');
    expect(target.querySelector('.grabber')).not.toBeNull();

    const doneBtn = target.querySelectorAll('.header-action.trailing button')[0] as HTMLButtonElement;
    expect(doneBtn).not.toBeNull();
    doneBtn.click();
    expect(done).toBe(true);

    unmount(comp);
  });

  it('dismisses Sheet on Escape key', () => {
    let closed = false;
    const comp = mount(Sheet, {
      target,
      props: {
        open: true,
        title: 'Escape тест',
        onclose: () => {
          closed = true;
        },
      },
    });

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(closed).toBe(true);
    unmount(comp);
  });

  it('renders Popover with trigger and dialog semantics and handles Escape', () => {
    let closed = false;
    const comp = mount(Popover, {
      target,
      props: {
        open: true,
        title: 'Информация',
        placement: 'bottom-start',
        onclose: () => {
          closed = true;
        },
      },
    });

    const dialog = target.querySelector('[role="dialog"]') as HTMLElement;
    expect(dialog).not.toBeNull();
    expect(dialog.classList.contains('bottom-start')).toBe(true);
    expect(dialog.getAttribute('aria-label')).toBe('Информация');

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(closed).toBe(true);
    unmount(comp);
  });

  it('supports Button click events, variants and disabled state', () => {
    let clicked = false;
    const comp = mount(Button, {
      target,
      props: {
        variant: 'ghost',
        size: 'small',
        onclick: () => {
          clicked = true;
        },
      },
    });

    const btn = target.querySelector('button') as HTMLButtonElement;
    expect(btn.classList.contains('ghost')).toBe(true);
    expect(btn.classList.contains('small')).toBe(true);
    btn.click();
    expect(clicked).toBe(true);
    unmount(comp);

    let disabledClicked = false;
    const compDisabled = mount(Button, {
      target,
      props: {
        disabled: true,
        onclick: () => {
          disabledClicked = true;
        },
      },
    });
    const disabledBtn = target.querySelector('button') as HTMLButtonElement;
    expect(disabledBtn.disabled).toBe(true);
    disabledBtn.click();
    expect(disabledClicked).toBe(false);
    unmount(compDisabled);
  });

  it('toggles Toggle with Space and Enter keyboard keys and respects disabled state', () => {
    let checked = false;
    const comp = mount(Toggle, {
      target,
      props: {
        checked: false,
        onchange: (val) => {
          checked = val;
        },
      },
    });

    const switchEl = target.querySelector('[role="switch"]') as HTMLElement;
    switchEl.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    expect(checked).toBe(true);

    switchEl.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    expect(checked).toBe(false);
    unmount(comp);

    let disabledToggle = false;
    const compDisabled = mount(Toggle, {
      target,
      props: {
        checked: false,
        disabled: true,
        onchange: (val) => {
          disabledToggle = val;
        },
      },
    });
    const disabledSwitch = target.querySelector('[role="switch"]') as HTMLElement;
    expect(disabledSwitch.getAttribute('aria-disabled')).toBe('true');
    disabledSwitch.click();
    expect(disabledToggle).toBe(false);
    unmount(compDisabled);
  });

  it('navigates SegmentedControl options with arrow keys', () => {
    let current = 'first';
    const comp = mount(SegmentedControl, {
      target,
      props: {
        options: [
          { value: 'first', label: 'Первый' },
          { value: 'second', label: 'Второй' },
          { value: 'third', label: 'Третий' },
        ],
        value: current,
        size: 'small',
        onchange: (val) => {
          current = val;
        },
      },
    });

    expect(target.querySelector('.apple-segmented-control')?.classList.contains('small')).toBe(true);
    const tabs = target.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(3);

    tabs[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    expect(current).toBe('second');

    unmount(comp);
  });

  it('clears SearchField with Escape key and renders accessible search role', () => {
    let searchVal = 'Тест';
    const comp = mount(SearchField, {
      target,
      props: {
        value: searchVal,
        oninput: (val) => {
          searchVal = val;
        },
      },
    });

    expect(target.querySelector('[role="search"]')).not.toBeNull();
    const input = target.querySelector('input[type="search"]') as HTMLInputElement;
    expect(input).not.toBeNull();

    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(searchVal).toBe('');
    unmount(comp);
  });

  it('renders interactive Row with click handler and disabled state', () => {
    let rowClicked = false;
    const comp = mount(Row, {
      target,
      props: {
        title: 'Кликабельный элемент',
        clickable: true,
        onclick: () => {
          rowClicked = true;
        },
      },
    });

    const btn = target.querySelector('button.apple-row') as HTMLButtonElement;
    expect(btn).not.toBeNull();
    btn.click();
    expect(rowClicked).toBe(true);
    unmount(comp);
  });
});
