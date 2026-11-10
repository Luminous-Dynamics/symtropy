// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Risk-proportionate, claim-specific model-evidence obligations.
//!
//! The evaluator does not calculate a probability of harm or a universal
//! model-credibility score. It applies a transparent ordinal policy to the
//! declared model influence and decision consequence, unions those
//! risk-tier obligations with claim-specific obligations, and checks exact
//! scoped evidence receipts. Digests and reviewer identities are provenance
//! references; this module does not cryptographically authenticate them.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

pub const ASSURANCE_POLICY_VERSION: &str = "civdyn-claim-assurance-v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrdinalLevel {
    Low,
    Moderate,
    High,
}

impl OrdinalLevel {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Tier1,
    Tier2,
    Tier3,
}

impl EvidenceTier {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tier1 => "tier_1",
            Self::Tier2 => "tier_2",
            Self::Tier3 => "tier_3",
        }
    }

    const fn from_levels(influence: OrdinalLevel, consequence: OrdinalLevel) -> Self {
        match influence.max(consequence) {
            OrdinalLevel::Low => Self::Tier1,
            OrdinalLevel::Moderate => Self::Tier2,
            OrdinalLevel::High => Self::Tier3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimClass {
    Exploratory,
    Forecast,
    CausalEffect,
    PolicyRecommendation,
}

impl ClaimClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exploratory => "exploratory",
            Self::Forecast => "forecast",
            Self::CausalEffect => "causal_effect",
            Self::PolicyRecommendation => "policy_recommendation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceType {
    ImplementationVerification,
    ModelConfigurationIdentity,
    DataRightsAndProvenance,
    ContextOfUseDeclaration,
    UncertaintyRegister,
    IndependentReplay,
    SplitPredictiveValidation,
    EmpiricalCalibration,
    StructuralDiscrepancyAssessment,
    SensitivityAnalysis,
    ApplicabilityDomain,
    IndependentReview,
    DriftMonitoringPlan,
    CausalEstimand,
    SourceInternalValidity,
    IdentificationAssumptions,
    PositivityAndOverlap,
    InterferenceAssessment,
    HeldOutChallenge,
    PartialIdentification,
    CompetingModelAnalysis,
    PolicyFeasibility,
    ObjectiveSpecificRegret,
    DistributionalImpact,
    IntergenerationalAccounting,
    PhysicalConservation,
    DecisionAuthority,
    Contestability,
    RollbackPlan,
    ProspectiveMonitoring,
}

impl EvidenceType {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ImplementationVerification => "implementation_verification",
            Self::ModelConfigurationIdentity => "model_configuration_identity",
            Self::DataRightsAndProvenance => "data_rights_and_provenance",
            Self::ContextOfUseDeclaration => "context_of_use_declaration",
            Self::UncertaintyRegister => "uncertainty_register",
            Self::IndependentReplay => "independent_replay",
            Self::SplitPredictiveValidation => "split_predictive_validation",
            Self::EmpiricalCalibration => "empirical_calibration",
            Self::StructuralDiscrepancyAssessment => "structural_discrepancy_assessment",
            Self::SensitivityAnalysis => "sensitivity_analysis",
            Self::ApplicabilityDomain => "applicability_domain",
            Self::IndependentReview => "independent_review",
            Self::DriftMonitoringPlan => "drift_monitoring_plan",
            Self::CausalEstimand => "causal_estimand",
            Self::SourceInternalValidity => "source_internal_validity",
            Self::IdentificationAssumptions => "identification_assumptions",
            Self::PositivityAndOverlap => "positivity_and_overlap",
            Self::InterferenceAssessment => "interference_assessment",
            Self::HeldOutChallenge => "held_out_challenge",
            Self::PartialIdentification => "partial_identification",
            Self::CompetingModelAnalysis => "competing_model_analysis",
            Self::PolicyFeasibility => "policy_feasibility",
            Self::ObjectiveSpecificRegret => "objective_specific_regret",
            Self::DistributionalImpact => "distributional_impact",
            Self::IntergenerationalAccounting => "intergenerational_accounting",
            Self::PhysicalConservation => "physical_conservation",
            Self::DecisionAuthority => "decision_authority",
            Self::Contestability => "contestability",
            Self::RollbackPlan => "rollback_plan",
            Self::ProspectiveMonitoring => "prospective_monitoring",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Pass,
    Fail,
    NotAssessed,
}

impl EvidenceStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceStatus {
    QualifiedForDeclaredUse,
    NotAssessed,
    RejectedForDeclaredUse,
}

impl AssuranceStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::QualifiedForDeclaredUse => "qualified_for_declared_use",
            Self::NotAssessed => "not_assessed",
            Self::RejectedForDeclaredUse => "rejected_for_declared_use",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationStatus {
    Satisfied,
    Missing,
    NotAssessed,
    Failed,
    StaleOrOutOfScope,
    InsufficientTier,
    IndependenceNotEstablished,
}

impl ObligationStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Missing => "missing",
            Self::NotAssessed => "not_assessed",
            Self::Failed => "failed",
            Self::StaleOrOutOfScope => "stale_or_out_of_scope",
            Self::InsufficientTier => "insufficient_tier",
            Self::IndependenceNotEstablished => "independence_not_established",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceContext {
    pub context_id: String,
    pub context_digest: String,
    pub decision_id: String,
    pub model_digest: String,
    pub configuration_digest: String,
    pub population_id: String,
    pub time_horizon: String,
    /// Expected empirical source identity for obligations that consume data.
    pub source_digest: Option<String>,
    /// Expected calibration source identity for predictive evidence.
    pub calibration_digest: Option<String>,
    /// Expected split identity for held-out evidence.
    pub split_manifest_digest: Option<String>,
    pub model_influence: OrdinalLevel,
    pub decision_consequence: OrdinalLevel,
    pub claim: ClaimClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub receipt_id: String,
    pub evidence_type: EvidenceType,
    pub status: EvidenceStatus,
    pub evidence_tier: EvidenceTier,
    pub model_digest: String,
    pub configuration_digest: String,
    pub context_digest: String,
    pub population_id: String,
    pub time_horizon: String,
    pub source_digest: Option<String>,
    pub calibration_digest: Option<String>,
    pub split_manifest_digest: Option<String>,
    pub applicable_claims: Vec<ClaimClass>,
    pub assessor_id: String,
    pub independent_review: bool,
    /// Content identity provided by the receipt producer, not verified here.
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceObligation {
    pub evidence_type: EvidenceType,
    pub minimum_tier: EvidenceTier,
    pub requires_independent_review: bool,
    pub requires_source_digest: bool,
    pub requires_calibration_digest: bool,
    pub requires_split_manifest_digest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationResult {
    pub evidence_type: EvidenceType,
    pub minimum_tier: EvidenceTier,
    pub status: ObligationStatus,
    pub matched_receipt_id: Option<String>,
    pub explanation_code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAssuranceReport {
    pub policy_version: String,
    pub context_id: String,
    pub context_digest: String,
    pub decision_id: String,
    pub model_digest: String,
    pub configuration_digest: String,
    pub claim: ClaimClass,
    pub model_influence: OrdinalLevel,
    pub decision_consequence: OrdinalLevel,
    pub evidence_tier: EvidenceTier,
    pub status: AssuranceStatus,
    pub qualified_obligation_count: usize,
    pub required_obligation_count: usize,
    pub obligations: Vec<ObligationResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimAssuranceError {
    EmptyContextIdentity,
    MissingRequiredContextDigest { field: &'static str },
    EmptyReceiptIdentity,
    DuplicateReceiptId { receipt_id: String },
    DuplicateObligation { evidence_type: EvidenceType },
}

impl fmt::Display for ClaimAssuranceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClaimAssuranceError {}

const fn obligation(
    evidence_type: EvidenceType,
    minimum_tier: EvidenceTier,
) -> EvidenceObligation {
    EvidenceObligation {
        evidence_type,
        minimum_tier,
        requires_independent_review: false,
        requires_source_digest: false,
        requires_calibration_digest: false,
        requires_split_manifest_digest: false,
    }
}

const fn reviewed_obligation(evidence_type: EvidenceType) -> EvidenceObligation {
    EvidenceObligation {
        evidence_type,
        minimum_tier: EvidenceTier::Tier2,
        requires_independent_review: true,
        requires_source_digest: false,
        requires_calibration_digest: false,
        requires_split_manifest_digest: false,
    }
}

const fn sourced_obligation(evidence_type: EvidenceType) -> EvidenceObligation {
    EvidenceObligation {
        evidence_type,
        minimum_tier: EvidenceTier::Tier1,
        requires_independent_review: false,
        requires_source_digest: true,
        requires_calibration_digest: false,
        requires_split_manifest_digest: false,
    }
}

const fn predictive_obligation(evidence_type: EvidenceType) -> EvidenceObligation {
    EvidenceObligation {
        evidence_type,
        minimum_tier: EvidenceTier::Tier2,
        requires_independent_review: false,
        requires_source_digest: true,
        requires_calibration_digest: true,
        requires_split_manifest_digest: true,
    }
}

const fn tier_three_obligation(evidence_type: EvidenceType) -> EvidenceObligation {
    EvidenceObligation {
        evidence_type,
        minimum_tier: EvidenceTier::Tier3,
        requires_independent_review: false,
        requires_source_digest: false,
        requires_calibration_digest: false,
        requires_split_manifest_digest: false,
    }
}

pub fn required_obligations(
    claim: ClaimClass,
    tier: EvidenceTier,
) -> Result<Vec<EvidenceObligation>, ClaimAssuranceError> {
    let mut requirements = Vec::new();

    // Tier 1: every declared use requires identity, provenance, an explicit
    // context, an uncertainty register and reproducible implementation evidence.
    requirements.extend([
        obligation(EvidenceType::ImplementationVerification, EvidenceTier::Tier1),
        obligation(EvidenceType::ModelConfigurationIdentity, EvidenceTier::Tier1),
        sourced_obligation(EvidenceType::DataRightsAndProvenance),
        obligation(EvidenceType::ContextOfUseDeclaration, EvidenceTier::Tier1),
        obligation(EvidenceType::UncertaintyRegister, EvidenceTier::Tier1),
        obligation(EvidenceType::IndependentReplay, EvidenceTier::Tier1),
    ]);

    if tier >= EvidenceTier::Tier2 {
        requirements.extend([
            predictive_obligation(EvidenceType::SplitPredictiveValidation),
            predictive_obligation(EvidenceType::EmpiricalCalibration),
            obligation(EvidenceType::StructuralDiscrepancyAssessment, EvidenceTier::Tier2),
            obligation(EvidenceType::SensitivityAnalysis, EvidenceTier::Tier2),
            obligation(EvidenceType::ApplicabilityDomain, EvidenceTier::Tier2),
            reviewed_obligation(EvidenceType::IndependentReview),
            obligation(EvidenceType::DriftMonitoringPlan, EvidenceTier::Tier2),
        ]);
    }

    if tier >= EvidenceTier::Tier3 {
        requirements.extend([
            tier_three_obligation(EvidenceType::HeldOutChallenge),
            tier_three_obligation(EvidenceType::PartialIdentification),
            tier_three_obligation(EvidenceType::CompetingModelAnalysis),
            tier_three_obligation(EvidenceType::PolicyFeasibility),
            tier_three_obligation(EvidenceType::ObjectiveSpecificRegret),
            tier_three_obligation(EvidenceType::DistributionalImpact),
            tier_three_obligation(EvidenceType::IntergenerationalAccounting),
            tier_three_obligation(EvidenceType::PhysicalConservation),
            tier_three_obligation(EvidenceType::DecisionAuthority),
            tier_three_obligation(EvidenceType::Contestability),
            tier_three_obligation(EvidenceType::RollbackPlan),
            tier_three_obligation(EvidenceType::ProspectiveMonitoring),
        ]);
    }

    match claim {
        ClaimClass::Exploratory => {}
        ClaimClass::Forecast => {
            requirements.extend([
                predictive_obligation(EvidenceType::SplitPredictiveValidation),
                predictive_obligation(EvidenceType::EmpiricalCalibration),
                obligation(EvidenceType::ApplicabilityDomain, EvidenceTier::Tier2),
                obligation(EvidenceType::UncertaintyRegister, EvidenceTier::Tier1),
            ]);
        }
        ClaimClass::CausalEffect => {
            requirements.extend([
                predictive_obligation(EvidenceType::SplitPredictiveValidation),
                predictive_obligation(EvidenceType::EmpiricalCalibration),
                obligation(EvidenceType::ApplicabilityDomain, EvidenceTier::Tier2),
                obligation(EvidenceType::CausalEstimand, EvidenceTier::Tier2),
                obligation(EvidenceType::SourceInternalValidity, EvidenceTier::Tier2),
                obligation(EvidenceType::IdentificationAssumptions, EvidenceTier::Tier2),
                sourced_obligation(EvidenceType::PositivityAndOverlap),
                obligation(EvidenceType::InterferenceAssessment, EvidenceTier::Tier2),
            ]);
        }
        ClaimClass::PolicyRecommendation => {
            requirements.extend([
                predictive_obligation(EvidenceType::SplitPredictiveValidation),
                predictive_obligation(EvidenceType::EmpiricalCalibration),
                obligation(EvidenceType::ApplicabilityDomain, EvidenceTier::Tier2),
                obligation(EvidenceType::CausalEstimand, EvidenceTier::Tier2),
                obligation(EvidenceType::SourceInternalValidity, EvidenceTier::Tier2),
                obligation(EvidenceType::IdentificationAssumptions, EvidenceTier::Tier2),
                sourced_obligation(EvidenceType::PositivityAndOverlap),
                obligation(EvidenceType::InterferenceAssessment, EvidenceTier::Tier2),
                tier_three_obligation(EvidenceType::CompetingModelAnalysis),
                tier_three_obligation(EvidenceType::PolicyFeasibility),
                tier_three_obligation(EvidenceType::ObjectiveSpecificRegret),
                tier_three_obligation(EvidenceType::DistributionalImpact),
                tier_three_obligation(EvidenceType::IntergenerationalAccounting),
                tier_three_obligation(EvidenceType::PhysicalConservation),
                tier_three_obligation(EvidenceType::DecisionAuthority),
                tier_three_obligation(EvidenceType::Contestability),
                tier_three_obligation(EvidenceType::RollbackPlan),
                tier_three_obligation(EvidenceType::ProspectiveMonitoring),
            ]);
        }
    }

    let mut seen = BTreeSet::new();
    for requirement in &requirements {
        if !seen.insert(requirement.evidence_type) {
            return Err(ClaimAssuranceError::DuplicateObligation {
                evidence_type: requirement.evidence_type,
            });
        }
    }
    Ok(requirements)
}

fn validate_context(context: &AssuranceContext) -> Result<(), ClaimAssuranceError> {
    for (value, field) in [
        (context.context_id.as_str(), "context_id"),
        (context.context_digest.as_str(), "context_digest"),
        (context.decision_id.as_str(), "decision_id"),
        (context.model_digest.as_str(), "model_digest"),
        (context.configuration_digest.as_str(), "configuration_digest"),
        (context.population_id.as_str(), "population_id"),
        (context.time_horizon.as_str(), "time_horizon"),
    ] {
        if value.trim().is_empty() {
            return Err(ClaimAssuranceError::MissingRequiredContextDigest { field });
        }
    }
    Ok(())
}

fn receipt_scope_matches(
    receipt: &EvidenceReceipt,
    context: &AssuranceContext,
) -> bool {
    receipt.model_digest == context.model_digest
        && receipt.configuration_digest == context.configuration_digest
        && receipt.context_digest == context.context_digest
        && receipt.population_id == context.population_id
        && receipt.time_horizon == context.time_horizon
        && receipt.applicable_claims.contains(&context.claim)
}

fn digest_scope_matches(
    receipt: &EvidenceReceipt,
    requirement: EvidenceObligation,
    context: &AssuranceContext,
) -> bool {
    if requirement.requires_source_digest
        && (context.source_digest.as_deref().is_none()
            || receipt.source_digest.as_deref() != context.source_digest.as_deref())
    {
        return false;
    }
    if requirement.requires_calibration_digest
        && (context.calibration_digest.as_deref().is_none()
            || receipt.calibration_digest.as_deref() != context.calibration_digest.as_deref())
    {
        return false;
    }
    if requirement.requires_split_manifest_digest
        && (context.split_manifest_digest.as_deref().is_none()
            || receipt.split_manifest_digest.as_deref()
                != context.split_manifest_digest.as_deref())
    {
        return false;
    }
    true
}

fn evaluate_obligation(
    requirement: EvidenceObligation,
    context: &AssuranceContext,
    receipts: &[EvidenceReceipt],
) -> ObligationResult {
    let typed: Vec<_> = receipts
        .iter()
        .filter(|receipt| receipt.evidence_type == requirement.evidence_type)
        .collect();

    let scoped: Vec<_> = typed
        .iter()
        .copied()
        .filter(|receipt| receipt_scope_matches(receipt, context))
        .collect();

    let relevant: Vec<_> = scoped
        .iter()
        .copied()
        .filter(|receipt| digest_scope_matches(receipt, requirement, context))
        .collect();

    if relevant.iter().any(|receipt| receipt.status == EvidenceStatus::Fail) {
        let failed = relevant
            .iter()
            .find(|receipt| receipt.status == EvidenceStatus::Fail)
            .expect("a failed receipt exists");
        return ObligationResult {
            evidence_type: requirement.evidence_type,
            minimum_tier: requirement.minimum_tier,
            status: ObligationStatus::Failed,
            matched_receipt_id: Some(failed.receipt_id.clone()),
            explanation_code: "matching_evidence_failed",
        };
    }

    if let Some(receipt) = relevant
        .iter()
        .find(|receipt| receipt.status == EvidenceStatus::Pass)
    {
        if receipt.evidence_tier < requirement.minimum_tier {
            return ObligationResult {
                evidence_type: requirement.evidence_type,
                minimum_tier: requirement.minimum_tier,
                status: ObligationStatus::InsufficientTier,
                matched_receipt_id: Some(receipt.receipt_id.clone()),
                explanation_code: "evidence_tier_below_required",
            };
        }
        if requirement.requires_independent_review && !receipt.independent_review {
            return ObligationResult {
                evidence_type: requirement.evidence_type,
                minimum_tier: requirement.minimum_tier,
                status: ObligationStatus::IndependenceNotEstablished,
                matched_receipt_id: Some(receipt.receipt_id.clone()),
                explanation_code: "independent_review_not_established",
            };
        }
        if receipt.receipt_digest.trim().is_empty() || receipt.assessor_id.trim().is_empty() {
            return ObligationResult {
                evidence_type: requirement.evidence_type,
                minimum_tier: requirement.minimum_tier,
                status: ObligationStatus::NotAssessed,
                matched_receipt_id: Some(receipt.receipt_id.clone()),
                explanation_code: "receipt_provenance_incomplete",
            };
        }
        return ObligationResult {
            evidence_type: requirement.evidence_type,
            minimum_tier: requirement.minimum_tier,
            status: ObligationStatus::Satisfied,
            matched_receipt_id: Some(receipt.receipt_id.clone()),
            explanation_code: "matching_passing_receipt",
        };
    }

    let has_not_assessed = relevant
        .iter()
        .any(|receipt| receipt.status == EvidenceStatus::NotAssessed);
    let status = if typed.is_empty() {
        ObligationStatus::Missing
    } else if scoped.is_empty() || relevant.is_empty() {
        ObligationStatus::StaleOrOutOfScope
    } else if has_not_assessed {
        ObligationStatus::NotAssessed
    } else {
        ObligationStatus::Missing
    };

    ObligationResult {
        evidence_type: requirement.evidence_type,
        minimum_tier: requirement.minimum_tier,
        status,
        matched_receipt_id: None,
        explanation_code: match status {
            ObligationStatus::Missing => "no_qualifying_receipt",
            ObligationStatus::NotAssessed => "receipt_not_assessed",
            ObligationStatus::StaleOrOutOfScope => "receipt_scope_or_identity_mismatch",
            _ => "obligation_not_satisfied",
        },
    }
}

pub fn assess_claim_assurance(
    context: &AssuranceContext,
    receipts: &[EvidenceReceipt],
) -> Result<ClaimAssuranceReport, ClaimAssuranceError> {
    validate_context(context)?;
    let mut receipt_ids = BTreeSet::new();
    for receipt in receipts {
        if receipt.receipt_id.trim().is_empty()
            || receipt.model_digest.trim().is_empty()
            || receipt.configuration_digest.trim().is_empty()
            || receipt.context_digest.trim().is_empty()
        {
            return Err(ClaimAssuranceError::EmptyReceiptIdentity);
        }
        if !receipt_ids.insert(receipt.receipt_id.as_str()) {
            return Err(ClaimAssuranceError::DuplicateReceiptId {
                receipt_id: receipt.receipt_id.clone(),
            });
        }
    }

    let tier = EvidenceTier::from_levels(
        context.model_influence,
        context.decision_consequence,
    );
    let required = required_obligations(context.claim, tier)?;
    let obligations: Vec<_> = required
        .iter()
        .copied()
        .map(|requirement| evaluate_obligation(requirement, context, receipts))
        .collect();

    let has_failure = obligations
        .iter()
        .any(|result| result.status == ObligationStatus::Failed);
    let qualified_count = obligations
        .iter()
        .filter(|result| result.status == ObligationStatus::Satisfied)
        .count();
    let status = if has_failure {
        AssuranceStatus::RejectedForDeclaredUse
    } else if qualified_count == obligations.len() {
        AssuranceStatus::QualifiedForDeclaredUse
    } else {
        AssuranceStatus::NotAssessed
    };

    Ok(ClaimAssuranceReport {
        policy_version: ASSURANCE_POLICY_VERSION.to_string(),
        context_id: context.context_id.clone(),
        context_digest: context.context_digest.clone(),
        decision_id: context.decision_id.clone(),
        model_digest: context.model_digest.clone(),
        configuration_digest: context.configuration_digest.clone(),
        claim: context.claim,
        model_influence: context.model_influence,
        decision_consequence: context.decision_consequence,
        evidence_tier: tier,
        status,
        qualified_obligation_count: qualified_count,
        required_obligation_count: obligations.len(),
        obligations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(
        influence: OrdinalLevel,
        consequence: OrdinalLevel,
        claim: ClaimClass,
    ) -> AssuranceContext {
        AssuranceContext {
            context_id: "regional-water-policy-v1".to_string(),
            context_digest: "sha256-water-policy-context".to_string(),
            decision_id: "district-water-allocation".to_string(),
            model_digest: "sha256-civdyn-model".to_string(),
            configuration_digest: "sha256-civdyn-configuration".to_string(),
            population_id: "district-households".to_string(),
            time_horizon: "2026-2030".to_string(),
            source_digest: Some("sha256-source-data".to_string()),
            calibration_digest: Some("sha256-calibration-data".to_string()),
            split_manifest_digest: Some("sha256-challenge-split".to_string()),
            model_influence: influence,
            decision_consequence: consequence,
            claim,
        }
    }

    fn receipt(
        evidence_type: EvidenceType,
        tier: EvidenceTier,
        claim: ClaimClass,
    ) -> EvidenceReceipt {
        EvidenceReceipt {
            receipt_id: format!("receipt-{}", evidence_type.name()),
            evidence_type,
            status: EvidenceStatus::Pass,
            evidence_tier: tier,
            model_digest: "sha256-civdyn-model".to_string(),
            configuration_digest: "sha256-civdyn-configuration".to_string(),
            context_digest: "sha256-water-policy-context".to_string(),
            population_id: "district-households".to_string(),
            time_horizon: "2026-2030".to_string(),
            source_digest: Some("sha256-source-data".to_string()),
            calibration_digest: Some("sha256-calibration-data".to_string()),
            split_manifest_digest: Some("sha256-challenge-split".to_string()),
            applicable_claims: vec![claim],
            assessor_id: "reviewer-a".to_string(),
            independent_review: evidence_type == EvidenceType::IndependentReview,
            receipt_digest: format!("sha256-receipt-{}", evidence_type.name()),
        }
    }

    fn receipts_for(
        context: &AssuranceContext,
    ) -> Vec<EvidenceReceipt> {
        required_obligations(
            context.claim,
            EvidenceTier::from_levels(
                context.model_influence,
                context.decision_consequence,
            ),
        )
        .expect("requirements")
        .iter()
        .map(|requirement| {
            let mut item = receipt(
                requirement.evidence_type,
                requirement.minimum_tier,
                context.claim,
            );
            item.independent_review = requirement.requires_independent_review;
            item
        })
        .collect()
    }

    #[test]
    fn frozen_influence_consequence_matrix_is_strictest_axis() {
        let levels = [OrdinalLevel::Low, OrdinalLevel::Moderate, OrdinalLevel::High];
        let expected = [
            [EvidenceTier::Tier1, EvidenceTier::Tier2, EvidenceTier::Tier3],
            [EvidenceTier::Tier2, EvidenceTier::Tier2, EvidenceTier::Tier3],
            [EvidenceTier::Tier3, EvidenceTier::Tier3, EvidenceTier::Tier3],
        ];
        for (row, influence) in levels.into_iter().enumerate() {
            for (column, consequence) in levels.into_iter().enumerate() {
                let report = assess_claim_assurance(
                    &context(influence, consequence, ClaimClass::Exploratory),
                    &[],
                )
                .expect("valid context");
                assert_eq!(report.evidence_tier, expected[row][column]);
            }
        }
    }

    #[test]
    fn low_risk_exploration_can_qualify_without_forecast_evidence() {
        let candidate = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::Exploratory);
        let receipts = receipts_for(&candidate);
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::QualifiedForDeclaredUse);
        assert_eq!(report.qualified_obligation_count, report.required_obligation_count);
    }

    #[test]
    fn forecast_claim_is_not_qualified_by_exploratory_receipts() {
        let exploratory = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::Exploratory);
        let forecast = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::Forecast);
        let receipts = receipts_for(&exploratory);
        let report = assess_claim_assurance(&forecast, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::NotAssessed);
        assert!(report.obligations.iter().any(|item| {
            item.evidence_type == EvidenceType::SplitPredictiveValidation
                && item.status == ObligationStatus::Missing
        }));
    }

    #[test]
    fn high_influence_or_high_consequence_requires_tier_three() {
        let candidate = context(
            OrdinalLevel::High,
            OrdinalLevel::Low,
            ClaimClass::Exploratory,
        );
        let report = assess_claim_assurance(&candidate, &[]).expect("assurance report");
        assert_eq!(report.evidence_tier, EvidenceTier::Tier3);
        assert!(report.required_obligation_count > 6);
    }

    #[test]
    fn predictive_transfer_receipt_cannot_satisfy_causal_identification() {
        let candidate = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::CausalEffect);
        let mut receipts = receipts_for(&candidate);
        receipts.retain(|item| item.evidence_type != EvidenceType::IdentificationAssumptions);
        receipts.push(receipt(
            EvidenceType::SplitPredictiveValidation,
            EvidenceTier::Tier3,
            ClaimClass::CausalEffect,
        ));
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::NotAssessed);
        assert!(report.obligations.iter().any(|item| {
            item.evidence_type == EvidenceType::IdentificationAssumptions
                && item.status == ObligationStatus::Missing
        }));
    }

    #[test]
    fn failed_physical_conservation_rejects_policy_even_if_everything_else_passes() {
        let candidate = context(
            OrdinalLevel::High,
            OrdinalLevel::High,
            ClaimClass::PolicyRecommendation,
        );
        let mut receipts = receipts_for(&candidate);
        let conservation = receipts
            .iter_mut()
            .find(|item| item.evidence_type == EvidenceType::PhysicalConservation)
            .expect("physical conservation obligation");
        conservation.status = EvidenceStatus::Fail;
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::RejectedForDeclaredUse);
        assert!(report.obligations.iter().any(|item| {
            item.evidence_type == EvidenceType::PhysicalConservation
                && item.status == ObligationStatus::Failed
        }));
    }

    #[test]
    fn changed_configuration_and_out_of_scope_population_do_not_count() {
        let candidate = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::Exploratory);
        let mut receipts = receipts_for(&candidate);
        receipts[0].configuration_digest = "sha256-stale-config".to_string();
        receipts[1].population_id = "other-population".to_string();
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::NotAssessed);
        assert!(report.obligations.iter().any(|item| {
            item.status == ObligationStatus::StaleOrOutOfScope
        }));
    }

    #[test]
    fn insufficient_tier_and_non_independent_review_do_not_qualify() {
        let candidate = context(OrdinalLevel::Moderate, OrdinalLevel::Moderate, ClaimClass::Exploratory);
        let mut receipts = receipts_for(&candidate);
        let review = receipts
            .iter_mut()
            .find(|item| item.evidence_type == EvidenceType::IndependentReview)
            .expect("independent review");
        review.independent_review = false;
        review.evidence_tier = EvidenceTier::Tier1;
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::NotAssessed);
        assert!(report.obligations.iter().any(|item| {
            item.evidence_type == EvidenceType::IndependentReview
                && item.status == ObligationStatus::InsufficientTier
        }));
    }

    #[test]
    fn missing_context_source_or_split_identity_fails_closed() {
        let mut candidate = context(OrdinalLevel::Moderate, OrdinalLevel::Moderate, ClaimClass::Forecast);
        candidate.split_manifest_digest = None;
        let receipts = receipts_for(&context(
            OrdinalLevel::Moderate,
            OrdinalLevel::Moderate,
            ClaimClass::Forecast,
        ));
        let report = assess_claim_assurance(&candidate, &receipts).expect("reportable missing scope");
        assert_eq!(report.status, AssuranceStatus::NotAssessed);
    }

    #[test]
    fn matching_failed_receipt_rejects_claim_and_duplicate_receipt_id_is_invalid() {
        let candidate = context(OrdinalLevel::Low, OrdinalLevel::Low, ClaimClass::Exploratory);
        let mut receipts = receipts_for(&candidate);
        receipts[0].status = EvidenceStatus::Fail;
        let report = assess_claim_assurance(&candidate, &receipts).expect("assurance report");
        assert_eq!(report.status, AssuranceStatus::RejectedForDeclaredUse);

        let mut duplicate = receipts_for(&candidate);
        duplicate.push(duplicate[0].clone());
        assert!(matches!(
            assess_claim_assurance(&candidate, &duplicate),
            Err(ClaimAssuranceError::DuplicateReceiptId { .. })
        ));
    }
}
