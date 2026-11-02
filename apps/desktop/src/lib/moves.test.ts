import { describe, expect, it } from 'vitest';
import { MOVE_NOTICE_MS, NO_MOVES, movesOfJob, trackMoves, type MoveTracker } from './moves';
import type { ClusterSnapshot, NodeId, TaskState, TaskView } from './types';

const task = (id: number, node: NodeId | null, state: TaskState = 'running'): TaskView => ({
  task_id: id,
  node,
  frame_start: id * 100 + 1,
  frame_end: id * 100 + 100,
  frames_done: 0,
  state,
  error: null,
});
const snap = (...tasks: TaskView[]): ClusterSnapshot => ({
  nodes: [],
  jobs: [{ job_id: 7, runner: 'fake', tasks, outputs: [] }],
  pending: [],
  devices: [],
});

/** Feeds snapshots in order, one second apart. */
function feed(...snapshots: ClusterSnapshot[]): MoveTracker {
  return snapshots.reduce((t, s, i) => trackMoves(t, s, 1000 * (i + 1)), NO_MOVES);
}

describe('trackMoves', () => {
  it('sees nothing on the first snapshot or while machines keep their pieces', () => {
    expect(feed(snap(task(1, 'mac'))).moves).toEqual([]);
    expect(feed(snap(task(1, 'mac')), snap(task(1, 'mac'))).moves).toEqual([]);
  });

  it('reports a piece that changed machine between snapshots', () => {
    const t = feed(snap(task(1, 'mac'), task(2, 'pi')), snap(task(1, 'pi'), task(2, 'pi')));
    expect(t.moves).toEqual([
      { job_id: 7, task_id: 1, frame_start: 101, frame_end: 200, from: 'mac', to: 'pi', at: 2000 },
    ]);
  });

  it('a requeued piece waits as "from mac", then shows where it went', () => {
    const waiting = feed(snap(task(1, 'mac')), snap(task(1, null, 'pending')));
    expect(waiting.moves).toMatchObject([{ from: 'mac', to: null }]);

    // Still waiting long after the notice time: the notice stays.
    const later = trackMoves(waiting, snap(task(1, null, 'pending')), 60_000);
    expect(later.moves).toMatchObject([{ from: 'mac', to: null, at: 2000 }]);

    const picked = trackMoves(later, snap(task(1, 'pi')), 61_000);
    expect(picked.moves).toEqual([
      { job_id: 7, task_id: 1, frame_start: 101, frame_end: 200, from: 'mac', to: 'pi', at: 61_000 },
    ]);
  });

  it('a notice expires after the notice time', () => {
    const moved = feed(snap(task(1, 'mac')), snap(task(1, 'pi')));
    expect(trackMoves(moved, snap(task(1, 'pi')), 2000 + MOVE_NOTICE_MS - 1).moves).toHaveLength(1);
    expect(trackMoves(moved, snap(task(1, 'pi')), 2000 + MOVE_NOTICE_MS).moves).toHaveLength(0);
  });

  it('back on the same machine (it reconnected) is not a move', () => {
    const t = feed(snap(task(1, 'mac')), snap(task(1, null, 'pending')), snap(task(1, 'mac')));
    expect(t.moves).toEqual([]);
  });

  it('forgets pieces whose job is gone', () => {
    const moved = feed(snap(task(1, 'mac')), snap(task(1, 'pi')));
    const empty: ClusterSnapshot = { nodes: [], jobs: [], pending: [], devices: [] };
    expect(trackMoves(moved, empty, 3000)).toEqual({ owners: {}, moves: [] });
  });

  it('movesOfJob lists the newest first, only for that job', () => {
    const t = feed(snap(task(1, 'a'), task(2, 'b')), snap(task(1, 'b'), task(2, 'b')), snap(task(1, 'b'), task(2, 'c')));
    expect(movesOfJob(t, 7).map((m) => m.task_id)).toEqual([2, 1]);
    expect(movesOfJob(t, 8)).toEqual([]);
  });
});
