//! Claim-specific admission policies for aerospace engineering evidence.
//!
//! This module does not verify CFD, certify artifacts, or authorize flight. It evaluates whether
//! caller-supplied, verifier-referenced evidence assertions satisfy a declared claim policy and
//! exact scope. Cryptographic trust in verifier identities is an external trust-store concern.

use std::collections::{BTreeMap, BTreeSet};

pub const CLAIM_ADMISSION_SCHEMA: &str = "aero-claim-admission/0.2.0";
pub const CLAIM_ADMISSION_EVIDENCE_LEVEL: &str =
    "claim_specific_policy_evaluation_with_exact_scope_and_full_evidence_manifest_digest_binding_not_artifact_verification_not_regulatory_approval_or_flight_authorization";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimClass {
    ExploratoryDesignRanking,
    AerodynamicPrediction,
    UnsteadyAerodynamicPrediction,
    StructuralLoadPrediction,
    AeroelasticStabilityPrediction,
    MissionPerformancePrediction,
    FlightRelease,
}

impl ClaimClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExploratoryDesignRanking => "exploratory_design_ranking",
            Self::AerodynamicPrediction => "aerodynamic_prediction",
            Self::UnsteadyAerodynamicPrediction => "unsteady_aerodynamic_prediction",
            Self::StructuralLoadPrediction => "structural_load_prediction",
            Self::AeroelasticStabilityPrediction => "aeroelastic_stability_prediction",
            Self::MissionPerformancePrediction => "mission_performance_prediction",
            Self::FlightRelease => "flight_release",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionConsequence {
    Exploratory,
    Engineering,
    High,
    SafetyCritical,
}

impl DecisionConsequence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exploratory => "exploratory",
            Self::Engineering => "engineering",
            Self::High => "high",
            Self::SafetyCritical => "safety_critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    GeometryIdentity,
    AnalysisInputsIdentity,
    SolverExecutableIdentity,
    ProcessReceipt,
    ConvergenceScreen,
    ComparisonProtocol,
    GridConvergence,
    TemporalConvergence,
    TimeStepSensitivity,
    ConservationAudit,
    StatisticalSamplingAssessment,
    ModelUncertaintyAssessment,
    ExperimentalValidation,
    MultivariateValidation,
    ValidationUncertaintyBudget,
    StructuralVerification,
    StructuralValidation,
    AeroelasticCouplingConvergence,
    FlutterValidation,
    PropulsionValidation,
    MissionModelVerification,
    MissionUncertaintyBudget,
    OperationalEnvelopeValidation,
    CalibrationEvidence,
    IndependentReview,
    CertificationBasisEvidence,
    CertificationAuthorityDecision,
    QualitySystemEvidence,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeometryIdentity => "geometry_identity",
            Self::AnalysisInputsIdentity => "analysis_inputs_identity",
            Self::SolverExecutableIdentity => "solver_executable_identity",
            Self::ProcessReceipt => "process_receipt",
            Self::ConvergenceScreen => "convergence_screen",
            Self::ComparisonProtocol => "comparison_protocol",
            Self::GridConvergence => "grid_convergence",
            Self::TemporalConvergence => "temporal_convergence",
            Self::TimeStepSensitivity => "time_step_sensitivity",
            Self::ConservationAudit => "conservation_audit",
            Self::StatisticalSamplingAssessment => "statistical_sampling_assessment",
            Self::ModelUncertaintyAssessment => "model_uncertainty_assessment",
            Self::ExperimentalValidation => "experimental_validation",
            Self::MultivariateValidation => "multivariate_validation",
            Self::ValidationUncertaintyBudget => "validation_uncertainty_budget",
            Self::StructuralVerification => "structural_verification",
            Self::StructuralValidation => "structural_validation",
            Self::AeroelasticCouplingConvergence => "aeroelastic_coupling_convergence",
            Self::FlutterValidation => "flutter_validation",
            Self::PropulsionValidation => "propulsion_validation",
            Self::MissionModelVerification => "mission_model_verification",
            Self::MissionUncertaintyBudget => "mission_uncertainty_budget",
            Self::OperationalEnvelopeValidation => "operational_envelope_validation",
            Self::CalibrationEvidence => "calibration_evidence",
            Self::IndependentReview => "independent_review",
            Self::CertificationBasisEvidence => "certification_basis_evidence",
            Self::CertificationAuthorityDecision => "certification_authority_decision_record",
            Self::QualitySystemEvidence => "quality_system_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceAssertion {
    Verified {
        artifact_sha256: String,
        verifier_id: String,
        verifier_sha256: String,
        verifier_receipt_sha256: String,
    },
    Unknown {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedEvidence {
    pub kind: EvidenceKind,
    /// Hash of the exact claim scope: geometry, configuration, operating point, model family,
    /// or a formally declared envelope. Evidence from another scope is not silently reusable.
    pub scope_sha256: String,
    pub assertion: EvidenceAssertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRequest {
    pub claim_id: String,
    pub claim_class: ClaimClass,
    pub consequence: DecisionConsequence,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimDecisionStatus {
    AdmittedWithinDeclaredScope,
    AbstainInsufficientOrUnknownEvidence,
    RejectedFailedOrMismatchedEvidence,
    RequiresHumanAuthorityDecision,
}

impl ClaimDecisionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdmittedWithinDeclaredScope => "admitted_within_declared_scope",
            Self::AbstainInsufficientOrUnknownEvidence => "abstain_insufficient_or_unknown_evidence",
            Self::RejectedFailedOrMismatchedEvidence => "rejected_failed_or_mismatched_evidence",
            Self::RequiresHumanAuthorityDecision => "requires_human_authority_decision",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimDecision {
    pub schema: String,
    pub claim_id: String,
    pub claim_class: ClaimClass,
    pub consequence: DecisionConsequence,
    pub scope_sha256: String,
    pub policy_sha256: String,
    /// Canonical digest over every supplied assertion, including verifier/artifact receipts and reasons.
    pub evidence_manifest_sha256: String,
    pub required_evidence: Vec<EvidenceKind>,
    pub verified_evidence: Vec<EvidenceKind>,
    pub unknown_evidence: Vec<EvidenceKind>,
    pub missing_evidence: Vec<EvidenceKind>,
    pub failed_evidence: Vec<EvidenceKind>,
    pub status: ClaimDecisionStatus,
    pub evidence_level: String,
    pub decision_sha256: String,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn add_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn add_hash(bytes: &mut Vec<u8>, value: &str) -> Result<(), String> {
    if !is_sha256(value) || value != value.to_ascii_lowercase() {
        return Err(
            "claim admission digest must be 64 lowercase hexadecimal characters".to_owned(),
        );
    }
    add_string(bytes, value);
    Ok(())
}

fn add_kinds(bytes: &mut Vec<u8>, kinds: &[EvidenceKind]) {
    bytes.extend_from_slice(&(kinds.len() as u64).to_le_bytes());
    for kind in kinds {
        add_string(bytes, kind.as_str());
    }
}

fn required_for(request: &ClaimRequest) -> BTreeSet<EvidenceKind> {
    use EvidenceKind as E;
    let mut required = BTreeSet::from([
        E::GeometryIdentity,
        E::AnalysisInputsIdentity,
        E::SolverExecutableIdentity,
        E::ProcessReceipt,
        E::ConvergenceScreen,
    ]);
    match request.claim_class {
        ClaimClass::ExploratoryDesignRanking => {
            required.insert(E::ComparisonProtocol);
        }
        ClaimClass::AerodynamicPrediction => {
            required.extend([
                E::GridConvergence,
                E::ConservationAudit,
                E::ModelUncertaintyAssessment,
                E::ExperimentalValidation,
                E::ValidationUncertaintyBudget,
            ]);
        }
        ClaimClass::UnsteadyAerodynamicPrediction => {
            required.extend([
                E::GridConvergence,
                E::TemporalConvergence,
                E::TimeStepSensitivity,
                E::ConservationAudit,
                E::StatisticalSamplingAssessment,
                E::ModelUncertaintyAssessment,
                E::ExperimentalValidation,
                E::MultivariateValidation,
                E::ValidationUncertaintyBudget,
            ]);
        }
        ClaimClass::StructuralLoadPrediction => {
            required.extend([
                E::GridConvergence,
                E::ConservationAudit,
                E::StructuralVerification,
                E::StructuralValidation,
                E::ModelUncertaintyAssessment,
                E::ValidationUncertaintyBudget,
            ]);
        }
        ClaimClass::AeroelasticStabilityPrediction => {
            required.extend([
                E::GridConvergence,
                E::TemporalConvergence,
                E::TimeStepSensitivity,
                E::ConservationAudit,
                E::StructuralVerification,
                E::StructuralValidation,
                E::AeroelasticCouplingConvergence,
                E::FlutterValidation,
                E::MultivariateValidation,
                E::ModelUncertaintyAssessment,
                E::ValidationUncertaintyBudget,
            ]);
        }
        ClaimClass::MissionPerformancePrediction => {
            required.extend([
                E::GridConvergence,
                E::ConservationAudit,
                E::ExperimentalValidation,
                E::StructuralVerification,
                E::StructuralValidation,
                E::PropulsionValidation,
                E::MissionModelVerification,
                E::MissionUncertaintyBudget,
                E::MultivariateValidation,
                E::ModelUncertaintyAssessment,
                E::ValidationUncertaintyBudget,
            ]);
        }
        ClaimClass::FlightRelease => {
            required.extend([
                E::GridConvergence,
                E::TemporalConvergence,
                E::TimeStepSensitivity,
                E::ConservationAudit,
                E::StatisticalSamplingAssessment,
                E::ExperimentalValidation,
                E::MultivariateValidation,
                E::ValidationUncertaintyBudget,
                E::StructuralVerification,
                E::StructuralValidation,
                E::AeroelasticCouplingConvergence,
                E::FlutterValidation,
                E::PropulsionValidation,
                E::MissionModelVerification,
                E::MissionUncertaintyBudget,
                E::OperationalEnvelopeValidation,
                E::CalibrationEvidence,
                E::IndependentReview,
                E::CertificationBasisEvidence,
                E::QualitySystemEvidence,
                E::ModelUncertaintyAssessment,
            ]);
        }
    }
    if matches!(
        request.consequence,
        DecisionConsequence::High | DecisionConsequence::SafetyCritical
    ) {
        required.insert(E::IndependentReview);
        required.insert(E::CalibrationEvidence);
        required.insert(E::ModelUncertaintyAssessment);
    }
    if request.consequence == DecisionConsequence::SafetyCritical {
        required.insert(E::MultivariateValidation);
    }
    required
}

fn policy_bytes(request: &ClaimRequest, required: &BTreeSet<EvidenceKind>) -> Vec<u8> {
    let mut bytes = Vec::new();
    add_string(&mut bytes, CLAIM_ADMISSION_SCHEMA);
    add_string(&mut bytes, request.claim_class.as_str());
    add_string(&mut bytes, request.consequence.as_str());
    add_kinds(&mut bytes, &required.iter().copied().collect::<Vec<_>>());
    bytes
}

fn evidence_manifest_bytes(
    supplied: &BTreeMap<EvidenceKind, &ScopedEvidence>,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    add_string(&mut bytes, "aero-claim-evidence-manifest/0.1.0");
    bytes.extend_from_slice(&(supplied.len() as u64).to_le_bytes());
    for (kind, item) in supplied {
        add_string(&mut bytes, kind.as_str());
        add_hash(&mut bytes, &item.scope_sha256)?;
        match &item.assertion {
            EvidenceAssertion::Verified {
                artifact_sha256,
                verifier_id,
                verifier_sha256,
                verifier_receipt_sha256,
            } => {
                bytes.push(1);
                add_hash(&mut bytes, artifact_sha256)?;
                add_string(&mut bytes, verifier_id);
                add_hash(&mut bytes, verifier_sha256)?;
                add_hash(&mut bytes, verifier_receipt_sha256)?;
            }
            EvidenceAssertion::Unknown { reason } => {
                bytes.push(2);
                add_string(&mut bytes, reason);
            }
            EvidenceAssertion::Failed { reason } => {
                bytes.push(3);
                add_string(&mut bytes, reason);
            }
        }
    }
    Ok(bytes)
}

fn decision_bytes(decision: &ClaimDecision) -> Vec<u8> {
    let mut bytes = Vec::new();
    add_string(&mut bytes, &decision.schema);
    add_string(&mut bytes, &decision.claim_id);
    add_string(&mut bytes, decision.claim_class.as_str());
    add_string(&mut bytes, decision.consequence.as_str());
    add_string(&mut bytes, &decision.scope_sha256);
    add_string(&mut bytes, &decision.policy_sha256);
    add_string(&mut bytes, &decision.evidence_manifest_sha256);
    add_kinds(&mut bytes, &decision.required_evidence);
    add_kinds(&mut bytes, &decision.verified_evidence);
    add_kinds(&mut bytes, &decision.unknown_evidence);
    add_kinds(&mut bytes, &decision.missing_evidence);
    add_kinds(&mut bytes, &decision.failed_evidence);
    add_string(&mut bytes, decision.status.as_str());
    add_string(&mut bytes, &decision.evidence_level);
    bytes
}

/// Evaluate evidence assertions against a claim-specific, exact-scope evidence policy.
///
/// A Verified assertion must carry artifact, verifier and verifier-receipt digests. This
/// function validates the assertion's structure and scope; it does not authenticate the
/// verifier against an external trust store or re-run the underlying artifact verifier.
pub fn evaluate_claim_admission(
    request: &ClaimRequest,
    evidence: &[ScopedEvidence],
) -> Result<ClaimDecision, String> {
    if request.claim_id.trim().is_empty() {
        return Err("claim_id must not be empty".to_owned());
    }
    add_hash(&mut Vec::new(), &request.scope_sha256)?;

    let required = required_for(request);
    let mut supplied = BTreeMap::new();
    let mut failed = BTreeSet::new();
    for item in evidence {
        add_hash(&mut Vec::new(), &item.scope_sha256)?;
        if supplied.insert(item.kind, item).is_some() {
            return Err(format!(
                "duplicate evidence assertion for {}",
                item.kind.as_str()
            ));
        }
        if item.scope_sha256 != request.scope_sha256 {
            failed.insert(item.kind);
            continue;
        }
        match &item.assertion {
            EvidenceAssertion::Verified {
                artifact_sha256,
                verifier_id,
                verifier_sha256,
                verifier_receipt_sha256,
            } => {
                add_hash(&mut Vec::new(), artifact_sha256)?;
                add_hash(&mut Vec::new(), verifier_sha256)?;
                add_hash(&mut Vec::new(), verifier_receipt_sha256)?;
                if verifier_id.trim().is_empty() {
                    return Err(format!(
                        "verified evidence {} has an empty verifier identity",
                        item.kind.as_str()
                    ));
                }
            }
            EvidenceAssertion::Unknown { reason } | EvidenceAssertion::Failed { reason } => {
                if reason.trim().is_empty() {
                    return Err(format!(
                        "non-verified evidence {} requires a reason",
                        item.kind.as_str()
                    ));
                }
            }
        }
    }

    let mut verified = BTreeSet::new();
    let mut unknown = BTreeSet::new();
    let mut missing = BTreeSet::new();
    for kind in &required {
        if failed.contains(kind) {
            continue;
        }
        match supplied.get(kind) {
            None => {
                missing.insert(*kind);
            }
            Some(ScopedEvidence {
                assertion: EvidenceAssertion::Verified { .. },
                ..
            }) => {
                verified.insert(*kind);
            }
            Some(ScopedEvidence {
                assertion: EvidenceAssertion::Unknown { .. },
                ..
            }) => {
                unknown.insert(*kind);
            }
            Some(ScopedEvidence {
                assertion: EvidenceAssertion::Failed { .. },
                ..
            }) => {
                failed.insert(*kind);
            }
        }
    }

    let evidence_manifest_sha256 =
        crate::integrity::sha256_hex(&evidence_manifest_bytes(&supplied)?)?;

    let status = if !failed.is_empty() {
        ClaimDecisionStatus::RejectedFailedOrMismatchedEvidence
    } else if !missing.is_empty() || !unknown.is_empty() {
        ClaimDecisionStatus::AbstainInsufficientOrUnknownEvidence
    } else if request.claim_class == ClaimClass::FlightRelease
        || request.consequence == DecisionConsequence::SafetyCritical
    {
        ClaimDecisionStatus::RequiresHumanAuthorityDecision
    } else {
        ClaimDecisionStatus::AdmittedWithinDeclaredScope
    };

    let policy_sha256 = {
        let bytes = policy_bytes(request, &required);
        crate::integrity::sha256_hex(&bytes)?
    };
    let mut decision = ClaimDecision {
        schema: CLAIM_ADMISSION_SCHEMA.to_owned(),
        claim_id: request.claim_id.clone(),
        claim_class: request.claim_class,
        consequence: request.consequence,
        scope_sha256: request.scope_sha256.clone(),
        policy_sha256,
        evidence_manifest_sha256,
        required_evidence: required.iter().copied().collect(),
        verified_evidence: verified.into_iter().collect(),
        unknown_evidence: unknown.into_iter().collect(),
        missing_evidence: missing.into_iter().collect(),
        failed_evidence: failed.into_iter().collect(),
        status,
        evidence_level: CLAIM_ADMISSION_EVIDENCE_LEVEL.to_owned(),
        decision_sha256: String::new(),
    };
    decision.decision_sha256 = crate::integrity::sha256_hex(&decision_bytes(&decision))?;
    Ok(decision)
}

/// Replay a claim decision from its original request and source assertions.
fn kinds_csv(kinds: &[EvidenceKind]) -> String {
    kinds.iter().map(|kind| kind.as_str()).collect::<Vec<_>>().join(";")
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const CLAIM_ADMISSION_CSV_HEADER: &str =
    "schema,claim_id,claim_class,consequence,scope_sha256,policy_sha256,evidence_manifest_sha256,required_evidence,verified_evidence,unknown_evidence,missing_evidence,failed_evidence,status,evidence_level,decision_sha256";

pub fn report_to_csv(decision: &ClaimDecision) -> String {
    let fields = [
        decision.schema.clone(),
        decision.claim_id.clone(),
        decision.claim_class.as_str().to_owned(),
        decision.consequence.as_str().to_owned(),
        decision.scope_sha256.clone(),
        decision.policy_sha256.clone(),
        decision.evidence_manifest_sha256.clone(),
        kinds_csv(&decision.required_evidence),
        kinds_csv(&decision.verified_evidence),
        kinds_csv(&decision.unknown_evidence),
        kinds_csv(&decision.missing_evidence),
        kinds_csv(&decision.failed_evidence),
        decision.status.as_str().to_owned(),
        decision.evidence_level.clone(),
        decision.decision_sha256.clone(),
    ];
    format!(
        "{}\n{}\n",
        CLAIM_ADMISSION_CSV_HEADER,
        fields.iter().map(|field| csv_escape(field)).collect::<Vec<_>>().join(",")
    )
}

pub fn request_to_csv(request: &ClaimRequest) -> String {
    let header = "claim_id,claim_class,consequence,scope_sha256";
    let fields = [
        request.claim_id.clone(),
        request.claim_class.as_str().to_owned(),
        request.consequence.as_str().to_owned(),
        request.scope_sha256.clone(),
    ];
    format!(
        "{header}\n{}\n",
        fields.iter().map(|field| csv_escape(field)).collect::<Vec<_>>().join(",")
    )
}

pub fn evidence_manifest_to_csv(evidence: &[ScopedEvidence]) -> Result<String, String> {
    let mut ordered = BTreeMap::new();
    for item in evidence {
        if ordered.insert(item.kind, item).is_some() {
            return Err(format!("duplicate evidence assertion for {}", item.kind.as_str()));
        }
    }
    let header = "kind,scope_sha256,assertion_status,artifact_sha256,verifier_id,verifier_sha256,verifier_receipt_sha256,reason";
    let mut output = format!("{header}\n");
    for (kind, item) in ordered {
        let (status, artifact, verifier_id, verifier, receipt, reason) =
            match &item.assertion {
                EvidenceAssertion::Verified {
                    artifact_sha256,
                    verifier_id,
                    verifier_sha256,
                    verifier_receipt_sha256,
                } => (
                    "verified",
                    artifact_sha256.as_str(),
                    verifier_id.as_str(),
                    verifier_sha256.as_str(),
                    verifier_receipt_sha256.as_str(),
                    "",
                ),
                EvidenceAssertion::Unknown { reason } => (
                    "unknown", "", "", "", "", reason.as_str(),
                ),
                EvidenceAssertion::Failed { reason } => (
                    "failed", "", "", "", "", reason.as_str(),
                ),
            };
        let fields = [
            kind.as_str(),
            item.scope_sha256.as_str(),
            status,
            artifact,
            verifier_id,
            verifier,
            receipt,
            reason,
        ];
        output.push_str(
            &fields.iter().map(|field| csv_escape(field)).collect::<Vec<_>>().join(","),
        );
        output.push('\n');
    }
    Ok(output)
}

pub fn verify_claim_admission(
    request: &ClaimRequest,
    evidence: &[ScopedEvidence],
    supplied: &ClaimDecision,
) -> Result<(), String> {
    if supplied.schema != CLAIM_ADMISSION_SCHEMA
        || supplied.evidence_level != CLAIM_ADMISSION_EVIDENCE_LEVEL
    {
        return Err("unsupported claim-admission schema or evidence ceiling".to_owned());
    }
    let recomputed = evaluate_claim_admission(request, evidence)?;
    if recomputed != *supplied {
        return Err("replayed claim-admission decision differs from supplied report".to_owned());
    }
    if crate::integrity::sha256_hex(&decision_bytes(supplied))? != supplied.decision_sha256 {
        return Err("claim-admission canonical digest mismatch".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn request(class: ClaimClass, consequence: DecisionConsequence) -> ClaimRequest {
        ClaimRequest {
            claim_id: "case-study-at-operating-point-a".to_owned(),
            claim_class: class,
            consequence,
            scope_sha256: digest('a'),
        }
    }

    fn verified(kind: EvidenceKind, scope_sha256: &str) -> ScopedEvidence {
        ScopedEvidence {
            kind,
            scope_sha256: scope_sha256.to_owned(),
            assertion: EvidenceAssertion::Verified {
                artifact_sha256: digest('b'),
                verifier_id: "pinned-independent-verifier-v1".to_owned(),
                verifier_sha256: digest('c'),
                verifier_receipt_sha256: digest('d'),
            },
        }
    }

    fn all_verified(request: &ClaimRequest) -> Vec<ScopedEvidence> {
        required_for(request)
            .into_iter()
            .map(|kind| verified(kind, &request.scope_sha256))
            .collect()
    }

    #[test]
    fn exploratory_claim_can_only_be_admitted_within_exact_declared_scope() {
        let request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        let evidence = all_verified(&request);
        let decision = evaluate_claim_admission(&request, &evidence).expect("valid assertions");
        assert_eq!(
            decision.status,
            ClaimDecisionStatus::AdmittedWithinDeclaredScope
        );
        verify_claim_admission(&request, &evidence, &decision).expect("independent replay");
    }

    #[test]
    fn higher_credibility_claim_abstains_when_validation_is_unknown_or_missing() {
        let request = request(
            ClaimClass::AerodynamicPrediction,
            DecisionConsequence::Engineering,
        );
        let mut evidence = all_verified(&request);
        evidence.retain(|item| item.kind != EvidenceKind::ExperimentalValidation);
        evidence.push(ScopedEvidence {
            kind: EvidenceKind::ExperimentalValidation,
            scope_sha256: request.scope_sha256.clone(),
            assertion: EvidenceAssertion::Unknown {
                reason: "no matched experimental observations admitted yet".to_owned(),
            },
        });
        let decision = evaluate_claim_admission(&request, &evidence).expect("valid assertions");
        assert_eq!(
            decision.status,
            ClaimDecisionStatus::AbstainInsufficientOrUnknownEvidence
        );
        assert!(decision.unknown_evidence.contains(&EvidenceKind::ExperimentalValidation));
    }

    #[test]
    fn evidence_from_another_geometry_or_operating_scope_is_rejected() {
        let request = request(
            ClaimClass::AerodynamicPrediction,
            DecisionConsequence::Engineering,
        );
        let mut evidence = all_verified(&request);
        let validation = evidence
            .iter_mut()
            .find(|item| item.kind == EvidenceKind::ExperimentalValidation)
            .expect("validation evidence");
        validation.scope_sha256 = digest('e');
        let decision = evaluate_claim_admission(&request, &evidence).expect("valid assertions");
        assert_eq!(
            decision.status,
            ClaimDecisionStatus::RejectedFailedOrMismatchedEvidence
        );
        assert!(decision.failed_evidence.contains(&EvidenceKind::ExperimentalValidation));
    }

    #[test]
    fn multivariate_validation_is_required_for_unsteady_aeroelastic_and_safety_critical_claims() {
        let unsteady = request(
            ClaimClass::UnsteadyAerodynamicPrediction,
            DecisionConsequence::Engineering,
        );
        assert!(required_for(&unsteady).contains(&EvidenceKind::MultivariateValidation));

        let aeroelastic = request(
            ClaimClass::AeroelasticStabilityPrediction,
            DecisionConsequence::Engineering,
        );
        assert!(required_for(&aeroelastic).contains(&EvidenceKind::MultivariateValidation));

        let safety = request(
            ClaimClass::AerodynamicPrediction,
            DecisionConsequence::SafetyCritical,
        );
        assert!(required_for(&safety).contains(&EvidenceKind::MultivariateValidation));
    }

    #[test]
    fn safety_critical_and_flight_release_claims_never_become_automatic_authorizations() {
        let request = request(
            ClaimClass::AerodynamicPrediction,
            DecisionConsequence::SafetyCritical,
        );
        let evidence = all_verified(&request);
        let decision = evaluate_claim_admission(&request, &evidence).expect("valid assertions");
        assert_eq!(
            decision.status,
            ClaimDecisionStatus::RequiresHumanAuthorityDecision
        );

        let flight = request(
            ClaimClass::FlightRelease,
            DecisionConsequence::SafetyCritical,
        );
        assert!(required_for(&flight).contains(&EvidenceKind::CertificationBasisEvidence));
        assert!(!required_for(&flight).contains(&EvidenceKind::CertificationAuthorityDecision));
        let mut flight_evidence = all_verified(&flight);
        // Even an archived authority-decision record cannot turn this tool into an
        // automatic release authority; the human/authority gate remains external.
        flight_evidence.push(verified(
            EvidenceKind::CertificationAuthorityDecision,
            &flight.scope_sha256,
        ));
        let flight_decision =
            evaluate_claim_admission(&flight, &flight_evidence).expect("valid assertions");
        assert_eq!(
            flight_decision.status,
            ClaimDecisionStatus::RequiresHumanAuthorityDecision
        );
    }

    #[test]
    fn decision_digest_binds_exact_artifact_and_verifier_receipts() {
        let request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        let evidence = all_verified(&request);
        let first = evaluate_claim_admission(&request, &evidence)
            .expect("first evidence manifest");
        let mut changed = evidence.clone();
        let artifact = changed
            .iter_mut()
            .find_map(|item| match &mut item.assertion {
                EvidenceAssertion::Verified { artifact_sha256, .. }
                    if item.kind == EvidenceKind::GeometryIdentity =>
                {
                    Some(artifact_sha256)
                }
                _ => None,
            })
            .expect("geometry artifact digest");
        *artifact = digest('e');
        let second = evaluate_claim_admission(&request, &changed)
            .expect("second evidence manifest");
        assert_eq!(first.status, second.status);
        assert_ne!(
            first.evidence_manifest_sha256,
            second.evidence_manifest_sha256
        );
        assert_ne!(first.decision_sha256, second.decision_sha256);
        verify_claim_admission(&request, &evidence, &first)
            .expect("first evidence manifest replays");
        assert!(
            verify_claim_admission(&request, &changed, &first)
                .expect_err("swapped evidence cannot replay prior decision")
                .contains("differs")
        );
    }

    #[test]
    fn evidence_manifest_is_order_independent_but_unknown_reason_sensitive() {
        let request = request(
            ClaimClass::AerodynamicPrediction,
            DecisionConsequence::Engineering,
        );
        let mut evidence = all_verified(&request);
        evidence.push(ScopedEvidence {
            kind: EvidenceKind::StatisticalSamplingAssessment,
            scope_sha256: request.scope_sha256.clone(),
            assertion: EvidenceAssertion::Unknown {
                reason: "not required for steady aerodynamic claim".to_owned(),
            },
        });
        let first = evaluate_claim_admission(&request, &evidence)
            .expect("supplemental evidence accepted and bound");
        let mut reordered = evidence.clone();
        reordered.reverse();
        let second = evaluate_claim_admission(&request, &reordered)
            .expect("reordered evidence");
        assert_eq!(first.evidence_manifest_sha256, second.evidence_manifest_sha256);
        let mut changed_reason = evidence;
        let supplemental = changed_reason
            .iter_mut()
            .find(|item| item.kind == EvidenceKind::StatisticalSamplingAssessment)
            .expect("supplemental assertion");
        supplemental.assertion = EvidenceAssertion::Unknown {
            reason: "sampling report unavailable".to_owned(),
        };
        let third = evaluate_claim_admission(&request, &changed_reason)
            .expect("changed supplemental reason");
        assert_ne!(first.evidence_manifest_sha256, third.evidence_manifest_sha256);
        assert_eq!(first.status, third.status);
    }

    #[test]
    fn claim_decision_csv_exports_policy_scope_and_evidence_manifest_digests() {
        let request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        let evidence = all_verified(&request);
        let decision = evaluate_claim_admission(&request, &evidence).expect("valid assertion set");
        let csv = report_to_csv(&decision);
        assert_eq!(csv.lines().count(), 2);
        assert!(csv.contains(&decision.policy_sha256));
        assert!(csv.contains(&decision.evidence_manifest_sha256));
        assert!(csv.contains(&decision.decision_sha256));
        assert!(csv.contains(ClaimDecisionStatus::AdmittedWithinDeclaredScope.as_str()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn independent_python_oracle_replays_claim_policy_manifest_and_decision() {
        let request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        let mut evidence = all_verified(&request);
        evidence.push(ScopedEvidence {
            kind: EvidenceKind::StatisticalSamplingAssessment,
            scope_sha256: request.scope_sha256.clone(),
            assertion: EvidenceAssertion::Unknown {
                reason: "supplemental sampling evidence not required by this exploratory claim".to_owned(),
            },
        });
        let decision = evaluate_claim_admission(&request, &evidence)
            .expect("claim decision with supplemental unknown evidence");

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aero-claim-admission-oracle-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create oracle fixture directory");
        let request_path = root.join("request.csv");
        let evidence_path = root.join("evidence.csv");
        let decision_path = root.join("decision.csv");
        fs::write(&request_path, request_to_csv(&request)).expect("write request");
        fs::write(
            &evidence_path,
            evidence_manifest_to_csv(&evidence).expect("serialize evidence manifest"),
        )
        .expect("write evidence manifest");
        fs::write(&decision_path, report_to_csv(&decision)).expect("write decision");

        let oracle_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/verify-aero-claim-admission-independent.py");
        let output = Command::new("python3")
            .arg(oracle_path)
            .arg(&request_path)
            .arg(&evidence_path)
            .arg(&decision_path)
            .output()
            .expect("run independent claim-admission oracle");
        assert!(
            output.status.success(),
            "independent oracle failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("independent_aero_claim_admission_oracle=pass")
        );
        fs::remove_dir_all(root).expect("remove oracle fixture directory");
    }

    #[test]
    fn uppercase_scope_or_artifact_digests_are_rejected_as_noncanonical() {
        let mut request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        request.scope_sha256 = request.scope_sha256.to_ascii_uppercase();
        assert!(
            evaluate_claim_admission(&request, &[])
                .expect_err("uppercase scope digest rejected")
                .contains("lowercase")
        );
    }

    #[test]
    fn duplicate_malformed_or_reasonless_evidence_is_rejected() {
        let request = request(
            ClaimClass::ExploratoryDesignRanking,
            DecisionConsequence::Exploratory,
        );
        let mut evidence = all_verified(&request);
        evidence.push(evidence[0].clone());
        assert!(
            evaluate_claim_admission(&request, &evidence)
                .expect_err("duplicate assertion rejected")
                .contains("duplicate evidence assertion")
        );

        let malformed = vec![ScopedEvidence {
            kind: EvidenceKind::GeometryIdentity,
            scope_sha256: request.scope_sha256.clone(),
            assertion: EvidenceAssertion::Unknown {
                reason: " ".to_owned(),
            },
        }];
        assert!(evaluate_claim_admission(&request, &malformed).is_err());
    }
}
