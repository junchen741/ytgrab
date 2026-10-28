//! 把 16-bit PCM 串流壓縮成畫波形用的數值；記憶體用量和影片長度無關。
//!
//! 每一段（1/50 秒）取平均音量（RMS），不取最大值：音量壓得很滿的流行歌幾乎每一段的最大值都頂到滿格，
//! 畫出來頂端是一條平線，看起來像被切掉；平均音量才看得出段落和起伏。前端畫的時候會再縮放到滿版。

pub const SAMPLE_RATE: u32 = 8_000;
pub const PEAKS_PER_SECOND: u32 = 50;

pub fn samples_per_peak() -> usize {
    (SAMPLE_RATE / PEAKS_PER_SECOND) as usize
}

pub struct PeakAccumulator {
    samples_per_peak: usize,
    in_bucket: usize,
    /// 這一段 sample 的平方和（-1.0～1.0 的比例）
    bucket_sum_sq: f64,
    pending_byte: Option<u8>,
    total_samples: u64,
    peaks: Vec<f32>,
}

impl PeakAccumulator {
    pub fn new(samples_per_peak: usize) -> Self {
        assert!(samples_per_peak > 0);
        Self { samples_per_peak, in_bucket: 0, bucket_sum_sq: 0.0, pending_byte: None, total_samples: 0, peaks: Vec::new() }
    }

    /// 接收任意長度的 s16le 位元組；跨 chunk 的半個 sample 會暫存到下一次。
    pub fn push_bytes(&mut self, mut bytes: &[u8]) {
        if let Some(lo) = self.pending_byte.take() {
            match bytes.split_first() {
                Some((&hi, rest)) => {
                    self.push_sample(i16::from_le_bytes([lo, hi]));
                    bytes = rest;
                }
                None => {
                    self.pending_byte = Some(lo);
                    return;
                }
            }
        }
        let mut pairs = bytes.chunks_exact(2);
        for pair in &mut pairs {
            self.push_sample(i16::from_le_bytes([pair[0], pair[1]]));
        }
        if let [last] = pairs.remainder() {
            self.pending_byte = Some(*last);
        }
    }

    fn push_sample(&mut self, s: i16) {
        let v = f64::from(s) / 32768.0;
        self.bucket_sum_sq += v * v;
        self.in_bucket += 1;
        self.total_samples += 1;
        if self.in_bucket == self.samples_per_peak {
            self.flush();
        }
    }

    fn flush(&mut self) {
        // 最後一段可能不滿，要除以實際的 sample 數
        self.peaks.push((self.bucket_sum_sq / self.in_bucket as f64).sqrt() as f32);
        self.bucket_sum_sq = 0.0;
        self.in_bucket = 0;
    }

    /// 回傳（每段的平均音量, 實際長度秒數）。
    pub fn finish(mut self) -> (Vec<f32>, f64) {
        if self.in_bucket > 0 {
            self.flush();
        }
        let duration = self.total_samples as f64 / f64::from(SAMPLE_RATE);
        (self.peaks, duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(samples: &[i16]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn assert_close(got: &[f32], want: &[f32]) {
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-6, "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn rms_per_bucket() {
        // 第一段 [0, 400] 的平均音量（RMS）是 400/√2，不是最大值 400
        let mut acc = PeakAccumulator::new(2);
        acc.push_bytes(&bytes(&[0, 400, -300, 300]));
        let (peaks, duration) = acc.finish();
        assert_close(&peaks, &[400.0 / 2f32.sqrt() / 32768.0, 300.0 / 32768.0]);
        assert_eq!(duration, 4.0 / SAMPLE_RATE as f64);
    }

    #[test]
    fn handles_samples_split_across_chunks() {
        let data = bytes(&[0, 400, -300, 300]);
        let mut acc = PeakAccumulator::new(2);
        for b in &data {
            acc.push_bytes(std::slice::from_ref(b));
        }
        assert_close(&acc.finish().0, &[400.0 / 2f32.sqrt() / 32768.0, 300.0 / 32768.0]);
    }

    #[test]
    fn partial_last_bucket_is_flushed() {
        // 最後一段只有 1 個 sample，平均要除以 1，不是除以整段的長度
        let mut acc = PeakAccumulator::new(2);
        acc.push_bytes(&bytes(&[100, -100, 3]));
        assert_close(&acc.finish().0, &[100.0 / 32768.0, 3.0 / 32768.0]);
    }

    #[test]
    fn most_negative_sample_is_full_scale() {
        let mut acc = PeakAccumulator::new(1);
        acc.push_bytes(&bytes(&[i16::MIN]));
        assert_eq!(acc.finish().0, vec![1.0]);
    }

    #[test]
    fn default_bucket_size_matches_rate() {
        assert_eq!(samples_per_peak(), 160);
    }
}
