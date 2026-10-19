<script lang="ts">
  import { summarizeJob, taskFraction } from '../lib/jobs';
  import { displayNames } from '../lib/names';
  import type { ClusterSnapshot } from '../lib/types';
  import Meter from './Meter.svelte';
  import StatusBadge from './StatusBadge.svelte';

  let { snapshot }: { snapshot: ClusterSnapshot } = $props();

  const names = $derived(displayNames(snapshot.nodes.map((n) => n.info)));
  // Newest job first.
  const jobs = $derived([...snapshot.jobs].reverse());
</script>

<section>
  <h2>Jobs</h2>
  {#each jobs as job (job.job_id)}
    {@const s = summarizeJob(job)}
    <article class="card job">
      <header>
        <div>
          <strong>Job {job.job_id + 1}</strong>
          <span class="muted"> · {s.totalFrames} frames on {job.tasks.length} task{job.tasks.length === 1 ? '' : 's'}</span>
        </div>
        <StatusBadge state={s.state} />
      </header>
      <Meter label="Progress" value={s.fraction * 100} severity={false} detail="{s.doneFrames} of {s.totalFrames} frames" />
      <table>
        <thead>
          <tr><th>Machine</th><th>Frames</th><th class="prog">Progress</th><th>Status</th></tr>
        </thead>
        <tbody>
          {#each job.tasks as t (t.task_id)}
            <tr>
              <td>{t.node ? (names.get(t.node) ?? t.node) : '–'}</td>
              <td class="nums">{t.frame_start}–{t.frame_end}</td>
              <td class="prog">
                <Meter label="Task {t.task_id + 1} progress" value={taskFraction(t) * 100} severity={false} height={6} />
              </td>
              <td>
                <StatusBadge state={t.state} />
                {#if t.error}<div class="err muted">{t.error}</div>{/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </article>
  {:else}
    <p class="muted">No jobs yet. Start one above and it will be split across the machines.</p>
  {/each}
</section>

<style>
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .job {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 12px;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--ink-2);
    font-size: 12px;
  }
  th,
  td {
    padding: 6px 8px 6px 0;
    border-top: 1px solid var(--grid);
    vertical-align: middle;
  }
  th {
    border-top: none;
  }
  .nums {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .prog {
    width: 38%;
  }
  .prog :global(.row) {
    display: none;
  }
  .err {
    font-size: 12px;
    max-width: 320px;
    overflow-wrap: anywhere;
  }
</style>
