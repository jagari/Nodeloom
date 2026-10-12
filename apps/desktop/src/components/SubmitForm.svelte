<script lang="ts">
  import type { JobView, NodeView, RunnerKind, TaskPayload } from '../lib/types';

  interface Props {
    nodes: NodeView[];
    submit: (payload: TaskPayload) => Promise<JobView>;
  }
  let { nodes, submit }: Props = $props();

  const available = $derived(
    (['blender', 'fake'] as RunnerKind[]).filter((r) => nodes.some((n) => n.info.runners.includes(r))),
  );

  let runner = $state<RunnerKind>('blender');
  let blendFile = $state('scene.blend');
  let frameStart = $state(1);
  let frameEnd = $state(250);
  let millis = $state(50);
  let busy = $state(false);
  let error = $state('');

  // Follow what the cluster can actually run.
  $effect(() => {
    if (available.length > 0 && !available.includes(runner)) runner = available[0]!;
  });

  async function onsubmit(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    busy = true;
    try {
      await submit(
        runner === 'blender'
          ? { runner, blend_file: blendFile.trim(), frame_start: frameStart, frame_end: frameEnd }
          : { runner, frame_start: frameStart, frame_end: frameEnd, millis_per_frame: millis },
      );
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<form class="card" {onsubmit}>
  <h2>Start a job</h2>
  <div class="grid">
    <label class="field">
      <span>Runner</span>
      <select bind:value={runner} disabled={available.length === 0}>
        {#each available as r (r)}
          <option value={r}>{r === 'blender' ? 'Blender render' : 'Test (sleep only)'}</option>
        {:else}
          <option>No machine online</option>
        {/each}
      </select>
    </label>
    {#if runner === 'blender'}
      <label class="field wide">
        <span>.blend file (relative to each machine’s work folder)</span>
        <input bind:value={blendFile} required spellcheck="false" />
      </label>
    {:else}
      <label class="field">
        <span>Milliseconds per frame</span>
        <input type="number" min="0" bind:value={millis} required />
      </label>
    {/if}
    <label class="field">
      <span>First frame</span>
      <input type="number" min="0" bind:value={frameStart} required />
    </label>
    <label class="field">
      <span>Last frame</span>
      <input type="number" min="0" bind:value={frameEnd} required />
    </label>
  </div>
  <div class="actions">
    <button class="btn primary" type="submit" disabled={busy || available.length === 0}>
      {busy ? 'Starting…' : 'Split across machines'}
    </button>
    {#if error}<span class="error" role="alert">{error}</span>{/if}
  </div>
</form>

<style>
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px;
  }
  .wide {
    grid-column: span 2;
  }
  @media (max-width: 520px) {
    .wide {
      grid-column: auto;
    }
  }
  .actions {
    margin-top: 14px;
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }
  .error {
    color: var(--ink);
    display: inline-flex;
    gap: 6px;
  }
  .error::before {
    content: '✕';
    color: var(--critical);
    font-weight: 700;
  }
</style>
