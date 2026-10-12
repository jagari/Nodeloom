import { describe, expect, it } from 'vitest';
import { parseEndpoint } from './client';

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
