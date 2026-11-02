<script lang="ts">
  import type { PairedRow } from '../lib/devices';
  import { errorText } from '../lib/i18n';
  import { t } from '../lib/locale.svelte';
  import Rich from './Rich.svelte';
  import type { NodeId } from '../lib/types';

  interface Props {
    rows: PairedRow[];
    live: boolean;
    remove: (id: NodeId) => Promise<void>;
  }
  let { rows, live, remove }: Props = $props();

  // Moves focus to Cancel when the confirmation appears (see PendingDevices).
  const focus = (node: HTMLElement) => node.focus();

  let busy = $state<NodeId | null>(null);
  /** Machine whose removal waits for a second click (not window.confirm(),
   * which the macOS webview always answers with false). */
  let confirming = $state<NodeId | null>(null);
  // Kept as caught, so the text follows a language switch.
  let error = $state<unknown>(null);

  async function onremove(row: PairedRow) {
    confirming = null;
    busy = row.id;
    error = null;
    try {
      await remove(row.id);
    } catch (e) {
      error = e;
    } finally {
      busy = null;
    }
  }
</script>

<!-- Folded: managing machines is not part of the demo, but stays one click away. -->
<details class="card paired">
  <summary><h2>{t('paired.title', { count: rows.length })}</h2></summary>
  {#if error}<p class="problem" role="alert">{errorText(error, t)}</p>{/if}
  <ul class="list">
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
          {row.online ? t('paired.online') : t('paired.offline')}
        </span>
        <span class="name">{row.name}</span>
        {#if confirming === row.id}
          <!-- Cancel takes the spot Remove had, so a double click cannot remove. -->
          <button class="btn danger" disabled={!live || busy !== null} onclick={() => onremove(row)}>
            {t('paired.removeForGood')}
          </button>
          <button class="btn" {@attach focus} onclick={() => (confirming = null)}>{t('common.cancel')}</button>
        {:else}
          <button class="btn" disabled={!live || busy !== null} onclick={() => (confirming = row.id)}>
            {busy === row.id ? t('paired.removing') : t('paired.remove')}
          </button>
        {/if}
      </li>
      {#if confirming === row.id}
        <li class="explain muted" role="status">
          <Rich text={t('paired.explain', { name: row.name })}>
            {#snippet reset()}<code>nodeloom-agent reset-identity</code>{/snippet}
            {#snippet token()}<code>--token</code>{/snippet}
            {#snippet rotate()}<code>nodeloom-master --rotate-admin-token</code>{/snippet}
          </Rich>
        </li>
      {/if}
    {/each}
  </ul>
</details>

<style>
  summary {
    cursor: pointer;
  }
  h2 {
    display: inline;
    margin: 0;
    font-size: 16px;
  }
  .list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
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
    white-space: nowrap;
    background: var(--page);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .problem {
    display: flex;
    gap: 6px;
    margin: 8px 0 0;
  }
  .problem::before {
    content: '✕';
    color: var(--critical);
    font-weight: 700;
  }
</style>
