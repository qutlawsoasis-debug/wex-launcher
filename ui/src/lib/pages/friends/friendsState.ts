/**
 * Words for the friends sign-in state (friendsLive.connection), shared by the
 * Friends page and the Home panel. Only what core reported; nothing invented.
 * HIG › Writing: say what happened and what the player can do.
 */

import type { FriendsConnection } from '../../services/friendsLive';
import { t } from '../../i18n';

export interface ConnectionText {
  title: string;
  text: string;
  /** «Повторить» makes sense (the server may answer next time). */
  retry: boolean;
  /** Not a problem: just in progress. */
  busy: boolean;
}

const UNREACHABLE_REASONS = ['network', 'timeout', 'server_error', 'no_friends_api', 'bad_response'];

/** `null` when friends work (signed in). */
export function describeConnection(c: FriendsConnection): ConnectionText | null {
  const server = c.server || t('friends2.server_generic');
  switch (c.state) {
    case 'signed_in':
      return null;
    case 'connecting':
      return { title: t('friends2.state_connecting'), text: '', retry: false, busy: true };
    case 'signed_out':
      if (c.reason === 'session_expired') return { title: t('friends2.state_connecting'), text: '', retry: true, busy: true };
      return { title: t('friends2.state_no_account_t'), text: t('friends2.state_no_account_d'), retry: false, busy: false };
    case 'unusable':
      return {
        title: t('friends2.state_unusable_t'),
        text: c.detail ? t('friends2.reason_nick_rejected', { nick: c.nick || '', detail: c.detail }) : t('friends2.reason_nick_rejected_plain', { nick: c.nick || '' }),
        retry: true,
        busy: false,
      };
    case 'not_configured':
      return { title: t('friends2.state_disabled_t'), text: t('friends2.state_disabled_d'), retry: false, busy: false };
    case 'unreachable': {
      const reason = UNREACHABLE_REASONS.includes(c.reason || '') ? c.reason : 'other';
      return { title: t('friends2.state_unreachable_t'), text: t(`friends2.reason_${reason}`, { server }), retry: true, busy: false };
    }
  }
}
