import type { NodeId, NodeView } from './types';

/**
 * Display name per node. Hostnames are used as-is unless two machines share
 * one (common with default names like "MacBook-Pro"); those get a short id
 * suffix, just long enough to tell them apart.
 */
export function displayNames(nodes: NodeView[]): Map<NodeId, string> {
  const byHost = new Map<string, NodeId[]>();
  for (const { info } of nodes) {
    byHost.set(info.hostname, [...(byHost.get(info.hostname) ?? []), info.id]);
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
