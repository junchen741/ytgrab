import WaveSurfer from 'wavesurfer.js';
import RegionsPlugin from 'wavesurfer.js/dist/plugins/regions.esm.js';
import type { AppError, ExportRequest, PreparedAudio, TrimSpec, VideoInfo } from '../api/types';
import { endPreviewStart, nextPlaybackAction } from '../lib/playback';
import { formatTime, parseTime } from '../lib/time';
import { approxLength, EDGE_MIN, MIN_GAP, trimFromRegions } from '../lib/trim';
import { h } from '../ui/dom';
import { videoHeader } from '../ui/videoHeader';

type Region = ReturnType<RegionsPlugin['addRegion']>;

const COLOR_EDGE = 'rgba(100, 116, 139, 0.45)';
const COLOR_CUT = 'rgba(220, 38, 38, 0.30)';
const COLOR_CUT_SELECTED = 'rgba(220, 38, 38, 0.60)';
/** 拖拉出來比這更短的紅色區段視為誤觸，自動刪掉（秒）。 */
const MIN_CUT = 0.5;

export interface EditorProps {
  video: VideoInfo;
  prepared: PreparedAudio;
  audioUrl: string;
  coverUrl: string | null;
  onExport(req: ExportRequest): Promise<void>;
  onCancel(): void;
}

export function editorView(p: EditorProps): { el: HTMLElement; destroy(): void } {
  const { duration, suggested } = p.prepared;

  // ---- 波形與區段 ----
  const waveEl = h('div', { class: 'waveform' });
  const regions = RegionsPlugin.create();
  const ws = WaveSurfer.create({
    container: waveEl,
    url: p.audioUrl,
    peaks: [p.prepared.peaks],
    duration,
    height: 140,
    normalize: true,
    waveColor: '#94a3b8',
    progressColor: '#0f766e',
    cursorColor: '#f59e0b',
    cursorWidth: 2,
    plugins: [regions],
  });

  // 區段都在下方 ws.once('ready') 裡建立；ready 之前一律用建議值。
  let head!: Region;
  let tail!: Region;
  let ready = false;
  const cuts = new Map<string, Region>();
  let selected: Region | null = null;

  const currentTrim = (): TrimSpec =>
    ready
      ? trimFromRegions(head.end, tail.start, duration, [...cuts.values()].map((r) => ({ start: r.start, end: r.end })))
      : { start: suggested.start, end: suggested.end, cuts: suggested.cuts.map((c) => ({ ...c })) };

  // ---- 起點／終點 ----
  const startInput = h('input', { type: 'text', class: 'time' });
  const endInput = h('input', { type: 'text', class: 'time' });
  const lengthLabel = h('span', { class: 'hint' });

  function refresh() {
    const t = currentTrim();
    startInput.value = formatTime(t.start);
    endInput.value = formatTime(t.end);
    lengthLabel.textContent = `成品長度約 ${formatTime(approxLength(t))}`;
  }

  function setStart(v: number) {
    if (!ready) return;
    const clamped = Math.min(Math.max(v, 0), tail.start - MIN_GAP);
    head.setOptions({ start: 0, end: Math.max(clamped, EDGE_MIN) });
    refresh();
  }

  function setEnd(v: number) {
    if (!ready) return;
    const clamped = Math.max(Math.min(v, duration), head.end + MIN_GAP);
    tail.setOptions({ start: Math.min(clamped, duration - EDGE_MIN), end: duration });
    refresh();
  }

  startInput.addEventListener('change', () => {
    const v = parseTime(startInput.value);
    if (v === null) refresh();
    else setStart(v);
  });
  endInput.addEventListener('change', () => {
    const v = parseTime(endInput.value);
    if (v === null) refresh();
    else setEnd(v);
  });

  const nudge = (label: string, fn: () => void) => h('button', { class: 'nudge', onclick: fn }, label);
  const timeRow = (label: string, input: HTMLInputElement, set: (v: number) => void, get: () => number) =>
    h(
      'div',
      { class: 'time-row' },
      h('label', {}, label),
      input,
      nudge('−1 秒', () => set(get() - 1)),
      nudge('−0.1', () => set(get() - 0.1)),
      nudge('+0.1', () => set(get() + 0.1)),
      nudge('+1 秒', () => set(get() + 1)),
      nudge('設為目前播放位置', () => set(ws.getCurrentTime())),
    );

  // ---- 刪除區段 ----
  const cutList = h('ul', { class: 'cut-list' });

  function renderCutList() {
    const sorted = [...cuts.values()].sort((a, b) => a.start - b.start);
    const items = sorted.map((r, i) =>
      h(
        'li',
        { class: r === selected ? 'selected' : '' },
        h('span', {}, `刪除區段 ${i + 1}：${formatTime(r.start)} – ${formatTime(r.end)}`),
        h('button', { class: 'link', onclick: () => playRaw(Math.max(0, r.start - 2)) }, '聽聽看'),
        h('button', { class: 'link danger', onclick: () => removeCut(r) }, '取消刪除'),
      ),
    );
    cutList.replaceChildren(
      ...(items.length > 0
        ? items
        : [h('li', { class: 'empty' }, '沒有刪除區段。要剪掉中間某一段，就在波形上按住滑鼠拖拉。')]),
    );
    refresh();
  }

  function registerCut(r: Region) {
    if (r === head || r === tail || cuts.has(r.id)) return;
    cuts.set(r.id, r);
    renderCutList();
  }

  function removeCut(r: Region) {
    cuts.delete(r.id);
    if (selected === r) selected = null;
    r.remove();
    renderCutList();
  }

  function select(r: Region | null) {
    selected?.setOptions({ color: COLOR_CUT });
    selected = r;
    r?.setOptions({ color: COLOR_CUT_SELECTED });
    renderCutList();
  }

  function useWholeTrack() {
    if (!ready) return;
    head.setOptions({ start: 0, end: EDGE_MIN });
    tail.setOptions({ start: duration - EDGE_MIN, end: duration });
    for (const r of [...cuts.values()]) removeCut(r);
    renderCutList();
  }

  ws.once('ready', () => {
    head = regions.addRegion({
      id: 'head',
      start: 0,
      end: Math.max(suggested.start, EDGE_MIN),
      color: COLOR_EDGE,
      drag: false,
      resizeStart: false,
      minLength: EDGE_MIN,
    });
    tail = regions.addRegion({
      id: 'tail',
      start: Math.min(suggested.end, duration - EDGE_MIN),
      end: duration,
      color: COLOR_EDGE,
      drag: false,
      resizeEnd: false,
      minLength: EDGE_MIN,
    });
    ready = true;
    for (const c of suggested.cuts) {
      registerCut(regions.addRegion({ start: c.start, end: c.end, color: COLOR_CUT }));
    }

    regions.enableDragSelection({ color: COLOR_CUT });
    // 拖拉的區段在放開滑鼠時才觸發 region-created（已對照原始碼確認），這時長度已是最終值
    regions.on('region-created', (r) => {
      if (r === head || r === tail || cuts.has(r.id)) return;
      if (r.end - r.start < MIN_CUT) {
        r.remove();
        return;
      }
      registerCut(r);
    });
    regions.on('region-update', () => refresh());
    regions.on('region-updated', (r) => {
      if (r === head && head.end > tail.start - MIN_GAP) head.setOptions({ start: 0, end: tail.start - MIN_GAP });
      if (r === tail && tail.start < head.end + MIN_GAP) tail.setOptions({ start: head.end + MIN_GAP, end: duration });
      renderCutList();
    });
    regions.on('region-clicked', (r, e) => {
      if (!cuts.has(r.id)) return;
      e.stopPropagation();
      select(r);
    });
    renderCutList();
  });

  // ---- 播放 ----
  let skipMode = false;
  ws.on('timeupdate', (t) => {
    if (!skipMode || !ws.isPlaying()) return;
    const action = nextPlaybackAction(t, currentTrim());
    if (action.type === 'seek') ws.setTime(action.to);
    else if (action.type === 'stop') {
      ws.pause();
      skipMode = false;
    }
  });

  function previewFrom(t: number) {
    skipMode = true;
    ws.setTime(t);
    void ws.play();
  }

  function playRaw(t: number) {
    skipMode = false;
    ws.setTime(t);
    void ws.play();
  }

  function togglePlain() {
    if (ws.isPlaying()) {
      ws.pause();
      return;
    }
    skipMode = false;
    void ws.play();
  }

  const onKey = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement) return;
    if ((e.key === 'Delete' || e.key === 'Backspace') && selected) {
      e.preventDefault();
      removeCut(selected);
    } else if (e.key === ' ') {
      e.preventDefault();
      togglePlain();
    }
  };
  document.addEventListener('keydown', onKey);

  // ---- 選項、歌曲資訊、匯出 ----
  const fadeIn = h('input', { type: 'checkbox' });
  const fadeOut = h('input', { type: 'checkbox' });
  const titleInput = h('input', { type: 'text', value: p.video.tags.title });
  const artistInput = h('input', { type: 'text', value: p.video.tags.artist });
  const message = h('p', { class: 'error-text' });
  const exportBtn = h('button', { class: 'primary big' }, '匯出 MP3');
  const cancelBtn = h('button', { class: 'big', onclick: () => p.onCancel() }, '取消');

  function setBusy(busy: boolean) {
    exportBtn.disabled = busy;
    cancelBtn.disabled = busy;
    exportBtn.textContent = busy ? '匯出中…' : '匯出 MP3';
  }

  exportBtn.addEventListener('click', async () => {
    ws.pause();
    message.textContent = '';
    setBusy(true);
    try {
      await p.onExport({
        trim: currentTrim(),
        fadeIn: fadeIn.checked,
        fadeOut: fadeOut.checked,
        title: titleInput.value,
        artist: artistInput.value,
      });
    } catch (e) {
      message.textContent = (e as AppError).message ?? String(e);
      setBusy(false);
    }
  });

  const hasSuggestion = suggested.start > 0 || suggested.end < duration || suggested.cuts.length > 0;
  const swatch = (color: string) => h('span', { class: 'swatch', style: `background:${color}` });

  const el = h(
    'section',
    { class: 'view editor' },
    videoHeader(p.video, p.coverUrl),
    hasSuggestion
      ? h(
          'p',
          { class: 'notice' },
          '已依照 SponsorBlock 社群資料，自動標出 MV 裡不是音樂的部分，請試聽確認。',
          h('button', { class: 'link', onclick: useWholeTrack }, '不要的話改用整首'),
        )
      : null,
    waveEl,
    h(
      'p',
      { class: 'legend' },
      swatch(COLOR_EDGE),
      '灰色＝頭尾不要的部分（拖拉邊緣調整）',
      swatch(COLOR_CUT),
      '紅色＝中間要刪掉的部分（在波形上按住拖拉新增）',
    ),
    h(
      'div',
      { class: 'row' },
      h('button', { class: 'primary', onclick: () => previewFrom(currentTrim().start) }, '▶ 試聽成品'),
      h('button', { onclick: () => previewFrom(endPreviewStart(currentTrim())) }, '▶ 聽結尾'),
      h('button', { onclick: togglePlain }, '⏯ 播放／暫停（空白鍵）'),
      lengthLabel,
    ),
    h(
      'div',
      { class: 'panel' },
      h('h3', {}, '起點與終點'),
      timeRow('起點', startInput, setStart, () => currentTrim().start),
      timeRow('終點', endInput, setEnd, () => currentTrim().end),
    ),
    h('div', { class: 'panel' }, h('h3', {}, '刪除區段'), cutList),
    h(
      'div',
      { class: 'panel' },
      h('h3', {}, '淡入淡出'),
      h(
        'div',
        { class: 'row' },
        h('label', {}, fadeIn, ' 開頭淡入（2 秒）'),
        h('label', {}, fadeOut, ' 結尾淡出（2 秒）'),
      ),
    ),
    h(
      'div',
      { class: 'panel' },
      h('h3', {}, '歌曲資訊（會顯示在手機的音樂播放器）'),
      h('div', { class: 'tags' }, h('label', {}, '歌名'), titleInput, h('label', {}, '演出者'), artistInput),
    ),
    message,
    h('div', { class: 'actions' }, cancelBtn, exportBtn),
  );

  renderCutList();

  return {
    el,
    destroy() {
      document.removeEventListener('keydown', onKey);
      ws.destroy();
    },
  };
}
