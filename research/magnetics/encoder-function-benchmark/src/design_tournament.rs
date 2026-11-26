//! Finite candidate tournament for function-qualified magnetic encoder designs.
//!
//! Candidate field maps are compared through the same independent field-to-angle
//! requirements. This module ranks supplied solver outputs; it does not solve or
//! optimize magnet geometry and never infers a production distribution.

use std::collections::HashSet;

use super::{
    EncoderRequirements, EvidenceKind, EvaluationError, FieldTrace,
    OperatingConditionScenarioSetMetrics, evaluate_operating_condition_scenarios,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MagneticDesignCandidate {
    pub candidate_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub field_archive_sha256: String,
    pub solver_run_ids: Vec<String>,
    /// Same preregistered condition family for every candidate.
    pub scenarios: Vec<FieldTrace>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MagneticDesignCandidateMetrics {
    pub candidate_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub field_archive_sha256: String,
    pub solver_run_ids: Vec<String>,
    pub scenario_metrics: OperatingConditionScenarioSetMetrics,
    pub accepted_all_scenarios: bool,
    pub is_pareto_optimal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MagneticDesignTournament {
    pub tournament_id: String,
    pub evidence_kind: EvidenceKind,
    pub frame_id: String,
    pub grid_id: String,
    pub phase_zero_calibration_id: String,
    pub candidate_count: usize,
    pub accepted_candidate_count: usize,
    pub pareto_candidate_ids: Vec<String>,
    /// Candidate order is preserved from the explicit input list.
    pub candidates: Vec<MagneticDesignCandidateMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesignTournamentError {
    InvalidTournamentId,
    EmptyCandidateSet,
    InvalidCandidateMetadata { index: usize },
    InvalidSha256 { index: usize, field: &'static str },
    EmptySolverRunSet { index: usize },
    DuplicateSolverRunId { index: usize },
    DuplicateCandidateId,
    DuplicateTraceId,
    MixedEvidenceKinds,
    FrameIdentityMismatch,
    GridIdentityMismatch,
    GridCoordinatesMismatch,
    ScenarioEvaluation(EvaluationError),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_candidate(
    candidate: &MagneticDesignCandidate,
    index: usize,
) -> Result<(), DesignTournamentError> {
    if !nonempty(&candidate.candidate_id)
        || !nonempty(&candidate.geometry_sha256)
        || !nonempty(&candidate.material_sha256)
        || !nonempty(&candidate.field_archive_sha256)
        || candidate.scenarios.is_empty()
    {
        return Err(DesignTournamentError::InvalidCandidateMetadata { index });
    }

    for (field, digest) in [
        ("geometry_sha256", candidate.geometry_sha256.as_str()),
        ("material_sha256", candidate.material_sha256.as_str()),
        ("field_archive_sha256", candidate.field_archive_sha256.as_str()),
    ] {
        if !valid_sha256(digest) {
            return Err(DesignTournamentError::InvalidSha256 { index, field });
        }
    }

    if candidate.solver_run_ids.is_empty() {
        return Err(DesignTournamentError::EmptySolverRunSet { index });
    }
    let mut run_ids = HashSet::with_capacity(candidate.solver_run_ids.len());
    for run_id in &candidate.solver_run_ids {
        if !nonempty(run_id) || !run_ids.insert(run_id.as_str()) {
            return Err(DesignTournamentError::DuplicateSolverRunId { index });
        }
    }
    Ok(())
}

fn same_angles(left: &FieldTrace, right: &FieldTrace) -> bool {
    left.samples.len() == right.samples.len()
        && left
            .samples
            .iter()
            .zip(right.samples.iter())
            .all(|(left, right)| {
                left.mechanical_angle_rad.to_bits() == right.mechanical_angle_rad.to_bits()
            })
}

fn dominates(
    left: &OperatingConditionScenarioSetMetrics,
    right: &OperatingConditionScenarioSetMetrics,
) -> bool {
    let (Some(left_ripple), Some(right_ripple)) = (
        left.worst_amplitude_ripple_fraction,
        right.worst_amplitude_ripple_fraction,
    ) else {
        return false;
    };
    let (Some(left_cross_axis), Some(right_cross_axis)) = (
        left.worst_rms_cross_axis_ratio,
        right.worst_rms_cross_axis_ratio,
    ) else {
        return false;
    };

    let no_worse = left.minimum_in_plane_field_t >= right.minimum_in_plane_field_t
        && left.worst_peak_phase_error_rad <= right.worst_peak_phase_error_rad
        && left.worst_rms_phase_error_rad <= right.worst_rms_phase_error_rad
        && left_ripple <= right_ripple
        && left_cross_axis <= right_cross_axis;
    let strictly_better = left.minimum_in_plane_field_t > right.minimum_in_plane_field_t
        || left.worst_peak_phase_error_rad < right.worst_peak_phase_error_rad
        || left.worst_rms_phase_error_rad < right.worst_rms_phase_error_rad
        || left_ripple < right_ripple
        || left_cross_axis < right_cross_axis;
    no_worse && strictly_better
}

/// Rank an explicit set of magnetic designs by their field-to-angle function.
///
/// Every candidate must use the same evidence class, frame and full-revolution
/// angular grid. Its scenario family may contain multiple operating conditions.
/// The same requirements and independently calibrated phase zero are applied to
/// all candidates. A candidate is accepted only if every supplied scenario
/// passes. The Pareto objectives are:
///
/// - maximize minimum in-plane field;
/// - minimize worst peak and RMS phase error;
/// - minimize worst amplitude ripple and cross-axis ratio.
///
/// No weighted scalar score is created. This is a finite tournament, not a
/// magnetostatic solver, a continuous optimizer or a global-optimality proof.
pub fn rank_magnetic_designs(
    tournament_id: &str,
    candidates: &[MagneticDesignCandidate],
    requirements: &EncoderRequirements,
) -> Result<MagneticDesignTournament, DesignTournamentError> {
    if !nonempty(tournament_id) {
        return Err(DesignTournamentError::InvalidTournamentId);
    }
    if candidates.is_empty() {
        return Err(DesignTournamentError::EmptyCandidateSet);
    }

    let first_candidate = &candidates[0];
    validate_candidate(first_candidate, 0)?;
    let first_trace = first_candidate
        .scenarios
        .first()
        .expect("candidate validation requires at least one scenario");
    let evidence_kind = first_trace.evidence_kind;
    let frame_id = first_trace.frame_id.as_str();
    let grid_id = first_trace.grid_id.as_str();

    let mut candidate_ids = HashSet::with_capacity(candidates.len());
    let mut trace_ids = HashSet::new();
    let mut results = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().enumerate() {
        validate_candidate(candidate, index)?;
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(DesignTournamentError::DuplicateCandidateId);
        }

        for trace in &candidate.scenarios {
            if !trace_ids.insert(trace.trace_id.as_str()) {
                return Err(DesignTournamentError::DuplicateTraceId);
            }
            if trace.evidence_kind != evidence_kind {
                return Err(DesignTournamentError::MixedEvidenceKinds);
            }
            if trace.frame_id != frame_id {
                return Err(DesignTournamentError::FrameIdentityMismatch);
            }
            if trace.grid_id != grid_id {
                return Err(DesignTournamentError::GridIdentityMismatch);
            }
            if !same_angles(first_trace, trace) {
                return Err(DesignTournamentError::GridCoordinatesMismatch);
            }
        }

        let scenario_metrics =
            evaluate_operating_condition_scenarios(&candidate.candidate_id, &candidate.scenarios, requirements)
                .map_err(DesignTournamentError::ScenarioEvaluation)?;
        let accepted_all_scenarios = scenario_metrics.all_scenarios_accepted;

        results.push(MagneticDesignCandidateMetrics {
            candidate_id: candidate.candidate_id.clone(),
            geometry_sha256: candidate.geometry_sha256.clone(),
            material_sha256: candidate.material_sha256.clone(),
            field_archive_sha256: candidate.field_archive_sha256.clone(),
            solver_run_ids: candidate.solver_run_ids.clone(),
            scenario_metrics,
            accepted_all_scenarios,
            is_pareto_optimal: false,
        });
    }

    for index in 0..results.len() {
        if !results[index].accepted_all_scenarios {
            continue;
        }
        let dominated = results.iter().enumerate().any(|(other_index, other)| {
            other_index != index
                && other.accepted_all_scenarios
                && dominates(
                    &other.scenario_metrics,
                    &results[index].scenario_metrics,
                )
        });
        results[index].is_pareto_optimal = !dominated;
    }

    let accepted_candidate_count = results
        .iter()
        .filter(|candidate| candidate.accepted_all_scenarios)
        .count();
    let pareto_candidate_ids = results
        .iter()
        .filter(|candidate| candidate.is_pareto_optimal)
        .map(|candidate| candidate.candidate_id.clone())
        .collect();

    Ok(MagneticDesignTournament {
        tournament_id: tournament_id.to_owned(),
        evidence_kind,
        frame_id: frame_id.to_owned(),
        grid_id: grid_id.to_owned(),
        phase_zero_calibration_id: requirements.phase_zero_calibration_id.clone(),
        candidate_count: results.len(),
        accepted_candidate_count,
        pareto_candidate_ids,
        candidates: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

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

    fn trace(
        trace_id: &str,
        condition_id: &str,
        amplitude_t: f64,
        phase_bias_rad: f64,
        evidence_kind: EvidenceKind,
    ) -> FieldTrace {
        let samples = (0..256)
            .map(|index| {
                let mechanical = TAU * index as f64 / 256.0;
                let electrical = 4.0 * mechanical + phase_bias_rad;
                FieldTraceSample {
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
            evidence_kind,
            samples,
        }
    }

    fn candidate(
        id: &str,
        amplitude_t: f64,
        phase_bias_rad: f64,
        evidence_kind: EvidenceKind,
    ) -> MagneticDesignCandidate {
        MagneticDesignCandidate {
            candidate_id: id.to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            field_archive_sha256: DIGEST_C.to_owned(),
            solver_run_ids: vec![format!("solver-run-{id}")],
            scenarios: vec![
                trace(
                    &format!("{id}-nominal"),
                    "nominal",
                    amplitude_t,
                    phase_bias_rad,
                    evidence_kind,
                ),
                trace(
                    &format!("{id}-gap"),
                    "increased-gap",
                    amplitude_t * 0.9,
                    phase_bias_rad,
                    evidence_kind,
                ),
            ],
        }
    }

    #[test]
    fn same_designs_produce_identical_deterministic_tournament_results() {
        let candidates = vec![
            candidate("design-a", 1.0, 0.01, EvidenceKind::SyntheticScenario),
            candidate("design-b", 1.2, 0.02, EvidenceKind::SyntheticScenario),
        ];
        let first = rank_magnetic_designs("tournament-001", &candidates, &requirements())
            .expect("valid tournament");
        let second = rank_magnetic_designs("tournament-001", &candidates, &requirements())
            .expect("same inputs");
        assert_eq!(first, second);
    }

    #[test]
    fn any_failing_operating_corner_rejects_the_whole_design() {
        let mut design = candidate("design-corner", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        design.scenarios.push(trace(
            "design-corner-stray-field",
            "external-stray-field",
            1.0,
            0.2,
            EvidenceKind::SyntheticScenario,
        ));
        let result =
            rank_magnetic_designs("tournament-002", &[design], &requirements()).expect("valid set");
        assert_eq!(result.accepted_candidate_count, 0);
        assert!(!result.candidates[0].accepted_all_scenarios);
    }

    #[test]
    fn tradeoff_designs_remain_pareto_optimal_and_dominated_design_is_removed() {
        let candidates = vec![
            candidate("high-margin", 1.4, 0.01, EvidenceKind::SyntheticScenario),
            candidate("low-phase", 1.0, 0.0, EvidenceKind::SyntheticScenario),
            candidate("dominated", 0.8, 0.03, EvidenceKind::SyntheticScenario),
        ];
        let result =
            rank_magnetic_designs("tournament-003", &candidates, &requirements()).unwrap();
        assert!(result.pareto_candidate_ids.contains(&"high-margin".to_owned()));
        assert!(result.pareto_candidate_ids.contains(&"low-phase".to_owned()));
        assert!(!result.pareto_candidate_ids.contains(&"dominated".to_owned()));
    }

    #[test]
    fn mixed_evidence_classes_are_rejected_across_candidates() {
        let candidates = vec![
            candidate("synthetic", 1.0, 0.01, EvidenceKind::SyntheticScenario),
            candidate("physical", 1.0, 0.01, EvidenceKind::PhysicalBuild),
        ];
        assert_eq!(
            rank_magnetic_designs("tournament-004", &candidates, &requirements())
                .expect_err("cannot mix evidence classes"),
            DesignTournamentError::MixedEvidenceKinds
        );
    }

    #[test]
    fn frame_and_grid_mismatches_are_rejected() {
        let mut wrong_frame =
            candidate("wrong-frame", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        wrong_frame.scenarios[0].frame_id = "different-frame".to_owned();
        assert_eq!(
            rank_magnetic_designs("tournament-005", &[wrong_frame], &requirements())
                .expect_err("frame mismatch"),
            DesignTournamentError::FrameIdentityMismatch
        );

        let mut wrong_grid =
            candidate("wrong-grid", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        wrong_grid.scenarios[0].samples[10].mechanical_angle_rad += 1.0e-6;
        assert_eq!(
            rank_magnetic_designs("tournament-006", &[wrong_grid], &requirements())
                .expect_err("angle grid mismatch"),
            DesignTournamentError::GridCoordinatesMismatch
        );
    }

    #[test]
    fn duplicate_candidate_trace_and_solver_ids_are_rejected() {
        let duplicate_candidates = vec![
            candidate("duplicate", 1.0, 0.01, EvidenceKind::SyntheticScenario),
            candidate("duplicate", 1.1, 0.01, EvidenceKind::SyntheticScenario),
        ];
        assert_eq!(
            rank_magnetic_designs("tournament-007", &duplicate_candidates, &requirements())
                .expect_err("duplicate candidate"),
            DesignTournamentError::DuplicateCandidateId
        );

        let mut duplicate_trace =
            candidate("duplicate-trace", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        duplicate_trace.scenarios[1].trace_id = duplicate_trace.scenarios[0].trace_id.clone();
        assert_eq!(
            rank_magnetic_designs("tournament-008", &[duplicate_trace], &requirements())
                .expect_err("duplicate trace"),
            DesignTournamentError::DuplicateTraceId
        );

        let mut duplicate_run =
            candidate("duplicate-run", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        duplicate_run.solver_run_ids.push(duplicate_run.solver_run_ids[0].clone());
        assert_eq!(
            rank_magnetic_designs("tournament-009", &[duplicate_run], &requirements())
                .expect_err("duplicate solver run"),
            DesignTournamentError::DuplicateSolverRunId { index: 0 }
        );
    }

    #[test]
    fn invalid_digests_and_empty_sources_are_rejected() {
        let mut invalid_digest =
            candidate("bad-digest", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        invalid_digest.geometry_sha256 = "not-a-sha256".to_owned();
        assert_eq!(
            rank_magnetic_designs("tournament-010", &[invalid_digest], &requirements())
                .expect_err("invalid digest"),
            DesignTournamentError::InvalidSha256 {
                index: 0,
                field: "geometry_sha256",
            }
        );

        let mut no_solver =
            candidate("no-solver", 1.0, 0.01, EvidenceKind::SyntheticScenario);
        no_solver.solver_run_ids.clear();
        assert_eq!(
            rank_magnetic_designs("tournament-011", &[no_solver], &requirements())
                .expect_err("solver provenance required"),
            DesignTournamentError::EmptySolverRunSet { index: 0 }
        );
    }

    #[test]
    fn tournament_requires_nonempty_identity_and_candidates() {
        assert_eq!(
            rank_magnetic_designs("", &[], &requirements())
                .expect_err("missing tournament ID"),
            DesignTournamentError::InvalidTournamentId
        );
        assert_eq!(
            rank_magnetic_designs("tournament-012", &[], &requirements())
                .expect_err("empty candidate set"),
            DesignTournamentError::EmptyCandidateSet
        );
    }
}
