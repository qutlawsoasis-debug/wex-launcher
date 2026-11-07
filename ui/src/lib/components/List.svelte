<!--
  Apple HIG List (Grouped) Component
  Specification: tasks/apple/hig/lists-and-tables.md
  Rules:
  - Inset grouped styling with card-like rounded corners
  - Optional header and footer text
  - Separates distinct sections of content
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    header?: string;
    footer?: string;
    inset?: boolean;
    children?: Snippet;
  }

  let { header, footer, inset = true, children }: Props = $props();
</script>

<div class="apple-list-section" class:inset>
  {#if header}
    <div class="list-header">{header}</div>
  {/if}

  <div class="list-container">
    {@render children?.()}
  </div>

  {#if footer}
    <div class="list-footer">{footer}</div>
  {/if}
</div>

<style>
  .apple-list-section {
    display: flex;
    flex-direction: column;
    margin-bottom: 20px;
    width: 100%;
  }

  .list-header {
    font-size: var(--text-caption-1);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-secondary-label);
    padding: 0 16px 6px;
  }

  .list-footer {
    font-size: var(--text-caption-1);
    color: var(--color-secondary-label);
    padding: 6px 16px 0;
    line-height: var(--line-caption-1);
  }

  .list-container {
    background-color: var(--card-bg);
    border: 1px solid var(--glass-border);
    overflow: hidden;
  }

  .apple-list-section.inset .list-container {
    border-radius: var(--radius-lg);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
  }
</style>
