import type { AppError } from '../api/types';
import { h } from '../ui/dom';
import { progressDisplay, type ProgressDisplay } from '../ui/progress';
import { donePanel } from './done';
import { errorPanel } from './error';

/** 上一首的結果：存好了，或出了問題。顯示在首頁的螢幕窗裡，取代「準備好了」。 */
export type HomeResult =
  | { kind: 'done'; path: string; onReveal(): void }
  | { kind: 'error'; error: AppError; onCopyReport(): Promise<void> };

export interface HomeProps {
  outputDir: string;
  version: string;
  prefill: string;
  result?: HomeResult;
  /** 剪貼簿裡的 YouTube 網址；輸入框還是空的時候才自動帶入。 */
  suggestUrl?: Promise<string | null>;
  onSubmit(url: string): void;
  onChangeFolder(): void;
  onOpenLogs(): void;
  onOpenNotices(): void;
}

export interface HomeView {
  el: HTMLElement;
  /**
   * 按下「開始」之後呼叫：網址列和存檔位置先鎖住，螢幕窗從「準備好了」（或上一首的結果）換成進度顯示。
   * 讀取影片資訊和下載都在這一頁進行，不換頁；下載完才進剪歌畫面。
   * 螢幕窗下面有「停止」鍵，按了呼叫 `onStop`。
   */
  startWork(initialText: string, onStop: () => void): ProgressDisplay;
}

export function homeView(p: HomeProps): HomeView {
  const input = h('input', { type: 'text', class: 'url-input', placeholder: '把 YouTube 網址貼在這裡', value: p.prefill });
  const hint = h('p', { class: 'hint' });
  const submit = () => {
    if (input.disabled) return;
    const url = input.value.trim();
    if (!url) {
      hint.textContent = '請先貼上 YouTube 影片網址。';
      return;
    }
    p.onSubmit(url);
  };
  const startBtn = h('button', { class: 'primary big', onclick: submit }, '開始');
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submit();
  });
  queueMicrotask(() => input.focus());
  void p.suggestUrl?.then((url) => {
    if (url && !input.value && !input.disabled) {
      input.value = url;
      input.select(); // 使用者若要貼別的網址，直接 Ctrl+V 就會取代
      hint.textContent = '已自動貼上剪貼簿裡的 YouTube 網址，按「開始」就好。';
    }
  });

  // 產品名在視窗標題列上，螢幕窗只顯示狀態：準備好了、上一首存好了、或出了問題
  const statusPanel = !p.result
    ? h(
        'div',
        { class: 'display' },
        h('p', { class: 'display-title' }, '準備好了'),
        h('p', {}, '貼上 YouTube 網址，把歌剪好存成 MP3。'),
      )
    : p.result.kind === 'done'
      ? donePanel(p.result)
      : errorPanel(p.result);

  // 路徑放在凹槽框裡，看起來像可以調整的欄位；點框本身或「變更」都會開選資料夾。
  // 框不放進 Tab 順序（tabindex -1），用鍵盤的人用「變更」，免得同一個動作要按兩次 Tab
  const pathWell = h(
    'button',
    { class: 'path-well', tabindex: -1, title: '按一下選擇存檔的資料夾', onclick: p.onChangeFolder },
    p.outputDir,
  );
  const changeBtn = h('button', { class: 'nudge', onclick: p.onChangeFolder }, '變更');

  // 網址列在最上面：打開軟體的第一件事就是貼網址；螢幕窗在下面顯示狀態
  const el = h(
    'section',
    { class: 'view home' },
    h('div', { class: 'url-row' }, input, startBtn),
    hint,
    statusPanel,
    h(
      'div',
      { class: 'folder' },
      h('span', { class: 'folder-label' }, '存檔位置'),
      h('div', { class: 'folder-row' }, pathWell, changeBtn),
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

  return {
    el,
    startWork(initialText, onStop) {
      for (const c of [input, startBtn, pathWell, changeBtn]) c.disabled = true;
      hint.textContent = '';
      const progress = progressDisplay(initialText);
      // 貼錯網址或等太久時可以中途放棄；按了就回首頁，網址留在框裡
      const stopBtn = h('button', { onclick: onStop }, '停止');
      statusPanel.replaceWith(h('div', { class: 'panel' }, progress.el, h('div', { class: 'row' }, stopBtn)));
      return progress;
    },
  };
}
