<script lang="ts">
  interface Props {
    name:
      | 'play'
      | 'pause'
      | 'server'
      | 'lobby'
      | 'friends'
      | 'wardrobe'
      | 'mods'
      | 'settings'
      | 'search'
      | 'close'
      | 'clear'
      | 'minimize'
      | 'maximize'
      | 'restore'
      | 'chevron-right'
      | 'chevron-left'
      | 'chevron-down'
      | 'check'
      | 'refresh'
      | 'info'
      | 'warning'
      | 'error'
      | 'sun'
      | 'moon'
      | 'folder'
      | 'cpu';
    size?: number;
    color?: string;
    class?: string;
  }

  let { name, size = 18, color = 'currentColor', class: className = '' }: Props = $props();
</script>

<svg
  class="apple-icon {className}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke={color}
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'play'}
    <polygon points="6 3 20 12 6 21 6 3" fill={color} stroke="none"></polygon>
  {:else if name === 'pause'}
    <rect x="6" y="4" width="4" height="16" rx="1" fill={color} stroke="none"></rect>
    <rect x="14" y="4" width="4" height="16" rx="1" fill={color} stroke="none"></rect>
  {:else if name === 'server'}
    <rect x="2" y="4" width="20" height="6" rx="2"></rect>
    <rect x="2" y="14" width="20" height="6" rx="2"></rect>
    <line x1="6" y1="7" x2="6.01" y2="7"></line>
    <line x1="6" y1="17" x2="6.01" y2="17"></line>
  {:else if name === 'lobby'}
    <circle cx="18" cy="5" r="3"></circle>
    <circle cx="6" cy="12" r="3"></circle>
    <circle cx="18" cy="19" r="3"></circle>
    <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line>
    <line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line>
  {:else if name === 'friends'}
    <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"></path>
    <circle cx="9" cy="7" r="4"></circle>
    <path d="M22 21v-2a4 4 0 0 0-3-3.87"></path>
    <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
  {:else if name === 'wardrobe'}
    <path d="M20.38 3.46 16 2a4 4 0 0 1-8 0L3.62 3.46a2 2 0 0 0-1.34 2.23l.58 3.5a2 2 0 0 0 1.55 1.63L7 11.5V20a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2v-8.5l2.59-.68a2 2 0 0 0 1.55-1.63l.58-3.5a2 2 0 0 0-1.34-2.23Z"></path>
  {:else if name === 'mods'}
    <path d="m7.5 4.27 9 5.15"></path>
    <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"></path>
    <path d="m3.3 7 8.7 5 8.7-5"></path>
    <path d="M12 22V12"></path>
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3"></circle>
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
  {:else if name === 'search'}
    <circle cx="11" cy="11" r="7"></circle>
    <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
  {:else if name === 'close' || name === 'clear'}
    <line x1="18" y1="6" x2="6" y2="18"></line>
    <line x1="6" y1="6" x2="18" y2="18"></line>
  {:else if name === 'minimize'}
    <line x1="5" y1="12" x2="19" y2="12"></line>
  {:else if name === 'maximize'}
    <rect x="5" y="5" width="14" height="14" rx="2"></rect>
  {:else if name === 'restore'}
    <rect x="7" y="7" width="12" height="12" rx="2"></rect>
    <path d="M5 15V5h10"></path>
  {:else if name === 'chevron-right'}
    <polyline points="9 18 15 12 9 6"></polyline>
  {:else if name === 'chevron-left'}
    <polyline points="15 18 9 12 15 6"></polyline>
  {:else if name === 'chevron-down'}
    <polyline points="6 9 12 15 18 9"></polyline>
  {:else if name === 'check'}
    <polyline points="20 6 9 17 4 12"></polyline>
  {:else if name === 'refresh'}
    <path d="M21.5 2v6h-6"></path>
    <path d="M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.19"></path>
  {:else if name === 'info'}
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="12" y1="16" x2="12" y2="12"></line>
    <line x1="12" y1="8" x2="12.01" y2="8"></line>
  {:else if name === 'warning'}
    <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"></path>
    <line x1="12" y1="9" x2="12" y2="13"></line>
    <line x1="12" y1="17" x2="12.01" y2="17"></line>
  {:else if name === 'error'}
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="15" y1="9" x2="9" y2="15"></line>
    <line x1="9" y1="9" x2="15" y2="15"></line>
  {:else if name === 'sun'}
    <circle cx="12" cy="12" r="4"></circle>
    <path d="M12 2v2"></path>
    <path d="M12 20v2"></path>
    <path d="m4.93 4.93 1.41 1.41"></path>
    <path d="m17.66 17.66 1.41 1.41"></path>
    <path d="M2 12h2"></path>
    <path d="M20 12h2"></path>
    <path d="m6.34 17.66-1.41 1.41"></path>
    <path d="m19.07 4.93-1.41 1.41"></path>
  {:else if name === 'moon'}
    <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"></path>
  {:else if name === 'folder'}
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
  {:else if name === 'cpu'}
    <rect x="4" y="4" width="16" height="16" rx="2"></rect>
    <rect x="9" y="9" width="6" height="6"></rect>
    <path d="M15 2v2"></path>
    <path d="M15 20v2"></path>
    <path d="M2 15h2"></path>
    <path d="M2 9h2"></path>
    <path d="M20 15h2"></path>
    <path d="M20 9h2"></path>
    <path d="M9 2v2"></path>
    <path d="M9 20v2"></path>
  {/if}
</svg>

<style>
  .apple-icon {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
  }
</style>
