<script lang="ts">
  import type { MasterInfo } from '../lib/client';
  import { slotColor, type CurrentWork } from '../lib/jobs';
  import type { MoveTracker } from '../lib/moves';
  import {
    MAP_HEIGHT,
    MAP_WIDTH,
    MASTER_HEIGHT,
    MASTER_WIDTH,
    NODE_R,
    clipName,
    mapLayout,
    receivingMachines,
    workLabel,
  } from '../lib/topology';
  import type { NodeId, NodeView, PairedDevice } from '../lib/types';

  interface Props {
    devices: PairedDevice[];
    nodes: NodeView[];
    /** Display name per machine, including ones that are offline. */
    names: Map<NodeId, string>;
    slots: Map<NodeId, number>;
    work: Map<NodeId, CurrentWork[]>;
    moves: MoveTracker;
    info: MasterInfo | null;
  }
  let { devices, nodes, names, slots, work, moves, info }: Props = $props();

  const online = $derived(new Set(nodes.map((n) => n.info.id)));
  const layout = $derived(mapLayout(devices, [...online]));
  const receiving = $derived(receivingMachines(moves));
  const m = $derived(layout.master);

  const working = $derived(layout.spots.filter((s) => work.has(s.id)).length);
  const summary = $derived(
    `Master with ${online.size} of ${layout.spots.length} machines connected, ${working} working.`,
  );
</script>

<section class="card map" aria-labelledby="map-title">
  <h2 id="map-title">Cluster</h2>
  <svg viewBox="0 0 {MAP_WIDTH} {MAP_HEIGHT}" role="img" aria-label={summary}>
    <!-- Links first so the machines and the master sit on top of them. -->
    {#each layout.spots as s (s.id)}
      {@const on = online.has(s.id)}
      <line
        class="link"
        class:offline={!on}
        class:busy={on && work.has(s.id)}
        class:received={on && receiving.has(s.id)}
        style:--c={slotColor(slots.get(s.id))}
        x1={m.x}
        y1={m.y}
        x2={s.x}
        y2={s.y}
      />
    {/each}

    <g class="master">
      <rect
        x={m.x - MASTER_WIDTH / 2}
        y={m.y - MASTER_HEIGHT / 2}
        width={MASTER_WIDTH}
        height={MASTER_HEIGHT}
        rx="10"
      />
      {#if info}
        <text x={m.x} y={m.y - 3} class="m-name">Master</text>
        <text x={m.x} y={m.y + 13} class="m-addr">{info.join_address}</text>
      {:else}
        <text x={m.x} y={m.y + 5} class="m-name">Master</text>
      {/if}
    </g>

    {#each layout.spots as s (s.id)}
      {@const on = online.has(s.id)}
      {@const w = work.get(s.id) ?? []}
      {@const pct = Math.round((w[0]?.fraction ?? 0) * 100)}
      {@const name = names.get(s.id) ?? s.id}
      <!-- Name and work outside the ring: above the top row, below the bottom row. -->
      {@const nameY = s.labelAbove ? s.y - NODE_R - 21 : s.y + NODE_R + 16}
      <g class="node" class:offline={!on} style:--c={slotColor(slots.get(s.id))}>
        <title>{name}: {on ? workLabel(w) : 'offline'}</title>
        {#if on}
          <circle class="track" cx={s.x} cy={s.y} r={NODE_R} />
          {#if w.length > 0}
            <!-- Progress of its current piece, clockwise from the top. -->
            <circle
              class="progress"
              cx={s.x}
              cy={s.y}
              r={NODE_R}
              pathLength="100"
              stroke-dasharray="{pct} 100"
              transform="rotate(-90 {s.x} {s.y})"
            />
          {/if}
          <circle class="core" cx={s.x} cy={s.y} r={NODE_R - 6} />
        {:else}
          <circle class="gone" cx={s.x} cy={s.y} r={NODE_R - 3} />
        {/if}
        <text x={s.x} y={nameY} class="n-name">{clipName(name, 17)}</text>
        <text x={s.x} y={nameY + 14} class="n-work">{on ? workLabel(w) : 'offline'}</text>
      </g>
    {:else}
      <text x={m.x} y={m.y + MASTER_HEIGHT} class="n-work">No machine paired yet</text>
    {/each}
  </svg>
</section>

<style>
  .map {
    padding: 12px 12px 8px;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  h2 {
    margin: 0 4px 4px;
    font-size: 16px;
  }
  svg {
    display: block;
    width: 100%;
    max-width: 640px;
    height: auto;
    margin: auto;
    overflow: visible;
  }
  text {
    text-anchor: middle;
    font-family: var(--font);
    font-variant-numeric: tabular-nums;
    /* A halo in the card colour keeps text readable where a link passes. */
    paint-order: stroke;
    stroke: var(--surface);
    stroke-width: 4px;
    stroke-linejoin: round;
  }

  .link {
    stroke: var(--axis);
    stroke-width: 2;
  }
  .link.busy {
    stroke: var(--c);
    stroke-width: 3;
  }
  .link.offline {
    stroke: var(--muted);
    stroke-dasharray: 5 5;
  }
  /* Took over a piece from a machine that left: heavier while the notice is up. */
  .link.received {
    stroke: var(--c);
    stroke-width: 7;
    stroke-linecap: round;
    animation: pulse 1.2s ease-in-out infinite;
  }

  .master rect {
    fill: var(--ink);
  }
  .master text {
    fill: var(--page);
    stroke: none;
  }
  .m-name {
    font-size: 15px;
    font-weight: 700;
  }
  .m-addr {
    font-size: 11px;
  }

  .track {
    fill: var(--surface);
    stroke: var(--grid);
    stroke-width: 5;
  }
  .progress {
    fill: none;
    stroke: var(--c);
    stroke-width: 5;
    transition: stroke-dasharray 0.4s ease;
  }
  .core {
    fill: var(--c);
  }
  .gone {
    fill: var(--surface);
    stroke: var(--muted);
    stroke-width: 2;
    stroke-dasharray: 4 3;
  }
  .n-name {
    fill: var(--ink);
    font-size: 13px;
    font-weight: 600;
  }
  .n-work {
    fill: var(--ink-2);
    font-size: 11.5px;
  }
  .offline .n-name,
  .offline .n-work {
    fill: var(--muted);
  }

  @keyframes pulse {
    50% {
      stroke-width: 11;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .link.received {
      animation: none;
    }
    .progress {
      transition: none;
    }
  }
</style>
