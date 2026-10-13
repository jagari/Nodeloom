<script lang="ts">
  import { shownName } from '../lib/devices';
  import type { PendingDevice } from '../lib/types';

  // Moves focus to the safe choice when a confirmation appears, so keyboard
  // and screen-reader users are not left on a button that just vanished.
  const focus = (node: HTMLElement) => node.focus();

  interface Props {
    pending: PendingDevice[];
    /** Why a request needs a second look, by public key. */
    flags: Map<string, string>;
    /** Buttons work only while the view is live; a stale list may be wrong. */
    live: boolean;
    approve: (publicKey: string) => Promise<void>;
    ignore: (publicKey: string) => Promise<void>;
  }
  let { pending, flags, live, approve, ignore }: Props = $props();

  let busy = $state<string | null>(null);
  /** Request whose approval waits for a second click. */
  let confirming = $state<string | null>(null);
  let errors = $state<Record<string, string>>({});

  // A two-step button in the card instead of window.confirm(): the macOS
  // webview answers confirm() with false, and the card itself shows which
  // machine (code, address) the second click approves.
  async function act(request: PendingDevice, kind: 'approve' | 'ignore') {
    if (kind === 'approve' && flags.has(request.public_key) && confirming !== request.public_key) {
      confirming = request.public_key;
      return;
    }
    confirming = null;
    busy = request.public_key;
    errors = { ...errors, [request.public_key]: '' };
    try {
      await (kind === 'approve' ? approve : ignore)(request.public_key);
    } catch (e) {
      errors = { ...errors, [request.public_key]: e instanceof Error ? e.message : String(e) };
    } finally {
      busy = null;
    }
  }
</script>

<section aria-labelledby="pending-title">
  <h2 id="pending-title">Waiting for approval</h2>
  <p class="muted hint">
    Approve a machine only if it shows the same code. On the machine, the agent prints it at start, or run
    <code>nodeloom-agent pairing-code</code>.
  </p>
  <div class="list">
    {#each pending as request (request.public_key)}
      {@const flag = flags.get(request.public_key)}
      {@const error = errors[request.public_key]}
      <article class="card request">
        <div class="who">
          <h3>{shownName(request)}</h3>
          <div class="muted sub">
            {request.info.os} · {request.info.arch}{request.address ? ` · ${request.address}` : ''}
          </div>
        </div>
        <div class="code" aria-label="Confirmation code {request.code}">{request.code}</div>
        {#if flag}
          <p class="note warn" role="status">{flag}</p>
        {/if}
        {#if error}
          <p class="note bad" role="alert">{error}</p>
        {/if}
        {#if confirming === request.public_key}
          <p class="note warn" role="status">
            Approve {shownName(request)} with code {request.code}{request.address ? ` from ${request.address}` : ''}?
            Check the code on the machine itself first.
          </p>
          <!-- Cancel takes the spot Approve had, so a double click cannot approve. -->
          <div class="actions">
            <button class="btn primary" disabled={!live || busy !== null} onclick={() => act(request, 'approve')}>
              Approve anyway
            </button>
            <button class="btn" {@attach focus} onclick={() => (confirming = null)}>Cancel</button>
          </div>
        {:else}
          <div class="actions">
            <button class="btn" disabled={!live || busy !== null} onclick={() => act(request, 'ignore')}>Ignore</button>
            <button class="btn primary" disabled={!live || busy !== null} onclick={() => act(request, 'approve')}>
              {busy === request.public_key ? 'Working…' : 'Approve'}
            </button>
          </div>
        {/if}
      </article>
    {/each}
  </div>
</section>

<style>
  h2 {
    margin: 0 0 4px;
    font-size: 16px;
  }
  .hint {
    margin: 0 0 12px;
  }
  .list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }
  .request {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-color: var(--series-1);
  }
  h3 {
    margin: 0;
    font-size: 15px;
    overflow-wrap: anywhere;
  }
  .sub {
    font-size: 12px;
  }
  .code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.06em;
    font-variant-numeric: tabular-nums;
  }
  .note {
    margin: 0;
    display: flex;
    gap: 6px;
    font-size: 13px;
  }
  .note::before {
    font-weight: 700;
  }
  .warn::before {
    content: '!';
    color: var(--warning);
  }
  .bad::before {
    content: '✕';
    color: var(--critical);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  code {
    background: var(--surface);
    padding: 1px 5px;
    border-radius: 4px;
  }
</style>
