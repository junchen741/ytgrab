/** 視窗內容的設計尺寸（邏輯像素），要和 src-tauri/tauri.conf.json 的 width/height/minWidth/minHeight 一致。 */
export const DESIGN = { width: 900, height: 740, minWidth: 720, minHeight: 600 };

export interface WindowFit {
  /** 視窗內容的大小、最小大小要乘上的倍率 */
  scale: number;
  /** 網頁要設的縮放，讓網頁看到的版面仍是設計尺寸 */
  zoom: number;
}

/**
 * Windows「協助工具 → 文字大小」調大時，WebView2 會把整個網頁等比例放大，但視窗大小不變，
 * 版面就被擠小（150% 時 900×740 的視窗只剩 600×493 可用，剪歌畫面會出現捲軸）。
 *
 * 做法：視窗跟著放大，盡量保留使用者要的大字；螢幕放不下時視窗只放到螢幕容得下的大小，
 * 再用網頁縮放把內容縮回來，讓網頁看到的版面一律是設計尺寸。
 *
 * @param textScale 網頁比視窗多出來的放大倍率（devicePixelRatio ÷ 螢幕縮放）
 * @param availWidth 螢幕工作區扣掉視窗框之後，視窗內容最大能有多寬（邏輯像素）
 * @param availHeight 同上，高度
 * @returns 不用調整時回傳 null
 */
export function fitWindow(textScale: number, availWidth: number, availHeight: number): WindowFit | null {
  if (!Number.isFinite(textScale) || textScale < 1.05) return null;
  const roomForScale = Math.min(availWidth / DESIGN.width, availHeight / DESIGN.height);
  const scale = Math.max(1, Math.min(textScale, Number.isFinite(roomForScale) ? roomForScale : 1));
  return { scale, zoom: scale / textScale };
}
