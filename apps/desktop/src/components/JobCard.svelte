<script lang="ts">
  import { unitWord } from '../lib/i18n';
  import { jobSegments, slotColor, summarizeJob } from '../lib/jobs';
  import { unitOf } from '../lib/llm';
  import { t } from '../lib/locale.svelte';
  import { movesOfJob, type MoveTracker } from '../lib/moves';
  import { jobRole } from '../lib/summary';
  import type { JobView, NodeId, SummaryView } from '../lib/types';
  import Rich from './Rich.svelte';
  import StatusBadge from './StatusBadge.svelte';

  interface Props {
    job: JobView;
    /** Its answers start open; older jobs' stay shut so they take no room. */
    newest: boolean;
    summaries: SummaryView[] | undefined;
    /** Display name per machine, including ones that just left. */
    names: Map<NodeId, string>;
    /** Colour slot per machine (`nodeSlots`). */
    slots: Map<NodeId, number>;
    moves: MoveTracker;
  }
  let { job, newest, summaries, names, slots, moves }: Props = $props();

  const nameOf = (id: NodeId | null) => (id ? (names.get(id) ?? id) : t('job.noMachine'));
  const pct = (fraction: number) => `${Math.round(fraction * 100)}%`;

  const s = $derived(summarizeJob(job));
  const layout = $derived(jobSegments(job));
  const jobMoves = $derived(movesOfJob(moves, job.job_id));
  const justMoved = $derived(new Set(jobMoves.filter((m) => m.to).map((m) => m.task_id)));
  const machines = $derived(new Set(job.tasks.map((task) => task.node).filter(Boolean)).size);
  const role = $derived(jobRole(summaries, job, t));
  const unitName = $derived(role ? role.unit : unitOf(job.runner));
  const unit = (n: number) => unitWord(t, unitName, n);
  const units = $derived(unit(2));
  const range = (start: number, end: number) => `${units} ${start}–${end}`;
  const total = $derived(t(`count.${unitName}`, { count: s.totalFrames }));
  const machineCount = $derived(t('count.machine', { count: machines }));
</script>

<article class="card job">
  <header>
    <div>
      <strong>{t('job.name', { id: job.job_id + 1 })}</strong>
      {#if role}<span class="role">{role.label}</span>{/if}
      <span class="muted">
        · {t('job.meta', { units: total, pieces: t('count.piece', { count: job.tasks.length }), machines: machineCount })}
      </span>
    </div>
    <div class="head-right">
      <span class="total">{pct(s.fraction)}</span>
      <StatusBadge state={s.state} />
    </div>
  </header>

  <div
    class="bar"
    role="img"
    aria-label={t('job.barLabel', { id: job.job_id + 1, pct: pct(s.fraction), total, machines: machineCount })}
  >
    {#each layout.segments as seg (seg.task.task_id)}
      {@const task = seg.task}
      <div
        class="seg {task.state}"
        class:moved={justMoved.has(task.task_id)}
        style:left="{seg.left}%"
        style:width="calc({seg.width}% - 3px)"
        style:--c={task.node ? slotColor(slots.get(task.node)) : 'var(--muted)'}
        title="{nameOf(task.node)} · {range(task.frame_start, task.frame_end)} · {pct(seg.fraction)}"
      >
        <div class="fill" style:width="{seg.fraction * 100}%"></div>
        {#if seg.width >= 12}
          <span class="in-label">{nameOf(task.node)}</span>
        {/if}
      </div>
    {/each}
  </div>
  <div class="axis muted" aria-hidden="true">
    <span>{unit(1)} {layout.first}</span><span>{layout.last}</span>
  </div>

  <!-- Announced to screen readers; seen by the audience as a banner. Under
       the bar, so a few notices never push the bar off a 720p screen. -->
  <div class="notices" role="status" aria-live="polite">
    {#each jobMoves as m (`${m.task_id}-${m.at}`)}
      <p class="notice" class:waiting={!m.to}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          {#if m.to}
            <path d="M2 8h10M8.5 4.5L12 8l-3.5 3.5" />
          {:else}
            <circle cx="8" cy="8" r="5.5" /><path d="M8 5v3.3l2 1.2" />
          {/if}
        </svg>
        {#if m.to}
          {@const dest = m.to}
          <span>
            <Rich text={t('move.moved', { range: range(m.frame_start, m.frame_end) })}>
              {#snippet label()}<strong>{t('move.label')}</strong>{/snippet}
              {#snippet from()}<strong>{nameOf(m.from)}</strong>{/snippet}
              {#snippet to()}<strong>{nameOf(dest)}</strong>{/snippet}
            </Rich>
          </span>
        {:else}
          <span>
            <Rich text={t('move.left', { range: range(m.frame_start, m.frame_end) })}>
              {#snippet who()}<strong>{t('move.leftWho', { name: nameOf(m.from) })}</strong>{/snippet}
            </Rich>
          </span>
        {/if}
      </p>
    {/each}
  </div>

  <ul class="pieces">
    {#each layout.segments as seg (seg.task.task_id)}
      {@const task = seg.task}
      <li>
        <span class="swatch" class:empty={!task.node} style:--c={task.node ? slotColor(slots.get(task.node)) : 'var(--muted)'}></span>
        <span class="who">
          <strong>{nameOf(task.node)}</strong>
          {#if moves.movedFrom[task.task_id]}
            <span class="tag">{t('move.from', { name: nameOf(moves.movedFrom[task.task_id] ?? null) })}</span>
          {/if}
        </span>
        <span class="nums">{range(task.frame_start, task.frame_end)}</span>
        <span class="nums pct">{pct(seg.fraction)}</span>
        <span class="state">
          <StatusBadge state={task.state} />
          {#if task.error}<span class="err muted">{task.error}</span>{/if}
        </span>
      </li>
    {/each}
  </ul>

  {#if job.outputs.length > 0}
    <details class="answers" open={newest}>
      <summary>{t('job.answers', { done: job.outputs.length, total: s.totalFrames })}</summary>
      <ol>
        {#each job.outputs as o (o.frame)}
          <li>
            <p class="q"><span class="num">{o.frame}.</span> {o.prompt}</p>
            <p class="a">{o.text}</p>
            <p class="by muted">
              <span class="swatch" style:--c={slotColor(slots.get(o.node))}></span>{nameOf(o.node)}
            </p>
          </li>
        {/each}
      </ol>
    </details>
  {/if}
</article>

<style>
  .job {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 8px;
  }
  .role {
    margin-left: 6px;
    padding: 1px 8px;
    border-radius: 10px;
    background: var(--page);
    border: 1px solid var(--border);
    font-size: 13px;
  }
  .head-right {
    display: inline-flex;
    align-items: baseline;
    gap: 12px;
  }
  .total {
    font-size: 22px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .notices:empty {
    display: none;
  }
  .notices {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .notice {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid var(--series-1);
    background: color-mix(in srgb, var(--series-1) 10%, var(--surface));
    font-size: 14px;
    animation: arrive 0.4s ease-out;
  }
  .notice svg {
    flex: none;
    margin-top: 2px;
    color: var(--series-1);
  }
  .notice.waiting {
    border-color: var(--warning);
    background: color-mix(in srgb, var(--warning) 10%, var(--surface));
  }
  .notice.waiting svg {
    color: var(--ink-2);
  }

  /* The frame axis: one piece per task, positioned by its frames. */
  .bar {
    position: relative;
    height: 36px;
  }
  .seg {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 6px;
    overflow: hidden;
    background: color-mix(in srgb, var(--c) 22%, var(--surface));
    transition:
      left 0.3s,
      width 0.3s;
  }
  .seg.pending {
    background: repeating-linear-gradient(
      45deg,
      var(--grid) 0 6px,
      transparent 6px 12px
    );
    outline: 1px dashed var(--axis);
    outline-offset: -1px;
  }
  .fill {
    height: 100%;
    background: var(--c);
    transition: width 0.4s ease;
  }
  .seg.failed .fill {
    background: var(--critical);
  }
  .in-label {
    position: absolute;
    inset: 0 8px;
    display: flex;
    align-items: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    /* Readable over both the filled and the empty part. */
    text-shadow:
      0 0 3px var(--surface),
      0 0 3px var(--surface),
      0 0 2px var(--surface);
  }
  .seg.moved {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--c);
    animation: pulse 1.2s ease-in-out 3;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    margin-top: -4px;
    font-variant-numeric: tabular-nums;
  }

  .pieces {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 13px;
  }
  .pieces li {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr) auto 3.5em 6.5em;
    gap: 10px;
    align-items: center;
    padding: 6px 0;
    border-top: 1px solid var(--grid);
  }
  .answers {
    margin-top: 12px;
    border-top: 1px solid var(--grid);
    padding-top: 10px;
  }
  .answers summary {
    cursor: pointer;
    font-weight: 600;
  }
  .answers ol {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
  }
  .answers li {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    animation: arrive 0.4s ease-out;
  }
  .answers p {
    margin: 0;
  }
  .q {
    font-weight: 600;
  }
  .num {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .a {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .by {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--c);
  }
  .swatch.empty {
    background: none;
    border: 1px dashed var(--axis);
  }
  .who {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 8px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .tag {
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .nums {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .pct {
    text-align: right;
    font-weight: 600;
  }
  .state {
    display: flex;
    flex-direction: column;
  }
  .err {
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  @media (max-width: 560px) {
    .pieces li {
      grid-template-columns: 12px minmax(0, 1fr) auto;
    }
    .pieces li > .nums:not(.pct) {
      display: none;
    }
    .state {
      grid-column: 2 / -1;
    }
  }

  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  @keyframes pulse {
    50% {
      box-shadow:
        0 0 0 2px var(--surface),
        0 0 0 6px var(--c);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .notice,
    .seg.moved {
      animation: none;
    }
    .seg,
    .fill {
      transition: none;
    }
  }
</style>
