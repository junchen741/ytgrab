import type { Range, TrimSpec } from '../api/types';

/** 頭尾灰色區段的最小寬度（秒）。 */
export const EDGE_MIN = 0.05;
/** 起點和終點之間至少要留的秒數。 */
export const MIN_GAP = 1;

export function trimFromRegions(headEnd: number, tailStart: number, duration: number, cuts: Range[]): TrimSpec {
  return {
    start: headEnd <= EDGE_MIN ? 0 : headEnd,
    end: tailStart >= duration - EDGE_MIN ? duration : tailStart,
    cuts: cuts
      .map((c) => ({ start: Math.min(c.start, c.end), end: Math.max(c.start, c.end) }))
      .sort((a, b) => a.start - b.start),
  };
}

/** 顯示用的成品長度（不含接縫交叉淡化那幾十毫秒）。 */
export function approxLength(trim: TrimSpec): number {
  let removed = 0;
  let cursor = trim.start;
  for (const c of [...trim.cuts].sort((a, b) => a.start - b.start)) {
    const s = Math.max(c.start, cursor);
    const e = Math.min(c.end, trim.end);
    if (e > s) {
      removed += e - s;
      cursor = e;
    }
  }
  return Math.max(0, trim.end - trim.start - removed);
}
