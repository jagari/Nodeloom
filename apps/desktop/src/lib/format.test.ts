import { describe, expect, it } from 'vitest';
import { clampPercent, formatAgo, formatBytes, formatPercent } from './format';

describe('formatBytes', () => {
  it('scales units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(16 * 1024 ** 3)).toBe('16 GB');
    expect(formatBytes(2 * 1024 ** 4)).toBe('2.0 TB');
  });
  it('handles bad input', () => {
    expect(formatBytes(-1)).toBe('–');
    expect(formatBytes(Number.NaN)).toBe('–');
  });
});

describe('percent', () => {
  it('clamps and rounds', () => {
    expect(clampPercent(150)).toBe(100);
    expect(clampPercent(-3)).toBe(0);
    expect(clampPercent(Number.NaN)).toBe(0);
    expect(formatPercent(42.6)).toBe('43%');
  });
});

describe('formatAgo', () => {
  it('picks a sensible unit', () => {
    expect(formatAgo(0.4)).toBe('now');
    expect(formatAgo(5)).toBe('5s ago');
    expect(formatAgo(120)).toBe('2m ago');
  });
});
