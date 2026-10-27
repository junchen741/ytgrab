//! 呼叫 yt-dlp：讀取影片資訊、下載音訊與縮圖、自我更新。

use crate::errors::{classify_ytdlp, AppError, ErrorKind};
use crate::title::{guess_tags, SongTags};
use crate::tools::{command, read_to_string_lossy, run, Tools};
use crate::video_url::canonical_url;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};

/// `-J` 的輸出存成這個檔名；下載時用 `--load-info-json` 讀回來，省掉第二次解析。
pub const INFO_FILE: &str = "info.json";
const PROGRESS_PREFIX: &str = "YTG|";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub live: bool,
    pub tags: SongTags,
}

pub struct Downloaded {
    pub audio: PathBuf,
    pub thumbnail: Option<PathBuf>,
}

pub fn parse_info(json: &str) -> Result<VideoInfo, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("無法解析 yt-dlp 輸出：{e}"))?;
    let text = |key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);
    let id = text("id").ok_or("yt-dlp 輸出缺少影片 ID")?;
    let title = text("title").unwrap_or_default();
    let channel = text("channel").or_else(|| text("uploader")).unwrap_or_default();
    let artist = v
        .get("artists")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| text("artist"));
    let live = matches!(text("live_status").as_deref(), Some("is_live" | "is_upcoming"));
    let tags = guess_tags(&title, &channel, text("track").as_deref(), artist.as_deref());
    Ok(VideoInfo {
        url: canonical_url(&id),
        id,
        title,
        channel,
        duration: v.get("duration").and_then(Value::as_f64),
        thumbnail: text("thumbnail"),
        live,
        tags,
    })
}

/// 解析 `--progress-template` 輸出的「YTG|已下載|總大小」，回傳百分比。
pub fn parse_progress_line(line: &str) -> Option<f64> {
    let rest = line.trim().strip_prefix(PROGRESS_PREFIX)?;
    let (done, total) = rest.split_once('|')?;
    let done: f64 = done.trim().parse().ok()?;
    let total: f64 = total.trim().parse().ok()?;
    (total > 0.0).then(|| (done / total * 100.0).clamp(0.0, 100.0))
}

/// 改用 Windows 的憑證存放區，不用 yt-dlp 內建的 certifi。
/// 防毒軟體（例如 Norton）的 HTTPS 檢查會換上自己的根憑證，只有 Windows 憑證存放區信任它，
/// 用 certifi 的話會出現 CERTIFICATE_VERIFY_FAILED，整個抓不到影片。
const SYSTEM_CERTS: [&str; 2] = ["--compat-options", "no-certifi"];

fn base_args(tools: &Tools) -> Vec<String> {
    vec![
        SYSTEM_CERTS[0].into(),
        SYSTEM_CERTS[1].into(),
        "--ignore-config".into(),
        "--no-playlist".into(),
        "--js-runtimes".into(),
        format!("deno:{}", tools.deno.display()),
        "--ffmpeg-location".into(),
        tools.ffmpeg.display().to_string(),
    ]
}

/// 找不到或不能執行 yt-dlp.exe，多半是被防毒軟體隔離；commands 會先嘗試還原再重試。
fn spawn_error(e: &std::io::Error) -> AppError {
    let kind = match e.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => ErrorKind::ToolMissing,
        _ => ErrorKind::Unknown,
    };
    AppError::new(kind, format!("無法啟動 yt-dlp：{e}"))
}

/// 讀取影片資訊，並把原始 JSON 存進暫存資料夾給下載用。
pub async fn fetch_info(tools: &Tools, url: &str, session_dir: &Path) -> Result<VideoInfo, AppError> {
    let mut cmd = command(&tools.ytdlp);
    cmd.args(base_args(tools)).arg("-J").arg(url);
    let out = run(cmd).await.map_err(|e| spawn_error(&e))?;
    if !out.success {
        return Err(AppError::new(classify_ytdlp(&out.stderr), out.stderr));
    }
    tokio::fs::write(session_dir.join(INFO_FILE), &out.stdout)
        .await
        .map_err(|e| AppError::new(ErrorKind::Unknown, format!("無法寫入暫存檔：{e}")))?;
    let info = parse_info(&out.stdout).map_err(|e| AppError::new(ErrorKind::Unknown, e))?;
    if info.live {
        return Err(AppError::new(ErrorKind::Live, "live_status 為 is_live 或 is_upcoming"));
    }
    Ok(info)
}

/// 下載最佳音質的純音訊串流（不轉檔），順便抓縮圖。
pub async fn download_audio(tools: &Tools, session_dir: &Path, on_progress: impl Fn(f64)) -> Result<Downloaded, AppError> {
    let mut cmd = command(&tools.ytdlp);
    cmd.args(base_args(tools))
        .arg("--load-info-json")
        .arg(session_dir.join(INFO_FILE))
        .args(["-f", "bestaudio", "--newline", "--progress-template"])
        .arg(format!(
            "download:{PROGRESS_PREFIX}%(progress.downloaded_bytes)s|%(progress.total_bytes,progress.total_bytes_estimate)s"
        ))
        .arg("-o")
        .arg(session_dir.join("audio.%(ext)s"))
        .arg("--write-thumbnail")
        .arg("-o")
        .arg(format!("thumbnail:{}", session_dir.join("thumb.%(ext)s").display()));

    let mut child = cmd.spawn().map_err(|e| spawn_error(&e))?;
    let stdout = child.stdout.take().expect("stdout 已設為 piped");
    let stderr = child.stderr.take().expect("stderr 已設為 piped");
    let stderr_task = tokio::spawn(read_to_string_lossy(stderr));

    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if let Some(p) = parse_progress_line(&String::from_utf8_lossy(&line)) {
                    on_progress(p);
                }
            }
        }
    }

    let status = child.wait().await.map_err(|e| spawn_error(&e))?;
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        return Err(AppError::new(classify_ytdlp(&stderr), stderr));
    }
    let audio = find_file(session_dir, "audio.")
        .ok_or_else(|| AppError::new(ErrorKind::Unknown, "下載完成，但找不到音訊檔"))?;
    Ok(Downloaded { audio, thumbnail: find_file(session_dir, "thumb.") })
}

fn find_file(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.starts_with(prefix) && !name.ends_with(".part") && !name.ends_with(".ytdl")
    })
}

pub async fn version(tools: &Tools) -> Result<String, String> {
    let mut cmd = command(&tools.ytdlp);
    cmd.arg("--version");
    let out = run(cmd).await.map_err(|e| e.to_string())?;
    if out.success { Ok(out.stdout.trim().to_string()) } else { Err(out.stderr) }
}

/// `yt-dlp -U` 會就地更新 exe（先改名舊檔再換上新檔），所以 yt-dlp.exe 必須放在可寫入的資料夾。
pub async fn self_update(tools: &Tools) -> Result<String, String> {
    let mut cmd = command(&tools.ytdlp);
    cmd.args(update_args());
    let out = run(cmd).await.map_err(|e| e.to_string())?;
    if !out.success {
        return Err(format!("{}\n{}", out.stdout, out.stderr));
    }
    version(tools).await
}

fn update_args() -> Vec<String> {
    vec![SYSTEM_CERTS[0].into(), SYSTEM_CERTS[1].into(), "-U".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_windows_certificate_store() {
        // 防毒軟體（例如 Norton）的 HTTPS 檢查會換上自己的根憑證，只有 Windows 憑證存放區信任它；
        // yt-dlp 內建的 certifi 不認得，會出現 CERTIFICATE_VERIFY_FAILED
        let tools = Tools::from_dir(Path::new("bin"));
        for args in [base_args(&tools), update_args()] {
            let pos = args.iter().position(|a| a == "--compat-options").expect("缺少 --compat-options");
            assert_eq!(args[pos + 1], "no-certifi", "{args:?}");
        }
    }

    #[test]
    fn parse_info_guesses_tags_from_title() {
        let json = r#"{"id":"BaW_jenozKc","title":"Ava Lin - Morning Light (Official Music Video)","channel":"Ava Lin","duration":263,"thumbnail":"https://i.ytimg.com/vi/x/maxresdefault.jpg","live_status":"not_live"}"#;
        let info = parse_info(json).unwrap();
        assert_eq!(info.url, "https://www.youtube.com/watch?v=BaW_jenozKc");
        assert_eq!(info.duration, Some(263.0));
        assert!(!info.live);
        assert_eq!(info.tags, SongTags { title: "Morning Light".into(), artist: "Ava Lin".into() });
    }

    #[test]
    fn parse_info_prefers_music_metadata() {
        let json = r#"{"id":"BaW_jenozKc","title":"Morning Light","channel":"Ava Lin - Topic","track":"Morning Light","artists":["Ava Lin","Someone"],"live_status":"not_live"}"#;
        let info = parse_info(json).unwrap();
        assert_eq!(info.tags, SongTags { title: "Morning Light".into(), artist: "Ava Lin".into() });
    }

    #[test]
    fn parse_info_detects_live_and_missing_id() {
        let live = parse_info(r#"{"id":"BaW_jenozKc","title":"x","live_status":"is_live"}"#).unwrap();
        assert!(live.live);
        assert!(parse_info(r#"{"title":"x"}"#).is_err());
        assert!(parse_info("not json").is_err());
    }

    #[test]
    fn progress_line_parsing() {
        assert_eq!(parse_progress_line("YTG|512|1024"), Some(50.0));
        assert_eq!(parse_progress_line("YTG|2048|1024\r\n"), Some(100.0));
        assert_eq!(parse_progress_line("YTG|NA|1024"), None);
        assert_eq!(parse_progress_line("YTG|100|NA"), None);
        assert_eq!(parse_progress_line("[download]  50.0% of 3.2MiB"), None);
    }
}
