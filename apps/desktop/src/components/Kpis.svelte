<script lang="ts">
  import { formatPercent } from '../lib/format';
  import type { ClusterSnapshot } from '../lib/types';

  let { snapshot }: { snapshot: ClusterSnapshot } = $props();

  const online = $derived(snapshot.nodes.length);
  const running = $derived(snapshot.nodes.reduce((sum, n) => sum + n.busy_tasks, 0));
  const reporting = $derived(snapshot.nodes.filter((n) => n.metrics));
  const avgCpu = $derived(
    reporting.length === 0
      ? null
      : reporting.reduce((sum, n) => sum + n.metrics!.cpu_percent, 0) / reporting.length,
  );
</script>

<section class="kpis" aria-label="Cluster summary">
  <div class="card tile">
    <div class="label">Machines online</div>
    <div class="value">{online}</div>
  </div>
  <div class="card tile">
    <div class="label">Tasks running</div>
    <div class="value">{running}</div>
  </div>
  <div class="card tile">
    <div class="label">Average CPU</div>
    <div class="value">{avgCpu === null ? '–' : formatPercent(avgCpu)}</div>
  </div>
</section>

<style>
  .kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 12px;
  }
  .label {
    color: var(--ink-2);
    font-size: 12px;
  }
  .value {
    font-size: 28px;
    font-weight: 600;
    margin-top: 2px;
  }
</style>
