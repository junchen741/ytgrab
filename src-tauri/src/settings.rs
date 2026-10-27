//! 使用者設定（目前只有存檔資料夾），存成 JSON。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub output_dir: PathBuf,
}

/// 檔案不存在或壞掉都回到預設值，不讓設定檔問題擋住程式啟動。
pub fn load(path: &Path, default_output: PathBuf) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Settings { output_dir: default_output })
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        let s = load(&dir.path().join("settings.json"), PathBuf::from(r"C:\Music\YouTube 音訊"));
        assert_eq!(s.output_dir, PathBuf::from(r"C:\Music\YouTube 音訊"));
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        let s = Settings { output_dir: PathBuf::from(r"D:\歌") };
        save(&path, &s).unwrap();
        assert_eq!(load(&path, PathBuf::from("x")), s);
    }

    #[test]
    fn corrupted_file_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(load(&path, PathBuf::from("x")).output_dir, PathBuf::from("x"));
    }
}
