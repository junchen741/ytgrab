// 檢查設計 token 的文字對比（WCAG 2.x），見 contrast.test.ts。

function luminance(hex: string): number {
  const channel = (i: number) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** 兩個 `#RRGGBB` 顏色的對比值（1 到 21）。 */
export function contrastRatio(fg: string, bg: string): number {
  const a = luminance(fg);
  const b = luminance(bg);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** 取出 CSS 文字裡所有 `--名稱: #RRGGBB`，key 不含 `--`；rgba 之類的值略過。 */
export function tokenColors(css: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const m of css.matchAll(/--([\w-]+)\s*:\s*(#[0-9a-fA-F]{6})\s*[;}]/g)) out[m[1]] = m[2];
  return out;
}
