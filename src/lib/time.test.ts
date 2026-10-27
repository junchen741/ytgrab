import { describe, expect, it } from 'vitest';
import { formatClock, formatTime, parseTime } from './time';

describe('formatTime', () => {
  it('分:秒.十分之一秒', () => {
    expect(formatTime(0)).toBe('0:00.0');
    expect(formatTime(65.25)).toBe('1:05.3');
    expect(formatTime(600)).toBe('10:00.0');
    expect(formatTime(-3)).toBe('0:00.0');
  });
});

describe('parseTime', () => {
  it('接受「分:秒」或純秒數', () => {
    expect(parseTime('1:05.3')).toBeCloseTo(65.3);
    expect(parseTime(' 75 ')).toBe(75);
    expect(parseTime('0:7.5')).toBeCloseTo(7.5);
  });
  it('格式錯誤回傳 null', () => {
    expect(parseTime('1:75')).toBeNull();
    expect(parseTime('abc')).toBeNull();
    expect(parseTime('')).toBeNull();
  });
});

describe('formatClock', () => {
  it('影片長度顯示', () => {
    expect(formatClock(225)).toBe('3:45');
    expect(formatClock(3723)).toBe('1:02:03');
    expect(formatClock(59.6)).toBe('1:00');
  });
});
