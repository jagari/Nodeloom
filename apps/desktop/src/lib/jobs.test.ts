import { describe, expect, it } from 'vitest';
import { DEMO_JOB, jobSegments, nodeSlots, slotColor, summarizeJob, taskFraction, workByNode } from './jobs';
import type { ClusterSnapshot, JobView, TaskState, TaskView } from './types';

const task = (id: number, start: number, end: number, done: number, state: TaskState): TaskView => ({
  task_id: id,
  node: 'a',
  frame_start: start,
  frame_end: end,
  frames_done: done,
  state,
  error: null,
});
const job = (...tasks: TaskView[]): JobView => ({ job_id: 1, tasks });

describe('summarizeJob', () => {
  it('sums frames across tasks', () => {
    const s = summarizeJob(job(task(1, 1, 10, 5, 'running'), task(2, 11, 20, 10, 'done')));
    expect(s).toMatchObject({ totalFrames: 20, doneFrames: 15, fraction: 0.75, state: 'running' });
  });

  it('is done only when every task is done', () => {
    expect(summarizeJob(job(task(1, 1, 2, 2, 'done'), task(2, 3, 4, 2, 'done'))).state).toBe('done');
    expect(summarizeJob(job(task(1, 1, 2, 2, 'done'), task(2, 3, 4, 0, 'pending'))).state).toBe('running');
  });

  it('a failed task fails the job', () => {
    expect(summarizeJob(job(task(1, 1, 2, 2, 'done'), task(2, 3, 4, 0, 'failed'))).state).toBe('failed');
  });

  it('pending when nothing started, and safe when empty', () => {
    expect(summarizeJob(job(task(1, 1, 2, 0, 'pending'))).state).toBe('pending');
    expect(summarizeJob(job())).toMatchObject({ totalFrames: 0, fraction: 0, state: 'pending' });
  });
});

describe('taskFraction', () => {
  it('caps at 1', () => {
    expect(taskFraction(task(1, 1, 4, 9, 'running'))).toBe(1);
    expect(taskFraction(task(1, 1, 4, 1, 'running'))).toBe(0.25);
  });
});

describe('jobSegments', () => {
  it('lays tasks out on the frame axis in frame order', () => {
    const { first, last, segments } = jobSegments(
      job(task(2, 301, 600, 150, 'running'), task(1, 1, 300, 300, 'done')),
    );
    expect([first, last]).toEqual([1, 600]);
    expect(segments.map((s) => s.task.task_id)).toEqual([1, 2]);
    expect(segments[0]).toMatchObject({ left: 0, width: 50, fraction: 1 });
    expect(segments[1]).toMatchObject({ left: 50, width: 50, fraction: 0.5 });
  });

  it('widths follow frame counts and add up to the whole axis', () => {
    const { segments } = jobSegments(job(task(1, 10, 19, 0, 'running'), task(2, 20, 49, 0, 'running')));
    expect(segments.map((s) => s.width)).toEqual([25, 75]);
    expect(segments[1]!.left).toBe(25);
  });

  it('is empty for a job without tasks', () => {
    expect(jobSegments(job()).segments).toEqual([]);
  });
});

describe('workByNode', () => {
  const snap = (jobs: JobView[]): ClusterSnapshot => ({ nodes: [], jobs, pending: [], devices: [] });

  it('lists running pieces per machine and leaves idle machines out', () => {
    const running = { ...task(1, 1, 100, 25, 'running'), node: 'mac' };
    const done = { ...task(2, 101, 200, 100, 'done'), node: 'pi' };
    const queued = { ...task(3, 201, 300, 0, 'pending'), node: null };
    const work = workByNode(snap([{ job_id: 4, tasks: [running, done, queued] }]));
    expect([...work.keys()]).toEqual(['mac']);
    expect(work.get('mac')).toEqual([
      { job_id: 4, task_id: 1, frame_start: 1, frame_end: 100, frames_done: 25, fraction: 0.25 },
    ]);
  });
});

describe('nodeSlots', () => {
  it('follows the paired list, so a machine leaving does not repaint the rest', () => {
    const devices = [
      { id: 'b', name: 'b' },
      { id: 'a', name: 'a' },
      { id: 'c', name: 'c' },
    ];
    const all = nodeSlots(devices, ['a', 'b', 'c']);
    const without = nodeSlots(devices, ['a', 'c']);
    expect(all.get('b')).toBe(1);
    expect(all.get('c')).toBe(3);
    expect(without.get('c')).toBe(3);
  });

  it('gives unpaired machines the next slots and runs out to neutral', () => {
    const slots = nodeSlots([], ['g', 'f', 'e', 'd', 'c', 'b', 'a']);
    expect(slots.get('a')).toBe(1);
    expect(slots.get('f')).toBe(6);
    expect(slots.get('g')).toBe(0);
    expect(slotColor(slots.get('a'))).toBe('var(--series-1)');
    expect(slotColor(0)).toBe('var(--muted)');
  });
});

describe('DEMO_JOB', () => {
  it('is the test runner over 600 frames at 50 ms (30 s on one machine)', () => {
    expect(DEMO_JOB).toEqual({ runner: 'fake', frame_start: 1, frame_end: 600, millis_per_frame: 50 });
  });
});
