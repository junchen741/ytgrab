import type { AppError } from '../api/types';
import { h } from '../ui/dom';

export function errorView(p: { error: AppError; onCopyReport(): Promise<void>; onBack(): void }): HTMLElement {
  const status = h('p', { class: 'hint' });
  const copy = async () => {
    try {
      await p.onCopyReport();
      status.textContent = '已複製！請直接貼到 LINE 傳給開發人員。';
    } catch {
      status.textContent = '複製失敗。請按首頁下方的「開啟記錄資料夾」，把裡面最新的檔案傳給開發人員。';
    }
  };
  return h(
    'section',
    { class: 'view error' },
    h('h2', {}, '出了一點問題'),
    h('p', { class: 'error-message' }, p.error.message),
    h(
      'div',
      { class: 'row' },
      h('button', { class: 'primary', onclick: copy }, '複製錯誤報告'),
      h('button', { onclick: p.onBack }, '回到首頁'),
    ),
    status,
  );
}
