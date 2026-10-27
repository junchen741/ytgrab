import { describe, expect, it } from 'vitest';
import { approxLength, trimFromRegions } from './trim';

describe('trimFromRegions', () => {
  it('極窄的頭尾區段代表「不裁」', () => {
    expect(trimFromRegions(0.05, 99.97, 100, [])).toEqual({ start: 0, end: 100, cuts: [] });
  });
  it('刪除區段會排序並修正顛倒的起訖', () => {
    const got = trimFromRegions(12.5, 80, 100, [
      { start: 50, end: 40 },
      { start: 20, end: 30 },
    ]);
    expect(got).toEqual({ start: 12.5, end: 80, cuts: [{ start: 20, end: 30 }, { start: 40, end: 50 }] });
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
