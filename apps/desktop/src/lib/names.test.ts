import { describe, expect, it } from 'vitest';
import { displayNames } from './names';
import type { NodeView } from './types';

const node = (id: string, hostname: string): NodeView => ({
  info: { id, hostname, os: '', arch: '', cpu_cores: 1, total_memory_bytes: 1, runners: [] },
  metrics: null,
  busy_tasks: 0,
});

describe('displayNames', () => {
  it('keeps unique hostnames untouched', () => {
    const names = displayNames([node('aaaa1111', 'studio'), node('bbbb2222', 'laptop')]);
    expect(names.get('aaaa1111')).toBe('studio');
    expect(names.get('bbbb2222')).toBe('laptop');
  });

  it('adds a short id only to duplicated hostnames', () => {
    const names = displayNames([node('aaaa1111', 'mac'), node('bbbb2222', 'mac'), node('cccc3333', 'pc')]);
    expect(names.get('aaaa1111')).toBe('mac (aaaa)');
    expect(names.get('bbbb2222')).toBe('mac (bbbb)');
    expect(names.get('cccc3333')).toBe('pc');
  });

  it('lengthens the suffix when ids share a prefix', () => {
    const names = displayNames([node('abcd1111', 'mac'), node('abcd2222', 'mac')]);
    expect(names.get('abcd1111')).toBe('mac (abcd1)');
    expect(names.get('abcd2222')).toBe('mac (abcd2)');
  });
});
