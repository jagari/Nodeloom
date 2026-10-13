<script lang="ts">
  import type { PairedRow } from '../lib/devices';
  import type { NodeId } from '../lib/types';

  interface Props {
    rows: PairedRow[];
    live: boolean;
    remove: (id: NodeId) => Promise<void>;
  }
  let { rows, live, remove }: Props = $props();

  let busy = $state<NodeId | null>(null);
  /** Machine whose removal waits for a second click (not window.confirm(),
   * which the macOS webview always answers with false). */
  let confirming = $state<NodeId | null>(null);
  let error = $state('');

  async function onremove(row: PairedRow) {
    confirming = null;
    busy = row.id;
    error = '';
    try {
      await remove(row.id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = null;
    }
  }
</script>

<section aria-labelledby="paired-title">
  <h2 id="paired-title">Paired machines</h2>
  {#if error}<p class="problem" role="alert">{error}</p>{/if}
  <ul class="card list">
    {#each rows as row (row.id)}
      <li>
        <!-- Status is icon + label, never color alone. -->
        <span class="state" class:online={row.online}>
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2">
            {#if row.online}
              <circle cx="8" cy="8" r="5" fill="currentColor" stroke="none" />
            {:else}
              <circle cx="8" cy="8" r="4.5" />
            {/if}
          </svg>
          {row.online ? 'Online' : 'Offline'}
        </span>
        <span class="name">{row.name}</span>
        {#if confirming === row.id}
          <button class="btn" onclick={() => (confirming = null)}>Cancel</button>
          <button class="btn danger" disabled={!live || busy !== null} onclick={() => onremove(row)}>
            Remove for good
          </button>
        {:else}
          <button class="btn" disabled={!live || busy !== null} onclick={() => (confirming = row.id)}>
            {busy === row.id ? 'Removing…' : 'Remove'}
          </button>
        {/if}
      </li>
      {#if confirming === row.id}
        <li class="explain muted">
          {row.name} is disconnected now and its key is refused from then on. To add it again, run
          <code>nodeloom-agent reset-identity</code> on it and approve it here. If it was started with
          <code>--token</code>, that machine also holds the admin token: rotate it with
          <code>nodeloom-master --rotate-admin-token</code>.
        </li>
      {/if}
    {/each}
  </ul>
</section>

<style>
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 4px 16px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    width: 72px;
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .state svg {
    color: var(--muted);
  }
  .state.online svg {
    color: var(--good);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .explain {
    display: block;
    border-top: none;
    padding-top: 0;
    font-size: 13px;
  }
  .btn.danger {
    color: var(--critical);
    font-weight: 600;
  }
  code {
    background: var(--page);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .problem {
    display: flex;
    gap: 6px;
    margin: 0 0 8px;
  }
  .problem::before {
    content: '✕';
    color: var(--critical);
    font-weight: 700;
  }
</style>
