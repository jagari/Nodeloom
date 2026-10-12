import type { ClusterSnapshot, JobView, TaskPayload } from './types';

export type ConnectionStatus = 'connecting' | 'live' | 'offline' | 'unauthorized';

export interface Endpoint {
  http: string;
  ws: string;
}

/**
 * Turns what the user typed ("192.168.1.5:7878", "http://mac.local:7878/")
 * into HTTP and WebSocket base URLs. Returns null for unusable input.
 */
export function parseEndpoint(input: string): Endpoint | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const withScheme = /^[a-z]+:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`;
  let url: URL;
  try {
    url = new URL(withScheme);
  } catch {
    return null;
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
  if (!url.hostname) return null;
  const secure = url.protocol === 'https:';
  return {
    http: `${url.protocol}//${url.host}`,
    ws: `${secure ? 'wss' : 'ws'}://${url.host}`,
  };
}

export interface ClientHandlers {
  onSnapshot(snapshot: ClusterSnapshot): void;
  onStatus(status: ConnectionStatus): void;
}

/** Keeps a live view of the master, reconnecting with backoff. */
export class ClusterClient {
  private socket: WebSocket | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private delay = 1000;
  private stopped = false;

  constructor(
    private readonly endpoint: Endpoint,
    private readonly token: string,
    private readonly handlers: ClientHandlers,
  ) {}

  start(): void {
    this.stopped = false;
    void this.connect();
  }

  stop(): void {
    this.stopped = true;
    clearTimeout(this.timer);
    this.socket?.close();
    this.socket = null;
  }

  async submit(payload: TaskPayload): Promise<JobView> {
    const response = await fetch(`${this.endpoint.http}/api/jobs`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${this.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ payload }),
    });
    if (!response.ok) {
      const detail = (await response.text()).trim();
      throw new Error(detail || `master answered ${response.status}`);
    }
    return (await response.json()) as JobView;
  }

  private async connect(): Promise<void> {
    this.handlers.onStatus('connecting');
    // Check the token over HTTP first: a rejected WebSocket handshake gives
    // the browser no way to tell "wrong token" from "master unreachable".
    try {
      const probe = await fetch(`${this.endpoint.http}/api/state`, {
        headers: { Authorization: `Bearer ${this.token}` },
      });
      if (this.stopped) return;
      if (probe.status === 401) {
        this.handlers.onStatus('unauthorized');
        return;
      }
      if (!probe.ok) throw new Error(`status ${probe.status}`);
    } catch {
      if (!this.stopped) this.scheduleRetry();
      return;
    }

    const socket = new WebSocket(`${this.endpoint.ws}/ws/ui?token=${encodeURIComponent(this.token)}`);
    this.socket = socket;
    socket.onmessage = (event) => {
      try {
        this.handlers.onSnapshot(JSON.parse(String(event.data)) as ClusterSnapshot);
        this.delay = 1000;
        this.handlers.onStatus('live');
      } catch {
        // Ignore a malformed frame; the next snapshot replaces it anyway.
      }
    };
    socket.onclose = () => {
      if (this.socket === socket) this.socket = null;
      if (!this.stopped) this.scheduleRetry();
    };
  }

  private scheduleRetry(): void {
    this.handlers.onStatus('offline');
    this.timer = setTimeout(() => void this.connect(), this.delay);
    this.delay = Math.min(this.delay * 2, 10_000);
  }
}
