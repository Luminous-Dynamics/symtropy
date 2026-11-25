//! Candidate-visible affine clock-registration reference estimators.
//!
//! Estimators consume only public paired calibration-anchor timestamps. They
//! do not read evaluator truth, event identities, arrival delays or hidden
//! clock parameters. The independent owner still qualifies proposals against
//! withheld anchors and true parameters.
//!
//! Scope: affine Beta-local-time -> Alpha-local-time registration. This module
//! is a small baseline tournament, not a sensor service or a general clock
//! synchronization package.

use std::collections::BTreeSet;
use std::fmt;

pub const MAX_CLOCK_ANCHORS_V0: usize = 128;
const HUBER_TUNING_V0: f64 = 1.345;
const MAD_NORMAL_SCALE_V0: f64 = 1.4826;
const MIN_VARIANCE_V0: f64 = 1.0e-24;
const MAX_HUBER_ITERATIONS_V0: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockAnchorPairV0 {
    pub anchor_id: u16,
    /// Measurement timestamp in Alpha's local sensor clock.
    pub alpha_local_time: i64,
    /// Measurement timestamp in Beta's local sensor clock.
    pub beta_local_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockFitErrorV0 {
    TooFewAnchors { found: usize },
    TooManyAnchors { found: usize, maximum: usize },
    DuplicateAnchorId { anchor_id: u16 },
    NonIdentifiableTimebase,
    NonPositiveScale,
    NonFiniteEstimate,
    InvalidIterationCount,
    InvalidTolerance,
}

impl fmt::Display for ClockFitErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewAnchors { found } => {
                write!(formatter, "affine clock fit needs at least two anchors; found {found}")
            }
            Self::TooManyAnchors { found, maximum } => {
                write!(formatter, "anchor count {found} exceeds v0 limit {maximum}")
            }
            Self::DuplicateAnchorId { anchor_id } => {
                write!(formatter, "duplicate calibration anchor ID {anchor_id}")
            }
            Self::NonIdentifiableTimebase => {
                formatter.write_str("anchor timestamps do not identify an affine clock rate")
            }
            Self::NonPositiveScale => {
                formatter.write_str("estimated clock scale must be finite and positive")
            }
            Self::NonFiniteEstimate => {
                formatter.write_str("clock estimator produced a non-finite value")
            }
            Self::InvalidIterationCount => {
                formatter.write_str("Huber iteration count must be between 1 and 100")
            }
            Self::InvalidTolerance => {
                formatter.write_str("Huber convergence tolerance must be finite and positive")
            }
        }
    }
}

impl std::error::Error for ClockFitErrorV0 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockFitV0 {
    /// Beta -> Alpha: alpha_time ~= offset + scale * beta_time.
    pub offset: f64,
    pub scale: f64,
    /// Exact integer origins for stable evaluation on large timestamp epochs.
    pub alpha_reference_time: i64,
    pub beta_reference_time: i64,
    /// Fitted Alpha-minus-reference offset when Beta equals its reference.
    pub reference_intercept: f64,
    pub anchor_count: usize,
    /// Residual metrics are calculated on the anchors used by the fit.
    pub residual_rms: f64,
    pub median_absolute_residual: f64,
    pub max_absolute_residual: f64,
    /// Zero for closed-form methods; actual reweighting iterations for Huber.
    pub iterations: usize,
}

impl ClockFitV0 {
    /// Predict Alpha time relative to `alpha_reference_time`, avoiding a
    /// lossy global-offset calculation for large absolute timestamp epochs.
    pub fn predict_alpha_delta_v0(&self, beta_local_time: i64) -> f64 {
        let beta_delta =
            (i128::from(beta_local_time) - i128::from(self.beta_reference_time)) as f64;
        self.reference_intercept + self.scale * beta_delta
    }
}

#[derive(Debug, Clone, Copy)]
struct CenteredPairV0 {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct LocalLineV0 {
    /// y ~= intercept + scale * x in coordinates centered on the first pair.
    intercept: f64,
    scale: f64,
}

#[derive(Debug, Clone, Copy)]
struct OriginsV0 {
    alpha: f64,
    beta: f64,
}

fn validate_pairs(pairs: &[ClockAnchorPairV0]) -> Result<OriginsV0, ClockFitErrorV0> {
    if pairs.len() < 2 {
        return Err(ClockFitErrorV0::TooFewAnchors { found: pairs.len() });
    }
    if pairs.len() > MAX_CLOCK_ANCHORS_V0 {
        return Err(ClockFitErrorV0::TooManyAnchors {
            found: pairs.len(),
            maximum: MAX_CLOCK_ANCHORS_V0,
        });
    }

    let mut ids = BTreeSet::new();
    for pair in pairs {
        if !ids.insert(pair.anchor_id) {
            return Err(ClockFitErrorV0::DuplicateAnchorId {
                anchor_id: pair.anchor_id,
            });
        }
    }

    let first = pairs[0];
    let has_distinct_beta_time = pairs
        .iter()
        .any(|pair| pair.beta_local_time != first.beta_local_time);
    if !has_distinct_beta_time {
        return Err(ClockFitErrorV0::NonIdentifiableTimebase);
    }

    Ok(OriginsV0 {
        alpha: first.alpha_local_time as f64,
        beta: first.beta_local_time as f64,
    })
}

fn centered_pairs(pairs: &[ClockAnchorPairV0]) -> Vec<CenteredPairV0> {
    let first = pairs[0];
    pairs
        .iter()
        .map(|pair| CenteredPairV0 {
            // Subtract in i128 before converting to f64. This preserves small
            // timestamp differences when the absolute epoch is very large.
            x: (i128::from(pair.beta_local_time) - i128::from(first.beta_local_time)) as f64,
            y: (i128::from(pair.alpha_local_time) - i128::from(first.alpha_local_time)) as f64,
        })
        .collect()
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let middle = ordered.len() / 2;
    if ordered.len() % 2 == 0 {
        Some((ordered[middle - 1] + ordered[middle]) * 0.5)
    } else {
        Some(ordered[middle])
    }
}

fn validate_line(line: LocalLineV0) -> Result<(), ClockFitErrorV0> {
    if !line.intercept.is_finite() || !line.scale.is_finite() {
        return Err(ClockFitErrorV0::NonFiniteEstimate);
    }
    if line.scale <= 0.0 {
        return Err(ClockFitErrorV0::NonPositiveScale);
    }
    Ok(())
}

fn weighted_least_squares(
    points: &[CenteredPairV0],
    weights: &[f64],
) -> Result<LocalLineV0, ClockFitErrorV0> {
    if points.len() != weights.len() || points.len() < 2 {
        return Err(ClockFitErrorV0::NonIdentifiableTimebase);
    }

    let total_weight = weights.iter().sum::<f64>();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err(ClockFitErrorV0::NonFiniteEstimate);
    }

    let mean_x = points
        .iter()
        .zip(weights)
        .map(|(point, weight)| point.x * weight)
        .sum::<f64>()
        / total_weight;
    let mean_y = points
        .iter()
        .zip(weights)
        .map(|(point, weight)| point.y * weight)
        .sum::<f64>()
        / total_weight;

    let mut denominator = 0.0;
    let mut numerator = 0.0;
    for (point, weight) in points.iter().zip(weights) {
        let centered_x = point.x - mean_x;
        denominator += weight * centered_x * centered_x;
        numerator += weight * centered_x * (point.y - mean_y);
    }

    if !denominator.is_finite() || denominator <= MIN_VARIANCE_V0 {
        return Err(ClockFitErrorV0::NonIdentifiableTimebase);
    }

    let line = LocalLineV0 {
        intercept: mean_y - (numerator / denominator) * mean_x,
        scale: numerator / denominator,
    };
    validate_line(line)?;
    Ok(line)
}

fn unweighted_least_squares(
    points: &[CenteredPairV0],
) -> Result<LocalLineV0, ClockFitErrorV0> {
    weighted_least_squares(points, &vec![1.0; points.len()])
}

fn residuals(points: &[CenteredPairV0], line: LocalLineV0) -> Vec<f64> {
    points
        .iter()
        .map(|point| point.y - (line.intercept + line.scale * point.x))
        .collect()
}

fn build_fit(
    pairs: &[ClockAnchorPairV0],
    origins: OriginsV0,
    points: &[CenteredPairV0],
    line: LocalLineV0,
    iterations: usize,
) -> Result<ClockFitV0, ClockFitErrorV0> {
    validate_line(line)?;
    let errors = residuals(points, line);
    let absolute = errors.iter().map(|value| value.abs()).collect::<Vec<_>>();
    let sum_squares = errors.iter().map(|value| value * value).sum::<f64>();
    let rms = (sum_squares / errors.len() as f64).sqrt();
    let median_absolute_residual =
        median(&absolute).ok_or(ClockFitErrorV0::NonFiniteEstimate)?;
    let max_absolute_residual = absolute.iter().copied().fold(0.0_f64, f64::max);

    let offset = origins.alpha + line.intercept - line.scale * origins.beta;
    let fit = ClockFitV0 {
        offset,
        scale: line.scale,
        alpha_reference_time: pairs[0].alpha_local_time,
        beta_reference_time: pairs[0].beta_local_time,
        reference_intercept: line.intercept,
        anchor_count: pairs.len(),
        residual_rms: rms,
        median_absolute_residual,
        max_absolute_residual,
        iterations,
    };

    if !fit.offset.is_finite()
        || !fit.scale.is_finite()
        || !fit.residual_rms.is_finite()
        || !fit.median_absolute_residual.is_finite()
        || !fit.max_absolute_residual.is_finite()
    {
        return Err(ClockFitErrorV0::NonFiniteEstimate);
    }
    if fit.scale <= 0.0 {
        return Err(ClockFitErrorV0::NonPositiveScale);
    }
    Ok(fit)
}

/// Ordinary least-squares affine fit. This is a high-efficiency reference
/// under approximately Gaussian, non-contaminated anchor error.
pub fn fit_affine_ols_v0(
    pairs: &[ClockAnchorPairV0],
) -> Result<ClockFitV0, ClockFitErrorV0> {
    let origins = validate_pairs(pairs)?;
    let points = centered_pairs(pairs);
    let line = unweighted_least_squares(&points)?;
    build_fit(pairs, origins, &points, line, 0)
}

fn theil_sen_line(points: &[CenteredPairV0]) -> Result<LocalLineV0, ClockFitErrorV0> {
    let mut slopes = Vec::new();

    for left in 0..points.len() {
        for right in left + 1..points.len() {
            let delta_x = points[right].x - points[left].x;
            if delta_x.abs() > 0.0 {
                slopes.push((points[right].y - points[left].y) / delta_x);
            }
        }
    }

    let scale = median(&slopes).ok_or(ClockFitErrorV0::NonIdentifiableTimebase)?;
    let intercepts = points
        .iter()
        .map(|point| point.y - scale * point.x)
        .collect::<Vec<_>>();
    let intercept = median(&intercepts).ok_or(ClockFitErrorV0::NonFiniteEstimate)?;
    let line = LocalLineV0 { intercept, scale };
    validate_line(line)?;
    Ok(line)
}

/// Theil-Sen affine baseline: median of all pairwise slopes and then median
/// intercept. It is robust to a minority of gross anchor outliers, but has
/// quadratic pair count and is intended only for the bounded v0 anchor set.
pub fn fit_affine_theil_sen_v0(
    pairs: &[ClockAnchorPairV0],
) -> Result<ClockFitV0, ClockFitErrorV0> {
    let origins = validate_pairs(pairs)?;
    let points = centered_pairs(pairs);
    let line = theil_sen_line(&points)?;
    build_fit(pairs, origins, &points, line, 0)
}
/// Huber IRLS affine fit. Theil-Sen supplies a robust initial line; MAD scales
/// residuals; Huber weights reduce the influence of gross anchor outliers.
/// This is a generic robust baseline, not a claim to reproduce MATH-SYNC's
/// full estimator or cohort protocol.
pub fn fit_affine_huber_irls_v0(
    pairs: &[ClockAnchorPairV0],
    max_iterations: usize,
    tolerance: f64,
) -> Result<ClockFitV0, ClockFitErrorV0> {
    if max_iterations == 0 || max_iterations > MAX_HUBER_ITERATIONS_V0 {
        return Err(ClockFitErrorV0::InvalidIterationCount);
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(ClockFitErrorV0::InvalidTolerance);
    }

    let origins = validate_pairs(pairs)?;
    let points = centered_pairs(pairs);
    let mut line = theil_sen_line(&points)?;

    for iteration in 1..=max_iterations {
        let errors = residuals(&points, line);
        let residual_center = median(&errors).ok_or(ClockFitErrorV0::NonFiniteEstimate)?;
        let centered_absolute = errors
            .iter()
            .map(|error| (error - residual_center).abs())
            .collect::<Vec<_>>();
        let mad = median(&centered_absolute).ok_or(ClockFitErrorV0::NonFiniteEstimate)?;
        let robust_scale = (MAD_NORMAL_SCALE_V0 * mad).max(1.0e-9);
        let cutoff = HUBER_TUNING_V0 * robust_scale;

        let weights = centered_absolute
            .iter()
            .map(|error| {
                if *error <= cutoff {
                    1.0
                } else {
                    cutoff / *error
                }
            })
            .collect::<Vec<_>>();
        let next = weighted_least_squares(&points, &weights)?;
        let intercept_delta = (next.intercept - line.intercept).abs();
        let scale_delta = (next.scale - line.scale).abs();
        line = next;

        if intercept_delta <= tolerance && scale_delta <= tolerance {
            return build_fit(pairs, origins, &points, line, iteration);
        }
    }

    build_fit(pairs, origins, &points, line, max_iterations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors(
        offset: i64,
        scale_numerator: i64,
        scale_denominator: i64,
    ) -> Vec<ClockAnchorPairV0> {
        (0..9)
            .map(|index| {
                let beta = 100_i64 + index * 100;
                ClockAnchorPairV0 {
                    anchor_id: u16::try_from(index).unwrap_or(u16::MAX),
                    beta_local_time: beta,
                    alpha_local_time: offset
                        + (scale_numerator * beta) / scale_denominator,
                }
            })
            .collect()
    }

    #[test]
    fn ols_recovers_exact_affine_map() {
        let pairs = anchors(-20, 2, 1);
        let fit = fit_affine_ols_v0(&pairs).unwrap();
        assert!((fit.offset + 20.0).abs() < 1.0e-9);
        assert!((fit.scale - 2.0).abs() < 1.0e-12);
        assert!(fit.residual_rms < 1.0e-9);
    }

    #[test]
    fn theil_sen_recovers_exact_affine_map() {
        let pairs = anchors(15, 3, 2);
        let fit = fit_affine_theil_sen_v0(&pairs).unwrap();
        assert!((fit.offset - 15.0).abs() < 1.0e-8);
        assert!((fit.scale - 1.5).abs() < 1.0e-12);
    }

    #[test]
    fn huber_resists_a_gross_anchor_outlier_better_than_ols() {
        let mut pairs = anchors(20, 2, 1);
        pairs[8].alpha_local_time += 500;
        let ols = fit_affine_ols_v0(&pairs).unwrap();
        let huber = fit_affine_huber_irls_v0(&pairs, 50, 1.0e-10).unwrap();
        let theil_sen = fit_affine_theil_sen_v0(&pairs).unwrap();

        let ols_error = (ols.offset - 20.0).abs() + (ols.scale - 2.0).abs() * 1_000.0;
        let huber_error = (huber.offset - 20.0).abs() + (huber.scale - 2.0).abs() * 1_000.0;
        let theil_sen_error =
            (theil_sen.offset - 20.0).abs() + (theil_sen.scale - 2.0).abs() * 1_000.0;

        assert!(huber_error < ols_error);
        assert!(theil_sen_error < ols_error);
        assert!(huber.median_absolute_residual < ols.median_absolute_residual);
        assert!(theil_sen.median_absolute_residual < ols.median_absolute_residual);
    }

    #[test]
    fn one_anchor_is_not_enough_to_identify_affine_rate() {
        let pair = ClockAnchorPairV0 {
            anchor_id: 0,
            alpha_local_time: 5,
            beta_local_time: 10,
        };
        assert_eq!(
            fit_affine_ols_v0(&[pair]),
            Err(ClockFitErrorV0::TooFewAnchors { found: 1 })
        );
    }

    #[test]
    fn repeated_beta_timestamps_are_not_identifiable() {
        let pairs = [
            ClockAnchorPairV0 {
                anchor_id: 0,
                alpha_local_time: 100,
                beta_local_time: 50,
            },
            ClockAnchorPairV0 {
                anchor_id: 1,
                alpha_local_time: 200,
                beta_local_time: 50,
            },
        ];
        assert_eq!(
            fit_affine_ols_v0(&pairs),
            Err(ClockFitErrorV0::NonIdentifiableTimebase)
        );
    }

    #[test]
    fn duplicate_anchor_identity_is_rejected() {
        let mut pairs = anchors(0, 1, 1);
        pairs[1].anchor_id = pairs[0].anchor_id;
        assert_eq!(
            fit_affine_ols_v0(&pairs),
            Err(ClockFitErrorV0::DuplicateAnchorId { anchor_id: 0 })
        );
    }

    #[test]
    fn robust_fit_is_invariant_to_anchor_input_order() {
        let mut pairs = anchors(10, 5, 4);
        pairs[6].alpha_local_time -= 300;
        let forward = fit_affine_huber_irls_v0(&pairs, 50, 1.0e-10).unwrap();
        pairs.reverse();
        let reversed = fit_affine_huber_irls_v0(&pairs, 50, 1.0e-10).unwrap();
        assert!((forward.offset - reversed.offset).abs() < 1.0e-7);
        assert!((forward.scale - reversed.scale).abs() < 1.0e-9);
    }

    #[test]
    fn centered_fit_preserves_rate_at_large_timestamp_epochs() {
        let beta_origin = 8_000_000_000_000_000_000_i64;
        let alpha_origin = -8_000_000_000_000_000_000_i64;
        let pairs = (0..9)
            .map(|index| {
                let delta = index * 1_000;
                ClockAnchorPairV0 {
                    anchor_id: u16::try_from(index).unwrap_or(u16::MAX),
                    beta_local_time: beta_origin + delta,
                    alpha_local_time: alpha_origin + delta * 2,
                }
            })
            .collect::<Vec<_>>();
        let fit = fit_affine_ols_v0(&pairs).unwrap();
        assert!((fit.scale - 2.0).abs() < 1.0e-12);
        assert!(fit.residual_rms < 1.0e-9);
        assert_eq!(fit.beta_reference_time, beta_origin);
        assert_eq!(fit.alpha_reference_time, alpha_origin);
        assert!((fit.predict_alpha_delta_v0(beta_origin + 4_000) - 8_000.0).abs() < 1.0e-9);
    }

    #[test]
    fn invalid_huber_configuration_fails_closed() {
        let pairs = anchors(0, 1, 1);
        assert_eq!(
            fit_affine_huber_irls_v0(&pairs, 0, 1.0e-6),
            Err(ClockFitErrorV0::InvalidIterationCount)
        );
        assert_eq!(
            fit_affine_huber_irls_v0(&pairs, 20, f64::NAN),
            Err(ClockFitErrorV0::InvalidTolerance)
        );
    }
}
