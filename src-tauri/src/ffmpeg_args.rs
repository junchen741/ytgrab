//! 組出 ffmpeg 的命令列參數（只組參數，不執行）。

use crate::peaks::SAMPLE_RATE;
use crate::segments::Range;
use std::path::Path;

/// 每個接縫的交叉淡化秒數，短到聽不出來，只為了消除「啪」聲。
pub const CROSSFADE_SECS: f64 = 0.03;
/// 勾選淡入/淡出時的長度。
pub const FADE_SECS: f64 = 2.0;
pub const MP3_BITRATE: &str = "192k";

pub struct ExportPlan<'a> {
    pub input: &'a Path,
    pub cover: Option<&'a Path>,
    pub keep: &'a [Range],
    pub fade_in: bool,
    pub fade_out: bool,
    pub title: &'a str,
    pub artist: &'a str,
    pub output: &'a Path,
}

pub fn output_duration(keep: &[Range]) -> f64 {
    let sum: f64 = keep.iter().map(|r| r.end - r.start).sum();
    sum - CROSSFADE_SECS * keep.len().saturating_sub(1) as f64
}

pub fn build_filter(keep: &[Range], fade_in: bool, fade_out: bool) -> String {
    debug_assert!(!keep.is_empty(), "keep_segments 保證至少有一段");
    let n = keep.len();
    let mut parts = Vec::new();

    let sources: Vec<String> = if n == 1 {
        vec!["0:a".to_string()]
    } else {
        let labels: Vec<String> = (0..n).map(|i| format!("i{i}")).collect();
        let outs: String = labels.iter().map(|l| format!("[{l}]")).collect();
        parts.push(format!("[0:a]asplit={n}{outs}"));
        labels
    };
    for (i, (r, src)) in keep.iter().zip(&sources).enumerate() {
        parts.push(format!("[{src}]atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS[s{i}]", r.start, r.end));
    }

    let mut last = "s0".to_string();
    for i in 1..n {
        parts.push(format!("[{last}][s{i}]acrossfade=d={CROSSFADE_SECS}:c1=tri:c2=tri[x{i}]"));
        last = format!("x{i}");
    }

    let total = output_duration(keep);
    let fade = FADE_SECS.min(total / 2.0);
    let mut chain = Vec::new();
    if fade_in {
        chain.push(format!("afade=t=in:st=0:d={fade:.3}"));
    }
    if fade_out {
        chain.push(format!("afade=t=out:st={:.3}:d={fade:.3}", total - fade));
    }
    if chain.is_empty() {
        chain.push("anull".to_string());
    }
    parts.push(format!("[{last}]{}[out]", chain.join(",")));
    parts.join(";")
}

pub fn export_args(plan: &ExportPlan) -> Vec<String> {
    let mut a: Vec<String> = vec!["-hide_banner".into(), "-nostdin".into(), "-y".into(), "-i".into(), path(plan.input)];
    if let Some(cover) = plan.cover {
        a.extend(["-i".into(), path(cover)]);
    }
    a.extend([
        "-filter_complex".into(),
        build_filter(plan.keep, plan.fade_in, plan.fade_out),
        "-map".into(),
        "[out]".into(),
    ]);
    if plan.cover.is_some() {
        a.extend(
            ["-map", "1:v", "-c:v", "copy", "-disposition:v", "attached_pic", "-metadata:s:v", "title=Album cover", "-metadata:s:v", "comment=Cover (front)"]
                .map(String::from),
        );
    }
    a.extend(["-c:a", "libmp3lame", "-b:a", MP3_BITRATE, "-ar", "44100", "-id3v2_version", "3"].map(String::from));
    a.extend([
        "-metadata".into(),
        format!("title={}", plan.title),
        "-metadata".into(),
        format!("artist={}", plan.artist),
        "-f".into(),
        "mp3".into(),
        path(plan.output),
    ]);
    a
}

/// 解碼成 8 kHz 單聲道 s16le 從 stdout 輸出，給 PeakAccumulator 用。
pub fn peaks_args(input: &Path) -> Vec<String> {
    let rate = SAMPLE_RATE.to_string();
    ["-hide_banner", "-nostdin", "-i"]
        .map(String::from)
        .into_iter()
        .chain([path(input)])
        .chain(["-vn", "-ac", "1", "-ar", rate.as_str(), "-f", "s16le", "pipe:1"].map(String::from))
        .collect()
}

/// 把 16:9 縮圖裁成正中央的正方形，縮成 600x600 的 JPEG。
pub fn cover_args(thumbnail: &Path, out: &Path) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        path(thumbnail),
        "-vf".into(),
        "crop='min(iw,ih)':'min(iw,ih)',scale=600:600".into(),
        "-frames:v".into(),
        "1".into(),
        "-q:v".into(),
        "3".into(),
        path(out),
    ]
}

fn path(p: &Path) -> String {
    p.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn r(start: f64, end: f64) -> Range {
        Range { start, end }
    }

    #[test]
    fn single_segment_without_fades() {
        assert_eq!(
            build_filter(&[r(1.0, 5.0)], false, false),
            "[0:a]atrim=start=1.000:end=5.000,asetpts=PTS-STARTPTS[s0];[s0]anull[out]"
        );
    }

    #[test]
    fn two_segments_with_crossfade_and_fades() {
        assert_eq!(
            build_filter(&[r(0.0, 10.0), r(20.0, 30.0)], true, true),
            "[0:a]asplit=2[i0][i1];\
             [i0]atrim=start=0.000:end=10.000,asetpts=PTS-STARTPTS[s0];\
             [i1]atrim=start=20.000:end=30.000,asetpts=PTS-STARTPTS[s1];\
             [s0][s1]acrossfade=d=0.03:c1=tri:c2=tri[x1];\
             [x1]afade=t=in:st=0:d=2.000,afade=t=out:st=17.970:d=2.000[out]"
        );
    }

    #[test]
    fn fade_shrinks_for_short_output() {
        let f = build_filter(&[r(0.0, 3.0)], false, true);
        assert!(f.ends_with("afade=t=out:st=1.500:d=1.500[out]"), "{f}");
    }

    #[test]
    fn output_duration_subtracts_crossfades() {
        let d = output_duration(&[r(0.0, 10.0), r(20.0, 30.0), r(40.0, 50.0)]);
        assert!((d - (30.0 - 2.0 * CROSSFADE_SECS)).abs() < 1e-9);
    }

    #[test]
    fn export_args_with_cover_and_tags() {
        let keep = [r(0.0, 10.0)];
        let plan = ExportPlan {
            input: Path::new(r"C:\tmp\audio.webm"),
            cover: Some(Path::new(r"C:\tmp\cover.jpg")),
            keep: &keep,
            fade_in: false,
            fade_out: false,
            title: "夜明けの歌",
            artist: "ミライ",
            output: Path::new(r"C:\out\x.tmp"),
        };
        let args = export_args(&plan);
        let joined = args.join(" ");
        assert!(joined.contains(r"-i C:\tmp\audio.webm -i C:\tmp\cover.jpg"));
        assert!(joined.contains("-map [out] -map 1:v -c:v copy -disposition:v attached_pic"));
        assert!(joined.contains("-c:a libmp3lame -b:a 192k"));
        assert!(args.contains(&"title=夜明けの歌".to_string()));
        assert!(args.contains(&"artist=ミライ".to_string()));
        assert_eq!(args.last().unwrap(), r"C:\out\x.tmp");
        assert_eq!(args[args.len() - 2], "mp3");
    }

    #[test]
    fn export_args_without_cover_has_no_video_map() {
        let keep = [r(0.0, 10.0)];
        let plan = ExportPlan {
            input: Path::new("a.webm"),
            cover: None,
            keep: &keep,
            fade_in: false,
            fade_out: false,
            title: "t",
            artist: "a",
            output: Path::new("o.tmp"),
        };
        assert!(!export_args(&plan).contains(&"1:v".to_string()));
    }

    #[test]
    fn peaks_and_cover_args() {
        assert_eq!(
            peaks_args(Path::new("a.webm")),
            ["-hide_banner", "-nostdin", "-i", "a.webm", "-vn", "-ac", "1", "-ar", "8000", "-f", "s16le", "pipe:1"]
        );
        let c = cover_args(Path::new("t.webp"), Path::new("c.jpg"));
        assert!(c.contains(&"crop='min(iw,ih)':'min(iw,ih)',scale=600:600".to_string()));
        assert_eq!(c.last().unwrap(), "c.jpg");
    }
}
