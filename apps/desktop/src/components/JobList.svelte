<script lang="ts">
  import type { MoveTracker } from '../lib/moves';
  import type { ClusterSnapshot, NodeId } from '../lib/types';
  import JobCard from './JobCard.svelte';

  interface Props {
    snapshot: ClusterSnapshot;
    /** Display name per machine, including ones that just left. */
    names: Map<NodeId, string>;
    /** Colour slot per machine (`nodeSlots`). */
    slots: Map<NodeId, number>;
    moves: MoveTracker;
  }
  let { snapshot, names, slots, moves }: Props = $props();

  // Newest job first. Only it is shown in full: on a projector the audience
  // should not have to find the job that is running.
  const jobs = $derived([...snapshot.jobs].reverse());
  const latest = $derived(jobs[0]);
  const earlier = $derived(jobs.slice(1));
</script>

<section class="jobs" aria-label="Jobs">
  {#if latest}
    <JobCard job={latest} newest summaries={snapshot.summaries} {names} {slots} {moves} />
  {:else}
    <p class="card muted">No jobs yet. Start one above and it will be split across the machines.</p>
  {/if}

  {#if earlier.length > 0}
    <details class="card earlier">
      <summary><h2>Earlier jobs ({earlier.length})</h2></summary>
      <div class="list">
        {#each earlier as job (job.job_id)}
          <JobCard {job} newest={false} summaries={snapshot.summaries} {names} {slots} {moves} />
        {/each}
      </div>
    </details>
  {/if}
</section>

<style>
  .jobs {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  p {
    margin: 0;
  }
  summary {
    cursor: pointer;
  }
  h2 {
    display: inline;
    margin: 0;
    font-size: 16px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 12px;
  }
</style>
