//! 把 yt-dlp / ffmpeg 的錯誤分類，轉成家人看得懂的訊息。

use crate::segments::TrimError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    InvalidUrl,
    AgeRestricted,
    MembersOnly,
    Private,
    Unavailable,
    Live,
    BotCheck,
    RateLimited,
    Network,
    Outdated,
    TooShort,
    Export,
    /// 找不到或無法執行外部工具，多半是被防毒軟體隔離。
    ToolMissing,
    /// 寫不進存檔資料夾，例如 Windows「受控資料夾存取」擋住未簽章程式。
    SaveFailed,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    pub detail: String,
}

impl AppError {
    pub fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self { kind, message: user_message(kind).to_string(), detail: detail.into() }
    }
}

impl From<TrimError> for AppError {
    fn from(e: TrimError) -> Self {
        AppError::new(ErrorKind::TooShort, format!("{e:?}"))
    }
}

pub fn classify_ytdlp(stderr: &str) -> ErrorKind {
    let s = stderr.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| s.contains(n));
    if has(&["confirm your age", "age-restricted", "inappropriate for some users"]) {
        ErrorKind::AgeRestricted
    } else if has(&["members-only", "join this channel", "available to this channel's members"]) {
        ErrorKind::MembersOnly
    } else if has(&["private video"]) {
        ErrorKind::Private
    } else if has(&["not a bot"]) {
        ErrorKind::BotCheck
    } else if has(&["http error 429", "too many requests"]) {
        ErrorKind::RateLimited
    } else if has(&["this live event", "premieres in", "is live now", "live stream"]) {
        ErrorKind::Live
    } else if has(&["video unavailable", "has been removed", "no longer available", "not available in your country"]) {
        ErrorKind::Unavailable
    } else if has(&["unable to download webpage", "getaddrinfo failed", "timed out", "connection reset", "network is unreachable", "failed to resolve"]) {
        ErrorKind::Network
    } else if has(&["nsig", "signature", "challenge", "requested format is not available", "javascript runtime", "unable to extract", "player response", "http error 403"]) {
        ErrorKind::Outdated
    } else {
        ErrorKind::Unknown
    }
}

/// 這兩類錯誤常常是 yt-dlp 太舊造成的，值得先更新再試一次。
pub fn is_retryable_after_update(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::Outdated | ErrorKind::Unknown)
}

pub fn user_message(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidUrl => "這不是可以處理的 YouTube 影片網址，請確認後再貼一次。",
        ErrorKind::AgeRestricted => "這部影片有年齡限制，需要登入 YouTube 才能觀看，目前無法下載。",
        ErrorKind::MembersOnly => "這部影片是頻道會員限定，目前無法下載。",
        ErrorKind::Private => "這部影片是私人影片，目前無法下載。",
        ErrorKind::Unavailable => "這部影片已被移除或目前無法觀看。",
        ErrorKind::Live => "這是直播或還沒開始的首播，請等直播結束後再試。",
        ErrorKind::BotCheck => "YouTube 暫時拒絕了連線，請過一段時間再試。如果一直失敗，請把錯誤報告傳給開發人員。",
        ErrorKind::RateLimited => "短時間內下載太多次，YouTube 暫時限制了連線，請過一段時間再試。",
        ErrorKind::Network => "網路連線有問題，請確認網路正常後再試一次。",
        ErrorKind::Outdated => "YouTube 可能剛改版，請過幾天再試。如果一直失敗，請把錯誤報告傳給開發人員。",
        ErrorKind::TooShort => "裁剪後的長度太短（不到 1 秒），請調整起點、終點或刪除區段。",
        ErrorKind::Export => "轉檔時發生錯誤。如果一直失敗，請把錯誤報告傳給開發人員。",
        ErrorKind::ToolMissing => "防毒軟體可能把 YTGrab 需要的元件刪掉了。請重新安裝 YTGrab 再試一次；如果一直發生，請把錯誤報告傳給開發人員。",
        ErrorKind::SaveFailed => "無法存到目前的存檔資料夾，可能是 Windows 的安全性設定擋住了。請到首頁按「變更」換一個資料夾（例如桌面上新建的資料夾）再試一次。",
        ErrorKind::Unknown => "發生了預期外的錯誤。如果一直失敗，請把錯誤報告傳給開發人員。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_ytdlp_errors() {
        let cases = [
            ("ERROR: [youtube] abc: Sign in to confirm your age. This video may be inappropriate", ErrorKind::AgeRestricted),
            ("ERROR: [youtube] abc: Join this channel to get access to members-only content", ErrorKind::MembersOnly),
            ("ERROR: [youtube] abc: Private video. Sign in if you've been granted access", ErrorKind::Private),
            ("ERROR: [youtube] abc: Sign in to confirm you’re not a bot", ErrorKind::BotCheck),
            ("ERROR: unable to download video data: HTTP Error 429: Too Many Requests", ErrorKind::RateLimited),
            ("ERROR: [youtube] abc: This live event will begin in 3 hours.", ErrorKind::Live),
            ("ERROR: [youtube] abc: Video unavailable. This video has been removed by the uploader", ErrorKind::Unavailable),
            ("ERROR: [youtube] abc: Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>", ErrorKind::Network),
            ("WARNING: [youtube] nsig extraction failed: Some formats may be missing\nERROR: Requested format is not available", ErrorKind::Outdated),
            ("ERROR: unable to download video data: HTTP Error 403: Forbidden", ErrorKind::Outdated),
            ("something nobody has seen before", ErrorKind::Unknown),
        ];
        for (stderr, expected) in cases {
            assert_eq!(classify_ytdlp(stderr), expected, "stderr = {stderr}");
        }
    }

    #[test]
    fn only_outdated_and_unknown_are_retried_after_update() {
        assert!(is_retryable_after_update(ErrorKind::Outdated));
        assert!(is_retryable_after_update(ErrorKind::Unknown));
        assert!(!is_retryable_after_update(ErrorKind::Private));
        assert!(!is_retryable_after_update(ErrorKind::Network));
    }

    #[test]
    fn messages_that_need_developer_mention_the_report() {
        for kind in [ErrorKind::Outdated, ErrorKind::Unknown, ErrorKind::Export, ErrorKind::BotCheck, ErrorKind::ToolMissing] {
            assert!(user_message(kind).contains("請把錯誤報告傳給開發人員"), "{kind:?}");
        }
        assert!(!user_message(ErrorKind::Private).contains("開發人員"));
    }

    #[test]
    fn app_error_carries_message_and_detail() {
        let e = AppError::new(ErrorKind::Network, "raw stderr");
        assert_eq!(e.message, user_message(ErrorKind::Network));
        assert_eq!(e.detail, "raw stderr");
    }
}
