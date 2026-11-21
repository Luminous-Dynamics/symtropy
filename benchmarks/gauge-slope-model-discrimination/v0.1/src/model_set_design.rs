//! Robust separation of two competing slope-response families.
//!
//! The linear-speed model is a(v) = b_L + c_1 * v.
//! The quadratic-speed model is a(v) = b_Q + c_2 * v^2.
//!
//! Each model gets its own unrestricted constant intercept. The intercepts
//! are profiled out by weighted centering. The remaining separation is
//! minimized over predeclared coefficient intervals, returning the worst-case
//! witness. This is a design-discrimination reference under independent
//! Gaussian acceleration errors with known standard deviations; it is not a
//! posterior, p-value, expected information gain or causal attribution.

use super::{CompensatedSum, DesignCondition, ModelDiscriminationError, validate_sigma};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoefficientBounds {
    pub minimum: f64,
    pub maximum: f64,
}

impl CoefficientBounds {
    fn validate(self) -> Result<(), ModelDiscriminationError> {
        if !self.minimum.is_finite()
            || !self.maximum.is_finite()
            || self.minimum > self.maximum
        {
            return Err(ModelDiscriminationError::InvalidCoefficientBounds);
        }
        Ok(())
    }

    fn contains_zero(self) -> bool {
        self.minimum <= 0.0 && self.maximum >= 0.0
    }
}

/// Plausible coefficient ranges for the two competing response families.
///
/// c1 has units (m/s^2)/(m/s) = 1/s.
/// c2 has units (m/s^2)/(m/s)^2 = 1/m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RivalResponseCoefficientBounds {
    pub linear_speed: CoefficientBounds,
    pub quadratic_speed: CoefficientBounds,
}

/// Worst-case nuisance-intercept-profiled separation between the linear- and
/// quadratic-speed response families over the declared coefficient ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorstCaseModelSeparation {
    pub condition_count: usize,
    /// Minimum weighted squared response separation over the coefficient box.
    pub worst_case_squared_standardized_separation: f64,
    /// Coefficient witness attaining the returned minimum (within FP tolerance).
    pub witness_linear_speed_coefficient_per_s: f64,
    pub witness_quadratic_speed_coefficient_per_m: f64,
    /// Weighted centered sums retained for audit/replay.
    pub weighted_centered_speed_sum: f64,
    pub weighted_centered_speed_squared_sum: f64,
    pub weighted_speed_speed_squared_cross_sum: f64,
}

fn objective(
    centered_speed: &[f64],
    centered_speed_squared: &[f64],
    weights: &[f64],
    linear: f64,
    quadratic: f64,
) -> Result<f64, ModelDiscriminationError> {
    let mut sum = CompensatedSum::default();
    for ((speed, speed_squared), weight) in centered_speed
        .iter()
        .zip(centered_speed_squared.iter())
        .zip(weights.iter())
    {
        let difference = linear * speed - quadratic * speed_squared;
        sum.add(weight * difference * difference);
    }
    let value = sum.value();
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(ModelDiscriminationError::NonFiniteArithmetic)
    }
}

fn consider_candidate(
    best: &mut Option<(f64, f64, f64)>,
    centered_speed: &[f64],
    centered_speed_squared: &[f64],
    weights: &[f64],
    linear: f64,
    quadratic: f64,
) -> Result<(), ModelDiscriminationError> {
    if !linear.is_finite() || !quadratic.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }
    let value = objective(
        centered_speed,
        centered_speed_squared,
        weights,
        linear,
        quadratic,
    )?;
    if best.as_ref().is_none_or(|(best_value, _, _)| value < *best_value) {
        *best = Some((value, linear, quadratic));
    }
    Ok(())
}

/// Compute the robust minimum separation between:
///
/// - a_L(v) = b_L + c_1 * v, and
/// - a_Q(v) = b_Q + c_2 * v^2.
///
/// The independent nuisance intercepts b_L and b_Q are profiled out by
/// centering v and v^2 under their weighted means. The coefficient pair is
/// then minimized over the caller's closed, predeclared rectangular bounds.
///
/// A zero score means the proposed design cannot robustly distinguish the
/// two model families within those coefficient ranges. This can occur for
/// two-speed designs even when both coefficients are non-zero.
pub fn score_linear_vs_quadratic_speed_models(
    conditions: &[DesignCondition],
    bounds: RivalResponseCoefficientBounds,
) -> Result<WorstCaseModelSeparation, ModelDiscriminationError> {
    if conditions.len() < 2 {
        return Err(ModelDiscriminationError::FewerThanTwoDesignConditions);
    }
    bounds.linear_speed.validate()?;
    bounds.quadratic_speed.validate()?;

    let mut weights = Vec::with_capacity(conditions.len());
    let mut sum_weight = CompensatedSum::default();
    let mut sum_weighted_speed = CompensatedSum::default();
    let mut sum_weighted_speed_squared = CompensatedSum::default();

    for condition in conditions {
        if !condition.speed_m_s.is_finite() || condition.speed_m_s < 0.0 {
            return Err(ModelDiscriminationError::InvalidSpeed);
        }
        let weight = validate_sigma(condition.acceleration_sigma_m_s2)?;
        let speed_squared = condition.speed_m_s * condition.speed_m_s;
        if !speed_squared.is_finite() {
            return Err(ModelDiscriminationError::NonFiniteArithmetic);
        }
        weights.push(weight);
        sum_weight.add(weight);
        sum_weighted_speed.add(weight * condition.speed_m_s);
        sum_weighted_speed_squared.add(weight * speed_squared);
    }

    let total_weight = sum_weight.value();
    let weighted_mean_speed = sum_weighted_speed.value() / total_weight;
    let weighted_mean_speed_squared = sum_weighted_speed_squared.value() / total_weight;
    if !total_weight.is_finite()
        || total_weight <= 0.0
        || !weighted_mean_speed.is_finite()
        || !weighted_mean_speed_squared.is_finite()
    {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let mut centered_speed = Vec::with_capacity(conditions.len());
    let mut centered_speed_squared = Vec::with_capacity(conditions.len());
    let mut sum_xx = CompensatedSum::default();
    let mut sum_xz = CompensatedSum::default();
    let mut sum_zz = CompensatedSum::default();

    for (condition, weight) in conditions.iter().zip(weights.iter().copied()) {
        let speed = condition.speed_m_s - weighted_mean_speed;
        let speed_squared =
            condition.speed_m_s * condition.speed_m_s - weighted_mean_speed_squared;
        centered_speed.push(speed);
        centered_speed_squared.push(speed_squared);
        sum_xx.add(weight * speed * speed);
        sum_xz.add(weight * speed * speed_squared);
        sum_zz.add(weight * speed_squared * speed_squared);
    }

    let xx = sum_xx.value();
    let xz = sum_xz.value();
    let zz = sum_zz.value();
    let speed_scale = total_weight * weighted_mean_speed.abs().max(1.0).powi(2);
    let speed_squared_scale =
        total_weight * weighted_mean_speed_squared.abs().max(1.0).powi(2);
    if !xx.is_finite()
        || !xz.is_finite()
        || !zz.is_finite()
        || xx <= f64::EPSILON * speed_scale
        || zz <= f64::EPSILON * speed_squared_scale
    {
        return Err(ModelDiscriminationError::InsufficientSpeedExcitation);
    }

    let mut best = None;

    // The unconstrained stationary point of this homogeneous quadratic is
    // (0, 0). Include it explicitly when the coefficient box contains it.
    if bounds.linear_speed.contains_zero() && bounds.quadratic_speed.contains_zero() {
        consider_candidate(
            &mut best,
            &centered_speed,
            &centered_speed_squared,
            &weights,
            0.0,
            0.0,
        )?;
    }

    // A convex quadratic on a rectangle attains its constrained minimum at
    // an interior stationary point or on the boundary. The only interior
    // stationary point is the origin, handled above. Minimize both pairs of
    // opposing edges analytically, clamping each edge's stationary point.
    for linear in [
        bounds.linear_speed.minimum,
        bounds.linear_speed.maximum,
    ] {
        let quadratic = (xz * linear / zz)
            .clamp(bounds.quadratic_speed.minimum, bounds.quadratic_speed.maximum);
        consider_candidate(
            &mut best,
            &centered_speed,
            &centered_speed_squared,
            &weights,
            linear,
            quadratic,
        )?;
    }

    for quadratic in [
        bounds.quadratic_speed.minimum,
        bounds.quadratic_speed.maximum,
    ] {
        let linear = (xz * quadratic / xx)
            .clamp(bounds.linear_speed.minimum, bounds.linear_speed.maximum);
        consider_candidate(
            &mut best,
            &centered_speed,
            &centered_speed_squared,
            &weights,
            linear,
            quadratic,
        )?;
    }

    let (separation, linear_witness, quadratic_witness) =
        best.ok_or(ModelDiscriminationError::NonFiniteArithmetic)?;

    Ok(WorstCaseModelSeparation {
        condition_count: conditions.len(),
        worst_case_squared_standardized_separation: separation,
        witness_linear_speed_coefficient_per_s: linear_witness,
        witness_quadratic_speed_coefficient_per_m: quadratic_witness,
        weighted_centered_speed_sum: xx,
        weighted_centered_speed_squared_sum: zz,
        weighted_speed_speed_squared_cross_sum: xz,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(speed_m_s: f64, sigma: f64) -> DesignCondition {
        DesignCondition {
            speed_m_s,
            acceleration_sigma_m_s2: sigma,
        }
    }

    fn exact_bounds(linear: f64, quadratic: f64) -> RivalResponseCoefficientBounds {
        RivalResponseCoefficientBounds {
            linear_speed: CoefficientBounds {
                minimum: linear,
                maximum: linear,
            },
            quadratic_speed: CoefficientBounds {
                minimum: quadratic,
                maximum: quadratic,
            },
        }
    }

    #[test]
    fn two_speed_design_can_make_linear_and_quadratic_models_equivalent() {
        // At v={0.5, 1.5}, c1=0.25 and c2=0.125 satisfy
        // c1*(v - mean(v)) = c2*(v^2 - mean(v^2)).
        let design = [condition(0.5, 0.02), condition(1.5, 0.02)];
        let result =
            score_linear_vs_quadratic_speed_models(&design, exact_bounds(0.25, 0.125))
                .unwrap();
        assert!(result.worst_case_squared_standardized_separation < 1.0e-20);
        assert!((result.witness_linear_speed_coefficient_per_s - 0.25).abs() < 1.0e-12);
        assert!((result.witness_quadratic_speed_coefficient_per_m - 0.125).abs() < 1.0e-12);
    }

    #[test]
    fn three_distinct_speeds_can_separate_fixed_rival_models() {
        let design = [
            condition(0.5, 0.02),
            condition(1.0, 0.02),
            condition(1.5, 0.02),
        ];
        let result =
            score_linear_vs_quadratic_speed_models(&design, exact_bounds(0.25, 0.125))
                .unwrap();
        assert!(result.worst_case_squared_standardized_separation > 0.0);
    }

    #[test]
    fn coefficient_box_including_common_zero_returns_zero_worst_case() {
        let design = [
            condition(0.5, 0.02),
            condition(1.0, 0.02),
            condition(1.5, 0.02),
        ];
        let bounds = RivalResponseCoefficientBounds {
            linear_speed: CoefficientBounds {
                minimum: 0.0,
                maximum: 0.3,
            },
            quadratic_speed: CoefficientBounds {
                minimum: 0.0,
                maximum: 0.2,
            },
        };
        let result = score_linear_vs_quadratic_speed_models(&design, bounds).unwrap();
        assert_eq!(result.worst_case_squared_standardized_separation, 0.0);
        assert_eq!(result.witness_linear_speed_coefficient_per_s, 0.0);
        assert_eq!(result.witness_quadratic_speed_coefficient_per_m, 0.0);
    }

    #[test]
    fn wider_speed_range_increases_fixed_hypothesis_separation_at_equal_budget() {
        let narrow = [
            condition(0.5, 0.02),
            condition(1.0, 0.02),
            condition(1.5, 0.02),
        ];
        let wide = [
            condition(0.25, 0.02),
            condition(1.0, 0.02),
            condition(2.5, 0.02),
        ];
        let bounds = exact_bounds(0.25, 0.125);
        let narrow_score = score_linear_vs_quadratic_speed_models(&narrow, bounds).unwrap();
        let wide_score = score_linear_vs_quadratic_speed_models(&wide, bounds).unwrap();
        assert!(wide_score.worst_case_squared_standardized_separation
            > narrow_score.worst_case_squared_standardized_separation);
        assert_eq!(wide_score.condition_count, narrow_score.condition_count);
    }

    #[test]
    fn doubling_all_acceleration_sigmas_quarters_the_separation() {
        let design = [
            condition(0.5, 0.02),
            condition(1.0, 0.02),
            condition(2.0, 0.02),
        ];
        let doubled = design.map(|item| DesignCondition {
            acceleration_sigma_m_s2: item.acceleration_sigma_m_s2 * 2.0,
            ..item
        });
        let bounds = exact_bounds(0.25, 0.125);
        let original = score_linear_vs_quadratic_speed_models(&design, bounds).unwrap();
        let noisier = score_linear_vs_quadratic_speed_models(&doubled, bounds).unwrap();
        assert!(
            (4.0 * noisier.worst_case_squared_standardized_separation
                - original.worst_case_squared_standardized_separation)
                .abs()
                < 1.0e-9
        );
    }

    #[test]
    fn invalid_coefficient_ranges_are_rejected() {
        let design = [condition(0.5, 0.02), condition(1.0, 0.02)];
        let mut bounds = exact_bounds(0.25, 0.125);
        bounds.linear_speed = CoefficientBounds {
            minimum: 1.0,
            maximum: -1.0,
        };
        assert_eq!(
            score_linear_vs_quadratic_speed_models(&design, bounds),
            Err(ModelDiscriminationError::InvalidCoefficientBounds)
        );
    }

    #[test]
    fn repeated_speed_conditions_have_no_model_separation_design_rank() {
        let design = [
            condition(1.0, 0.02),
            condition(1.0, 0.03),
            condition(1.0, 0.04),
        ];
        assert_eq!(
            score_linear_vs_quadratic_speed_models(&design, exact_bounds(0.25, 0.125)),
            Err(ModelDiscriminationError::InsufficientSpeedExcitation)
        );
    }

    #[test]
    fn input_permutation_preserves_the_worst_case_score() {
        let original = [
            condition(0.25, 0.02),
            condition(1.0, 0.03),
            condition(2.5, 0.025),
            condition(1.5, 0.02),
        ];
        let permuted = [original[3], original[1], original[0], original[2]];
        let bounds = RivalResponseCoefficientBounds {
            linear_speed: CoefficientBounds {
                minimum: 0.2,
                maximum: 0.3,
            },
            quadratic_speed: CoefficientBounds {
                minimum: 0.1,
                maximum: 0.15,
            },
        };
        let first = score_linear_vs_quadratic_speed_models(&original, bounds).unwrap();
        let second = score_linear_vs_quadratic_speed_models(&permuted, bounds).unwrap();
        assert!(
            (first.worst_case_squared_standardized_separation
                - second.worst_case_squared_standardized_separation)
                .abs()
                < 1.0e-9
        );
        assert!(
            (first.witness_linear_speed_coefficient_per_s
                - second.witness_linear_speed_coefficient_per_s)
                .abs()
                < 1.0e-9
        );
        assert!(
            (first.witness_quadratic_speed_coefficient_per_m
                - second.witness_quadratic_speed_coefficient_per_m)
                .abs()
                < 1.0e-9
        );
    }
}
