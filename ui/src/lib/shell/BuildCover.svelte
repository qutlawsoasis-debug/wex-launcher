<!--
  Build cover: the build's colour with its version set in large type
  (reference: .cover). No game art in the chrome.
-->
<script lang="ts">
  import { coverVar } from './builds.svelte';

  interface Props {
    id: string;
    size?: 'lib' | 'bar';
  }

  let { id, size = 'lib' }: Props = $props();
</script>

<span class="cover {size}" style:background={coverVar(id)} aria-hidden="true"><b>{id}</b></span>

<style>
  .cover {
    position: relative;
    display: grid;
    place-items: end start;
    overflow: hidden;
    color: var(--cover-ink);
    border-radius: var(--r-s);
    flex: none;
  }
  .cover b {
    font-weight: var(--w-display);
    letter-spacing: -0.04em;
    line-height: 0.8;
  }
  .lib {
    font-size: var(--t-cap);
    width: 40px;
    height: 40px;
    padding: 4px;
  }
  .lib b {
    font-size: 11px;
  }
  .bar {
    width: 48px;
    height: 48px;
    padding: 8px;
    animation: cover-in var(--dur-ind) var(--spring);
  }
  .bar b {
    font-size: 13px;
  }
  @keyframes cover-in {
    from {
      transform: scale(0.8) rotate(-6deg);
      opacity: 0;
    }
  }
</style>
