import { h } from '../ui/dom';

export function doneView(p: { path: string; onReveal(): void; onNext(): void }): HTMLElement {
  const name = p.path.split(/[\\/]/).pop() ?? p.path;
  return h(
    'section',
    { class: 'view done' },
    h(
      'div',
      { class: 'display' },
      h('p', { class: 'display-title' }, '完成了！'),
      h('p', { class: 'saved' }, '已儲存：', h('strong', {}, name)),
    ),
    h('p', { class: 'hint' }, '用 USB 線把檔案複製到手機的「Music」資料夾就能聽了。'),
    h(
      'div',
      { class: 'row' },
      h('button', { class: 'primary big', onclick: p.onReveal }, '開啟資料夾'),
      h('button', { class: 'big', onclick: p.onNext }, '處理下一首'),
    ),
  );
}
