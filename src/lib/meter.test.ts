import { expect, test } from 'vitest';
import { litSegments } from './meter';

test.each([
  [0, 0],
  [2.4, 0],
  [2.5, 1],
  [42, 8],
  [100, 20],
  [130, 20],
  [-5, 0],
  [Number.NaN, 0],
])('litSegments(%s, 20) = %s', (p, want) => {
  expect(litSegments(p, 20)).toBe(want);
});
