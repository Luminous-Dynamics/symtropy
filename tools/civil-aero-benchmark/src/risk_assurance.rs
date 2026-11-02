//! Risk-proportional evidence planning for engineering claims.
//!
//! This module turns explicit model influence, decision consequence, domain exposure and
//! analysis regime into a minimum evidence-gate profile. It does not estimate those risk inputs,
//! certify regulatory compliance, or calibrate probabilities. All required gates are hard
//! constraints: value-of-information and optimizer utility cannot override a missing or failed gate.

use std::collections::BTreeMap;

use crate::integrity::sha256_hex;

pub const RISK_ASSURANCE_SCHEMA: &str = "aero-risk-assurance/0.1.0";
pub const RISK_ASSURANCE_EVIDENCE_LEVEL: &str =
    "policy_profile_from_declared_risk_inputs_and_caller_reported_evidence_references_and_digests_not_independently_replayed_not_risk_estimate_or_regulatory_approval";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelInfluence {
    Low,
    Moderate,
    High,
}

impl ModelInfluence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionConsequence {
    Low,
    Moderate,
    High,
    SafetyCritical,
}

impl DecisionConsequence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::SafetyCritical => "safety_critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainExposure {
    InDomain,
    NearBoundary,
    OutOfDomain,
}

impl DomainExposure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InDomain => "in_domain",
            Self::NearBoundary => "near_boundary",
            Self::OutOfDomain => "out_of_domain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisRegime {
    Steady,
    UnsteadyDualTime,
    PeriodicStatistical,
    TransientEndpoint,
    CoupledTransient,
}

impl AnalysisRegime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steady => "steady",
            Self::UnsteadyDualTime => "unsteady_dual_time",
            Self::PeriodicStatistical => "periodic_statistical",
            Self::TransientEndpoint => "transient_endpoint",
            Self::CoupledTransient => "coupled_transient",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceTier {
    Screening,
    Standard,
    Enhanced,
    Maximum,
}

impl AssuranceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Screening => "screening",
            Self::Standard => "standard",
            Self::Enhanced => "enhanced",
            Self::Maximum => "maximum",
        }
    }

    fn raise(self, levels: u8) -> Self {
        let mut tier = self;
        for _ in 0..levels {
            tier = match tier {
                Self::Screening => Self::Standard,
                Self::Standard => Self::Enhanced,
                Self::Enhanced => Self::Maximum,
                Self::Maximum => Self::Maximum,
            };
        }
        tier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceGate {
    ProblemIdentity,
    InputArtifactProvenance,
    ProtocolConformance,
    ProcessReceiptReplay,
    NativeTermination,
    DeclaredConvergence,
    Conservation,
    SpatialGridConvergence,
    SensitivityAnalysis,
    InputUncertaintyQuantification,
    TemporalResolution,
    PerTimeStepInnerConvergence,
    StatisticalWindowConvergence,
    CoupledDisciplineConsistency,
    IndependentModelComparison,
    ExperimentalValidation,
    BlindDomainShiftChallenge,
    PhysicalConfirmation,
    IndependentReproduction,
    IndependentReview,
}

impl EvidenceGate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProblemIdentity => "problem_identity",
            Self::InputArtifactProvenance => "input_artifact_provenance",
            Self::ProtocolConformance => "protocol_conformance",
            Self::ProcessReceiptReplay => "process_receipt_replay",
            Self::NativeTermination => "native_termination",
            Self::DeclaredConvergence => "declared_convergence",
            Self::Conservation => "conservation",
            Self::SpatialGridConvergence => "spatial_grid_convergence",
            Self::SensitivityAnalysis => "sensitivity_analysis",
            Self::InputUncertaintyQuantification => "input_uncertainty_quantification",
            Self::TemporalResolution => "temporal_resolution",
            Self::PerTimeStepInnerConvergence => "per_time_step_inner_convergence",
            Self::StatisticalWindowConvergence => "statistical_window_convergence",
            Self::CoupledDisciplineConsistency => "coupled_discipline_consistency",
            Self::IndependentModelComparison => "independent_model_comparison",
            Self::ExperimentalValidation => "experimental_validation",
            Self::BlindDomainShiftChallenge => "blind_domain_shift_challenge",
            Self::PhysicalConfirmation => "physical_confirmation",
            Self::IndependentReproduction => "independent_reproduction",
            Self::IndependentReview => "independent_review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Passed,
    Failed,
    Unknown,
}

impl EvidenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskContext {
    pub claim_id: String,
    /// How strongly this model can change the engineering decision.
    pub model_influence: ModelInfluence,
    /// Consequence if the decision is wrong or the model is misleading.
    pub decision_consequence: DecisionConsequence,
    pub domain_exposure: DomainExposure,
    pub analysis_regime: AnalysisRegime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservation {
    pub gate: EvidenceGate,
    pub status: EvidenceStatus,
    /// Immutable locator for the underlying report, artifact, test or experiment.
    pub source_reference: String,
    /// SHA-256 of the exact evidence artifact. Required for a passing observation.
    pub source_sha256: String,
    /// Required when failed or unknown; records why the gate is not satisfied.
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceReport {
    pub schema: String,
    pub claim_id: String,
    pub model_influence: String,
    pub decision_consequence: String,
    pub domain_exposure: String,
    pub analysis_regime: String,
    pub assurance_tier: String,
    pub required_gates: Vec<String>,
    pub passed_gates: Vec<String>,
    pub supplemental_observations: Vec<String>,
    pub blockers: Vec<String>,
    pub admission_status: String,
    pub evidence_level: String,
    pub assessment_sha256: String,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// A monotone, categorical policy. Risk is not collapsed into a caller-selected weighted score.
pub fn assurance_tier(context: &RiskContext) -> AssuranceTier {
    if context.decision_consequence == DecisionConsequence::SafetyCritical {
        return AssuranceTier::Maximum;
    }

    let base = if context.model_influence == ModelInfluence::High
        || context.decision_consequence == DecisionConsequence::High
        || (context.model_influence == ModelInfluence::Moderate
            && context.decision_consequence == DecisionConsequence::Moderate)
    {
        AssuranceTier::Enhanced
    } else if context.model_influence == ModelInfluence::Moderate
        || context.decision_consequence == DecisionConsequence::Moderate
    {
        AssuranceTier::Standard
    } else {
        AssuranceTier::Screening
    };

    let domain_adjustment = match context.domain_exposure {
        DomainExposure::InDomain => 0,
        DomainExposure::NearBoundary => 1,
        DomainExposure::OutOfDomain => 2,
    };
    base.raise(domain_adjustment)
}

/// Return the minimum gates for a declared risk context. This is a research assurance policy,
/// not a substitute for applicable certification rules or a domain authority's requirements.
pub fn required_evidence_gates(context: &RiskContext) -> Vec<EvidenceGate> {
    let tier = assurance_tier(context);
    let mut gates = vec![
        EvidenceGate::ProblemIdentity,
        EvidenceGate::InputArtifactProvenance,
        EvidenceGate::ProtocolConformance,
        EvidenceGate::ProcessReceiptReplay,
        EvidenceGate::NativeTermination,
        EvidenceGate::DeclaredConvergence,
        EvidenceGate::Conservation,
    ];

    if tier >= AssuranceTier::Standard {
        gates.extend([
            EvidenceGate::SpatialGridConvergence,
            EvidenceGate::SensitivityAnalysis,
            EvidenceGate::InputUncertaintyQuantification,
        ]);
    }

    match context.analysis_regime {
        AnalysisRegime::Steady => {}
        AnalysisRegime::UnsteadyDualTime => gates.extend([
            EvidenceGate::TemporalResolution,
            EvidenceGate::PerTimeStepInnerConvergence,
            EvidenceGate::StatisticalWindowConvergence,
        ]),
        AnalysisRegime::PeriodicStatistical => gates.extend([
            EvidenceGate::TemporalResolution,
            EvidenceGate::StatisticalWindowConvergence,
        ]),
        AnalysisRegime::TransientEndpoint => {
            gates.push(EvidenceGate::TemporalResolution);
        }
        AnalysisRegime::CoupledTransient => gates.extend([
            EvidenceGate::TemporalResolution,
            EvidenceGate::CoupledDisciplineConsistency,
        ]),
    }

    if tier >= AssuranceTier::Enhanced {
        gates.extend([
            EvidenceGate::IndependentModelComparison,
            EvidenceGate::ExperimentalValidation,
            EvidenceGate::BlindDomainShiftChallenge,
        ]);
    }

    if tier >= AssuranceTier::Maximum {
        gates.extend([
            EvidenceGate::PhysicalConfirmation,
            EvidenceGate::IndependentReproduction,
            EvidenceGate::IndependentReview,
        ]);
    }

    gates.sort();
    gates.dedup();
    gates
}

fn validate_observation(observation: &EvidenceObservation) -> Result<(), String> {
    match observation.status {
        EvidenceStatus::Passed => {
            if observation.source_reference.trim().is_empty()
                || !is_sha256(&observation.source_sha256)
                || !observation.rationale.trim().is_empty()
            {
                return Err(format!(
                    "passing evidence gate {} requires a source reference and SHA-256, and no failure rationale",
                    observation.gate.as_str()
                ));
            }
        }
        EvidenceStatus::Failed | EvidenceStatus::Unknown => {
            if observation.rationale.trim().is_empty() {
                return Err(format!(
                    "{} evidence gate {} requires a rationale",
                    observation.status.as_str(),
                    observation.gate.as_str()
                ));
            }
            if !observation.source_sha256.is_empty()
                && !is_sha256(&observation.source_sha256)
            {
                return Err(format!(
                    "evidence gate {} has a malformed source SHA-256",
                    observation.gate.as_str()
                ));
            }
        }
    }
    Ok(())
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&(value as u64).to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_bytes(
    context: &RiskContext,
    tier: AssuranceTier,
    required: &[EvidenceGate],
    observations: &[EvidenceObservation],
    report: &AssuranceReport,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        context.claim_id.as_str(),
        context.model_influence.as_str(),
        context.decision_consequence.as_str(),
        context.domain_exposure.as_str(),
        context.analysis_regime.as_str(),
        tier.as_str(),
        report.admission_status.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, required.len());
    for gate in required {
        append_string(&mut bytes, gate.as_str());
    }
    append_u64(&mut bytes, observations.len());
    for observation in observations {
        append_string(&mut bytes, observation.gate.as_str());
        append_string(&mut bytes, observation.status.as_str());
        append_string(&mut bytes, &observation.source_reference);
        append_string(&mut bytes, &observation.source_sha256);
        append_string(&mut bytes, &observation.rationale);
    }
    append_u64(&mut bytes, report.passed_gates.len());
    for gate in &report.passed_gates {
        append_string(&mut bytes, gate);
    }
    append_u64(&mut bytes, report.supplemental_observations.len());
    for observation in &report.supplemental_observations {
        append_string(&mut bytes, observation);
    }
    append_u64(&mut bytes, report.blockers.len());
    for blocker in &report.blockers {
        append_string(&mut bytes, blocker);
    }
    bytes
}

pub fn evaluate_claim_assurance(
    context: &RiskContext,
    observations: &[EvidenceObservation],
) -> Result<AssuranceReport, String> {
    if context.claim_id.trim().is_empty() {
        return Err("risk assurance claim_id must not be empty".to_owned());
    }

    let required = required_evidence_gates(context);
    let mut by_gate = BTreeMap::new();
    for observation in observations {
        validate_observation(observation)?;
        if by_gate.insert(observation.gate, observation).is_some() {
            return Err(format!(
                "duplicate evidence observation for gate {}",
                observation.gate.as_str()
            ));
        }
    }

    let mut passed_gates = Vec::new();
    let mut blockers = Vec::new();
    for gate in &required {
        match by_gate.get(gate) {
            Some(observation) if observation.status == EvidenceStatus::Passed => {
                passed_gates.push(gate.as_str().to_owned());
            }
            Some(observation) => blockers.push(format!(
                "{}:{}",
                observation.status.as_str(),
                gate.as_str()
            )),
            None => blockers.push(format!("missing:{}", gate.as_str())),
        }
    }

    let tier = assurance_tier(context);
    let mut canonical_observations = observations.to_vec();
    canonical_observations.sort_by_key(|observation| observation.gate);
    let supplemental_observations = canonical_observations
        .iter()
        .filter(|observation| !required.contains(&observation.gate))
        .map(|observation| {
            format!("{}:{}", observation.gate.as_str(), observation.status.as_str())
        })
        .collect::<Vec<_>>();
    let mut report = AssuranceReport {
        schema: RISK_ASSURANCE_SCHEMA.to_owned(),
        claim_id: context.claim_id.clone(),
        model_influence: context.model_influence.as_str().to_owned(),
        decision_consequence: context.decision_consequence.as_str().to_owned(),
        domain_exposure: context.domain_exposure.as_str().to_owned(),
        analysis_regime: context.analysis_regime.as_str().to_owned(),
        assurance_tier: tier.as_str().to_owned(),
        required_gates: required.iter().map(|gate| gate.as_str().to_owned()).collect(),
        passed_gates,
        admission_status: if blockers.is_empty() {
            "policy_profile_satisfied_by_declared_observations".to_owned()
        } else {
            "policy_profile_withheld_required_observations_missing_failed_or_unknown".to_owned()
        },
        supplemental_observations,
        blockers,
        evidence_level: RISK_ASSURANCE_EVIDENCE_LEVEL.to_owned(),
        assessment_sha256: String::new(),
    };
    report.assessment_sha256 = sha256_hex(&canonical_bytes(
        context,
        tier,
        &required,
        &canonical_observations,
        &report,
    ))?;
    Ok(report)
}

/// Recompute the risk-profile policy and assessment digest.
/// This does not reopen evidence artifacts. Callers must run the appropriate receipt, report,
/// calibration-certificate or physical-measurement verifier before marking an observation Passed.
pub fn verify_claim_assurance(
    context: &RiskContext,
    observations: &[EvidenceObservation],
    report: &AssuranceReport,
) -> Result<(), String> {
    if report.schema != RISK_ASSURANCE_SCHEMA
        || report.evidence_level != RISK_ASSURANCE_EVIDENCE_LEVEL
    {
        return Err("unsupported risk-assurance schema or evidence ceiling".to_owned());
    }
    let recomputed = evaluate_claim_assurance(context, observations)?;
    if recomputed != *report {
        return Err("replayed risk-assurance assessment differs from supplied report".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn report_to_csv(report: &AssuranceReport) -> String {
    let header = [
        "schema",
        "claim_id",
        "model_influence",
        "decision_consequence",
        "domain_exposure",
        "analysis_regime",
        "assurance_tier",
        "required_gates",
        "passed_gates",
        "supplemental_observations",
        "blockers",
        "admission_status",
        "evidence_level",
        "assessment_sha256",
    ];
    let required_gates = report.required_gates.join(";");
    let passed_gates = report.passed_gates.join(";");
    let supplemental_observations = report.supplemental_observations.join(";");
    let blockers = report.blockers.join(";");
    let values = [
        report.schema.as_str(),
        report.claim_id.as_str(),
        report.model_influence.as_str(),
        report.decision_consequence.as_str(),
        report.domain_exposure.as_str(),
        report.analysis_regime.as_str(),
        report.assurance_tier.as_str(),
        required_gates.as_str(),
        passed_gates.as_str(),
        supplemental_observations.as_str(),
        blockers.as_str(),
        report.admission_status.as_str(),
        report.evidence_level.as_str(),
        report.assessment_sha256.as_str(),
    ];
    format!(
        "{}\n{}\n",
        header.join(","),
        values
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(
        model_influence: ModelInfluence,
        decision_consequence: DecisionConsequence,
        domain_exposure: DomainExposure,
        analysis_regime: AnalysisRegime,
    ) -> RiskContext {
        RiskContext {
            claim_id: "wing-drag-at-crm-condition".to_owned(),
            model_influence,
            decision_consequence,
            domain_exposure,
            analysis_regime,
        }
    }

    fn passing_observations(context: &RiskContext) -> Vec<EvidenceObservation> {
        required_evidence_gates(context)
            .into_iter()
            .enumerate()
            .map(|(index, gate)| EvidenceObservation {
                gate,
                status: EvidenceStatus::Passed,
                source_reference: format!("synthetic-fixture:{index}"),
                source_sha256: format!("{:064x}", index + 1),
                rationale: String::new(),
            })
            .collect()
    }

    #[test]
    fn declared_risk_axes_monotonically_increase_or_preserve_assurance_tier() {
        let base = assurance_tier(&context(
            ModelInfluence::Low,
            DecisionConsequence::Low,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        ));
        assert_eq!(base, AssuranceTier::Screening);
        assert!(assurance_tier(&context(
            ModelInfluence::Moderate,
            DecisionConsequence::Low,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        )) >= base);
        assert!(assurance_tier(&context(
            ModelInfluence::Low,
            DecisionConsequence::Moderate,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        )) >= base);
        assert!(assurance_tier(&context(
            ModelInfluence::Low,
            DecisionConsequence::Low,
            DomainExposure::NearBoundary,
            AnalysisRegime::Steady,
        )) > base);
        assert_eq!(
            assurance_tier(&context(
                ModelInfluence::High,
                DecisionConsequence::SafetyCritical,
                DomainExposure::OutOfDomain,
                AnalysisRegime::CoupledTransient,
            )),
            AssuranceTier::Maximum
        );
    }

    #[test]
    fn unsteady_dual_time_policy_requires_inner_temporal_and_window_evidence() {
        let context = context(
            ModelInfluence::Moderate,
            DecisionConsequence::Moderate,
            DomainExposure::InDomain,
            AnalysisRegime::UnsteadyDualTime,
        );
        let gates = required_evidence_gates(&context);
        for gate in [
            EvidenceGate::TemporalResolution,
            EvidenceGate::PerTimeStepInnerConvergence,
            EvidenceGate::StatisticalWindowConvergence,
            EvidenceGate::SpatialGridConvergence,
            EvidenceGate::InputUncertaintyQuantification,
        ] {
            assert!(gates.contains(&gate), "missing required gate {}", gate.as_str());
        }
    }

    #[test]
    fn transient_endpoint_requires_temporal_resolution_not_statistical_window() {
        let context = context(
            ModelInfluence::Moderate,
            DecisionConsequence::Moderate,
            DomainExposure::InDomain,
            AnalysisRegime::TransientEndpoint,
        );
        let gates = required_evidence_gates(&context);
        assert!(gates.contains(&EvidenceGate::TemporalResolution));
        assert!(!gates.contains(&EvidenceGate::StatisticalWindowConvergence));
        assert!(!gates.contains(&EvidenceGate::PerTimeStepInnerConvergence));
    }

    #[test]
    fn all_source_bound_required_gates_admit_and_replay_deterministically() {
        let context = context(
            ModelInfluence::Low,
            DecisionConsequence::Low,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        );
        let observations = passing_observations(&context);
        let report = evaluate_claim_assurance(&context, &observations).expect("evaluate");
        assert_eq!(report.admission_status, "policy_profile_satisfied_by_declared_observations");
        verify_claim_assurance(&context, &observations, &report).expect("replay");
        assert_eq!(
            report.assessment_sha256,
            evaluate_claim_assurance(&context, &observations)
                .expect("repeat evaluation")
                .assessment_sha256
        );
    }

    #[test]
    fn missing_failed_and_unknown_gates_withhold_admission() {
        let context = context(
            ModelInfluence::Moderate,
            DecisionConsequence::Moderate,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        );
        let mut observations = passing_observations(&context);
        observations.pop();
        let missing = evaluate_claim_assurance(&context, &observations).expect("assessment");
        assert!(missing.admission_status.starts_with("policy_profile_withheld"));
        assert!(missing.blockers.iter().any(|blocker| blocker.starts_with("missing:")));

        let mut observations = passing_observations(&context);
        observations[0].status = EvidenceStatus::Unknown;
        observations[0].source_sha256.clear();
        observations[0].rationale = "no independent boundary-identity receipt".to_owned();
        let unknown = evaluate_claim_assurance(&context, &observations).expect("unknown assessment");
        assert!(unknown.admission_status.starts_with("policy_profile_withheld"));

        let mut observations = passing_observations(&context);
        observations[0].status = EvidenceStatus::Failed;
        observations[0].rationale = "protocol identity mismatch".to_owned();
        let failed = evaluate_claim_assurance(&context, &observations).expect("failed assessment");
        assert!(failed.admission_status.starts_with("policy_profile_withheld"));
    }

    #[test]
    fn passing_gate_without_source_digest_or_reference_is_rejected() {
        let context = context(
            ModelInfluence::Low,
            DecisionConsequence::Low,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        );
        let mut observations = passing_observations(&context);
        observations[0].source_sha256.clear();
        assert!(
            evaluate_claim_assurance(&context, &observations)
                .expect_err("passing evidence without digest must reject")
                .contains("requires a source reference and SHA-256")
        );
    }

    #[test]
    fn extra_evidence_is_preserved_but_does_not_lower_the_required_profile() {
        let context = context(
            ModelInfluence::Low,
            DecisionConsequence::Low,
            DomainExposure::InDomain,
            AnalysisRegime::Steady,
        );
        let mut observations = passing_observations(&context);
        observations.push(EvidenceObservation {
            gate: EvidenceGate::ExperimentalValidation,
            status: EvidenceStatus::Passed,
            source_reference: "optional-experiment".to_owned(),
            source_sha256: "a".repeat(64),
            rationale: String::new(),
        });
        let report = evaluate_claim_assurance(&context, &observations).expect("assessment");
        assert_eq!(report.admission_status, "policy_profile_satisfied_by_declared_observations");
        assert!(!report.required_gates.contains(&"experimental_validation".to_owned()));
        assert_eq!(
            report.supplemental_observations,
            vec!["experimental_validation:passed"]
        );
    }
}
