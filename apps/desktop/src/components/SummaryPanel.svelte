<script lang="ts">
  import { errorText } from '../lib/i18n';
  import { availableModels } from '../lib/llm';
  import { t } from '../lib/locale.svelte';
  import { countChars, documentProblem, estimateParts, readDocument } from '../lib/pdf';
  import { summaryProgress } from '../lib/summary';
  import type { ClusterSnapshot, SubmitSummary, SummaryView } from '../lib/types';
  import Rich from './Rich.svelte';
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
  const machineCount = $derived(t('count.machine', { count: machines }));

  // The document read so far; only its text is sent to the master.
  let file = $state<{ name: string; text: string; pages: number | null; chars: number } | null>(null);
  /** Reading progress: pages so far, or 0 of 0 before the first. Null when not reading. */
  let reading = $state<{ done: number; total: number } | null>(null);
  let busy = $state(false);
  // Kept as caught (with the file it was about), so the text follows a language switch.
  let error = $state<{ error: unknown; file?: string } | null>(null);
  const problem = $derived(file ? documentProblem(file.text, t) : '');

  async function onchange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const chosen = input.files?.[0];
    if (!chosen) return;
    error = null;
    file = null;
    reading = { done: 0, total: 0 };
    try {
      const doc = await readDocument(chosen, (done, total) => (reading = { done, total }));
      file = { name: chosen.name, text: doc.text, pages: doc.pages, chars: countChars(doc.text) };
    } catch (e) {
      error = { error: e, file: chosen.name };
    } finally {
      reading = null;
      // Choosing the same file again reads it again.
      input.value = '';
    }
  }

  async function start() {
    if (!file) return;
    error = null;
    busy = true;
    try {
      await summarize({ model, title: file.name, text: file.text });
    } catch (e) {
      error = { error: e };
    } finally {
      busy = false;
    }
  }

  const errorMessage = $derived(
    !error
      ? ''
      : error.file
        ? t('summary.readFailed', { name: error.file, error: errorText(error.error, t) })
        : errorText(error.error, t),
  );

  // Newest first; only the newest result starts open.
  const summaries = $derived([...(snapshot.summaries ?? [])].reverse());

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
      <h2>{t('summary.title')}</h2>
      {#if models.length === 0}
        <p class="muted">
          <Rich text={t('summary.needsModel')}>
            {#snippet ask()}<strong>{t('ai.title')}</strong>{/snippet}
          </Rich>
        </p>
      {:else}
        <p class="muted">{t('summary.intro', { machines: machineCount })}</p>
      {/if}
    </div>
  </div>

  {#if models.length > 0}
    <div class="fields">
      <label class="field file">
        <span>{t('summary.file')}</span>
        <input type="file" accept=".pdf,.txt,.md,application/pdf,text/plain,text/markdown" {onchange} disabled={!!reading || busy} />
      </label>
      {#if models.length > 1}
        <label class="field">
          <span>{t('ai.model')}</span>
          <select bind:value={model}>
            {#each models as m (m.model)}
              <option value={m.model}>{m.model} · {t('count.machine', { count: m.machines })}</option>
            {/each}
          </select>
        </label>
      {/if}
      <button class="btn primary big" type="button" disabled={!file || !!problem || busy || !!reading || !model} onclick={start}>
        {busy ? t('common.starting') : t('summary.start', { machines: machineCount })}
      </button>
    </div>
    <p class="muted doc" aria-live="polite">
      {#if reading}
        {reading.total ? t('summary.readingPage', reading) : t('summary.reading')}
      {:else if file}
        <strong>{file.name}</strong>
        {#if file.pages !== null}· {t('count.page', { count: file.pages })}{/if}
        · {t('count.char', { count: file.chars })}
        · {t('summary.aboutParts', { parts: t('count.part', { count: estimateParts(file.chars) }) })}
      {/if}
    </p>
    {#if problem}<p class="error" role="alert">{problem}</p>{/if}
  {/if}
  {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}

  {#each summaries as s, i (s.summary_id)}
    {@const p = summaryProgress(s, snapshot.jobs, t)}
    <article class="summary">
      <header>
        <div>
          <strong>{s.title}</strong>
          <span class="muted"
            >· {t('summary.meta', { chars: t('count.char', { count: s.chars }), parts: t('count.part', { count: s.parts }) })} · {s.model}</span
          >
        </div>
        <StatusBadge state={s.state} />
      </header>
      {#if s.state === 'running' && p}
        <div class="step">
          <span
            >{p.machines > 1
              ? t('summary.stepOn', { step: p.step, label: p.label, machines: t('count.machine', { count: p.machines }) })
              : t('summary.step', { step: p.step, label: p.label })}</span
          >
          <span class="nums">{p.done} / {p.total}</span>
        </div>
        <div class="track" role="progressbar" aria-valuemin={0} aria-valuemax={p.total} aria-valuenow={p.done} aria-label={p.label}>
          <div class="fill" style:width="{p.total ? (p.done / p.total) * 100 : 0}%"></div>
        </div>
      {:else if s.state === 'failed'}
        <!-- The master's own error is shown as sent. -->
        <p class="error">{s.error ?? t('summary.failed')}</p>
      {:else if s.result}
        <details class="result" open={i === 0}>
          <summary>{t('summary.result')} <span class="muted">({t('count.step', { count: s.jobs.length })})</span></summary>
          <p class="text">{s.result}</p>
          <button class="btn" type="button" onclick={() => copy(s)}>
            {copied === s.summary_id ? t('common.copied') : t('common.copy')}
          </button>
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
