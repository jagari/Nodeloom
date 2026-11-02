<script lang="ts">
  import { slotColor, type CurrentWork } from '../lib/jobs';
  import type { NodeId, NodeView } from '../lib/types';
  import NodeRow from './NodeRow.svelte';

  interface Props {
    nodes: NodeView[];
    names: Map<NodeId, string>;
    work: Map<NodeId, CurrentWork[]>;
    slots: Map<NodeId, number>;
  }
  let { nodes, names, work, slots }: Props = $props();

  // Same order as the map and the colours, not arrival order.
  const rows = $derived(
    [...nodes].sort((a, b) => (slots.get(a.info.id) || 99) - (slots.get(b.info.id) || 99)),
  );
</script>

<section class="card machines" aria-labelledby="machines-title">
  <h2 id="machines-title">Machines <span class="muted">({nodes.length} online)</span></h2>
  {#if nodes.length === 0}
    <p class="muted">No machine is online yet. See <strong>Add a machine</strong>.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th scope="col">Machine</th>
          <th scope="col">Status</th>
          <th scope="col">Working on</th>
          <th scope="col" class="num">CPU</th>
          <th scope="col" class="num">Memory</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as node (node.info.id)}
          <NodeRow
            {node}
            name={names.get(node.info.id) ?? node.info.hostname}
            work={work.get(node.info.id) ?? []}
            color={slotColor(slots.get(node.info.id))}
          />
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .machines {
    padding: 12px 12px 6px;
    min-width: 0;
  }
  h2 {
    margin: 0 4px 6px;
    font-size: 16px;
  }
  h2 .muted {
    font-weight: 400;
    font-size: 13px;
  }
  p {
    margin: 0 4px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 14px;
  }
  th {
    text-align: left;
    font-size: 12px;
    font-weight: 400;
    color: var(--ink-2);
    padding: 0 8px 4px;
  }
  /* Fixed widths keep the numbers in line from row to row. */
  th:nth-child(2) {
    width: 84px;
  }
  th:nth-child(3) {
    width: 27%;
  }
  th:nth-child(4) {
    width: 60px;
  }
  th:nth-child(5) {
    width: 108px;
  }
  .num {
    text-align: right;
  }
  @media (max-width: 720px) {
    table,
    tbody {
      display: block;
    }
    thead {
      display: none;
    }
  }
</style>
