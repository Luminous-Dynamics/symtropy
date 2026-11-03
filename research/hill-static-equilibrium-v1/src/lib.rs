// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A bounded isometric Hill-type musculotendon equilibrium oracle.
//!
//! V1 includes active/passive fiber force-length curves, an elastic tendon
//! force-length curve, constant-thickness pennation geometry, first-order
//! activation/deactivation dynamics, and a bracketed equilibrium solver.
//!
//! V1 is deliberately static with respect to fiber velocity. It does not
//! implement the force-velocity curve, dynamic tendon/fiber integration,
//! moment arms, joint torques, or a complete Millard/OpenSim model.
//!
//! Curve and parameter digests are syntax-checked references supplied by an
//! external artifact authority. This crate does not hash/authenticate files.

#![forbid(unsafe_code)]

const MAX_CURVE_POINTS: usize = 4096;
const MAX_ROOT_ITERATIONS: u32 = 256;
const MAX_FIBER_LENGTH_RATIO: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveKind {
    ActiveForceLength,
    PassiveForceLength,
    TendonForceLength,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePoint {
    /// Dimensionless normalized muscle-fiber or tendon length.
    pub normalized_length: f64,
    /// Dimensionless normalized force.
    pub normalized_force: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampledCurve {
    pub kind: CurveKind,
    pub source_sha256: String,
    pub points: Vec<CurvePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MuscleCurves {
    pub active_force_length: SampledCurve,
    pub passive_force_length: SampledCurve,
    pub tendon_force_length: SampledCurve,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MuscleParameters {
    pub muscle_id: String,
    pub parameter_sha256: String,
    /// Maximum isometric muscle-fiber force in newtons.
    pub maximum_isometric_force_n: f64,
    /// Optimal fiber length in metres.
    pub optimal_fiber_length_m: f64,
    /// Tendon slack length in metres.
    pub tendon_slack_length_m: f64,
    /// Optimal pennation angle in radians.
    pub optimal_pennation_angle_rad: f64,
    /// Minimum and maximum fiber search lengths, normalized by optimal length.
    pub minimum_fiber_length_ratio: f64,
    pub maximum_fiber_length_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivationDynamics {
    pub activation_time_constant_s: f64,
    pub deactivation_time_constant_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticSolvePolicy {
    pub maximum_iterations: u32,
    pub fiber_length_tolerance_m: f64,
    pub force_residual_tolerance_n: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticEquilibriumResult {
    pub muscle_id: String,
    pub parameter_sha256: String,
    pub active_force_length_sha256: String,
    pub passive_force_length_sha256: String,
    pub tendon_force_length_sha256: String,
    pub musculotendon_length_m: f64,
    pub activation: f64,
    pub solve_policy: StaticSolvePolicy,
    pub fiber_length_m: f64,
    pub tendon_length_m: f64,
    pub normalized_fiber_length: f64,
    pub normalized_tendon_length: f64,
    pub pennation_angle_rad: f64,
    pub fiber_force_n: f64,
    pub tendon_force_n: f64,
    pub transmitted_fiber_force_n: f64,
    /// Transmitted fiber force minus tendon force.
    pub force_residual_n: f64,
    pub iterations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivationUpdate {
    pub previous_activation: f64,
    pub excitation: f64,
    pub next_activation: f64,
    pub time_constant_s: f64,
    pub time_step_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MuscleError {
    InvalidIdentifier,
    InvalidDigest,
    InvalidCurve(CurveKind),
    CurveKindMismatch,
    CurveOutOfDomain(CurveKind),
    InvalidMuscleParameters,
    InvalidActivation,
    InvalidTimeStep,
    InvalidActivationDynamics,
    InvalidSolvePolicy,
    InvalidMusculotendonLength,
    ImpossiblePennationGeometry,
    NonPositiveTendonLength,
    NonFiniteValue,
    NonUniqueEquilibrium,
    NoEquilibriumBracket {
        lower_residual_n: f64,
        upper_residual_n: f64,
    },
    IterationLimit {
        iterations: u32,
        last_residual_n: f64,
        bracket_width_m: f64,
    },
    ResidualToleranceUnmet {
        residual_n: f64,
        bracket_width_m: f64,
    },
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl SampledCurve {
    pub fn validate(&self, expected_kind: CurveKind) -> Result<(), MuscleError> {
        if self.kind != expected_kind {
            return Err(MuscleError::CurveKindMismatch);
        }
        if !valid_sha256(&self.source_sha256) {
            return Err(MuscleError::InvalidDigest);
        }
        if self.points.len() < 2 || self.points.len() > MAX_CURVE_POINTS {
            return Err(MuscleError::InvalidCurve(self.kind));
        }

        let mut previous_length = None;
        for point in &self.points {
            if !point.normalized_length.is_finite()
                || !point.normalized_force.is_finite()
                || point.normalized_length < 0.0
                || point.normalized_force < 0.0
            {
                return Err(MuscleError::InvalidCurve(self.kind));
            }
            if previous_length.is_some_and(|previous| point.normalized_length <= previous) {
                return Err(MuscleError::InvalidCurve(self.kind));
            }
            previous_length = Some(point.normalized_length);
        }

        Ok(())
    }

    /// Piecewise-linear interpolation. Extrapolation is deliberately rejected.
    pub fn evaluate(&self, normalized_length: f64) -> Result<f64, MuscleError> {
        if !normalized_length.is_finite() {
            return Err(MuscleError::NonFiniteValue);
        }
        self.validate(self.kind)?;

        let first = self.points.first().ok_or(MuscleError::InvalidCurve(self.kind))?;
        let last = self.points.last().ok_or(MuscleError::InvalidCurve(self.kind))?;
        if normalized_length < first.normalized_length
            || normalized_length > last.normalized_length
        {
            return Err(MuscleError::CurveOutOfDomain(self.kind));
        }

        match self.points.binary_search_by(|point| {
            point.normalized_length.total_cmp(&normalized_length)
        }) {
            Ok(index) => Ok(self.points[index].normalized_force),
            Err(upper_index) => {
                if upper_index == 0 || upper_index >= self.points.len() {
                    return Err(MuscleError::CurveOutOfDomain(self.kind));
                }
                let lower = self.points[upper_index - 1];
                let upper = self.points[upper_index];
                let fraction = (normalized_length - lower.normalized_length)
                    / (upper.normalized_length - lower.normalized_length);
                let force = lower.normalized_force
                    + fraction * (upper.normalized_force - lower.normalized_force);
                if force.is_finite() && force >= 0.0 {
                    Ok(force)
                } else {
                    Err(MuscleError::NonFiniteValue)
                }
            }
        }
    }
}

impl MuscleCurves {
    pub fn validate(&self) -> Result<(), MuscleError> {
        self.active_force_length
            .validate(CurveKind::ActiveForceLength)?;
        self.passive_force_length
            .validate(CurveKind::PassiveForceLength)?;
        self.tendon_force_length
            .validate(CurveKind::TendonForceLength)?;
        Ok(())
    }
}

impl MuscleParameters {
    pub fn validate(&self) -> Result<(), MuscleError> {
        if !valid_identifier(&self.muscle_id) {
            return Err(MuscleError::InvalidIdentifier);
        }
        if !valid_sha256(&self.parameter_sha256) {
            return Err(MuscleError::InvalidDigest);
        }

        let values = [
            self.maximum_isometric_force_n,
            self.optimal_fiber_length_m,
            self.tendon_slack_length_m,
            self.optimal_pennation_angle_rad,
            self.minimum_fiber_length_ratio,
            self.maximum_fiber_length_ratio,
        ];
        if values.iter().any(|value| !value.is_finite())
            || self.maximum_isometric_force_n <= 0.0
            || self.optimal_fiber_length_m <= 0.0
            || self.tendon_slack_length_m <= 0.0
            || self.optimal_pennation_angle_rad < 0.0
            || self.optimal_pennation_angle_rad >= std::f64::consts::FRAC_PI_2
            || self.minimum_fiber_length_ratio <= 0.0
            || self.maximum_fiber_length_ratio <= self.minimum_fiber_length_ratio
            || self.maximum_fiber_length_ratio > MAX_FIBER_LENGTH_RATIO
            || self.minimum_fiber_length_ratio
                <= self.optimal_pennation_angle_rad.sin()
            || !(self.optimal_fiber_length_m * self.maximum_fiber_length_ratio).is_finite()
        {
            return Err(MuscleError::InvalidMuscleParameters);
        }
        Ok(())
    }
}

impl ActivationDynamics {
    pub fn validate(&self) -> Result<(), MuscleError> {
        if !self.activation_time_constant_s.is_finite()
            || !self.deactivation_time_constant_s.is_finite()
            || self.activation_time_constant_s <= 0.0
            || self.deactivation_time_constant_s <= 0.0
        {
            return Err(MuscleError::InvalidActivationDynamics);
        }
        Ok(())
    }
}

impl StaticSolvePolicy {
    pub fn validate(&self) -> Result<(), MuscleError> {
        if self.maximum_iterations == 0
            || self.maximum_iterations > MAX_ROOT_ITERATIONS
            || !self.fiber_length_tolerance_m.is_finite()
            || self.fiber_length_tolerance_m <= 0.0
            || !self.force_residual_tolerance_n.is_finite()
            || self.force_residual_tolerance_n <= 0.0
        {
            return Err(MuscleError::InvalidSolvePolicy);
        }
        Ok(())
    }
}

/// Exact first-order update for constant excitation over one time interval.
///
/// The appropriate rise/fall time constant is selected from the direction of
/// the excitation change. The result is not Euler-step dependent.
pub fn update_activation(
    previous_activation: f64,
    excitation: f64,
    time_step_s: f64,
    dynamics: ActivationDynamics,
) -> Result<ActivationUpdate, MuscleError> {
    dynamics.validate()?;
    if !previous_activation.is_finite()
        || !excitation.is_finite()
        || !(0.0..=1.0).contains(&previous_activation)
        || !(0.0..=1.0).contains(&excitation)
    {
        return Err(MuscleError::InvalidActivation);
    }
    if !time_step_s.is_finite() || time_step_s <= 0.0 {
        return Err(MuscleError::InvalidTimeStep);
    }

    let time_constant_s = if excitation > previous_activation {
        dynamics.activation_time_constant_s
    } else {
        dynamics.deactivation_time_constant_s
    };
    let ratio = time_step_s / time_constant_s;
    let decay = if ratio.is_finite() {
        (-ratio).exp()
    } else {
        0.0
    };
    let next_activation = excitation + (previous_activation - excitation) * decay;
    if !next_activation.is_finite() || !(0.0..=1.0).contains(&next_activation) {
        return Err(MuscleError::NonFiniteValue);
    }

    Ok(ActivationUpdate {
        previous_activation,
        excitation,
        next_activation,
        time_constant_s,
        time_step_s,
    })
}

struct ForceEvaluation {
    tendon_length_m: f64,
    normalized_fiber_length: f64,
    normalized_tendon_length: f64,
    pennation_angle_rad: f64,
    fiber_force_n: f64,
    tendon_force_n: f64,
    transmitted_fiber_force_n: f64,
    residual_n: f64,
}

fn evaluate_residual(
    parameters: &MuscleParameters,
    curves: &MuscleCurves,
    musculotendon_length_m: f64,
    activation: f64,
    fiber_length_m: f64,
) -> Result<ForceEvaluation, MuscleError> {
    let normalized_fiber_length = fiber_length_m / parameters.optimal_fiber_length_m;
    let pennation_width_m = parameters.optimal_fiber_length_m
        * parameters.optimal_pennation_angle_rad.sin();
    let sine_pennation = pennation_width_m / fiber_length_m;
    if !sine_pennation.is_finite() || !(0.0..=1.0).contains(&sine_pennation) {
        return Err(MuscleError::ImpossiblePennationGeometry);
    }

    let cosine_pennation = (1.0 - sine_pennation * sine_pennation).sqrt();
    let pennation_angle_rad = sine_pennation.asin();
    let tendon_length_m = musculotendon_length_m - fiber_length_m * cosine_pennation;
    if !tendon_length_m.is_finite() || tendon_length_m <= 0.0 {
        return Err(MuscleError::NonPositiveTendonLength);
    }

    let normalized_tendon_length = tendon_length_m / parameters.tendon_slack_length_m;
    let active_factor = curves
        .active_force_length
        .evaluate(normalized_fiber_length)?;
    let passive_factor = curves
        .passive_force_length
        .evaluate(normalized_fiber_length)?;
    let tendon_factor = curves
        .tendon_force_length
        .evaluate(normalized_tendon_length)?;

    let fiber_force_n = parameters.maximum_isometric_force_n
        * (activation * active_factor + passive_factor);
    let tendon_force_n = parameters.maximum_isometric_force_n * tendon_factor;
    let transmitted_fiber_force_n = fiber_force_n * cosine_pennation;
    let residual_n = transmitted_fiber_force_n - tendon_force_n;

    if [
        fiber_force_n,
        tendon_force_n,
        transmitted_fiber_force_n,
        residual_n,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(MuscleError::NonFiniteValue);
    }

    Ok(ForceEvaluation {
        tendon_length_m,
        normalized_fiber_length,
        normalized_tendon_length,
        pennation_angle_rad,
        fiber_force_n,
        tendon_force_n,
        transmitted_fiber_force_n,
        residual_n,
    })
}

/// Solve isometric fiber–tendon force equilibrium over the declared fiber-length
/// interval using bounded bisection. The first bracketed root is returned; this
/// does not prove that a non-monotone curve set has a globally unique solution.
pub fn solve_static_equilibrium(
    parameters: &MuscleParameters,
    curves: &MuscleCurves,
    musculotendon_length_m: f64,
    activation: f64,
    policy: StaticSolvePolicy,
) -> Result<StaticEquilibriumResult, MuscleError> {
    parameters.validate()?;
    curves.validate()?;
    policy.validate()?;

    if !musculotendon_length_m.is_finite() || musculotendon_length_m <= 0.0 {
        return Err(MuscleError::InvalidMusculotendonLength);
    }
    if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
        return Err(MuscleError::InvalidActivation);
    }

    let mut lower_fiber_length_m =
        parameters.optimal_fiber_length_m * parameters.minimum_fiber_length_ratio;
    let mut upper_fiber_length_m =
        parameters.optimal_fiber_length_m * parameters.maximum_fiber_length_ratio;

    let mut lower_eval = evaluate_residual(
        parameters,
        curves,
        musculotendon_length_m,
        activation,
        lower_fiber_length_m,
    )?;
    let mut upper_eval = evaluate_residual(
        parameters,
        curves,
        musculotendon_length_m,
        activation,
        upper_fiber_length_m,
    )?;

    if lower_eval.residual_n.abs() <= policy.force_residual_tolerance_n
        && upper_eval.residual_n.abs() <= policy.force_residual_tolerance_n
    {
        let midpoint = lower_fiber_length_m
            + 0.5 * (upper_fiber_length_m - lower_fiber_length_m);
        let middle_eval = evaluate_residual(
            parameters,
            curves,
            musculotendon_length_m,
            activation,
            midpoint,
        )?;
        if middle_eval.residual_n.abs() <= policy.force_residual_tolerance_n {
            return Err(MuscleError::NonUniqueEquilibrium);
        }
    }

    if lower_eval.residual_n.abs() <= policy.force_residual_tolerance_n {
        return to_result(
            parameters,
            curves,
            musculotendon_length_m,
            activation,
            policy,
            lower_eval,
            lower_fiber_length_m,
            0,
        );
    }
    if upper_eval.residual_n.abs() <= policy.force_residual_tolerance_n {
        return to_result(
            parameters,
            curves,
            musculotendon_length_m,
            activation,
            policy,
            upper_eval,
            upper_fiber_length_m,
            0,
        );
    }

    if lower_eval.residual_n.signum() == upper_eval.residual_n.signum() {
        return Err(MuscleError::NoEquilibriumBracket {
            lower_residual_n: lower_eval.residual_n,
            upper_residual_n: upper_eval.residual_n,
        });
    }

    for iteration in 1..=policy.maximum_iterations {
        let midpoint = lower_fiber_length_m
            + 0.5 * (upper_fiber_length_m - lower_fiber_length_m);
        let middle_eval = evaluate_residual(
            parameters,
            curves,
            musculotendon_length_m,
            activation,
            midpoint,
        )?;

        if middle_eval.residual_n.abs() <= policy.force_residual_tolerance_n {
            return to_result(
                parameters,
                curves,
                musculotendon_length_m,
                activation,
                policy,
                middle_eval,
                midpoint,
                iteration,
            );
        }

        if middle_eval.residual_n.signum() == lower_eval.residual_n.signum() {
            lower_fiber_length_m = midpoint;
            lower_eval = middle_eval;
        } else {
            upper_fiber_length_m = midpoint;
            upper_eval = middle_eval;
        }

        let bracket_width_m = upper_fiber_length_m - lower_fiber_length_m;
        if bracket_width_m <= policy.fiber_length_tolerance_m {
            let final_midpoint = lower_fiber_length_m + 0.5 * bracket_width_m;
            let final_eval = evaluate_residual(
                parameters,
                curves,
                musculotendon_length_m,
                activation,
                final_midpoint,
            )?;
            if final_eval.residual_n.abs() <= policy.force_residual_tolerance_n {
                return to_result(
                    parameters,
                    curves,
                    musculotendon_length_m,
                    activation,
                    policy,
                    final_eval,
                    final_midpoint,
                    iteration,
                );
            }
            return Err(MuscleError::ResidualToleranceUnmet {
                residual_n: final_eval.residual_n,
                bracket_width_m,
            });
        }
    }

    Err(MuscleError::IterationLimit {
        iterations: policy.maximum_iterations,
        last_residual_n: 0.5 * (lower_eval.residual_n + upper_eval.residual_n),
        bracket_width_m: upper_fiber_length_m - lower_fiber_length_m,
    })
}

fn to_result(
    parameters: &MuscleParameters,
    curves: &MuscleCurves,
    musculotendon_length_m: f64,
    activation: f64,
    policy: StaticSolvePolicy,
    evaluation: ForceEvaluation,
    fiber_length_m: f64,
    iterations: u32,
) -> Result<StaticEquilibriumResult, MuscleError> {
    if [
        fiber_length_m,
        musculotendon_length_m,
        activation,
        evaluation.tendon_length_m,
        evaluation.normalized_fiber_length,
        evaluation.normalized_tendon_length,
        evaluation.pennation_angle_rad,
        evaluation.fiber_force_n,
        evaluation.tendon_force_n,
        evaluation.transmitted_fiber_force_n,
        evaluation.residual_n,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(MuscleError::NonFiniteValue);
    }

    Ok(StaticEquilibriumResult {
        muscle_id: parameters.muscle_id.clone(),
        parameter_sha256: parameters.parameter_sha256.clone(),
        active_force_length_sha256: curves.active_force_length.source_sha256.clone(),
        passive_force_length_sha256: curves.passive_force_length.source_sha256.clone(),
        tendon_force_length_sha256: curves.tendon_force_length.source_sha256.clone(),
        musculotendon_length_m,
        activation,
        solve_policy: policy,
        fiber_length_m,
        tendon_length_m: evaluation.tendon_length_m,
        normalized_fiber_length: evaluation.normalized_fiber_length,
        normalized_tendon_length: evaluation.normalized_tendon_length,
        pennation_angle_rad: evaluation.pennation_angle_rad,
        fiber_force_n: evaluation.fiber_force_n,
        tendon_force_n: evaluation.tendon_force_n,
        transmitted_fiber_force_n: evaluation.transmitted_fiber_force_n,
        force_residual_n: evaluation.residual_n,
        iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        "a".repeat(64)
    }

    fn curve(kind: CurveKind, points: &[(f64, f64)]) -> SampledCurve {
        SampledCurve {
            kind,
            source_sha256: digest(),
            points: points
                .iter()
                .map(|(normalized_length, normalized_force)| CurvePoint {
                    normalized_length: *normalized_length,
                    normalized_force: *normalized_force,
                })
                .collect(),
        }
    }

    fn curves() -> MuscleCurves {
        MuscleCurves {
            active_force_length: curve(
                CurveKind::ActiveForceLength,
                &[(0.5, 0.5), (0.8, 0.8), (1.0, 1.0), (1.2, 0.8), (1.5, 0.2)],
            ),
            passive_force_length: curve(
                CurveKind::PassiveForceLength,
                &[(0.5, 0.0), (1.0, 0.0), (1.5, 1.0), (2.0, 5.0)],
            ),
            tendon_force_length: curve(
                CurveKind::TendonForceLength,
                &[
                    (0.5, 0.0),
                    (0.9, 0.0),
                    (1.0, 0.0),
                    (1.05, 1.0),
                    (1.1, 2.0),
                    (1.5, 10.0),
                    (2.0, 25.0),
                ],
            ),
        }
    }

    fn parameters(pennation_angle_rad: f64) -> MuscleParameters {
        MuscleParameters {
            muscle_id: "foreleg-extensor-1".to_string(),
            parameter_sha256: digest(),
            maximum_isometric_force_n: 100.0,
            optimal_fiber_length_m: 0.01,
            tendon_slack_length_m: 0.01,
            optimal_pennation_angle_rad: pennation_angle_rad,
            minimum_fiber_length_ratio: 0.8,
            maximum_fiber_length_ratio: 1.2,
        }
    }

    fn policy() -> StaticSolvePolicy {
        StaticSolvePolicy {
            maximum_iterations: 128,
            fiber_length_tolerance_m: 1.0e-12,
            force_residual_tolerance_n: 1.0e-8,
        }
    }

    #[test]
    fn solves_known_answer_isometric_fiber_tendon_equilibrium() {
        let result = solve_static_equilibrium(
            &parameters(0.0),
            &curves(),
            0.0205,
            1.0,
            policy(),
        )
        .expect("static equilibrium");
        assert!((result.fiber_length_m - 0.01).abs() < 1.0e-10);
        assert!((result.tendon_length_m - tendon_length).abs() < 1.0e-10);
        assert_eq!(result.muscle_id, "foreleg-extensor-1");
        assert_eq!(result.parameter_sha256, digest());
        assert_eq!(result.active_force_length_sha256, digest());
        assert_eq!(result.musculotendon_length_m, musculotendon_length);
        assert_eq!(result.activation, 1.0);
        assert_eq!(result.solve_policy, policy());
        assert!((result.fiber_force_n - 100.0).abs() < 1.0e-8);
        assert!((result.tendon_force_n - 100.0).abs() < 1.0e-8);
        assert!(result.force_residual_n.abs() <= 1.0e-8);
    }

    #[test]
    fn pennation_uses_constant_thickness_geometry_and_cosine_projection() {
        let pennation = 30.0_f64.to_radians();
        let expected_cosine = pennation.cos();
        let tendon_length = 0.01 * (1.0 + 0.05 * expected_cosine);
        let musculotendon_length = 0.01 * expected_cosine + tendon_length;
        let result = solve_static_equilibrium(
            &parameters(pennation),
            &curves(),
            musculotendon_length,
            1.0,
            policy(),
        )
        .expect("pennated equilibrium");
        assert!((result.fiber_length_m - 0.01).abs() < 1.0e-10);
        assert!((result.pennation_angle_rad - pennation).abs() < 1.0e-10);
        assert!(
            (result.transmitted_fiber_force_n - 100.0 * expected_cosine).abs() < 1.0e-8
        );
        assert!((result.tendon_force_n - result.transmitted_fiber_force_n).abs() < 1.0e-8);
    }

    #[test]
    fn curve_interpolation_is_linear_and_extrapolation_rejects() {
        let table = curve(
            CurveKind::ActiveForceLength,
            &[(0.5, 0.0), (1.0, 1.0), (1.5, 0.0)],
        );
        assert!((table.evaluate(0.75).expect("interpolation") - 0.5).abs() < 1.0e-12);
        assert_eq!(
            table.evaluate(0.49),
            Err(MuscleError::CurveOutOfDomain(CurveKind::ActiveForceLength))
        );
    }

    #[test]
    fn rejects_duplicate_or_non_monotonic_curve_abscissae() {
        let table = curve(
            CurveKind::ActiveForceLength,
            &[(0.5, 0.0), (1.0, 1.0), (1.0, 0.5)],
        );
        assert_eq!(
            table.validate(CurveKind::ActiveForceLength),
            Err(MuscleError::InvalidCurve(CurveKind::ActiveForceLength))
        );
    }

    #[test]
    fn rejects_curve_kind_mismatch() {
        let mut input = curves();
        input.active_force_length.kind = CurveKind::PassiveForceLength;
        assert_eq!(input.validate(), Err(MuscleError::CurveKindMismatch));
    }

    #[test]
    fn activation_uses_exact_exponential_rise_and_fall() {
        let dynamics = ActivationDynamics {
            activation_time_constant_s: 0.02,
            deactivation_time_constant_s: 0.04,
        };
        let rise = update_activation(0.0, 1.0, 0.02, dynamics).expect("rise");
        let fall = update_activation(1.0, 0.0, 0.04, dynamics).expect("fall");
        assert!((rise.next_activation - (1.0 - (-1.0_f64).exp())).abs() < 1.0e-12);
        assert!((fall.next_activation - (-1.0_f64).exp()).abs() < 1.0e-12);
        assert_eq!(rise.time_constant_s, 0.02);
        assert_eq!(fall.time_constant_s, 0.04);
    }

    #[test]
    fn detects_non_unique_slack_equilibrium_instead_of_inventing_fiber_length() {
        let mut muscle = parameters(0.0);
        muscle.maximum_fiber_length_ratio = 1.0;
        let result = solve_static_equilibrium(
            &muscle,
            &curves(),
            0.018,
            0.0,
            policy(),
        );
        assert_eq!(result, Err(MuscleError::NonUniqueEquilibrium));
    }

    #[test]
    fn rejects_no_bracket_instead_of_clamping_force_or_length() {
        let result = solve_static_equilibrium(
            &parameters(0.0),
            &curves(),
            0.024,
            1.0,
            policy(),
        );
        assert!(matches!(result, Err(MuscleError::NoEquilibriumBracket { .. })));
    }

    #[test]
    fn rejects_impossible_pennation_geometry() {
        let mut invalid = parameters(60.0_f64.to_radians());
        invalid.minimum_fiber_length_ratio = 0.8;
        assert_eq!(invalid.validate(), Err(MuscleError::InvalidMuscleParameters));
    }

    #[test]
    fn rejects_invalid_digest_and_non_finite_curve_points() {
        let mut invalid = curves();
        invalid.tendon_force_length.source_sha256 = "no-digest".to_string();
        assert_eq!(
            invalid.validate(),
            Err(MuscleError::InvalidDigest)
        );

        let mut invalid_point = curves();
        invalid_point.active_force_length.points[1].normalized_force = f64::NAN;
        assert_eq!(
            invalid_point.validate(),
            Err(MuscleError::InvalidCurve(CurveKind::ActiveForceLength))
        );
    }

    #[test]
    fn rejects_invalid_solver_policy() {
        let mut invalid = policy();
        invalid.maximum_iterations = 0;
        assert_eq!(
            solve_static_equilibrium(&parameters(0.0), &curves(), 0.0205, 1.0, invalid),
            Err(MuscleError::InvalidSolvePolicy)
        );
    }

    #[test]
    fn reports_iteration_limit_with_last_bracket_diagnostics() {
        let mut limited = policy();
        limited.maximum_iterations = 1;
        limited.fiber_length_tolerance_m = 1.0e-15;
        limited.force_residual_tolerance_n = 1.0e-14;
        assert!(matches!(
            solve_static_equilibrium(&parameters(0.0), &curves(), 0.0204, 1.0, limited),
            Err(MuscleError::IterationLimit { iterations: 1, .. })
        ));
    }

    #[test]
    fn passive_fiber_force_supports_isometric_equilibrium_at_zero_activation() {
        let mut muscle = parameters(0.0);
        muscle.maximum_fiber_length_ratio = 1.5;
        let result = solve_static_equilibrium(
            &muscle,
            &curves(),
            0.0255,
            0.0,
            policy(),
        )
        .expect("passive equilibrium");
        assert!((result.normalized_fiber_length - 1.5).abs() < 1.0e-12);
        assert!((result.fiber_force_n - 100.0).abs() < 1.0e-8);
        assert!((result.tendon_force_n - 100.0).abs() < 1.0e-8);
    }
}
