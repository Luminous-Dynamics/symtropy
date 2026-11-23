// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Interval-certified orbital center-of-linearity (CoL) energy path limiter.
//!
//! This module isolates the energy path used by NASA CARA PeakOverlapPos.m:
//!
//!   r(x) = r_A + x * (r_B - r_A)
//!   v(x) = v_u + beta * (r(x) - r_u)
//!   E(x) = 0.5 * ||v(x)||^2 - GM / ||r(x)||
//!
//! Unlike the NASA reference's coarse-grid minimization, this implementation
//! certifies a chronological prefix of the complete x in [0,1] path using
//! outward-widened interval arithmetic. It will not skip an earlier unsafe or
//! unresolved interval to select a later safe point.
//!
//! A successful full-step result proves, for this interval implementation,
//! that every path fraction satisfies the declared relative-energy-change
//! limit and remains bound. A reduced result identifies a certified prefix and
//! a first boundary/violation bracket. Resource exhaustion and unresolved
//! singularities produce explicit abstention.
//!
//! This is a numerical interval kernel, not a formally machine-verified
//! theorem and not a full NASA iterative PeakOverlapPos implementation.

use crate::KernelError;

/// Inputs defining a conditional-velocity CoL interpolation path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColEnergyGuardInputs {
    /// Initial CoL position r_A, in the declared length unit.
    pub initial_col_position: [f64; 3],
    /// Proposed target CoL position r_B, in the same frame and unit.
    pub target_col_position: [f64; 3],
    /// Conditional Gaussian mean position r_u.
    pub conditional_mean_position: [f64; 3],
    /// Conditional Gaussian mean velocity v_u.
    pub conditional_mean_velocity: [f64; 3],
    /// Conditional velocity regression beta = B*A^-1.
    pub conditional_velocity_regression: [[f64; 3]; 3],
    /// Gravitational parameter GM in length^3 / time^2.
    pub gravitational_parameter: f64,
    /// Maximum allowed relative change in specific orbital energy.
    ///
    /// The path is safe only when |E(x)-E(0)| <= tolerance*|E(0)|.
    pub maximum_relative_energy_change: f64,
}

/// Resource and resolution limits for the chronological interval search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColEnergyGuardConfig {
    /// Maximum number of interval boxes evaluated over the full path.
    pub maximum_interval_evaluations: usize,
    /// Minimum width of an unresolved boundary bracket in x.
    pub minimum_fraction_bracket_width: f64,
}

impl Default for ColEnergyGuardConfig {
    fn default() -> Self {
        Self {
            maximum_interval_evaluations: 4096,
            minimum_fraction_bracket_width: 1.0e-6,
        }
    }
}

/// Disposition of the chronological CoL energy-path qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColEnergyGuardDisposition {
    /// The entire interpolation path [0,1] was interval-certified safe.
    FullStepCertified,
    /// A nonzero prefix was certified, and a later boundary/violation bracket was found.
    ReducedPrefixCertified,
    /// The interval budget was exhausted before the first boundary was resolved.
    AbstainedBudget,
    /// The boundary remained ambiguous at the requested fraction resolution.
    AbstainedUnresolved,
}

/// Evidence returned by the interval-certified CoL energy path limiter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColEnergyGuardReport {
    /// Certified safe prefix endpoint. Every x in [0, this value] was certified.
    pub certified_safe_prefix: f64,
    /// First interval not certified safe; absent only when the full path passed.
    pub first_boundary_bracket: Option<[f64; 2]>,
    /// Outward enclosure of the reference specific energy at x=0.
    pub reference_energy_enclosure: [f64; 2],
    /// Number of interval boxes evaluated.
    pub interval_evaluations: usize,
    /// Final qualification disposition.
    pub disposition: ColEnergyGuardDisposition,
}

/// Certify the largest chronological prefix of a CoL interpolation path.
///
/// Safety requires both:
/// - the whole interval's specific orbital energy is strictly negative; and
/// - every value in the interval satisfies the declared relative energy-change
///   limit against the x=0 reference energy.
///
/// Subintervals are processed left-to-right. An unsafe or unresolved interval
/// terminates the prefix; later safe islands are never selected across it.
pub fn certify_col_energy_path(
    inputs: ColEnergyGuardInputs,
    config: ColEnergyGuardConfig,
) -> Result<ColEnergyGuardReport, KernelError> {
    validate_inputs(inputs)?;
    if config.maximum_interval_evaluations == 0
        || !config.minimum_fraction_bracket_width.is_finite()
        || config.minimum_fraction_bracket_width <= 0.0
        || config.minimum_fraction_bracket_width >= 1.0
    {
        return Err(KernelError::InvalidColEnergyGuardConfig);
    }

    let reference_energy = energy_enclosure(inputs, 0.0, 0.0)
        .ok_or(KernelError::NumericalOverflow)?;
    if reference_energy.upper >= 0.0 {
        return Err(KernelError::UnboundReferenceCoL);
    }

    let mut state = SearchState {
        evaluations: 0,
        maximum_evaluations: config.maximum_interval_evaluations,
        minimum_width: config.minimum_fraction_bracket_width,
        reference_energy,
        tolerance: inputs.maximum_relative_energy_change,
    };

    let result = certify_prefix(&inputs, &mut state, 0.0, 1.0);
    match result {
        PrefixResult::Safe => Ok(ColEnergyGuardReport {
            certified_safe_prefix: 1.0,
            first_boundary_bracket: None,
            reference_energy_enclosure: [reference_energy.lower, reference_energy.upper],
            interval_evaluations: state.evaluations,
            disposition: ColEnergyGuardDisposition::FullStepCertified,
        }),
        PrefixResult::Stopped {
            safe_prefix,
            bracket,
            reason,
        } => Ok(ColEnergyGuardReport {
            certified_safe_prefix: safe_prefix,
            first_boundary_bracket: Some(bracket),
            reference_energy_enclosure: [reference_energy.lower, reference_energy.upper],
            interval_evaluations: state.evaluations,
            disposition: match reason {
                StopReason::Violation => ColEnergyGuardDisposition::ReducedPrefixCertified,
                StopReason::Budget => ColEnergyGuardDisposition::AbstainedBudget,
                StopReason::Unresolved => ColEnergyGuardDisposition::AbstainedUnresolved,
            },
        }),
    }
}

#[derive(Debug, Clone, Copy)]
struct SearchState {
    evaluations: usize,
    maximum_evaluations: usize,
    minimum_width: f64,
    reference_energy: Enclosure,
    tolerance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    Violation,
    Budget,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PrefixResult {
    Safe,
    Stopped {
        safe_prefix: f64,
        bracket: [f64; 2],
        reason: StopReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Enclosure {
    lower: f64,
    upper: f64,
}

impl Enclosure {
    fn point(value: f64) -> Option<Self> {
        widen(value, value)
    }

    fn add(self, other: Self) -> Option<Self> {
        widen(self.lower + other.lower, self.upper + other.upper)
    }

    fn subtract(self, other: Self) -> Option<Self> {
        widen(self.lower - other.upper, self.upper - other.lower)
    }

    fn scale(self, factor: f64) -> Option<Self> {
        if !factor.is_finite() {
            return None;
        }
        if factor >= 0.0 {
            widen(self.lower * factor, self.upper * factor)
        } else {
            widen(self.upper * factor, self.lower * factor)
        }
    }

    fn multiply(self, other: Self) -> Option<Self> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if !products.iter().all(|value| value.is_finite()) {
            return None;
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        widen(lower, upper)
    }

    fn square(self) -> Option<Self> {
        let maximum = self.lower.abs().max(self.upper.abs());
        let upper = maximum * maximum;
        if !upper.is_finite() {
            return None;
        }
        let lower = if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            let minimum = self.lower.abs().min(self.upper.abs());
            minimum * minimum
        };
        widen(lower.max(0.0), upper)
    }

    fn sqrt_nonnegative(self) -> Option<Self> {
        if self.lower < 0.0 {
            return None;
        }
        let lower = self.lower.sqrt();
        let upper = self.upper.sqrt();
        widen(lower, upper)
    }

    fn reciprocal_positive(self) -> Option<Self> {
        if self.lower <= 0.0 || self.upper <= 0.0 {
            return None;
        }
        widen(1.0 / self.upper, 1.0 / self.lower)
    }

    fn minimum_absolute(self) -> f64 {
        if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            self.lower.abs().min(self.upper.abs())
        }
    }

    fn maximum_absolute(self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }
}

fn widen(lower: f64, upper: f64) -> Option<Enclosure> {
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return None;
    }
    let widened_lower = next_down(lower);
    let widened_upper = next_up(upper);
    if !widened_lower.is_finite() || !widened_upper.is_finite() {
        None
    } else {
        Some(Enclosure {
            lower: widened_lower,
            upper: widened_upper,
        })
    }
}

fn validate_inputs(inputs: ColEnergyGuardInputs) -> Result<(), KernelError> {
    if !inputs.gravitational_parameter.is_finite()
        || inputs.gravitational_parameter <= 0.0
        || !inputs.maximum_relative_energy_change.is_finite()
        || inputs.maximum_relative_energy_change <= 0.0
    {
        return Err(KernelError::InvalidColEnergyGuardInputs);
    }
    let vectors = [
        inputs.initial_col_position,
        inputs.target_col_position,
        inputs.conditional_mean_position,
        inputs.conditional_mean_velocity,
    ];
    if !vectors.iter().flatten().all(|value| value.is_finite())
        || !inputs
            .conditional_velocity_regression
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    {
        return Err(KernelError::NonFiniteValue);
    }
    Ok(())
}

fn energy_enclosure(
    inputs: ColEnergyGuardInputs,
    x_lower: f64,
    x_upper: f64,
) -> Option<Enclosure> {
    if !x_lower.is_finite()
        || !x_upper.is_finite()
        || x_lower < 0.0
        || x_upper > 1.0
        || x_lower > x_upper
    {
        return None;
    }
    let x = Enclosure {
        lower: next_down(x_lower),
        upper: next_up(x_upper),
    };
    if !x.lower.is_finite() || !x.upper.is_finite() {
        return None;
    }

    let mut position = [Enclosure { lower: 0.0, upper: 0.0 }; 3];
    let mut velocity = [Enclosure { lower: 0.0, upper: 0.0 }; 3];

    for i in 0..3 {
        let initial = Enclosure::point(inputs.initial_col_position[i])?;
        let target = Enclosure::point(inputs.target_col_position[i])?;
        let delta = target.subtract(initial)?;
        position[i] = initial.add(x.multiply(delta)?)?;
    }

    for i in 0..3 {
        let mut value = Enclosure::point(inputs.conditional_mean_velocity[i])?;
        for j in 0..3 {
            let beta = Enclosure::point(inputs.conditional_velocity_regression[i][j])?;
            let pos_offset = position[j]
                .subtract(Enclosure::point(inputs.conditional_mean_position[j])?)?;
            value = value.add(beta.multiply(pos_offset)?)?;
        }
        velocity[i] = value;
    }

    let mut speed_squared = Enclosure { lower: 0.0, upper: 0.0 };
    let mut radius_squared = Enclosure { lower: 0.0, upper: 0.0 };
    for i in 0..3 {
        speed_squared = speed_squared.add(velocity[i].square()?)?;
        radius_squared = radius_squared.add(position[i].square()?)?;
    }
    let kinetic = speed_squared.scale(0.5)?;
    let radius = radius_squared.sqrt_nonnegative()?;
    let inverse_radius = radius.reciprocal_positive()?;
    let potential = inverse_radius.scale(-inputs.gravitational_parameter)?;
    kinetic.add(potential)
}

fn certify_prefix(
    inputs: &ColEnergyGuardInputs,
    state: &mut SearchState,
    lower: f64,
    upper: f64,
) -> PrefixResult {
    if state.evaluations >= state.maximum_evaluations {
        return PrefixResult::Stopped {
            safe_prefix: lower,
            bracket: [lower, upper],
            reason: StopReason::Budget,
        };
    }
    state.evaluations += 1;

    if let Some(energy) = energy_enclosure(*inputs, lower, upper) {
        match classify_energy_interval(energy, state.reference_energy, state.tolerance) {
            IntervalSafety::Safe => return PrefixResult::Safe,
            IntervalSafety::DefinitelyUnsafe => {
                return PrefixResult::Stopped {
                    safe_prefix: lower,
                    bracket: [lower, upper],
                    reason: StopReason::Violation,
                };
            }
            IntervalSafety::Unresolved => {}
        }
    }

    if upper - lower <= state.minimum_width {
        return PrefixResult::Stopped {
            safe_prefix: lower,
            bracket: [lower, upper],
            reason: StopReason::Unresolved,
        };
    }

    let midpoint = 0.5 * (lower + upper);
    if midpoint <= lower || midpoint >= upper {
        return PrefixResult::Stopped {
            safe_prefix: lower,
            bracket: [lower, upper],
            reason: StopReason::Unresolved,
        };
    }

    match certify_prefix(inputs, state, lower, midpoint) {
        PrefixResult::Safe => certify_prefix(inputs, state, midpoint, upper),
        stopped => stopped,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntervalSafety {
    Safe,
    DefinitelyUnsafe,
    Unresolved,
}

fn classify_energy_interval(
    energy: Enclosure,
    reference: Enclosure,
    relative_tolerance: f64,
) -> IntervalSafety {
    if energy.lower >= 0.0 {
        return IntervalSafety::DefinitelyUnsafe;
    }

    let difference = match energy.subtract(reference) {
        Some(value) => value,
        None => return IntervalSafety::Unresolved,
    };
    let reference_abs_min = next_down(-reference.upper).max(0.0);
    let reference_abs_max = next_up(-reference.lower).max(0.0);
    let tolerance_min = match Enclosure::point(relative_tolerance)
        .and_then(|value| value.scale(reference_abs_min))
    {
        Some(value) => value.lower.max(0.0),
        None => return IntervalSafety::Unresolved,
    };
    let tolerance_max = match Enclosure::point(relative_tolerance)
        .and_then(|value| value.scale(reference_abs_max))
    {
        Some(value) => value.upper.max(0.0),
        None => return IntervalSafety::Unresolved,
    };

    let maximum_difference = difference.maximum_absolute();
    let minimum_difference = difference.minimum_absolute();

    if energy.upper < 0.0 && maximum_difference <= tolerance_min {
        IntervalSafety::Safe
    } else if minimum_difference > tolerance_max {
        IntervalSafety::DefinitelyUnsafe
    } else {
        IntervalSafety::Unresolved
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constant_bound_path() -> ColEnergyGuardInputs {
        ColEnergyGuardInputs {
            initial_col_position: [7.0e6, 0.0, 0.0],
            target_col_position: [7.0e6, 0.0, 0.0],
            conditional_mean_position: [7.0e6, 0.0, 0.0],
            conditional_mean_velocity: [0.0; 3],
            conditional_velocity_regression: [[0.0; 3]; 3],
            gravitational_parameter: 3.986004418e14,
            maximum_relative_energy_change: 0.1,
        }
    }

    #[test]
    fn constant_bound_path_certifies_full_step() {
        let report = certify_col_energy_path(
            constant_bound_path(),
            ColEnergyGuardConfig::default(),
        )
        .expect("valid bound path");
        assert_eq!(report.disposition, ColEnergyGuardDisposition::FullStepCertified);
        assert_eq!(report.certified_safe_prefix, 1.0);
        assert!(report.first_boundary_bracket.is_none());
        assert!(report.reference_energy_enclosure[1] < 0.0);
    }

    #[test]
    fn energy_change_boundary_returns_chronological_safe_prefix() {
        let mut inputs = constant_bound_path();
        inputs.initial_col_position = [10.0e6, 0.0, 0.0];
        inputs.target_col_position = [1.0e6, 0.0, 0.0];
        inputs.conditional_mean_position = [10.0e6, 0.0, 0.0];
        inputs.gravitational_parameter = 1.0;
        inputs.maximum_relative_energy_change = 0.2;
        let report = certify_col_energy_path(inputs, ColEnergyGuardConfig::default())
            .expect("valid bound path");
        assert!(report.certified_safe_prefix > 0.18);
        assert!(report.certified_safe_prefix < 0.19);
        assert!(report.first_boundary_bracket.is_some());
        assert_eq!(
            report.disposition,
            ColEnergyGuardDisposition::ReducedPrefixCertified
        );
    }

    #[test]
    fn path_that_becomes_unbound_is_stopped_before_first_unbound_region() {
        let mut inputs = constant_bound_path();
        inputs.initial_col_position = [2.0, 0.0, 0.0];
        inputs.target_col_position = [20.0, 0.0, 0.0];
        inputs.conditional_mean_position = [2.0, 0.0, 0.0];
        inputs.conditional_mean_velocity = [0.9, 0.0, 0.0];
        inputs.gravitational_parameter = 1.0;
        inputs.maximum_relative_energy_change = 100.0;
        let report = certify_col_energy_path(inputs, ColEnergyGuardConfig::default())
            .expect("initial path is bound");
        assert!(report.certified_safe_prefix < 1.0);
        assert_eq!(
            report.disposition,
            ColEnergyGuardDisposition::ReducedPrefixCertified
        );
    }

    #[test]
    fn a_later_safe_island_cannot_be_selected_across_an_earlier_violation() {
        let mut inputs = constant_bound_path();
        inputs.initial_col_position = [10.0e6, 0.0, 0.0];
        inputs.target_col_position = [1.0e6, 0.0, 0.0];
        inputs.conditional_mean_position = [10.0e6, 0.0, 0.0];
        inputs.gravitational_parameter = 1.0;
        inputs.maximum_relative_energy_change = 0.2;
        let report = certify_col_energy_path(inputs, ColEnergyGuardConfig::default())
            .expect("valid path");
        let bracket = report.first_boundary_bracket.expect("boundary bracket");
        assert!(bracket[0] <= bracket[1]);
        assert!(report.certified_safe_prefix <= bracket[0]);
    }

    #[test]
    fn interval_budget_exhaustion_abstains() {
        let mut inputs = constant_bound_path();
        inputs.initial_col_position = [10.0e6, 0.0, 0.0];
        inputs.target_col_position = [1.0e6, 0.0, 0.0];
        inputs.conditional_mean_position = [10.0e6, 0.0, 0.0];
        inputs.gravitational_parameter = 1.0;
        inputs.maximum_relative_energy_change = 0.2;
        let report = certify_col_energy_path(
            inputs,
            ColEnergyGuardConfig {
                maximum_interval_evaluations: 1,
                minimum_fraction_bracket_width: 1.0e-6,
            },
        )
        .expect("valid path");
        assert_eq!(report.disposition, ColEnergyGuardDisposition::AbstainedBudget);
        assert!(report.certified_safe_prefix <= 1.0);
    }

    #[test]
    fn a_path_through_the_central_singularity_is_not_certified() {
        let mut inputs = constant_bound_path();
        inputs.initial_col_position = [1.0, 0.0, 0.0];
        inputs.target_col_position = [-1.0, 0.0, 0.0];
        inputs.conditional_mean_position = [1.0, 0.0, 0.0];
        inputs.gravitational_parameter = 1.0;
        inputs.maximum_relative_energy_change = 10.0;
        let report = certify_col_energy_path(inputs, ColEnergyGuardConfig::default())
            .expect("reference state is bound");
        assert_ne!(report.disposition, ColEnergyGuardDisposition::FullStepCertified);
    }

    #[test]
    fn unbound_reference_state_is_rejected() {
        let mut inputs = constant_bound_path();
        inputs.conditional_mean_velocity = [2.0e4, 0.0, 0.0];
        assert_eq!(
            certify_col_energy_path(inputs, ColEnergyGuardConfig::default()),
            Err(KernelError::UnboundReferenceCoL)
        );
    }

    #[test]
    fn invalid_budget_tolerance_and_gravitational_parameter_fail_closed() {
        let inputs = constant_bound_path();
        assert_eq!(
            certify_col_energy_path(
                inputs,
                ColEnergyGuardConfig {
                    maximum_interval_evaluations: 0,
                    minimum_fraction_bracket_width: 1.0e-6,
                }
            ),
            Err(KernelError::InvalidColEnergyGuardConfig)
        );

        let mut invalid = inputs;
        invalid.gravitational_parameter = 0.0;
        assert_eq!(
            certify_col_energy_path(invalid, ColEnergyGuardConfig::default()),
            Err(KernelError::InvalidColEnergyGuardInputs)
        );
    }
}
