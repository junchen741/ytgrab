/** 段式進度條要亮幾格：先把百分比限制在 0–100，再四捨五入；不是有限數字時回傳 0。 */
export function litSegments(percent: number, total: number): number {
  if (!Number.isFinite(percent)) return 0;
  const p = Math.min(100, Math.max(0, percent));
  return Math.round((p / 100) * total);
}
