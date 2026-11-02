<script lang="ts">
  import { onMount } from 'svelte';
  import { ClusterClient, parseEndpoint, type ConnectionStatus, type MasterInfo } from './lib/client';
  import { lookalikes, pairedRows } from './lib/devices';
  import { nodeSlots, workByNode } from './lib/jobs';
  import { initialConnection } from './lib/launch';
  import { NO_MOVES, trackMoves, type MoveTracker } from './lib/moves';
  import { displayNames } from './lib/names';
  import type { ClusterSnapshot, NodeId, SubmitSummary, TaskPayload } from './lib/types';
  import AddMachine from './components/AddMachine.svelte';
  import ClusterMap from './components/ClusterMap.svelte';
  import ConnectForm from './components/ConnectForm.svelte';
  import JobList from './components/JobList.svelte';
  import Kpis from './components/Kpis.svelte';
  import MachineList from './components/MachineList.svelte';
  import PairedDevices from './components/PairedDevices.svelte';
  import PendingDevices from './components/PendingDevices.svelte';
  import SubmitForm from './components/SubmitForm.svelte';
  import SummaryPanel from './components/SummaryPanel.svelte';

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
  let client = $state<ClusterClient | null>(null);
  let info = $state<MasterInfo | null>(null);
  let moves = $state<MoveTracker>(NO_MOVES);

  function start(nextHost: string, nextToken: string) {
    client?.stop();
    const endpoint = parseEndpoint(nextHost);
    if (!endpoint) return;
    host = nextHost;
    token = nextToken;
    save(HOST_KEY, nextHost);
    save(TOKEN_KEY, nextToken);
    moves = NO_MOVES;
    snapshot = EMPTY;
    info = null;
    client = new ClusterClient(endpoint, nextToken, {
      onSnapshot(s) {
        snapshot = s;
        moves = trackMoves(moves, s, Date.now());
      },
      onStatus(s) {
        status = s;
        // Asked once connected: the address other machines join at.
        if (s === 'live' && !info) {
          client
            ?.info()
            .then((i) => (info = i))
            .catch(() => {});
        }
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
    // Expire "moved" notices even when no snapshot arrives.
    const tick = setInterval(() => {
      if (moves.moves.length > 0) moves = trackMoves(moves, snapshot, Date.now());
    }, 1000);
    return () => {
      clearInterval(tick);
      client?.stop();
    };
  });

  const names = $derived(displayNames(snapshot.nodes.map((n) => n.info)));
  // Machines that just left still name their pieces in "moved from …".
  const jobNames = $derived(
    new Map([...displayNames(snapshot.devices.map((d) => ({ id: d.id, hostname: d.name }))), ...names]),
  );
  const slots = $derived(nodeSlots(snapshot.devices, snapshot.nodes.map((n) => n.info.id)));
  const work = $derived(workByNode(snapshot));
  const paired = $derived(pairedRows(snapshot.devices, snapshot.nodes));
  const flags = $derived(lookalikes(snapshot.pending, snapshot.devices));
  // Still setting up: how to add machines goes on top, open. Once there are
  // two or more and nobody waits, it folds away below the jobs.
  const onboarding = $derived(snapshot.nodes.length <= 1 || snapshot.pending.length > 0);

  function connected(): ClusterClient {
    if (!client) throw new Error('Not connected');
    return client;
  }
  const submit = (payload: TaskPayload) => connected().submit(payload);
  const summarize = (request: SubmitSummary) => connected().summarize(request);
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
      <Kpis {snapshot} />
      <div class="conn">
        <span class="state {status}" role="status">
          <span class="dot" aria-hidden="true"></span>{statusText[status]}
        </span>
        <span class="muted host">{host}</span>
        <button class="btn" onclick={disconnect}>Disconnect</button>
      </div>
    </header>

    <div class="stack" class:stale={status !== 'live'}>
      {#if onboarding}
        {#if snapshot.pending.length > 0}
          <PendingDevices pending={snapshot.pending} {flags} live={status === 'live'} {approve} {ignore} />
        {/if}
        <AddMachine {info} open />
      {/if}

      <!-- Who is in the cluster and what each machine is doing, side by side. -->
      <div class="overview">
        <ClusterMap devices={snapshot.devices} nodes={snapshot.nodes} names={jobNames} {slots} {work} {moves} {info} />
        <MachineList nodes={snapshot.nodes} {names} {work} {slots} />
      </div>

      <SubmitForm nodes={snapshot.nodes} {submit} />
      <JobList {snapshot} names={jobNames} {slots} {moves} />
      <SummaryPanel {snapshot} {summarize} />

      {#if !onboarding}
        <AddMachine {info} open={false} />
      {/if}
      {#if paired.length > 0}
        <PairedDevices rows={paired} live={status === 'live'} {remove} />
      {/if}
    </div>
  </main>
{/if}

<style>
  /* Sized for a 1280×720 projector: map, machines, Demo job and the
     running job fit without scrolling. */
  main {
    max-width: 1280px;
    margin: 0 auto;
    padding: 12px 16px 40px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 24px;
    margin-bottom: 12px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
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
    gap: 12px;
    transition: opacity 0.2s;
  }
  /* Hold the last render, dimmed, while reconnecting: no layout jump. */
  .stack.stale {
    opacity: 0.6;
  }
  .overview {
    display: grid;
    grid-template-columns: minmax(0, 520px) minmax(0, 1fr);
    gap: 12px;
    align-items: stretch;
  }
  @media (max-width: 1099px) {
    .overview {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
