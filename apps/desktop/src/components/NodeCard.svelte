<script lang="ts">
  import { formatBytes, formatPercent } from '../lib/format';
  import type { Sample } from '../lib/history';
  import type { NodeView } from '../lib/types';
  import Meter from './Meter.svelte';
  import Sparkline from './Sparkline.svelte';

  let { node, name, samples }: { node: NodeView; name: string; samples: Sample[] } = $props();

  const m = $derived(node.metrics);
  const memPct = $derived(m && m.memory_total_bytes > 0 ? (m.memory_used_bytes / m.memory_total_bytes) * 100 : 0);
</script>

<article class="card node">
  <header>
    <div>
      <h3>{name}</h3>
      <div class="muted sub">{node.info.os} · {node.info.arch} · {node.info.cpu_cores} cores</div>
    </div>
    <span class="busy" class:active={node.busy_tasks > 0}>
      {node.busy_tasks > 0 ? `${node.busy_tasks} running` : 'Idle'}
    </span>
  </header>

  {#if m}
    <div class="cpu">
      <div class="cpu-head">
        <span class="muted">CPU</span>
        <span class="big">{formatPercent(m.cpu_percent)}</span>
      </div>
      <Sparkline {samples} field="cpu" label="CPU usage of {name}" />
    </div>
    <Meter
      label="Memory"
      value={memPct}
      detail="{formatBytes(m.memory_used_bytes)} of {formatBytes(m.memory_total_bytes)}"
    />
  {:else}
    <p class="muted">Waiting for the first report…</p>
  {/if}

  <footer>
    {#each node.info.runners as runner (runner)}
      <span class="chip">{runner}</span>
    {:else}
      <span class="muted">No runners available</span>
    {/each}
  </footer>
</article>

<style>
  .node {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: flex-start;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    overflow-wrap: anywhere;
  }
  .sub {
    font-size: 12px;
  }
  .busy {
    font-size: 12px;
    color: var(--ink-2);
    white-space: nowrap;
  }
  .busy.active {
    color: var(--ink);
    font-weight: 600;
  }
  .cpu-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 4px;
  }
  .big {
    font-size: 22px;
    font-weight: 600;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 9px;
    font-size: 12px;
    color: var(--ink-2);
  }
  p {
    margin: 0;
  }
</style>
