import type { Translate, UnitName } from './i18n';
import type { NodeView, RunnerKind, TaskPayload } from './types';

/**
 * Limits for typed questions. `MAX_PROMPTS` mirrors nodeloom-core's
 * validate.rs; its prompt length limit is higher (for document parts, see
 * summary.ts), and typed questions stay short to fit the 64 KiB request.
 */
export const MAX_PROMPTS = 200;
export const MAX_PROMPT_CHARS = 2000;

/** Short questions with one-sentence answers: quick on a small CPU model. */
export const SAMPLE_PROMPTS = [
  'In one sentence, what is photosynthesis?',
  'In one sentence, why is the sky blue?',
  'In one sentence, what does a CPU do?',
  'Name three primary colors.',
  'In one sentence, what is a black hole?',
  'Translate "good morning" into Korean.',
  'In one sentence, what is machine learning?',
  'What is the capital of Australia?',
  'In one sentence, how does a rainbow form?',
  'Give one tip for writing clean code.',
  'In one sentence, what is DNA?',
  'In one sentence, why do we have seasons?',
];

export interface ModelChoice {
  model: string;
  machines: number;
}

/** Models offered by the connected machines, most widely installed first. */
export function availableModels(nodes: NodeView[]): ModelChoice[] {
  const count = new Map<string, number>();
  for (const n of nodes) {
    for (const m of n.info.llm_models ?? []) count.set(m, (count.get(m) ?? 0) + 1);
  }
  return [...count]
    .map(([model, machines]) => ({ model, machines }))
    .sort((a, b) => b.machines - a.machines || a.model.localeCompare(b.model));
}

/** One prompt per non-empty line. */
export function parsePrompts(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** Why these prompts cannot be sent, or '' when they can. */
export function promptProblem(prompts: string[], t: Translate): string {
  if (prompts.length === 0) return t('prompt.none');
  if (prompts.length > MAX_PROMPTS) return t('prompt.tooMany', { max: MAX_PROMPTS });
  const long = prompts.findIndex((p) => p.length > MAX_PROMPT_CHARS);
  if (long >= 0) return t('prompt.tooLong', { n: long + 1, max: MAX_PROMPT_CHARS });
  return '';
}

export function llmJob(model: string, prompts: string[]): TaskPayload {
  return { runner: 'llm', model, prompts, frame_start: 1, frame_end: prompts.length };
}

/** What a job's units are called on screen: an LLM job's are questions. */
export function unitOf(runner: RunnerKind): UnitName {
  return runner === 'llm' ? 'question' : 'frame';
}
