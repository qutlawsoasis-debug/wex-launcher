<!-- First letter on a system colour, used where there is no real icon or skin (reference: .mono). -->
<script lang="ts">
  interface Props {
    name: string;
    shape?: 'round' | 'square' | 'large';
    /** Fixed colour 1–5 (--av-N); otherwise picked from the name so it stays stable. */
    tone?: number;
  }

  let { name, shape = 'round', tone: fixed }: Props = $props();

  const letter = $derived(name.replace(/[^\p{L}\p{N}]/gu, '').slice(0, 1).toUpperCase() || '?');
  const tone = $derived.by(() => {
    if (fixed && fixed >= 1 && fixed <= 5) return fixed;
    let h = 2166136261;
    for (const c of name) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
    return ((h >>> 0) % 5) + 1;
  });
</script>

<span class="mono {shape}" style:background="var(--av-{tone})" aria-hidden="true">{letter}</span>

<style>
  .mono {
    display: grid;
    place-items: center;
    color: var(--cover-ink);
    font-weight: var(--w-bold);
    border-radius: 50%;
    flex: none;
    width: 36px;
    height: 36px;
    font-size: var(--t-sub);
  }
  .square {
    border-radius: var(--r-s);
    width: 40px;
    height: 40px;
  }
  .large {
    border-radius: var(--r-m);
    width: 56px;
    height: 56px;
    font-size: 22px;
  }
</style>
