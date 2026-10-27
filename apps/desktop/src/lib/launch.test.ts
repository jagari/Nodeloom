import { describe, expect, it } from 'vitest';
import { initialConnection } from './launch';

const master = { protocol: 'http:', host: '192.168.1.20:7878', hostname: '192.168.1.20', hash: '' };
const none = { host: '', token: '' };

describe('initialConnection', () => {
  it('defaults to the master that served the page', () => {
    expect(initialConnection(master, none, false)).toEqual({ host: '192.168.1.20:7878', token: '', clearHash: false });
  });

  it('takes the token from the first-start link and asks to drop it', () => {
    const place = { ...master, hash: '#token=abc123' };
    expect(initialConnection(place, none, false)).toEqual({ host: '192.168.1.20:7878', token: 'abc123', clearHash: true });
  });

  it('keeps a remembered connection over the page address', () => {
    const stored = { host: 'studio.local:7878', token: 't' };
    expect(initialConnection(master, stored, false)).toEqual({ ...stored, clearHash: false });
  });

  it('does not treat the dev server or the desktop shell as a master', () => {
    const dev = { protocol: 'http:', host: 'localhost:5173', hostname: 'localhost', hash: '' };
    expect(initialConnection(dev, none, true).host).toBe('');
    const tauri = { protocol: 'http:', host: 'tauri.localhost', hostname: 'tauri.localhost', hash: '' };
    expect(initialConnection(tauri, none, false).host).toBe('');
    const mac = { protocol: 'tauri:', host: 'localhost', hostname: 'localhost', hash: '' };
    expect(initialConnection(mac, none, false).host).toBe('');
  });
});
