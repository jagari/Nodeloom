import { describe, expect, it } from 'vitest';
import {
  errorText,
  initialLang,
  LANG_KEY,
  loadLang,
  MESSAGES,
  MessageError,
  saveLang,
  splitSlots,
  translate,
  translator,
  unitWord,
  type Message,
} from './i18n';
import { jobRole } from './summary';

const forms = (m: Message) => (typeof m === 'string' ? [m] : [m.one, m.other]);
const placeholders = (m: Message) => new Set(forms(m).flatMap((f) => [...f.matchAll(/\{(\w+)\}/g)].map((x) => x[1])));

describe('dictionaries', () => {
  it('have the same keys in English and Korean', () => {
    expect(Object.keys(MESSAGES.ko).sort()).toEqual(Object.keys(MESSAGES.en).sort());
  });

  it('have no empty text', () => {
    for (const lang of ['en', 'ko'] as const) {
      for (const [key, message] of Object.entries(MESSAGES[lang])) {
        for (const f of forms(message)) expect(f.trim(), `${lang} ${key}`).not.toBe('');
      }
    }
  });

  it('use the same placeholders in both languages', () => {
    for (const [key, message] of Object.entries(MESSAGES.en)) {
      const ko = MESSAGES.ko[key as keyof typeof MESSAGES.ko];
      expect(placeholders(ko), key).toEqual(placeholders(message));
    }
  });
});

describe('translate', () => {
  it('fills placeholders and groups digits per language', () => {
    expect(translate('en', 'job.name', { id: 3 })).toBe('Job 3');
    expect(translate('ko', 'job.name', { id: 3 })).toBe('작업 3');
    expect(translate('en', 'doc.tooLong', { chars: 200001, max: 200000 })).toContain('200,001');
    expect(translate('ko', 'summary.readFailed', { name: 'a.pdf', error: 'x' })).toBe('a.pdf 파일을 읽지 못했습니다: x');
  });

  it('leaves a placeholder without a value for markup', () => {
    expect(translate('en', 'machines.none')).toBe('No machine is online yet. See {add}.');
  });

  it('counts the way each language does', () => {
    expect(translate('en', 'count.machine', { count: 1 })).toBe('1 machine');
    expect(translate('en', 'count.machine', { count: 2 })).toBe('2 machines');
    expect(translate('en', 'count.machine', { count: 0 })).toBe('0 machines');
    expect(translate('ko', 'count.machine', { count: 1 })).toBe('기기 1대');
    expect(translate('ko', 'count.machine', { count: 2 })).toBe('기기 2대');
    expect(unitWord(translator('en'), 'frame', 1)).toBe('frame');
    expect(unitWord(translator('en'), 'frame', 600)).toBe('frames');
    expect(unitWord(translator('ko'), 'frame', 600)).toBe('프레임');
  });

  it('builds Korean sentences in lib functions too', () => {
    const job = { job_id: 4, runner: 'llm' as const, tasks: [], outputs: [] };
    const summary = { summary_id: 1, title: 'r.pdf', model: 'm', chars: 1, parts: 3, jobs: [4], state: 'running' as const, result: null, error: null };
    expect(jobRole([summary], job, translator('ko'))?.label).toBe('“r.pdf”의 부분');
  });
});

describe('splitSlots', () => {
  it('cuts text around markup places', () => {
    expect(splitSlots('See {add}.')).toEqual([{ text: 'See ' }, { slot: 'add' }, { text: '.' }]);
    expect(splitSlots('{a}{b}')).toEqual([{ slot: 'a' }, { slot: 'b' }]);
    expect(splitSlots('plain')).toEqual([{ text: 'plain' }]);
  });
});

describe('errors', () => {
  it('words its own errors in the current language, passes others on', () => {
    const e = new MessageError('err.status', { status: '500' });
    expect(e.message).toBe('master answered 500');
    expect(errorText(e, translator('ko'))).toBe('마스터 응답 500');
    expect(errorText(new Error('x is already paired'), translator('ko'))).toBe('x is already paired');
    expect(errorText('odd', translator('en'))).toBe('odd');
  });
});

describe('starting language', () => {
  it('prefers the saved choice, then a Korean browser, then English', () => {
    expect(initialLang('ko', ['en-US'])).toBe('ko');
    expect(initialLang('en', ['ko-KR'])).toBe('en');
    expect(initialLang(null, ['ko-KR', 'en'])).toBe('ko');
    expect(initialLang(null, ['ko'])).toBe('ko');
    expect(initialLang(null, ['en-GB', 'ko'])).toBe('en');
    expect(initialLang(null, [])).toBe('en');
  });

  it('ignores an odd saved value', () => {
    expect(initialLang('fr', ['en-US'])).toBe('en');
    expect(initialLang('', ['ko-KR'])).toBe('ko');
    expect(initialLang('KO', ['en'])).toBe('en');
  });

  it('works when storage is blocked', () => {
    const blocked = () => {
      throw new Error('SecurityError');
    };
    expect(loadLang(blocked)).toBeNull();
    expect(() => saveLang(blocked, 'ko')).not.toThrow();
    const failing = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('full'); } };
    expect(loadLang(() => failing)).toBeNull();
    expect(() => saveLang(() => failing, 'ko')).not.toThrow();
    expect(initialLang(loadLang(blocked), ['en'])).toBe('en');
  });

  it('saves and reads back the choice', () => {
    const store = new Map<string, string>();
    const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
    saveLang(() => storage, 'ko');
    expect(store.get(LANG_KEY)).toBe('ko');
    expect(initialLang(loadLang(() => storage), ['en'])).toBe('ko');
  });
});
