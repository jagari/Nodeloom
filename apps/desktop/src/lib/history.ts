import { clampPercent } from './format';
import type { ClusterSnapshot, NodeId } from './types';

export interface Sample {
  /** Agent-side timestamp in ms; identical timestamps are the same sample. */
  t: number;
  cpu: number;
  memory: number;
}

export type History = Record<NodeId, Sample[]>;

export const HISTORY_LENGTH = 60;

/**
 * Appends the newest metrics of every node to its history. Snapshots arrive
 * more often than agents report, so a sample is only added when its timestamp
 * differs from the last one. Nodes that disappeared are dropped.
 */
export function updateHistory(prev: History, snapshot: ClusterSnapshot, max = HISTORY_LENGTH): History {
  const next: History = {};
  for (const node of snapshot.nodes) {
    const id = node.info.id;
    const samples = prev[id] ?? [];
    const m = node.metrics;
    if (!m || samples[samples.length - 1]?.t === m.timestamp_ms) {
      next[id] = samples;
      continue;
    }
    const total = m.memory_total_bytes || 1;
    const sample: Sample = {
      t: m.timestamp_ms,
      cpu: clampPercent(m.cpu_percent),
      memory: clampPercent((m.memory_used_bytes / total) * 100),
    };
    next[id] = [...samples, sample].slice(-max);
  }
  return next;
}
