import { summarizeJob } from './jobs';
import type { JobView, SummaryView } from './types';

export interface SummaryProgress {
  /** 1 for the parts, then one per combining round. */
  step: number;
  /** What the current round does, for people. */
  label: string;
  done: number;
  total: number;
  machines: number;
}

/** Whether a round is the one writing the final summary: a single prompt. */
function isFinal(job: JobView): boolean {
  return summarizeJob(job).totalFrames === 1;
}

/** Where a summary is, from its newest job. Null when that job is gone. */
export function summaryProgress(summary: SummaryView, jobs: JobView[]): SummaryProgress | null {
  const lastId = summary.jobs[summary.jobs.length - 1];
  const job = jobs.find((j) => j.job_id === lastId);
  if (!job) return null;
  const s = summarizeJob(job);
  const step = summary.jobs.length;
  const machines = new Set(job.tasks.map((t) => t.node).filter(Boolean)).size;
  let label: string;
  if (step === 1 && summary.parts > 1) {
    label = `Summarizing ${summary.parts} parts`;
  } else if (isFinal(job)) {
    label = 'Writing the final summary';
  } else {
    label = `Combining ${s.totalFrames} groups of summaries`;
  }
  return { step, label, done: s.doneFrames, total: s.totalFrames, machines };
}

export interface JobRole {
  label: string;
  /** What the job's units are: parts, groups of summaries, or the summary. */
  unit: [singular: string, plural: string];
}

/** What a job does for a summary, for its card in the job list; null if none. */
export function jobRole(summaries: SummaryView[] | undefined, job: JobView): JobRole | null {
  for (const summary of summaries ?? []) {
    const index = summary.jobs.indexOf(job.job_id);
    if (index < 0) continue;
    if (index === 0 && summary.parts > 1) return { label: `Parts of “${summary.title}”`, unit: ['part', 'parts'] };
    if (isFinal(job)) return { label: `Final summary of “${summary.title}”`, unit: ['summary', 'summaries'] };
    return { label: `Combining summaries of “${summary.title}”`, unit: ['group', 'groups'] };
  }
  return null;
}
