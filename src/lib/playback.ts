import type { TrimSpec } from '../api/types';

export type PlaybackAction = { type: 'none' } | { type: 'seek'; to: number } | { type: 'stop' };

/** 播放位置的容許誤差；跳轉後的位置可能比目標早幾毫秒。 */
const EPS = 0.05;

/** 「試聽成品」時，每次播放位置更新就問一次：要不要跳過、要不要停。 */
export function nextPlaybackAction(t: number, trim: TrimSpec): PlaybackAction {
  if (t >= trim.end) return { type: 'stop' };
  if (t < trim.start - EPS) return { type: 'seek', to: trim.start };
  for (const c of trim.cuts) {
    const s = Math.min(c.start, c.end);
    const e = Math.max(c.start, c.end);
    if (t >= s && t < e) return e >= trim.end ? { type: 'stop' } : { type: 'seek', to: e };
  }
  return { type: 'none' };
}

export function endPreviewStart(trim: TrimSpec, seconds = 3): number {
  return Math.max(trim.start, trim.end - seconds);
}
