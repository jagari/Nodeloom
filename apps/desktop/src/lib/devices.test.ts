import { describe, expect, it } from 'vitest';
import { lookalikes, pairedRows } from './devices';
import type { NodeView, PendingDevice } from './types';

const info = (id: string, hostname: string) => ({
  id,
  hostname,
  os: 'test',
  arch: 'test',
  cpu_cores: 1,
  total_memory_bytes: 1,
  runners: [],
});
const node = (id: string): NodeView => ({ info: info(id, id), metrics: null, busy_tasks: 0 });
const request = (key: string, id: string, hostname: string, code: string): PendingDevice => ({
  public_key: key,
  code,
  info: info(id, hostname),
  address: '192.168.1.9',
});

describe('pairedRows', () => {
  it('marks connected machines online and lists them first', () => {
    const rows = pairedRows(
      [
        { id: 'b1', name: 'beta' },
        { id: 'a1', name: 'alpha' },
        { id: 'c1', name: 'gamma' },
      ],
      [node('c1')],
    );
    expect(rows).toEqual([
      { id: 'c1', name: 'gamma', online: true },
      { id: 'a1', name: 'alpha', online: false },
      { id: 'b1', name: 'beta', online: false },
    ]);
  });

  it('tells machines with the same name apart', () => {
    const rows = pairedRows(
      [
        { id: 'aaaa1', name: 'mac' },
        { id: 'bbbb2', name: 'mac' },
      ],
      [],
    );
    expect(rows.map((r) => r.name)).toEqual(['mac (aaaa)', 'mac (bbbb)']);
  });
});

describe('lookalikes', () => {
  it('flags nothing when every request is distinct', () => {
    const pending = [request('k1', 'a', 'studio', '111 111'), request('k2', 'b', 'laptop', '222 222')];
    expect(lookalikes(pending, []).size).toBe(0);
  });

  it('flags requests sharing a code, id or name', () => {
    const pending = [
      request('k1', 'a', 'mac', '111 111'),
      request('k2', 'b', 'mac', '111 111'),
      request('k3', 'c', 'pi', '333 333'),
      request('k4', 'c', 'pi-2', '444 444'),
      request('k5', 'd', 'pc', '555 555'),
      request('k6', 'e', 'pc', '666 666'),
    ];
    const flags = lookalikes(pending, []);
    expect(flags.get('k1')).toMatch(/same code/);
    expect(flags.get('k2')).toMatch(/same code/);
    expect(flags.get('k3')).toMatch(/same id/);
    expect(flags.get('k4')).toMatch(/same id/);
    expect(flags.get('k5')).toMatch(/same name/);
    expect(flags.get('k6')).toMatch(/same name/);
  });

  it('compares names the way they look, not byte by byte', () => {
    const pending = [request('k1', 'a', 'Jun-MacBook ', '111 111'), request('k2', 'b', 'jun-macbook', '222 222')];
    const flags = lookalikes(pending, [{ id: 'c', name: 'ＳＴＵＤＩＯ' }]);
    expect(flags.get('k1')).toMatch(/same name/);
    expect(flags.get('k2')).toMatch(/same name/);
    expect(lookalikes([request('k3', 'd', 'studio', '333 333')], [{ id: 'c', name: 'ＳＴＵＤＩＯ' }]).get('k3')).toMatch(
      /paired machine/,
    );
  });

  it('folds dash lookalikes and compares the name the card shows', () => {
    const paired = [{ id: 'old', name: 'Jun-MacBook' }];
    expect(lookalikes([request('k1', 'x', 'Jun\u2010MacBook', '111 111')], paired).get('k1')).toMatch(/paired/);
    // No hostname: the card shows the id, so compare that.
    expect(lookalikes([request('k2', 'Jun-MacBook', '', '222 222')], paired).get('k2')).toMatch(/paired/);
  });

  it('flags a request named like a paired machine', () => {
    const flags = lookalikes([request('k1', 'new', 'studio', '111 111')], [{ id: 'old', name: 'studio' }]);
    expect(flags.get('k1')).toMatch(/paired machine/);
  });
});
