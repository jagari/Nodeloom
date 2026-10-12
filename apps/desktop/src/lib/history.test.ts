import { describe, expect, it } from 'vitest';
import { updateHistory } from './history';
import type { ClusterSnapshot, Metrics, NodeView } from './types';

function node(id: string, metrics: Metrics | null): NodeView {
  return {
    info: { id, hostname: id, os: 'x', arch: 'x', cpu_cores: 4, total_memory_bytes: 100, runners: [] },
    metrics,
    busy_tasks: 0,
  };
}
const m = (t: number, cpu: number, used = 50): Metrics => ({
  cpu_percent: cpu,
  memory_used_bytes: used,
  memory_total_bytes: 100,
  timestamp_ms: t,
});
const snap = (...nodes: NodeView[]): ClusterSnapshot => ({ nodes, jobs: [] });

describe('updateHistory', () => {
  it('adds a sample per new timestamp and ignores repeats', () => {
    let h = updateHistory({}, snap(node('a', m(1, 10))));
    h = updateHistory(h, snap(node('a', m(1, 10))));
    h = updateHistory(h, snap(node('a', m(2, 20, 25))));
    expect(h['a']).toEqual([
      { t: 1, cpu: 10, memory: 50 },
      { t: 2, cpu: 20, memory: 25 },
    ]);
  });

  it('keeps only the newest samples', () => {
    let h = {};
    for (let t = 1; t <= 10; t++) h = updateHistory(h, snap(node('a', m(t, t))), 3);
    expect((h as Record<string, { t: number }[]>)['a']?.map((s) => s.t)).toEqual([8, 9, 10]);
  });

  it('drops nodes that left and tolerates missing metrics', () => {
    let h = updateHistory({}, snap(node('a', m(1, 1)), node('b', null)));
    expect(h['b']).toEqual([]);
    h = updateHistory(h, snap(node('b', null)));
    expect(Object.keys(h)).toEqual(['b']);
  });

  it('clamps out-of-range values', () => {
    const h = updateHistory({}, snap(node('a', m(1, 480, 900))));
    expect(h['a']?.[0]).toEqual({ t: 1, cpu: 100, memory: 100 });
  });
});
