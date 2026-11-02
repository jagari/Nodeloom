import { machineOrder, type CurrentWork } from './jobs';
import { unitWord } from './llm';
import type { MoveTracker } from './moves';
import type { NodeId, PairedDevice } from './types';

/** The cluster map's drawing area (SVG viewBox units, about CSS px). */
export const MAP_WIDTH = 540;
export const MAP_HEIGHT = 232;
export const NODE_R = 16;
/** Room for a machine's two label lines; longer names are clipped to fit. */
export const LABEL_WIDTH = 128;
export const MASTER_WIDTH = 136;
export const MASTER_HEIGHT = 46;

/** Label lines sit outside the ring: above the top row, below the bottom row. */
const LABEL_ROOM = 34;
const TOP_Y = NODE_R + LABEL_ROOM + 4;
const BOTTOM_Y = MAP_HEIGHT - TOP_Y;
/** How far the outer machines of a row bend toward the middle: a ring, not two lines. */
const ARC = 28;

export interface Point {
  x: number;
  y: number;
}

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** One machine's place on the map. */
export interface Spot extends Point {
  id: NodeId;
  /** Top row: name and work above the circle, so its link to the master stays clear. */
  labelAbove: boolean;
}

export interface MapLayout {
  master: Point;
  spots: Spot[];
}

/**
 * Places the master in the middle and every paired machine around it,
 * clockwise from the top left: half on an arc above, the rest on an arc
 * below. Rows rather than a circle because each machine carries two lines of
 * text; side by side, 4 per row still leave the labels apart.
 *
 * Places follow `machineOrder` and count machines that are offline, so one
 * that leaves keeps its place (drawn as offline) and the others stay put.
 */
export function mapLayout(devices: PairedDevice[], online: NodeId[]): MapLayout {
  const order = machineOrder(devices, online);
  const master = { x: MAP_WIDTH / 2, y: MAP_HEIGHT / 2 };
  const top = Math.ceil(order.length / 2);
  const bottom = order.length - top;

  const spots = order.map((id, i) => {
    const onTop = i < top;
    const count = onTop ? top : bottom;
    // The bottom row runs right to left, so the order reads clockwise.
    const slot = onTop ? i : count - 1 - (i - top);
    const x = (MAP_WIDTH * (slot + 0.5)) / count;
    const bend = ARC * ((x - master.x) / (MAP_WIDTH / 2)) ** 2;
    return { id, x, y: onTop ? TOP_Y + bend : BOTTOM_Y - bend, labelAbove: onTop };
  });
  return { master, spots };
}

/** The area a machine's circle, ring and two label lines take. */
export function spotBox(s: Spot): Box {
  return {
    left: s.x - LABEL_WIDTH / 2,
    top: s.y - NODE_R - (s.labelAbove ? LABEL_ROOM : 4),
    right: s.x + LABEL_WIDTH / 2,
    bottom: s.y + NODE_R + (s.labelAbove ? 4 : LABEL_ROOM),
  };
}

export function masterBox(p: Point): Box {
  return {
    left: p.x - MASTER_WIDTH / 2,
    top: p.y - MASTER_HEIGHT / 2,
    right: p.x + MASTER_WIDTH / 2,
    bottom: p.y + MASTER_HEIGHT / 2,
  };
}

export function overlaps(a: Box, b: Box): boolean {
  return a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
}

/**
 * Machines that just took over a piece from one that left, while the
 * "Moved to another machine" notice is up. Their link is drawn heavier.
 */
export function receivingMachines(tracker: MoveTracker): Set<NodeId> {
  return new Set(tracker.moves.flatMap((m) => (m.to ? [m.to] : [])));
}

/** "frames 201–400 · 45%", "+1" when it runs more pieces, or "Idle". */
export function workLabel(work: CurrentWork[]): string {
  const w = work[0];
  if (!w) return 'Idle';
  const more = work.length > 1 ? ` +${work.length - 1}` : '';
  return `${unitWord(w.runner, 2)} ${w.frame_start}–${w.frame_end} · ${Math.round(w.fraction * 100)}%${more}`;
}

/**
 * Shortens a display name to `max` characters, since SVG text does not
 * ellipsize by itself. The "(1ba6)" that `displayNames` adds to tell same-named
 * machines apart is kept; the hostname before it is cut instead.
 */
export function clipName(name: string, max: number): string {
  const chars = [...name];
  if (chars.length <= max) return name;
  const suffix = /\s\([0-9a-z-]+\)$/i.exec(name)?.[0] ?? '';
  const keep = max - 1 - [...suffix].length;
  if (keep < 1) return `${chars.slice(0, max - 1).join('')}…`;
  return `${chars.slice(0, keep).join('')}…${suffix}`;
}
