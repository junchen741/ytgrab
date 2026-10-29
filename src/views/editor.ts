import WaveSurfer from 'wavesurfer.js';
import RegionsPlugin from 'wavesurfer.js/dist/plugins/regions.esm.js';
import type { AppError, ExportRequest, PreparedAudio, Range, TrimSpec, VideoInfo } from '../api/types';
import { endPreviewStart, nextPlaybackAction } from '../lib/playback';
import { formatTime, parseTime } from '../lib/time';
import { approxLength, draggedPastEdge, EDGE_MIN, exactEndFor, MIN_GAP, tailMinSeconds, trimFromRegions } from '../lib/trim';
import { h } from '../ui/dom';
import { videoHeader } from '../ui/videoHeader';

type Region = ReturnType<RegionsPlugin['addRegion']>;

// 波形畫在螢幕窗（深色玻璃）上：頭尾不要的部分蓋一層半透明玻璃色，看起來像沒亮；刪除段蓋半透明紅色。
const COLOR_EDGE = 'rgba(13, 27, 26, 0.72)';
// 紅色蓋在青綠色的波形上容易混成灰色：不透明度要到 0.64 以上，紅色才壓得過波形的青綠色。
// styles.css 另外替刪除段加紅色邊框
const COLOR_CUT = 'rgba(255, 123, 123, 0.7)';
const COLOR_CUT_SELECTED = 'rgba(255, 123, 123, 0.88)';
/** 拖拉出來比這更短的紅色區段視為誤觸，自動刪掉（秒）。 */
const MIN_CUT = 0.5;
/** 頭尾灰色區段的拉桿寬度（像素）與顏色。 */
const EDGE_HANDLE_PX = 14;
const COLOR_EDGE_HANDLE = '#B7C7C3';

/**
 * 頭尾灰色區段的拉桿外觀（`side` 是灰色區段朝向波形內部的那一側）。
 * wavesurfer 原本的把手只有 6px、畫在區段內側；改成較寬的拉桿，用不到的另一側把手隱藏。位置由 placeEdgeHandle 決定。
 */
function styleEdgeHandle(r: Region, side: 'left' | 'right', label: string) {
  const outer = side === 'left' ? 'right' : 'left';
  const hidden = r.element?.querySelector<HTMLElement>(`[part~="region-handle-${outer}"]`);
  if (hidden) hidden.style.display = 'none';
  const handle = r.element?.querySelector<HTMLElement>(`[part~="region-handle-${side}"]`);
  if (!handle) return;
  handle.style.width = `${EDGE_HANDLE_PX}px`;
  handle.style.setProperty(outer, 'auto');
  handle.style.background = COLOR_EDGE_HANDLE;
  handle.style.border = 'none';
  handle.style.borderRadius = '4px';
  handle.title = label;
}

/**
 * 拉桿一律貼在「要」與「不要」的交界、放在不要的那一側，不擋到要保留的波形：
 * - 有裁：拉桿在灰色區段裡、貼著內緣。
 * - 沒裁：拉桿在波形框外面、內側貼齊波形邊緣（需要 styles.css 讓波形容器不裁掉超出的部分）；
 *   這時灰色區段只剩幾個像素寬（見 tailMinSeconds），一併設成透明，看起來就是完全沒有灰色。
 */
function placeEdgeHandle(r: Region, side: 'left' | 'right', trimmed: boolean) {
  const handle = r.element?.querySelector<HTMLElement>(`[part~="region-handle-${side}"]`);
  if (!handle || !r.element) return;
  handle.style.setProperty(side, trimmed ? '0' : '100%');
  r.element.style.backgroundColor = trimmed ? COLOR_EDGE : 'transparent';
}

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
    height: 120,
    // 波形最高只畫到 85% 高度：音量很大的歌會整段頂到最大，貼著上下邊緣看起來像被切掉
    barHeight: 0.85,
    normalize: true,
    waveColor: '#5FD9BA',
    progressColor: '#B8FFEC',
    cursorColor: '#F0A531',
    cursorWidth: 2,
    plugins: [regions],
  });

  // 區段都在下方 ws.once('ready') 裡建立；ready 之前一律用建議值。
  let head!: Region;
  let tail!: Region;
  let ready = false;
  /** 終點灰色區段的最小寬度（秒），ready 時依波形寬度算出來，見 tailMinSeconds。 */
  let tailMin = EDGE_MIN;
  /** 終點落在結尾前 tailMin 秒以內時的精確終點（終點區段畫不了那麼窄），見 exactEndFor。拖動終點拉桿時清掉。 */
  let exactEnd: number | null = null;
  const cuts = new Map<string, Region>();
  let selected: Region | null = null;

  const currentTrim = (): TrimSpec =>
    ready
      ? trimFromRegions(
          head.end,
          tail.start,
          duration,
          [...cuts.values()].map((r) => ({ start: r.start, end: r.end })),
          tailMin,
          exactEnd,
        )
      : { start: suggested.start, end: suggested.end, cuts: suggested.cuts.map((c) => ({ ...c })) };

  // ---- 起點／終點 ----
  const startInput = h('input', { type: 'text', class: 'time' });
  const endInput = h('input', { type: 'text', class: 'time' });
  const lengthValue = h('span', { class: 'dig' });
  const lengthLabel = h('span', { class: 'length' }, '成品長度約 ', lengthValue);

  function refresh() {
    const t = currentTrim();
    startInput.value = formatTime(t.start);
    endInput.value = formatTime(t.end);
    lengthValue.textContent = formatTime(approxLength(t));
    if (ready) {
      placeEdgeHandle(head, 'right', t.start > 0);
      placeEdgeHandle(tail, 'left', t.end < duration);
    }
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
    exactEnd = exactEndFor(clamped, duration, tailMin);
    tail.setOptions({ start: Math.min(clamped, duration - tailMin), end: duration });
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

  // 起點／終點的輸入框在螢幕窗裡（見下方 el）；微調鍵分成兩組放在螢幕窗正下方，分別對齊起點和終點
  const nudge = (label: string, fn: () => void) => h('button', { class: 'nudge', onclick: fn }, label);
  const nudgeRow = (label: string, set: (v: number) => void, get: () => number) =>
    h(
      'div',
      { class: 'nudges', role: 'group', 'aria-label': `微調${label}` },
      h('span', { class: 'nudge-label' }, label),
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
        h('span', {}, `刪除區段 ${i + 1}：`, h('span', { class: 'cut-time' }, `${formatTime(r.start)} – ${formatTime(r.end)}`)),
        // 「取消刪除」是把這段留下來，不是危險的動作，所以和「聽聽看」一樣用一般的小按鍵
        h(
          'span',
          { class: 'cut-actions' },
          h('button', { class: 'nudge', onclick: () => playRaw(Math.max(0, r.start - 2)) }, '聽聽看'),
          h('button', { class: 'nudge', onclick: () => removeCut(r) }, '取消刪除'),
        ),
      ),
    );
    cutList.replaceChildren(
      ...(items.length > 0
        ? items
        : [h('li', { class: 'empty' }, '當前沒有刪除區段。')]),
    );
    refresh();
  }

  function registerCut(r: Region) {
    if (r === head || r === tail || cuts.has(r.id)) return;
    cuts.set(r.id, r);
    renderCutList();
  }

  /** 程式自己加的刪除區段（建議值、復原）；加的當下 region-created 會同步觸發，用 addingCut 讓它略過，不要被當成誤觸刪掉。 */
  let addingCut = false;
  function addCut(c: Range) {
    addingCut = true;
    try {
      registerCut(regions.addRegion({ start: c.start, end: c.end, color: COLOR_CUT }));
    } finally {
      addingCut = false;
    }
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

  /** 把頭尾與刪除區段整組換成 `t`。 */
  function applyTrim(t: TrimSpec) {
    if (!ready) return;
    head.setOptions({ start: 0, end: Math.max(t.start, EDGE_MIN) });
    exactEnd = exactEndFor(t.end, duration, tailMin);
    tail.setOptions({ start: Math.min(t.end, duration - tailMin), end: duration });
    for (const r of [...cuts.values()]) removeCut(r);
    for (const c of t.cuts) addCut(c);
    renderCutList();
  }

  // ---- SponsorBlock 提示：「改用整首」與「復原」可以來回切換 ----
  const notice = h('p', { class: 'notice' });
  /** 按「改用整首」之前的裁剪；null 代表目前不是「已改用整首」的狀態。 */
  let beforeWhole: TrimSpec | null = null;

  function renderNotice() {
    notice.replaceChildren(
      ...(beforeWhole
        ? ['已改用整首。', h('button', { class: 'link', onclick: undoWholeTrack }, '復原')]
        : [
            '已依照 SponsorBlock 社群資料，自動標出 MV 裡不是音樂的部分，請試聽確認。',
            h('button', { class: 'link', onclick: useWholeTrack }, '不要的話改用整首'),
          ]),
    );
  }

  function useWholeTrack() {
    if (!ready) return;
    beforeWhole = currentTrim();
    applyTrim({ start: 0, end: duration, cuts: [] });
    renderNotice();
  }

  function undoWholeTrack() {
    if (!ready || !beforeWhole) return;
    applyTrim(beforeWhole);
    beforeWhole = null;
    renderNotice();
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
    tailMin = tailMinSeconds(ws.getDuration(), ws.getWrapper().clientWidth);
    exactEnd = exactEndFor(suggested.end, duration, tailMin);
    tail = regions.addRegion({
      id: 'tail',
      start: Math.min(suggested.end, duration - tailMin),
      end: duration,
      color: COLOR_EDGE,
      drag: false,
      resizeEnd: false,
      minLength: tailMin,
    });
    styleEdgeHandle(head, 'right', '拖拉調整起點');
    styleEdgeHandle(tail, 'left', '拖拉調整終點');
    ready = true;
    for (const c of suggested.cuts) addCut(c);

    regions.enableDragSelection({ color: COLOR_CUT });
    // 拖拉的區段在放開滑鼠時才觸發 region-created（已對照原始碼確認），這時長度已是最終值
    regions.on('region-created', (r) => {
      if (addingCut || r === head || r === tail || cuts.has(r.id)) return;
      if (r.end - r.start < MIN_CUT) {
        r.remove();
        return;
      }
      registerCut(r);
    });
    regions.on('region-update', (r) => {
      // 使用者拖了終點拉桿：終點改由拉桿位置決定
      if (r === tail) exactEnd = null;
      refresh();
    });
    regions.on('region-updated', (r) => {
      // 拉桿拖過邊緣時 wavesurfer 會停在差一點的位置；用指標實際移動的距離判斷，拖過了就直接設成「不裁」
      if (pointerDown && (r === head || r === tail)) {
        const movedSec = ((lastPointerX - pointerDown.x) * ws.getDuration()) / Math.max(1, ws.getWrapper().clientWidth);
        if (r === head && draggedPastEdge('head', pointerDown.headEnd, movedSec, duration)) {
          head.setOptions({ start: 0, end: EDGE_MIN });
        }
        if (r === tail && draggedPastEdge('tail', pointerDown.tailStart, movedSec, duration, tailMin)) {
          tail.setOptions({ start: duration - tailMin, end: duration });
        }
      }
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

  // 記下按下滑鼠時的指標與頭尾位置，以及拖拉中指標最後的位置（給上面的 region-updated 判斷有沒有拖過邊緣）。
  // 用 capture 階段，wavesurfer 在 shadow DOM 裡處理拖拉也一樣收得到。
  let pointerDown: { x: number; headEnd: number; tailStart: number } | null = null;
  let lastPointerX = 0;
  const onPointerDown = (e: PointerEvent) => {
    lastPointerX = e.clientX;
    pointerDown = ready ? { x: e.clientX, headEnd: head.end, tailStart: tail.start } : null;
  };
  const onPointerMove = (e: PointerEvent) => {
    lastPointerX = e.clientX;
  };
  document.addEventListener('pointerdown', onPointerDown, true);
  document.addEventListener('pointermove', onPointerMove, true);

  // ---- 選項、歌曲資訊、匯出 ----
  const fadeIn = h('input', { type: 'checkbox', class: 'switch' });
  const fadeOut = h('input', { type: 'checkbox', class: 'switch' });
  const titleInput = h('input', { type: 'text', value: p.video.tags.title });
  const artistInput = h('input', { type: 'text', value: p.video.tags.artist });
  const message = h('p', { class: 'error-text' });
  const exportBtn = h('button', { class: 'primary big' }, '存成 MP3');
  const cancelBtn = h('button', { class: 'big', onclick: () => p.onCancel() }, '取消');

  function setBusy(busy: boolean) {
    exportBtn.disabled = busy;
    cancelBtn.disabled = busy;
    exportBtn.textContent = busy ? '存檔中…' : '存成 MP3';
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
  const el = h(
    'section',
    { class: 'view editor' },
    // 螢幕窗：影片資訊、波形、起點／成品長度／終點
    h(
      'div',
      { class: 'display' },
      videoHeader(p.video, p.coverUrl),
      waveEl,
      h(
        'div',
        { class: 'readout' },
        h('label', {}, '起點 ', startInput),
        lengthLabel,
        h('label', {}, '終點 ', endInput),
      ),
    ),
    h(
      'div',
      { class: 'nudge-bar' },
      nudgeRow('起點', setStart, () => currentTrim().start),
      nudgeRow('終點', setEnd, () => currentTrim().end),
    ),
    h(
      'p',
      { class: 'legend' },
      h('span', {}, h('span', { class: 'swatch edge' }), '暗掉的＝移除歌曲頭尾部分，拖拉兩邊的拉桿調整'),
      h('span', {}, h('span', { class: 'swatch cut' }), '紅色＝歌曲中間刪除區段，在波形上按住拖拉即可新增'),
    ),
    hasSuggestion ? notice : null,
    h(
      'div',
      { class: 'transport' },
      // 每個畫面只有一顆琥珀主鍵，這個畫面是「存成 MP3」
      h('button', { onclick: () => previewFrom(currentTrim().start) }, '▶ 試聽成品'),
      h('button', { onclick: () => previewFrom(endPreviewStart(currentTrim())) }, '▶ 聽結尾'),
      h('button', { onclick: togglePlain }, '⏯ 播放／暫停', h('span', { class: 'key-hint' }, '空白鍵')),
      h(
        'div',
        { class: 'fades', role: 'group', 'aria-label': '淡入淡出（2 秒）' },
        h('span', { class: 'fades-label' }, '淡入淡出（2 秒）'),
        h('label', { class: 'switch-row' }, fadeIn, '開頭'),
        h('label', { class: 'switch-row' }, fadeOut, '結尾'),
      ),
    ),
    // 機身下半部：左欄是刪除區段（每一列自己寫著「刪除區段 N」，不另外放標題），右欄是歌曲資訊
    h(
      'div',
      { class: 'controls' },
      h('div', {}, cutList),
      h(
        'div',
        {},
        h('h3', {}, '歌曲資訊'),
        h('div', { class: 'tags' }, h('label', {}, '歌名'), titleInput, h('label', {}, '演出者'), artistInput),
      ),
    ),
    message,
    h('div', { class: 'actions' }, cancelBtn, exportBtn),
  );

  renderNotice();
  renderCutList();

  return {
    el,
    destroy() {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown, true);
      document.removeEventListener('pointermove', onPointerMove, true);
      ws.destroy();
    },
  };
}
