<!-- Page heading (reference: .page-title). HIG › Typography: one large title per page. -->
<script lang="ts">
  interface Props {
    title: string;
    subtitle?: string;
  }

  let { title, subtitle }: Props = $props();
</script>

<div class="page-title">
  <h1>{title}</h1>
  {#if subtitle}<p>{subtitle}</p>{/if}
</div>

<style>
  h1 {
    margin: 0;
    font-size: var(--t-t1);
    font-weight: var(--w-display);
    letter-spacing: -0.02em;
  }
  p {
    margin: 4px 0 0;
    color: var(--label-2);
  }
</style>
