/** 編輯器用：m:ss.s */
export function formatTime(sec: number): string {
  const tenths = Math.round(Math.max(0, sec) * 10);
  const minutes = Math.floor(tenths / 600);
  const seconds = (tenths % 600) / 10;
  return `${minutes}:${seconds.toFixed(1).padStart(4, '0')}`;
}

/** 接受「1:05.3」或「65.3」；格式錯誤回傳 null。 */
export function parseTime(text: string): number | null {
  const m = text.trim().match(/^(?:(\d+):)?(\d+(?:\.\d+)?)$/);
  if (!m) return null;
  const minutes = m[1] ? Number(m[1]) : 0;
  const seconds = Number(m[2]);
  if (m[1] && seconds >= 60) return null;
  return minutes * 60 + seconds;
}

/** 影片長度：m:ss 或 h:mm:ss */
export function formatClock(sec: number): string {
  const total = Math.max(0, Math.round(sec));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const ss = String(total % 60).padStart(2, '0');
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${ss}` : `${minutes}:${ss}`;
}
