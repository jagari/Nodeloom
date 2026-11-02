<script lang="ts">
  import { DEMO_JOB } from '../lib/jobs';
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

  // Machines able to run the demo (the test runner is opt-in per agent).
  const demoMachines = $derived(nodes.filter((n) => n.info.runners.includes('fake')).length);
  const demoBlocked = $derived(
    nodes.length === 0
      ? 'Add a machine first.'
      : demoMachines === 0
        ? 'No machine has the test runner. Start agents with --enable-fake-runner.'
        : '',
  );
  // 600 frames × 50 ms, shared evenly: what the audience will wait.
  const demoSeconds = $derived(
    demoMachines === 0 ? 0 : Math.ceil((DEMO_JOB.frame_end - DEMO_JOB.frame_start + 1) * 0.05 / demoMachines),
  );

  async function send(payload: TaskPayload) {
    error = '';
    busy = true;
    try {
      await submit(payload);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function onsubmit(event: SubmitEvent) {
    event.preventDefault();
    void send(
      runner === 'blender'
        ? { runner, blend_file: blendFile.trim(), frame_start: frameStart, frame_end: frameEnd }
        : { runner, frame_start: frameStart, frame_end: frameEnd, millis_per_frame: millis },
    );
  }
</script>

<form class="card" {onsubmit}>
  <div class="demo">
    <div>
      <h2>Start a job</h2>
      <p class="muted">
        {#if demoBlocked}
          {demoBlocked}
        {:else}
          Demo job: 600 test frames split across {demoMachines} machine{demoMachines === 1 ? '' : 's'}, about {demoSeconds}
          s. A test that only waits; nothing is rendered.
        {/if}
      </p>
    </div>
    <button class="btn primary big" type="button" disabled={busy || !!demoBlocked} onclick={() => send(DEMO_JOB)}>
      {busy ? 'Starting…' : 'Demo job'}
    </button>
  </div>
  <details>
    <summary>Custom job</summary>
    <div class="grid">
      <label class="field">
        <span>Runner</span>
        <select bind:value={runner} disabled={available.length === 0}>
          {#each available as r (r)}
            <option value={r}>{r === 'blender' ? 'Blender render' : 'Test (sleep only)'}</option>
          {:else}
            <option>Add a machine first</option>
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
    </div>
  </details>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</form>

<style>
  h2 {
    margin: 0 0 4px;
    font-size: 16px;
  }
  .demo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }
  .demo p {
    margin: 0;
  }
  .big {
    font-size: 16px;
    padding: 10px 22px;
  }
  details {
    margin-top: 12px;
    border-top: 1px solid var(--grid);
    padding-top: 10px;
  }
  summary {
    cursor: pointer;
    color: var(--ink-2);
    margin-bottom: 10px;
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
    margin: 12px 0 0;
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
