import { describe, expect, it } from 'vitest';
import { summarizeJob, taskFraction } from './jobs';
import type { JobView, TaskState, TaskView } from './types';

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
