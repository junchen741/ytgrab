//! 從 YouTube 影片標題推測「歌名」與「演出者」，並清掉常見的雜訊字樣。

use regex::{Captures, Regex};
use serde::Serialize;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SongTags {
    pub title: String,
    pub artist: String,
}

/// 括號內容含有這些英文字（必須是完整的字，前後不是英數字），整組括號就視為雜訊。
/// 不能只比對子字串，否則「Birthday」裡的 hd 也會被誤判。
static NOISE_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9])(?:official|mv|m/v|video|audio|lyrics?|visualizer|4k|hd|hq|premiere)(?:$|[^a-z0-9])")
        .unwrap()
});
/// 中文雜訊字直接比對子字串。
const NOISE_CJK: &[&str] = &["官方", "完整版", "歌詞", "字幕", "中字", "高清", "動態", "首播"];

static BRACKET_GROUP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[(\[【（]([^)\]】）]*)[)\]】）]").unwrap());
/// 結尾的雜訊字。第 1 組是雜訊前面那個字元（或開頭），取代時保留它，用來確保雜訊是完整的字。
static TRAILING_NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(^|[^a-z0-9])[\s\-|]*(?:official\s+(?:music\s+)?video|official\s+(?:mv|audio)|music\s+video|lyric\s+video|m/?v|4k|hd|hq)\s*$",
    )
    .unwrap()
});
static QUOTED_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[《「『【]([^》」』】]+)[》」』】]").unwrap());

pub fn guess_tags(raw_title: &str, channel: &str, track: Option<&str>, artist: Option<&str>) -> SongTags {
    let non_empty = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    if let (Some(title), Some(artist)) = (non_empty(track), non_empty(artist)) {
        return SongTags { title, artist };
    }

    let cleaned = clean_title(raw_title);
    let channel_artist = clean_channel(channel);

    if let Some(caps) = QUOTED_TITLE.captures(&cleaned) {
        let whole = caps.get(0).expect("第 0 組一定存在");
        let before = cleaned[..whole.start()]
            .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—'))
            .trim();
        return SongTags {
            title: caps[1].trim().to_string(),
            artist: if before.is_empty() { channel_artist } else { before.to_string() },
        };
    }
    for sep in [" - ", " – ", " — "] {
        if let Some((a, t)) = cleaned.split_once(sep) {
            return SongTags { title: t.trim().to_string(), artist: a.trim().to_string() };
        }
    }
    for sep in [" / ", "／"] {
        if let Some((t, a)) = cleaned.split_once(sep) {
            return SongTags { title: t.trim().to_string(), artist: a.trim().to_string() };
        }
    }
    SongTags { title: cleaned, artist: channel_artist }
}

pub fn clean_title(raw: &str) -> String {
    let without_groups = BRACKET_GROUP.replace_all(raw, |caps: &Captures| {
        if is_noise(&caps[1]) { String::new() } else { caps[0].to_string() }
    });
    let mut s = without_groups.split_whitespace().collect::<Vec<_>>().join(" ");
    loop {
        let next = TRAILING_NOISE.replace(&s, "${1}").into_owned();
        if next == s {
            break;
        }
        s = next;
    }
    let s = s
        .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '|'))
        .trim()
        .to_string();
    if s.is_empty() { raw.trim().to_string() } else { s }
}

fn is_noise(text: &str) -> bool {
    NOISE_WORD.is_match(text) || NOISE_CJK.iter().any(|k| text.contains(k))
}

pub fn clean_channel(channel: &str) -> String {
    let c = channel.trim();
    let c = c.strip_suffix(" - Topic").unwrap_or(c);
    let c = c.strip_suffix("VEVO").unwrap_or(c);
    c.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(title: &str, artist: &str) -> SongTags {
        SongTags { title: title.into(), artist: artist.into() }
    }

    #[test]
    fn uses_track_and_artist_from_metadata_first() {
        let got = guess_tags("whatever (Official Video)", "Some Channel", Some("Song"), Some("Singer"));
        assert_eq!(got, tags("Song", "Singer"));
    }

    // 測試用的歌手與歌名都是虛構的（repo 是公開的，不放真實的音樂作品）

    #[test]
    fn dash_separator_with_noise_brackets() {
        let got = guess_tags("Ava Lin - Morning Light (Official Music Video)", "Ava Lin", None, None);
        assert_eq!(got, tags("Morning Light", "Ava Lin"));
    }

    #[test]
    fn japanese_quote_and_trailing_noise() {
        let got = guess_tags("ミライ「夜明けの歌」 Official Music Video", "ミライ / Studio Hoshi", None, None);
        assert_eq!(got, tags("夜明けの歌", "ミライ"));
    }

    #[test]
    fn lenticular_bracket_title_is_kept_when_not_noise() {
        let got = guess_tags("林小雨 Rain Lin【晴天的約定 Promise of Sunshine】Official MV", "林小雨 Rain Lin", None, None);
        assert_eq!(got, tags("晴天的約定 Promise of Sunshine", "林小雨 Rain Lin"));
    }

    #[test]
    fn slash_means_title_then_artist() {
        let got = guess_tags("【MV】桜の坂道 / 青木ひかり", "channel", None, None);
        assert_eq!(got, tags("桜の坂道", "青木ひかり"));
    }

    #[test]
    fn falls_back_to_channel_name() {
        let got = guess_tags("rainy day study music", "Cozy Beats Channel", None, None);
        assert_eq!(got, tags("rainy day study music", "Cozy Beats Channel"));
    }

    #[test]
    fn keeps_feat_brackets() {
        assert_eq!(clean_title("Song (feat. Someone) [Official Video]"), "Song (feat. Someone)");
    }

    #[test]
    fn keeps_words_that_merely_contain_noise_letters() {
        assert_eq!(clean_title("Song (Birthday Edition)"), "Song (Birthday Edition)");
        assert_eq!(clean_title("Life of a PhD"), "Life of a PhD");
    }

    #[test]
    fn title_that_is_only_noise_falls_back_to_raw() {
        assert_eq!(clean_title("(Official Video)"), "(Official Video)");
    }

    #[test]
    fn channel_cleanup() {
        assert_eq!(clean_channel("Ava Lin - Topic"), "Ava Lin");
        assert_eq!(clean_channel("AvaLinVEVO"), "AvaLin");
    }
}
