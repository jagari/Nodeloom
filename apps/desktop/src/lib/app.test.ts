import { describe, expect, it } from 'vitest';
import { describeAgent, inDesktopApp, masterHost } from './app';

describe('inDesktopApp', () => {
  const bridge = { __TAURI_INTERNALS__: {} };
  const own = { protocol: 'tauri:', hostname: 'localhost' };
  const windows = { protocol: 'http:', hostname: 'tauri.localhost' };
  const dashboard = { protocol: 'http:', hostname: '127.0.0.1' };

  it('is true on the app\'s own files when Tauri is present', () => {
    expect(inDesktopApp(bridge, own, false)).toBe(true);
    expect(inDesktopApp(bridge, windows, false)).toBe(true);
    expect(inDesktopApp(bridge, { protocol: 'http:', hostname: 'localhost' }, true)).toBe(true);
  });
  it('is false in a browser and on the dashboard a master serves', () => {
    expect(inDesktopApp({}, own, false)).toBe(false);
    expect(inDesktopApp({}, dashboard, true)).toBe(false);
    expect(inDesktopApp(bridge, dashboard, false)).toBe(false);
  });
});

describe('masterHost', () => {
  it('shows host and port of the agent endpoint', () => {
    expect(masterHost('ws://192.168.1.5:7878/ws/agent')).toBe('192.168.1.5:7878');
    expect(masterHost('ws://127.0.0.1:7878/ws/agent')).toBe('127.0.0.1:7878');
  });
  it('falls back to the text it was given', () => {
    expect(masterHost('not a url')).toBe('not a url');
  });
});

describe('describeAgent', () => {
  const master = 'ws://10.0.0.2:7878/ws/agent';

  it('names each step of joining', () => {
    expect(describeAgent({ state: 'searching' }).tone).toBe('busy');
    expect(describeAgent({ state: 'connecting', master, last_error: null })).toMatchObject({
      tone: 'busy',
      title: 'Connecting to 10.0.0.2:7878…',
    });
    expect(describeAgent({ state: 'waiting_for_approval', master, code: '123 456' })).toMatchObject({
      tone: 'waiting',
      title: 'Waiting for approval',
    });
    expect(describeAgent({ state: 'joined', master })).toMatchObject({
      tone: 'good',
      title: 'In the cluster at 10.0.0.2:7878',
    });
  });

  it('keeps the reason a connection failed on screen', () => {
    const summary = describeAgent({ state: 'connecting', master, last_error: 'connection refused' });
    expect(summary).toMatchObject({ tone: 'problem', removed: false });
    expect(summary.detail).toContain('connection refused');
  });

  it('offers a way back to a removed machine', () => {
    const summary = describeAgent({
      state: 'connecting',
      master,
      last_error:
        'master rejected us: this machine was removed from the cluster; run `nodeloom-agent reset-identity` on it and approve it again',
    });
    expect(summary).toMatchObject({ tone: 'problem', removed: true, title: 'Removed from 10.0.0.2:7878' });
    expect(summary.detail).not.toContain('nodeloom-agent');
  });
});
