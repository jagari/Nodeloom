<script lang="ts">
  import { formatPercent } from '../lib/format';
  import { t } from '../lib/locale.svelte';
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

<!-- One small line next to the title: the map and list below carry the detail. -->
<dl class="kpis" aria-label={t('kpi.label')}>
  <div>
    <dt>{t('kpi.online')}</dt>
    <dd>{online}</dd>
  </div>
  <div>
    <dt>{t('kpi.running')}</dt>
    <dd>{running}</dd>
  </div>
  <div>
    <dt>{t('kpi.cpu')}</dt>
    <dd>{avgCpu === null ? '–' : formatPercent(avgCpu)}</dd>
  </div>
</dl>

<style>
  .kpis {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 20px;
    margin: 0;
  }
  .kpis div {
    display: flex;
    align-items: baseline;
    flex-direction: row-reverse;
    gap: 6px;
  }
  dt {
    color: var(--ink-2);
    font-size: 13px;
  }
  dd {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
</style>
