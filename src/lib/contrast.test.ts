// vitest 預設會把 CSS 匯入（包括 ?raw）換成空字串，所以直接讀檔。
// @ts-expect-error 沒有安裝 @types/node（和 vite.config.ts 一樣）
import { readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { contrastRatio, tokenColors } from './contrast';

const css: string = readFileSync(new URL('../styles.css', import.meta.url), 'utf8');

test('contrastRatio 黑白是 21、同色是 1', () => {
  expect(contrastRatio('#000000', '#FFFFFF')).toBeCloseTo(21, 1);
  expect(contrastRatio('#777777', '#777777')).toBeCloseTo(1, 5);
});

test('tokenColors 只取 #RRGGBB', () => {
  expect(tokenColors(':root { --a: #112233; --b: rgba(1,2,3,.5); }')).toEqual({ a: '#112233' });
});

test.each([
  ['ink', 'body-top'],
  ['ink', 'body-bottom'],
  ['ink-muted', 'body-top'],
  ['ink-muted', 'body-bottom'],
  ['ink', 'key'],
  ['ink-on-key', 'key'],
  ['ink', 'well'],
  ['ink-muted', 'well'],
  ['phosphor', 'glass'],
  ['amber', 'glass'],
  ['cut', 'glass'],
  ['amber-ink', 'amber'],
])('styles.css：--%s 放在 --%s 上的對比 ≥ 4.5', (fg, bg) => {
  const t = tokenColors(css);
  expect(contrastRatio(t[fg], t[bg])).toBeGreaterThanOrEqual(4.5);
});
