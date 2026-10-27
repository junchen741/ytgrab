//! 外部工具（yt-dlp、ffmpeg、deno）的位置，以及啟動子程序的共用設定。

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

/// Windows 的 CREATE_NO_WINDOW：子程序不要跳出黑色主控台視窗。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub deno: PathBuf,
}

impl Tools {
    /// 三個工具在同一個資料夾（開發與測試時用 `src-tauri/binaries`）。
    pub fn from_dir(dir: &Path) -> Self {
        Self { ytdlp: dir.join("yt-dlp.exe"), ffmpeg: dir.join("ffmpeg.exe"), deno: dir.join("deno.exe") }
    }

    /// 安裝後的配置：ffmpeg、deno 直接用安裝資料夾裡的；
    /// yt-dlp 複製一份到使用者資料夾，因為它要能自我更新。
    ///
    /// 複製失敗（例如安裝資料夾那份已經被防毒軟體隔離）時不中斷啟動，只寫進記錄檔。
    /// 之後真的要執行 yt-dlp 時會因為找不到檔案回報 `ToolMissing`，
    /// 畫面顯示「請重新安裝」的說明，而不是程式一開就閃退。
    pub fn prepare(resource_bin: &Path, data_bin: &Path) -> Self {
        let ytdlp = data_bin.join("yt-dlp.exe");
        if !ytdlp.exists() {
            let copied = std::fs::create_dir_all(data_bin)
                .and_then(|_| std::fs::copy(resource_bin.join("yt-dlp.exe"), &ytdlp));
            if let Err(e) = copied {
                crate::logging::error(format!("無法準備 yt-dlp：{e}"));
            }
        }
        Self { ytdlp, ffmpeg: resource_bin.join("ffmpeg.exe"), deno: resource_bin.join("deno.exe") }
    }

    /// 自我更新把檔案弄壞、或被防毒軟體刪掉時，用安裝資料夾裡附的版本還原。
    pub fn restore_ytdlp(&self, resource_bin: &Path) -> std::io::Result<()> {
        if let Some(dir) = self.ytdlp.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(resource_bin.join("yt-dlp.exe"), &self.ytdlp).map(|_| ())
    }
}

/// 建立子程序：stdout/stderr 接管線、不開主控台視窗、強制 Python 用 UTF-8 輸出。
pub fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub struct Captured {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// 執行到結束，收集全部輸出。
pub async fn run(mut cmd: Command) -> std::io::Result<Captured> {
    let out = cmd.output().await?;
    Ok(Captured {
        success: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// 在背景把 stderr 讀完，避免管線塞滿讓子程序卡住。
pub async fn read_to_string_lossy(mut r: impl AsyncRead + Unpin) -> String {
    let mut buf = Vec::new();
    let _ = r.read_to_end(&mut buf).await;
    String::from_utf8_lossy(&buf).into_owned()
}
