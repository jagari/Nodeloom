import type { Translate, UnitName } from './i18n';
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
export function summaryProgress(summary: SummaryView, jobs: JobView[], t: Translate): SummaryProgress | null {
  const lastId = summary.jobs[summary.jobs.length - 1];
  const job = jobs.find((j) => j.job_id === lastId);
  if (!job) return null;
  const s = summarizeJob(job);
  const step = summary.jobs.length;
  const machines = new Set(job.tasks.map((t) => t.node).filter(Boolean)).size;
  let label: string;
  if (step === 1 && summary.parts > 1) {
    label = t('sumstep.parts', { parts: t('count.part', { count: summary.parts }) });
  } else if (isFinal(job)) {
    label = t('sumstep.final');
  } else {
    label = t('sumstep.combine', { groups: t('count.group', { count: s.totalFrames }) });
  }
  return { step, label, done: s.doneFrames, total: s.totalFrames, machines };
}

export interface JobRole {
  label: string;
  /** What the job's units are: parts, groups of summaries, or the summary. */
  unit: UnitName;
}

/** What a job does for a summary, for its card in the job list; null if none. */
export function jobRole(summaries: SummaryView[] | undefined, job: JobView, t: Translate): JobRole | null {
  for (const summary of summaries ?? []) {
    const index = summary.jobs.indexOf(job.job_id);
    if (index < 0) continue;
    const title = summary.title;
    if (index === 0 && summary.parts > 1) return { label: t('role.parts', { title }), unit: 'part' };
    if (isFinal(job)) return { label: t('role.final', { title }), unit: 'summary' };
    return { label: t('role.combine', { title }), unit: 'group' };
  }
  return null;
}
