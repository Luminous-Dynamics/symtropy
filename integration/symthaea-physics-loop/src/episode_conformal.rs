//! Episode-block conformal calibration for vector residual predictions.
//!
//! Scores are computed at the episode level (maximum vector error within an episode),
//! not by treating strongly dependent adjacent time steps as exchangeable samples.
//! The split-conformal finite-sample rank is valid only when calibration and future
//! episode scores are exchangeable. Cross-family dynamics shift violates that assumption;
//! in that setting the returned radius is an empirical calibration diagnostic, not a
//! coverage guarantee. Always report held-out coverage by scenario family.

/// Finite-sample split-conformal upper quantile for nonnegative episode scores.
///
/// Returns None for an empty/invalid sample, invalid alpha, non-finite/negative scores,
/// or when the requested rank exceeds the available calibration sample count (which
/// corresponds to an unbounded radius at the requested nominal coverage).
pub fn split_conformal_radius(scores: &[f64], alpha: f64) -> Option<f64> {
    if scores.is_empty()
        || !alpha.is_finite()
        || !(0.0..1.0).contains(&alpha)
        || scores.iter().any(|score| !score.is_finite() || *score < 0.0)
    {
        return None;
    }

    let rank = (((scores.len() + 1) as f64) * (1.0 - alpha)).ceil() as usize;
    if rank == 0 || rank > scores.len() {
        return None;
    }

    let mut ordered = scores.to_vec();
    ordered.sort_by(f64::total_cmp);
    ordered.get(rank - 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_finite_sample_corrected_rank() {
        let scores = (1..=9).map(|value| value as f64).collect::<Vec<_>>();
        assert_eq!(split_conformal_radius(&scores, 0.1), Some(9.0));
        assert_eq!(split_conformal_radius(&scores, 0.2), Some(8.0));
    }

    #[test]
    fn refuses_nominal_coverage_when_calibration_sample_is_too_small() {
        let scores = (1..=8).map(|value| value as f64).collect::<Vec<_>>();
        assert_eq!(split_conformal_radius(&scores, 0.1), None);
    }

    #[test]
    fn fails_closed_on_invalid_scores_or_coverage() {
        assert_eq!(split_conformal_radius(&[], 0.1), None);
        assert_eq!(split_conformal_radius(&[1.0, f64::NAN], 0.1), None);
        assert_eq!(split_conformal_radius(&[1.0, f64::INFINITY], 0.1), None);
        assert_eq!(split_conformal_radius(&[-0.1, 1.0], 0.1), None);
        assert_eq!(split_conformal_radius(&[1.0, 2.0], 0.0), None);
        assert_eq!(split_conformal_radius(&[1.0, 2.0], 1.0), None);
        assert_eq!(split_conformal_radius(&[1.0, 2.0], f64::NAN), None);
    }

    #[test]
    fn does_not_assume_input_scores_are_sorted() {
        assert_eq!(split_conformal_radius(&[4.0, 1.0, 3.0, 2.0], 0.4), Some(3.0));
    }
}
