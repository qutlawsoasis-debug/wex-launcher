/** Which Friends tab opens next (an invite or a join link opens Rooms). Consumed once. */
export const friendsIntent = $state<{ tab: 'friends' | 'rooms' }>({ tab: 'friends' });
