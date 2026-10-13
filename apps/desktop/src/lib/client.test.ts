import { afterEach, describe, expect, it, vi } from 'vitest';
import { ClusterClient, parseEndpoint } from './client';

describe('parseEndpoint', () => {
  it('adds http:// when the scheme is missing', () => {
    expect(parseEndpoint('192.168.1.5:7878')).toEqual({
      http: 'http://192.168.1.5:7878',
      ws: 'ws://192.168.1.5:7878',
    });
  });

  it('keeps https and maps it to wss, dropping any path', () => {
    expect(parseEndpoint(' https://render.example.com/some/path ')).toEqual({
      http: 'https://render.example.com',
      ws: 'wss://render.example.com',
    });
  });

  it('rejects junk and other schemes', () => {
    expect(parseEndpoint('')).toBeNull();
    expect(parseEndpoint('   ')).toBeNull();
    expect(parseEndpoint('ftp://host')).toBeNull();
    expect(parseEndpoint('http://')).toBeNull();
  });
});

describe('ClusterClient requests', () => {
  const endpoint = { http: 'http://master:7878', ws: 'ws://master:7878' };
  const handlers = { onSnapshot() {}, onStatus() {} };

  function withFetch(status: number, body = '') {
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal('fetch', async (url: string, init: RequestInit) => {
      calls.push({ url, init });
      return new Response(status === 204 ? null : body, { status });
    });
    return calls;
  }

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('approves, ignores and removes with the admin token', async () => {
    const calls = withFetch(204);
    const client = new ClusterClient(endpoint, 'secret', handlers);
    await client.approve('ab12');
    await client.ignore('cd34');
    await client.removeDevice('node/1');
    expect(calls.map((c) => [c.init.method, c.url])).toEqual([
      ['POST', 'http://master:7878/api/pending/ab12/approve'],
      ['POST', 'http://master:7878/api/pending/cd34/ignore'],
      ['DELETE', 'http://master:7878/api/devices/node%2F1'],
    ]);
    for (const { init } of calls) {
      expect((init.headers as Record<string, string>).Authorization).toBe('Bearer secret');
      expect(init.body).toBeUndefined();
    }
  });

  it('explains a request that vanished, and passes on conflicts', async () => {
    const client = new ClusterClient(endpoint, 'secret', handlers);
    withFetch(404);
    await expect(client.approve('ab12')).rejects.toThrow(/no longer waiting/);
    await expect(client.removeDevice('x')).rejects.toThrow(/not paired/);
    withFetch(409, 'x is already paired with another key; remove it first');
    await expect(client.approve('ab12')).rejects.toThrow(/already paired/);
    withFetch(401);
    await expect(client.ignore('ab12')).rejects.toThrow(/admin token/);
  });

  it('sends jobs as JSON', async () => {
    const calls = withFetch(200, '{"job_id":1,"tasks":[]}');
    const client = new ClusterClient(endpoint, 'secret', handlers);
    const job = await client.submit({ runner: 'fake', frame_start: 1, frame_end: 2, millis_per_frame: 0 });
    expect(job.job_id).toBe(1);
    expect((calls[0]!.init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    expect(JSON.parse(String(calls[0]!.init.body))).toEqual({
      payload: { runner: 'fake', frame_start: 1, frame_end: 2, millis_per_frame: 0 },
    });
  });
});
