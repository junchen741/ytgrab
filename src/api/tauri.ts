import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { Api } from './index';
import type { PreparedAudio, ProgressEvent } from './types';
import { DESIGN, fitWindow } from '../lib/windowFit';

const ZOOM_KEY = 'ytgrab-zoom';
/** 放大後的視窗和螢幕工作區邊緣至少留這麼多（邏輯像素） */
const FIT_MARGIN = 8;

export const tauriApi: Api = {
  appInfo: () => invoke('app_info'),
  fetchInfo: (url) => invoke('fetch_info', { url }),
  async prepareAudio(onProgress) {
    const unlisten = await listen<ProgressEvent>('prepare-progress', (e) => onProgress(e.payload));
    try {
      return await invoke<PreparedAudio>('prepare_audio');
    } finally {
      unlisten();
    }
  },
  stopWork: () => invoke('stop_work'),
  exportMp3: (req) => invoke('export_mp3', { req }),
  setOutputDir: (dir) => invoke('set_output_dir', { dir }),
  async pickFolder(current) {
    const picked = await open({ directory: true, defaultPath: current, title: '選擇存檔資料夾' });
    return typeof picked === 'string' ? picked : null;
  },
  async revealFile(path) {
    await revealItemInDir(path);
  },
  openLogs: () => invoke('open_special', { target: 'logs' }),
  openNotices: () => invoke('open_special', { target: 'notices' }),
  copyErrorReport: ({ url, stage, error }) =>
    invoke('copy_error_report', { url, stage, kind: error.kind, detail: error.detail }),
  clipboardUrl: () => invoke('clipboard_youtube_url'),
  onYtdlpUpdating: (cb) => listen<boolean>('ytdlp-update', (e) => cb(e.payload)),
  // 有人在聽關閉事件時，Tauri 會先擋下關窗；沒呼叫 preventDefault 的話由這裡關掉視窗
  //（要有 capabilities 的 core:window:allow-destroy 權限，否則選了「關閉」也關不掉）
  guardWindowClose(shouldClose) {
    const unlisten = getCurrentWindow().onCloseRequested(async (e) => {
      if (!(await shouldClose())) e.preventDefault();
    });
    return () => void unlisten.then((stop) => stop());
  },
  async fitWindow() {
    const win = getCurrentWindow();
    // 網頁重新整理時，上次設的縮放還在，devicePixelRatio 會把它算進去，要扣掉
    let applied = 1;
    try {
      applied = Number(sessionStorage.getItem(ZOOM_KEY)) || 1;
    } catch {
      // 讀不到就當作沒縮放過
    }
    const [sf, monitor, outer, inner] = await Promise.all([
      win.scaleFactor(),
      currentMonitor(),
      win.outerSize(),
      win.innerSize(),
    ]);
    const textScale = window.devicePixelRatio / (sf * applied);
    // 視窗框（標題列、邊框）的大小，邏輯像素
    const frameW = (outer.width - inner.width) / sf;
    const frameH = (outer.height - inner.height) / sf;
    const area = monitor?.workArea;
    const availW = area ? area.size.width / sf - frameW - FIT_MARGIN * 2 : 0;
    const availH = area ? area.size.height / sf - frameH - FIT_MARGIN * 2 : 0;
    const fit = fitWindow(textScale, availW, availH);

    const zoom = fit?.zoom ?? 1;
    if (Math.abs(zoom - applied) > 0.001) {
      await getCurrentWebview().setZoom(zoom);
      try {
        sessionStorage.setItem(ZOOM_KEY, String(zoom));
      } catch {
        // 存不了只影響開發時重新整理，忽略
      }
    }
    if (!fit || fit.scale === 1) return;

    const w = Math.round(DESIGN.width * fit.scale);
    const h = Math.round(DESIGN.height * fit.scale);
    await win.setMinSize(new LogicalSize(Math.round(DESIGN.minWidth * fit.scale), Math.round(DESIGN.minHeight * fit.scale)));
    await win.setSize(new LogicalSize(w, h));
    if (area) {
      // 置中在扣掉工作列的區域，不要壓到工作列
      const x = area.position.x + (area.size.width - (w + frameW) * sf) / 2;
      const y = area.position.y + (area.size.height - (h + frameH) * sf) / 2;
      await win.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
    }
  },
  audioUrl: (path) => convertFileSrc(path),
  imageUrl: (path) => convertFileSrc(path),
};
