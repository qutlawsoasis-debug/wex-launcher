<!--
  Apple HIG Sidebar Component
  Specification: tasks/apple/hig/sidebars.md
  Rules:
  - Floats in Liquid Glass functional layer
  - Distinct navigation destinations with SF-style symbols
  - Active selection indicator with accent color or prominent fill
  - Concise group labels and badges
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './icons/Icon.svelte';

  export interface SidebarItem {
    id: string;
    label: string;
    icon:
      | 'play'
      | 'server'
      | 'friends'
      | 'mods'
      | 'settings';
    badge?: string | number;
  }

  interface Props {
    items: SidebarItem[];
    activeId: string;
    collapsed?: boolean;
    onselect?: (id: string) => void;
    footer?: Snippet;
  }

  let { items, activeId = $bindable('play'), collapsed = false, onselect, footer }: Props = $props();

  function selectItem(id: string) {
    activeId = id;
    onselect?.(id);
  }
</script>

<aside class="apple-sidebar" class:collapsed aria-label="Main Navigation">
  <div class="sidebar-header" data-tauri-drag-region>
    <div class="app-brand">
      <div class="brand-glyph">
        <Icon name="play" size={14} color="#ffffff" />
      </div>
      {#if !collapsed}
        <div class="brand-text">
          <span class="brand-title">Wex</span>
          <span class="brand-subtitle">Launcher</span>
        </div>
      {/if}
    </div>
  </div>

  <nav class="sidebar-nav">
    <div class="nav-section-title">{collapsed ? '•••' : 'Навигация'}</div>
    <ul class="nav-list">
      {#each items as item}
        {@const isActive = item.id === activeId}
        <li>
          <button
            type="button"
            class="nav-item"
            class:active={isActive}
            onclick={() => selectItem(item.id)}
            title={collapsed ? item.label : undefined}
          >
            <span class="item-icon">
              <Icon
                name={item.icon}
                size={18}
                color={isActive ? 'var(--color-system-blue)' : 'var(--color-secondary-label)'}
              />
            </span>

            {#if !collapsed}
              <span class="item-label">{item.label}</span>

              {#if item.badge}
                <span class="item-badge">{item.badge}</span>
              {/if}
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  {#if footer}
    <div class="sidebar-footer">
      {@render footer()}
    </div>
  {/if}
</aside>

<style>
  .apple-sidebar {
    width: 220px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: var(--glass-sidebar-bg);
    backdrop-filter: blur(28px) saturate(190%);
    border-right: 1px solid var(--glass-border);
    user-select: none;
    transition: width var(--motion-duration-normal) var(--motion-spring-ease);
    flex-shrink: 0;
    z-index: 10;
  }

  .apple-sidebar.collapsed {
    width: 68px;
  }

  .sidebar-header {
    height: 52px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--glass-border);
  }

  .app-brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .brand-glyph {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: linear-gradient(135deg, #0091ff, #005bb5);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 6px rgba(0, 145, 255, 0.4);
    flex-shrink: 0;
  }

  .brand-text {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .brand-title {
    font-size: var(--text-headline);
    font-weight: 700;
    color: var(--color-label);
    letter-spacing: -0.3px;
  }

  .brand-subtitle {
    font-size: var(--text-caption-2);
    font-weight: 500;
    color: var(--color-secondary-label);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .sidebar-nav {
    flex: 1;
    padding: 12px 10px;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .nav-section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-tertiary-label);
    padding: 4px 8px 8px;
  }

  .nav-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    width: 100%;
    height: 36px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    border-radius: var(--radius-md);
    background-color: transparent;
    cursor: pointer;
    font-family: var(--font-system);
    font-size: var(--text-subheadline);
    font-weight: 500;
    color: var(--color-label);
    text-align: left;
    transition:
      background-color var(--motion-duration-short) ease,
      transform var(--motion-duration-short) var(--motion-spring-ease);
    outline: none;
  }

  .nav-item:hover:not(.active) {
    background-color: var(--color-quaternary-system-fill);
  }

  .nav-item:active {
    transform: scale(0.98);
  }

  .nav-item.active {
    background-color: var(--color-system-fill);
    font-weight: 600;
  }

  :global([data-theme="dark"]) .nav-item.active {
    background-color: rgba(255, 255, 255, 0.12);
  }

  .item-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-badge {
    font-size: var(--text-caption-2);
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    background-color: var(--color-system-blue);
    color: #ffffff;
  }

  .sidebar-footer {
    padding: 12px;
    border-top: 1px solid var(--glass-border);
  }
</style>
