// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Numerical-stability and projection-convergence observability.
//!
//! This module adds measurements around the existing transparent CPU reference
//! solver without changing its numerical update. It exposes signed stability
//! margins, preserves the exact stability limit that rejected a step, and runs
//! the same deterministic divergent MAC field through a bounded sweep of fixed
//! Jacobi iteration counts. These are diagnostics, not acceptance thresholds.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference::{
    MAX_COMBINED_EXPLICIT_NUMBER, MAX_PRESSURE_ITERATIONS, PeriodicMac2d, PeriodicMacConfig,
    ReferenceStateError, ReferenceStepError, ReferenceStepReport,
};

pub const STABILITY_PROBE_SCHEMA_ID: &str = "continuum-stability-probe-v0.1";
pub const PROJECTION_ITERATION_SWEEP_SCHEMA_ID: &str =
    "continuum-projection-iteration-sweep-v0.1";
pub const COMPRESSIVE_MODE_PROFILE_ID: &str = "periodic-compressive-mac-mode-v0.1";
pub const MAX_NUMERICAL_OBSERVABILITY_POINTS: usize = 32;

/// Which declared explicit-stability limit was exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilityLimitKind {
    AdvectiveCfl,
    DiffusionNumber,
    CombinedExplicitNumber,
}

/// Signed distance to each declared stability limit for one admitted step.
///
/// Margins are `limit - observed`, so positive values are inside the declared
/// envelope and zero is exactly on its boundary. A rejected step is represented
/// by [`StabilityViolationEvidence`] instead of fabricating an admitted sample.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabilityMarginSample {
    pub max_advective_cfl: f64,
    pub max_advective_cfl_limit: f64,
    pub advective_cfl_margin: f64,
    pub diffusion_number: f64,
    pub diffusion_number_limit: f64,
    pub diffusion_number_margin: f64,
    pub combined_explicit_number: f64,
    pub combined_explicit_number_limit: f64,
    pub combined_explicit_number_margin: f64,
}

/// Exact stability-envelope rejection extracted from a typed reference error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabilityViolationEvidence {
    pub limit_kind: StabilityLimitKind,
    pub observed_value: f64,
    pub configured_limit: f64,
    /// `configured_limit - observed_value`; negative for a genuine violation.
    pub signed_margin: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", content = "evidence", rename_all = "snake_case")]
pub enum StabilityProbeOutcome {
    Admitted(StabilityMarginSample),
    Rejected(StabilityViolationEvidence),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabilityProbePoint {
    pub dt_s: f64,
    pub outcome: StabilityProbeOutcome,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabilityProbeReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub initial_amplitude_mps: f64,
    pub points: Vec<StabilityProbePoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionIterationSweepPoint {
    pub pressure_iterations: usize,
    pub solver_profile: String,
    pub divergence_rms_before_per_s: f64,
    pub divergence_rms_after_per_s: f64,
    /// `after / before` when the initial divergent field has nonzero divergence.
    pub divergence_reduction_ratio: Option<f64>,
    pub pressure_residual_rms_pa_per_m2: f64,
    /// Residual divided by the first sweep point's residual. `None` only if the
    /// first residual is exactly zero.
    pub pressure_residual_ratio_to_first: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionIterationSweepReport {
    pub schema_id: String,
    pub field_profile: String,
    pub dt_s: f64,
    pub amplitude_mps: f64,
    pub points: Vec<ProjectionIterationSweepPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumericalObservabilityError {
    InvalidDt,
    InvalidAmplitude,
    TooFewPoints,
    TooManyPoints,
    InvalidIterationCount,
    NonIncreasingIterationCounts,
    NonFiniteDerivedMetric(&'static str),
    State(ReferenceStateError),
    Step(ReferenceStepError),
    UnexpectedNonStabilityFailure(ReferenceStepError),
}

impl fmt::Display for NumericalObservabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDt => write!(f, "observability dt must be finite and > 0"),
            Self::InvalidAmplitude => write!(f, "observability amplitude must be finite and > 0"),
            Self::TooFewPoints => write!(f, "observability sweep requires at least two points"),
            Self::TooManyPoints => write!(
                f,
                "observability sweep exceeds {MAX_NUMERICAL_OBSERVABILITY_POINTS} points"
            ),
            Self::InvalidIterationCount => write!(
                f,
                "pressure iteration counts must be in 1..={MAX_PRESSURE_ITERATIONS}"
            ),
            Self::NonIncreasingIterationCounts => {
                write!(f, "pressure iteration counts must be strictly increasing")
            }
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "derived observability metric {name} is non-finite")
            }
            Self::State(source) => write!(f, "observability state initialization failed: {source}"),
            Self::Step(source) => write!(f, "observability projection failed: {source}"),
            Self::UnexpectedNonStabilityFailure(source) => write!(
                f,
                "stability probe encountered a non-stability step failure: {source}"
            ),
        }
    }
}

impl std::error::Error for NumericalObservabilityError {}

impl From<ReferenceStateError> for NumericalObservabilityError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

/// Derive signed margins for a step that the reference solver admitted.
pub fn stability_margin_from_step(
    config: &PeriodicMacConfig,
    report: &ReferenceStepReport,
) -> Result<StabilityMarginSample, NumericalObservabilityError> {
    let sample = StabilityMarginSample {
        max_advective_cfl: report.max_advective_cfl,
        max_advective_cfl_limit: config.max_advective_cfl,
        advective_cfl_margin: config.max_advective_cfl - report.max_advective_cfl,
        diffusion_number: report.diffusion_number,
        diffusion_number_limit: config.max_diffusion_number,
        diffusion_number_margin: config.max_diffusion_number - report.diffusion_number,
        combined_explicit_number: report.combined_explicit_number,
        combined_explicit_number_limit: MAX_COMBINED_EXPLICIT_NUMBER,
        combined_explicit_number_margin:
            MAX_COMBINED_EXPLICIT_NUMBER - report.combined_explicit_number,
    };
    for (name, value) in [
        ("max_advective_cfl", sample.max_advective_cfl),
        ("max_advective_cfl_limit", sample.max_advective_cfl_limit),
        ("advective_cfl_margin", sample.advective_cfl_margin),
        ("diffusion_number", sample.diffusion_number),
        ("diffusion_number_limit", sample.diffusion_number_limit),
        ("diffusion_number_margin", sample.diffusion_number_margin),
        ("combined_explicit_number", sample.combined_explicit_number),
        (
            "combined_explicit_number_limit",
            sample.combined_explicit_number_limit,
        ),
        (
            "combined_explicit_number_margin",
            sample.combined_explicit_number_margin,
        ),
    ] {
        ensure_finite(name, value)?;
    }
    Ok(sample)
}

/// Extract an exact stability rejection from the reference solver's typed error.
/// Non-stability errors intentionally return `None` rather than being conflated
/// with a numerical stability-envelope violation.
pub fn stability_violation_from_error(
    error: &ReferenceStepError,
) -> Option<StabilityViolationEvidence> {
    let (limit_kind, observed_value, configured_limit) = match error {
        ReferenceStepError::CflLimitExceeded { observed, limit }
        | ReferenceStepError::PredictorCflLimitExceeded { observed, limit } => {
            (StabilityLimitKind::AdvectiveCfl, *observed, *limit)
        }
        ReferenceStepError::DiffusionLimitExceeded { observed, limit } => {
            (StabilityLimitKind::DiffusionNumber, *observed, *limit)
        }
        ReferenceStepError::CombinedExplicitLimitExceeded { observed }
        | ReferenceStepError::PredictorCombinedExplicitLimitExceeded { observed } => (
            StabilityLimitKind::CombinedExplicitNumber,
            *observed,
            MAX_COMBINED_EXPLICIT_NUMBER,
        ),
        _ => return None,
    };
    Some(StabilityViolationEvidence {
        limit_kind,
        observed_value,
        configured_limit,
        signed_margin: configured_limit - observed_value,
    })
}

/// Probe a set of candidate timesteps from the exact same passive Taylor-Green
/// initial state. Stability-envelope rejections are retained as outcomes; any
/// unrelated failure remains an outer error and cannot be mislabeled.
pub fn run_passive_taylor_green_stability_probe(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_values_s: &[f64],
) -> Result<StabilityProbeReport, NumericalObservabilityError> {
    validate_amplitude(initial_amplitude_mps)?;
    validate_point_count(dt_values_s.len())?;
    if dt_values_s
        .iter()
        .any(|dt_s| !dt_s.is_finite() || *dt_s <= 0.0)
    {
        return Err(NumericalObservabilityError::InvalidDt);
    }

    let solver_profile = config.profile_identity();
    let mut points = Vec::with_capacity(dt_values_s.len());
    for &dt_s in dt_values_s {
        let mut state = PeriodicMac2d::taylor_green(config.clone(), initial_amplitude_mps)?;
        let outcome = match state.step(dt_s) {
            Ok(report) => {
                StabilityProbeOutcome::Admitted(stability_margin_from_step(&config, &report)?)
            }
            Err(error) => match stability_violation_from_error(&error) {
                Some(evidence) => StabilityProbeOutcome::Rejected(evidence),
                None => {
                    return Err(NumericalObservabilityError::UnexpectedNonStabilityFailure(
                        error,
                    ));
                }
            },
        };
        points.push(StabilityProbePoint { dt_s, outcome });
    }

    Ok(StabilityProbeReport {
        schema_id: STABILITY_PROBE_SCHEMA_ID.to_owned(),
        solver_profile,
        initial_amplitude_mps,
        points,
    })
}

/// Measure fixed-iteration Jacobi projection convergence without modifying the
/// solver. Every point starts from the same deterministic periodic compressive
/// MAC field and differs only in `pressure_iterations`.
pub fn run_projection_iteration_sweep(
    base_config: PeriodicMacConfig,
    amplitude_mps: f64,
    dt_s: f64,
    iteration_counts: &[usize],
) -> Result<ProjectionIterationSweepReport, NumericalObservabilityError> {
    validate_amplitude(amplitude_mps)?;
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(NumericalObservabilityError::InvalidDt);
    }
    validate_point_count(iteration_counts.len())?;
    if iteration_counts
        .iter()
        .any(|count| *count == 0 || *count > MAX_PRESSURE_ITERATIONS)
    {
        return Err(NumericalObservabilityError::InvalidIterationCount);
    }
    if iteration_counts.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(NumericalObservabilityError::NonIncreasingIterationCounts);
    }

    // Validate geometry/profile bounds before constructing the shared field.
    PeriodicMac2d::zeros(base_config.clone())?;
    let (u_faces, v_faces) = compressive_mode_faces(&base_config, amplitude_mps);
    let field_profile = format!(
        "{COMPRESSIVE_MODE_PROFILE_ID};n={}x{};L={:016x},{:016x};a={:016x}",
        base_config.nx,
        base_config.ny,
        base_config.length_x_m.to_bits(),
        base_config.length_y_m.to_bits(),
        amplitude_mps.to_bits(),
    );

    let mut points = Vec::with_capacity(iteration_counts.len());
    let mut first_residual = None;
    for &pressure_iterations in iteration_counts {
        let mut config = base_config.clone();
        config.pressure_iterations = pressure_iterations;
        let solver_profile = config.profile_identity();
        let mut state = PeriodicMac2d::from_faces(config, u_faces.clone(), v_faces.clone())?;
        let projection = state
            .project_velocity(dt_s)
            .map_err(NumericalObservabilityError::Step)?;

        let divergence_reduction_ratio = if projection.divergence_rms_before_per_s > 0.0 {
            Some(finite_ratio(
                "projection_divergence_reduction_ratio",
                projection.divergence_rms_after_per_s,
                projection.divergence_rms_before_per_s,
            )?)
        } else {
            None
        };
        let reference_residual = *first_residual.get_or_insert(projection.pressure_residual_rms_pa_per_m2);
        let pressure_residual_ratio_to_first = if reference_residual > 0.0 {
            Some(finite_ratio(
                "pressure_residual_ratio_to_first",
                projection.pressure_residual_rms_pa_per_m2,
                reference_residual,
            )?)
        } else {
            None
        };

        points.push(ProjectionIterationSweepPoint {
            pressure_iterations,
            solver_profile,
            divergence_rms_before_per_s: projection.divergence_rms_before_per_s,
            divergence_rms_after_per_s: projection.divergence_rms_after_per_s,
            divergence_reduction_ratio,
            pressure_residual_rms_pa_per_m2: projection.pressure_residual_rms_pa_per_m2,
            pressure_residual_ratio_to_first,
        });
    }

    Ok(ProjectionIterationSweepReport {
        schema_id: PROJECTION_ITERATION_SWEEP_SCHEMA_ID.to_owned(),
        field_profile,
        dt_s,
        amplitude_mps,
        points,
    })
}

fn compressive_mode_faces(config: &PeriodicMacConfig, amplitude_mps: f64) -> (Vec<f64>, Vec<f64>) {
    let mut u_faces = vec![0.0; config.nx * config.ny];
    let mut v_faces = vec![0.0; config.nx * config.ny];
    let dx = config.dx();
    let dy = config.dy();
    let kx = std::f64::consts::TAU / config.length_x_m;
    let ky = std::f64::consts::TAU / config.length_y_m;
    for j in 0..config.ny {
        for i in 0..config.nx {
            let index = j * config.nx + i;
            u_faces[index] = amplitude_mps * (kx * i as f64 * dx).sin();
            v_faces[index] = amplitude_mps * (ky * j as f64 * dy).sin();
        }
    }
    (u_faces, v_faces)
}

fn validate_amplitude(amplitude_mps: f64) -> Result<(), NumericalObservabilityError> {
    if !amplitude_mps.is_finite() || amplitude_mps <= 0.0 {
        return Err(NumericalObservabilityError::InvalidAmplitude);
    }
    Ok(())
}

fn validate_point_count(count: usize) -> Result<(), NumericalObservabilityError> {
    if count < 2 {
        return Err(NumericalObservabilityError::TooFewPoints);
    }
    if count > MAX_NUMERICAL_OBSERVABILITY_POINTS {
        return Err(NumericalObservabilityError::TooManyPoints);
    }
    Ok(())
}

fn finite_ratio(
    name: &'static str,
    numerator: f64,
    denominator: f64,
) -> Result<f64, NumericalObservabilityError> {
    let ratio = numerator / denominator;
    ensure_finite(name, ratio)?;
    Ok(ratio)
}

fn ensure_finite(name: &'static str, value: f64) -> Result<(), NumericalObservabilityError> {
    if !value.is_finite() {
        return Err(NumericalObservabilityError::NonFiniteDerivedMetric(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: 12,
            ny: 12,
            length_x_m: std::f64::consts::TAU,
            length_y_m: std::f64::consts::TAU,
            slab_depth_m: 1.0,
            density_kg_m3: 1.0,
            kinematic_viscosity_m2_s: 0.01,
            pressure_iterations: 64,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    #[test]
    fn admitted_step_retains_nonnegative_stability_margins() {
        let config = config();
        let mut state = PeriodicMac2d::taylor_green(config.clone(), 0.08).unwrap();
        let report = state.step(0.001).unwrap();
        let margins = stability_margin_from_step(&config, &report).unwrap();
        assert!(margins.advective_cfl_margin >= 0.0);
        assert!(margins.diffusion_number_margin >= 0.0);
        assert!(margins.combined_explicit_number_margin >= 0.0);
    }

    #[test]
    fn cfl_and_diffusion_rejections_are_not_conflated() {
        let config = config();
        let mut cfl_state = PeriodicMac2d::taylor_green(config.clone(), 0.5).unwrap();
        let cfl_error = cfl_state.step(0.5).unwrap_err();
        let cfl = stability_violation_from_error(&cfl_error).unwrap();
        assert_eq!(cfl.limit_kind, StabilityLimitKind::AdvectiveCfl);
        assert!(cfl.signed_margin < 0.0);

        let mut diffusion_config = config;
        diffusion_config.kinematic_viscosity_m2_s = 1.0;
        let mut diffusion_state = PeriodicMac2d::zeros(diffusion_config).unwrap();
        let diffusion_error = diffusion_state.step(0.2).unwrap_err();
        let diffusion = stability_violation_from_error(&diffusion_error).unwrap();
        assert_eq!(diffusion.limit_kind, StabilityLimitKind::DiffusionNumber);
        assert!(diffusion.signed_margin < 0.0);
    }

    #[test]
    fn mixed_timestep_probe_retains_rejection_and_admission() {
        let report = run_passive_taylor_green_stability_probe(config(), 0.5, &[0.5, 0.05])
            .unwrap();
        assert!(matches!(
            report.points[0].outcome,
            StabilityProbeOutcome::Rejected(_)
        ));
        assert!(matches!(
            report.points[1].outcome,
            StabilityProbeOutcome::Admitted(_)
        ));
    }

    #[test]
    fn projection_iteration_sweep_retains_convergence_observables() {
        let report = run_projection_iteration_sweep(config(), 0.08, 0.001, &[1, 4, 16, 64])
            .unwrap();
        assert_eq!(report.points.len(), 4);
        let first = &report.points[0];
        let last = &report.points[3];
        assert!(first.divergence_rms_before_per_s > 0.0);
        assert!(last.divergence_rms_after_per_s <= first.divergence_rms_after_per_s);
        assert!(last.pressure_residual_rms_pa_per_m2 <= first.pressure_residual_rms_pa_per_m2);
        assert!(last.pressure_residual_ratio_to_first.unwrap() <= 1.0);
    }

    #[test]
    fn projection_iteration_sweep_is_deterministic() {
        let first = run_projection_iteration_sweep(config(), 0.08, 0.001, &[1, 4, 16]).unwrap();
        let second = run_projection_iteration_sweep(config(), 0.08, 0.001, &[1, 4, 16]).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn malformed_iteration_sweep_fails_closed() {
        assert_eq!(
            run_projection_iteration_sweep(config(), 0.08, 0.001, &[4, 4]),
            Err(NumericalObservabilityError::NonIncreasingIterationCounts)
        );
        assert_eq!(
            run_projection_iteration_sweep(config(), 0.08, 0.001, &[0, 4]),
            Err(NumericalObservabilityError::InvalidIterationCount)
        );
    }
}
