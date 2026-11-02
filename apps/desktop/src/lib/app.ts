/**
 * The desktop app's own page: setup, this machine's status and settings.
 * These types mirror `apps/desktop/src-tauri/src/lib.rs` (`AppView`) and
 * `nodeloom_agent::AgentStatus`; keep them in step by hand, like types.ts.
 */

export type Role = 'master' | 'agent';

export type AgentStatus =
  | { state: 'searching' }
  | { state: 'connecting'; master: string; last_error: string | null }
  | { state: 'waiting_for_approval'; master: string; code: string }
  | { state: 'joined'; master: string };

export interface AppView {
  version: string;
  role: Role | null;
  code: string | null;
  agent: AgentStatus | null;
  master: { lan_url: string } | null;
  /** Set when this machine joins a typed address instead of searching. */
  master_address: string | null;
  autostart: boolean;
  error: string | null;
  log_file: string | null;
}

/**
 * True on the desktop app's own page. The master machine's window loads the
 * dashboard from the master over loopback; Tauri may inject its bridge
 * there too (with no permissions), so the origin decides, not the bridge
 * alone. The app's own files come from `tauri://localhost`, or
 * `http://tauri.localhost` on Windows, or the Vite server in development.
 */
export function inDesktopApp(w: object, place: { protocol: string; hostname: string }, dev: boolean): boolean {
  const ownFiles = place.protocol === 'tauri:' || place.hostname === 'tauri.localhost' || dev;
  return ownFiles && '__TAURI_INTERNALS__' in w;
}

/**
 * True on Windows, where the first listening port raises a firewall prompt
 * and a "Public" network blocks other machines even when allowed. Decided
 * from `navigator.userAgent`, which reads "Windows NT" in WebView2.
 */
export function onWindows(userAgent: string): boolean {
  return /Windows/i.test(userAgent);
}

/**
 * Whether to offer typing the master's address: while nothing is found or
 * joining fails, and always once one is typed (to change or clear it).
 */
export function offerAddress(status: AgentStatus, typed: string | null): boolean {
  if (typed) return true;
  return status.state === 'searching' || (status.state === 'connecting' && status.last_error !== null);
}

/** `ws://192.168.1.5:7878/ws/agent` → `192.168.1.5:7878`. */
export function masterHost(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

export type Tone = 'busy' | 'waiting' | 'good' | 'problem';

export interface AgentSummary {
  tone: Tone;
  title: string;
  detail: string;
  /** The master removed this machine; only a new identity gets back in. */
  removed: boolean;
}

/** What to tell the user about this machine's agent. */
export function describeAgent(status: AgentStatus): AgentSummary {
  switch (status.state) {
    case 'searching':
      return {
        tone: 'busy',
        title: 'Looking for a master…',
        detail: 'Start Nodeloom on the machine that runs the cluster, on this same network.',
        removed: false,
      };
    case 'connecting':
      if (!status.last_error) {
        return { tone: 'busy', title: `Connecting to ${masterHost(status.master)}…`, detail: '', removed: false };
      }
      // The master's wording (`REMOVED` in nodeloom-master's state.rs).
      if (status.last_error.includes('was removed from the cluster')) {
        return {
          tone: 'problem',
          title: `Removed from ${masterHost(status.master)}`,
          detail: 'The dashboard removed this machine. To add it again, pair it as a new machine.',
          removed: true,
        };
      }
      return {
        tone: 'problem',
        title: `Cannot join ${masterHost(status.master)}`,
        detail: `${status.last_error}. Retrying…`,
        removed: false,
      };
    case 'waiting_for_approval':
      return {
        tone: 'waiting',
        title: 'Waiting for approval',
        detail: `Approve this machine in the dashboard of ${masterHost(status.master)} if it shows the same code.`,
        removed: false,
      };
    case 'joined':
      return {
        tone: 'good',
        title: `In the cluster at ${masterHost(status.master)}`,
        detail: 'Ready for work.',
        removed: false,
      };
  }
}
