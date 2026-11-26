//! Summaries of what tools measured, e.g. a load test's latencies.

/// The `p`th percentile (0–100) of `sorted`, by nearest rank; NaN when
/// there's nothing to rank.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    // Rank 1 is the smallest; a fraction of a rank rounds up.
    let rank = (p / 100.0 * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}

/// The mean of `values`; NaN when there are none.
#[allow(clippy::cast_precision_loss)]
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { f64::NAN } else { values.iter().sum::<f64>() / values.len() as f64 }
}

/// `part` as a percentage of `whole`; 0 when `whole` is.
#[allow(clippy::cast_precision_loss)]
pub fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 { 0.0 } else { part as f64 * 100.0 / whole as f64 }
}

/// How many per second, over `seconds`; 0 when no time passed.
#[allow(clippy::cast_precision_loss)]
pub fn per_second(count: usize, seconds: f64) -> f64 {
    if seconds > 0.0 { count as f64 / seconds } else { 0.0 }
}

/// Sort numbers that are never NaN, smallest first.
pub fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_by_nearest_rank() {
        let values = sorted((1..=100).rev().map(f64::from).collect());
        assert_eq!(percentile(&values, 50.0), 50.0);
        assert_eq!(percentile(&values, 95.0), 95.0);
        assert_eq!(percentile(&values, 99.9), 100.0);
        assert_eq!(percentile(&values, 0.0), 1.0);
        assert_eq!(percentile(&[7.0], 99.0), 7.0);
        assert!(percentile(&[], 50.0).is_nan());
    }

    #[test]
    fn means_percents_and_rates() {
        assert_eq!(mean(&[1.0, 2.0, 6.0]), 3.0);
        assert!(mean(&[]).is_nan());
        assert_eq!(percent(1, 4), 25.0);
        assert_eq!(percent(0, 0), 0.0);
        assert_eq!(per_second(30, 2.0), 15.0);
        assert_eq!(per_second(30, 0.0), 0.0);
    }
}
