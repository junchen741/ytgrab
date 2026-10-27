import type { VideoInfo } from '../api/types';
import { h } from '../ui/dom';
import { videoHeader } from '../ui/videoHeader';

export function progressView(initialText: string) {
  const header = h('div');
  const text = h('p', { class: 'lead' }, initialText);
  const bar = h('progress', { max: 100 });
  const el = h('section', { class: 'view progress' }, header, text, bar);
  return {
    el,
    setText(t: string) {
      text.textContent = t;
    },
    /** null = 不確定進度（跑馬燈） */
    setPercent(p: number | null) {
      if (p === null) bar.removeAttribute('value');
      else bar.value = p;
    },
    setVideo(v: VideoInfo) {
      header.replaceChildren(videoHeader(v));
    },
  };
}
