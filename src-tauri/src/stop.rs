//! 「停止」鍵：讀取影片資訊或下載到一半時，讓使用者中途放棄。

use std::future::Future;
use tokio::sync::watch;

/// 裡面是「按了幾次停止」的計數；每個工作開始時記下當時的值，之後值變了就代表要停。
/// 所以之前按過的停止不會影響之後才開始的工作。
pub struct StopSignal(watch::Sender<u64>);

impl Default for StopSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl StopSignal {
    pub fn new() -> Self {
        Self(watch::channel(0).0)
    }

    /// 停止目前正在進行的工作。
    pub fn stop(&self) {
        // 沒有工作在等也要記下來（send_modify 不管有沒有接收端都會更新）
        self.0.send_modify(|n| *n = n.wrapping_add(1));
    }

    /// 執行 `work`；途中有人呼叫 [`stop`](Self::stop) 就把它丟掉並回傳 None。
    /// 丟掉時 yt-dlp、ffmpeg 子程序會因為 kill_on_drop 跟著結束（見 tools::command）。
    pub async fn run<F: Future>(&self, work: F) -> Option<F::Output> {
        let mut stopped = self.0.subscribe();
        stopped.borrow_and_update();
        tokio::select! {
            out = work => Some(out),
            Ok(()) = stopped.changed() => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    /// 被丟掉（drop）時把旗標設成 true，用來確認工作真的被丟掉了（子程序靠 kill_on_drop 結束）。
    struct DropFlag(Arc<AtomicBool>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn stop_during_work_drops_it() {
        let signal = Arc::new(StopSignal::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(dropped.clone());
        let stopper = signal.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            stopper.stop();
        });

        let work = async move {
            let _flag = flag;
            std::future::pending::<()>().await
        };
        let got = tokio::time::timeout(Duration::from_secs(2), signal.run(work)).await;
        assert!(matches!(got, Ok(None)), "按了停止應該馬上回傳 None：{got:?}");
        assert!(dropped.load(Ordering::SeqCst), "工作要被丟掉");
    }

    #[tokio::test]
    async fn stop_before_work_starts_does_not_cancel_it() {
        // 上一首按過停止，不能影響下一次開始的工作
        let signal = StopSignal::new();
        signal.stop();
        let got = signal
            .run(async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                7
            })
            .await;
        assert_eq!(got, Some(7));
    }
}
