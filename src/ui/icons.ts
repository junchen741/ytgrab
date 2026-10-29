// 按鍵上的小圖示：同一套 SVG、同樣高度，顏色跟著按鍵文字（currentColor）。
// 不用 ▶ ⏯ 這類文字符號：它們的大小、粗細跟著字型走，⏯ 會比 ▶ 小一號。
const SVG_NS = 'http://www.w3.org/2000/svg';

const SHAPES = {
  /** 播放：向右的三角形 */
  play: { width: 16, paths: ['M3 1.5 14 8 3 14.5Z'] },
  /** 播放／暫停：三角形加兩條直槓 */
  playPause: { width: 26, paths: ['M2 1.5 12 8 2 14.5Z', 'M16 2h3.2v12H16Z', 'M21.8 2H25v12h-3.2Z'] },
} as const;

export type IconName = keyof typeof SHAPES;

export function icon(name: IconName): SVGSVGElement {
  const shape = SHAPES[name];
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'icon');
  svg.setAttribute('viewBox', `0 0 ${shape.width} 16`);
  svg.setAttribute('aria-hidden', 'true');
  for (const d of shape.paths) {
    const path = document.createElementNS(SVG_NS, 'path');
    path.setAttribute('d', d);
    path.setAttribute('fill', 'currentColor');
    svg.append(path);
  }
  return svg;
}
