import { describe, expect, it } from 'vitest';
import { endPreviewStart, nextPlaybackAction } from './playback';

const trim = { start: 5, end: 50, cuts: [{ start: 20, end: 30 }] };

describe('nextPlaybackAction', () => {
  it('起點之前 → 跳到起點', () => {
    expect(nextPlaybackAction(2, trim)).toEqual({ type: 'seek', to: 5 });
  });
  it('剛好在起點附近不動', () => {
    expect(nextPlaybackAction(4.98, trim)).toEqual({ type: 'none' });
  });
  it('進入刪除區段 → 跳到區段結尾', () => {
    expect(nextPlaybackAction(21, trim)).toEqual({ type: 'seek', to: 30 });
  });
  it('顛倒的刪除區段也能處理', () => {
    expect(nextPlaybackAction(21, { start: 0, end: 50, cuts: [{ start: 30, end: 20 }] })).toEqual({ type: 'seek', to: 30 });
  });
  it('刪除區段一路延伸到終點 → 停止', () => {
    expect(nextPlaybackAction(45, { start: 0, end: 50, cuts: [{ start: 40, end: 60 }] })).toEqual({ type: 'stop' });
  });
  it('到達終點 → 停止', () => {
    expect(nextPlaybackAction(50, trim)).toEqual({ type: 'stop' });
  });
  it('一般位置不動', () => {
    expect(nextPlaybackAction(10, trim)).toEqual({ type: 'none' });
  });
});

describe('endPreviewStart', () => {
  it('從終點前 3 秒開始，但不早於起點', () => {
    expect(endPreviewStart({ start: 0, end: 100, cuts: [] })).toBe(97);
    expect(endPreviewStart({ start: 10, end: 11, cuts: [] })).toBe(10);
  });
});
