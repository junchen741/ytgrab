import type { Range, TrimSpec } from '../api/types';

/** 頭尾灰色區段的最小寬度（秒）。 */
export const EDGE_MIN = 0.05;
/** 起點和終點之間至少要留的秒數。 */
export const MIN_GAP = 1;
/** 終點灰色區段至少要有的像素寬。 */
export const TAIL_MIN_PX = 2;

/**
 * 終點灰色區段的最小寬度（秒）。wavesurfer 只畫「看得到」的區段，而且會把區段起點換算成像素後四捨五入；
 * 長影片的終點區段如果只有 EDGE_MIN 寬，起點會被四捨五入成剛好等於波形寬度，被當成在畫面外而不畫。
 */
export function tailMinSeconds(totalDuration: number, widthPx: number): number {
  if (widthPx <= 0) return EDGE_MIN;
  return Math.max(EDGE_MIN, (TAIL_MIN_PX * totalDuration) / widthPx);
}

/**
 * 頭尾拉桿是不是被拖過了邊緣。wavesurfer 遇到會讓區段太窄或超出範圍的拖拉更新會整個拒絕，
 * 停在上一個位置（離邊緣差一點）；所以改用「按下時的位置 + 指標實際移動的秒數」算出本來要拖到哪裡。
 */
export function draggedPastEdge(
  side: 'head' | 'tail',
  posAtDown: number,
  movedSec: number,
  duration: number,
  tailMin = EDGE_MIN,
): boolean {
  const intended = posAtDown + movedSec;
  return side === 'head' ? intended <= EDGE_MIN : intended >= duration - tailMin;
}

/** `tailMin`：終點區段在這個秒數內都算「不裁」，見 tailMinSeconds。 */
export function trimFromRegions(
  headEnd: number,
  tailStart: number,
  duration: number,
  cuts: Range[],
  tailMin = EDGE_MIN,
): TrimSpec {
  return {
    start: headEnd <= EDGE_MIN ? 0 : headEnd,
    end: tailStart >= duration - tailMin ? duration : tailStart,
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
