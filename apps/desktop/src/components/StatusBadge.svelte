<script lang="ts">
  import type { JobState } from '../lib/jobs';

  let { state }: { state: JobState } = $props();

  const labels: Record<JobState, string> = {
    pending: 'Queued',
    running: 'Running',
    done: 'Done',
    failed: 'Failed',
  };
</script>

<!-- Status is always icon + label, never color alone. -->
<span class="badge {state}">
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    {#if state === 'done'}
      <path d="M3 8.5l3.2 3.2L13 4.8" />
    {:else if state === 'failed'}
      <path d="M4 4l8 8M12 4l-8 8" />
    {:else if state === 'running'}
      <path d="M5 3.5l7 4.5-7 4.5z" fill="currentColor" stroke="none" />
    {:else}
      <circle cx="8" cy="8" r="5.5" /><path d="M8 5v3.3l2 1.2" />
    {/if}
  </svg>
  <span class="label">{labels[state]}</span>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .label {
    color: var(--ink);
  }
  .running svg {
    color: var(--series-1);
  }
  .done svg {
    color: var(--good);
  }
  .failed svg {
    color: var(--critical);
  }
  .pending svg {
    color: var(--muted);
  }
</style>
