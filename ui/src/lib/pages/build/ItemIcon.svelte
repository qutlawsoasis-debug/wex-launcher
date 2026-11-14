<!--
  The picture at the start of a mod / pack / shader / world row: the item's own
  icon when it has one (a mod's icon from its jar), otherwise a neutral glyph
  for the kind of item on a gray fill. No letters and no colour coding: a
  colour must mean something (HIG › Color: use colour sparingly, controls are
  neutral; HIG › Icons: a glyph says what the item is). Pixel-art icons are
  scaled without smoothing so they stay crisp. Decorative: the row's name is the
  accessible label.
-->
<script lang="ts">
  interface Props {
    kind: 'mod' | 'pack' | 'shader' | 'world';
    src?: string | null;
  }
  let { kind, src = null }: Props = $props();
  let broken = $state(false);
  $effect(() => {
    void src;
    broken = false;
  });
</script>

{#if src && !broken}
  <img class="ic" {src} alt="" aria-hidden="true" draggable="false" onerror={() => (broken = true)} />
{:else}
  <span class="ic glyph" aria-hidden="true">
    <svg viewBox="0 0 24 24">
      {#if kind === 'mod'}
        <!-- A cube: a block of the game. -->
        <path d="M12 3.2 19.6 7.4v9.2L12 20.8 4.4 16.6V7.4z M4.4 7.4 12 11.6l7.6-4.2 M12 11.6v9.2" />
      {:else if kind === 'pack'}
        <!-- Stacked layers: textures over the game. -->
        <path d="M12 4 20 8.5 12 13 4 8.5z M4 12.5 12 17l8-4.5 M4 16.5 12 21l8-4.5" />
      {:else if kind === 'shader'}
        <!-- The sun: light and shadow. -->
        <circle cx="12" cy="12" r="3.6" /><path d="M12 3v2.2M12 18.8V21M3 12h2.2M18.8 12H21M5.6 5.6l1.6 1.6M16.8 16.8l1.6 1.6M5.6 18.4l1.6-1.6M16.8 7.2l1.6-1.6" />
      {:else}
        <!-- A globe: a world. -->
        <circle cx="12" cy="12" r="8.2" /><path d="M3.8 12h16.4M12 3.8c2.4 2.3 3.4 5 3.4 8.2s-1 5.9-3.4 8.2c-2.4-2.3-3.4-5-3.4-8.2s1-5.9 3.4-8.2z" />
      {/if}
    </svg>
  </span>
{/if}

<style>
  .ic {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: var(--r-s);
  }
  img.ic {
    object-fit: cover;
    image-rendering: pixelated;
    background: var(--fill-2);
  }
  .glyph {
    display: grid;
    place-items: center;
    background: var(--fill-2);
    color: var(--label-2);
  }
  .glyph svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
</style>
