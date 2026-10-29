//! 整個程式共用的狀態。

use crate::settings::Settings;
use crate::stop::StopSignal;
use crate::tools::Tools;
use crate::ytdlp::VideoInfo;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, MutexGuard};
use tokio::sync::RwLock;

/// 一次只處理一首歌，這裡記錄目前這首的暫存檔。
pub struct Session {
    pub dir: PathBuf,
    pub info: VideoInfo,
    pub audio: Option<PathBuf>,
    pub cover: Option<PathBuf>,
    pub duration: f64,
}

pub struct AppState {
    pub tools: Tools,
    pub resource_bin: PathBuf,
    pub cache_dir: PathBuf,
    pub settings_path: PathBuf,
    pub settings: Mutex<Settings>,
    /// yt-dlp 自我更新時拿寫入鎖，其他 yt-dlp 呼叫拿讀取鎖，避免更新到一半被執行。
    pub ytdlp_gate: RwLock<()>,
    pub ytdlp_version: Mutex<String>,
    /// 正在更新 yt-dlp；前端用來顯示「正在更新下載元件」。
    pub ytdlp_updating: AtomicBool,
    /// 這次開啟程式後是否已經「失敗 → 強制更新 → 重試」過，避免一直重試。
    pub retried_with_update: AtomicBool,
    pub session: Mutex<Option<Session>>,
    /// 首頁的「停止」鍵：結束正在進行的讀取影片資訊或下載。
    pub stop: StopSignal,
}

impl AppState {
    pub fn new(tools: Tools, resource_bin: PathBuf, cache_dir: PathBuf, settings_path: PathBuf, settings: Settings) -> Self {
        Self {
            tools,
            resource_bin,
            cache_dir,
            settings_path,
            settings: Mutex::new(settings),
            ytdlp_gate: RwLock::new(()),
            ytdlp_version: Mutex::new("檢查中".to_string()),
            ytdlp_updating: AtomicBool::new(false),
            retried_with_update: AtomicBool::new(false),
            session: Mutex::new(None),
            stop: StopSignal::new(),
        }
    }
}

/// 鎖被 panic 汙染時照樣拿來用；這些資料都很單純，不值得讓整個程式掛掉。
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
