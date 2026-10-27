<script lang="ts">
  import { onMount } from 'svelte';
  import { ClusterClient, parseEndpoint, type ConnectionStatus } from './lib/client';
  import { lookalikes, pairedRows } from './lib/devices';
  import { updateHistory, type History } from './lib/history';
  import { initialConnection } from './lib/launch';
  import { displayNames } from './lib/names';
  import type { ClusterSnapshot, NodeId, TaskPayload } from './lib/types';
  import ConnectForm from './components/ConnectForm.svelte';
  import JobList from './components/JobList.svelte';
  import Kpis from './components/Kpis.svelte';
  import NodeCard from './components/NodeCard.svelte';
  import PairedDevices from './components/PairedDevices.svelte';
  import PendingDevices from './components/PendingDevices.svelte';
  import SubmitForm from './components/SubmitForm.svelte';

  const EMPTY: ClusterSnapshot = { nodes: [], jobs: [], pending: [], devices: [] };

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

  const launch = initialConnection(
    window.location,
    { host: load(HOST_KEY), token: load(TOKEN_KEY) },
    import.meta.env.DEV,
  );
  if (launch.clearHash) {
    // Keep the token out of the address bar, history and screenshots.
    window.history.replaceState(null, '', window.location.pathname + window.location.search);
  }

  let host = $state(launch.host);
  let token = $state(launch.token);
  let status = $state<ConnectionStatus>('connecting');
  let snapshot = $state<ClusterSnapshot>(EMPTY);
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
    snapshot = EMPTY;
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

  // Reconnect at start from a remembered connection or the master's link.
  // Not an $effect: one that reads `client` re-runs after start() sets it,
  // and its cleanup would stop the client it just made.
  onMount(() => {
    if (host && token) start(host, token);
    return () => client?.stop();
  });

  const names = $derived(displayNames(snapshot.nodes.map((n) => n.info)));
  const paired = $derived(pairedRows(snapshot.devices, snapshot.nodes));
  const flags = $derived(lookalikes(snapshot.pending, snapshot.devices));

  function connected(): ClusterClient {
    if (!client) throw new Error('Not connected');
    return client;
  }
  const submit = (payload: TaskPayload) => connected().submit(payload);
  const approve = (key: string) => connected().approve(key);
  const ignore = (key: string) => connected().ignore(key);
  const remove = (id: NodeId) => connected().removeDevice(id);

  const statusText: Record<ConnectionStatus, string> = {
    connecting: 'Connecting…',
    live: 'Live',
    offline: 'Master unreachable, retrying…',
    unauthorized: 'Wrong admin token',
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
      {#if snapshot.pending.length > 0}
        <PendingDevices pending={snapshot.pending} {flags} live={status === 'live'} {approve} {ignore} />
      {/if}

      <Kpis {snapshot} />

      <section>
        <h2>Machines</h2>
        {#if snapshot.nodes.length === 0}
          <p class="muted">
            No machine is online. Start <code>nodeloom-agent</code> on a machine on this network; it will appear
            here for approval.
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

      {#if paired.length > 0}
        <PairedDevices rows={paired} live={status === 'live'} {remove} />
      {/if}
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
