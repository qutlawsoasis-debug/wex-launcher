<!--
  Apple HIG Sidebar Component
  Specification: tasks/apple/hig/sidebars.md
  Rules:
  - Floats in Liquid Glass functional layer
  - Distinct navigation destinations with SF-style symbols
  - Active selection indicator with accent color or prominent fill
  - Clean layout without redundant category headers when items form a single primary list
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
      | 'lobby'
      | 'friends'
      | 'wardrobe'
      | 'mods'
      | 'settings'
      | 'workshop';
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

  let activeIndex = $derived(items.findIndex((item) => item.id === activeId));
  let buttonEls: HTMLButtonElement[] = $state([]);

  function selectItem(id: string) {
    activeId = id;
    onselect?.(id);
  }

  function handleKeyDown(e: KeyboardEvent, index: number) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const nextIndex = (index + 1) % items.length;
      buttonEls[nextIndex]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prevIndex = (index - 1 + items.length) % items.length;
      buttonEls[prevIndex]?.focus();
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      selectItem(items[index].id);
    }
  }
</script>

<aside class="apple-sidebar" class:collapsed aria-label="Основная навигация">
  <div class="sidebar-header" data-tauri-drag-region>
    <div class="app-brand">
      <div class="brand-glyph">
        <Icon name="play" size={14} color="#ffffff" />
      </div>
      {#if !collapsed}
        <div class="brand-text">
          <span class="brand-title">Wex</span>
        </div>
      {/if}
    </div>
  </div>

  <nav class="sidebar-nav">
    {#if activeIndex >= 0}
      <div
        class="nav-active-capsule"
        style="transform: translateY({activeIndex * 40}px);"
        aria-hidden="true"
      ></div>
    {/if}
    <ul class="nav-list">
      {#each items as item, i (item.id)}
        {@const isActive = item.id === activeId}
        <li>
          <button
            type="button"
            bind:this={buttonEls[i]}
            class="nav-item"
            class:active={isActive}
            aria-current={isActive ? 'page' : undefined}
            aria-label={item.label}
            onclick={() => selectItem(item.id)}
            onkeydown={(e) => handleKeyDown(e, i)}
            title={collapsed ? item.label : undefined}
          >
            <span class="item-icon">
              <Icon
                name={item.icon}
                size={18}
                color={isActive ? 'var(--color-system-cyan)' : 'var(--color-secondary-label)'}
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
    align-items: center;
  }

  .brand-title {
    font-size: var(--text-title-3);
    font-weight: 700;
    color: var(--color-label);
    letter-spacing: -0.3px;
  }

  .sidebar-nav {
    position: relative;
    flex: 1;
    padding: 12px 10px;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .nav-active-capsule {
    position: absolute;
    top: 12px;
    left: 10px;
    right: 10px;
    height: 38px;
    border-radius: var(--radius-md);
    background-color: var(--color-system-fill);
    pointer-events: none;
    z-index: 0;
    transition:
      transform var(--motion-duration-morph) var(--motion-curve-spring-bouncy),
      opacity var(--motion-duration-short) ease;
    will-change: transform;
  }

  :global([data-theme="dark"]) .nav-active-capsule {
    background-color: rgba(255, 255, 255, 0.12);
  }

  .nav-list {
    position: relative;
    z-index: 1;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
  }

  .nav-item {
    position: relative;
    width: 100%;
    height: 38px;
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
      transform var(--motion-duration-press) var(--motion-curve-press);
    outline: none;
  }

  .nav-item::after {
    content: '';
    position: absolute;
    top: -3px;
    bottom: -3px;
    left: 0;
    right: 0;
  }

  .nav-item:focus-visible {
    outline: 2px solid var(--color-system-blue);
    outline-offset: -2px;
  }

  .nav-item:hover:not(.active) {
    background-color: var(--color-quaternary-system-fill);
  }

  .nav-item:active {
    transform: scale(var(--motion-press-scale, 0.97));
  }

  .nav-item.active {
    font-weight: 600;
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
