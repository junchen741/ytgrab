//! 產生「複製錯誤報告」按鈕要放進剪貼簿的文字。

pub const DETAIL_LINES: usize = 40;

pub struct ReportInput<'a> {
    pub time: &'a str,
    pub app_version: &'a str,
    pub ytdlp_version: &'a str,
    pub os: &'a str,
    pub url: &'a str,
    pub stage: &'a str,
    pub kind: &'a str,
    pub detail: &'a str,
    pub home_dir: Option<&'a str>,
}

pub fn build_report(input: &ReportInput) -> String {
    let mut detail = last_lines(input.detail, DETAIL_LINES);
    if let Some(home) = input.home_dir.filter(|h| !h.is_empty()) {
        detail = detail.replace(home, "%USERPROFILE%");
    }
    format!(
        "【YTGrab 錯誤報告】\n時間：{}\n軟體版本：{}\nyt-dlp 版本：{}\nWindows：{}\n影片網址：{}\n發生步驟：{}\n錯誤類型：{}\n----- 錯誤訊息（最後 {} 行）-----\n{}",
        input.time,
        input.app_version,
        input.ytdlp_version,
        input.os,
        input.url,
        input.stage,
        input.kind,
        DETAIL_LINES,
        detail,
    )
}

pub fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(detail: &'a str, home: Option<&'a str>) -> ReportInput<'a> {
        ReportInput {
            time: "2026-10-27 10:00:00",
            app_version: "0.1.0",
            ytdlp_version: "2026.10.01",
            os: "Windows 10.0.19045",
            url: "https://www.youtube.com/watch?v=BaW_jenozKc",
            stage: "下載",
            kind: "outdated",
            detail,
            home_dir: home,
        }
    }

    #[test]
    fn last_lines_keeps_tail() {
        assert_eq!(last_lines("a\nb\nc\nd", 2), "c\nd");
        assert_eq!(last_lines("a", 5), "a");
    }

    #[test]
    fn report_contains_all_fields() {
        let text = build_report(&input("ERROR: boom", None));
        for needle in ["YTGrab 錯誤報告", "0.1.0", "2026.10.01", "Windows 10.0.19045", "BaW_jenozKc", "下載", "outdated", "ERROR: boom"] {
            assert!(text.contains(needle), "缺少 {needle}");
        }
    }

    #[test]
    fn home_directory_is_redacted() {
        let text = build_report(&input(r"C:\Users\Mom\AppData\Local\x", Some(r"C:\Users\Mom")));
        assert!(text.contains(r"%USERPROFILE%\AppData\Local\x"));
        assert!(!text.contains("Mom"));
    }

    #[test]
    fn long_detail_is_truncated() {
        let detail = (1..=100).map(|i| format!("line{i}")).collect::<Vec<_>>().join("\n");
        let text = build_report(&input(&detail, None));
        assert!(text.contains("line100"));
        assert!(text.contains("line61"));
        assert!(!text.contains("line60\n"));
    }
}
