// 開發用畫面預覽：/preview.html?view=<home|progress|progress-wait|editor|done|error>[&long=1]
// 用假資料畫出單一畫面，給 scripts/preview-shots.ps1 截圖與檢查橫向捲軸。正式版不會打包這一頁。
// 剪歌畫面可以加 &peaks=<網址>，改用真實歌曲的波形（JSON：{ "duration": 秒, "peaks": [...] }，每秒 50 點）。
import type { PreparedAudio } from '../api/types';
import '../styles.css';
import { mount } from '../ui/dom';
import { doneView } from '../views/done';
import { editorView } from '../views/editor';
import { errorView } from '../views/error';
import { homeView } from '../views/home';
import {
  fixtureCoverUrl,
  fixtureError,
  fixtureOutputDir,
  fixturePrepared,
  fixtureSavedPath,
  fixtureVideo,
  silentWavUrl,
} from './fixtures';

const params = new URLSearchParams(location.search);
const view = params.get('view') ?? 'home';
const long = params.get('long') === '1';
const root = document.querySelector<HTMLElement>('#app')!;
const log =
  (name: string) =>
  (...args: unknown[]) =>
    console.log(`[preview] ${name}`, ...args);

const home = (url: string | null) =>
  homeView({
    outputDir: fixtureOutputDir(long),
    version: '0.1.0',
    prefill: '',
    suggestUrl: Promise.resolve(url),
    onSubmit: log('onSubmit'),
    onChangeFolder: log('onChangeFolder'),
    onOpenLogs: log('onOpenLogs'),
    onOpenNotices: log('onOpenNotices'),
  });

const editor = (prepared: PreparedAudio) =>
  editorView({
    video: fixtureVideo(long),
    prepared,
    // 音訊長度要和 prepared.duration 一樣：音訊載入後 wavesurfer 會改用音訊本身的長度，
    // 太短的話區段會被當成畫面外而不畫，拖拉新增的刪除段也會短到被當成誤觸刪掉
    audioUrl: silentWavUrl(prepared.duration),
    coverUrl: fixtureCoverUrl(),
    onExport: async (req) => log('onExport')(req),
    onCancel: log('onCancel'),
  }).el;

function render(): Node {
  switch (view) {
    // 讀取與下載都在首頁進行：先畫首頁，再像 main.ts 一樣呼叫 startWork
    case 'progress': {
      const h = home(null);
      const p = h.startWork('正在讀取影片資訊…');
      p.setVideo(fixtureVideo(long));
      p.setText('正在下載音訊…');
      p.setPercent(42);
      return h.el;
    }
    case 'progress-wait': {
      const h = home(null);
      // 和 main.ts 的 UPDATING_TEXT 同一段話
      const p = h.startWork('正在更新下載元件（第一次開啟或 YouTube 改版後需要），最多約一分鐘…');
      p.setPercent(null);
      return h.el;
    }
    case 'editor': {
      const peaksUrl = params.get('peaks');
      if (peaksUrl) {
        void fetch(peaksUrl)
          .then((r) => r.json() as Promise<{ duration: number; peaks: number[] }>)
          .then(({ duration, peaks }) =>
            mount(root, editor({ ...fixturePrepared(false), duration, peaks, suggested: { start: 0, end: duration, cuts: [] } })),
          );
        return document.createTextNode('');
      }
      return editor(fixturePrepared(long));
    }
    case 'done':
      return doneView({ path: fixtureSavedPath(long), onReveal: log('onReveal'), onNext: log('onNext') });
    case 'error':
      return errorView({ error: fixtureError(long), onCopyReport: async () => log('onCopyReport')(), onBack: log('onBack') });
    default:
      return home(long ? 'https://www.youtube.com/watch?v=preview0001' : null).el;
  }
}

mount(root, render());

// 等 wavesurfer 和字型就緒，再標記有沒有橫向捲軸
setTimeout(() => {
  const doc = document.documentElement;
  document.body.dataset.overflow = doc.scrollWidth > doc.clientWidth ? 'yes' : 'no';
  document.body.dataset.ready = '1';
}, 1500);
