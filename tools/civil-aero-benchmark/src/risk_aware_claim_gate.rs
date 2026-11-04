//! Composes model-credibility requirements with claim-specific evidence admission.
//!
//! The gate binds a risk assessment, its required controls, control assertions and the base claim
//! decision to one exact scope. It does not authenticate verifier identities against a trust store,
//! rerun artifact verifiers, certify models, or authorize release. A Critical tier always remains
//! subject to an external human-authority decision.

use std::collections::BTreeMap;

use crate::claim_admission::{
    ClaimDecision, ClaimDecisionStatus, ClaimRequest, EvidenceAssertion, ScopedEvidence,
    evaluate_claim_admission,
};
use crate::integrity::sha256_hex;
use crate::risk_credibility::{
    ArtifactAccess, CredibilityTier, ModelCredibilityAssessment, ModelCredibilityContext,
    assess_model_credibility,
};

pub const RISK_AWARE_CLAIM_GATE_SCHEMA: &str = "aero-risk-aware-claim-gate/0.1.0";
pub const RISK_AWARE_CLAIM_GATE_EVIDENCE_LEVEL: &str = "claim_admission_and_risk_credibility_requirements_and_control_assertions_bound_to_exact_scope_structural_verifier_references_not_verifier_authentication_not_release_authorization";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredibilityControlAssertion {
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
pub struct ScopedCredibilityControl {
    pub control_id: String,
    pub scope_sha256: String,
    pub assertion: CredibilityControlAssertion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredibilityControlStatus {
    Verified,
    Unknown,
    Failed,
    Missing,
    ScopeMismatch,
}

impl CredibilityControlStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unknown => "unknown",
            Self::Failed => "failed",
            Self::Missing => "missing",
            Self::ScopeMismatch => "scope_mismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredibilityControlResult {
    pub control_id: String,
    pub scope_sha256: String,
    pub status: CredibilityControlStatus,
    pub artifact_sha256: String,
    pub verifier_id: String,
    pub verifier_sha256: String,
    pub verifier_receipt_sha256: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskAwareClaimStatus {
    AdmittedWithinDeclaredUse,
    AbstainInsufficientOrUnknownEvidence,
    RejectedFailedOrMismatchedEvidence,
    BlockedDecisiveArtifactsUnavailable,
    RequiresHumanAuthorityDecision,
}

impl RiskAwareClaimStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdmittedWithinDeclaredUse => "admitted_within_declared_use",
            Self::AbstainInsufficientOrUnknownEvidence => {
                "abstain_insufficient_or_unknown_evidence"
            }
            Self::RejectedFailedOrMismatchedEvidence => "rejected_failed_or_mismatched_evidence",
            Self::BlockedDecisiveArtifactsUnavailable => "blocked_decisive_artifacts_unavailable",
            Self::RequiresHumanAuthorityDecision => "requires_human_authority_decision",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskAwareClaimDecision {
    pub schema: String,
    pub claim_id: String,
    pub model_id: String,
    pub intended_use: String,
    pub scope_sha256: String,
    pub claim_decision: ClaimDecision,
    pub credibility_assessment: ModelCredibilityAssessment,
    pub credibility_assessment_sha256: String,
    pub control_manifest_sha256: String,
    pub controls: Vec<CredibilityControlResult>,
    pub status: RiskAwareClaimStatus,
    pub evidence_level: &'static str,
    pub decision_sha256: String,
}

pub struct RiskAwareClaimRequest<'a> {
    pub claim: &'a ClaimRequest,
    pub credibility_context: &'a ModelCredibilityContext,
    pub claim_evidence: &'a [ScopedEvidence],
    pub credibility_controls: &'a [ScopedCredibilityControl],
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_strings(bytes: &mut Vec<u8>, values: &[String]) {
    append_u64(bytes, values.len() as u64);
    for value in values {
        append_string(bytes, value);
    }
}

fn claim_and_credibility_consequences_match(
    claim: crate::claim_admission::DecisionConsequence,
    credibility: crate::risk_credibility::DecisionConsequence,
) -> bool {
    use crate::claim_admission::DecisionConsequence as C;
    use crate::risk_credibility::DecisionConsequence as R;

    matches!(
        (claim, credibility),
        (C::Exploratory, R::Limited)
            | (C::Engineering, R::Significant)
            | (C::High, R::Severe)
            | (C::SafetyCritical, R::Severe)
    )
}

fn credibility_assessment_bytes(
    context: &ModelCredibilityContext,
    assessment: &ModelCredibilityAssessment,
    scope_sha256: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        RISK_AWARE_CLAIM_GATE_SCHEMA,
        scope_sha256,
        context.model_id.as_str(),
        context.intended_use.as_str(),
        context.model_creation.as_str(),
        context.logic_transparency.as_str(),
        context.artifact_access.as_str(),
        context.model_influence.as_str(),
        context.decision_consequence.as_str(),
        context.influence_rationale.as_str(),
        context.consequence_rationale.as_str(),
        context.classification_reference.as_str(),
        assessment.tier.as_str(),
        assessment.status,
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, assessment.requirements.len() as u64);
    for requirement in &assessment.requirements {
        append_string(&mut bytes, requirement.control_id);
        append_string(&mut bytes, requirement.rationale);
    }
    append_u64(&mut bytes, assessment.restrictions.len() as u64);
    for restriction in &assessment.restrictions {
        append_string(&mut bytes, restriction);
    }
    bytes
}

fn control_manifest_bytes(
    controls: &BTreeMap<String, &ScopedCredibilityControl>,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-credibility-control-manifest/0.1.0");
    append_u64(&mut bytes, controls.len() as u64);
    for (control_id, control) in controls {
        append_string(&mut bytes, control_id);
        append_string(&mut bytes, &control.scope_sha256);
        match &control.assertion {
            CredibilityControlAssertion::Verified {
                artifact_sha256,
                verifier_id,
                verifier_sha256,
                verifier_receipt_sha256,
            } => {
                if !is_sha256(artifact_sha256)
                    || !is_sha256(verifier_sha256)
                    || !is_sha256(verifier_receipt_sha256)
                    || verifier_id.trim().is_empty()
                {
                    return Err(format!(
                        "verified control {control_id} has malformed verifier references"
                    ));
                }
                bytes.push(1);
                append_string(&mut bytes, artifact_sha256);
                append_string(&mut bytes, verifier_id);
                append_string(&mut bytes, verifier_sha256);
                append_string(&mut bytes, verifier_receipt_sha256);
            }
            CredibilityControlAssertion::Unknown { reason } => {
                if reason.trim().is_empty() {
                    return Err(format!("unknown control {control_id} needs a reason"));
                }
                bytes.push(2);
                append_string(&mut bytes, reason);
            }
            CredibilityControlAssertion::Failed { reason } => {
                if reason.trim().is_empty() {
                    return Err(format!("failed control {control_id} needs a reason"));
                }
                bytes.push(3);
                append_string(&mut bytes, reason);
            }
        }
    }
    Ok(bytes)
}

fn build_control_results(
    scope_sha256: &str,
    assessment: &ModelCredibilityAssessment,
    supplied: &[ScopedCredibilityControl],
) -> Result<Vec<CredibilityControlResult>, String> {
    let required = assessment
        .requirements
        .iter()
        .map(|requirement| (requirement.control_id, requirement))
        .collect::<BTreeMap<_, _>>();
    let mut by_id = BTreeMap::<String, &ScopedCredibilityControl>::new();
    for control in supplied {
        if control.control_id.trim().is_empty() {
            return Err("credibility control_id must not be empty".to_owned());
        }
        if !required.contains_key(control.control_id.as_str()) {
            return Err(format!(
                "credibility assertion references a control not required by this policy: {}",
                control.control_id
            ));
        }
        if by_id.insert(control.control_id.clone(), control).is_some() {
            return Err(format!(
                "duplicate credibility control assertion: {}",
                control.control_id
            ));
        }
    }

    let mut results = Vec::with_capacity(assessment.requirements.len());
    for requirement in &assessment.requirements {
        let control_id = requirement.control_id;
        let Some(control) = by_id.get(control_id) else {
            results.push(CredibilityControlResult {
                control_id: control_id.to_owned(),
                scope_sha256: scope_sha256.to_owned(),
                status: CredibilityControlStatus::Missing,
                artifact_sha256: String::new(),
                verifier_id: String::new(),
                verifier_sha256: String::new(),
                verifier_receipt_sha256: String::new(),
                reason: "required control assertion was not supplied".to_owned(),
            });
            continue;
        };
        if control.scope_sha256 != scope_sha256 {
            results.push(CredibilityControlResult {
                control_id: control_id.to_owned(),
                scope_sha256: control.scope_sha256.clone(),
                status: CredibilityControlStatus::ScopeMismatch,
                artifact_sha256: String::new(),
                verifier_id: String::new(),
                verifier_sha256: String::new(),
                verifier_receipt_sha256: String::new(),
                reason: "control evidence belongs to a different claim scope".to_owned(),
            });
            continue;
        }
        let result = match &control.assertion {
            CredibilityControlAssertion::Verified {
                artifact_sha256,
                verifier_id,
                verifier_sha256,
                verifier_receipt_sha256,
            } => CredibilityControlResult {
                control_id: control_id.to_owned(),
                scope_sha256: control.scope_sha256.clone(),
                status: CredibilityControlStatus::Verified,
                artifact_sha256: artifact_sha256.clone(),
                verifier_id: verifier_id.clone(),
                verifier_sha256: verifier_sha256.clone(),
                verifier_receipt_sha256: verifier_receipt_sha256.clone(),
                reason: String::new(),
            },
            CredibilityControlAssertion::Unknown { reason } => CredibilityControlResult {
                control_id: control_id.to_owned(),
                scope_sha256: control.scope_sha256.clone(),
                status: CredibilityControlStatus::Unknown,
                artifact_sha256: String::new(),
                verifier_id: String::new(),
                verifier_sha256: String::new(),
                verifier_receipt_sha256: String::new(),
                reason: reason.clone(),
            },
            CredibilityControlAssertion::Failed { reason } => CredibilityControlResult {
                control_id: control_id.to_owned(),
                scope_sha256: control.scope_sha256.clone(),
                status: CredibilityControlStatus::Failed,
                artifact_sha256: String::new(),
                verifier_id: String::new(),
                verifier_sha256: String::new(),
                verifier_receipt_sha256: String::new(),
                reason: reason.clone(),
            },
        };
        results.push(result);
    }
    Ok(results)
}

fn control_results_bytes(controls: &[CredibilityControlResult]) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-credibility-control-results/0.1.0");
    append_u64(&mut bytes, controls.len() as u64);
    for control in controls {
        for value in [
            control.control_id.as_str(),
            control.scope_sha256.as_str(),
            control.status.as_str(),
            control.artifact_sha256.as_str(),
            control.verifier_id.as_str(),
            control.verifier_sha256.as_str(),
            control.verifier_receipt_sha256.as_str(),
            control.reason.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
    }
    bytes
}

fn decision_bytes(report: &RiskAwareClaimDecision) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.claim_id.as_str(),
        report.model_id.as_str(),
        report.intended_use.as_str(),
        report.scope_sha256.as_str(),
        report.claim_decision.decision_sha256.as_str(),
        report.credibility_assessment_sha256.as_str(),
        report.control_manifest_sha256.as_str(),
        report.status.as_str(),
        report.evidence_level,
    ] {
        append_string(&mut bytes, value);
    }
    bytes.extend_from_slice(&control_results_bytes(&report.controls));
    let restrictions = report
        .credibility_assessment
        .restrictions
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    append_strings(&mut bytes, &restrictions);
    bytes
}

/// Evaluate ordinary claim evidence and model-credibility controls as one exact-scope decision.
/// The claim and credibility consequence categories must agree using an explicit mapping. A
/// Critical credibility tier, flight-release claim or safety-critical claim always requires
/// external human authority even when every supplied control is Verified.
pub fn evaluate_risk_aware_claim_admission(
    request: &RiskAwareClaimRequest<'_>,
) -> Result<RiskAwareClaimDecision, String> {
    if request.claim.scope_sha256.len() != 64 || !is_sha256(&request.claim.scope_sha256) {
        return Err("claim scope must be a lowercase SHA-256 digest".to_owned());
    }
    if !claim_and_credibility_consequences_match(
        request.claim.consequence,
        request.credibility_context.decision_consequence,
    ) {
        return Err(
            "claim consequence and model-credibility consequence classifications do not match"
                .to_owned(),
        );
    }

    let claim_decision = evaluate_claim_admission(request.claim, request.claim_evidence)?;
    let credibility_assessment = assess_model_credibility(request.credibility_context)?;
    let credibility_assessment_sha256 = sha256_hex(&credibility_assessment_bytes(
        request.credibility_context,
        &credibility_assessment,
        &request.claim.scope_sha256,
    ))?;

    let mut supplied = BTreeMap::<String, &ScopedCredibilityControl>::new();
    for control in request.credibility_controls {
        if supplied
            .insert(control.control_id.clone(), control)
            .is_some()
        {
            return Err(format!(
                "duplicate credibility control assertion: {}",
                control.control_id
            ));
        }
    }
    let control_manifest_sha256 = sha256_hex(&control_manifest_bytes(&supplied)?)?;
    let controls = build_control_results(
        &request.claim.scope_sha256,
        &credibility_assessment,
        request.credibility_controls,
    )?;

    let has_failed = controls.iter().any(|control| {
        matches!(
            control.status,
            CredibilityControlStatus::Failed | CredibilityControlStatus::ScopeMismatch
        )
    });
    let has_missing_or_unknown = controls.iter().any(|control| {
        matches!(
            control.status,
            CredibilityControlStatus::Missing | CredibilityControlStatus::Unknown
        )
    });

    let status = if claim_decision.status == ClaimDecisionStatus::RejectedFailedOrMismatchedEvidence
        || has_failed
    {
        RiskAwareClaimStatus::RejectedFailedOrMismatchedEvidence
    } else if credibility_assessment.artifact_access == ArtifactAccess::Unavailable {
        RiskAwareClaimStatus::BlockedDecisiveArtifactsUnavailable
    } else if claim_decision.status == ClaimDecisionStatus::AbstainInsufficientOrUnknownEvidence
        || has_missing_or_unknown
    {
        RiskAwareClaimStatus::AbstainInsufficientOrUnknownEvidence
    } else if claim_decision.status == ClaimDecisionStatus::RequiresHumanAuthorityDecision
        || credibility_assessment.tier == CredibilityTier::Critical
    {
        RiskAwareClaimStatus::RequiresHumanAuthorityDecision
    } else {
        RiskAwareClaimStatus::AdmittedWithinDeclaredUse
    };

    let mut report = RiskAwareClaimDecision {
        schema: RISK_AWARE_CLAIM_GATE_SCHEMA.to_owned(),
        claim_id: request.claim.claim_id.clone(),
        model_id: credibility_assessment.model_id.clone(),
        intended_use: credibility_assessment.intended_use.clone(),
        scope_sha256: request.claim.scope_sha256.clone(),
        claim_decision,
        credibility_assessment,
        credibility_assessment_sha256,
        control_manifest_sha256,
        controls,
        status,
        evidence_level: RISK_AWARE_CLAIM_GATE_EVIDENCE_LEVEL,
        decision_sha256: String::new(),
    };
    report.decision_sha256 = sha256_hex(&decision_bytes(&report))?;
    Ok(report)
}

/// Recompute both policies, every control assertion and the combined digest from the source inputs.
pub fn verify_risk_aware_claim_admission(
    request: &RiskAwareClaimRequest<'_>,
    report: &RiskAwareClaimDecision,
) -> Result<(), String> {
    if report.schema != RISK_AWARE_CLAIM_GATE_SCHEMA
        || report.evidence_level != RISK_AWARE_CLAIM_GATE_EVIDENCE_LEVEL
    {
        return Err("unsupported risk-aware claim gate schema or evidence ceiling".to_owned());
    }
    let recomputed = evaluate_risk_aware_claim_admission(request)?;
    if recomputed != *report {
        return Err("replayed risk-aware claim decision differs from supplied report".to_owned());
    }
    if sha256_hex(&decision_bytes(report))? != report.decision_sha256 {
        return Err("risk-aware claim decision digest mismatch".to_owned());
    }
    Ok(())
}

pub fn report_to_csv(report: &RiskAwareClaimDecision) -> String {
    let header = "schema,claim_id,model_id,intended_use,scope_sha256,claim_decision_sha256,claim_decision_status,credibility_assessment_sha256,control_manifest_sha256,credibility_tier,restrictions,control_id,control_scope_sha256,control_status,artifact_sha256,verifier_id,verifier_sha256,verifier_receipt_sha256,reason,decision_status,decision_sha256,evidence_level";
    let tier = report.credibility_assessment.tier.as_str();
    let restrictions = report.credibility_assessment.restrictions.join(";");
    let mut output = format!("{header}\n");
    for control in &report.controls {
        let values = [
            report.schema.clone(),
            report.claim_id.clone(),
            report.model_id.clone(),
            report.intended_use.clone(),
            report.scope_sha256.clone(),
            report.claim_decision.decision_sha256.clone(),
            report.claim_decision.status.as_str().to_owned(),
            report.credibility_assessment_sha256.clone(),
            report.control_manifest_sha256.clone(),
            tier.to_owned(),
            restrictions.clone(),
            control.control_id.clone(),
            control.scope_sha256.clone(),
            control.status.as_str().to_owned(),
            control.artifact_sha256.clone(),
            control.verifier_id.clone(),
            control.verifier_sha256.clone(),
            control.verifier_receipt_sha256.clone(),
            control.reason.clone(),
            report.status.as_str().to_owned(),
            report.decision_sha256.clone(),
            report.evidence_level.to_owned(),
        ];
        output.push_str(
            &values
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_admission::{ClaimClass, DecisionConsequence, EvidenceKind};
    use crate::risk_credibility::{
        ArtifactAccess, LogicTransparency, ModelCreation, ModelInfluence,
    };

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn claim_request(scope: &str, consequence: DecisionConsequence) -> ClaimRequest {
        ClaimRequest {
            claim_id: "civil-design-screening".to_owned(),
            claim_class: ClaimClass::ExploratoryDesignRanking,
            consequence,
            scope_sha256: scope.to_owned(),
        }
    }

    fn credibility_context(
        influence: ModelInfluence,
        consequence: crate::risk_credibility::DecisionConsequence,
        access: ArtifactAccess,
    ) -> ModelCredibilityContext {
        ModelCredibilityContext {
            model_id: "symtropy-aero-model-v0.1".to_owned(),
            intended_use: "offline civil aircraft design exploration".to_owned(),
            model_creation: ModelCreation::Mechanistic,
            logic_transparency: LogicTransparency::Inspectable,
            artifact_access: access,
            model_influence: influence,
            decision_consequence: consequence,
            influence_rationale: "declared role in design selection".to_owned(),
            consequence_rationale: "declared downstream decision impact".to_owned(),
            classification_reference: "luminous-risk-policy-v0.1".to_owned(),
        }
    }

    fn all_claim_evidence(scope: &str) -> Vec<ScopedEvidence> {
        use EvidenceKind as E;
        [
            E::GeometryIdentity,
            E::AnalysisInputsIdentity,
            E::SolverExecutableIdentity,
            E::ProcessReceipt,
            E::ConvergenceScreen,
            E::ComparisonProtocol,
            E::GridConvergence,
            E::TemporalConvergence,
            E::TimeStepSensitivity,
            E::ConservationAudit,
            E::StatisticalSamplingAssessment,
            E::ModelUncertaintyAssessment,
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
            E::CertificationAuthorityDecision,
            E::QualitySystemEvidence,
        ]
        .into_iter()
        .map(|kind| ScopedEvidence {
            kind,
            scope_sha256: scope.to_owned(),
            assertion: EvidenceAssertion::Verified {
                artifact_sha256: digest('b'),
                verifier_id: "independent-verifier-v1".to_owned(),
                verifier_sha256: digest('c'),
                verifier_receipt_sha256: digest('d'),
            },
        })
        .collect()
    }

    fn all_controls(
        scope: &str,
        context: &ModelCredibilityContext,
    ) -> Vec<ScopedCredibilityControl> {
        let assessment = assess_model_credibility(context).expect("valid credibility context");
        assessment
            .requirements
            .iter()
            .map(|requirement| ScopedCredibilityControl {
                control_id: requirement.control_id.to_owned(),
                scope_sha256: scope.to_owned(),
                assertion: CredibilityControlAssertion::Verified {
                    artifact_sha256: digest('e'),
                    verifier_id: "independent-control-verifier-v1".to_owned(),
                    verifier_sha256: digest('f'),
                    verifier_receipt_sha256: digest('1'),
                },
            })
            .collect()
    }

    #[test]
    fn risk_controls_and_claim_evidence_are_bound_into_one_replayable_decision() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let controls = all_controls(&scope, &context);
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let report =
            evaluate_risk_aware_claim_admission(&request).expect("complete risk-aware claim");
        assert_eq!(
            report.status,
            RiskAwareClaimStatus::AdmittedWithinDeclaredUse
        );
        assert_eq!(
            report.credibility_assessment.tier,
            CredibilityTier::Screening
        );
        assert!(report.controls.iter().all(|control| control.control_id
            != "no_release_authorization_from_assessment"
            && control.control_id != "screening_only_label"));
        assert!(
            report
                .credibility_assessment
                .restrictions
                .iter()
                .any(|restriction| restriction.contains("never authorizes flight"))
        );
        let csv = report_to_csv(&report);
        assert!(csv.contains("requirements-only"));
        assert!(csv.contains("admitted_within_declared_use"));
        verify_risk_aware_claim_admission(&request, &report).expect("independent replay");
    }

    #[test]
    fn missing_or_unknown_risk_controls_abstain_and_failed_controls_reject() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);

        let mut missing = all_controls(&scope, &context);
        missing.retain(|control| control.control_id != "code_verification");
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &missing,
        };
        let abstained =
            evaluate_risk_aware_claim_admission(&request).expect("missing control abstains");
        assert_eq!(
            abstained.status,
            RiskAwareClaimStatus::AbstainInsufficientOrUnknownEvidence
        );

        let mut failed = all_controls(&scope, &context);
        let control = failed
            .iter_mut()
            .find(|control| control.control_id == "code_verification")
            .expect("code verification control");
        control.assertion = CredibilityControlAssertion::Failed {
            reason: "known-answer suite failed".to_owned(),
        };
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &failed,
        };
        let rejected =
            evaluate_risk_aware_claim_admission(&request).expect("failed control rejects");
        assert_eq!(
            rejected.status,
            RiskAwareClaimStatus::RejectedFailedOrMismatchedEvidence
        );
    }

    #[test]
    fn mismatched_scope_and_unavailable_artifacts_never_promote_a_claim() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let mut controls = all_controls(&scope, &context);
        let wrong_scope = controls
            .iter_mut()
            .find(|control| control.control_id == "code_verification")
            .expect("code verification");
        wrong_scope.scope_sha256 = digest('9');
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let rejected =
            evaluate_risk_aware_claim_admission(&request).expect("scope mismatch rejects");
        assert_eq!(
            rejected.status,
            RiskAwareClaimStatus::RejectedFailedOrMismatchedEvidence
        );

        let unavailable_context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::Unavailable,
        );
        let unavailable_controls = all_controls(&scope, &unavailable_context);
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &unavailable_context,
            claim_evidence: &claim_evidence,
            credibility_controls: &unavailable_controls,
        };
        let blocked =
            evaluate_risk_aware_claim_admission(&request).expect("unavailable artifacts block");
        assert_eq!(
            blocked.status,
            RiskAwareClaimStatus::BlockedDecisiveArtifactsUnavailable
        );
    }

    #[test]
    fn restricted_artifacts_require_approved_third_party_audit_and_keep_scope_restriction() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::Restricted,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let mut controls = all_controls(&scope, &context);
        let third_party_audit = controls
            .iter()
            .find(|control| control.control_id == "approved_third_party_artifact_audit")
            .expect("restricted artifacts require approved third-party audit");
        assert!(matches!(
            &third_party_audit.assertion,
            CredibilityControlAssertion::Verified { .. }
        ));
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let report = evaluate_risk_aware_claim_admission(&request)
            .expect("third-party audit allows declared-use admission");
        assert_eq!(
            report.status,
            RiskAwareClaimStatus::AdmittedWithinDeclaredUse
        );
        assert!(
            report
                .credibility_assessment
                .restrictions
                .iter()
                .any(|restriction| restriction.contains("access-restricted"))
        );
        assert!(report_to_csv(&report).contains("access-restricted"));

        controls.retain(|control| control.control_id != "approved_third_party_artifact_audit");
        let incomplete = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let abstained = evaluate_risk_aware_claim_admission(&incomplete)
            .expect("missing restricted artifact audit abstains");
        assert_eq!(
            abstained.status,
            RiskAwareClaimStatus::AbstainInsufficientOrUnknownEvidence
        );
    }

    #[test]
    fn malformed_verified_control_references_are_rejected() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let mut controls = all_controls(&scope, &context);
        let control = controls
            .iter_mut()
            .find(|control| control.control_id == "code_verification")
            .expect("code verification control");
        control.assertion = CredibilityControlAssertion::Verified {
            artifact_sha256: "not-a-digest".to_owned(),
            verifier_id: "independent-control-verifier-v1".to_owned(),
            verifier_sha256: digest('f'),
            verifier_receipt_sha256: digest('1'),
        };
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        assert!(
            evaluate_risk_aware_claim_admission(&request)
                .expect_err("malformed verified reference rejected")
                .contains("malformed verifier references")
        );
    }

    #[test]
    fn critical_influence_and_consequence_require_human_authority() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::SafetyCritical);
        let context = credibility_context(
            ModelInfluence::Controlling,
            crate::risk_credibility::DecisionConsequence::Severe,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let controls = all_controls(&scope, &context);
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let report =
            evaluate_risk_aware_claim_admission(&request).expect("complete critical evidence");
        assert_eq!(
            report.credibility_assessment.tier,
            CredibilityTier::Critical
        );
        assert_eq!(
            report.status,
            RiskAwareClaimStatus::RequiresHumanAuthorityDecision
        );
    }

    #[test]
    fn claim_and_credibility_consequence_mismatch_is_rejected() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Engineering);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let controls = all_controls(&scope, &context);
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        assert!(
            evaluate_risk_aware_claim_admission(&request)
                .expect_err("inconsistent consequence mapping")
                .contains("do not match")
        );
    }

    #[test]
    fn replay_rejects_mutated_credibility_control() {
        let scope = digest('a');
        let claim = claim_request(&scope, DecisionConsequence::Exploratory);
        let context = credibility_context(
            ModelInfluence::Advisory,
            crate::risk_credibility::DecisionConsequence::Limited,
            ArtifactAccess::IndependentReplayAvailable,
        );
        let claim_evidence = all_claim_evidence(&scope);
        let controls = all_controls(&scope, &context);
        let request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &controls,
        };
        let report = evaluate_risk_aware_claim_admission(&request).expect("complete claim");
        let mut changed = controls.clone();
        changed[0].scope_sha256 = digest('8');
        let changed_request = RiskAwareClaimRequest {
            claim: &claim,
            credibility_context: &context,
            claim_evidence: &claim_evidence,
            credibility_controls: &changed,
        };
        assert!(verify_risk_aware_claim_admission(&changed_request, &report).is_err());
    }
}
