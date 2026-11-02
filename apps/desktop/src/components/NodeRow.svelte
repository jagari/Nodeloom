<script lang="ts">
  import { formatBytes, formatPercent, severity } from '../lib/format';
  import type { CurrentWork } from '../lib/jobs';
  import { unitWord } from '../lib/i18n';
  import { unitOf } from '../lib/llm';
  import { t } from '../lib/locale.svelte';
  import type { NodeView } from '../lib/types';

  interface Props {
    node: NodeView;
    name: string;
    /** Pieces this machine is running now (`workByNode`). */
    work?: CurrentWork[];
    /** Its colour on the job bars and the map, so they can be matched by eye. */
    color?: string;
  }
  let { node, name, work = [], color = 'var(--muted)' }: Props = $props();

  const m = $derived(node.metrics);
  const memPct = $derived(m && m.memory_total_bytes > 0 ? (m.memory_used_bytes / m.memory_total_bytes) * 100 : 0);
  const memLevel = $derived(severity(memPct));
  const range = (w: CurrentWork) => `${unitWord(t, unitOf(w.runner), 2)} ${w.frame_start}–${w.frame_end}`;
</script>

<tr style:--c={color}>
  <td class="machine">
    <div class="name" title={name}>
      <span class="swatch" aria-hidden="true"></span>
      <strong>{name}</strong>
    </div>
    <div class="sub">
      {#each node.info.runners as runner (runner)}
        <span class="chip">{runner}</span>
      {/each}
      <span class="muted" title="{node.info.os} · {node.info.arch} · {t('count.core', { count: node.info.cpu_cores })}"
        >{node.info.os} · {node.info.arch}</span
      >
    </div>
  </td>
  <td class="status">
    <!-- Status is icon + word, never colour alone. -->
    <span class="busy" class:active={work.length > 0}>
      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
        {#if work.length > 0}
          <path d="M5 3.5l7 4.5-7 4.5z" fill="currentColor" />
        {:else}
          <circle cx="8" cy="8" r="4.5" fill="none" stroke="currentColor" stroke-width="2" />
        {/if}
      </svg>
      {work.length > 0 ? t('node.working') : t('node.idle')}
    </span>
  </td>
  <td class="work">
    {#each work as w (w.task_id)}
      <div class="task">
        <span class="what">{t('job.name', { id: w.job_id + 1 })} · {range(w)}</span>
        <div
          class="track"
          role="progressbar"
          aria-label={t('node.progress', { range: range(w), name })}
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={Math.round(w.fraction * 100)}
        >
          <div class="fill" style:width="{w.fraction * 100}%"></div>
        </div>
        <span class="pct">{Math.round(w.fraction * 100)}%</span>
      </div>
    {:else}
      <span class="muted">–</span>
    {/each}
  </td>
  {#if m}
    <td class="num cpu">
      <span class="label">{t('col.cpu')}</span>
      {formatPercent(m.cpu_percent)}
    </td>
    <td class="num mem" title={t('node.memoryOf', { used: formatBytes(m.memory_used_bytes), total: formatBytes(m.memory_total_bytes) })}>
      <span class="label">{t('col.memory')}</span>
      {#if memLevel !== 'ok'}
        <!-- Severity is icon + word, never colour alone. -->
        <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" class={memLevel} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 2.2l6.2 11H1.8z" /><path d="M8 6.8v3M8 11.8v.1" />
        </svg>
        <span class="sev">{memLevel === 'critical' ? t('mem.critical') : t('mem.high')}</span>
      {/if}
      {formatPercent(memPct)}
    </td>
  {:else}
    <td class="num waiting muted" colspan="2">{t('node.waitingReport')}</td>
  {/if}
</tr>

<style>
  td {
    padding: 7px 8px;
    border-top: 1px solid var(--grid);
    vertical-align: middle;
  }
  .machine {
    min-width: 0;
  }
  /* One line each, cut with an ellipsis: rows stay short on a projector. */
  .name,
  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    min-width: 0;
  }
  .name strong,
  .sub .muted {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .swatch {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--c);
  }
  .chip {
    flex: none;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
    font-size: 11px;
    color: var(--ink-2);
  }
  .sub {
    font-size: 12px;
    margin: 2px 0 0 18px;
  }
  .busy {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    color: var(--ink-2);
    white-space: nowrap;
  }
  .busy.active {
    color: var(--ink);
    font-weight: 600;
  }
  .busy.active svg {
    color: var(--series-1);
  }
  .task {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 3em;
    column-gap: 8px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .task + .task {
    margin-top: 4px;
  }
  .what {
    grid-column: 1 / -1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .track {
    align-self: center;
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    background: color-mix(in srgb, var(--c) 22%, var(--surface));
  }
  .fill {
    height: 100%;
    background: var(--c);
    transition: width 0.4s ease;
  }
  .pct {
    text-align: right;
    font-weight: 600;
  }
  .num {
    text-align: right;
    white-space: nowrap;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .mem svg {
    vertical-align: -2px;
  }
  .sev {
    margin-right: 2px;
  }
  svg.warning {
    color: var(--warning);
  }
  svg.critical {
    color: var(--critical);
  }
  .waiting {
    font-weight: 400;
    font-size: 12px;
  }
  /* Column headers name these on wide screens; stacked rows say it inline. */
  .label {
    display: none;
    font-weight: 400;
    color: var(--ink-2);
    margin-right: 6px;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }

  /* Narrow: each machine becomes a small block instead of a wide row. */
  @media (max-width: 720px) {
    tr {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 4px 12px;
      padding: 8px 0;
      border-top: 1px solid var(--grid);
    }
    td {
      display: block;
      padding: 0;
      border: 0;
    }
    .work {
      grid-column: 1 / -1;
    }
    /* Room is short here; wrap rather than cut the "(1ba6)" that tells machines apart. */
    .name strong {
      white-space: normal;
      overflow-wrap: anywhere;
    }
    .num {
      text-align: left;
    }
    .label {
      display: inline;
    }
  }
</style>
