import { describe, expect, it } from 'vitest';
import { translator } from './i18n';
import type { CurrentWork } from './jobs';
import { NO_MOVES, type Move } from './moves';
import {
  MAP_HEIGHT,
  MAP_WIDTH,
  clipName,
  mapLayout,
  masterBox,
  overlaps,
  receivingMachines,
  spotBox,
  workLabel,
} from './topology';
import type { PairedDevice } from './types';

const en = translator('en');

const devices = (n: number): PairedDevice[] =>
  Array.from({ length: n }, (_, i) => ({ id: `id-${i}`, name: `machine-${i}` }));

describe('mapLayout', () => {
  it('keeps 1 to 8 machines apart, off the master and inside the map', () => {
    for (let n = 1; n <= 8; n++) {
      const { master, spots } = mapLayout(devices(n), []);
      expect(spots).toHaveLength(n);
      const boxes = spots.map(spotBox);
      for (const [i, b] of boxes.entries()) {
        expect(b.left, `${n} machines, #${i}`).toBeGreaterThanOrEqual(0);
        expect(b.top, `${n} machines, #${i}`).toBeGreaterThanOrEqual(0);
        expect(b.right, `${n} machines, #${i}`).toBeLessThanOrEqual(MAP_WIDTH);
        expect(b.bottom, `${n} machines, #${i}`).toBeLessThanOrEqual(MAP_HEIGHT);
        expect(overlaps(b, masterBox(master)), `${n} machines, #${i} on the master`).toBe(false);
        for (const [j, other] of boxes.entries()) {
          if (j > i) expect(overlaps(b, other), `${n} machines, #${i} and #${j}`).toBe(false);
        }
      }
    }
  });

  it('follows the paired list, then unknown machines by id', () => {
    const { spots } = mapLayout(devices(2), ['zz', 'id-1', 'aa']);
    expect(spots.map((s) => s.id)).toEqual(['id-0', 'id-1', 'aa', 'zz']);
  });

  it('keeps every place when a machine goes offline', () => {
    const all = mapLayout(devices(3), ['id-0', 'id-1', 'id-2']);
    const oneLeft = mapLayout(devices(3), ['id-0', 'id-2']);
    expect(oneLeft).toEqual(all);
  });

  it('reads clockwise: top row left to right, then bottom row right to left', () => {
    const { master, spots } = mapLayout(devices(4), []);
    const [a, b, c, d] = spots;
    expect(a!.y).toBeLessThan(master.y);
    expect(b!.y).toBeLessThan(master.y);
    expect(a!.x).toBeLessThan(b!.x);
    expect(c!.y).toBeGreaterThan(master.y);
    expect(d!.y).toBeGreaterThan(master.y);
    expect(c!.x).toBeGreaterThan(d!.x);
  });

  it('has only the master when nothing is paired', () => {
    expect(mapLayout([], []).spots).toEqual([]);
  });
});

describe('receivingMachines', () => {
  const move = (to: string | null): Move => ({
    job_id: 1,
    task_id: 1,
    frame_start: 1,
    frame_end: 2,
    from: 'gone',
    to,
    at: 0,
  });

  it('lists machines that took a piece over, not pieces still waiting', () => {
    expect(receivingMachines({ ...NO_MOVES, moves: [move('mac'), move(null)] })).toEqual(new Set(['mac']));
    expect(receivingMachines(NO_MOVES).size).toBe(0);
  });
});

describe('workLabel', () => {
  const work = (runner: 'fake' | 'llm', fraction: number): CurrentWork => ({
    job_id: 2,
    runner,
    task_id: 1,
    frame_start: 201,
    frame_end: 400,
    frames_done: 0,
    fraction,
  });

  it('names the piece and its progress', () => {
    expect(workLabel([work('fake', 0.454)], en)).toBe('frames 201–400 · 45%');
    expect(workLabel([work('llm', 1)], en)).toBe('questions 201–400 · 100%');
  });

  it('counts extra pieces and says Idle without one', () => {
    expect(workLabel([work('fake', 0), work('fake', 0)], en)).toBe('frames 201–400 · 0% +1');
    expect(workLabel([], en)).toBe('Idle');
  });
});

describe('clipName', () => {
  it('shortens long names only', () => {
    expect(clipName('pi', 5)).toBe('pi');
    expect(clipName('raspberrypi', 5)).toBe('rasp…');
    expect(clipName('맥북프로입니다', 4)).toBe('맥북프…');
  });

  it('keeps the suffix that tells same-named machines apart', () => {
    expect(clipName('DESKTOP-RFLA9Q6 (1ba6)', 17)).toBe('DESKTOP-R… (1ba6)');
    expect(clipName('DESKTOP-RFLA9Q6 (1ba6)', 17)).toHaveLength(17);
  });
});
