<script lang="ts">
  import type { Snippet } from 'svelte';
  import { splitSlots } from '../lib/i18n';

  // A translated sentence with markup in it: each `{name}` left in the text
  // is drawn by the snippet of that name, so word order stays the
  // translation's own (code, links and bold words move with it).
  let { text, ...slots }: { text: string; [slot: string]: Snippet | string } = $props();

  const segments = $derived(splitSlots(text));
  const slotOf = (name: string) => {
    const s = slots[name];
    return typeof s === 'function' ? s : null;
  };
</script>

{#each segments as seg, i (i)}{#if 'slot' in seg}{@const s = slotOf(seg.slot)}{#if s}{@render s()}{:else}{`{${seg.slot}}`}{/if}{:else}{seg.text}{/if}{/each}
