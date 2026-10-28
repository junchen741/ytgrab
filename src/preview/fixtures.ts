// 開發用畫面預覽（preview.html）的假資料。全部固定不變，每次截圖才會一樣。
import type { AppError, PreparedAudio, VideoInfo } from '../api/types';

const PEAKS_PER_SECOND = 50;

const LONG_TITLE =
  '【官方完整版】鄧麗君 Teresa Teng《月亮代表我的心》The Moon Represents My Heart 1977 經典重現 高音質 HD 修復版 Official Music Video Remastered';
const LONG_CHANNEL = '鄧麗君文教基金會 Teresa Teng Foundation 官方頻道 Official Channel（經典華語老歌與演唱會珍藏影像）';
const LONG_DIR =
  'C:\\Users\\SomeoneWithAVeryLongUserName\\Music\\YouTube 音訊\\家人分享\\2026 年\\爸媽的老歌精選\\鄧麗君與其他華語經典老歌收藏資料夾\\第二批';

function svgUrl(width: number, height: number): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">` +
    '<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">' +
    '<stop offset="0" stop-color="#C98B6B"/><stop offset="1" stop-color="#5B3A52"/>' +
    '</linearGradient></defs>' +
    `<rect width="${width}" height="${height}" fill="url(#g)"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** 固定的波形：幾個三角函數疊起來，開頭淡入，數值介於 0.05 到 1。 */
function fakePeaks(duration: number): number[] {
  const n = Math.round(duration * PEAKS_PER_SECOND);
  const peaks: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = i / PEAKS_PER_SECOND;
    const body = 0.35 + 0.3 * Math.abs(Math.sin(t * 0.21)) + 0.25 * Math.abs(Math.sin(i * 0.9) * Math.cos(i * 0.13));
    const intro = Math.min(1, t / 8);
    peaks.push(Math.min(1, Math.max(0.05, body * (0.3 + 0.7 * intro))));
  }
  return peaks;
}

export function fixtureVideo(long: boolean): VideoInfo {
  return {
    id: 'preview0001',
    url: 'https://www.youtube.com/watch?v=preview0001',
    title: long ? LONG_TITLE : '月亮代表我的心',
    channel: long ? LONG_CHANNEL : '鄧麗君 Teresa Teng',
    duration: long ? 4530 : 252,
    thumbnail: svgUrl(480, 270),
    live: false,
    tags: { title: long ? LONG_TITLE : '月亮代表我的心', artist: '鄧麗君' },
  };
}

export function fixturePrepared(long: boolean): PreparedAudio {
  const duration = long ? 4530 : 252;
  return {
    audioPath: 'preview.m4a',
    duration,
    peaks: fakePeaks(duration),
    peaksPerSecond: PEAKS_PER_SECOND,
    coverPath: null,
    suggested: long
      ? {
          start: 0,
          end: duration,
          cuts: [
            { start: 300, end: 360 },
            { start: 900, end: 1010.5 },
            { start: 1500, end: 1532.4 },
            { start: 2400, end: 2488 },
            { start: 3100, end: 3175.2 },
            { start: 4000, end: 4100 },
          ],
        }
      : { start: 12, end: 221.5, cuts: [{ start: 112.4, end: 134 }] },
  };
}

export function fixtureCoverUrl(): string {
  return svgUrl(600, 600);
}

export function fixtureOutputDir(long: boolean): string {
  return long ? LONG_DIR : 'C:\\Users\\爸爸\\Music\\YouTube 音訊';
}

export function fixtureSavedPath(long: boolean): string {
  return long ? `${LONG_DIR}\\${LONG_TITLE} - 鄧麗君.mp3` : 'C:\\Users\\爸爸\\Music\\YouTube 音訊\\月亮代表我的心 - 鄧麗君.mp3';
}

export function fixtureError(long: boolean): AppError {
  return {
    kind: 'preview',
    message: long
      ? '這部影片目前無法下載。可能是影片被設成私人、有年齡限制，或是只有特定國家才能觀看。請確認網址是不是正確的影片，或換一部影片再試一次；如果一直失敗，請按「複製錯誤報告」，把內容貼到 LINE 傳給開發人員。'
      : '這部影片無法下載，可能是私人影片。',
    detail: 'preview fixture',
  };
}

/** 8 kHz、8-bit 單聲道的無聲 WAV，給預覽頁的 wavesurfer 當音訊來源。 */
export function silentWavUrl(seconds: number): string {
  const rate = 8000;
  const samples = Math.max(1, Math.round(rate * seconds));
  const buf = new ArrayBuffer(44 + samples);
  const v = new DataView(buf);
  const ascii = (offset: number, s: string) => {
    for (let i = 0; i < s.length; i++) v.setUint8(offset + i, s.charCodeAt(i));
  };
  ascii(0, 'RIFF');
  v.setUint32(4, 36 + samples, true);
  ascii(8, 'WAVE');
  ascii(12, 'fmt ');
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true); // PCM
  v.setUint16(22, 1, true); // 單聲道
  v.setUint32(24, rate, true);
  v.setUint32(28, rate, true); // byte rate
  v.setUint16(32, 1, true); // block align
  v.setUint16(34, 8, true); // bits per sample
  ascii(36, 'data');
  v.setUint32(40, samples, true);
  new Uint8Array(buf, 44).fill(128); // 8-bit 的無聲是 128
  return URL.createObjectURL(new Blob([buf], { type: 'audio/wav' }));
}
