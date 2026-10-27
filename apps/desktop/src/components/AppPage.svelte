<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { describeAgent, type AppView, type Role, type Tone } from '../lib/app';

  let view = $state<AppView | null>(null);
  let problem = $state('');
  let busy = $state(false);
  // Setup choices.
  let picked = $state<Role>('master');
  let startAtLogin = $state(true);
  let confirmingChange = $state(false);

  async function refresh() {
    try {
      view = await invoke<AppView>('app_state');
    } catch (e) {
      problem = String(e);
    }
  }

  // The agent's status changes on its own (found a master, approved), so
  // poll; the app restarts on role changes, so there is nothing else to watch.
  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => clearInterval(timer);
  });

  async function run(command: string, args: Record<string, unknown> = {}) {
    busy = true;
    problem = '';
    try {
      await invoke(command, args);
      await refresh();
    } catch (e) {
      problem = String(e);
    } finally {
      busy = false;
    }
  }

  const chooseRole = (role: Role | null, autostart?: boolean) => run('choose_role', { role, autostart });
  const setAutostart = (enabled: boolean) => run('set_autostart', { enabled });

  const agent = $derived(view?.agent ? describeAgent(view.agent) : null);
  const toneLabel: Record<Tone, string> = {
    busy: 'Working',
    waiting: 'Waiting',
    good: 'Connected',
    problem: 'Problem',
  };
</script>

<main>
  {#if !view}
    <p class="muted">Loading…</p>
  {:else if view.role === null}
    <form
      class="card setup"
      onsubmit={(e) => {
        e.preventDefault();
        chooseRole(picked, startAtLogin);
      }}
    >
      <h1>Set up this machine</h1>
      <fieldset>
        <legend class="sr-only">What this machine does</legend>
        <label class="choice" class:on={picked === 'master'}>
          <input type="radio" bind:group={picked} value="master" />
          <span>
            <strong>Start a cluster here</strong>
            <span class="muted">
              This machine runs the cluster and shows the dashboard. Pick this on one machine that is usually on.
            </span>
          </span>
        </label>
        <label class="choice" class:on={picked === 'agent'}>
          <input type="radio" bind:group={picked} value="agent" />
          <span>
            <strong>Join a cluster</strong>
            <span class="muted">
              This machine works for a cluster on this network. It finds it by itself; you approve it once there.
            </span>
          </span>
        </label>
      </fieldset>
      <label class="check">
        <input type="checkbox" bind:checked={startAtLogin} />
        Start Nodeloom when I log in
      </label>
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      <button class="btn primary" type="submit" disabled={busy}>Continue</button>
    </form>
  {:else}
    <div class="card panel">
      <header>
        <h1>{view.role === 'master' ? 'This machine runs the cluster' : 'This machine works for a cluster'}</h1>
        <span class="muted version">Nodeloom {view.version}</span>
      </header>

      {#if view.error}
        <p class="problem" role="alert">Could not start: {view.error}</p>
      {/if}

      {#if view.master}
        <section>
          <h2>Dashboard</h2>
          <button class="btn primary" onclick={() => run('open_dashboard')}>Open dashboard</button>
          <p class="muted">From another device on this network, open</p>
          <code>{view.master.lan_url}</code>
        </section>
      {/if}

      {#if agent}
        <section>
          <h2>This machine</h2>
          <p class="status tone-{agent.tone}" role="status">
            <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              {#if agent.tone === 'good'}
                <path d="M3 8.5l3.2 3.2L13 4.8" />
              {:else if agent.tone === 'problem'}
                <path d="M4 4l8 8M12 4l-8 8" />
              {:else if agent.tone === 'waiting'}
                <circle cx="8" cy="8" r="5.5" /><path d="M8 5v3.3l2 1.2" />
              {:else}
                <path d="M8 2.5a5.5 5.5 0 1 1-5.5 5.5" />
              {/if}
            </svg>
            <span class="sr-only">{toneLabel[agent.tone]}:</span>
            <strong>{agent.title}</strong>
          </p>
          {#if agent.detail}<p class="muted">{agent.detail}</p>{/if}
          {#if agent.removed}
            <button class="btn" disabled={busy} onclick={() => run('reset_identity')}>Pair as a new machine</button>
          {/if}
          {#if view.code && view.role === 'agent'}
            <p class="code-line">
              <span class="muted">Confirmation code</span>
              <span class="code">{view.code}</span>
            </p>
          {/if}
        </section>
      {/if}

      <section>
        <h2>Settings</h2>
        <label class="check">
          <input
            type="checkbox"
            checked={view.autostart}
            disabled={busy}
            onchange={(e) => setAutostart(e.currentTarget.checked)}
          />
          Start Nodeloom when I log in
        </label>
        {#if confirmingChange}
          <p class="muted">Nodeloom restarts and asks again. Paired machines and this machine's identity stay.</p>
          <div class="row">
            <button class="btn" disabled={busy} onclick={() => chooseRole(null)}>Restart and choose</button>
            <button class="btn" onclick={() => (confirmingChange = false)}>Cancel</button>
          </div>
        {:else}
          <div class="row">
            <button class="btn" onclick={() => (confirmingChange = true)}>Change role…</button>
          </div>
        {/if}
        {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      </section>
    </div>
  {/if}
</main>

<style>
  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 8vh 16px 48px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 14px;
    color: var(--ink-2);
  }
  p {
    margin: 0;
  }
  .setup,
  .panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .panel header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 8px;
  }
  .version {
    font-size: 12px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .choice {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px;
    cursor: pointer;
  }
  .choice.on {
    border-color: var(--series-1);
  }
  .choice > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .choice input {
    margin-top: 3px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .status {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .tone-good svg {
    color: var(--good);
  }
  .tone-problem svg {
    color: var(--critical);
  }
  .tone-waiting svg {
    color: var(--warning);
  }
  .tone-busy svg {
    color: var(--series-1);
  }
  .code-line {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .code {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.08em;
    font-variant-numeric: tabular-nums;
  }
  .problem {
    display: flex;
    gap: 6px;
  }
  .problem::before {
    content: '✕';
    color: var(--critical);
    font-weight: 700;
  }
  code {
    background: var(--page);
    padding: 1px 5px;
    border-radius: 4px;
    overflow-wrap: anywhere;
  }
</style>
