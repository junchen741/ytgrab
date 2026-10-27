import type { VideoInfo } from '../api/types';
import { formatClock } from '../lib/time';
import { h } from './dom';

/** 縮圖 + 標題 + 頻道與長度。有封面時改顯示正方形封面（就是 MP3 裡會放的那張）。 */
export function videoHeader(v: VideoInfo, coverUrl: string | null = null): HTMLElement {
  const src = coverUrl ?? v.thumbnail;
  const meta = v.duration ? `${v.channel}・${formatClock(v.duration)}` : v.channel;
  return h(
    'div',
    { class: 'video-header' },
    src ? h('img', { class: coverUrl ? 'thumb square' : 'thumb', src, alt: '' }) : null,
    h('div', {}, h('div', { class: 'video-title' }, v.title), h('div', { class: 'video-meta' }, meta)),
  );
}
