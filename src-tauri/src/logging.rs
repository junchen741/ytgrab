//! 本機記錄檔：每天一個檔案，只保留最近 7 天。

use chrono::{Local, NaiveDate};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub const KEEP_DAYS: i64 = 7;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn log_file_name(date: NaiveDate) -> String {
    format!("ytgrab-{}.log", date.format("%Y-%m-%d"))
}

/// 回傳應該刪掉的檔名（日期距今 >= keep_days 天）。不是記錄檔格式的檔案一律不動。
pub fn expired_logs(names: &[String], today: NaiveDate, keep_days: i64) -> Vec<String> {
    names
        .iter()
        .filter(|n| {
            n.strip_prefix("ytgrab-")
                .and_then(|r| r.strip_suffix(".log"))
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .is_some_and(|d| (today - d).num_days() >= keep_days)
        })
        .cloned()
        .collect()
}

/// 啟動時呼叫一次：建立資料夾、清掉舊檔。
pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    if let Ok(entries) = fs::read_dir(&dir) {
        let names: Vec<String> = entries.flatten().filter_map(|e| e.file_name().into_string().ok()).collect();
        for name in expired_logs(&names, Local::now().date_naive(), KEEP_DAYS) {
            let _ = fs::remove_file(dir.join(name));
        }
    }
    let _ = LOG_DIR.set(dir);
}

pub fn dir() -> Option<PathBuf> {
    LOG_DIR.get().cloned()
}

fn write(level: &str, msg: &str) {
    let Some(dir) = LOG_DIR.get() else { return };
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let now = Local::now();
    let path = dir.join(log_file_name(now.date_naive()));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] [{level}] {msg}", now.format("%H:%M:%S"));
    }
}

pub fn info(msg: impl AsRef<str>) {
    write("INFO", msg.as_ref());
}

pub fn error(msg: impl AsRef<str>) {
    write("ERROR", msg.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn file_name_uses_date() {
        assert_eq!(log_file_name(d("2026-10-27")), "ytgrab-2026-10-27.log");
    }

    #[test]
    fn keeps_seven_days_including_today() {
        let names: Vec<String> = ["ytgrab-2026-10-27.log", "ytgrab-2026-10-21.log", "ytgrab-2026-10-20.log", "ytgrab-2026-01-01.log", "other.txt", "ytgrab-bad.log"]
            .map(String::from)
            .to_vec();
        let mut got = expired_logs(&names, d("2026-10-27"), KEEP_DAYS);
        got.sort();
        assert_eq!(got, vec!["ytgrab-2026-01-01.log".to_string(), "ytgrab-2026-10-20.log".to_string()]);
    }
}
