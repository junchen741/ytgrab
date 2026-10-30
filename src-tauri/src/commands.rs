//! 前端可以呼叫的 Tauri commands。

use crate::errors::{is_retryable_after_update, AppError, ErrorKind};
use crate::logging;
use crate::media::{self, ExportJob};
use crate::peaks::PEAKS_PER_SECOND;
use crate::report::{build_report, ReportInput};
use crate::segments::{keep_segments, TrimSpec};
use crate::settings;
use crate::sponsorblock;
use crate::state::{lock, AppState, Session};
use crate::title::SongTags;
use crate::video_url::{canonical_url, extract_video_id};
use crate::ytdlp::{self, VideoInfo};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub ytdlp_version: String,
    pub ytdlp_updating: bool,
    pub output_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAudio {
    pub audio_path: String,
    pub duration: f64,
    pub peaks: Vec<f32>,
    pub peaks_per_second: u32,
    pub cover_path: Option<String>,
    pub suggested: TrimSpec,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub trim: TrimSpec,
    pub fade_in: bool,
    pub fade_out: bool,
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Serialize)]
struct ProgressEvent {
    stage: &'static str,
    percent: Option<f64>,
}

fn log_err(e: &AppError) {
    logging::error(format!("{:?}：{}", e.kind, e.detail));
}

fn no_session() -> AppError {
    AppError::new(ErrorKind::Unknown, "沒有進行中的影片")
}

fn stopped() -> AppError {
    AppError::new(ErrorKind::Cancelled, "使用者按了停止")
}

/// 「正在更新 yt-dlp」結束時（包括更新到一半被「停止」丟掉）把狀態改回來並通知前端，
/// 否則前端下次會一直顯示「正在更新下載元件」。
struct UpdatingDone<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
}

impl Drop for UpdatingDone<'_> {
    fn drop(&mut self) {
        self.state.ytdlp_updating.store(false, Ordering::SeqCst);
        let _ = self.app.emit("ytdlp-update", false);
    }
}

/// 清掉上一首的暫存檔，開一個新的暫存資料夾。
fn new_session_dir(cache_dir: &Path) -> std::io::Result<PathBuf> {
    let root = cache_dir.join("sessions");
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// yt-dlp 自我更新的時間上限；網路慢時，不讓使用者無限期等下去。
const UPDATE_TIMEOUT: Duration = Duration::from_secs(60);

/// 更新 yt-dlp；更新把檔案弄壞時，用安裝時附的版本還原。
/// 開始與結束時送出 `ytdlp-update` 事件（true/false），前端據此顯示「正在更新下載元件」。
pub async fn update_ytdlp(app: &AppHandle, state: &AppState) {
    let _gate = state.ytdlp_gate.write().await;
    state.ytdlp_updating.store(true, Ordering::SeqCst);
    let _ = app.emit("ytdlp-update", true);
    // 比 _gate 晚宣告、先被 drop：先通知更新結束，再放開寫入鎖（和原本的順序一樣）
    let _done = UpdatingDone { app, state };

    // 逾時會丟棄 future，子程序因 kill_on_drop 被結束；yt-dlp 是先下載成 .new 再換檔，原本的 exe 不受影響
    let result = match tokio::time::timeout(UPDATE_TIMEOUT, ytdlp::self_update(&state.tools)).await {
        Ok(r) => r,
        Err(_) => Err(format!("超過 {} 秒仍未完成，這次先略過", UPDATE_TIMEOUT.as_secs())),
    };
    match result {
        Ok(v) => {
            logging::info(format!("yt-dlp 版本：{v}"));
            *lock(&state.ytdlp_version) = v;
        }
        Err(e) => {
            logging::error(format!("yt-dlp 更新失敗：{e}"));
            if ytdlp::version(&state.tools).await.is_err() {
                if let Err(e) = state.tools.restore_ytdlp(&state.resource_bin) {
                    logging::error(format!("還原 yt-dlp 失敗：{e}"));
                }
            }
            // 錯誤報告會附上這個值；還原後仍無法執行就明確寫出來，方便判斷是不是被防毒隔離
            *lock(&state.ytdlp_version) = match ytdlp::version(&state.tools).await {
                Ok(v) => v,
                Err(_) => "無法執行（可能被防毒軟體隔離）".to_string(),
            };
        }
    }
    // 「正在更新」由 _done 在這裡 drop 時改回來
}

/// 兩種失敗值得自動補救一次再重試：
/// - 找不到 yt-dlp（多半是被防毒刪掉）→ 用安裝資料夾裡的副本還原
/// - 看起來是 yt-dlp 太舊 → 更新（每次開啟程式只做一次）
async fn with_update_retry<T, F, Fut>(app: &AppHandle, state: &AppState, op: F) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let first = {
        let _gate = state.ytdlp_gate.read().await;
        op().await
    };
    match first {
        Err(e) if e.kind == ErrorKind::ToolMissing => {
            logging::error(format!("找不到 yt-dlp，嘗試從安裝資料夾還原：{}", e.detail));
            {
                let _gate = state.ytdlp_gate.write().await;
                if let Err(err) = state.tools.restore_ytdlp(&state.resource_bin) {
                    logging::error(format!("還原 yt-dlp 失敗：{err}"));
                    return Err(e);
                }
            }
            let _gate = state.ytdlp_gate.read().await;
            op().await
        }
        Err(e) if is_retryable_after_update(e.kind) && !state.retried_with_update.swap(true, Ordering::SeqCst) => {
            logging::info(format!("失敗（{:?}），先更新 yt-dlp 再重試", e.kind));
            update_ytdlp(app, state).await;
            let _gate = state.ytdlp_gate.read().await;
            op().await
        }
        other => other,
    }
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        ytdlp_version: lock(&state.ytdlp_version).clone(),
        ytdlp_updating: state.ytdlp_updating.load(Ordering::SeqCst),
        output_dir: lock(&state.settings).output_dir.display().to_string(),
    }
}

#[tauri::command]
pub async fn fetch_info(app: AppHandle, url: String, state: State<'_, AppState>) -> Result<VideoInfo, AppError> {
    let id = extract_video_id(&url).ok_or_else(|| AppError::new(ErrorKind::InvalidUrl, url.clone()))?;
    let canonical = canonical_url(&id);
    let dir = new_session_dir(&state.cache_dir)
        .map_err(|e| AppError::new(ErrorKind::Unknown, format!("無法建立暫存資料夾：{e}")))?;
    logging::info(format!("解析 {canonical}"));

    let info = state
        .stop
        .run(with_update_retry(&app, state.inner(), || ytdlp::fetch_info(&state.tools, &canonical, &dir)))
        .await
        .ok_or_else(stopped)?
        .inspect_err(log_err)?;
    *lock(&state.session) = Some(Session { dir, info: info.clone(), audio: None, cover: None, duration: 0.0 });
    Ok(info)
}

#[tauri::command]
pub async fn prepare_audio(app: AppHandle, state: State<'_, AppState>) -> Result<PreparedAudio, AppError> {
    let (dir, video_id) = {
        let guard = lock(&state.session);
        let s = guard.as_ref().ok_or_else(no_session)?;
        (s.dir.clone(), s.info.id.clone())
    };
    let emit = |stage: &'static str, percent: Option<f64>| {
        let _ = app.emit("prepare-progress", ProgressEvent { stage, percent });
    };

    // 下載和分析波形都可以按「停止」中途放棄
    let work = async {
        emit("download", Some(0.0));
        let downloaded = with_update_retry(&app, state.inner(), || {
            ytdlp::download_audio(&state.tools, &dir, |p| emit("download", Some(p)))
        })
        .await?;

        emit("analyze", None);
        let cover_target = dir.join("cover.jpg");
        let (peaks_result, non_music, cover) = tokio::join!(
            media::compute_peaks(&state.tools, &downloaded.audio),
            sponsorblock::fetch_non_music(&video_id),
            async {
                match &downloaded.thumbnail {
                    Some(t) => media::make_cover(&state.tools, t, &cover_target).await,
                    None => None,
                }
            },
        );
        let (peaks, duration) = peaks_result?;
        Ok::<_, AppError>((downloaded, peaks, duration, non_music, cover))
    };
    let (downloaded, peaks, duration, non_music, cover) =
        state.stop.run(work).await.ok_or_else(stopped)?.inspect_err(log_err)?;
    let suggested = sponsorblock::suggest_trim(&non_music, duration);
    logging::info(format!("音訊 {duration:.1} 秒，SponsorBlock 非音樂段落 {} 段", non_music.len()));

    if let Some(s) = lock(&state.session).as_mut() {
        s.audio = Some(downloaded.audio.clone());
        s.cover = cover.clone();
        s.duration = duration;
    }
    Ok(PreparedAudio {
        audio_path: downloaded.audio.display().to_string(),
        duration,
        peaks,
        peaks_per_second: PEAKS_PER_SECOND,
        cover_path: cover.map(|c| c.display().to_string()),
        suggested,
    })
}

/// 首頁的「停止」鍵：結束正在進行的讀取影片資訊或下載（見 StopSignal）。
#[tauri::command]
pub fn stop_work(state: State<'_, AppState>) {
    logging::info("使用者按了停止");
    state.stop.stop();
}

#[tauri::command]
pub async fn export_mp3(req: ExportRequest, state: State<'_, AppState>) -> Result<String, AppError> {
    let (audio, cover, duration) = {
        let guard = lock(&state.session);
        let s = guard.as_ref().ok_or_else(no_session)?;
        (s.audio.clone().ok_or_else(no_session)?, s.cover.clone(), s.duration)
    };
    let keep = keep_segments(&req.trim, duration).map_err(AppError::from)?;
    let output_dir = lock(&state.settings).output_dir.clone();
    let tags = SongTags { title: req.title.trim().to_string(), artist: req.artist.trim().to_string() };
    let job = ExportJob {
        audio: &audio,
        cover: cover.as_deref(),
        keep: &keep,
        fade_in: req.fade_in,
        fade_out: req.fade_out,
        tags: &tags,
        output_dir: &output_dir,
    };
    let out = media::export_mp3(&state.tools, &job).await.inspect_err(log_err)?;
    logging::info(format!("已匯出 {}", out.display()));
    Ok(out.display().to_string())
}

#[tauri::command]
pub fn set_output_dir(dir: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut s = lock(&state.settings);
    s.output_dir = PathBuf::from(dir);
    settings::save(&state.settings_path, &s).map_err(|e| AppError::new(ErrorKind::Unknown, format!("無法儲存設定：{e}")))
}

/// 產生錯誤報告並直接放進剪貼簿（在 Rust 端寫剪貼簿，不用另外開前端權限）。
#[tauri::command]
pub fn copy_error_report(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    stage: String,
    kind: String,
    detail: String,
) -> Result<(), AppError> {
    let ytdlp_version = lock(&state.ytdlp_version).clone();
    let home = app.path().home_dir().ok().map(|p| p.display().to_string());
    let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let os = os_info::get().to_string();
    let version = app.package_info().version.to_string();
    let text = build_report(&ReportInput {
        time: &time,
        app_version: &version,
        ytdlp_version: &ytdlp_version,
        os: &os,
        url: &url,
        stage: &stage,
        kind: &kind,
        detail: &detail,
        home_dir: home.as_deref(),
    });
    app.clipboard().write_text(text).map_err(|e| AppError::new(ErrorKind::Unknown, e.to_string()))
}

/// 用 Windows 預設程式開啟記錄資料夾或授權聲明。
#[tauri::command]
pub fn open_special(app: AppHandle, target: String) -> Result<(), AppError> {
    let path = match target.as_str() {
        "logs" => logging::dir(),
        "notices" => app.path().resource_dir().ok().map(|d| d.join("THIRD_PARTY_NOTICES.txt")),
        _ => None,
    }
    .ok_or_else(|| AppError::new(ErrorKind::Unknown, format!("未知的開啟目標：{target}")))?;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| AppError::new(ErrorKind::Unknown, e.to_string()))
}

/// 剪貼簿裡是 YouTube 影片網址才回傳，讓首頁自動帶入；其他內容一律不讀出來。
#[tauri::command]
pub fn clipboard_youtube_url(app: AppHandle) -> Option<String> {
    let text = app.clipboard().read_text().ok()?;
    let text = text.trim();
    extract_video_id(text).map(|_| text.to_string())
}
