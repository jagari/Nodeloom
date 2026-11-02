<script lang="ts">
  import { parseEndpoint } from '../lib/client';
  import { t } from '../lib/locale.svelte';
  import Rich from './Rich.svelte';

  interface Props {
    initialHost: string;
    problem: string;
    connect: (host: string, token: string) => void;
  }
  let { initialHost, problem, connect }: Props = $props();

  // Only the starting value matters; the user edits it from here.
  // svelte-ignore state_referenced_locally
  let host = $state(initialHost);
  let token = $state('');
  let invalid = $state(false);

  function onsubmit(event: SubmitEvent) {
    event.preventDefault();
    invalid = parseEndpoint(host) === null;
    if (!invalid) connect(host.trim(), token.trim());
  }
</script>

<form class="card connect" {onsubmit}>
  <h1>{t('connect.title')}</h1>
  <p class="muted">
    <Rich text={t('connect.intro')}>
      {#snippet master()}<code>nodeloom-master</code>{/snippet}
      {#snippet file()}<code>admin-token</code>{/snippet}
    </Rich>
  </p>
  <label class="field">
    <span>{t('connect.address')}</span>
    <input bind:value={host} placeholder="192.168.1.20:7878" required spellcheck="false" />
  </label>
  {#if invalid}<p class="problem" role="alert">{t('connect.badAddress')}</p>{/if}
  <label class="field">
    <span>{t('connect.token')}</span>
    <input bind:value={token} type="password" autocomplete="off" required spellcheck="false" />
  </label>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
  <button class="btn primary" type="submit">{t('connect.submit')}</button>
</form>

<style>
  .connect {
    max-width: 420px;
    margin: 12vh auto 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  p {
    margin: 0;
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
  }
</style>
