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
      ><path d="M0 0h40v3.6c-2.6 0-3.2 3-6.2 3S30.2 4.4 27.4 4.4s-3.6 3.6-7 3.6-3.4-3-6.4-3-3.2 2.4-6.2 2.4S3.4 3.8 0 3.8z" /></svg
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
