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

  it('renders SegmentedControl with options and active selection pill', () => {
    const comp = mount(SegmentedControl, {
      target,
      props: {
        options: [
          { value: '1.21.1', label: '1.21.1' },
          { value: '1.20.1', label: '1.20.1' },
        ],
        value: '1.21.1',
      },
    });

    const tabs = target.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(2);
    expect(tabs[0].classList.contains('selected')).toBe(true);
    expect(tabs[0].getAttribute('aria-selected')).toBe('true');

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
    expect(target.querySelector('.progress-indeterminate')).not.toBeNull();
    unmount(compIndet);
  });

  it('renders Toast with semantic status and auto-dismiss handler', () => {
    const comp = mount(Toast, {
      target,
      props: {
        toast: {
          id: 'test-toast',
          type: 'success',
          title: 'Успешно выполнено',
          description: 'Тестовое описание',
        },
      },
    });

    expect(target.querySelector('.apple-toast.success')).not.toBeNull();
    expect(target.querySelector('.toast-title')?.textContent).toBe('Успешно выполнено');
    expect(target.querySelector('.toast-description')?.textContent).toBe('Тестовое описание');
    unmount(comp);
  });
});
