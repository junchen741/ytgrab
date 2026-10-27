//! 執行 ffmpeg：計算波形、製作封面、匯出 MP3。

use crate::errors::{AppError, ErrorKind};
use crate::ffmpeg_args::{cover_args, export_args, peaks_args, ExportPlan};
use crate::filename::{song_stem, unique_path};
use crate::logging;
use crate::peaks::{samples_per_peak, PeakAccumulator};
use crate::segments::Range;
use crate::title::SongTags;
use crate::tools::{command, read_to_string_lossy, run, Tools};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

pub struct ExportJob<'a> {
    pub audio: &'a Path,
    pub cover: Option<&'a Path>,
    pub keep: &'a [Range],
    pub fade_in: bool,
    pub fade_out: bool,
    pub tags: &'a SongTags,
    pub output_dir: &'a Path,
}

fn ffmpeg_error(detail: impl Into<String>) -> AppError {
    AppError::new(ErrorKind::Export, detail)
}

/// 找不到或不能執行 ffmpeg.exe，多半是被防毒軟體隔離。
fn spawn_failed(e: std::io::Error) -> AppError {
    let kind = match e.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => ErrorKind::ToolMissing,
        _ => ErrorKind::Export,
    };
    AppError::new(kind, format!("無法啟動 ffmpeg：{e}"))
}

fn save_failed(detail: impl Into<String>) -> AppError {
    AppError::new(ErrorKind::SaveFailed, detail)
}

/// 邊解碼邊算峰值；回傳（峰值, 實際長度秒數）。
pub async fn compute_peaks(tools: &Tools, audio: &Path) -> Result<(Vec<f32>, f64), AppError> {
    let mut cmd = command(&tools.ffmpeg);
    cmd.args(peaks_args(audio));
    let mut child = cmd.spawn().map_err(spawn_failed)?;
    let mut stdout = child.stdout.take().expect("stdout 已設為 piped");
    let stderr_task = tokio::spawn(read_to_string_lossy(child.stderr.take().expect("stderr 已設為 piped")));

    let mut acc = PeakAccumulator::new(samples_per_peak());
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = stdout
            .read(&mut buf)
            .await
            .map_err(|e| ffmpeg_error(format!("讀取 ffmpeg 輸出失敗：{e}")))?;
        if n == 0 {
            break;
        }
        acc.push_bytes(&buf[..n]);
    }
    let status = child.wait().await.map_err(|e| ffmpeg_error(e.to_string()))?;
    if !status.success() {
        return Err(ffmpeg_error(stderr_task.await.unwrap_or_default()));
    }
    Ok(acc.finish())
}

/// 封面做不出來不算錯誤，只是 MP3 沒有封面。
pub async fn make_cover(tools: &Tools, thumbnail: &Path, out: &Path) -> Option<PathBuf> {
    let mut cmd = command(&tools.ffmpeg);
    cmd.args(cover_args(thumbnail, out));
    match run(cmd).await {
        Ok(c) if c.success && out.exists() => Some(out.to_path_buf()),
        Ok(c) => {
            logging::error(format!("封面轉換失敗：{}", c.stderr));
            None
        }
        Err(e) => {
            logging::error(format!("無法啟動 ffmpeg 製作封面：{e}"));
            None
        }
    }
}

/// 先寫到暫存檔，成功後才改名成正式檔名，避免留下壞掉的半成品。
pub async fn export_mp3(tools: &Tools, job: &ExportJob<'_>) -> Result<PathBuf, AppError> {
    tokio::fs::create_dir_all(job.output_dir)
        .await
        .map_err(|e| save_failed(format!("無法建立資料夾 {}：{e}", job.output_dir.display())))?;
    let stem = song_stem(&job.tags.artist, &job.tags.title);
    let final_path = unique_path(job.output_dir, &stem, "mp3");
    let tmp = job.output_dir.join(format!(".{stem}.ytgrab-tmp"));

    let plan = ExportPlan {
        input: job.audio,
        cover: job.cover,
        keep: job.keep,
        fade_in: job.fade_in,
        fade_out: job.fade_out,
        title: &job.tags.title,
        artist: &job.tags.artist,
        output: &tmp,
    };
    let mut cmd = command(&tools.ffmpeg);
    cmd.args(export_args(&plan));
    let out = run(cmd).await.map_err(spawn_failed)?;
    if !out.success {
        let _ = tokio::fs::remove_file(&tmp).await;
        // 「受控資料夾存取」等安全性設定擋住寫入時，ffmpeg 會回報 Permission denied
        if out.stderr.contains("Permission denied") {
            return Err(save_failed(out.stderr));
        }
        return Err(ffmpeg_error(out.stderr));
    }
    tokio::fs::rename(&tmp, &final_path)
        .await
        .map_err(|e| save_failed(format!("無法存成 {}：{e}", final_path.display())))?;
    Ok(final_path)
}
