import { describe, expect, it } from 'vitest';
import tauriConf from '../../src-tauri/tauri.conf.json';
import { DESIGN, fitWindow } from './windowFit';

it('設計尺寸和 tauri.conf.json 的視窗設定一致', () => {
  const win = tauriConf.app.windows[0];
  expect(DESIGN).toEqual({ width: win.width, height: win.height, minWidth: win.minWidth, minHeight: win.minHeight });
});

describe('fitWindow', () => {
  it('沒有放大文字就不調整', () => {
    expect(fitWindow(1, 1920, 1000)).toBeNull();
    expect(fitWindow(1.02, 1920, 1000)).toBeNull();
    expect(fitWindow(NaN, 1920, 1000)).toBeNull();
  });

  it('螢幕夠大：視窗跟著文字放大，網頁不縮放', () => {
    const fit = fitWindow(1.5, 1600, 1120)!;
    expect(fit.scale).toBeCloseTo(1.5);
    expect(fit.zoom).toBeCloseTo(1);
  });

  it('螢幕不夠大：視窗放到容得下為止，其餘用網頁縮放抵銷', () => {
    const fit = fitWindow(1.5, 1600, 925)!; // 高度只容得下 1.25 倍
    expect(fit.scale).toBeCloseTo(1.25);
    expect(fit.zoom).toBeCloseTo(1.25 / 1.5);
  });

  it('螢幕比設計尺寸還小：視窗維持原大小，內容縮回設計比例', () => {
    const fit = fitWindow(2, 800, 600)!;
    expect(fit.scale).toBe(1);
    expect(fit.zoom).toBeCloseTo(0.5);
  });
});
