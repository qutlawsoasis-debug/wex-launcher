import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import EmptyState from './EmptyState.svelte';

describe('EmptyState Component (Мягкие пустые состояния)', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders soft empty state with title and description', async () => {
    const comp = mount(EmptyState, {
      target,
      props: {
        icon: 'friends',
        title: 'Нет друзей онлайн',
        description: 'Отправьте код другу, чтобы играть вместе.',
      },
    });
    await tick();

    const empty = target.querySelector('[data-testid="empty-state"]');
    expect(empty).not.toBeNull();
    expect(target.textContent).toContain('Нет друзей онлайн');
    expect(target.textContent).toContain('Отправьте код другу, чтобы играть вместе.');

    unmount(comp);
  });

  it('renders action button and invokes onaction callback', async () => {
    const handleAction = vi.fn();
    const comp = mount(EmptyState, {
      target,
      props: {
        title: 'Нет серверов',
        description: 'Добавьте свой первый сервер',
        actionText: 'Добавить сервер',
        onaction: handleAction,
      },
    });
    await tick();

    const btn = target.querySelector('button');
    expect(btn).not.toBeNull();
    btn?.click();

    expect(handleAction).toHaveBeenCalledTimes(1);
    unmount(comp);
  });
});
