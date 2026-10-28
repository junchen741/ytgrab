import { describe, expect, it } from 'vitest';
import { approxLength, draggedPastEdge, EDGE_MIN, tailMinSeconds, trimFromRegions } from './trim';

describe('trimFromRegions', () => {
  it('極窄的頭尾區段代表「不裁」', () => {
    expect(trimFromRegions(0.05, 99.97, 100, [])).toEqual({ start: 0, end: 100, cuts: [] });
  });
  it('終點「不裁」的範圍可以放寬（長影片的終點區段至少要有幾個像素寬）', () => {
    expect(trimFromRegions(0.05, 99.7, 100, [], 0.5)).toEqual({ start: 0, end: 100, cuts: [] });
    expect(trimFromRegions(0.05, 99.4, 100, [], 0.5).end).toBe(99.4);
  });
  it('刪除區段會排序並修正顛倒的起訖', () => {
    const got = trimFromRegions(12.5, 80, 100, [
      { start: 50, end: 40 },
      { start: 20, end: 30 },
    ]);
    expect(got).toEqual({ start: 12.5, end: 80, cuts: [{ start: 20, end: 30 }, { start: 40, end: 50 }] });
  });
});

describe('tailMinSeconds', () => {
  // wavesurfer 把區段起點換算成像素後四捨五入，等於波形寬度就當成畫面外不畫
  it('長影片：終點區段至少要有 2 個像素寬', () => {
    expect(tailMinSeconds(284.1, 830)).toBeCloseTo(0.6846, 3);
  });
  it('短片：至少 EDGE_MIN', () => {
    expect(tailMinSeconds(19, 830)).toBe(EDGE_MIN);
  });
  it('寬度還沒算出來（0）時退回 EDGE_MIN', () => {
    expect(tailMinSeconds(284.1, 0)).toBe(EDGE_MIN);
  });
});

describe('draggedPastEdge', () => {
  // wavesurfer 遇到會超出範圍的拖拉更新會整個拒絕，停在離邊緣差一點的地方；改用指標實際移動的距離判斷
  it('起點拉桿往左拖超過開頭', () => {
    expect(draggedPastEdge('head', 10, -10.5, 284)).toBe(true);
    expect(draggedPastEdge('head', 10, -9, 284)).toBe(false);
  });
  it('終點拉桿往右拖超過結尾（考慮終點區段的最小寬度）', () => {
    expect(draggedPastEdge('tail', 270, 15, 284.085, 0.68)).toBe(true);
    expect(draggedPastEdge('tail', 270, 13.5, 284.085, 0.68)).toBe(true);
    expect(draggedPastEdge('tail', 270, 5, 284.085, 0.68)).toBe(false);
  });
});

describe('approxLength', () => {
  it('扣掉刪除區段，重疊與超出範圍的部分不重複扣', () => {
    const trim = { start: 10, end: 100, cuts: [{ start: 20, end: 30 }, { start: 25, end: 35 }, { start: 90, end: 120 }] };
    expect(approxLength(trim)).toBe(65);
  });
  it('起點之前的刪除區段只扣範圍內的部分', () => {
    expect(approxLength({ start: 10, end: 20, cuts: [{ start: 0, end: 15 }] })).toBe(5);
  });
});
