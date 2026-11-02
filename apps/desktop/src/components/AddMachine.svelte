<script lang="ts">
  import type { MasterInfo } from '../lib/client';

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

  const command = $derived(info ? `nodeloom-agent --master ${info.join_address} --enable-fake-runner` : '');
</script>

<details class="card add" {open}>
  <summary><h2>Add a machine</h2></summary>
  <ol>
    <li>On the other computer, install Nodeloom (the same installer as this one) and open it.</li>
    <li>Choose <strong>Join a cluster</strong>. On this network it finds this cluster by itself.</li>
    <li>
      It appears here under <strong>Waiting for approval</strong> with a 6-digit code. Approve it if the code is the
      same as on that computer.
    </li>
  </ol>

  <h3>Not showing up?</h3>
  <ul>
    <li>Both computers must be on the same network (not a guest Wi-Fi or a VPN).</li>
    <li>
      On Windows, allow Nodeloom through the firewall when asked, on this computer and the other one. If the network is
      set to <em>Public</em>, switch it to <em>Private</em>.
    </li>
    {#if info}
      <li>
        <div class="manual">
          <span>On the other computer, choose <strong>Enter the address</strong> and type:</span>
          <span class="copyable">
            <code>{info.join_address}</code>
            <button class="btn small" type="button" onclick={() => copy(info.join_address)}>
              {copied === info.join_address ? 'Copied' : 'Copy'}
            </button>
          </span>
        </div>
      </li>
      <li>
        <div class="manual">
          <span>On a Raspberry Pi, Jetson or server (command line):</span>
          <span class="copyable">
            <code>{command}</code>
            <button class="btn small" type="button" onclick={() => copy(command)}>
              {copied === command ? 'Copied' : 'Copy'}
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
