import type { AppError } from '../api/types';
import { h } from '../ui/dom';

/**
 * 首頁螢幕窗的「出了一點問題」狀態（琥珀色）。網址還留在上面的框裡，
 * 所以不另外放「回到首頁」：按「開始」就能重試，也可以直接換一個網址。
 */
export function errorPanel(p: { error: AppError; onCopyReport(): Promise<void> }): HTMLElement {
  const status = h('span', { class: 'hint', role: 'status' });
  const copy = async () => {
    try {
      await p.onCopyReport();
      status.textContent = '已複製！請直接貼到 LINE 傳給開發人員。';
    } catch {
      status.textContent = '複製失敗。請按下方的「開啟記錄資料夾」，把裡面最新的檔案傳給開發人員。';
    }
  };
  return h(
    'div',
    { class: 'panel' },
    h(
      'div',
      { class: 'display warn' },
      h('p', { class: 'display-title' }, '出了一點問題'),
      h('p', { class: 'error-message' }, p.error.message),
      h('p', {}, '網址還在上面，可以按「開始」再試一次，或換一個網址。'),
    ),
    // 主鍵是網址列旁的「開始」，這裡用一般按鍵
    h('div', { class: 'row' }, h('button', { onclick: copy }, '複製錯誤報告'), status),
  );
}
