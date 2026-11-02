<script lang="ts">
  import { availableModels } from '../lib/llm';
  import { countChars, documentProblem, estimateParts, readDocument } from '../lib/pdf';
  import { summaryProgress } from '../lib/summary';
  import type { ClusterSnapshot, SubmitSummary, SummaryView } from '../lib/types';
  import StatusBadge from './StatusBadge.svelte';

  interface Props {
    snapshot: ClusterSnapshot;
    summarize: (request: SubmitSummary) => Promise<SummaryView>;
  }
  let { snapshot, summarize }: Props = $props();

  const models = $derived(availableModels(snapshot.nodes));
  let model = $state('');
  $effect(() => {
    if (models.length > 0 && !models.some((m) => m.model === model)) model = models[0]!.model;
  });
  const machines = $derived(models.find((m) => m.model === model)?.machines ?? 0);

  // The document read so far; only its text is sent to the master.
  let file = $state<{ name: string; text: string; pages: number | null; chars: number } | null>(null);
  let reading = $state('');
  let busy = $state(false);
  let error = $state('');
  const problem = $derived(file ? documentProblem(file.text) : '');

  async function onchange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const chosen = input.files?.[0];
    if (!chosen) return;
    error = '';
    file = null;
    reading = 'Reading…';
    try {
      const doc = await readDocument(chosen, (done, total) => (reading = `Reading page ${done} of ${total}…`));
      file = { name: chosen.name, text: doc.text, pages: doc.pages, chars: countChars(doc.text) };
    } catch (e) {
      error = `Could not read ${chosen.name}: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      reading = '';
      // Choosing the same file again reads it again.
      input.value = '';
    }
  }

  async function start() {
    if (!file) return;
    error = '';
    busy = true;
    try {
      await summarize({ model, title: file.name, text: file.text });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  // Newest first; only the newest result starts open.
  const summaries = $derived([...(snapshot.summaries ?? [])].reverse());
  const fmt = (n: number) => n.toLocaleString('en-US');

  let copied = $state<number | null>(null);
  async function copy(s: SummaryView) {
    try {
      await navigator.clipboard.writeText(s.result ?? '');
      copied = s.summary_id;
      setTimeout(() => (copied = null), 2000);
    } catch {
      // Clipboard needs a secure context; the text stays selectable.
    }
  }
</script>

<section class="card">
  <div class="head">
    <div class="intro">
      <h2>Summarize a document</h2>
      {#if models.length === 0}
        <p class="muted">Needs a machine with a local AI model; see <strong>Ask the local AI</strong> above.</p>
      {:else}
        <p class="muted">
          Open a PDF or text file. Its parts are summarized at the same time on {machines} machine{machines === 1
            ? ''
            : 's'}, then combined into one summary. Only the text leaves this screen.
        </p>
      {/if}
    </div>
  </div>

  {#if models.length > 0}
    <div class="fields">
      <label class="field file">
        <span>Document (.pdf, .txt, .md)</span>
        <input type="file" accept=".pdf,.txt,.md,application/pdf,text/plain,text/markdown" {onchange} disabled={!!reading || busy} />
      </label>
      {#if models.length > 1}
        <label class="field">
          <span>Model</span>
          <select bind:value={model}>
            {#each models as m (m.model)}
              <option value={m.model}>{m.model} · {m.machines} machine{m.machines === 1 ? '' : 's'}</option>
            {/each}
          </select>
        </label>
      {/if}
      <button class="btn primary big" type="button" disabled={!file || !!problem || busy || !!reading || !model} onclick={start}>
        {busy ? 'Starting…' : `Summarize on ${machines} machine${machines === 1 ? '' : 's'}`}
      </button>
    </div>
    <p class="muted doc" aria-live="polite">
      {#if reading}
        {reading}
      {:else if file}
        <strong>{file.name}</strong>
        {#if file.pages !== null}· {file.pages} page{file.pages === 1 ? '' : 's'}{/if}
        · {fmt(file.chars)} characters · about {estimateParts(file.chars)} part{estimateParts(file.chars) === 1 ? '' : 's'}
      {/if}
    </p>
    {#if problem}<p class="error" role="alert">{problem}</p>{/if}
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#each summaries as s, i (s.summary_id)}
    {@const p = summaryProgress(s, snapshot.jobs)}
    <article class="summary">
      <header>
        <div>
          <strong>{s.title}</strong>
          <span class="muted">· {fmt(s.chars)} characters in {s.parts} part{s.parts === 1 ? '' : 's'} · {s.model}</span>
        </div>
        <StatusBadge state={s.state} />
      </header>
      {#if s.state === 'running' && p}
        <div class="step">
          <span>Step {p.step}: {p.label}{p.machines > 1 ? ` on ${p.machines} machines` : ''}</span>
          <span class="nums">{p.done} / {p.total}</span>
        </div>
        <div class="track" role="progressbar" aria-valuemin={0} aria-valuemax={p.total} aria-valuenow={p.done} aria-label={p.label}>
          <div class="fill" style:width="{p.total ? (p.done / p.total) * 100 : 0}%"></div>
        </div>
      {:else if s.state === 'failed'}
        <p class="error">{s.error ?? 'Failed'}</p>
      {:else if s.result}
        <details class="result" open={i === 0}>
          <summary>Summary <span class="muted">({s.jobs.length} step{s.jobs.length === 1 ? '' : 's'})</span></summary>
          <p class="text">{s.result}</p>
          <button class="btn" type="button" onclick={() => copy(s)}>{copied === s.summary_id ? 'Copied' : 'Copy'}</button>
        </details>
      {/if}
    </article>
  {/each}
</section>

<style>
  h2 {
    margin: 0 0 4px;
    font-size: 16px;
  }
  .head p {
    margin: 0;
  }
  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: flex-end;
    margin-top: 12px;
  }
  .file {
    flex: 1 1 260px;
  }
  .file input {
    padding: 6px;
  }
  .big {
    font-size: 16px;
    padding: 10px 22px;
  }
  .doc {
    margin: 8px 0 0;
    min-height: 1.4em;
  }
  .summary {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--grid);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 8px;
  }
  .step {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 14px;
  }
  .nums {
    font-variant-numeric: tabular-nums;
  }
  .track {
    height: 8px;
    border-radius: 4px;
    background: var(--grid);
    overflow: hidden;
  }
  .track .fill {
    height: 100%;
    background: var(--series-1);
    transition: width 0.3s ease-out;
  }
  .result summary {
    cursor: pointer;
    color: var(--ink-2);
  }
  .text {
    white-space: pre-wrap;
    line-height: 1.55;
    margin: 8px 0;
    padding: 10px 12px;
    background: var(--page);
    border-radius: 8px;
  }
  .error {
    margin: 8px 0 0;
    color: var(--ink);
    display: flex;
    gap: 6px;
  }
  .error::before {
    content: '✕';
    color: var(--critical);
    font-weight: 700;
  }
</style>
