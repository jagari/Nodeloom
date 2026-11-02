import { describe, expect, it } from 'vitest';
import { translator } from './i18n';
import { countChars, documentProblem, estimateParts, joinPages, MAX_DOCUMENT_CHARS, pageText } from './pdf';

const en = translator('en');

describe('pageText', () => {
  it('keeps line breaks and joins hyphenated words', () => {
    const runs = [
      { str: 'Distributed comput-', hasEOL: true },
      { str: 'ing splits work.  ', hasEOL: true },
      { str: '분산 처리', hasEOL: false },
      { str: '는 빠르다.', hasEOL: true },
    ];
    expect(pageText(runs)).toBe('Distributed computing splits work.\n분산 처리는 빠르다.');
  });

  it('keeps a dash before a capital or a number', () => {
    expect(pageText([{ str: 'GPT-', hasEOL: true }, { str: '4 and A-', hasEOL: true }, { str: 'B', hasEOL: false }])).toBe(
      'GPT-\n4 and A-\nB',
    );
  });
});

describe('joinPages', () => {
  it('makes pages paragraphs and drops empty ones', () => {
    expect(joinPages(['one', '', 'two'])).toBe('one\n\ntwo');
  });
});

describe('documentProblem', () => {
  it('explains an empty (scanned) document', () => {
    expect(documentProblem(' \n ', en)).toMatch(/scanned PDF/);
  });

  it('counts characters the way the master does', () => {
    expect(countChars('가나😀')).toBe(3);
    expect(documentProblem('가'.repeat(MAX_DOCUMENT_CHARS), en)).toBe('');
    expect(documentProblem('가'.repeat(MAX_DOCUMENT_CHARS + 1), en)).toMatch(/at most 200,000/);
  });
});

describe('estimateParts', () => {
  it('is at least one', () => {
    expect(estimateParts(10)).toBe(1);
    expect(estimateParts(25_000)).toBeGreaterThanOrEqual(10);
  });
});
