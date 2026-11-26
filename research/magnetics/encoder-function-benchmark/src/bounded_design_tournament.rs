//! Distribution-free field and rotor-reference uncertainty for encoder design ranking.
//!
//! This layer reuses the nominal field-to-angle evaluator and design tournament.
//! It propagates supplied simultaneous deterministic bounds; it does not infer
//! distributions, manufacturing yield, or physical truth.

use super::{
    DesignTournamentError, EncoderRequirements, FieldTrace, MagneticDesignCandidate,
    MagneticDesignCandidateMetrics, MagneticDesignTournament, evaluate_trace,
    rank_magnetic_designs,
};

#[derive(Clone, Debug, PartialEq)]
pub struct FieldTraceUncertaintyBounds {
    pub trace_id: String,
    pub frame_id: String,
    pub operating_condition_id: String,
    pub grid_id: String,
    pub field_archive_sha256: String,
    pub source_solver_run_id: String,
    pub uncertainty_record_id: String,
    pub phase_zero_calibration_id: String,
    /// Simultaneous Euclidean 3D field-vector error radius, tesla, for each sample.
    pub field_vector_error_radius_t: Vec<f64>,
    /// Simultaneous absolute mechanical-reference-angle error bound, radians.
    pub mechanical_angle_error_bound_rad: Vec<f64>,
    /// Absolute error bound of the independent phase-zero calibration, radians.
    pub phase_zero_error_bound_rad: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedMagneticDesignCandidate {
    pub design: MagneticDesignCandidate,
    /// One uncertainty record for each design scenario, in the same explicit order.
    pub scenario_uncertainty: Vec<FieldTraceUncertaintyBounds>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedScenarioMetrics {
    pub trace_id: String,
    pub operating_condition_id: String,
    pub uncertainty_record_id: String,
    pub source_solver_run_id: String,
    pub minimum_in_plane_field_lower_bound_t: f64,
    pub maximum_in_plane_field_upper_bound_t: f64,
    pub maximum_absolute_phase_error_upper_bound_rad: f64,
    pub rms_phase_error_upper_bound_rad: f64,
    pub p95_absolute_phase_error_upper_bound_rad: f64,
    pub amplitude_ripple_upper_bound_fraction: Option<f64>,
    pub rms_cross_axis_ratio_upper_bound: Option<f64>,
    pub unbounded_phase_sample_count: usize,
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedMagneticDesignMetrics {
    pub candidate_id: String,
    pub nominal_candidate_metrics: MagneticDesignCandidateMetrics,
    pub robustly_accepted: bool,
    pub minimum_in_plane_field_lower_bound_t: f64,
    pub worst_peak_phase_error_upper_bound_rad: f64,
    pub worst_rms_phase_error_upper_bound_rad: f64,
    pub worst_p95_phase_error_upper_bound_rad: f64,
    pub worst_amplitude_ripple_upper_bound_fraction: Option<f64>,
    pub worst_rms_cross_axis_ratio_upper_bound: Option<f64>,
    pub scenarios: Vec<BoundedScenarioMetrics>,
    pub is_pareto_optimal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedMagneticDesignTournament {
    pub nominal_tournament: MagneticDesignTournament,
    pub uncertainty_record_count: usize,
    pub robustly_accepted_candidate_count: usize,
    pub pareto_candidate_ids: Vec<String>,
    pub candidates: Vec<BoundedMagneticDesignMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoundedDesignTournamentError {
    BaseTournament(DesignTournamentError),
    UncertaintyScenarioCountMismatch { candidate_index: usize },
    InvalidUncertaintyMetadata { candidate_index: usize, scenario_index: usize },
    UncertaintyTraceIdentityMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintyFrameIdentityMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintyConditionIdentityMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintyGridIdentityMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintyArchiveDigestMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintySolverRunMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintyCalibrationMismatch { candidate_index: usize, scenario_index: usize },
    UncertaintySampleCountMismatch { candidate_index: usize, scenario_index: usize },
    InvalidUncertaintyBound { candidate_index: usize, scenario_index: usize, sample_index: usize },
    InvalidPhaseZeroBound { candidate_index: usize, scenario_index: usize },
    NumericOverflow { candidate_index: usize, scenario_index: usize, sample_index: usize },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_uncertainty_record(
    design: &MagneticDesignCandidate,
    trace: &FieldTrace,
    bounds: &FieldTraceUncertaintyBounds,
    requirements: &EncoderRequirements,
    candidate_index: usize,
    scenario_index: usize,
) -> Result<(), BoundedDesignTournamentError> {
    if !nonempty(&bounds.uncertainty_record_id)
        || !nonempty(&bounds.phase_zero_calibration_id)
        || !nonempty(&bounds.source_solver_run_id)
        || !nonempty(&bounds.field_archive_sha256)
    {
        return Err(BoundedDesignTournamentError::InvalidUncertaintyMetadata {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.trace_id != trace.trace_id {
        return Err(BoundedDesignTournamentError::UncertaintyTraceIdentityMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.frame_id != trace.frame_id {
        return Err(BoundedDesignTournamentError::UncertaintyFrameIdentityMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.operating_condition_id != trace.operating_condition_id {
        return Err(BoundedDesignTournamentError::UncertaintyConditionIdentityMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.grid_id != trace.grid_id {
        return Err(BoundedDesignTournamentError::UncertaintyGridIdentityMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.field_archive_sha256 != design.field_archive_sha256 {
        return Err(BoundedDesignTournamentError::UncertaintyArchiveDigestMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if !design
        .solver_run_ids
        .iter()
        .any(|run_id| run_id == &bounds.source_solver_run_id)
    {
        return Err(BoundedDesignTournamentError::UncertaintySolverRunMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.phase_zero_calibration_id != requirements.phase_zero_calibration_id {
        return Err(BoundedDesignTournamentError::UncertaintyCalibrationMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if bounds.field_vector_error_radius_t.len() != trace.samples.len()
        || bounds.mechanical_angle_error_bound_rad.len() != trace.samples.len()
    {
        return Err(BoundedDesignTournamentError::UncertaintySampleCountMismatch {
            candidate_index,
            scenario_index,
        });
    }
    if !bounds.phase_zero_error_bound_rad.is_finite() || bounds.phase_zero_error_bound_rad < 0.0 {
        return Err(BoundedDesignTournamentError::InvalidPhaseZeroBound {
            candidate_index,
            scenario_index,
        });
    }
    for (sample_index, (field_bound, angle_bound)) in bounds
        .field_vector_error_radius_t
        .iter()
        .zip(bounds.mechanical_angle_error_bound_rad.iter())
        .enumerate()
    {
        let field_bound = *field_bound;
        let angle_bound = *angle_bound;
        if !field_bound.is_finite()
            || field_bound < 0.0
            || !angle_bound.is_finite()
            || angle_bound < 0.0
        {
            return Err(BoundedDesignTournamentError::InvalidUncertaintyBound {
                candidate_index,
                scenario_index,
                sample_index,
            });
        }
    }
    Ok(())
}

fn wrap_to_pi(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI
}

fn rms(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |accumulator, value| accumulator.hypot(*value))
        / (values.len() as f64).sqrt()
}

fn percentile_95(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = (sorted.len() * 95).div_ceil(100).saturating_sub(1);
    sorted[index]
}

fn bounded_scenario_metrics(
    trace: &FieldTrace,
    bounds: &FieldTraceUncertaintyBounds,
    requirements: &EncoderRequirements,
    candidate_index: usize,
    scenario_index: usize,
) -> Result<BoundedScenarioMetrics, BoundedDesignTournamentError> {
    let nominal = evaluate_trace(trace, requirements).map_err(|error| {
        BoundedDesignTournamentError::BaseTournament(DesignTournamentError::ScenarioEvaluation(
            error,
        ))
    })?;

    let mut phase_error_upper_bounds = Vec::with_capacity(trace.samples.len());
    let mut lower_in_plane_magnitudes = Vec::with_capacity(trace.samples.len());
    let mut upper_in_plane_magnitudes = Vec::with_capacity(trace.samples.len());
    let mut upper_out_of_plane_magnitudes = Vec::with_capacity(trace.samples.len());
    let mut unbounded_phase_sample_count = 0_usize;

    let pole_pairs = requirements.pole_pairs as f64;
    for (sample_index, (sample, (field_radius, angle_radius))) in trace
        .samples
        .iter()
        .zip(
            bounds
                .field_vector_error_radius_t
                .iter()
                .zip(bounds.mechanical_angle_error_bound_rad.iter()),
        )
        .enumerate()
    {
        let nominal_in_plane = sample.bx_t.hypot(sample.by_t);
        let radius = *field_radius;
        let angle_bound_rad = *angle_radius;
        let lower_in_plane = nominal_in_plane - radius;
        let upper_in_plane = nominal_in_plane + radius;
        if !nominal_in_plane.is_finite() || !lower_in_plane.is_finite() || !upper_in_plane.is_finite() {
            return Err(BoundedDesignTournamentError::NumericOverflow {
                candidate_index,
                scenario_index,
                sample_index,
            });
        }

        let upper_out_of_plane = sample.bz_t.abs() + radius;
        if !upper_out_of_plane.is_finite() {
            return Err(BoundedDesignTournamentError::NumericOverflow {
                candidate_index,
                scenario_index,
                sample_index,
            });
        }
        lower_in_plane_magnitudes.push(lower_in_plane.max(0.0));
        upper_in_plane_magnitudes.push(upper_in_plane);
        upper_out_of_plane_magnitudes.push(upper_out_of_plane);

        if nominal_in_plane <= 0.0 || radius >= nominal_in_plane {
            unbounded_phase_sample_count += 1;
            phase_error_upper_bounds.push(std::f64::consts::PI);
            continue;
        }

        // A 2D perturbation disk of radius r centred on a vector of magnitude m
        // subtends at most asin(r/m) radians at the origin when r < m.
        let field_direction_error = (radius / nominal_in_plane).asin();
        let mechanical_phase_error = pole_pairs * angle_bound_rad;
        let total_phase_error_bound = field_direction_error
            + mechanical_phase_error
            + bounds.phase_zero_error_bound_rad;
        if !total_phase_error_bound.is_finite() {
            return Err(BoundedDesignTournamentError::NumericOverflow {
                candidate_index,
                scenario_index,
                sample_index,
            });
        }
        let expected_phase = requirements.phase_rotation_sense as f64
            * pole_pairs
            * sample.mechanical_angle_rad
            + requirements.phase_zero_offset_rad;
        let nominal_phase_error =
            wrap_to_pi(sample.by_t.atan2(sample.bx_t) - expected_phase);
        let upper_phase_error =
            (nominal_phase_error.abs() + total_phase_error_bound).min(std::f64::consts::PI);
        if !upper_phase_error.is_finite() {
            return Err(BoundedDesignTournamentError::NumericOverflow {
                candidate_index,
                scenario_index,
                sample_index,
            });
        }
        phase_error_upper_bounds.push(upper_phase_error);
    }

    let minimum_in_plane_field_lower_bound_t = lower_in_plane_magnitudes
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let maximum_in_plane_field_upper_bound_t = upper_in_plane_magnitudes
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    let mean_in_plane_field_lower_bound_t =
        lower_in_plane_magnitudes.iter().sum::<f64>() / lower_in_plane_magnitudes.len() as f64;

    let amplitude_ripple_upper_bound_fraction =
        if mean_in_plane_field_lower_bound_t > 0.0 {
            Some(
                (maximum_in_plane_field_upper_bound_t - minimum_in_plane_field_lower_bound_t)
                    / mean_in_plane_field_lower_bound_t,
            )
        } else {
            None
        };

    let rms_in_plane_field_lower_bound_t = rms(&lower_in_plane_magnitudes);
    let rms_out_of_plane_field_upper_bound_t = rms(&upper_out_of_plane_magnitudes);
    let rms_cross_axis_ratio_upper_bound = if rms_in_plane_field_lower_bound_t > 0.0 {
        Some(rms_out_of_plane_field_upper_bound_t / rms_in_plane_field_lower_bound_t)
    } else {
        None
    };

    let maximum_absolute_phase_error_upper_bound_rad = phase_error_upper_bounds
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    let rms_phase_error_upper_bound_rad = rms(&phase_error_upper_bounds);
    let p95_absolute_phase_error_upper_bound_rad = percentile_95(&phase_error_upper_bounds);
    if !minimum_in_plane_field_lower_bound_t.is_finite()
        || !maximum_in_plane_field_upper_bound_t.is_finite()
        || !rms_in_plane_field_lower_bound_t.is_finite()
        || !rms_out_of_plane_field_upper_bound_t.is_finite()
        || !maximum_absolute_phase_error_upper_bound_rad.is_finite()
        || !rms_phase_error_upper_bound_rad.is_finite()
        || !p95_absolute_phase_error_upper_bound_rad.is_finite()
        || amplitude_ripple_upper_bound_fraction.is_some_and(|value| !value.is_finite())
        || rms_cross_axis_ratio_upper_bound.is_some_and(|value| !value.is_finite())
    {
        return Err(BoundedDesignTournamentError::NumericOverflow {
            candidate_index,
            scenario_index,
            sample_index: 0,
        });
    }

    let accepted = unbounded_phase_sample_count == 0
        && minimum_in_plane_field_lower_bound_t >= requirements.minimum_in_plane_field_t
        && maximum_absolute_phase_error_upper_bound_rad
            <= requirements.maximum_peak_phase_error_rad
        && rms_phase_error_upper_bound_rad <= requirements.maximum_rms_phase_error_rad
        && amplitude_ripple_upper_bound_fraction
            .is_some_and(|value| value <= requirements.maximum_amplitude_ripple_fraction)
        && rms_cross_axis_ratio_upper_bound
            .is_some_and(|value| value <= requirements.maximum_rms_cross_axis_ratio);

    Ok(BoundedScenarioMetrics {
        trace_id: trace.trace_id.clone(),
        operating_condition_id: trace.operating_condition_id.clone(),
        uncertainty_record_id: bounds.uncertainty_record_id.clone(),
        source_solver_run_id: bounds.source_solver_run_id.clone(),
        minimum_in_plane_field_lower_bound_t,
        maximum_in_plane_field_upper_bound_t,
        maximum_absolute_phase_error_upper_bound_rad,
        rms_phase_error_upper_bound_rad,
        p95_absolute_phase_error_upper_bound_rad,
        amplitude_ripple_upper_bound_fraction,
        rms_cross_axis_ratio_upper_bound,
        unbounded_phase_sample_count,
        accepted,
    })
}

fn dominates(left: &BoundedMagneticDesignMetrics, right: &BoundedMagneticDesignMetrics) -> bool {
    let (Some(left_ripple), Some(right_ripple)) = (
        left.worst_amplitude_ripple_upper_bound_fraction,
        right.worst_amplitude_ripple_upper_bound_fraction,
    ) else {
        return false;
    };
    let (Some(left_cross), Some(right_cross)) = (
        left.worst_rms_cross_axis_ratio_upper_bound,
        right.worst_rms_cross_axis_ratio_upper_bound,
    ) else {
        return false;
    };

    let no_worse = left.minimum_in_plane_field_lower_bound_t
        >= right.minimum_in_plane_field_lower_bound_t
        && left.worst_peak_phase_error_upper_bound_rad
            <= right.worst_peak_phase_error_upper_bound_rad
        && left.worst_rms_phase_error_upper_bound_rad
            <= right.worst_rms_phase_error_upper_bound_rad
        && left.worst_p95_phase_error_upper_bound_rad
            <= right.worst_p95_phase_error_upper_bound_rad
        && left_ripple <= right_ripple
        && left_cross <= right_cross;
    let strictly_better = left.minimum_in_plane_field_lower_bound_t
        > right.minimum_in_plane_field_lower_bound_t
        || left.worst_peak_phase_error_upper_bound_rad
            < right.worst_peak_phase_error_upper_bound_rad
        || left.worst_rms_phase_error_upper_bound_rad
            < right.worst_rms_phase_error_upper_bound_rad
        || left.worst_p95_phase_error_upper_bound_rad
            < right.worst_p95_phase_error_upper_bound_rad
        || left_ripple < right_ripple
        || left_cross < right_cross;
    no_worse && strictly_better
}

/// Rank candidate magnetic designs under distribution-free field and reference bounds.
///
/// For each sample with nominal in-plane field magnitude m and simultaneous
/// Euclidean 3D field error radius r < m, the angular perturbation is bounded
/// by asin(r/m). Mechanical-reference uncertainty contributes p*delta_theta,
/// and independent phase-zero uncertainty is added directly. When r >= m,
/// zero in-plane field is admissible and phase is unbounded, so the scenario fails.
///
/// Amplitude ripple and cross-axis ratio use conservative lower denominators.
/// No Gaussian, IID, confidence or manufacturing-yield assumptions are made.
pub fn rank_bounded_magnetic_designs(
    tournament_id: &str,
    candidates: &[BoundedMagneticDesignCandidate],
    requirements: &EncoderRequirements,
) -> Result<BoundedMagneticDesignTournament, BoundedDesignTournamentError> {
    let designs = candidates
        .iter()
        .map(|candidate| candidate.design.clone())
        .collect::<Vec<_>>();
    let nominal_tournament = rank_magnetic_designs(tournament_id, &designs, requirements)
        .map_err(BoundedDesignTournamentError::BaseTournament)?;

    let mut bounded_candidates = Vec::with_capacity(candidates.len());
    let mut uncertainty_record_count = 0_usize;

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if candidate.scenario_uncertainty.len() != candidate.design.scenarios.len() {
            return Err(BoundedDesignTournamentError::UncertaintyScenarioCountMismatch {
                candidate_index,
            });
        }

        let mut scenario_results = Vec::with_capacity(candidate.design.scenarios.len());
        for (scenario_index, (trace, uncertainty)) in candidate
            .design
            .scenarios
            .iter()
            .zip(candidate.scenario_uncertainty.iter())
            .enumerate()
        {
            validate_uncertainty_record(
                &candidate.design,
                trace,
                uncertainty,
                requirements,
                candidate_index,
                scenario_index,
            )?;
            let metrics = bounded_scenario_metrics(
                trace,
                uncertainty,
                requirements,
                candidate_index,
                scenario_index,
            )?;
            scenario_results.push(metrics);
            uncertainty_record_count += 1;
        }

        let minimum_in_plane_field_lower_bound_t = scenario_results
            .iter()
            .map(|scenario| scenario.minimum_in_plane_field_lower_bound_t)
            .fold(f64::INFINITY, f64::min);
        let worst_peak_phase_error_upper_bound_rad = scenario_results
            .iter()
            .map(|scenario| scenario.maximum_absolute_phase_error_upper_bound_rad)
            .fold(0.0_f64, f64::max);
        let worst_rms_phase_error_upper_bound_rad = scenario_results
            .iter()
            .map(|scenario| scenario.rms_phase_error_upper_bound_rad)
            .fold(0.0_f64, f64::max);
        let worst_p95_phase_error_upper_bound_rad = scenario_results
            .iter()
            .map(|scenario| scenario.p95_absolute_phase_error_upper_bound_rad)
            .fold(0.0_f64, f64::max);
        let worst_amplitude_ripple_upper_bound_fraction = scenario_results
            .iter()
            .map(|scenario| scenario.amplitude_ripple_upper_bound_fraction)
            .try_fold(0.0_f64, |maximum, value| value.map(|value| maximum.max(value)));
        let worst_rms_cross_axis_ratio_upper_bound = scenario_results
            .iter()
            .map(|scenario| scenario.rms_cross_axis_ratio_upper_bound)
            .try_fold(0.0_f64, |maximum, value| value.map(|value| maximum.max(value)));

        let nominal_candidate_metrics = nominal_tournament.candidates[candidate_index].clone();
        let robustly_accepted = nominal_candidate_metrics.accepted_all_scenarios
            && scenario_results.iter().all(|scenario| scenario.accepted);

        bounded_candidates.push(BoundedMagneticDesignMetrics {
            candidate_id: candidate.design.candidate_id.clone(),
            nominal_candidate_metrics,
            robustly_accepted,
            minimum_in_plane_field_lower_bound_t,
            worst_peak_phase_error_upper_bound_rad,
            worst_rms_phase_error_upper_bound_rad,
            worst_p95_phase_error_upper_bound_rad,
            worst_amplitude_ripple_upper_bound_fraction,
            worst_rms_cross_axis_ratio_upper_bound,
            scenarios: scenario_results,
            is_pareto_optimal: false,
        });
    }

    for index in 0..bounded_candidates.len() {
        if !bounded_candidates[index].robustly_accepted {
            continue;
        }
        let is_dominated = bounded_candidates.iter().enumerate().any(|(other_index, other)| {
            other_index != index && other.robustly_accepted && dominates(other, &bounded_candidates[index])
        });
        bounded_candidates[index].is_pareto_optimal = !is_dominated;
    }

    let robustly_accepted_candidate_count = bounded_candidates
        .iter()
        .filter(|candidate| candidate.robustly_accepted)
        .count();
    let pareto_candidate_ids = bounded_candidates
        .iter()
        .filter(|candidate| candidate.is_pareto_optimal)
        .map(|candidate| candidate.candidate_id.clone())
        .collect();

    Ok(BoundedMagneticDesignTournament {
        nominal_tournament,
        uncertainty_record_count,
        robustly_accepted_candidate_count,
        pareto_candidate_ids,
        candidates: bounded_candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceKind, FieldSample};
    use std::f64::consts::TAU;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1.0e-12,
            "expected {left:.16e} to be within tolerance of {right:.16e}"
        );
    }

    fn requirements() -> EncoderRequirements {
        EncoderRequirements {
            pole_pairs: 4,
            phase_rotation_sense: 1,
            phase_zero_calibration_id: "independent-phase-zero-v1".to_owned(),
            phase_zero_offset_rad: 0.0,
            minimum_in_plane_field_t: 0.1,
            maximum_peak_phase_error_rad: 0.05,
            maximum_rms_phase_error_rad: 0.03,
            maximum_amplitude_ripple_fraction: 0.20,
            maximum_rms_cross_axis_ratio: 0.05,
        }
    }

    fn trace(trace_id: &str, condition_id: &str, amplitude_t: f64, phase_bias_rad: f64) -> FieldTrace {
        let samples = (0..256)
            .map(|index| {
                let mechanical = TAU * index as f64 / 256.0;
                let electrical = 4.0 * mechanical + phase_bias_rad;
                FieldSample {
                    mechanical_angle_rad: mechanical,
                    bx_t: amplitude_t * electrical.cos(),
                    by_t: amplitude_t * electrical.sin(),
                    bz_t: 0.0,
                }
            })
            .collect();
        FieldTrace {
            trace_id: trace_id.to_owned(),
            frame_id: "encoder-frame-001".to_owned(),
            operating_condition_id: condition_id.to_owned(),
            grid_id: "grid-256".to_owned(),
            evidence_kind: EvidenceKind::SyntheticScenario,
            samples,
        }
    }

    fn candidate(id: &str, amplitude_t: f64, phase_bias_rad: f64) -> MagneticDesignCandidate {
        MagneticDesignCandidate {
            candidate_id: id.to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            field_archive_sha256: DIGEST_C.to_owned(),
            solver_run_ids: vec![format!("solver-run-{id}")],
            scenarios: vec![
                trace(&format!("{id}-nominal"), "nominal", amplitude_t, phase_bias_rad),
                trace(&format!("{id}-gap"), "increased-gap", amplitude_t * 0.9, phase_bias_rad),
            ],
        }
    }

    fn bounded_candidate(
        id: &str,
        amplitude_t: f64,
        phase_bias_rad: f64,
        field_radius_t: f64,
        angle_error_rad: f64,
        phase_zero_error_rad: f64,
    ) -> BoundedMagneticDesignCandidate {
        let design = candidate(id, amplitude_t, phase_bias_rad);
        let scenario_uncertainty = design
            .scenarios
            .iter()
            .map(|trace| FieldTraceUncertaintyBounds {
                trace_id: trace.trace_id.clone(),
                frame_id: trace.frame_id.clone(),
                operating_condition_id: trace.operating_condition_id.clone(),
                grid_id: trace.grid_id.clone(),
                field_archive_sha256: design.field_archive_sha256.clone(),
                source_solver_run_id: design.solver_run_ids[0].clone(),
                uncertainty_record_id: format!("uncertainty-{}", trace.trace_id),
                phase_zero_calibration_id: "independent-phase-zero-v1".to_owned(),
                field_vector_error_radius_t: vec![field_radius_t; trace.samples.len()],
                mechanical_angle_error_bound_rad: vec![angle_error_rad; trace.samples.len()],
                phase_zero_error_bound_rad: phase_zero_error_rad,
            })
            .collect();
        BoundedMagneticDesignCandidate { design, scenario_uncertainty }
    }

    #[test]
    fn zero_uncertainty_matches_nominal_design_function_metrics() {
        let candidate = bounded_candidate("zero-bounds", 1.0, 0.01, 0.0, 0.0, 0.0);
        let nominal = rank_magnetic_designs(
            "zero-bounds-tournament",
            std::slice::from_ref(&candidate.design),
            &requirements(),
        )
        .unwrap();
        let bounded = rank_bounded_magnetic_designs(
            "zero-bounds-tournament",
            &[candidate],
            &requirements(),
        )
        .unwrap();

        let nominal_scenarios = &nominal.candidates[0].scenario_metrics.scenarios;
        let bounded_scenarios = &bounded.candidates[0].scenarios;
        for (base, robust) in nominal_scenarios.iter().zip(bounded_scenarios) {
            assert_close(
                robust.minimum_in_plane_field_lower_bound_t,
                base.minimum_in_plane_field_t,
            );
            assert_close(
                robust.maximum_absolute_phase_error_upper_bound_rad,
                base.maximum_absolute_phase_error_rad,
            );
            assert_close(
                robust.rms_phase_error_upper_bound_rad,
                base.rms_phase_error_rad,
            );
            assert_close(
                robust.p95_absolute_phase_error_upper_bound_rad,
                base.p95_absolute_phase_error_rad,
            );
            assert_close(
                robust.amplitude_ripple_upper_bound_fraction.unwrap(),
                base.amplitude_ripple_fraction.unwrap(),
            );
            assert_close(
                robust.rms_cross_axis_ratio_upper_bound.unwrap(),
                base.rms_cross_axis_ratio.unwrap(),
            );
        }
    }

    #[test]
    fn increasing_field_error_reduces_field_margin_and_increases_phase_bound() {
        let low = bounded_candidate("low-error", 1.0, 0.01, 0.01, 0.0, 0.0);
        let high = bounded_candidate("high-error", 1.0, 0.01, 0.05, 0.0, 0.0);
        let low_result =
            rank_bounded_magnetic_designs("monotonic-low", &[low], &requirements()).unwrap();
        let high_result =
            rank_bounded_magnetic_designs("monotonic-high", &[high], &requirements()).unwrap();

        assert!(
            high_result.candidates[0].minimum_in_plane_field_lower_bound_t
                < low_result.candidates[0].minimum_in_plane_field_lower_bound_t
        );
        assert!(
            high_result.candidates[0].worst_peak_phase_error_upper_bound_rad
                > low_result.candidates[0].worst_peak_phase_error_upper_bound_rad
        );
    }

    #[test]
    fn field_error_radius_reaching_nominal_magnitude_makes_phase_unbounded() {
        let candidate = bounded_candidate("unbounded-phase", 1.0, 0.0, 1.0, 0.0, 0.0);
        let result = rank_bounded_magnetic_designs(
            "unbounded-phase-tournament",
            &[candidate],
            &requirements(),
        )
        .unwrap();
        assert_eq!(result.robustly_accepted_candidate_count, 0);
        assert!(result.candidates[0]
            .scenarios
            .iter()
            .all(|scenario| scenario.unbounded_phase_sample_count > 0));
        assert!(!result.candidates[0].robustly_accepted);
    }

    #[test]
    fn mechanical_reference_uncertainty_scales_with_pole_pairs() {
        let candidate = bounded_candidate("angle-bound", 1.0, 0.0, 0.0, 0.01, 0.0);
        let result =
            rank_bounded_magnetic_designs("angle-tournament", &[candidate], &requirements()).unwrap();
        assert!(
            (result.candidates[0].worst_peak_phase_error_upper_bound_rad - 0.04).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn independent_phase_zero_error_is_added_to_field_direction_error() {
        let no_offset = bounded_candidate("phase-zero-a", 1.0, 0.0, 0.01, 0.0, 0.0);
        let with_offset = bounded_candidate("phase-zero-b", 1.0, 0.0, 0.01, 0.0, 0.005);
        let baseline =
            rank_bounded_magnetic_designs("phase-zero-base", &[no_offset], &requirements()).unwrap();
        let perturbed = rank_bounded_magnetic_designs(
            "phase-zero-perturbed",
            &[with_offset],
            &requirements(),
        )
        .unwrap();
        assert!(
            perturbed.candidates[0].worst_peak_phase_error_upper_bound_rad
                > baseline.candidates[0].worst_peak_phase_error_upper_bound_rad
        );
    }

    #[test]
    fn uncertainty_records_must_match_the_source_archive_and_calibration() {
        let mut candidate = bounded_candidate("bad-source", 1.0, 0.0, 0.01, 0.0, 0.0);
        candidate.scenario_uncertainty[0].field_archive_sha256 = "different-archive".to_owned();
        assert_eq!(
            rank_bounded_magnetic_designs(
                "bad-source-tournament",
                &[candidate],
                &requirements(),
            )
            .expect_err("archive mismatch"),
            BoundedDesignTournamentError::UncertaintyArchiveDigestMismatch {
                candidate_index: 0,
                scenario_index: 0
            }
        );
    }

    #[test]
    fn nonfinite_or_negative_sample_bounds_are_rejected() {
        let mut candidate = bounded_candidate("bad-bound", 1.0, 0.0, 0.01, 0.0, 0.0);
        candidate.scenario_uncertainty[1].field_vector_error_radius_t[3] = f64::NAN;
        assert_eq!(
            rank_bounded_magnetic_designs(
                "bad-bound-tournament",
                &[candidate],
                &requirements(),
            )
            .expect_err("non-finite radius"),
            BoundedDesignTournamentError::InvalidUncertaintyBound {
                candidate_index: 0,
                scenario_index: 1,
                sample_index: 3
            }
        );
    }

    #[test]
    fn uncertainty_sample_count_and_condition_records_cannot_be_pooled() {
        let mut candidate = bounded_candidate("bad-count", 1.0, 0.0, 0.01, 0.0, 0.0);
        candidate.scenario_uncertainty[0].field_vector_error_radius_t.pop();
        assert_eq!(
            rank_bounded_magnetic_designs("bad-count-tournament", &[candidate], &requirements())
                .expect_err("sample count mismatch"),
            BoundedDesignTournamentError::UncertaintySampleCountMismatch {
                candidate_index: 0,
                scenario_index: 0
            }
        );
    }
}
