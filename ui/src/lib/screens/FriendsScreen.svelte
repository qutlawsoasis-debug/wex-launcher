<!--
  Wex Launcher — Screen «Друзья»
  Rules:
  - Clean empty state without fabricated friend activity when disconnected
  - Search field for discovering players
  - Compliant with HIG lists and empty states
-->
<script lang="ts">
  import Button from '../components/Button.svelte';
  import SearchField from '../components/SearchField.svelte';
  import Icon from '../components/icons/Icon.svelte';

  interface Friend {
    id: string;
    username: string;
    status: 'playing' | 'online' | 'offline';
    activity?: string;
  }

  let searchQuery = $state('');
  // No fake dummy friends — empty state by default without live social backend
  let friends = $state<Friend[]>([]);

  let filteredFriends = $derived(
    friends.filter((f) => f.username.toLowerCase().includes(searchQuery.toLowerCase()))
  );
</script>

<div class="friends-screen">
  <div class="screen-toolbar">
    <SearchField placeholder="Поиск игроков по никнейму..." bind:value={searchQuery} />
    <Button variant="primary" size="small">
      <Icon name="friends" size={14} color="#ffffff" />
      <span>Найти игрока</span>
    </Button>
  </div>

  <div class="friends-content">
    {#if filteredFriends.length > 0}
      <!-- Render friends list if populated -->
      <div class="friends-list">
        <!-- populated list -->
      </div>
    {:else}
      <div class="empty-state-card">
        <div class="empty-icon-wrap">
          <Icon name="friends" size={40} color="var(--color-tertiary-label)" />
        </div>
        <h3 class="empty-title">Список друзей пуст</h3>
        <p class="empty-desc">
          {searchQuery
            ? 'По запросу «' + searchQuery + '» ни одного игрока не найдено.'
            : 'У вас пока нет добавленных друзей. Введите никнейм друга в строке поиска, чтобы отправить приглашение в лобби.'}
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  .friends-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow-y: auto;
    font-family: var(--font-system);
    box-sizing: border-box;
  }

  .screen-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .friends-content {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .empty-state-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 24px;
    gap: 12px;
    text-align: center;
    background-color: var(--card-bg);
    border: 1px dashed var(--glass-border);
    border-radius: var(--radius-lg);
    margin-top: 12px;
  }

  .empty-icon-wrap {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background-color: var(--color-quaternary-system-fill);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-title {
    font-size: var(--text-headline);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
  }

  .empty-desc {
    font-size: var(--text-subheadline);
    color: var(--color-secondary-label);
    max-width: 420px;
    line-height: 1.4;
    margin: 0;
  }
</style>
