//! 把 16-bit PCM 串流壓縮成波形峰值；記憶體用量和影片長度無關。

pub const SAMPLE_RATE: u32 = 8_000;
pub const PEAKS_PER_SECOND: u32 = 50;

pub fn samples_per_peak() -> usize {
    (SAMPLE_RATE / PEAKS_PER_SECOND) as usize
}

pub struct PeakAccumulator {
    samples_per_peak: usize,
    in_bucket: usize,
    bucket_max: u32,
    pending_byte: Option<u8>,
    total_samples: u64,
    peaks: Vec<f32>,
}

impl PeakAccumulator {
    pub fn new(samples_per_peak: usize) -> Self {
        assert!(samples_per_peak > 0);
        Self { samples_per_peak, in_bucket: 0, bucket_max: 0, pending_byte: None, total_samples: 0, peaks: Vec::new() }
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
        self.bucket_max = self.bucket_max.max(u32::from(s.unsigned_abs()));
        self.in_bucket += 1;
        self.total_samples += 1;
        if self.in_bucket == self.samples_per_peak {
            self.flush();
        }
    }

    fn flush(&mut self) {
        self.peaks.push(self.bucket_max as f32 / 32768.0);
        self.bucket_max = 0;
        self.in_bucket = 0;
    }

    /// 回傳（峰值, 實際長度秒數）。
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

    #[test]
    fn max_abs_per_bucket() {
        let mut acc = PeakAccumulator::new(2);
        acc.push_bytes(&bytes(&[100, -200, 300, 50]));
        let (peaks, duration) = acc.finish();
        assert_eq!(peaks, vec![200.0 / 32768.0, 300.0 / 32768.0]);
        assert_eq!(duration, 4.0 / SAMPLE_RATE as f64);
    }

    #[test]
    fn handles_samples_split_across_chunks() {
        let data = bytes(&[100, -200, 300, 50]);
        let mut acc = PeakAccumulator::new(2);
        for b in &data {
            acc.push_bytes(std::slice::from_ref(b));
        }
        assert_eq!(acc.finish().0, vec![200.0 / 32768.0, 300.0 / 32768.0]);
    }

    #[test]
    fn partial_last_bucket_is_flushed() {
        let mut acc = PeakAccumulator::new(2);
        acc.push_bytes(&bytes(&[1, 2, 3]));
        assert_eq!(acc.finish().0.len(), 2);
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
