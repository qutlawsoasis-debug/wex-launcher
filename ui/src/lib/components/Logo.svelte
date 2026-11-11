<!--
  Wex Brand Mark Component
  
  Specification: SPEC 1.1 & Design System
  Easy to replace with an official SVG/PNG asset when finalized.
  Just edit or replace this file.
-->
<script lang="ts">
  import { t } from '../i18n';

  interface Props {
    size?: 'small' | 'medium' | 'large' | 'hero';
    showSubtitle?: boolean;
    subtitle?: string;
  }

  let {
    size = 'medium',
    showSubtitle = false,
    subtitle = t('logo.subtitle'),
  }: Props = $props();
</script>

<div class="wex-brand {size}" aria-label="Wex">
  <div class="brand-wordmark">
    <!-- Clean, authoritative Apple typographic wordmark -->
    <span class="letter-w">W</span><span class="letter-ex">ex</span>
  </div>
  {#if showSubtitle}
    <span class="brand-subtitle">{subtitle}</span>
  {/if}
</div>

<style>
  .wex-brand {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    user-select: none;
  }

  .brand-wordmark {
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", var(--font-system);
    font-weight: 800;
    line-height: 1;
    letter-spacing: -0.04em;
    color: var(--color-label);
    display: flex;
    align-items: baseline;
  }

  .letter-w {
    color: var(--color-system-blue);
  }

  .letter-ex {
    color: var(--color-label);
  }

  /* Size Presets */
  .small .brand-wordmark {
    font-size: 1.25rem;
    letter-spacing: -0.03em;
  }

  .medium .brand-wordmark {
    font-size: 2.25rem;
    letter-spacing: -0.04em;
  }

  .large .brand-wordmark {
    font-size: 3.5rem;
    letter-spacing: -0.05em;
  }

  .hero .brand-wordmark {
    font-size: 4.75rem;
    letter-spacing: -0.06em;
  }

  .brand-subtitle {
    margin-top: 0.35rem;
    font-family: var(--font-system);
    font-size: var(--font-size-caption1);
    font-weight: 500;
    color: var(--color-secondary-label);
  }
</style>
