<!--
  Wex Launcher — Screen «Друзья»
  Rules:
  - List of friends with status and current game activity
  - Fast search and direct invite actions
-->
<script lang="ts">
  import List from '../components/List.svelte';
  import Row from '../components/Row.svelte';
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

  let friends = $state<Friend[]>([
    {
      id: '1',
      username: 'Alex_Pro',
      status: 'playing',
      activity: 'Играет на Aura Official (1.21.1)',
    },
    {
      id: '2',
      username: 'CraftMaster',
      status: 'playing',
      activity: 'Выживание • Одиночный мир',
    },
    {
      id: '3',
      username: 'DiamondHunter',
      status: 'online',
      activity: 'В лаунчере',
    },
    {
      id: '4',
      username: 'NightWatcher',
      status: 'offline',
      activity: 'Был в сети 3 часа назад',
    },
  ]);

  let filteredFriends = $derived(
    friends.filter((f) => f.username.toLowerCase().includes(searchQuery.toLowerCase()))
  );
</script>

<div class="friends-screen">
  <div class="screen-toolbar">
    <SearchField placeholder="Поиск друзей..." bind:value={searchQuery} />
    <Button variant="primary" size="small">
      <Icon name="friends" size={14} color="#ffffff" />
      <span>Найти игрока</span>
    </Button>
  </div>

  <div class="friends-content">
    <List header="Друзья онлайн">
      {#each filteredFriends.filter((f) => f.status !== 'offline') as f}
        <Row title={f.username} subtitle={f.activity} clickable>
          {#snippet leading()}
            <div class="avatar" class:playing={f.status === 'playing'}>
              <span class="avatar-initial">{f.username[0]}</span>
              <span class="status-indicator {f.status}"></span>
            </div>
          {/snippet}

          {#snippet trailing()}
            <Button variant="secondary" size="small">
              {f.status === 'playing' ? 'Присоединиться' : 'Пригласить'}
            </Button>
          {/snippet}
        </Row>
      {/each}
    </List>

    <List header="Не в сети">
      {#each filteredFriends.filter((f) => f.status === 'offline') as f}
        <Row title={f.username} subtitle={f.activity}>
          {#snippet leading()}
            <div class="avatar offline">
              <span class="avatar-initial">{f.username[0]}</span>
              <span class="status-indicator offline"></span>
            </div>
          {/snippet}
        </Row>
      {/each}
    </List>
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
  }

  .screen-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .friends-content {
    flex: 1;
  }

  .avatar {
    position: relative;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: linear-gradient(135deg, #4facfe, #00f2fe);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    font-weight: 700;
    font-size: var(--text-subheadline);
  }

  .avatar.playing {
    background: linear-gradient(135deg, #0091ff, #6d7cff);
  }

  .avatar.offline {
    background: var(--color-quaternary-system-fill);
    color: var(--color-tertiary-label);
  }

  .status-indicator {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid var(--color-system-background);
  }

  .status-indicator.playing {
    background-color: var(--color-system-blue);
  }

  .status-indicator.online {
    background-color: var(--color-system-green);
  }

  .status-indicator.offline {
    background-color: var(--color-system-gray);
  }
</style>
