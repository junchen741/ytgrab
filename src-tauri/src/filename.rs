//! 產生 Windows 合法、而且不會覆蓋舊檔的檔名。

use std::path::{Path, PathBuf};

pub const MAX_STEM_CHARS: usize = 120;

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_stem(input: &str) -> String {
    let mapped: String = input
        .chars()
        .map(|c| match c {
            '<' => '＜',
            '>' => '＞',
            ':' => '：',
            '"' => '＂',
            '/' => '／',
            '\\' => '＼',
            '|' => '｜',
            '?' => '？',
            '*' => '＊',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let collapsed = mapped.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = collapsed.chars().take(MAX_STEM_CHARS).collect();
    let trimmed = truncated.trim_end_matches(['.', ' ']).trim_start().to_string();
    if trimmed.is_empty() {
        return "未命名".to_string();
    }
    if RESERVED.contains(&trimmed.to_ascii_uppercase().as_str()) {
        format!("{trimmed}_")
    } else {
        trimmed
    }
}

/// 「演出者 - 歌名」，其中一個是空的就只用另一個。
pub fn song_stem(artist: &str, title: &str) -> String {
    let (a, t) = (artist.trim(), title.trim());
    let raw = match (a.is_empty(), t.is_empty()) {
        (true, _) => t.to_string(),
        (false, true) => a.to_string(),
        (false, false) => format!("{a} - {t}"),
    };
    sanitize_stem(&raw)
}

/// 檔名已存在時依序嘗試「名稱 (2)」「名稱 (3)」…
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}).{ext}")))
        .find(|p| !p.exists())
        .expect("無限序列一定找得到可用的檔名")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replaces_forbidden_characters_with_fullwidth() {
        assert_eq!(sanitize_stem(r#"a<b>c:d"e/f\g|h?i*j"#), "a＜b＞c：d＂e／f＼g｜h？i＊j");
    }

    #[test]
    fn trims_dots_spaces_and_control_chars() {
        assert_eq!(sanitize_stem("  hello\tworld.. "), "hello world");
    }

    #[test]
    fn reserved_device_names_get_suffix() {
        assert_eq!(sanitize_stem("con"), "con_");
        assert_eq!(sanitize_stem("LPT1"), "LPT1_");
    }

    #[test]
    fn empty_becomes_default_name() {
        assert_eq!(sanitize_stem(" ... "), "未命名");
    }

    #[test]
    fn long_names_are_truncated_by_chars() {
        let long = "歌".repeat(300);
        assert_eq!(sanitize_stem(&long).chars().count(), MAX_STEM_CHARS);
    }

    #[test]
    fn song_stem_joins_artist_and_title() {
        assert_eq!(song_stem("ミライ", "夜明けの歌"), "ミライ - 夜明けの歌");
        assert_eq!(song_stem("", "Only Title"), "Only Title");
        assert_eq!(song_stem("A/B Duo", "Go.Go."), "A／B Duo - Go.Go");
    }

    #[test]
    fn unique_path_adds_number_when_taken() {
        let dir = tempfile::tempdir().unwrap();
        let first = unique_path(dir.path(), "歌", "mp3");
        assert_eq!(first, dir.path().join("歌.mp3"));
        fs::write(&first, b"x").unwrap();
        let second = unique_path(dir.path(), "歌", "mp3");
        assert_eq!(second, dir.path().join("歌 (2).mp3"));
        fs::write(&second, b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "歌", "mp3"), dir.path().join("歌 (3).mp3"));
    }
}
