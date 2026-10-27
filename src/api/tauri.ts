import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { Api } from './index';
import type { PreparedAudio, ProgressEvent } from './types';

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
  audioUrl: (path) => convertFileSrc(path),
  imageUrl: (path) => convertFileSrc(path),
};
