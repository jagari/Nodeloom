import { describe, expect, it } from 'vitest';
import { jobRole, summaryProgress } from './summary';
import type { JobView, SummaryView } from './types';

function job(job_id: number, pieces: [number, number, number, string | null][]): JobView {
  return {
    job_id,
    runner: 'llm',
    tasks: pieces.map(([frame_start, frame_end, frames_done, node], i) => ({
      task_id: job_id * 10 + i,
      node,
      frame_start,
      frame_end,
      frames_done,
      state: frames_done > frame_end - frame_start ? 'done' : 'running',
      error: null,
    })),
    outputs: [],
  };
}

const summary = (jobs: number[], parts = 12): SummaryView => ({
  summary_id: 0,
  title: 'report.pdf',
  model: 'llama3.2:1b',
  chars: 30000,
  parts,
  jobs,
  state: 'running',
  result: null,
  error: null,
});

describe('summaryProgress', () => {
  it('counts the parts across machines first', () => {
    const parts = job(4, [
      [1, 6, 3, 'a'],
      [7, 12, 6, 'b'],
    ]);
    expect(summaryProgress(summary([4]), [parts])).toEqual({
      step: 1,
      label: 'Summarizing 12 parts',
      done: 9,
      total: 12,
      machines: 2,
    });
  });

  it('names combining and final rounds, and copes with a missing job', () => {
    const combine = job(5, [[1, 3, 0, 'a']]);
    const last = job(6, [[1, 1, 0, 'b']]);
    expect(summaryProgress(summary([4, 5]), [combine])?.label).toBe('Combining 3 groups of summaries');
    expect(summaryProgress(summary([4, 5, 6]), [combine, last])?.label).toBe('Writing the final summary');
    expect(summaryProgress(summary([4, 9]), [combine])).toBeNull();
    // A short document goes straight to the final summary.
    expect(summaryProgress(summary([6], 1), [last])?.label).toBe('Writing the final summary');
  });
});

describe('jobRole', () => {
  it('labels the jobs of a summary only', () => {
    const s = summary([4, 6]);
    expect(jobRole([s], job(4, [[1, 12, 0, 'a']]))).toEqual({ label: 'Parts of “report.pdf”', unit: ['part', 'parts'] });
    expect(jobRole([summary([4, 5, 6])], job(5, [[1, 3, 0, 'a']]))?.unit).toEqual(['group', 'groups']);
    expect(jobRole([s], job(6, [[1, 1, 0, 'a']]))?.label).toBe('Final summary of “report.pdf”');
    expect(jobRole([s], job(7, [[1, 1, 0, 'a']]))).toBeNull();
    expect(jobRole(undefined, job(4, [[1, 1, 0, 'a']]))).toBeNull();
  });
});
