import type { VideoInfo } from '../api/types';
import { litSegments } from '../lib/meter';
import { h } from './dom';
import { videoHeader } from './videoHeader';

/** 螢幕窗裡的段式進度條有幾格。 */
const SEGMENTS = 20;

/** 顯示進度的螢幕窗（首頁按下「開始」後，原本的「準備好了」會換成這個）。 */
export interface ProgressDisplay {
  el: HTMLElement;
  setText(t: string): void;
  /** null = 不確定進度（亮格來回掃動） */
  setPercent(p: number | null): void;
  setVideo(v: VideoInfo): void;
}

export function progressDisplay(initialText: string): ProgressDisplay {
  const header = h('div');
  const text = h('p', { class: 'status' }, initialText);
  // --i 給 CSS 的掃動動畫算每一格亮起的時間
  const segments = Array.from({ length: SEGMENTS }, (_, i) => h('i', { style: `--i:${i}` }));
  const value = h('span', { class: 'meter-value dig' });
  // 還沒收到百分比之前是不確定進度，和原本沒設 value 的 <progress> 一樣
  const meter = h(
    'div',
    { class: 'meter indeterminate', role: 'progressbar', 'aria-valuemin': 0, 'aria-valuemax': 100, 'aria-label': '進度' },
    h('span', { class: 'meter-cells' }, ...segments),
    value,
  );
  const el = h('div', { class: 'display' }, header, text, meter);
  return {
    el,
    setText(t) {
      text.textContent = t;
    },
    setPercent(p) {
      if (p === null) {
        meter.removeAttribute('aria-valuenow');
        meter.classList.add('indeterminate');
        segments.forEach((s) => s.classList.remove('on'));
        value.textContent = '';
        return;
      }
      const lit = litSegments(p, SEGMENTS);
      meter.classList.remove('indeterminate');
      meter.setAttribute('aria-valuenow', String(Math.round(p)));
      segments.forEach((s, i) => s.classList.toggle('on', i < lit));
      value.textContent = `${Math.round(p)}%`;
    },
    setVideo(v) {
      header.replaceChildren(videoHeader(v));
    },
  };
}
