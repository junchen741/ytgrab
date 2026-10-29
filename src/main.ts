import './styles.css';
import { api } from './api';
import { isAppError, type AppError, type AppInfo, type PreparedAudio, type Stage, type VideoInfo } from './api/types';
import { mount } from './ui/dom';
import { doneView } from './views/done';
import { editorView } from './views/editor';
import { errorView } from './views/error';
import { homeView, type HomeView } from './views/home';

const UPDATING_TEXT = '正在更新下載元件（第一次開啟或 YouTube 改版後需要），最多約一分鐘…';

const root = document.querySelector<HTMLElement>('#app')!;
let appInfo: AppInfo;
let leaveCurrentView: (() => void) | undefined;
let ytdlpUpdating = false;
/** 上一次處理的網址；回到首頁時不要再把同一個網址從剪貼簿帶進來。 */
let lastUrl = '';
/** 目前的首頁；讀取影片資訊和下載都在首頁進行（見 HomeView.startWork）。 */
let home: HomeView;

function show(view: HTMLElement, onLeave?: () => void) {
  leaveCurrentView?.();
  leaveCurrentView = onLeave;
  mount(root, view);
}

function toAppError(e: unknown): AppError {
  if (isAppError(e)) return e;
  return { kind: 'unknown', message: '發生了預期外的錯誤。如果一直失敗，請把錯誤報告傳給開發人員。', detail: String(e) };
}

function goHome(prefill = '') {
  home = homeView({
    outputDir: appInfo.outputDir,
    version: appInfo.version,
    prefill,
    suggestUrl: prefill
      ? undefined
      : api
          .clipboardUrl()
          .then((u) => (u && u !== lastUrl ? u : null))
          .catch(() => null),
    onSubmit: (url) => void startFlow(url),
    onChangeFolder: () => void changeFolder(),
    onOpenLogs: () => void api.openLogs(),
    onOpenNotices: () => void api.openNotices(),
  });
  show(home.el);
}

async function changeFolder() {
  const dir = await api.pickFolder(appInfo.outputDir);
  if (!dir) return;
  await api.setOutputDir(dir);
  appInfo = { ...appInfo, outputDir: dir };
  goHome();
}

function showError(e: unknown, url: string, stage: Stage) {
  const error = toAppError(e);
  show(errorView({ error, onCopyReport: () => api.copyErrorReport({ url, stage, error }), onBack: () => goHome(url) }));
}

async function startFlow(url: string) {
  lastUrl = url;
  // 開程式時的 yt-dlp 更新還沒結束的話，解析會先等它；先說清楚，免得家人以為當掉了
  const progress = home.startWork(ytdlpUpdating ? UPDATING_TEXT : '正在讀取影片資訊…');

  let video: VideoInfo;
  try {
    video = await api.fetchInfo(url);
  } catch (e) {
    showError(e, url, '解析');
    return;
  }
  progress.setVideo(video);
  progress.setText('正在下載音訊…');

  let prepared: PreparedAudio;
  try {
    prepared = await api.prepareAudio((ev) => {
      if (ev.stage === 'download') {
        progress.setPercent(ev.percent);
      } else {
        progress.setText('正在分析波形…');
        progress.setPercent(null);
      }
    });
  } catch (e) {
    showError(e, video.url, '下載');
    return;
  }
  openEditor(video, prepared);
}

function openEditor(video: VideoInfo, prepared: PreparedAudio) {
  const editor = editorView({
    video,
    prepared,
    audioUrl: api.audioUrl(prepared.audioPath),
    coverUrl: prepared.coverPath ? api.imageUrl(prepared.coverPath) : null,
    onExport: async (req) => {
      let out: string;
      try {
        out = await api.exportMp3(req);
      } catch (e) {
        const error = toAppError(e);
        if (error.kind === 'tooShort') throw error; // 留在編輯畫面，顯示在按鈕上方
        showError(error, video.url, '匯出');
        return;
      }
      show(doneView({ path: out, onReveal: () => void api.revealFile(out), onNext: () => goHome() }));
    },
    onCancel: () => goHome(),
  });
  // 只有剪歌畫面按 ✕ 會先問；離開剪歌畫面就恢復直接關閉
  const stopGuard = api.guardWindowClose(editor.confirmClose);
  show(editor.el, () => {
    stopGuard();
    editor.destroy();
  });
}

async function boot() {
  appInfo = await api.appInfo();
  ytdlpUpdating = appInfo.ytdlpUpdating;
  await api.onYtdlpUpdating((updating) => {
    ytdlpUpdating = updating;
  });
  goHome();
}

void boot();
