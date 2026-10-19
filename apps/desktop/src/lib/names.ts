import type { NodeId } from './types';

export interface Named {
  id: NodeId;
  hostname: string;
}

/**
 * Display name per machine. Hostnames are used as-is unless two machines
 * share one (common with default names like "MacBook-Pro"); those get a short
 * id suffix, just long enough to tell them apart.
 */
export function displayNames(machines: Named[]): Map<NodeId, string> {
  const byHost = new Map<string, NodeId[]>();
  for (const { id, hostname } of machines) {
    byHost.set(hostname, [...(byHost.get(hostname) ?? []), id]);
  }

  const names = new Map<NodeId, string>();
  for (const [host, ids] of byHost) {
    if (ids.length === 1) {
      names.set(ids[0]!, host);
      continue;
    }
    let len = 4;
    while (len < 64 && new Set(ids.map((id) => id.slice(0, len))).size < ids.length) len++;
    for (const id of ids) names.set(id, `${host} (${id.slice(0, len)})`);
  }
  return names;
}
