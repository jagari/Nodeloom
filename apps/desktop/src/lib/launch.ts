/** The parts of `window.location` the start-up logic reads. */
export interface Place {
  protocol: string;
  host: string;
  hostname: string;
  hash: string;
}

export interface Launch {
  host: string;
  token: string;
  /** The token came in the URL; remove it from the address bar. */
  clearHash: boolean;
}

/**
 * Where the dashboard connects first. A master serves this page itself, so
 * when it is opened from one, that master is the default. On first start the
 * master prints a link with `#token=…`: the fragment never reaches the
 * server, and the page takes the token from it and then drops it.
 * The Vite dev server and the Tauri shell (`tauri.localhost` on Windows) are
 * not masters.
 */
export function initialConnection(place: Place, stored: { host: string; token: string }, dev: boolean): Launch {
  const servedByMaster =
    !dev && (place.protocol === 'http:' || place.protocol === 'https:') && place.hostname !== 'tauri.localhost';
  const fromHash = new URLSearchParams(place.hash.replace(/^#/, '')).get('token')?.trim() ?? '';
  if (fromHash) {
    return { host: servedByMaster ? place.host : stored.host, token: fromHash, clearHash: true };
  }
  return { host: stored.host || (servedByMaster ? place.host : ''), token: stored.token, clearHash: false };
}
