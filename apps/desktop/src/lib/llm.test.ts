import { describe, expect, it } from 'vitest';
import { availableModels, llmJob, parsePrompts, promptProblem, SAMPLE_PROMPTS, unitWord } from './llm';
import type { NodeView } from './types';

function node(id: string, models?: string[]): NodeView {
  return {
    info: {
      id,
      hostname: id,
      os: 'test',
      arch: 'x',
      cpu_cores: 4,
      total_memory_bytes: 1,
      runners: models ? ['fake', 'llm'] : ['fake'],
      llm_models: models,
    },
    metrics: null,
    busy_tasks: 0,
  };
}

describe('availableModels', () => {
  it('counts machines per model, most common first', () => {
    const nodes = [node('a', ['tiny:1b', 'big:7b']), node('b', ['tiny:1b']), node('c')];
    expect(availableModels(nodes)).toEqual([
      { model: 'tiny:1b', machines: 2 },
      { model: 'big:7b', machines: 1 },
    ]);
    expect(availableModels([node('c')])).toEqual([]);
  });
});

describe('prompts', () => {
  it('takes one question per non-empty line', () => {
    expect(parsePrompts(' first \n\n second\r\n')).toEqual(['first', 'second']);
  });

  it('explains what cannot be sent', () => {
    expect(promptProblem([])).toMatch(/at least one/);
    expect(promptProblem(Array(201).fill('q'))).toMatch(/At most 200/);
    expect(promptProblem(['ok', 'x'.repeat(2001)])).toMatch(/Question 2/);
    expect(promptProblem(SAMPLE_PROMPTS)).toBe('');
  });

  it('numbers the questions as frames', () => {
    expect(llmJob('tiny:1b', ['a', 'b', 'c'])).toEqual({
      runner: 'llm',
      model: 'tiny:1b',
      prompts: ['a', 'b', 'c'],
      frame_start: 1,
      frame_end: 3,
    });
  });
});

describe('unitWord', () => {
  it('speaks of questions for LLM jobs', () => {
    expect(unitWord('llm', 1)).toBe('question');
    expect(unitWord('llm', 12)).toBe('questions');
    expect(unitWord('fake', 600)).toBe('frames');
  });
});
