//! 查詢 SponsorBlock 的「非音樂段落」，換算成預設裁剪範圍。

use crate::segments::{Range, TrimSpec, MIN_TOTAL};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

const API_BASE: &str = "https://sponsor.ajay.app/api/skipSegments";
const CATEGORY: &str = "music_offtopic";
/// 段落離開頭或結尾在這個秒數內，就視為「頭尾」而不是「中間」。
const EDGE_TOLERANCE: f64 = 1.0;
/// 兩段之間的空隙小於這個秒數就合併。
const MERGE_GAP: f64 = 0.5;

#[derive(Deserialize)]
struct VideoEntry {
    #[serde(rename = "videoID")]
    video_id: String,
    segments: Vec<SegmentEntry>,
}

#[derive(Deserialize)]
struct SegmentEntry {
    category: String,
    segment: [f64; 2],
}

pub fn hash_prefix(video_id: &str) -> String {
    let hash = Sha256::digest(video_id.as_bytes());
    format!("{:02x}{:02x}", hash[0], hash[1])
}

/// 不帶 categories 時伺服器只回 `sponsor` 類別，所以一定要指定。
/// `%5B%22music_offtopic%22%5D` 就是 URL 編碼後的 `["music_offtopic"]`。
pub fn query_url(video_id: &str) -> String {
    format!("{API_BASE}/{}?categories=%5B%22{CATEGORY}%22%5D", hash_prefix(video_id))
}

pub fn parse_segments(body: &str, video_id: &str) -> Vec<Range> {
    let Ok(entries) = serde_json::from_str::<Vec<VideoEntry>>(body) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|e| e.video_id == video_id)
        .flat_map(|e| e.segments)
        .filter(|s| s.category == CATEGORY)
        .map(|s| Range { start: s.segment[0], end: s.segment[1] })
        .collect()
}

pub fn suggest_trim(non_music: &[Range], duration: f64) -> TrimSpec {
    let whole = TrimSpec { start: 0.0, end: duration, cuts: Vec::new() };

    let mut sorted = non_music.to_vec();
    sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut merged: Vec<Range> = Vec::new();
    for r in sorted {
        match merged.last_mut() {
            Some(last) if r.start - last.end <= MERGE_GAP => last.end = last.end.max(r.end),
            _ => merged.push(r),
        }
    }

    let mut spec = whole.clone();
    for r in merged {
        if r.start <= spec.start + EDGE_TOLERANCE {
            spec.start = spec.start.max(r.end);
        } else if r.end >= duration - EDGE_TOLERANCE {
            spec.end = spec.end.min(r.start);
        } else {
            spec.cuts.push(r);
        }
    }
    if spec.end - spec.start < MIN_TOTAL {
        return whole;
    }
    spec
}

/// 查詢失敗一律回傳空清單；這只是「建議」，不能讓它擋住主流程。
/// 查無資料時伺服器回 404（body 是 `[]`），也會落在「非成功狀態 → 空清單」。
pub async fn fetch_non_music(video_id: &str) -> Vec<Range> {
    let url = query_url(video_id);
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let Ok(resp) = client.get(url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    match resp.text().await {
        Ok(body) => parse_segments(&body, video_id),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(start: f64, end: f64) -> Range {
        Range { start, end }
    }

    #[test]
    fn hash_prefix_is_first_four_hex_chars_of_sha256() {
        // 驗證方式（Git Bash）：printf 'BaW_jenozKc' | sha256sum  → 83d237ce...
        assert_eq!(hash_prefix("BaW_jenozKc"), "83d2");
    }

    #[test]
    fn query_url_uses_prefix_and_category_list() {
        assert_eq!(
            query_url("BaW_jenozKc"),
            "https://sponsor.ajay.app/api/skipSegments/83d2?categories=%5B%22music_offtopic%22%5D"
        );
    }

    #[test]
    fn parse_keeps_only_matching_video_and_category() {
        let body = r#"[
          {"videoID":"other123456","segments":[{"category":"music_offtopic","actionType":"skip","segment":[1.0,2.0]}]},
          {"videoID":"BaW_jenozKc","segments":[
            {"category":"music_offtopic","actionType":"skip","segment":[0.0,12.5]},
            {"category":"sponsor","actionType":"skip","segment":[30.0,40.0]},
            {"category":"music_offtopic","actionType":"skip","segment":[200.0,212.0]}
          ]}
        ]"#;
        assert_eq!(parse_segments(body, "BaW_jenozKc"), vec![r(0.0, 12.5), r(200.0, 212.0)]);
    }

    #[test]
    fn parse_garbage_returns_empty() {
        assert!(parse_segments("not json", "BaW_jenozKc").is_empty());
    }

    #[test]
    fn suggest_maps_edges_to_start_end_and_middle_to_cuts() {
        let got = suggest_trim(&[r(0.0, 12.5), r(100.0, 110.0), r(200.0, 212.0)], 212.3);
        assert_eq!(got, TrimSpec { start: 12.5, end: 200.0, cuts: vec![r(100.0, 110.0)] });
    }

    #[test]
    fn suggest_merges_adjacent_segments() {
        let got = suggest_trim(&[r(0.5, 10.0), r(10.2, 20.0)], 300.0);
        assert_eq!(got, TrimSpec { start: 20.0, end: 300.0, cuts: vec![] });
    }

    #[test]
    fn suggest_without_data_is_whole_track() {
        assert_eq!(suggest_trim(&[], 180.0), TrimSpec { start: 0.0, end: 180.0, cuts: vec![] });
    }

    #[test]
    fn suggest_ignores_data_that_would_leave_nothing() {
        let got = suggest_trim(&[r(0.0, 179.5)], 180.0);
        assert_eq!(got, TrimSpec { start: 0.0, end: 180.0, cuts: vec![] });
    }
}
