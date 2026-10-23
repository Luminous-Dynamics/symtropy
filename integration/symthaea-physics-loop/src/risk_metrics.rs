//! Small, deterministic risk summaries for robotics benchmark receipts.
//!
//! These functions summarize an empirical sample. They do not provide formal
//! risk bounds, distribution-free guarantees, or a safety case.
pub fn empirical_upper_tail_mean(values: &[f64], confidence: f64) -> Option<f64> {
    if values.is_empty()
        || !confidence.is_finite()
        || !(0.0..1.0).contains(&confidence)
        || !values.iter().all(|value| value.is_finite())
    {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let tail_count = (((1.0 - confidence) * sorted.len() as f64).ceil() as usize)
        .max(1)
        .min(sorted.len());
    let tail = &sorted[sorted.len() - tail_count..];
    Some(tail.iter().sum::<f64>() / tail.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvar90_is_mean_of_empirical_worst_ten_percent() {
        let values = (1..=10).map(f64::from).collect::<Vec<_>>();
        assert_eq!(empirical_upper_tail_mean(&values, 0.90), Some(10.0));
    }

    #[test]
    fn tail_mean_uses_ceiling_when_fractional_sample_count() {
        let values = (1..=10).map(f64::from).collect::<Vec<_>>();
        assert_eq!(empirical_upper_tail_mean(&values, 0.80), Some(9.5));
    }

    #[test]
    fn empty_invalid_or_non_finite_samples_fail_closed() {
        assert_eq!(empirical_upper_tail_mean(&[], 0.90), None);
        assert_eq!(empirical_upper_tail_mean(&[1.0], 1.0), None);
        assert_eq!(empirical_upper_tail_mean(&[1.0], f64::NAN), None);
        assert_eq!(empirical_upper_tail_mean(&[1.0, f64::INFINITY], 0.90), None);
    }

    #[test]
    fn one_sample_returns_that_sample_for_any_valid_confidence() {
        assert_eq!(empirical_upper_tail_mean(&[2.5], 0.90), Some(2.5));
    }
}
