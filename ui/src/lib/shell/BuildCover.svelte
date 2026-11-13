<!--
  Build cover: the build's colour with its version set in large type
  (reference: .cover). No game art in the chrome.
-->
<script lang="ts">
  import { coverVar } from './builds.svelte';
  import { season } from '../design/season.svelte';

  interface Props {
    id: string;
    size?: 'lib' | 'bar';
  }

  let { id, size = 'lib' }: Props = $props();
</script>

<span class="cover {size}" style:background={coverVar(id)} aria-hidden="true"
  >{#if season.winter}<svg class="snowcap" viewBox="0 0 40 9" preserveAspectRatio="none"
      ><path d="M0 0h40v4.2c-2.2 0-2.6 3.6-5 3.6s-2.4-2.6-4.6-2.6-2.4 4-5.2 4-2.6-3.4-5-3.4-2.2 2.2-4.4 2.2S13 5 10.8 5 8.4 8.6 5.6 8.6 2.4 4.4 0 4.4z" /></svg
    >{/if}<b>{id}</b></span
>

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
  /* Winter look: snow lying on the tile (static, so it stays under Reduce Motion). */
  .snowcap {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 22%;
    fill: #fff;
    opacity: 0.92;
    filter: drop-shadow(0 1px 0.5px rgb(0 0 0 / 0.18));
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
