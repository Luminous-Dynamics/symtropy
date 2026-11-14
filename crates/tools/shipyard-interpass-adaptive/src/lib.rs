//! Evidence-gated, offline inter-pass recommendation contract.
//!
//! V0 accepts synthetic contract observations only. It does not parse BAM raw
//! files, qualify a welding procedure, predict physical weld quality, or
//! authorize robot execution. Every numeric objective is a symbolic unit.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredGeometry,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcedureScope {
    SyntheticEnvelopeOnly,
    UnverifiedPhysicalProcedure,
    QualifiedPhysicalProcedure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanQuality {
    pub registered: bool,
    pub coverage_permille: u16,
    pub minimum_coverage_permille: u16,
    pub occlusion_permille: u16,
    pub maximum_occlusion_permille: u16,
    pub registration_error_units: u64,
    pub maximum_registration_error_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedState {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub after_pass_id: String,
    pub scan_id: String,
    pub scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub coordinate_frame: String,
    pub residual_fill_area_units: u64,
    pub uncertainty_bound_units: u64,
    pub observed_at_tick: u64,
    pub quality: ScanQuality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcedureEnvelope {
    pub procedure_id: String,
    pub revision: String,
    pub scope: ProcedureScope,
    pub minimum_deposition_area_units: u64,
    pub maximum_deposition_area_units: u64,
    pub live_execution_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassCandidate {
    pub candidate_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub source_scan_id: String,
    pub source_scan_sha256: [u8; 32],
    pub source_registration_receipt_sha256: [u8; 32],
    pub procedure_id: String,
    pub procedure_revision: String,
    pub deposition_area_units: u64,
    pub duration_units: u64,
    pub material_units: u64,
    pub energy_units: u64,
    pub robustness_risk_units: u64,
    pub within_declared_procedure_envelope: bool,
    pub resource_feasible: bool,
    pub path_reachable: bool,
    pub collision_free: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerInput {
    pub observed_state: ObservedState,
    pub active_workpiece_id: String,
    pub active_joint_revision: String,
    pub active_layer_id: String,
    pub expected_predecessor_pass_id: String,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub maximum_accepted_uncertainty_units: u64,
    pub future_information_used: bool,
    pub model_domain_supported: bool,
    pub live_execution_requested: bool,
    pub procedure: ProcedureEnvelope,
    pub candidates: Vec<PassCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    PhysicalOrSimulationEvidenceNotAdmitted,
    ScanNotRegistered,
    ScanDigestMissing,
    RegistrationReceiptMissing,
    ScanCoverageBelowMinimum,
    ScanOcclusionAboveMaximum,
    ScanRegistrationErrorTooLarge,
    UncertaintyNotBounded,
    UncertaintyExceedsDecisionLimit,
    StalePredecessorState,
    ObservationFromFuture,
    ObservationTooOld,
    FutureInformationLeakage,
    ModelOutsideDeclaredDomain,
    ProcedureScopeNotAdmitted,
    LiveExecutionForbidden,
    InvalidProcedureEnvelope,
    DuplicateCandidateIdentity,
    NoFeasibleCandidate,
    NumericOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateExclusion {
    IdentityMismatch,
    WrongSourceScan,
    WrongProcedureRevision,
    OutsideDeclaredProcedureEnvelope,
    ResourceInfeasible,
    PathUnreachable,
    Collision,
    ZeroDuration,
    ZeroMaterial,
    DepositionOutsideProcedureBounds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveVector {
    pub robust_geometry_error_units: u64,
    pub duration_units: u64,
    pub material_units: u64,
    pub energy_units: u64,
    pub robustness_risk_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub candidate_id: String,
    pub objectives: ObjectiveVector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedCandidate {
    pub candidate_id: String,
    pub reason: CandidateExclusion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Abstain {
        reason: AbstainReason,
        excluded: Vec<ExcludedCandidate>,
    },
    ParetoRecommendations {
        recommendations: Vec<Recommendation>,
        feasible_candidate_count: usize,
        excluded: Vec<ExcludedCandidate>,
        automatic_selection: bool,
        live_execution_authorized: bool,
        claim_ceiling: &'static str,
    },
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn validate_observation(input: &PlannerInput) -> Result<(), AbstainReason> {
    let state = &input.observed_state;
    let identities = [
        state.workpiece_id.as_str(),
        state.joint_revision.as_str(),
        state.layer_id.as_str(),
        state.after_pass_id.as_str(),
        state.scan_id.as_str(),
        state.coordinate_frame.as_str(),
        input.active_workpiece_id.as_str(),
        input.active_joint_revision.as_str(),
        input.active_layer_id.as_str(),
        input.expected_predecessor_pass_id.as_str(),
    ];
    if identities.into_iter().any(is_empty) {
        return Err(AbstainReason::EmptyIdentity);
    }

    if state.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(AbstainReason::PhysicalOrSimulationEvidenceNotAdmitted);
    }
    if input.procedure.scope != ProcedureScope::SyntheticEnvelopeOnly {
        return Err(AbstainReason::ProcedureScopeNotAdmitted);
    }
    if input.procedure.live_execution_authorized || input.live_execution_requested {
        return Err(AbstainReason::LiveExecutionForbidden);
    }
    if !input.model_domain_supported {
        return Err(AbstainReason::ModelOutsideDeclaredDomain);
    }
    if input.future_information_used {
        return Err(AbstainReason::FutureInformationLeakage);
    }

    if state.workpiece_id != input.active_workpiece_id
        || state.joint_revision != input.active_joint_revision
        || state.layer_id != input.active_layer_id
        || state.after_pass_id != input.expected_predecessor_pass_id
    {
        return Err(AbstainReason::StalePredecessorState);
    }
    if state.observed_at_tick > input.decision_at_tick {
        return Err(AbstainReason::ObservationFromFuture);
    }
    if input
        .decision_at_tick
        .saturating_sub(state.observed_at_tick)
        > input.maximum_observation_age_ticks
    {
        return Err(AbstainReason::ObservationTooOld);
    }

    if !state.quality.registered {
        return Err(AbstainReason::ScanNotRegistered);
    }
    if state.scan_sha256.iter().all(|byte| *byte == 0) {
        return Err(AbstainReason::ScanDigestMissing);
    }
    if state
        .registration_receipt_sha256
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(AbstainReason::RegistrationReceiptMissing);
    }
    if state.quality.minimum_coverage_permille > 1_000
        || state.quality.coverage_permille > 1_000
        || state.quality.coverage_permille < state.quality.minimum_coverage_permille
    {
        return Err(AbstainReason::ScanCoverageBelowMinimum);
    }
    if state.quality.maximum_occlusion_permille > 1_000
        || state.quality.occlusion_permille > 1_000
        || state.quality.occlusion_permille > state.quality.maximum_occlusion_permille
    {
        return Err(AbstainReason::ScanOcclusionAboveMaximum);
    }
    if state.quality.registration_error_units > state.quality.maximum_registration_error_units {
        return Err(AbstainReason::ScanRegistrationErrorTooLarge);
    }
    if input.procedure.procedure_id.trim().is_empty()
        || input.procedure.revision.trim().is_empty()
        || input.procedure.minimum_deposition_area_units
            > input.procedure.maximum_deposition_area_units
    {
        return Err(AbstainReason::InvalidProcedureEnvelope);
    }

    if state.uncertainty_bound_units == 0 {
        return Err(AbstainReason::UncertaintyNotBounded);
    }
    if state.uncertainty_bound_units > input.maximum_accepted_uncertainty_units {
        return Err(AbstainReason::UncertaintyExceedsDecisionLimit);
    }
    state
        .residual_fill_area_units
        .checked_add(state.uncertainty_bound_units)
        .ok_or(AbstainReason::NumericOverflow)?;
    Ok(())
}

fn objectives_for(
    state: &ObservedState,
    candidate: &PassCandidate,
) -> Result<ObjectiveVector, AbstainReason> {
    let low = state
        .residual_fill_area_units
        .saturating_sub(state.uncertainty_bound_units);
    let high = state
        .residual_fill_area_units
        .checked_add(state.uncertainty_bound_units)
        .ok_or(AbstainReason::NumericOverflow)?;
    let underfill = high.abs_diff(candidate.deposition_area_units);
    let overfill = low.abs_diff(candidate.deposition_area_units);
    Ok(ObjectiveVector {
        robust_geometry_error_units: underfill.max(overfill),
        duration_units: candidate.duration_units,
        material_units: candidate.material_units,
        energy_units: candidate.energy_units,
        robustness_risk_units: candidate.robustness_risk_units,
    })
}

fn exclusion_for(input: &PlannerInput, candidate: &PassCandidate) -> Option<CandidateExclusion> {
    let state = &input.observed_state;
    if is_empty(&candidate.candidate_id)
        || candidate.workpiece_id != state.workpiece_id
        || candidate.joint_revision != state.joint_revision
        || candidate.layer_id != state.layer_id
        || candidate.predecessor_pass_id != state.after_pass_id
    {
        return Some(CandidateExclusion::IdentityMismatch);
    }
    if candidate.source_scan_id != state.scan_id
        || candidate.source_scan_sha256 != state.scan_sha256
        || candidate.source_registration_receipt_sha256 != state.registration_receipt_sha256
    {
        return Some(CandidateExclusion::WrongSourceScan);
    }
    if candidate.procedure_id != input.procedure.procedure_id
        || candidate.procedure_revision != input.procedure.revision
    {
        return Some(CandidateExclusion::WrongProcedureRevision);
    }
    if !candidate.within_declared_procedure_envelope {
        return Some(CandidateExclusion::OutsideDeclaredProcedureEnvelope);
    }
    if !candidate.resource_feasible {
        return Some(CandidateExclusion::ResourceInfeasible);
    }
    if !candidate.path_reachable {
        return Some(CandidateExclusion::PathUnreachable);
    }
    if !candidate.collision_free {
        return Some(CandidateExclusion::Collision);
    }
    if candidate.duration_units == 0 {
        return Some(CandidateExclusion::ZeroDuration);
    }
    if candidate.material_units == 0 {
        return Some(CandidateExclusion::ZeroMaterial);
    }
    if candidate.deposition_area_units < input.procedure.minimum_deposition_area_units
        || candidate.deposition_area_units > input.procedure.maximum_deposition_area_units
    {
        return Some(CandidateExclusion::DepositionOutsideProcedureBounds);
    }
    None
}

fn dominates(left: &ObjectiveVector, right: &ObjectiveVector) -> bool {
    let left_values = [
        left.robust_geometry_error_units,
        left.duration_units,
        left.material_units,
        left.energy_units,
        left.robustness_risk_units,
    ];
    let right_values = [
        right.robust_geometry_error_units,
        right.duration_units,
        right.material_units,
        right.energy_units,
        right.robustness_risk_units,
    ];
    let mut strictly_better = false;
    for (left_value, right_value) in left_values.into_iter().zip(right_values) {
        if left_value > right_value {
            return false;
        }
        if left_value < right_value {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Produce a recommendation frontier from the latest admissible synthetic scan.
///
/// The function never picks a single pass automatically and never authorizes
/// live execution. All outputs use symbolic units and are not physical weld
/// predictions, WPS qualification, NDT decisions or production authority.
pub fn recommend_next_pass(input: &PlannerInput) -> Decision {
    if let Err(reason) = validate_observation(input) {
        return Decision::Abstain {
            reason,
            excluded: Vec::new(),
        };
    }

    let mut ids = HashSet::new();
    for candidate in &input.candidates {
        if !ids.insert(candidate.candidate_id.as_str()) {
            return Decision::Abstain {
                reason: AbstainReason::DuplicateCandidateIdentity,
                excluded: Vec::new(),
            };
        }
    }

    let mut feasible = Vec::new();
    let mut excluded = Vec::new();
    for candidate in &input.candidates {
        if let Some(reason) = exclusion_for(input, candidate) {
            excluded.push(ExcludedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            });
            continue;
        }
        match objectives_for(&input.observed_state, candidate) {
            Ok(objectives) => feasible.push(Recommendation {
                candidate_id: candidate.candidate_id.clone(),
                objectives,
            }),
            Err(reason) => {
                return Decision::Abstain { reason, excluded };
            }
        }
    }

    if feasible.is_empty() {
        return Decision::Abstain {
            reason: AbstainReason::NoFeasibleCandidate,
            excluded,
        };
    }

    let frontier: Vec<Recommendation> = feasible
        .iter()
        .filter(|candidate| {
            !feasible.iter().any(|other| {
                other.candidate_id != candidate.candidate_id
                    && dominates(&other.objectives, &candidate.objectives)
            })
        })
        .cloned()
        .collect();

    let mut recommendations = frontier;
    recommendations.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    Decision::ParetoRecommendations {
        recommendations,
        feasible_candidate_count: feasible.len(),
        excluded,
        automatic_selection: false,
        live_execution_authorized: false,
        claim_ceiling: "SYNTHETIC_INTERPASS_PLAN_CONTRACT_ONLY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> PlannerInput {
        let state = ObservedState {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            after_pass_id: "PASS-02".to_owned(),
            scan_id: "SCAN-AFTER-PASS-02".to_owned(),
            scan_sha256: [0x11; 32],
            registration_receipt_sha256: [0x22; 32],
            coordinate_frame: "SYNTHETIC-JOINT-FRAME".to_owned(),
            residual_fill_area_units: 100,
            uncertainty_bound_units: 10,
            observed_at_tick: 100,
            quality: ScanQuality {
                registered: true,
                coverage_permille: 995,
                minimum_coverage_permille: 980,
                occlusion_permille: 5,
                maximum_occlusion_permille: 50,
                registration_error_units: 1,
                maximum_registration_error_units: 2,
            },
        };
        let procedure = ProcedureEnvelope {
            procedure_id: "SYNTHETIC-PROC-01".to_owned(),
            revision: "R1".to_owned(),
            scope: ProcedureScope::SyntheticEnvelopeOnly,
            minimum_deposition_area_units: 60,
            maximum_deposition_area_units: 140,
            live_execution_authorized: false,
        };
        let make_candidate = |id: &str, deposition, time, material, energy, risk| PassCandidate {
            candidate_id: id.to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            predecessor_pass_id: "PASS-02".to_owned(),
            source_scan_id: "SCAN-AFTER-PASS-02".to_owned(),
            source_scan_sha256: [0x11; 32],
            source_registration_receipt_sha256: [0x22; 32],
            procedure_id: "SYNTHETIC-PROC-01".to_owned(),
            procedure_revision: "R1".to_owned(),
            deposition_area_units: deposition,
            duration_units: time,
            material_units: material,
            energy_units: energy,
            robustness_risk_units: risk,
            within_declared_procedure_envelope: true,
            resource_feasible: true,
            path_reachable: true,
            collision_free: true,
        };
        let candidates = vec![
            make_candidate("FILL-UNDER", 82, 12, 82, 70, 1),
            make_candidate("FILL-MID", 100, 15, 100, 80, 2),
            make_candidate("FILL-OVER", 130, 10, 130, 60, 1),
        ];
        PlannerInput {
            observed_state: state,
            active_workpiece_id: "WID-SYNTH-01".to_owned(),
            active_joint_revision: "JOINT-R1".to_owned(),
            active_layer_id: "LAYER-03".to_owned(),
            expected_predecessor_pass_id: "PASS-02".to_owned(),
            decision_at_tick: 100,
            maximum_observation_age_ticks: 10,
            maximum_accepted_uncertainty_units: 30,
            future_information_used: false,
            model_domain_supported: true,
            live_execution_requested: false,
            procedure,
            candidates,
        }
    }

    #[test]
    fn returns_a_pareto_set_without_automatically_picking_a_pass() {
        let result = recommend_next_pass(&base_input());
        let Decision::ParetoRecommendations {
            recommendations,
            feasible_candidate_count,
            automatic_selection,
            live_execution_authorized,
            claim_ceiling,
            ..
        } = result
        else {
            panic!("valid synthetic observation should produce a recommendation frontier");
        };
        assert_eq!(feasible_candidate_count, 3);
        assert_eq!(
            recommendations
                .iter()
                .map(|item| item.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["FILL-MID", "FILL-OVER", "FILL-UNDER"]
        );
        assert_eq!(
            recommendations[0].objectives.robust_geometry_error_units,
            10
        );
        assert_eq!(
            recommendations[1].objectives.robust_geometry_error_units,
            40
        );
        assert_eq!(
            recommendations[2].objectives.robust_geometry_error_units,
            28
        );
        assert!(!automatic_selection);
        assert!(!live_execution_authorized);
        assert_eq!(claim_ceiling, "SYNTHETIC_INTERPASS_PLAN_CONTRACT_ONLY");
    }

    #[test]
    fn geometry_only_and_time_only_baselines_choose_different_passes() {
        let Decision::ParetoRecommendations {
            recommendations, ..
        } = recommend_next_pass(&base_input())
        else {
            panic!("valid synthetic observation should produce recommendations");
        };
        let geometry_only = recommendations
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
                    .then_with(|| left.candidate_id.cmp(&right.candidate_id))
            })
            .expect("geometry-only baseline candidate");
        let time_only = recommendations
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .duration_units
                    .cmp(&right.objectives.duration_units)
                    .then_with(|| left.candidate_id.cmp(&right.candidate_id))
            })
            .expect("time-only baseline candidate");
        assert_eq!(geometry_only.candidate_id, "FILL-MID");
        assert_eq!(time_only.candidate_id, "FILL-OVER");
        assert_eq!(recommendations.len(), 3);
    }

    #[test]
    fn receding_horizon_replans_from_the_new_after_pass_scan() {
        let first = recommend_next_pass(&base_input());
        let Decision::ParetoRecommendations {
            recommendations: first_frontier,
            ..
        } = first
        else {
            panic!("first observation should produce recommendations");
        };
        let first_geometry_best = first_frontier
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
            })
            .expect("first geometry baseline");
        assert_eq!(first_geometry_best.candidate_id, "FILL-MID");

        let mut second_input = base_input();
        second_input.observed_state.after_pass_id = "PASS-03".to_owned();
        second_input.observed_state.scan_id = "SCAN-AFTER-PASS-03".to_owned();
        second_input.observed_state.scan_sha256 = [0x33; 32];
        second_input.observed_state.registration_receipt_sha256 = [0x44; 32];
        second_input.observed_state.residual_fill_area_units = 75;
        second_input.observed_state.uncertainty_bound_units = 5;
        second_input.observed_state.observed_at_tick = 110;
        second_input.expected_predecessor_pass_id = "PASS-03".to_owned();
        second_input.decision_at_tick = 110;

        let next_actions = [("FILL-65", 65), ("FILL-75", 75), ("FILL-90", 90)];
        for (candidate, (candidate_id, deposition)) in
            second_input.candidates.iter_mut().zip(next_actions)
        {
            candidate.candidate_id = candidate_id.to_owned();
            candidate.predecessor_pass_id = "PASS-03".to_owned();
            candidate.source_scan_id = "SCAN-AFTER-PASS-03".to_owned();
            candidate.source_scan_sha256 = [0x33; 32];
            candidate.source_registration_receipt_sha256 = [0x44; 32];
            candidate.deposition_area_units = deposition;
        }

        let second = recommend_next_pass(&second_input);
        let Decision::ParetoRecommendations {
            recommendations: second_frontier,
            ..
        } = second
        else {
            panic!("new observation should produce a new recommendation frontier");
        };
        let second_geometry_best = second_frontier
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
            })
            .expect("second geometry baseline");
        assert_eq!(second_geometry_best.candidate_id, "FILL-75");
        assert_ne!(
            first_geometry_best.candidate_id,
            second_geometry_best.candidate_id
        );
    }

    #[test]
    fn stale_predecessor_scan_causes_abstention() {
        let mut input = base_input();
        input.observed_state.after_pass_id = "PASS-01".to_owned();
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::StalePredecessorState,
                ..
            }
        ));
    }

    #[test]
    fn unregistered_scan_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.registered = false;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanNotRegistered,
                ..
            }
        ));
    }

    #[test]
    fn low_coverage_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.coverage_permille = 900;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanCoverageBelowMinimum,
                ..
            }
        ));
    }

    #[test]
    fn excessive_occlusion_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.occlusion_permille = 80;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanOcclusionAboveMaximum,
                ..
            }
        ));
    }

    #[test]
    fn excessive_registration_error_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.registration_error_units = 3;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanRegistrationErrorTooLarge,
                ..
            }
        ));
    }

    #[test]
    fn future_observation_causes_abstention() {
        let mut input = base_input();
        input.observed_state.observed_at_tick = 101;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationFromFuture,
                ..
            }
        ));
    }

    #[test]
    fn old_observation_causes_abstention() {
        let mut input = base_input();
        input.observed_state.observed_at_tick = 50;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationTooOld,
                ..
            }
        ));
    }

    #[test]
    fn future_information_leakage_causes_abstention() {
        let mut input = base_input();
        input.future_information_used = true;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::FutureInformationLeakage,
                ..
            }
        ));
    }

    #[test]
    fn out_of_domain_model_causes_abstention() {
        let mut input = base_input();
        input.model_domain_supported = false;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ModelOutsideDeclaredDomain,
                ..
            }
        ));
    }

    #[test]
    fn measured_or_simulated_data_are_not_yet_admitted_by_v0() {
        let mut input = base_input();
        input.observed_state.evidence_class = EvidenceClass::MeasuredGeometry;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::PhysicalOrSimulationEvidenceNotAdmitted,
                ..
            }
        ));
    }

    #[test]
    fn live_execution_is_never_authorized() {
        let mut input = base_input();
        input.live_execution_requested = true;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::LiveExecutionForbidden,
                ..
            }
        ));
    }

    #[test]
    fn candidate_bound_to_another_scan_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_scan_id = "STALE-SCAN".to_owned();
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should be feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn procedure_revision_mismatch_is_excluded() {
        let mut input = base_input();
        input.candidates[0].procedure_revision = "R0".to_owned();
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should be feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(
            excluded[0].reason,
            CandidateExclusion::WrongProcedureRevision
        );
    }

    #[test]
    fn infeasible_candidates_are_not_pareto_options() {
        let mut input = base_input();
        input.candidates[0].collision_free = false;
        input.candidates[1].resource_feasible = false;
        let Decision::ParetoRecommendations {
            recommendations,
            feasible_candidate_count,
            excluded,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("one feasible candidate remains");
        };
        assert_eq!(feasible_candidate_count, 1);
        assert_eq!(recommendations.len(), 1);
        assert_eq!(excluded.len(), 2);
    }

    #[test]
    fn candidate_with_wrong_scan_digest_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_scan_sha256 = [0x33; 32];
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should remain feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn candidate_with_wrong_registration_receipt_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_registration_receipt_sha256 = [0x44; 32];
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should remain feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn missing_scan_digest_causes_abstention() {
        let mut input = base_input();
        input.observed_state.scan_sha256 = [0; 32];
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanDigestMissing,
                ..
            }
        ));
    }

    #[test]
    fn missing_registration_receipt_causes_abstention() {
        let mut input = base_input();
        input.observed_state.registration_receipt_sha256 = [0; 32];
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::RegistrationReceiptMissing,
                ..
            }
        ));
    }

    #[test]
    fn no_feasible_candidates_causes_abstention() {
        let mut input = base_input();
        for candidate in &mut input.candidates {
            candidate.resource_feasible = false;
        }
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::NoFeasibleCandidate,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_candidate_identity_causes_abstention() {
        let mut input = base_input();
        input.candidates[1].candidate_id = input.candidates[0].candidate_id.clone();
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::DuplicateCandidateIdentity,
                ..
            }
        ));
    }

    #[test]
    fn robust_geometry_error_accounts_for_measurement_uncertainty() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 25;
        let Decision::ParetoRecommendations {
            recommendations, ..
        } = recommend_next_pass(&input)
        else {
            panic!("valid synthetic input should be admitted");
        };
        let middle = recommendations
            .iter()
            .find(|r| r.candidate_id == "FILL-MID")
            .unwrap();
        assert_eq!(middle.objectives.robust_geometry_error_units, 25);
    }

    #[test]
    fn physical_procedure_scope_is_not_implied_by_synthetic_contract() {
        let mut input = base_input();
        input.procedure.scope = ProcedureScope::UnverifiedPhysicalProcedure;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ProcedureScopeNotAdmitted,
                ..
            }
        ));
    }

    #[test]
    fn zero_uncertainty_is_not_admitted_as_a_measured_bound() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 0;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::UncertaintyNotBounded,
                ..
            }
        ));
    }

    #[test]
    fn empty_candidate_identity_is_excluded() {
        let mut input = base_input();
        input.candidates[0].candidate_id.clear();
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should remain feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::IdentityMismatch);
    }

    #[test]
    fn uncertainty_above_decision_limit_causes_abstention() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 31;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::UncertaintyExceedsDecisionLimit,
                ..
            }
        ));
    }

    #[test]
    fn numeric_overflow_abstains_instead_of_wrapping() {
        let mut input = base_input();
        input.observed_state.residual_fill_area_units = u64::MAX;
        input.observed_state.uncertainty_bound_units = 1;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::NumericOverflow,
                ..
            }
        ));
    }
}
