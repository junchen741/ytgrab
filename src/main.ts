import './styles.css';
import { api } from './api';
import { isAppError, type AppError, type AppInfo, type PreparedAudio, type Stage, type VideoInfo } from './api/types';
import { mount } from './ui/dom';
import { editorView } from './views/editor';
import { homeView, type HomeResult, type HomeView } from './views/home';

const UPDATING_TEXT = '正在更新下載元件（第一次開啟或 YouTube 改版後需要），最多約一分鐘…';

const root = document.querySelector<HTMLElement>('#app')!;
let appInfo: AppInfo;
let leaveCurrentView: (() => void) | undefined;
let ytdlpUpdating = false;
/** 上一次處理的網址；回到首頁時不要再把同一個網址從剪貼簿帶進來。 */
let lastUrl = '';
/** 目前的首頁；讀取影片資訊、下載、存好與出錯的結果都在首頁的螢幕窗裡顯示。 */
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

function goHome(prefill = '', result?: HomeResult) {
  home = homeView({
    outputDir: appInfo.outputDir,
    version: appInfo.version,
    prefill,
    result,
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

/** 回到首頁、網址留在框裡，螢幕窗顯示錯誤；按「開始」就能重試。 */
function showError(e: unknown, url: string, stage: Stage) {
  const error = toAppError(e);
  goHome(url, { kind: 'error', error, onCopyReport: () => api.copyErrorReport({ url, stage, error }) });
}

/** 每次按「開始」加一；按「停止」也加一，舊的那次流程看到數字變了就不再往下走。 */
let flowId = 0;

async function startFlow(url: string) {
  lastUrl = url;
  const id = ++flowId;
  const stopped = () => id !== flowId;
  // 開程式時的 yt-dlp 更新還沒結束的話，解析會先等它；先說清楚，免得家人以為當掉了
  const progress = home.startWork(ytdlpUpdating ? UPDATING_TEXT : '正在讀取影片資訊…', () => {
    // 後端收到後會結束 yt-dlp／ffmpeg；不用等它，直接回首頁，網址留在框裡
    flowId++;
    void api.stopWork();
    goHome(url);
  });

  let video: VideoInfo;
  try {
    video = await api.fetchInfo(url);
  } catch (e) {
    if (!stopped()) showError(e, url, '解析');
    return;
  }
  if (stopped()) return;
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
    if (!stopped()) showError(e, video.url, '下載');
    return;
  }
  if (stopped()) return;
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
      goHome('', { kind: 'done', path: out, onReveal: () => void api.revealFile(out) });
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
