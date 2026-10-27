//! 解析使用者貼上的 YouTube 網址，取出影片 ID，並轉成只含影片 ID 的標準網址。

/// 取出 11 碼影片 ID；不是支援的 YouTube 影片網址時回傳 `None`。
pub fn extract_video_id(input: &str) -> Option<String> {
    let s = input.trim();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let s = s.split('#').next().unwrap_or("");
    let (host, rest) = s.split_once('/').unwrap_or((s, ""));
    let host_lc = host.to_ascii_lowercase();
    let host = host_lc
        .strip_prefix("www.")
        .or_else(|| host_lc.strip_prefix("m."))
        .unwrap_or(&host_lc);
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));

    let candidate = match host {
        "youtu.be" => path.split('/').next().map(str::to_string),
        "youtube.com" | "music.youtube.com" => {
            if path == "watch" {
                query_param(query, "v")
            } else {
                path.strip_prefix("shorts/")
                    .or_else(|| path.strip_prefix("live/"))
                    .and_then(|id| id.split('/').next())
                    .map(str::to_string)
            }
        }
        _ => None,
    };
    candidate.filter(|id| is_valid_id(id))
}

/// 轉成標準網址；只保留影片 ID，所以播放清單等參數都會被丟掉。
pub fn canonical_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={video_id}")
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn is_valid_id(id: &str) -> bool {
    id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "BaW_jenozKc";

    #[test]
    fn watch_url_with_playlist_keeps_only_video() {
        let got = extract_video_id("https://www.youtube.com/watch?v=BaW_jenozKc&list=PL123&index=2");
        assert_eq!(got.as_deref(), Some(ID));
    }

    #[test]
    fn short_link_with_share_param() {
        assert_eq!(extract_video_id("https://youtu.be/BaW_jenozKc?si=abc").as_deref(), Some(ID));
    }

    #[test]
    fn music_and_mobile_hosts() {
        assert_eq!(extract_video_id("https://music.youtube.com/watch?v=BaW_jenozKc&feature=share").as_deref(), Some(ID));
        assert_eq!(extract_video_id("https://m.youtube.com/watch?v=BaW_jenozKc").as_deref(), Some(ID));
    }

    #[test]
    fn shorts_and_live_paths() {
        assert_eq!(extract_video_id("https://youtube.com/shorts/BaW_jenozKc").as_deref(), Some(ID));
        assert_eq!(extract_video_id("https://www.youtube.com/live/BaW_jenozKc?feature=share").as_deref(), Some(ID));
    }

    #[test]
    fn without_scheme_with_spaces_and_fragment() {
        assert_eq!(extract_video_id("  youtube.com/watch?v=BaW_jenozKc#t=10  ").as_deref(), Some(ID));
    }

    #[test]
    fn rejects_other_sites_playlists_and_bad_ids() {
        assert_eq!(extract_video_id("https://example.com/watch?v=BaW_jenozKc"), None);
        assert_eq!(extract_video_id("https://www.youtube.com/playlist?list=PL123"), None);
        assert_eq!(extract_video_id("https://youtu.be/short"), None);
        assert_eq!(extract_video_id(""), None);
    }

    #[test]
    fn canonical_url_only_has_video_id() {
        assert_eq!(canonical_url(ID), "https://www.youtube.com/watch?v=BaW_jenozKc");
    }
}
