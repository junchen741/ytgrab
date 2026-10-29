// 畫面只認得這個介面。桌面版用 Tauri 實作；網頁版（A）之後換成呼叫 Cloudflare Worker 的實作。
import type { AppError, AppInfo, ExportRequest, PreparedAudio, ProgressEvent, Stage, VideoInfo } from './types';

export interface Api {
  appInfo(): Promise<AppInfo>;
  fetchInfo(url: string): Promise<VideoInfo>;
  prepareAudio(onProgress: (e: ProgressEvent) => void): Promise<PreparedAudio>;
  /** 首頁的「停止」：正在進行的 fetchInfo／prepareAudio 會以 kind 'cancelled' 的錯誤結束。 */
  stopWork(): Promise<void>;
  exportMp3(req: ExportRequest): Promise<string>;
  setOutputDir(dir: string): Promise<void>;
  pickFolder(current: string): Promise<string | null>;
  revealFile(path: string): Promise<void>;
  openLogs(): Promise<void>;
  openNotices(): Promise<void>;
  copyErrorReport(ctx: { url: string; stage: Stage; error: AppError }): Promise<void>;
  /** 剪貼簿裡是 YouTube 影片網址才回傳。 */
  clipboardUrl(): Promise<string | null>;
  /** yt-dlp 開始／結束更新時通知；回傳取消訂閱的函式。 */
  onYtdlpUpdating(cb: (updating: boolean) => void): Promise<() => void>;
  /** 使用者按視窗的 ✕（或 Alt+F4）時先問 `shouldClose`，回答 false 就不關；回傳取消攔截的函式。 */
  guardWindowClose(shouldClose: () => Promise<boolean>): () => void;
  audioUrl(path: string): string;
  imageUrl(path: string): string;
}

export { tauriApi as api } from './tauri';
