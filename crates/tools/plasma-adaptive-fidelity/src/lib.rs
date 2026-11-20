//! Fail-closed local policy for choosing a qualified moment order or kinetic patch.
//!
//! This crate consumes independently produced evidence declarations. It does
//! not compute model error, hyperbolicity, realizability, conservation, or run
//! a solver. The output is a policy decision/request only, never actuation.

pub const SCHEMA_VERSION: &str = "luminous.plasma-adaptive-fidelity.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Pass,
    Unknown,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCompatibility {
    pub physics_model_id: String,
    pub species_scope_id: String,
    pub moment_family_id: String,
    pub basis_id: String,
    pub basis_normalization_id: String,
    pub geometry_id: String,
    pub boundary_semantics_id: String,
    pub mesh_sha256: String,
    pub solver_build_sha256: String,
    pub tableau_sha256: String,
    pub time_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvidence {
    pub disposition: EvidenceDisposition,
    pub evidence_sha256: String,
    pub evaluated_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibratedErrorBound {
    pub disposition: EvidenceDisposition,
    pub method_id: String,
    pub error_metric_id: String,
    pub error_unit_id: String,
    pub upper_bound: f64,
    pub calibration_receipt_sha256: String,
    pub evaluated_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EigenbasisConditionEvidence {
    pub disposition: EvidenceDisposition,
    pub upper_bound: f64,
    pub admissible_limit: f64,
    pub evidence_sha256: String,
    pub evaluated_state_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MomentOrderCandidate {
    pub profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub compatibility: ModelCompatibility,
    pub parallel_order_p: u32,
    pub perpendicular_order_q: u32,
    pub calibrated_error: CalibratedErrorBound,
    pub hyperbolicity: GateEvidence,
    pub realizability: GateEvidence,
    pub conservation: GateEvidence,
    pub eigenbasis_condition: EigenbasisConditionEvidence,
    pub computational_cost: f64,
    pub cost_unit_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KineticPatchProfile {
    pub patch_profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub compatibility: ModelCompatibility,
    pub qualification: GateEvidence,
    pub computational_cost: f64,
    pub cost_unit_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveFidelityRequest {
    pub schema_version: String,
    pub request_id: String,
    pub local_state_sha256: String,
    pub compatibility: ModelCompatibility,
    pub local_error_tolerance: f64,
    pub error_metric_id: String,
    pub error_unit_id: String,
    pub cost_unit_id: String,
    pub moment_candidates: Vec<MomentOrderCandidate>,
    pub kinetic_patch: Option<KineticPatchProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IneligibilityReason {
    EmptyProfileId,
    InvalidProfileDigest,
    InvalidModelDigest,
    CompatibilityMismatch,
    ZeroParallelMomentOrder,
    ErrorEstimatorUnknown,
    ErrorEstimatorRejected,
    ErrorEstimatorStateMismatch,
    ErrorEstimatorIdentityMismatch,
    InvalidErrorUpperBound,
    ErrorUpperBoundExceedsTolerance,
    InvalidCalibrationDigest,
    HyperbolicityUnknown,
    HyperbolicityRejected,
    HyperbolicityStateMismatch,
    InvalidHyperbolicityDigest,
    RealizabilityUnknown,
    RealizabilityRejected,
    RealizabilityStateMismatch,
    InvalidRealizabilityDigest,
    ConservationUnknown,
    ConservationRejected,
    ConservationStateMismatch,
    InvalidConservationDigest,
    EigenbasisConditionUnknown,
    EigenbasisConditionRejected,
    EigenbasisConditionStateMismatch,
    InvalidEigenbasisConditionDigest,
    InvalidEigenbasisCondition,
    EigenbasisConditionExceeded,
    KineticPatchStateMismatch,
    InvalidKineticPatchEvidenceDigest,
    KineticPatchQualificationUnknown,
    KineticPatchQualificationRejected,
    InvalidComputationalCost,
    CostUnitMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateIneligibility {
    pub profile_id: String,
    pub reason: IneligibilityReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedMomentOrder {
    pub profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub parallel_order_p: u32,
    pub perpendicular_order_q: u32,
    pub calibrated_error_upper_bound: f64,
    pub eigenbasis_condition_upper_bound: f64,
    pub eigenbasis_condition_evidence_sha256: String,
    pub computational_cost: f64,
    pub cost_unit_id: String,
    pub hyperbolicity_evidence_sha256: String,
    pub realizability_evidence_sha256: String,
    pub conservation_evidence_sha256: String,
    pub calibration_receipt_sha256: String,
    pub ineligible_candidates: Vec<CandidateIneligibility>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KineticPatchRequest {
    pub patch_profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub computational_cost: f64,
    pub cost_unit_id: String,
    pub qualification_evidence_sha256: String,
    pub ineligible_candidates: Vec<CandidateIneligibility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    NoCandidateProfiles,
    NoAdmissibleMomentOrder,
    KineticPatchQualificationUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    SelectedMomentOrder(SelectedMomentOrder),
    RequestKineticPatch(KineticPatchRequest),
    Unknown {
        reason: UnknownReason,
        ineligible_candidates: Vec<CandidateIneligibility>,
    },
    Refused {
        ineligible_candidates: Vec<CandidateIneligibility>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    UnsupportedSchemaVersion,
    EmptyIdentity(&'static str),
    InvalidLocalStateDigest,
    InvalidCompatibilityDigest(&'static str),
    InvalidErrorTolerance,
    InvalidRequestCompatibility,
    DuplicateCandidateProfile(String),
}

#[derive(Debug, Clone)]
struct EligibleCandidate<'a> {
    candidate: &'a MomentOrderCandidate,
}

#[derive(Debug, Clone)]
enum CandidateAssessment {
    Eligible,
    Unknown(IneligibilityReason),
    Rejected(IneligibilityReason),
}

/// Evaluate a single local state against independently qualified model-order
/// profiles. No solver state is mutated and no action is dispatched.
pub fn evaluate_adaptive_fidelity(
    request: &AdaptiveFidelityRequest,
) -> Result<PolicyDecision, PolicyError> {
    validate_request(request)?;

    let mut seen = std::collections::BTreeSet::new();
    for candidate in &request.moment_candidates {
        if !seen.insert(candidate.profile_id.clone()) {
            return Err(PolicyError::DuplicateCandidateProfile(
                candidate.profile_id.clone(),
            ));
        }
    }

    let mut eligible: Vec<EligibleCandidate<'_>> = Vec::new();
    let mut rejected = Vec::new();
    let mut has_unknown_candidate = false;

    for candidate in &request.moment_candidates {
        match assess_candidate(candidate, request) {
            CandidateAssessment::Eligible => eligible.push(EligibleCandidate { candidate }),
            CandidateAssessment::Unknown(reason) => {
                has_unknown_candidate = true;
                rejected.push(CandidateIneligibility {
                    profile_id: candidate.profile_id.clone(),
                    reason,
                });
            }
            CandidateAssessment::Rejected(reason) => rejected.push(CandidateIneligibility {
                profile_id: candidate.profile_id.clone(),
                reason,
            }),
        }
    }
    rejected.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));

    eligible.sort_by(|left, right| {
        let a = left.candidate;
        let b = right.candidate;
        a.computational_cost
            .total_cmp(&b.computational_cost)
            .then_with(|| {
                ((u128::from(a.parallel_order_p) + 1) * (u128::from(a.perpendicular_order_q) + 1))
                    .cmp(&((u128::from(b.parallel_order_p) + 1) * (u128::from(b.perpendicular_order_q) + 1)))
            })
            .then_with(|| a.parallel_order_p.cmp(&b.parallel_order_p))
            .then_with(|| a.perpendicular_order_q.cmp(&b.perpendicular_order_q))
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });

    if let Some(selected) = eligible.first().map(|item| item.candidate) {
        return Ok(PolicyDecision::SelectedMomentOrder(SelectedMomentOrder {
            profile_id: selected.profile_id.clone(),
            profile_sha256: selected.profile_sha256.clone(),
            model_sha256: selected.model_sha256.clone(),
            parallel_order_p: selected.parallel_order_p,
            perpendicular_order_q: selected.perpendicular_order_q,
            calibrated_error_upper_bound: selected.calibrated_error.upper_bound,
            eigenbasis_condition_upper_bound: selected.eigenbasis_condition.upper_bound,
            eigenbasis_condition_evidence_sha256: selected.eigenbasis_condition.evidence_sha256.clone(),
            computational_cost: selected.computational_cost,
            cost_unit_id: selected.cost_unit_id.clone(),
            hyperbolicity_evidence_sha256: selected.hyperbolicity.evidence_sha256.clone(),
            realizability_evidence_sha256: selected.realizability.evidence_sha256.clone(),
            conservation_evidence_sha256: selected.conservation.evidence_sha256.clone(),
            calibration_receipt_sha256: selected.calibrated_error.calibration_receipt_sha256.clone(),
            ineligible_candidates: rejected,
        }));
    }

    if request.moment_candidates.is_empty() {
        if let Some(patch) = &request.kinetic_patch {
            match assess_kinetic_patch(patch, request) {
                CandidateAssessment::Eligible => {
                    return Ok(PolicyDecision::RequestKineticPatch(KineticPatchRequest {
                        patch_profile_id: patch.patch_profile_id.clone(),
                        profile_sha256: patch.profile_sha256.clone(),
                        model_sha256: patch.model_sha256.clone(),
                        computational_cost: patch.computational_cost,
                        cost_unit_id: patch.cost_unit_id.clone(),
                        qualification_evidence_sha256: patch.qualification.evidence_sha256.clone(),
                        ineligible_candidates: rejected,
                    }));
                }
                CandidateAssessment::Unknown(reason) => {
                    rejected.push(CandidateIneligibility {
                        profile_id: patch.patch_profile_id.clone(),
                        reason,
                    });
                    rejected.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
                    return Ok(PolicyDecision::Unknown {
                        reason: UnknownReason::KineticPatchQualificationUnknown,
                        ineligible_candidates: rejected,
                    });
                }
                CandidateAssessment::Rejected(reason) => {
                    rejected.push(CandidateIneligibility {
                        profile_id: patch.patch_profile_id.clone(),
                        reason,
                    });
                    rejected.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
                    return Ok(PolicyDecision::Refused {
                        ineligible_candidates: rejected,
                        reason: "kinetic patch is ineligible".to_owned(),
                    });
                }
            }
        }
        return Ok(PolicyDecision::Unknown {
            reason: UnknownReason::NoCandidateProfiles,
            ineligible_candidates: rejected,
        });
    }

    if let Some(patch) = &request.kinetic_patch {
        match assess_kinetic_patch(patch, request) {
            CandidateAssessment::Eligible => {
                return Ok(PolicyDecision::RequestKineticPatch(KineticPatchRequest {
                    patch_profile_id: patch.patch_profile_id.clone(),
                    profile_sha256: patch.profile_sha256.clone(),
                    model_sha256: patch.model_sha256.clone(),
                    computational_cost: patch.computational_cost,
                    cost_unit_id: patch.cost_unit_id.clone(),
                    qualification_evidence_sha256: patch.qualification.evidence_sha256.clone(),
                    ineligible_candidates: rejected,
                }));
            }
            CandidateAssessment::Unknown(reason) => {
                rejected.push(CandidateIneligibility {
                    profile_id: patch.patch_profile_id.clone(),
                    reason,
                });
                rejected.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
                return Ok(PolicyDecision::Unknown {
                    reason: UnknownReason::KineticPatchQualificationUnknown,
                    ineligible_candidates: rejected,
                });
            }
            CandidateAssessment::Rejected(reason) => {
                rejected.push(CandidateIneligibility {
                    profile_id: patch.patch_profile_id.clone(),
                    reason,
                });
                rejected.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
            }
        }
    }

    if has_unknown_candidate {
        Ok(PolicyDecision::Unknown {
            reason: UnknownReason::NoAdmissibleMomentOrder,
            ineligible_candidates: rejected,
        })
    } else {
        Ok(PolicyDecision::Refused {
            ineligible_candidates: rejected,
            reason: "no moment order satisfies the local error and physical-admissibility gates, and no qualified kinetic patch is available".to_owned(),
        })
    }
}

fn validate_request(request: &AdaptiveFidelityRequest) -> Result<(), PolicyError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(PolicyError::UnsupportedSchemaVersion);
    }
    if request.request_id.trim().is_empty() {
        return Err(PolicyError::EmptyIdentity("request_id"));
    }
    if !is_sha256(&request.local_state_sha256) {
        return Err(PolicyError::InvalidLocalStateDigest);
    }
    if !request.local_error_tolerance.is_finite() || request.local_error_tolerance < 0.0 {
        return Err(PolicyError::InvalidErrorTolerance);
    }
    for (name, value) in [
        ("error_metric_id", request.error_metric_id.as_str()),
        ("error_unit_id", request.error_unit_id.as_str()),
        ("cost_unit_id", request.cost_unit_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PolicyError::EmptyIdentity(name));
        }
    }
    validate_compatibility(&request.compatibility)
}

fn validate_compatibility(value: &ModelCompatibility) -> Result<(), PolicyError> {
    for (name, identity) in [
        ("physics_model_id", value.physics_model_id.as_str()),
        ("species_scope_id", value.species_scope_id.as_str()),
        ("moment_family_id", value.moment_family_id.as_str()),
        ("basis_id", value.basis_id.as_str()),
        ("basis_normalization_id", value.basis_normalization_id.as_str()),
        ("geometry_id", value.geometry_id.as_str()),
        ("boundary_semantics_id", value.boundary_semantics_id.as_str()),
        ("time_unit_id", value.time_unit_id.as_str()),
    ] {
        if identity.trim().is_empty() {
            return Err(PolicyError::InvalidRequestCompatibility);
        }
        let _ = name;
    }
    if !is_sha256(&value.mesh_sha256) {
        return Err(PolicyError::InvalidCompatibilityDigest("mesh_sha256"));
    }
    if !is_sha256(&value.solver_build_sha256) {
        return Err(PolicyError::InvalidCompatibilityDigest("solver_build_sha256"));
    }
    if !is_sha256(&value.tableau_sha256) {
        return Err(PolicyError::InvalidCompatibilityDigest("tableau_sha256"));
    }
    Ok(())
}

fn assess_candidate(
    candidate: &MomentOrderCandidate,
    request: &AdaptiveFidelityRequest,
) -> CandidateAssessment {
    if candidate.profile_id.trim().is_empty() {
        return CandidateAssessment::Rejected(IneligibilityReason::EmptyProfileId);
    }
    if !is_sha256(&candidate.profile_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidProfileDigest);
    }
    if !is_sha256(&candidate.model_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidModelDigest);
    }
    if candidate.compatibility != request.compatibility {
        return CandidateAssessment::Rejected(IneligibilityReason::CompatibilityMismatch);
    }
    if candidate.parallel_order_p == 0 {
        return CandidateAssessment::Rejected(IneligibilityReason::ZeroParallelMomentOrder);
    }
    if candidate.cost_unit_id != request.cost_unit_id {
        return CandidateAssessment::Rejected(IneligibilityReason::CostUnitMismatch);
    }
    if !candidate.computational_cost.is_finite() || candidate.computational_cost <= 0.0 {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidComputationalCost);
    }

    let error = &candidate.calibrated_error;
    if error.evaluated_state_sha256 != request.local_state_sha256 {
        return CandidateAssessment::Rejected(IneligibilityReason::ErrorEstimatorStateMismatch);
    }
    if error.method_id.trim().is_empty()
        || error.error_metric_id != request.error_metric_id
        || error.error_unit_id != request.error_unit_id
    {
        return CandidateAssessment::Rejected(IneligibilityReason::ErrorEstimatorIdentityMismatch);
    }
    if !is_sha256(&error.calibration_receipt_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidCalibrationDigest);
    }
    if !error.upper_bound.is_finite() || error.upper_bound < 0.0 {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidErrorUpperBound);
    }
    match error.disposition {
        EvidenceDisposition::Pass => {}
        EvidenceDisposition::Unknown => {
            return CandidateAssessment::Unknown(IneligibilityReason::ErrorEstimatorUnknown);
        }
        EvidenceDisposition::Rejected => {
            return CandidateAssessment::Rejected(IneligibilityReason::ErrorEstimatorRejected);
        }
    }
    if error.upper_bound > request.local_error_tolerance {
        return CandidateAssessment::Rejected(IneligibilityReason::ErrorUpperBoundExceedsTolerance);
    }

    if let Some(reason) = assess_gate(&candidate.hyperbolicity, &request.local_state_sha256,
        IneligibilityReason::HyperbolicityUnknown,
        IneligibilityReason::HyperbolicityRejected,
        IneligibilityReason::HyperbolicityStateMismatch,
        IneligibilityReason::InvalidHyperbolicityDigest)
    {
        return reason;
    }
    if let Some(reason) = assess_gate(&candidate.realizability, &request.local_state_sha256,
        IneligibilityReason::RealizabilityUnknown,
        IneligibilityReason::RealizabilityRejected,
        IneligibilityReason::RealizabilityStateMismatch,
        IneligibilityReason::InvalidRealizabilityDigest)
    {
        return reason;
    }
    if let Some(reason) = assess_gate(&candidate.conservation, &request.local_state_sha256,
        IneligibilityReason::ConservationUnknown,
        IneligibilityReason::ConservationRejected,
        IneligibilityReason::ConservationStateMismatch,
        IneligibilityReason::InvalidConservationDigest)
    {
        return reason;
    }

    let eigenbasis = &candidate.eigenbasis_condition;
    if eigenbasis.evaluated_state_sha256 != request.local_state_sha256 {
        return CandidateAssessment::Rejected(IneligibilityReason::EigenbasisConditionStateMismatch);
    }
    if !is_sha256(&eigenbasis.evidence_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidEigenbasisConditionDigest);
    }
    match eigenbasis.disposition {
        EvidenceDisposition::Pass => {}
        EvidenceDisposition::Unknown => {
            return CandidateAssessment::Unknown(IneligibilityReason::EigenbasisConditionUnknown);
        }
        EvidenceDisposition::Rejected => {
            return CandidateAssessment::Rejected(IneligibilityReason::EigenbasisConditionRejected);
        }
    }
    if !eigenbasis.upper_bound.is_finite()
        || eigenbasis.upper_bound <= 0.0
        || !eigenbasis.admissible_limit.is_finite()
        || eigenbasis.admissible_limit <= 0.0
    {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidEigenbasisCondition);
    }
    if eigenbasis.upper_bound > eigenbasis.admissible_limit {
        return CandidateAssessment::Rejected(IneligibilityReason::EigenbasisConditionExceeded);
    }

    CandidateAssessment::Eligible
}

fn assess_kinetic_patch(
    patch: &KineticPatchProfile,
    request: &AdaptiveFidelityRequest,
) -> CandidateAssessment {
    if patch.patch_profile_id.trim().is_empty() {
        return CandidateAssessment::Rejected(IneligibilityReason::EmptyProfileId);
    }
    if !is_sha256(&patch.profile_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidProfileDigest);
    }
    if !is_sha256(&patch.model_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidModelDigest);
    }
    if patch.compatibility != request.compatibility {
        return CandidateAssessment::Rejected(IneligibilityReason::CompatibilityMismatch);
    }
    if patch.cost_unit_id != request.cost_unit_id {
        return CandidateAssessment::Rejected(IneligibilityReason::CostUnitMismatch);
    }
    if !patch.computational_cost.is_finite() || patch.computational_cost <= 0.0 {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidComputationalCost);
    }
    if patch.qualification.evaluated_state_sha256 != request.local_state_sha256 {
        return CandidateAssessment::Rejected(IneligibilityReason::KineticPatchStateMismatch);
    }
    if !is_sha256(&patch.qualification.evidence_sha256) {
        return CandidateAssessment::Rejected(IneligibilityReason::InvalidKineticPatchEvidenceDigest);
    }
    match patch.qualification.disposition {
        EvidenceDisposition::Pass => CandidateAssessment::Eligible,
        EvidenceDisposition::Unknown => {
            CandidateAssessment::Unknown(IneligibilityReason::KineticPatchQualificationUnknown)
        }
        EvidenceDisposition::Rejected => {
            CandidateAssessment::Rejected(IneligibilityReason::KineticPatchQualificationRejected)
        }
    }
}

fn assess_gate(
    gate: &GateEvidence,
    expected_state_sha256: &str,
    unknown: IneligibilityReason,
    rejected: IneligibilityReason,
    state_mismatch: IneligibilityReason,
    invalid_digest: IneligibilityReason,
) -> Option<CandidateAssessment> {
    if gate.evaluated_state_sha256 != expected_state_sha256 {
        return Some(CandidateAssessment::Rejected(state_mismatch));
    }
    if !is_sha256(&gate.evidence_sha256) {
        return Some(CandidateAssessment::Rejected(invalid_digest));
    }
    match gate.disposition {
        EvidenceDisposition::Pass => None,
        EvidenceDisposition::Unknown => Some(CandidateAssessment::Unknown(unknown)),
        EvidenceDisposition::Rejected => Some(CandidateAssessment::Rejected(rejected)),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> ModelCompatibility {
        ModelCompatibility {
            physics_model_id: "synthetic-vlasov-drift-kinetic-1d".into(),
            species_scope_id: "synthetic-electrons".into(),
            moment_family_id: "synthetic-hermite-laguerre".into(),
            basis_id: "synthetic-hermite-laguerre-basis".into(),
            basis_normalization_id: "synthetic-normalization-v1".into(),
            geometry_id: "synthetic-slab".into(),
            boundary_semantics_id: "synthetic-periodic".into(),
            mesh_sha256: digest('a'),
            solver_build_sha256: digest('b'),
            tableau_sha256: digest('c'),
            time_unit_id: "normalized-time".into(),
        }
    }

    fn gate(disposition: EvidenceDisposition, character: char) -> GateEvidence {
        GateEvidence {
            disposition,
            evidence_sha256: digest(character),
            evaluated_state_sha256: digest('f'),
        }
    }

    fn candidate(
        profile_id: &str,
        p: u32,
        q: u32,
        cost: f64,
        error: f64,
    ) -> MomentOrderCandidate {
        MomentOrderCandidate {
            profile_id: profile_id.into(),
            profile_sha256: digest('1'),
            model_sha256: digest('2'),
            compatibility: compatibility(),
            parallel_order_p: p,
            perpendicular_order_q: q,
            calibrated_error: CalibratedErrorBound {
                disposition: EvidenceDisposition::Pass,
                method_id: "synthetic-a-posteriori-error-bound".into(),
                error_metric_id: "normalized-moment-l2".into(),
                error_unit_id: "dimensionless".into(),
                upper_bound: error,
                calibration_receipt_sha256: digest('3'),
                evaluated_state_sha256: digest('f'),
            },
            hyperbolicity: gate(EvidenceDisposition::Pass, '4'),
            realizability: gate(EvidenceDisposition::Pass, '5'),
            conservation: gate(EvidenceDisposition::Pass, '6'),
            eigenbasis_condition: EigenbasisConditionEvidence {
                disposition: EvidenceDisposition::Pass,
                upper_bound: 4.0,
                admissible_limit: 10.0,
                evidence_sha256: digest('a'),
                evaluated_state_sha256: digest('f'),
            },
            computational_cost: cost,
            cost_unit_id: "normalized-cost-unit".into(),
        }
    }

    fn request(candidates: Vec<MomentOrderCandidate>) -> AdaptiveFidelityRequest {
        AdaptiveFidelityRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: "synthetic-policy-request-1".into(),
            local_state_sha256: digest('f'),
            compatibility: compatibility(),
            local_error_tolerance: 0.05,
            error_metric_id: "normalized-moment-l2".into(),
            error_unit_id: "dimensionless".into(),
            cost_unit_id: "normalized-cost-unit".into(),
            moment_candidates: candidates,
            kinetic_patch: None,
        }
    }

    fn kinetic_patch() -> KineticPatchProfile {
        KineticPatchProfile {
            patch_profile_id: "synthetic-kinetic-patch".into(),
            profile_sha256: digest('7'),
            model_sha256: digest('8'),
            compatibility: compatibility(),
            qualification: gate(EvidenceDisposition::Pass, '9'),
            computational_cost: 20.0,
            cost_unit_id: "normalized-cost-unit".into(),
        }
    }

    #[test]
    fn selects_lowest_cost_eligible_order() {
        let result = evaluate_adaptive_fidelity(&request(vec![
            candidate("high", 8, 8, 10.0, 0.01),
            candidate("low", 4, 4, 2.0, 0.04),
        ]))
        .expect("policy evaluates");
        assert!(matches!(
            result,
            PolicyDecision::SelectedMomentOrder(SelectedMomentOrder {
                profile_id,
                parallel_order_p: 4,
                perpendicular_order_q: 4,
                ..
            }) if profile_id == "low"
        ));
    }

    #[test]
    fn selection_is_independent_of_candidate_input_order() {
        let a = candidate("order-a", 4, 4, 2.0, 0.04);
        let b = candidate("order-b", 8, 8, 3.0, 0.01);
        let first = evaluate_adaptive_fidelity(&request(vec![a.clone(), b.clone()])).unwrap();
        let second = evaluate_adaptive_fidelity(&request(vec![b, a])).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn higher_order_is_selected_when_lower_order_exceeds_local_error_tolerance() {
        let result = evaluate_adaptive_fidelity(&request(vec![
            candidate("low", 4, 4, 1.0, 0.08),
            candidate("high", 8, 8, 4.0, 0.03),
        ]))
        .unwrap();
        assert!(matches!(
            result,
            PolicyDecision::SelectedMomentOrder(ref selected)
                if selected.profile_id == "high" && selected.calibrated_error_upper_bound == 0.03
        ));
    }

    #[test]
    fn unknown_hyperbolicity_does_not_block_a_separately_qualified_candidate() {
        let mut unknown = candidate("unknown", 4, 4, 1.0, 0.01);
        unknown.hyperbolicity.disposition = EvidenceDisposition::Unknown;
        let result = evaluate_adaptive_fidelity(&request(vec![
            unknown,
            candidate("qualified", 6, 6, 3.0, 0.03),
        ]))
        .unwrap();
        assert!(matches!(
            result,
            PolicyDecision::SelectedMomentOrder(ref selected)
                if selected.profile_id == "qualified"
                    && selected.ineligible_candidates.len() == 1
        ));
    }

    #[test]
    fn non_hyperbolic_candidate_is_never_selected() {
        let mut invalid = candidate("non-hyperbolic", 8, 8, 1.0, 0.01);
        invalid.hyperbolicity.disposition = EvidenceDisposition::Rejected;
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(result, PolicyDecision::Refused { .. }));
    }

    #[test]
    fn ill_conditioned_eigenbasis_is_rejected_even_with_real_eigenvalue_certificate() {
        let mut invalid = candidate("ill-conditioned", 8, 8, 1.0, 0.01);
        invalid.eigenbasis_condition.upper_bound = 1.0e8;
        invalid.eigenbasis_condition.admissible_limit = 1.0e5;
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates.iter().any(|item|
                item.reason == IneligibilityReason::EigenbasisConditionExceeded)
        ));
    }

    #[test]
    fn state_mismatched_error_estimate_is_rejected_as_stale() {
        let mut invalid = candidate("stale", 4, 4, 1.0, 0.01);
        invalid.calibrated_error.evaluated_state_sha256 = digest('0');
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates.iter().any(|item|
                item.reason == IneligibilityReason::ErrorEstimatorStateMismatch)
        ));
    }

    #[test]
    fn basis_or_solver_mismatch_is_rejected() {
        let mut invalid = candidate("wrong-basis", 4, 4, 1.0, 0.01);
        invalid.compatibility.basis_id = "different-basis".into();
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates[0].reason == IneligibilityReason::CompatibilityMismatch
        ));
    }

    #[test]
    fn qualified_kinetic_patch_is_requested_when_no_moment_order_is_adequate() {
        let mut req = request(vec![
            candidate("low", 4, 4, 1.0, 0.2),
            candidate("unstable-high", 8, 8, 3.0, 0.01),
        ]);
        req.moment_candidates[1].hyperbolicity.disposition = EvidenceDisposition::Rejected;
        req.kinetic_patch = Some(kinetic_patch());
        let result = evaluate_adaptive_fidelity(&req).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::RequestKineticPatch(ref patch)
                if patch.patch_profile_id == "synthetic-kinetic-patch"
        ));
    }

    #[test]
    fn unknown_kinetic_patch_qualification_returns_unknown_not_execution() {
        let mut req = request(vec![]);
        let mut patch = kinetic_patch();
        patch.qualification.disposition = EvidenceDisposition::Unknown;
        req.kinetic_patch = Some(patch);
        assert_eq!(
            evaluate_adaptive_fidelity(&req).unwrap(),
            PolicyDecision::Unknown {
                reason: UnknownReason::KineticPatchQualificationUnknown,
                ineligible_candidates: vec![],
            }
        );
    }


    #[test]
    fn unknown_eigenbasis_condition_keeps_the_policy_unknown() {
        let mut invalid = candidate("unresolved-eigenbasis", 4, 4, 1.0, 0.01);
        invalid.eigenbasis_condition.disposition = EvidenceDisposition::Unknown;
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Unknown {
                reason: UnknownReason::NoAdmissibleMomentOrder,
                ineligible_candidates,
            } if ineligible_candidates.iter().any(|item|
                item.reason == IneligibilityReason::EigenbasisConditionUnknown)
        ));
    }

    #[test]
    fn kinetic_patch_state_mismatch_is_refused_not_requested() {
        let mut req = request(vec![]);
        let mut patch = kinetic_patch();
        patch.qualification.evaluated_state_sha256 = digest('0');
        req.kinetic_patch = Some(patch);
        assert!(matches!(
            evaluate_adaptive_fidelity(&req).unwrap(),
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates.iter().any(|item|
                item.profile_id == "synthetic-kinetic-patch"
                    && item.reason == IneligibilityReason::KineticPatchStateMismatch)
        ));
    }

    #[test]
    fn no_admissible_order_and_no_kinetic_patch_is_refused() {
        let mut invalid = candidate("invalid", 4, 4, 1.0, 0.01);
        invalid.conservation.disposition = EvidenceDisposition::Rejected;
        assert!(matches!(
            evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap(),
            PolicyDecision::Refused { .. }
        ));
    }

    #[test]
    fn duplicate_candidate_profile_identity_is_rejected() {
        assert_eq!(
            evaluate_adaptive_fidelity(&request(vec![
                candidate("same", 4, 4, 1.0, 0.01),
                candidate("same", 8, 8, 2.0, 0.02),
            ])),
            Err(PolicyError::DuplicateCandidateProfile("same".into()))
        );
    }

    #[test]
    fn candidate_with_different_cost_unit_is_rejected() {
        let mut invalid = candidate("wrong-cost-unit", 4, 4, 1.0, 0.01);
        invalid.cost_unit_id = "seconds".into();
        let result = evaluate_adaptive_fidelity(&request(vec![invalid])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates[0].reason == IneligibilityReason::CostUnitMismatch
        ));
    }


    #[test]
    fn zero_perpendicular_order_is_supported_for_reduced_model_families() {
        let mut reduced = candidate("parallel-only", 4, 4, 1.0, 0.01);
        reduced.perpendicular_order_q = 0;
        let result = evaluate_adaptive_fidelity(&request(vec![reduced])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::SelectedMomentOrder(ref selected)
                if selected.profile_id == "parallel-only"
                    && selected.perpendicular_order_q == 0
        ));
    }

    #[test]
    fn qualified_kinetic_patch_can_be_requested_without_moment_candidates() {
        let mut req = request(vec![]);
        req.kinetic_patch = Some(kinetic_patch());
        assert!(matches!(
            evaluate_adaptive_fidelity(&req).unwrap(),
            PolicyDecision::RequestKineticPatch(ref patch)
                if patch.patch_profile_id == "synthetic-kinetic-patch"
        ));
    }

    #[test]
    fn candidate_order_tie_uses_lower_moment_complexity_then_profile_id() {
        let result = evaluate_adaptive_fidelity(&request(vec![
            candidate("higher-complexity", 8, 8, 2.0, 0.01),
            candidate("lower-complexity", 4, 4, 2.0, 0.01),
        ]))
        .unwrap();
        assert!(matches!(
            result,
            PolicyDecision::SelectedMomentOrder(ref selected)
                if selected.profile_id == "lower-complexity"
        ));
    }

    #[test]
    fn non_finite_candidate_error_or_cost_is_rejected() {
        let mut invalid_error = candidate("nan-error", 4, 4, 1.0, 0.01);
        invalid_error.calibrated_error.upper_bound = f64::NAN;
        let result = evaluate_adaptive_fidelity(&request(vec![invalid_error])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates[0].reason == IneligibilityReason::InvalidErrorUpperBound
        ));

        let mut invalid_cost = candidate("infinite-cost", 8, 8, 1.0, 0.01);
        invalid_cost.computational_cost = f64::INFINITY;
        let result = evaluate_adaptive_fidelity(&request(vec![invalid_cost])).unwrap();
        assert!(matches!(
            result,
            PolicyDecision::Refused {
                ineligible_candidates,
                ..
            } if ineligible_candidates[0].reason == IneligibilityReason::InvalidComputationalCost
        ));
    }

    #[test]
    fn no_candidate_profiles_and_no_patch_remains_unknown() {
        assert_eq!(
            evaluate_adaptive_fidelity(&request(vec![])).unwrap(),
            PolicyDecision::Unknown {
                reason: UnknownReason::NoCandidateProfiles,
                ineligible_candidates: vec![],
            }
        );
    }

    #[test]
    fn request_rejects_invalid_top_level_local_state_or_tolerance() {
        let mut invalid_state = request(vec![]);
        invalid_state.local_state_sha256 = "unknown".into();
        assert_eq!(
            evaluate_adaptive_fidelity(&invalid_state),
            Err(PolicyError::InvalidLocalStateDigest)
        );

        let mut invalid_tolerance = request(vec![]);
        invalid_tolerance.local_error_tolerance = f64::NAN;
        assert_eq!(
            evaluate_adaptive_fidelity(&invalid_tolerance),
            Err(PolicyError::InvalidErrorTolerance)
        );
    }
}
