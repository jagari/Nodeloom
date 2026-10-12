<script lang="ts">
  import { clampPercent } from '../lib/format';

  interface Props {
    /** 0..100 */
    value: number;
    label: string;
    detail?: string;
    /** Colour the fill by severity (resource meters), not by progress. */
    severity?: boolean;
    height?: number;
  }
  let { value, label, detail = '', severity = true, height = 8 }: Props = $props();

  const pct = $derived(clampPercent(value));
  const level = $derived(!severity ? 'ok' : pct >= 95 ? 'critical' : pct >= 80 ? 'warning' : 'ok');
</script>

<div class="meter">
  <div class="row">
    <span class="label">{label}</span>
    <span class="value">
      {#if level !== 'ok'}
        <!-- Severity is icon + word, never color alone. -->
        <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" class={level} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 2.2l6.2 11H1.8z" /><path d="M8 6.8v3M8 11.8v.1" />
        </svg>
        <span class="sev">{level === 'critical' ? 'Critical' : 'High'}</span>
      {/if}
      {Math.round(pct)}%
    </span>
  </div>
  <div
    class="track"
    style:height="{height}px"
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(pct)}
  >
    <div class="fill {level}" style:width="{pct}%"></div>
  </div>
  {#if detail}<div class="detail muted">{detail}</div>{/if}
</div>

<style>
  .row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 5px;
    color: var(--ink-2);
    font-size: 12px;
  }
  .value {
    color: var(--ink);
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .sev {
    font-weight: 600;
  }
  svg.warning {
    color: var(--warning);
  }
  svg.critical {
    color: var(--critical);
  }
  .track {
    background: var(--meter-track);
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--meter-fill);
    border-radius: 4px;
    transition: width 0.3s ease;
  }
  .fill.warning {
    background: var(--warning);
  }
  .fill.critical {
    background: var(--critical);
  }
  .detail {
    margin-top: 4px;
    font-size: 12px;
  }
</style>
