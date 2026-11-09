import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import DeepLinkConfirmationModal from './DeepLinkConfirmationModal.svelte';

describe('DeepLinkConfirmationModal Component', () => {
  let target: HTMLElement;

  beforeEach(() => {
    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    target.remove();
  });

  it('renders confirmation modal when open with action details', async () => {
    const comp = mount(DeepLinkConfirmationModal, {
      target,
      props: {
        open: true,
        action: {
          kind: 'join',
          title: 'Вход в лобби #A7X9',
          subtitle: 'Игрок Steve приглашает вас присоединиться к общему миру.',
          target: '#A7X9',
          raw_uri: 'wex://lobby/join?code=A7X9',
          confirm_text: 'Присоединиться',
        },
      },
    });
    await tick();

    const modal = target.querySelector('[data-testid="deeplink-modal"]');
    expect(modal).not.toBeNull();
    expect(target.textContent).toContain('Вход в лобби #A7X9');
    expect(target.textContent).toContain('#A7X9');
    expect(target.textContent).toContain('wex://lobby/join?code=A7X9');

    unmount(comp);
  });

  it('handles confirm and cancel actions', async () => {
    const onconfirm = vi.fn();
    const oncancel = vi.fn();

    const comp = mount(DeepLinkConfirmationModal, {
      target,
      props: {
        open: true,
        action: {
          kind: 'server',
          title: 'Подключение к серверу',
          subtitle: 'Добавить сервер в список избранных',
          target: 'mc.example.com:25565',
        },
        onconfirm,
        oncancel,
      },
    });
    await tick();

    const confirmBtn = target.querySelector('.deeplink-actions button.apple-button.primary') as HTMLButtonElement;
    expect(confirmBtn).not.toBeNull();
    confirmBtn.click();
    expect(onconfirm).toHaveBeenCalledTimes(1);

    unmount(comp);
  });
});
