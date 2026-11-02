<script lang="ts">
  import { errorText } from '../lib/i18n';
  import { DEMO_JOB } from '../lib/jobs';
  import { availableModels, llmJob, parsePrompts, promptProblem, SAMPLE_PROMPTS } from '../lib/llm';
  import { t } from '../lib/locale.svelte';
  import type { JobView, NodeView, TaskPayload } from '../lib/types';
  import Rich from './Rich.svelte';

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
  // Kept as caught, so the text follows a language switch.
  let error = $state<unknown>(null);

  // Follow what the cluster can actually run.
  $effect(() => {
    if (available.length > 0 && !available.includes(runner)) runner = available[0]!;
  });

  // Machines able to run the demo (the test runner is opt-in per agent).
  const demoMachines = $derived(nodes.filter((n) => n.info.runners.includes('fake')).length);
  const demoBlocked = $derived(
    nodes.length === 0
      ? t('start.noMachine')
      : demoMachines === 0
        ? t('start.noFake', { flag: '--enable-fake-runner' })
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
  // The sample questions are content, not interface: they stay as written.
  let promptText = $state(SAMPLE_PROMPTS.join('\n'));
  const prompts = $derived(parsePrompts(promptText));
  const problem = $derived(promptProblem(prompts, t));
  const aiMachines = $derived(models.find((m) => m.model === model)?.machines ?? 0);

  async function send(payload: TaskPayload) {
    error = null;
    busy = true;
    try {
      await submit(payload);
    } catch (e) {
      error = e;
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
        <h2>{t('start.title')}</h2>
        <p class="muted">
          {#if demoBlocked}
            {demoBlocked}
          {:else}
            {t('start.demoHint', { machines: t('count.machine', { count: demoMachines }), seconds: demoSeconds })}
          {/if}
        </p>
      </div>
      <button class="btn primary big" type="button" disabled={busy || !!demoBlocked} onclick={() => send(DEMO_JOB)}>
        {busy ? t('common.starting') : t('start.demo')}
      </button>
    </div>
    <details>
      <summary>{t('custom.title')}</summary>
      <div class="grid">
        <label class="field">
          <span>{t('custom.runner')}</span>
          <select bind:value={runner} disabled={available.length === 0}>
            {#each available as r (r)}
              <option value={r}>{r === 'blender' ? t('custom.blender') : t('custom.fake')}</option>
            {:else}
              <option>{t('custom.addFirst')}</option>
            {/each}
          </select>
        </label>
        {#if runner === 'blender'}
          <label class="field wide">
            <span>{t('custom.blendFile')}</span>
            <input bind:value={blendFile} required spellcheck="false" />
          </label>
        {:else}
          <label class="field">
            <span>{t('custom.millis')}</span>
            <input type="number" min="0" bind:value={millis} required />
          </label>
        {/if}
        <label class="field">
          <span>{t('custom.first')}</span>
          <input type="number" min="0" bind:value={frameStart} required />
        </label>
        <label class="field">
          <span>{t('custom.last')}</span>
          <input type="number" min="0" bind:value={frameEnd} required />
        </label>
      </div>
      <div class="actions">
        <button class="btn primary" type="submit" disabled={busy || available.length === 0}>
          {busy ? t('common.starting') : t('custom.submit')}
        </button>
      </div>
    </details>
  </div>
  <div class="demo ai">
    <div class="ai-text">
      <h3>{t('ai.title')}</h3>
      {#if models.length === 0}
        <p class="muted">
          <Rich text={t('ai.none')}>
            {#snippet ollama()}<a href="https://ollama.com/download" target="_blank" rel="noreferrer">Ollama</a>{/snippet}
            {#snippet command()}<code>ollama pull llama3.2:1b</code>{/snippet}
          </Rich>
        </p>
      {:else}
        <p class="muted">
          {t('ai.hint', {
            questions: t('count.question', { count: prompts.length }),
            machines: t('count.machine', { count: aiMachines }),
          })}
        </p>
        <div class="ai-fields">
          <label class="field">
            <!-- The options already say "model · N machines". -->
            <span class="sr-only">{t('ai.model')}</span>
            <select bind:value={model}>
              {#each models as m (m.model)}
                <option value={m.model}>{m.model} · {t('count.machine', { count: m.machines })}</option>
              {/each}
            </select>
          </label>
          <button
            class="btn primary big"
            type="button"
            disabled={busy || !!problem || !model}
            onclick={() => send(llmJob(model, prompts))}
          >
            {busy ? t('common.starting') : t('ai.ask')}
          </button>
        </div>
        <details class="questions">
          <summary>{t('ai.questions', { count: prompts.length })}</summary>
          <label class="field">
            <span>{t('ai.perLine')}</span>
            <textarea rows="8" bind:value={promptText} spellcheck="false"></textarea>
          </label>
        </details>
        {#if problem}<p class="error" role="alert">{problem}</p>{/if}
      {/if}
    </div>
  </div>
  {#if error}<p class="error" role="alert">{errorText(error, t)}</p>{/if}
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
