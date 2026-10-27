export interface SongTags {
  title: string;
  artist: string;
}

export interface VideoInfo {
  id: string;
  url: string;
  title: string;
  channel: string;
  duration: number | null;
  thumbnail: string | null;
  live: boolean;
  tags: SongTags;
}

export interface Range {
  start: number;
  end: number;
}

export interface TrimSpec {
  start: number;
  end: number;
  cuts: Range[];
}

export interface PreparedAudio {
  audioPath: string;
  duration: number;
  peaks: number[];
  peaksPerSecond: number;
  coverPath: string | null;
  suggested: TrimSpec;
}

export interface ExportRequest {
  trim: TrimSpec;
  fadeIn: boolean;
  fadeOut: boolean;
  title: string;
  artist: string;
}

export interface AppInfo {
  version: string;
  ytdlpVersion: string;
  ytdlpUpdating: boolean;
  outputDir: string;
}

export interface ProgressEvent {
  stage: 'download' | 'analyze';
  percent: number | null;
}

export type Stage = '解析' | '下載' | '匯出';

export interface AppError {
  kind: string;
  message: string;
  detail: string;
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'kind' in e && 'message' in e && 'detail' in e;
}
