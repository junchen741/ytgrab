//! 端到端測試：真的連 YouTube 下載、裁剪、匯出。需要網路，預設不跑：
//! cargo test --manifest-path src-tauri/Cargo.toml --test e2e -- --ignored --nocapture

use std::path::PathBuf;
use ytgrab_lib::media::{self, ExportJob};
use ytgrab_lib::segments::{keep_segments, Range, TrimSpec};
use ytgrab_lib::title::SongTags;
use ytgrab_lib::tools::{command, run, Tools};
use ytgrab_lib::ytdlp;

const TEST_URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

fn tools() -> Tools {
    Tools::from_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"))
}

#[tokio::test]
#[ignore = "需要網路與 src-tauri/binaries"]
async fn download_trim_and_export() {
    let tools = tools();
    let work = tempfile::tempdir().unwrap();
    let session = work.path().join("session");
    std::fs::create_dir_all(&session).unwrap();

    let info = ytdlp::fetch_info(&tools, TEST_URL, &session).await.expect("解析失敗");
    assert_eq!(info.id, "jNQXAC9IVRw");

    let downloaded = ytdlp::download_audio(&tools, &session, |p| println!("下載 {p:.0}%"))
        .await
        .expect("下載失敗");
    assert!(downloaded.audio.exists());

    let (peaks, duration) = media::compute_peaks(&tools, &downloaded.audio).await.expect("波形失敗");
    assert!(duration > 17.0 && duration < 21.0, "duration = {duration}");
    assert!((peaks.len() as f64 - duration * 50.0).abs() <= 1.0);

    let cover = match &downloaded.thumbnail {
        Some(t) => media::make_cover(&tools, t, &session.join("cover.jpg")).await,
        None => None,
    };
    assert!(cover.is_some(), "封面應該做得出來");

    let spec = TrimSpec { start: 1.0, end: 7.0, cuts: vec![Range { start: 3.0, end: 4.0 }] };
    let keep = keep_segments(&spec, duration).unwrap();
    assert_eq!(keep.len(), 2);

    let out_dir = work.path().join("out");
    let tags = SongTags { title: info.tags.title.clone(), artist: info.tags.artist.clone() };
    let job = ExportJob {
        audio: &downloaded.audio,
        cover: cover.as_deref(),
        keep: &keep,
        fade_in: true,
        fade_out: true,
        tags: &tags,
        output_dir: &out_dir,
    };
    let out = media::export_mp3(&tools, &job).await.expect("匯出失敗");
    assert_eq!(out.extension().unwrap(), "mp3");

    // 成品 = 2 秒 + 3 秒 - 0.03 秒交叉淡化；MP3 編碼會多出幾十毫秒補白
    let (_, out_duration) = media::compute_peaks(&tools, &out).await.unwrap();
    assert!((out_duration - 4.97).abs() < 0.2, "out_duration = {out_duration}");

    // 歌曲資訊有寫進去
    let mut cmd = command(&tools.ffmpeg);
    cmd.args(["-hide_banner", "-i"]).arg(&out).args(["-f", "ffmetadata", "-"]);
    let meta = run(cmd).await.unwrap().stdout;
    assert!(meta.lines().any(|l| l.starts_with("title=") && l.len() > 6), "{meta}");

    // 同名再匯出一次，不會覆蓋
    let out2 = media::export_mp3(&tools, &job).await.unwrap();
    assert_ne!(out, out2);
    assert!(out2.file_name().unwrap().to_string_lossy().contains("(2)"));
}
