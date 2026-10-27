//! 把「起點、終點、紅色刪除區段」換算成實際要保留的片段。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: f64,
    pub end: f64,
}

/// 前端送來的裁剪設定，也是 SponsorBlock 建議值的格式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimSpec {
    pub start: f64,
    pub end: f64,
    pub cuts: Vec<Range>,
}

/// 比這更短的保留片段直接丟掉，避免交叉淡化時出錯。
pub const MIN_KEEP_PIECE: f64 = 0.1;
/// 成品至少要這麼長。
pub const MIN_TOTAL: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimError {
    InvalidRange,
    TooShort,
}

pub fn keep_segments(spec: &TrimSpec, duration: f64) -> Result<Vec<Range>, TrimError> {
    let start = spec.start.max(0.0);
    let end = spec.end.min(duration);
    if end <= start {
        return Err(TrimError::InvalidRange);
    }

    let mut cuts: Vec<Range> = spec
        .cuts
        .iter()
        .map(|c| Range {
            start: c.start.min(c.end).max(start),
            end: c.start.max(c.end).min(end),
        })
        .filter(|c| c.end > c.start)
        .collect();
    cuts.sort_by(|a, b| a.start.total_cmp(&b.start));

    let mut keep = Vec::new();
    let mut cursor = start;
    for c in cuts {
        if c.start > cursor {
            keep.push(Range { start: cursor, end: c.start });
        }
        cursor = cursor.max(c.end);
    }
    if end > cursor {
        keep.push(Range { start: cursor, end });
    }
    keep.retain(|r| r.end - r.start >= MIN_KEEP_PIECE);

    let total: f64 = keep.iter().map(|r| r.end - r.start).sum();
    if total < MIN_TOTAL {
        return Err(TrimError::TooShort);
    }
    Ok(keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(start: f64, end: f64) -> Range {
        Range { start, end }
    }

    fn spec(start: f64, end: f64, cuts: Vec<Range>) -> TrimSpec {
        TrimSpec { start, end, cuts }
    }

    #[test]
    fn no_cuts_keeps_start_to_end() {
        assert_eq!(keep_segments(&spec(5.0, 100.0, vec![]), 200.0), Ok(vec![r(5.0, 100.0)]));
    }

    #[test]
    fn one_middle_cut_splits_in_two() {
        let got = keep_segments(&spec(0.0, 100.0, vec![r(40.0, 50.0)]), 100.0);
        assert_eq!(got, Ok(vec![r(0.0, 40.0), r(50.0, 100.0)]));
    }

    #[test]
    fn overlapping_and_unsorted_cuts_are_merged() {
        let cuts = vec![r(60.0, 70.0), r(40.0, 50.0), r(45.0, 55.0)];
        let got = keep_segments(&spec(0.0, 100.0, cuts), 100.0);
        assert_eq!(got, Ok(vec![r(0.0, 40.0), r(55.0, 60.0), r(70.0, 100.0)]));
    }

    #[test]
    fn cuts_outside_range_are_clipped_or_ignored() {
        let cuts = vec![r(0.0, 15.0), r(150.0, 160.0), r(95.0, 120.0)];
        let got = keep_segments(&spec(10.0, 100.0, cuts), 200.0);
        assert_eq!(got, Ok(vec![r(15.0, 95.0)]));
    }

    #[test]
    fn reversed_cut_is_normalized() {
        let got = keep_segments(&spec(0.0, 100.0, vec![r(50.0, 40.0)]), 100.0);
        assert_eq!(got, Ok(vec![r(0.0, 40.0), r(50.0, 100.0)]));
    }

    #[test]
    fn tiny_pieces_are_dropped() {
        let got = keep_segments(&spec(0.0, 100.0, vec![r(0.05, 50.0)]), 100.0);
        assert_eq!(got, Ok(vec![r(50.0, 100.0)]));
    }

    #[test]
    fn end_is_clamped_to_duration() {
        assert_eq!(keep_segments(&spec(0.0, 999.0, vec![]), 30.0), Ok(vec![r(0.0, 30.0)]));
    }

    #[test]
    fn too_short_or_invalid() {
        assert_eq!(keep_segments(&spec(10.0, 10.5, vec![]), 100.0), Err(TrimError::TooShort));
        assert_eq!(keep_segments(&spec(50.0, 40.0, vec![]), 100.0), Err(TrimError::InvalidRange));
    }
}
