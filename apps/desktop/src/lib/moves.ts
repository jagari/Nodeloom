import type { ClusterSnapshot, NodeId } from './types';

/** How long a "moved to another machine" notice stays up. */
export const MOVE_NOTICE_MS = 10_000;

/** A piece that left the machine it was on. */
export interface Move {
  job_id: number;
  task_id: number;
  frame_start: number;
  frame_end: number;
  from: NodeId;
  /** The machine it went to; null while it waits for one. */
  to: NodeId | null;
  /** When the move was seen (ms, `Date.now()` clock). */
  at: number;
}

export interface MoveTracker {
  /** Last machine seen on each task, by task id. */
  owners: Record<number, NodeId>;
  /** Moves still worth showing, oldest first. */
  moves: Move[];
}

export const NO_MOVES: MoveTracker = { owners: {}, moves: [] };

/**
 * Compares a new snapshot with what was seen before and records pieces that
 * changed machine. The master never says "moved": it requeues a lost
 * machine's pieces (node becomes null) and hands them out again, usually in
 * the same update. So each task's last machine is remembered here, and a
 * different machine later, or none while queued again, counts as a move.
 *
 * A notice lasts `keepMs`; one still waiting for a machine stays until the
 * piece is picked up or the job is gone. Call it on every snapshot and on a
 * timer so notices expire even when nothing else changes.
 */
export function trackMoves(
  prev: MoveTracker,
  snapshot: ClusterSnapshot,
  now: number,
  keepMs = MOVE_NOTICE_MS,
): MoveTracker {
  const owners: Record<number, NodeId> = {};
  const fresh: Move[] = [];
  const pendingIds = new Set<number>();
  const alive = new Set<number>();

  for (const job of snapshot.jobs) {
    for (const t of job.tasks) {
      alive.add(t.task_id);
      if (t.state === 'pending') pendingIds.add(t.task_id);
      const before = prev.owners[t.task_id];
      const base = { job_id: job.job_id, task_id: t.task_id, frame_start: t.frame_start, frame_end: t.frame_end };

      if (t.node) {
        owners[t.task_id] = t.node;
        if (before && before !== t.node) fresh.push({ ...base, from: before, to: t.node, at: now });
      } else if (before) {
        // Queued again with no machine: keep who had it, so the move is
        // reported as from that machine once someone picks it up.
        owners[t.task_id] = before;
        const waiting = prev.moves.some((m) => m.task_id === t.task_id && m.to === null);
        if (t.state === 'pending' && !waiting) fresh.push({ ...base, from: before, to: null, at: now });
      }
    }
  }

  const replaced = new Set(fresh.map((m) => m.task_id));
  const kept = prev.moves.filter((m) => {
    if (!alive.has(m.task_id)) return false;
    // A newer move of the same piece supersedes this one.
    if (replaced.has(m.task_id)) return false;
    if (m.to === null) return pendingIds.has(m.task_id);
    return now - m.at < keepMs;
  });

  return { owners, moves: [...kept, ...fresh] };
}

/** Moves of one job, newest first. */
export function movesOfJob(tracker: MoveTracker, jobId: number): Move[] {
  return tracker.moves.filter((m) => m.job_id === jobId).reverse();
}
