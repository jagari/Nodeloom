<script lang="ts">
  import { DEMO_JOB } from '../lib/jobs';
  import { availableModels, llmJob, parsePrompts, promptProblem, SAMPLE_PROMPTS } from '../lib/llm';
  import type { JobView, NodeView, TaskPayload } from '../lib/types';

  interface Props {
    nodes: NodeView[];
    submit: (payload: TaskPayload) => Promise<JobView>;
  }
  let { nodes, submit }: Props = $props();

  const available = $derived(
    (['blender', 'fake'] as const).filter((r) => nodes.some((n) => n.info.runners.includes(r))),
  );

  let runner = $state<'blender' | 'fake'>('blender');
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

  // Local AI: models installed on the connected machines.
  const models = $derived(availableModels(nodes));
  let model = $state('');
  $effect(() => {
    if (models.length > 0 && !models.some((m) => m.model === model)) model = models[0]!.model;
  });
  let promptText = $state(SAMPLE_PROMPTS.join('\n'));
  const prompts = $derived(parsePrompts(promptText));
  const problem = $derived(promptProblem(prompts));
  const aiMachines = $derived(models.find((m) => m.model === model)?.machines ?? 0);

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

<form class="card start" {onsubmit}>
  <div class="col">
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
  </div>
  <div class="demo ai">
    <div class="ai-text">
      <h3>Ask the local AI</h3>
      {#if models.length === 0}
        <p class="muted">
          No machine runs a local LLM yet. Install <a href="https://ollama.com/download" target="_blank" rel="noreferrer"
            >Ollama</a
          >
          and a model on the machines (for example <code>ollama pull llama3.2:1b</code>), then restart Nodeloom there.
        </p>
      {:else}
        <p class="muted">
          {prompts.length} question{prompts.length === 1 ? '' : 's'} split across {aiMachines} machine{aiMachines === 1
            ? ''
            : 's'}; each answers with its own model.
        </p>
        <div class="ai-fields">
          <label class="field">
            <!-- The options already say "model · N machines". -->
            <span class="sr-only">Model</span>
            <select bind:value={model}>
              {#each models as m (m.model)}
                <option value={m.model}>{m.model} · {m.machines} machine{m.machines === 1 ? '' : 's'}</option>
              {/each}
            </select>
          </label>
          <button
            class="btn primary big"
            type="button"
            disabled={busy || !!problem || !model}
            onclick={() => send(llmJob(model, prompts))}
          >
            {busy ? 'Starting…' : 'Ask the AI'}
          </button>
        </div>
        <details class="questions">
          <summary>Questions ({prompts.length})</summary>
          <label class="field">
            <span>One question per line</span>
            <textarea rows="8" bind:value={promptText} spellcheck="false"></textarea>
          </label>
        </details>
        {#if problem}<p class="error" role="alert">{problem}</p>{/if}
      {/if}
    </div>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</form>

<style>
  /* Demo and local AI side by side, so the job they start stays on screen. */
  .start {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    column-gap: 24px;
  }
  .start > .error {
    grid-column: 1 / -1;
  }
  @media (max-width: 960px) {
    .start {
      grid-template-columns: minmax(0, 1fr);
    }
  }
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
  .demo > div {
    flex: 1 1 220px;
  }
  .big {
    font-size: 16px;
    padding: 10px 22px;
  }
  .ai {
    padding-left: 24px;
    border-left: 1px solid var(--grid);
    align-items: flex-start;
  }
  @media (max-width: 960px) {
    .ai {
      margin-top: 14px;
      padding: 14px 0 0;
      border-left: 0;
      border-top: 1px solid var(--grid);
    }
  }
  .ai-text {
    flex: 1 1 240px;
    min-width: 0;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 15px;
  }
  /* The button sits by the model, so the explanation above keeps one line. */
  .ai-fields {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 8px;
    align-items: flex-end;
  }
  .ai-fields .field {
    flex: 1 1 200px;
  }
  .questions summary {
    margin-bottom: 6px;
  }
  textarea {
    width: 100%;
    background: var(--page);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 10px;
    font: inherit;
    color: inherit;
    resize: vertical;
  }
  code {
    background: var(--page);
    padding: 1px 5px;
    border-radius: 4px;
  }
  details {
    margin-top: 12px;
    border-top: 1px solid var(--grid);
    padding-top: 10px;
  }
  summary {
    cursor: pointer;
    color: var(--ink-2);
  }
  details[open] > summary {
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
