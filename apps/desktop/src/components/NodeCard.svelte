<script lang="ts">
  import { formatBytes, formatPercent } from '../lib/format';
  import type { Sample } from '../lib/history';
  import type { CurrentWork } from '../lib/jobs';
  import type { NodeView } from '../lib/types';
  import Meter from './Meter.svelte';
  import Sparkline from './Sparkline.svelte';

  interface Props {
    node: NodeView;
    name: string;
    samples: Sample[];
    /** Pieces this machine is running now (`workByNode`). */
    work?: CurrentWork[];
    /** Its colour on the job bars, so the two can be matched by eye. */
    color?: string;
  }
  let { node, name, samples, work = [], color = 'var(--muted)' }: Props = $props();

  const m = $derived(node.metrics);
  const memPct = $derived(m && m.memory_total_bytes > 0 ? (m.memory_used_bytes / m.memory_total_bytes) * 100 : 0);
</script>

<article class="card node">
  <header>
    <div>
      <h3><span class="swatch" style:--c={color} aria-hidden="true"></span>{name}</h3>
      <div class="muted sub">{node.info.os} · {node.info.arch} · {node.info.cpu_cores} cores</div>
    </div>
    <span class="busy" class:active={work.length > 0}>
      {#if work.length > 0}
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M5 3.5l7 4.5-7 4.5z" fill="currentColor" /></svg>
        Working
      {:else}
        Idle
      {/if}
    </span>
  </header>

  <div class="now" style:--c={color}>
    {#each work as w (w.task_id)}
      <div class="task">
        <div class="task-head">
          <span>Job {w.job_id + 1} · <strong>frames {w.frame_start}–{w.frame_end}</strong></span>
          <span class="task-pct">{Math.round(w.fraction * 100)}%</span>
        </div>
        <div
          class="task-track"
          role="progressbar"
          aria-label="Frames {w.frame_start}–{w.frame_end} on {name}"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={Math.round(w.fraction * 100)}
        >
          <div class="task-fill" style:width="{w.fraction * 100}%"></div>
        </div>
      </div>
    {:else}
      <p class="muted idle">Idle · waiting for work</p>
    {/each}
  </div>

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
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .swatch {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--c);
  }
  .now {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--page);
    border: 1px solid var(--border);
  }
  .task-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
    margin-bottom: 5px;
    font-variant-numeric: tabular-nums;
  }
  .task-pct {
    font-weight: 600;
  }
  .task-track {
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: color-mix(in srgb, var(--c) 22%, var(--surface));
  }
  .task-fill {
    height: 100%;
    background: var(--c);
    transition: width 0.4s ease;
  }
  .idle {
    font-size: 13px;
  }
  .sub {
    font-size: 12px;
  }
  .busy {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--ink-2);
    white-space: nowrap;
  }
  .busy.active {
    color: var(--ink);
    font-weight: 600;
  }
  .busy svg {
    color: var(--series-1);
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
