import type { JobView, TaskView } from './types';

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
