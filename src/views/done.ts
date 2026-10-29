import { h } from '../ui/dom';

/**
 * 首頁螢幕窗的「完成了」狀態（存完 MP3 回到首頁時顯示）。
 * 不另外放「處理下一首」：網址列就在上面，直接貼下一首就好。
 */
export function donePanel(p: { path: string; onReveal(): void }): HTMLElement {
  const name = p.path.split(/[\\/]/).pop() ?? p.path;
  return h(
    'div',
    { class: 'panel' },
    h(
      'div',
      { class: 'display' },
      h('p', { class: 'display-title' }, '完成了！'),
      h('p', { class: 'saved' }, '已儲存：', h('strong', {}, name)),
      h('p', {}, '用 USB 線把檔案複製到手機的「Music」資料夾就能聽了。'),
    ),
    // 主鍵是網址列旁的「開始」，這裡用一般按鍵
    h(
      'div',
      { class: 'row' },
      h('button', { onclick: p.onReveal }, '開啟資料夾'),
      h('span', { class: 'hint' }, '要剪下一首，把網址貼在上面就好。'),
    ),
  );
}
