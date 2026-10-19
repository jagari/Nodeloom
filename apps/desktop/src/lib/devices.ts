import { displayNames } from './names';
import type { NodeId, NodeView, PairedDevice, PendingDevice } from './types';

export interface PairedRow {
  id: NodeId;
  name: string;
  online: boolean;
}

/** Paired machines with their connection state, online ones first. */
export function pairedRows(devices: PairedDevice[], nodes: NodeView[]): PairedRow[] {
  const names = displayNames(devices.map((d) => ({ id: d.id, hostname: d.name })));
  const online = new Set(nodes.map((n) => n.info.id));
  return devices
    .map((d) => ({ id: d.id, name: names.get(d.id) ?? d.name, online: online.has(d.id) }))
    .sort((a, b) => Number(b.online) - Number(a.online) || a.name.localeCompare(b.name));
}

/**
 * A name as people read it: compatibility forms folded (full-width letters,
 * ligatures), spacing collapsed, case ignored. Lookalike letters from other
 * scripts are not caught.
 */
export function sameName(name: string): string {
  return name
    .normalize('NFKC')
    .replace(/[\u2010-\u2015\u2212\uFE58\uFE63\uFF0D]/g, '-')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase();
}

/** What a request's card shows as its name. */
export function shownName(request: PendingDevice): string {
  return request.info.hostname || request.info.id;
}

/**
 * Why a waiting request deserves a second look, keyed by public key. Two
 * requests that look alike may mean someone is imitating a real machine, so
 * the person approving should check the code on the machine itself.
 */
export function lookalikes(pending: PendingDevice[], devices: PairedDevice[]): Map<string, string> {
  const count = <T>(values: T[]) => {
    const counts = new Map<T, number>();
    for (const v of values) counts.set(v, (counts.get(v) ?? 0) + 1);
    return counts;
  };
  const codes = count(pending.map((p) => p.code));
  const hosts = count(pending.map((p) => sameName(shownName(p))));
  const ids = count(pending.map((p) => p.info.id));
  const pairedNames = new Set(devices.map((d) => sameName(d.name)));

  const reasons = new Map<string, string>();
  for (const p of pending) {
    const reason =
      (codes.get(p.code) ?? 0) > 1
        ? 'Another waiting machine shows the same code.'
        : (ids.get(p.info.id) ?? 0) > 1
          ? 'Another waiting machine uses the same id.'
          : (hosts.get(sameName(shownName(p))) ?? 0) > 1
            ? 'Another waiting machine has the same name.'
            : pairedNames.has(sameName(shownName(p)))
              ? 'A paired machine already has this name.'
              : null;
    if (reason) reasons.set(p.public_key, reason);
  }
  return reasons;
}
