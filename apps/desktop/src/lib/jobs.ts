import type { ClusterSnapshot, JobView, NodeId, PairedDevice, TaskPayload, TaskView } from './types';

export type JobState = 'pending' | 'running' | 'done' | 'failed';

export interface JobSummary {
  totalFrames: number;
  doneFrames: number;
  /** 0..1 */
  fraction: number;
  state: JobState;
}

export function taskFrames(task: TaskView): number {
  return Math.max(0, task.frame_end - task.frame_start + 1);
}

export function taskFraction(task: TaskView): number {
  const total = taskFrames(task);
  return total === 0 ? 0 : Math.min(1, task.frames_done / total);
}

export function summarizeJob(job: JobView): JobSummary {
  const totalFrames = job.tasks.reduce((sum, t) => sum + taskFrames(t), 0);
  const doneFrames = job.tasks.reduce((sum, t) => sum + Math.min(t.frames_done, taskFrames(t)), 0);
  const fraction = totalFrames === 0 ? 0 : doneFrames / totalFrames;

  let state: JobState = 'pending';
  if (job.tasks.some((t) => t.state === 'failed')) state = 'failed';
  else if (job.tasks.length > 0 && job.tasks.every((t) => t.state === 'done')) state = 'done';
  else if (job.tasks.some((t) => t.state === 'running' || t.state === 'done')) state = 'running';

  return { totalFrames, doneFrames, fraction, state };
}

/**
 * The demo the presentation runs: the test runner (sleep only) over 600
 * frames at 50 ms each, so 30 s on one machine, about 10 s on three. Long
 * enough to watch the pieces fill and to pull a machine mid-way.
 */
export const DEMO_JOB: TaskPayload = { runner: 'fake', frame_start: 1, frame_end: 600, millis_per_frame: 50 };

/** One piece of a job laid out on the job's frame axis. */
export interface Segment {
  task: TaskView;
  /** Offset and width on the job's frame axis, in percent. */
  left: number;
  width: number;
  /** 0..1 */
  fraction: number;
}

/**
 * Places every task on one horizontal frame axis, from the job's first frame
 * to its last, in frame order. Widths follow frame counts, so a machine that
 * got a bigger piece gets a longer bar.
 */
export function jobSegments(job: JobView): { first: number; last: number; segments: Segment[] } {
  if (job.tasks.length === 0) return { first: 0, last: 0, segments: [] };
  const first = Math.min(...job.tasks.map((t) => t.frame_start));
  const last = Math.max(...job.tasks.map((t) => t.frame_end));
  const span = last - first + 1;
  const segments = [...job.tasks]
    .sort((a, b) => a.frame_start - b.frame_start || a.task_id - b.task_id)
    .map((task) => ({
      task,
      left: ((task.frame_start - first) / span) * 100,
      width: (taskFrames(task) / span) * 100,
      fraction: taskFraction(task),
    }));
  return { first, last, segments };
}

/** What a machine is working on right now. */
export interface CurrentWork {
  job_id: number;
  task_id: number;
  frame_start: number;
  frame_end: number;
  frames_done: number;
  /** 0..1 */
  fraction: number;
}

/** Running pieces per machine, oldest job first. Idle machines are absent. */
export function workByNode(snapshot: ClusterSnapshot): Map<NodeId, CurrentWork[]> {
  const work = new Map<NodeId, CurrentWork[]>();
  for (const job of snapshot.jobs) {
    for (const t of job.tasks) {
      if (t.state !== 'running' || !t.node) continue;
      const list = work.get(t.node) ?? [];
      list.push({
        job_id: job.job_id,
        task_id: t.task_id,
        frame_start: t.frame_start,
        frame_end: t.frame_end,
        frames_done: t.frames_done,
        fraction: taskFraction(t),
      });
      work.set(t.node, list);
    }
  }
  return work;
}

/** Categorical slots defined in app.css (`--series-1` … `--series-6`). */
export const SERIES_SLOTS = 6;

/**
 * A stable colour slot (1-based) per machine, or 0 for "no slot" (shown in
 * a neutral colour, still with its name). Slots follow the paired-device
 * list, which only grows while the master runs, so a machine that leaves
 * does not repaint the others mid-demo. Machines not in that list (rare)
 * take the next free slots in id order.
 */
export function nodeSlots(devices: PairedDevice[], online: NodeId[]): Map<NodeId, number> {
  const order: NodeId[] = devices.map((d) => d.id);
  for (const id of [...online].sort()) if (!order.includes(id)) order.push(id);
  const slots = new Map<NodeId, number>();
  order.forEach((id, i) => slots.set(id, i < SERIES_SLOTS ? i + 1 : 0));
  return slots;
}

/** CSS colour for a slot from `nodeSlots`. */
export function slotColor(slot: number | undefined): string {
  return slot ? `var(--series-${slot})` : 'var(--muted)';
}
