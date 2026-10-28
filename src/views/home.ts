import { h } from '../ui/dom';

export interface HomeProps {
  outputDir: string;
  version: string;
  prefill: string;
  /** 剪貼簿裡的 YouTube 網址；輸入框還是空的時候才自動帶入。 */
  suggestUrl?: Promise<string | null>;
  onSubmit(url: string): void;
  onChangeFolder(): void;
  onOpenLogs(): void;
  onOpenNotices(): void;
}

export function homeView(p: HomeProps): HTMLElement {
  const input = h('input', { type: 'text', class: 'url-input', placeholder: '把 YouTube 網址貼在這裡', value: p.prefill });
  const hint = h('p', { class: 'hint' });
  const submit = () => {
    const url = input.value.trim();
    if (!url) {
      hint.textContent = '請先貼上 YouTube 影片網址。';
      return;
    }
    p.onSubmit(url);
  };
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submit();
  });
  queueMicrotask(() => input.focus());
  void p.suggestUrl?.then((url) => {
    if (url && !input.value) {
      input.value = url;
      input.select(); // 使用者若要貼別的網址，直接 Ctrl+V 就會取代
      hint.textContent = '已自動貼上剪貼簿裡的 YouTube 網址，按「開始」就好。';
    }
  });

  // 產品名已經在機身的名牌上（index.html），螢幕窗只顯示狀態
  return h(
    'section',
    { class: 'view home' },
    h(
      'div',
      { class: 'display' },
      h('p', { class: 'display-title' }, '準備好了'),
      h('p', {}, '貼上 YouTube 網址，把歌剪好存成 MP3。'),
    ),
    h('div', { class: 'url-row' }, input, h('button', { class: 'primary big', onclick: submit }, '開始')),
    hint,
    h(
      'div',
      { class: 'folder' },
      '存檔位置：',
      h('span', { class: 'path' }, p.outputDir),
      h('button', { class: 'link', onclick: p.onChangeFolder }, '變更'),
    ),
    h(
      'footer',
      {},
      h('span', {}, `v${p.version}　僅供個人使用，請尊重著作權`),
      h(
        'span',
        { class: 'footer-links' },
        h('button', { class: 'link', onclick: p.onOpenNotices }, '授權資訊'),
        h('button', { class: 'link', onclick: p.onOpenLogs }, '開啟記錄資料夾'),
      ),
    ),
  );
}
