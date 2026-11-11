<!--
  Apple HIG Empty State Component (Мягкие пустые состояния)
  Specification: tasks/apple/hig/empty-states.md & Apple HIG Human Interface Guidelines
  Rules:
  - Soft, graceful entrance animation (scale 0.98 -> 1, opacity 0 -> 1)
  - Clear visual hierarchy: Icon in gentle circular tint, Title, descriptive Help Text, optional Primary Action
  - Never fabricate fake data; explain authentically how to populate the view
  - Reduce Motion support: instant opacity entrance without scale/transform
-->
<script lang="ts">
  import type { Snippet, ComponentProps } from 'svelte';
  import Icon from './icons/Icon.svelte';
  import Button from './Button.svelte';
  import { theme } from '../design/theme.svelte';

  type IconName = ComponentProps<typeof Icon>['name'];

  interface Props {
    icon?: IconName;
    title: string;
    description?: string;
    actionText?: string;
    actionIcon?: IconName;
    onaction?: () => void;
    compact?: boolean;
    children?: Snippet;
  }

  let {
    icon = 'info',
    title,
    description,
    actionText,
    actionIcon,
    onaction,
    compact = false,
    children,
  }: Props = $props();
</script>

<div
  class="apple-empty-state"
  class:compact
  class:reduce-motion={theme.motionReduced}
  role="region"
  aria-label={title}
  data-testid="empty-state"
>
  {#if icon}
    <div class="empty-icon-bubble" aria-hidden="true">
      <Icon name={icon} size={compact ? 24 : 36} color="var(--color-tertiary-label)" />
    </div>
  {/if}

  <div class="empty-text-group">
    <h3 class="empty-headline">{title}</h3>
    {#if description}
      <p class="empty-description">{description}</p>
    {/if}
  </div>

  {#if actionText && onaction}
    <div class="empty-action-wrap">
      <Button variant="secondary" size={compact ? 'small' : 'regular'} onclick={onaction}>
        {#if actionIcon}
          <Icon name={actionIcon} size={14} color="var(--color-label)" />
        {/if}
        <span>{actionText}</span>
      </Button>
    </div>
  {/if}

  {@render children?.()}
</div>

<style>
  @keyframes softEmptyEnter {
    from {
      opacity: 0;
      transform: scale(0.975) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .apple-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 36px 20px;
    margin: auto;
    width: 100%;
    max-width: 480px;
    box-sizing: border-box;
    animation: softEmptyEnter var(--motion-duration-normal, 280ms) var(--motion-curve-spring-gentle, cubic-bezier(0.16, 1, 0.3, 1)) both;
    user-select: none;
  }

  .apple-empty-state.compact {
    padding: 20px 14px;
    max-width: 360px;
  }

  .empty-icon-bubble {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background-color: var(--color-quaternary-system-fill, rgba(120, 120, 128, 0.12));
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
    transition: transform var(--motion-duration-hover, 180ms) ease;
  }

  .compact .empty-icon-bubble {
    width: 48px;
    height: 48px;
    margin-bottom: 12px;
  }

  .empty-text-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 420px;
  }

  .empty-headline {
    font-size: var(--text-headline, 17px);
    font-weight: 600;
    color: var(--color-label);
    margin: 0;
    letter-spacing: -0.015em;
  }

  .compact .empty-headline {
    font-size: var(--text-subheadline, 15px);
  }

  .empty-description {
    font-size: var(--text-subheadline, 14px);
    color: var(--color-secondary-label);
    line-height: 1.42;
    margin: 0;
  }

  .compact .empty-description {
    font-size: var(--text-footnote, 13px);
  }

  .empty-action-wrap {
    margin-top: 18px;
    display: flex;
    justify-content: center;
  }

  .compact .empty-action-wrap {
    margin-top: 12px;
  }

  /* Reduce Motion Support */
  @media (prefers-reduced-motion: reduce) {
    .apple-empty-state {
      animation: none !important;
      opacity: 1;
      transform: none !important;
    }
  }

  :global([data-reduce-motion='true']) .apple-empty-state {
    animation: none !important;
    opacity: 1;
    transform: none !important;
  }
</style>
