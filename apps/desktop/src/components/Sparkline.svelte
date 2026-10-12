<script lang="ts">
  import { formatAgo, formatPercent } from '../lib/format';
  import { HISTORY_LENGTH, type Sample } from '../lib/history';

  interface Props {
    samples: Sample[];
    /** Which measurement to plot. */
    field: 'cpu' | 'memory';
    label: string;
  }
  let { samples, field, label }: Props = $props();

  const W = 240;
  const H = 56;
  const PAD = 6; // keeps the end dot and its ring inside the viewBox
  const step = (W - PAD * 2) / (HISTORY_LENGTH - 1);

  // Newest sample sits at the right edge; the line grows in from the right.
  const points = $derived(
    samples.map((s, i) => ({
      x: W - PAD - (samples.length - 1 - i) * step,
      y: PAD + (1 - s[field] / 100) * (H - PAD * 2),
      value: s[field],
      t: s.t,
    })),
  );
  const line = $derived(points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' '));
  const area = $derived(
    points.length > 1
      ? `${line} ${points[points.length - 1]!.x.toFixed(1)},${H - PAD} ${points[0]!.x.toFixed(1)},${H - PAD}`
      : '',
  );
  const last = $derived(points[points.length - 1]);

  let hover = $state<number | null>(null);
  const active = $derived(hover !== null ? points[hover] : undefined);
  // The highlighted point: the hovered one, else the newest.
  const dot = $derived(active ?? last);

  function nearest(clientX: number, el: SVGSVGElement): number | null {
    if (points.length === 0) return null;
    const rect = el.getBoundingClientRect();
    const x = ((clientX - rect.left) / rect.width) * W;
    let best = 0;
    for (let i = 1; i < points.length; i++) {
      if (Math.abs(points[i]!.x - x) < Math.abs(points[best]!.x - x)) best = i;
    }
    return best;
  }

  function onKey(event: KeyboardEvent) {
    if (points.length === 0) return;
    const current = hover ?? points.length - 1;
    if (event.key === 'ArrowLeft') hover = Math.max(0, current - 1);
    else if (event.key === 'ArrowRight') hover = Math.min(points.length - 1, current + 1);
    else if (event.key === 'Escape') hover = null;
    else return;
    event.preventDefault();
  }

  const now = $derived(last?.t ?? 0);
</script>

<div class="spark">
  <!-- The chart is keyboard-operable (arrow keys move the readout), so it is interactive despite role=img. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <svg
    viewBox="0 0 {W} {H}"
    role="img"
    aria-label="{label}: {last ? formatPercent(last.value) : 'no data'}, last {samples.length} samples"
    tabindex="0"
    onpointermove={(e) => (hover = nearest(e.clientX, e.currentTarget))}
    onpointerleave={() => (hover = null)}
    onfocus={() => (hover ??= points.length ? points.length - 1 : null)}
    onblur={() => (hover = null)}
    onkeydown={onKey}
  >
    <!-- Recessive hairline grid at 0 / 50 / 100 %. -->
    {#each [0, 50, 100] as g (g)}
      <line x1={PAD} x2={W - PAD} y1={PAD + (1 - g / 100) * (H - PAD * 2)} y2={PAD + (1 - g / 100) * (H - PAD * 2)} stroke="var(--grid)" stroke-width="1" />
    {/each}
    {#if points.length > 1}
      <polygon points={area} fill="var(--series-1)" opacity="0.1" />
      <polyline points={line} fill="none" stroke="var(--series-1)" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
    {/if}
    {#if active}
      <line x1={active.x} x2={active.x} y1={PAD} y2={H - PAD} stroke="var(--axis)" stroke-width="1" />
    {/if}
    {#if dot}
      <!-- 2px surface ring keeps the dot legible where it crosses the line. -->
      <circle cx={dot.x} cy={dot.y} r="6" fill="var(--surface)" />
      <circle cx={dot.x} cy={dot.y} r="4" fill="var(--series-1)" />
    {/if}
  </svg>

  {#if active}
    <div class="tip" style:left="{Math.min(80, Math.max(20, (active.x / W) * 100))}%">
      <strong>{formatPercent(active.value)}</strong>
      <span>{formatAgo((now - active.t) / 1000)}</span>
    </div>
  {/if}

  <!-- Table twin: every plotted value stays reachable without hovering. -->
  <table class="sr-only">
    <caption>{label}, newest last</caption>
    <thead><tr><th>Seconds before latest</th><th>Value</th></tr></thead>
    <tbody>
      {#each points as p (p.t)}
        <tr><td>{Math.round((now - p.t) / 1000)}</td><td>{formatPercent(p.value)}</td></tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .spark {
    position: relative;
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    touch-action: pan-y;
  }
  .tip {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 3px 8px;
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
    display: flex;
    gap: 6px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  }
  .tip strong {
    color: var(--ink);
  }
  .tip span {
    color: var(--ink-2);
  }
</style>
