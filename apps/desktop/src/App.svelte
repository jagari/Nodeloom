<script lang="ts">
  import { ClusterClient, parseEndpoint, type ConnectionStatus } from './lib/client';
  import { updateHistory, type History } from './lib/history';
  import { displayNames } from './lib/names';
  import type { ClusterSnapshot, TaskPayload } from './lib/types';
  import ConnectForm from './components/ConnectForm.svelte';
  import JobList from './components/JobList.svelte';
  import Kpis from './components/Kpis.svelte';
  import NodeCard from './components/NodeCard.svelte';
  import SubmitForm from './components/SubmitForm.svelte';

  const HOST_KEY = 'nodeloom.host';
  const TOKEN_KEY = 'nodeloom.token';

  // Remembering the connection is a convenience only; storage can be blocked.
  function load(key: string): string {
    try {
      return localStorage.getItem(key) ?? '';
    } catch {
      return '';
    }
  }
  function save(key: string, value: string) {
    try {
      value ? localStorage.setItem(key, value) : localStorage.removeItem(key);
    } catch {
      /* ignore */
    }
  }

  let host = $state(load(HOST_KEY));
  let token = $state(load(TOKEN_KEY));
  let status = $state<ConnectionStatus>('connecting');
  let snapshot = $state<ClusterSnapshot>({ nodes: [], jobs: [] });
  let history = $state<History>({});
  let client = $state<ClusterClient | null>(null);

  function start(nextHost: string, nextToken: string) {
    client?.stop();
    const endpoint = parseEndpoint(nextHost);
    if (!endpoint) return;
    host = nextHost;
    token = nextToken;
    save(HOST_KEY, nextHost);
    save(TOKEN_KEY, nextToken);
    history = {};
    snapshot = { nodes: [], jobs: [] };
    client = new ClusterClient(endpoint, nextToken, {
      onSnapshot(s) {
        snapshot = s;
        history = updateHistory(history, s);
      },
      onStatus(s) {
        status = s;
      },
    });
    client.start();
  }

  function disconnect() {
    client?.stop();
    client = null;
    token = '';
    save(TOKEN_KEY, '');
  }

  $effect(() => {
    if (host && token && !client) start(host, token);
    return () => client?.stop();
  });

  const names = $derived(displayNames(snapshot.nodes));

  const submit = (payload: TaskPayload) => {
    if (!client) throw new Error('Not connected');
    return client.submit(payload);
  };

  const statusText: Record<ConnectionStatus, string> = {
    connecting: 'Connecting…',
    live: 'Live',
    offline: 'Master unreachable, retrying…',
    unauthorized: 'Wrong pairing token',
  };
</script>

{#if !client || status === 'unauthorized'}
  <main>
    <ConnectForm
      initialHost={host}
      problem={status === 'unauthorized' && client ? statusText.unauthorized : ''}
      connect={start}
    />
  </main>
{:else}
  <main>
    <header class="top">
      <h1>Nodeloom</h1>
      <div class="conn">
        <span class="state {status}" role="status">
          <span class="dot" aria-hidden="true"></span>{statusText[status]}
        </span>
        <span class="muted host">{host}</span>
        <button class="btn" onclick={disconnect}>Disconnect</button>
      </div>
    </header>

    <div class="stack" class:stale={status !== 'live'}>
      <Kpis {snapshot} />

      <section>
        <h2>Machines</h2>
        {#if snapshot.nodes.length === 0}
          <p class="muted">
            No machine has joined yet. Run <code>nodeloom-agent --token …</code> on each machine.
          </p>
        {:else}
          <div class="nodes">
            {#each snapshot.nodes as node (node.info.id)}
              <NodeCard {node} name={names.get(node.info.id) ?? node.info.hostname} samples={history[node.info.id] ?? []} />
            {/each}
          </div>
        {/if}
      </section>

      <SubmitForm nodes={snapshot.nodes} {submit} />
      <JobList {snapshot} />
    </div>
  </main>
{/if}

<style>
  main {
    max-width: 1100px;
    margin: 0 auto;
    padding: 20px 16px 48px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 18px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .conn {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
  }
  .state.live .dot {
    background: var(--good);
  }
  .state.offline .dot {
    background: var(--warning);
  }
  .host {
    font-size: 12px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 24px;
    transition: opacity 0.2s;
  }
  /* Hold the last render, dimmed, while reconnecting: no layout jump. */
  .stack.stale {
    opacity: 0.6;
  }
  .nodes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 12px;
  }
  code {
    background: var(--surface);
    padding: 1px 5px;
    border-radius: 4px;
  }
</style>
