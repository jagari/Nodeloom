<script lang="ts">
  import { hasCable, joinAddresses, type LinkKind, type MasterInfo } from '../lib/client';
  import { t } from '../lib/locale.svelte';
  import Rich from './Rich.svelte';

  interface Props {
    info: MasterInfo | null;
    /** Open by default while this machine is still alone. */
    open: boolean;
  }
  let { info, open }: Props = $props();

  let copied = $state('');

  // The clipboard only works on secure pages (loopback, https); on a LAN
  // address the text stays selectable instead.
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = text;
      setTimeout(() => (copied = ''), 1500);
    } catch {
      copied = '';
    }
  }

  const addresses = $derived(info ? joinAddresses(info) : []);
  const cable = $derived(hasCable(addresses));
  const linkKey = {
    direct: 'link.direct',
    thunderbolt: 'link.thunderbolt',
    wired: 'link.wired',
    wifi: 'link.wifi',
    other: 'link.other',
  } as const satisfies Record<LinkKind, string>;

  const command = $derived(info ? `nodeloom-agent --master ${info.join_address} --enable-fake-runner` : '');
</script>

<details class="card add" {open}>
  <summary><h2>{t('add.title')}</h2></summary>
  <ol>
    <li>{t('add.step1')}</li>
    <!-- "Join a cluster" and "Enter the address" name buttons in the desktop app's own page, which is English for now. -->
    <li>
      <Rich text={t('add.step2')}>{#snippet join()}<strong>Join a cluster</strong>{/snippet}</Rich>
    </li>
    <li>
      <Rich text={t('add.step3')}>{#snippet waiting()}<strong>{t('pending.title')}</strong>{/snippet}</Rich>
    </li>
  </ol>

  <h3>{t('add.trouble')}</h3>
  <ul>
    <li>{t('add.network')}</li>
    <li>
      <Rich text={t('add.firewall')}>
        {#snippet pub()}<em>{t('add.public')}</em>{/snippet}
        {#snippet priv()}<em>{t('add.private')}</em>{/snippet}
      </Rich>
    </li>
    {#if info}
      <li>
        <div class="manual">
          <span><Rich text={t('add.manual')}>{#snippet enter()}<strong>Enter the address</strong>{/snippet}</Rich></span>
          {#if cable}<span class="hint">{t('add.cable')}</span>{/if}
          <!-- Best link first; the kind is a word, not a colour. -->
          <ul class="addresses">
            {#each addresses as a (a.address + a.interface)}
              <li class="copyable">
                <span class="kind">{t(linkKey[a.kind])}{#if a.interface}<span class="muted"> · {a.interface}</span>{/if}</span>
                <code>{a.address}</code>
                <button class="btn small" type="button" onclick={() => copy(a.address)}>
                  {copied === a.address ? t('common.copied') : t('common.copy')}
                </button>
              </li>
            {/each}
          </ul>
        </div>
      </li>
      <li>
        <div class="manual">
          <span>{t('add.cli')}</span>
          <span class="copyable">
            <code>{command}</code>
            <button class="btn small" type="button" onclick={() => copy(command)}>
              {copied === command ? t('common.copied') : t('common.copy')}
            </button>
          </span>
        </div>
      </li>
    {/if}
  </ul>
</details>

<style>
  .add {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  summary {
    cursor: pointer;
    list-style-position: outside;
  }
  h2 {
    display: inline;
    margin: 0;
    font-size: 16px;
  }
  h3 {
    margin: 14px 0 4px;
    font-size: 14px;
    color: var(--ink-2);
  }
  ol,
  ul {
    margin: 8px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .manual {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .addresses {
    list-style: none;
    padding-left: 0;
    margin: 4px 0 0;
    gap: 4px;
  }
  .kind {
    min-width: 9em;
    font-size: 13px;
  }
  .hint {
    font-size: 13px;
    color: var(--ink-2);
  }
  .copyable {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  code {
    background: var(--page);
    border: 1px solid var(--border);
    padding: 2px 6px;
    border-radius: 4px;
    user-select: all;
    overflow-wrap: anywhere;
  }
  .small {
    padding: 3px 10px;
    font-size: 12px;
  }
</style>
