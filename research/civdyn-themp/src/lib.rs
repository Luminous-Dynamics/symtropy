// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Machine-auditable theory–empirics passport.
//!
//! This crate checks whether a model's constructs, mechanisms, operationalizations,
//! evidence-use roles, predictions and alternative explanations are explicit and
//! internally consistent. It does not determine whether a theory is true.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPurpose {
    Explanation,
    Prediction,
    Description,
    Exploration,
}

impl ModelPurpose {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Explanation => "explanation",
            Self::Prediction => "prediction",
            Self::Description => "description",
            Self::Exploration => "exploration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingPolicy {
    UnknownNotZero,
    NotMeasured,
    SeparateMissingCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroSemantics {
    ZeroIsValue,
    ZeroMeansNoObservation,
    ZeroNotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceRole {
    Characterization,
    Calibration,
    ProcessCredibility,
    InternalValidation,
    Verification,
    ExternalValidation,
    SensitivityExploration,
    CounterfactualExploration,
}

impl EvidenceRole {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Characterization => "characterization",
            Self::Calibration => "calibration",
            Self::ProcessCredibility => "process_credibility",
            Self::InternalValidation => "internal_validation",
            Self::Verification => "verification",
            Self::ExternalValidation => "external_validation",
            Self::SensitivityExploration => "sensitivity_exploration",
            Self::CounterfactualExploration => "counterfactual_exploration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    HistoricalObservation,
    ContemporaryObservation,
    ModelInference,
    CounterfactualProjection,
}

impl EvidenceClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::HistoricalObservation => "historical_observation",
            Self::ContemporaryObservation => "contemporary_observation",
            Self::ModelInference => "model_inference",
            Self::CounterfactualProjection => "counterfactual_projection",
        }
    }

    const fn is_empirical_observation(self) -> bool {
        matches!(
            self,
            Self::HistoricalObservation | Self::ContemporaryObservation
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessStatus {
    Open,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedDirection {
    Positive,
    Negative,
    EquivalentWithinDeclaredMargin,
    NonMonotonic,
    Distributional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternLevel {
    Micro,
    Meso,
    Macro,
    Ecological,
    Institutional,
    InterSystem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSpec {
    pub id: String,
    pub description: String,
    pub level: PatternLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Construct {
    pub id: String,
    pub definition: String,
    pub theoretical_basis: String,
    pub theory_only: bool,
    pub theory_only_reason: Option<String>,
    pub operationalization_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indicator {
    pub id: String,
    pub construct_id: String,
    pub definition: String,
    pub unit: String,
    pub aggregation_rule: String,
    pub missing_policy: MissingPolicy,
    pub zero_semantics: ZeroSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanismHypothesis {
    pub id: String,
    pub input_construct_ids: Vec<String>,
    pub outcome_construct_id: String,
    pub description: String,
    pub theoretical_rationale: String,
    pub lag_steps: u32,
    pub boundary_conditions: Vec<String>,
    pub falsifier: String,
    pub alternative_explanation_ids: Vec<String>,
    /// Required when no alternative explanation is yet identified.
    pub open_alternative_research_gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternativeExplanation {
    pub id: String,
    pub target_mechanism_id: String,
    pub description: String,
    /// Patterns on which this alternative makes a different prediction.
    pub discriminating_pattern_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionExpectation {
    pub id: String,
    pub mechanism_id: String,
    pub pattern_id: String,
    pub outcome_indicator_id: String,
    pub expected_direction: ExpectedDirection,
    pub holdout_partition_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceUse {
    pub id: String,
    pub source_id: String,
    pub source_family_id: String,
    pub partition_id: String,
    pub role: EvidenceRole,
    pub evidence_class: EvidenceClass,
    pub source_digest: String,
    pub access_status: AccessStatus,
    pub uncertainty_statement: String,
    pub pattern_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPassport {
    pub id: String,
    pub purpose: ModelPurpose,
    pub target_phenomenon: String,
    pub system_boundary: String,
    pub target_population_or_system: String,
    pub temporal_scale: String,
    pub spatial_scale: String,
    pub constructs: Vec<Construct>,
    pub indicators: Vec<Indicator>,
    pub mechanisms: Vec<MechanismHypothesis>,
    pub alternatives: Vec<AlternativeExplanation>,
    pub patterns: Vec<PatternSpec>,
    pub predictions: Vec<PredictionExpectation>,
    pub evidence_uses: Vec<EvidenceUse>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Error,
    Warning,
    Information,
}

impl FindingSeverity {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceFinding {
    pub severity: FindingSeverity,
    pub code: &'static str,
    pub subject_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    pub passport_id: String,
    pub purpose: ModelPurpose,
    pub findings: Vec<ConformanceFinding>,
    pub checked_constructs: usize,
    pub checked_indicators: usize,
    pub checked_mechanisms: usize,
    pub checked_alternatives: usize,
    pub checked_predictions: usize,
    pub checked_evidence_uses: usize,
}

impl ConformanceReport {
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == FindingSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == FindingSeverity::Warning)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassportError {
    EmptyPassportId,
}

impl fmt::Display for PassportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPassportId => write!(f, "passport ID must not be empty"),
        }
    }
}

impl std::error::Error for PassportError {}

pub fn evaluate_passport(passport: &ModelPassport) -> Result<ConformanceReport, PassportError> {
    if passport.id.trim().is_empty() {
        return Err(PassportError::EmptyPassportId);
    }

    let mut findings = Vec::new();
    let mut construct_ids = BTreeSet::new();
    let mut indicator_ids = BTreeSet::new();
    let mut mechanism_ids = BTreeSet::new();
    let mut alternative_ids = BTreeSet::new();
    let mut pattern_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();

    for value in [
        ("target_phenomenon", passport.target_phenomenon.as_str()),
        ("system_boundary", passport.system_boundary.as_str()),
        (
            "target_population_or_system",
            passport.target_population_or_system.as_str(),
        ),
        ("temporal_scale", passport.temporal_scale.as_str()),
        ("spatial_scale", passport.spatial_scale.as_str()),
    ] {
        if value.1.trim().is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "scope_field_empty",
                value.0,
                "model purpose and target-system scope must be explicit",
            );
        }
    }

    for construct in &passport.constructs {
        if construct.id.trim().is_empty() || !construct_ids.insert(construct.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "construct_id_empty_or_duplicate",
                &construct.id,
                "construct IDs must be non-empty and unique",
            );
        }
        if construct.definition.trim().is_empty() || construct.theoretical_basis.trim().is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "construct_undefined",
                &construct.id,
                "construct definition and theoretical basis are required",
            );
        }
        if construct.theory_only {
            if construct
                .theory_only_reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "theory_only_reason_missing",
                    &construct.id,
                    "a non-operationalized theory-only construct needs an explicit reason",
                );
            }
        } else if construct.operationalization_ids.is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "construct_not_operationalized",
                &construct.id,
                "construct has no linked observation indicator and is not declared theory-only",
            );
        }
    }

    for indicator in &passport.indicators {
        if indicator.id.trim().is_empty() || !indicator_ids.insert(indicator.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "indicator_id_empty_or_duplicate",
                &indicator.id,
                "indicator IDs must be non-empty and unique",
            );
        }
        if !construct_ids.contains(indicator.construct_id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "indicator_construct_unknown",
                &indicator.id,
                format!("unknown construct {}", indicator.construct_id),
            );
        }
        for (field, value) in [
            ("definition", indicator.definition.as_str()),
            ("unit", indicator.unit.as_str()),
            ("aggregation_rule", indicator.aggregation_rule.as_str()),
        ] {
            if value.trim().is_empty() {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "indicator_operationalization_incomplete",
                    &indicator.id,
                    format!("indicator {field} must be explicit"),
                );
            }
        }
        if let Some(construct) = passport
            .constructs
            .iter()
            .find(|construct| construct.id == indicator.construct_id)
            && !construct
                .operationalization_ids
                .iter()
                .any(|id| id == &indicator.id)
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "indicator_not_reciprocally_linked",
                &indicator.id,
                format!(
                    "construct {} does not declare this operationalization",
                    indicator.construct_id
                ),
            );
        }
    }

    for construct in &passport.constructs {
        for indicator_id in &construct.operationalization_ids {
            if !indicator_ids.contains(indicator_id.as_str()) {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "construct_indicator_unknown",
                    &construct.id,
                    format!("unknown operationalization {indicator_id}"),
                );
            }
        }
    }

    for pattern in &passport.patterns {
        if pattern.id.trim().is_empty() || !pattern_ids.insert(pattern.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "pattern_id_empty_or_duplicate",
                &pattern.id,
                "pattern IDs must be non-empty and unique",
            );
        }
        if pattern.description.trim().is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "pattern_description_missing",
                &pattern.id,
                "a pattern needs an observable description",
            );
        }
    }

    for mechanism in &passport.mechanisms {
        if mechanism.id.trim().is_empty() || !mechanism_ids.insert(mechanism.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "mechanism_id_empty_or_duplicate",
                &mechanism.id,
                "mechanism IDs must be non-empty and unique",
            );
        }
        if mechanism.description.trim().is_empty()
            || mechanism.theoretical_rationale.trim().is_empty()
            || mechanism.falsifier.trim().is_empty()
            || mechanism.boundary_conditions.is_empty()
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "mechanism_explanation_incomplete",
                &mechanism.id,
                "mechanism description, theoretical rationale, falsifier and boundary conditions are required",
            );
        }
        for construct_id in mechanism
            .input_construct_ids
            .iter()
            .chain(std::iter::once(&mechanism.outcome_construct_id))
        {
            if !construct_ids.contains(construct_id.as_str()) {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "mechanism_construct_unknown",
                    &mechanism.id,
                    format!("unknown construct {construct_id}"),
                );
            }
        }
        if mechanism.alternative_explanation_ids.is_empty()
            && mechanism
                .open_alternative_research_gap
                .as_deref()
                .is_none_or(|gap| gap.trim().is_empty())
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "alternative_explanation_or_gap_missing",
                &mechanism.id,
                "provide a competing explanation or document the open alternative-research gap",
            );
        }
    }

    for alternative in &passport.alternatives {
        if alternative.id.trim().is_empty() || !alternative_ids.insert(alternative.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "alternative_id_empty_or_duplicate",
                &alternative.id,
                "alternative explanation IDs must be non-empty and unique",
            );
        }
        if !mechanism_ids.contains(alternative.target_mechanism_id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "alternative_target_unknown",
                &alternative.id,
                format!(
                    "unknown target mechanism {}",
                    alternative.target_mechanism_id
                ),
            );
        }
        if alternative.description.trim().is_empty()
            || alternative.discriminating_pattern_ids.is_empty()
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "alternative_not_discriminable",
                &alternative.id,
                "alternative explanation needs a description and at least one discriminating pattern",
            );
        }
        for pattern_id in &alternative.discriminating_pattern_ids {
            if !pattern_ids.contains(pattern_id.as_str()) {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "alternative_pattern_unknown",
                    &alternative.id,
                    format!("unknown discriminating pattern {pattern_id}"),
                );
            }
        }
        if let Some(mechanism) = passport
            .mechanisms
            .iter()
            .find(|mechanism| mechanism.id == alternative.target_mechanism_id)
            && !mechanism
                .alternative_explanation_ids
                .iter()
                .any(|id| id == &alternative.id)
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "alternative_not_reciprocally_linked",
                &alternative.id,
                format!(
                    "mechanism {} does not declare this alternative",
                    mechanism.id
                ),
            );
        }
    }

    for mechanism in &passport.mechanisms {
        for alternative_id in &mechanism.alternative_explanation_ids {
            if !alternative_ids.contains(alternative_id.as_str()) {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "mechanism_alternative_unknown",
                    &mechanism.id,
                    format!("unknown alternative explanation {alternative_id}"),
                );
            }
        }
    }

    for prediction in &passport.predictions {
        if prediction.id.trim().is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_id_empty",
                &prediction.id,
                "prediction IDs must be non-empty",
            );
        }
        if !mechanism_ids.contains(prediction.mechanism_id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_mechanism_unknown",
                &prediction.id,
                format!("unknown mechanism {}", prediction.mechanism_id),
            );
        }
        if !pattern_ids.contains(prediction.pattern_id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_pattern_unknown",
                &prediction.id,
                format!("unknown pattern {}", prediction.pattern_id),
            );
        }
        if !indicator_ids.contains(prediction.outcome_indicator_id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_indicator_unknown",
                &prediction.id,
                format!("unknown indicator {}", prediction.outcome_indicator_id),
            );
        }
        if prediction.holdout_partition_id.trim().is_empty() {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_holdout_missing",
                &prediction.id,
                "prediction must declare a held-out evaluation partition",
            );
        }
        let has_external_evidence = passport.evidence_uses.iter().any(|evidence| {
            evidence.role == EvidenceRole::ExternalValidation
                && evidence.partition_id == prediction.holdout_partition_id
                && evidence
                    .pattern_ids
                    .iter()
                    .any(|id| id == &prediction.pattern_id)
                && evidence.evidence_class.is_empirical_observation()
        });
        if !has_external_evidence {
            push(
                &mut findings,
                FindingSeverity::Error,
                "prediction_without_external_observation",
                &prediction.id,
                "prediction pattern has no linked empirical external-validation evidence in the declared holdout partition",
            );
        }
    }

    let mut evidence_partition_roles: BTreeMap<(&str, &str), BTreeSet<&'static str>> =
        BTreeMap::new();
    for evidence in &passport.evidence_uses {
        if evidence.id.trim().is_empty() || !evidence_ids.insert(evidence.id.as_str()) {
            push(
                &mut findings,
                FindingSeverity::Error,
                "evidence_use_id_empty_or_duplicate",
                &evidence.id,
                "evidence-use IDs must be non-empty and unique",
            );
        }
        for (field, value) in [
            ("source_id", evidence.source_id.as_str()),
            ("source_family_id", evidence.source_family_id.as_str()),
            ("partition_id", evidence.partition_id.as_str()),
            ("source_digest", evidence.source_digest.as_str()),
            (
                "uncertainty_statement",
                evidence.uncertainty_statement.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "evidence_provenance_incomplete",
                    &evidence.id,
                    format!("evidence {field} must be explicit"),
                );
            }
        }
        for pattern_id in &evidence.pattern_ids {
            if !pattern_ids.contains(pattern_id.as_str()) {
                push(
                    &mut findings,
                    FindingSeverity::Error,
                    "evidence_pattern_unknown",
                    &evidence.id,
                    format!("unknown linked pattern {pattern_id}"),
                );
            }
        }
        if evidence.access_status == AccessStatus::Unknown {
            push(
                &mut findings,
                FindingSeverity::Warning,
                "evidence_access_status_unknown",
                &evidence.id,
                "license, consent or reuse status must be resolved before external use",
            );
        }
        if evidence.role == EvidenceRole::ExternalValidation
            && !evidence.evidence_class.is_empirical_observation()
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "non_empirical_external_validation",
                &evidence.id,
                "model inference or counterfactual projection cannot stand in for external empirical validation",
            );
        }
        if evidence.role == EvidenceRole::Calibration
            && !evidence.evidence_class.is_empirical_observation()
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "non_empirical_calibration",
                &evidence.id,
                "calibration must use an explicitly classified empirical observation",
            );
        }
        evidence_partition_roles
            .entry((evidence.source_id.as_str(), evidence.partition_id.as_str()))
            .or_default()
            .insert(evidence.role.name());
    }

    for ((source_id, partition_id), roles) in &evidence_partition_roles {
        if roles.contains(EvidenceRole::Calibration.name())
            && roles.contains(EvidenceRole::ExternalValidation.name())
        {
            push(
                &mut findings,
                FindingSeverity::Error,
                "calibration_external_partition_leakage",
                source_id,
                format!(
                    "source {source_id} partition {partition_id} is used for both calibration and external validation"
                ),
            );
        }
    }

    if passport.purpose == ModelPurpose::Explanation && passport.mechanisms.is_empty() {
        push(
            &mut findings,
            FindingSeverity::Error,
            "explanation_without_mechanisms",
            &passport.id,
            "an explanatory model needs explicit generative mechanisms",
        );
    }
    if passport.purpose == ModelPurpose::Prediction && passport.predictions.is_empty() {
        push(
            &mut findings,
            FindingSeverity::Error,
            "prediction_without_expectations",
            &passport.id,
            "a predictive model needs predeclared held-out prediction expectations",
        );
    }
    if passport.purpose == ModelPurpose::Exploration
        && passport.mechanisms.is_empty()
        && passport.predictions.is_empty()
    {
        push(
            &mut findings,
            FindingSeverity::Warning,
            "exploration_without_testable_model",
            &passport.id,
            "exploration should declare a mechanism or explicit pattern expectation",
        );
    }

    Ok(ConformanceReport {
        passport_id: passport.id.clone(),
        purpose: passport.purpose,
        findings,
        checked_constructs: passport.constructs.len(),
        checked_indicators: passport.indicators.len(),
        checked_mechanisms: passport.mechanisms.len(),
        checked_alternatives: passport.alternatives.len(),
        checked_predictions: passport.predictions.len(),
        checked_evidence_uses: passport.evidence_uses.len(),
    })
}

fn push(
    findings: &mut Vec<ConformanceFinding>,
    severity: FindingSeverity,
    code: &'static str,
    subject_id: &str,
    detail: impl Into<String>,
) {
    findings.push(ConformanceFinding {
        severity,
        code,
        subject_id: subject_id.to_owned(),
        detail: detail.into(),
    });
}

pub fn reference_passport() -> ModelPassport {
    ModelPassport {
        id: "themp_reference_ses".into(),
        purpose: ModelPurpose::Explanation,
        target_phenomenon: "cooperative water allocation under collective scarcity".into(),
        system_boundary: "one basin, two districts and four household service nodes".into(),
        target_population_or_system: "synthetic socio-ecological service network".into(),
        temporal_scale: "daily time steps".into(),
        spatial_scale: "basin to household".into(),
        constructs: vec![
            Construct {
                id: "institutional_trust".into(),
                definition: "confidence that allocation procedures and reciprocal commitments will be honoured".into(),
                theoretical_basis: "credible commitment and repeated cooperation mechanisms".into(),
                theory_only: false,
                theory_only_reason: None,
                operationalization_ids: vec!["trust_survey_index".into()],
            },
            Construct {
                id: "essential_service".into(),
                definition: "fraction of declared essential water demand delivered per household".into(),
                theoretical_basis: "service access as a proximal outcome of resource allocation".into(),
                theory_only: false,
                theory_only_reason: None,
                operationalization_ids: vec!["water_service_coverage".into()],
            },
            Construct {
                id: "reciprocal_expectation".into(),
                definition: "expectation that other districts will reciprocate resource sharing".into(),
                theoretical_basis: "conditional cooperation under repeated common-pool dilemmas".into(),
                theory_only: true,
                theory_only_reason: Some("v0 records this latent construct as a theory term; a validated survey or behavioural measure is not yet attached".into()),
                operationalization_ids: Vec::new(),
            },
        ],
        indicators: vec![
            Indicator {
                id: "trust_survey_index".into(),
                construct_id: "institutional_trust".into(),
                definition: "mean of a predeclared, source-versioned trust questionnaire scale".into(),
                unit: "scale_0_1".into(),
                aggregation_rule: "respondent mean within district and survey wave".into(),
                missing_policy: MissingPolicy::UnknownNotZero,
                zero_semantics: ZeroSemantics::ZeroIsValue,
            },
            Indicator {
                id: "water_service_coverage".into(),
                construct_id: "essential_service".into(),
                definition: "delivered essential water divided by declared essential water demand".into(),
                unit: "fraction".into(),
                aggregation_rule: "report household distribution and district quantiles; do not pool households into one unstratified mean".into(),
                missing_policy: MissingPolicy::SeparateMissingCategory,
                zero_semantics: ZeroSemantics::ZeroIsValue,
            },
        ],
        mechanisms: vec![
            MechanismHypothesis {
                id: "trust_reciprocity_allocation".into(),
                input_construct_ids: vec![
                    "institutional_trust".into(),
                    "reciprocal_expectation".into(),
                ],
                outcome_construct_id: "essential_service".into(),
                description: "credible reciprocal expectations increase willingness to share basin water, improving household water coverage when transfers are physically feasible".into(),
                theoretical_rationale: "repeated cooperation and credible commitment in common-pool resource allocation".into(),
                lag_steps: 1,
                boundary_conditions: vec![
                    "the upstream basin retains transferable stock after its declared reserve".into(),
                    "the transfer network remains physically available".into(),
                ],
                falsifier: "under matched exogenous supply and transfer feasibility, changing trust/reciprocity while preserving the allocation rule does not change transfer uptake or downstream coverage within the predeclared detectable range".into(),
                alternative_explanation_ids: vec!["infrastructure_capacity_only".into()],
                open_alternative_research_gap: None,
            },
        ],
        alternatives: vec![
            AlternativeExplanation {
                id: "infrastructure_capacity_only".into(),
                target_mechanism_id: "trust_reciprocity_allocation".into(),
                description: "household service coverage is primarily constrained by pipe and pump capacity; trust changes do not mediate resource transfers".into(),
                discriminating_pattern_ids: vec!["transfer_uptake_vs_capacity".into()],
            },
        ],
        patterns: vec![
            PatternSpec {
                id: "service_coverage_by_household".into(),
                description: "distribution of essential water service coverage across households after a supply shock".into(),
                level: PatternLevel::Micro,
            },
            PatternSpec {
                id: "transfer_uptake_vs_capacity".into(),
                description: "transfer volume conditional on spare network capacity and declared trust/reciprocity state".into(),
                level: PatternLevel::Meso,
            },
        ],
        predictions: vec![
            PredictionExpectation {
                id: "higher_trust_transfer_prediction".into(),
                mechanism_id: "trust_reciprocity_allocation".into(),
                pattern_id: "transfer_uptake_vs_capacity".into(),
                outcome_indicator_id: "water_service_coverage".into(),
                expected_direction: ExpectedDirection::Positive,
                holdout_partition_id: "regional_holdout_02".into(),
            },
        ],
        evidence_uses: vec![
            EvidenceUse {
                id: "theory_source_collective_action".into(),
                source_id: "source_collective_action_review".into(),
                source_family_id: "collective_action_theory".into(),
                partition_id: "literature_scope_2026".into(),
                role: EvidenceRole::Characterization,
                evidence_class: EvidenceClass::HistoricalObservation,
                source_digest: "sha256:theory-source-a".into(),
                access_status: AccessStatus::Open,
                uncertainty_statement: "theoretical synthesis; not a direct measurement of this synthetic task".into(),
                pattern_ids: vec!["transfer_uptake_vs_capacity".into()],
            },
            EvidenceUse {
                id: "calibration_dataset_wave_a".into(),
                source_id: "source_survey_dataset".into(),
                source_family_id: "district_household_survey".into(),
                partition_id: "calibration_wave_01".into(),
                role: EvidenceRole::Calibration,
                evidence_class: EvidenceClass::ContemporaryObservation,
                source_digest: "sha256:survey-calibration-a".into(),
                access_status: AccessStatus::Restricted,
                uncertainty_statement: "measurement error and non-response are reported by district; restricted reuse requires authorization".into(),
                pattern_ids: vec!["service_coverage_by_household".into()],
            },
            EvidenceUse {
                id: "external_validation_wave_b".into(),
                source_id: "source_survey_dataset".into(),
                source_family_id: "district_household_survey".into(),
                partition_id: "regional_holdout_02".into(),
                role: EvidenceRole::ExternalValidation,
                evidence_class: EvidenceClass::ContemporaryObservation,
                source_digest: "sha256:survey-holdout-b".into(),
                access_status: AccessStatus::Restricted,
                uncertainty_statement: "independent regional holdout partition; do not tune model parameters against this partition".into(),
                pattern_ids: vec!["transfer_uptake_vs_capacity".into()],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_passport_separates_theory_calibration_and_external_validation() {
        let report = evaluate_passport(&reference_passport()).unwrap();
        assert!(!report.has_errors());
        assert_eq!(report.checked_constructs, 3);
        assert_eq!(report.checked_mechanisms, 1);
        assert_eq!(report.checked_alternatives, 1);
        assert_eq!(report.checked_predictions, 1);
        assert_eq!(report.checked_evidence_uses, 3);
    }

    #[test]
    fn calibration_and_external_validation_cannot_share_the_same_partition() {
        let mut passport = reference_passport();
        passport.evidence_uses[2].partition_id = "calibration_wave_01".into();
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "calibration_external_partition_leakage" })
        );
    }

    #[test]
    fn unoperationalized_construct_requires_an_explicit_theory_only_reason() {
        let mut passport = reference_passport();
        passport.constructs[0].operationalization_ids.clear();
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "construct_not_operationalized" })
        );

        passport.constructs[0].theory_only = true;
        passport.constructs[0].theory_only_reason =
            Some("latent construct is not directly measured".into());
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| { finding.code == "construct_not_operationalized" })
        );
    }

    #[test]
    fn missing_falsifier_or_boundary_condition_fails_explanatory_claim() {
        let mut passport = reference_passport();
        passport.mechanisms[0].falsifier.clear();
        passport.mechanisms[0].boundary_conditions.clear();
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "mechanism_explanation_incomplete" })
        );
    }

    #[test]
    fn plausible_alternative_must_have_a_discriminating_pattern() {
        let mut passport = reference_passport();
        passport.alternatives[0].discriminating_pattern_ids.clear();
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "alternative_not_discriminable" })
        );
    }

    #[test]
    fn prediction_purpose_needs_predeclared_held_out_expectations() {
        let mut passport = reference_passport();
        passport.purpose = ModelPurpose::Prediction;
        passport.predictions.clear();
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "prediction_without_expectations" })
        );
    }

    #[test]
    fn model_inference_cannot_be_used_as_external_observation() {
        let mut passport = reference_passport();
        passport.evidence_uses[2].evidence_class = EvidenceClass::ModelInference;
        let report = evaluate_passport(&passport).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "non_empirical_external_validation" })
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "prediction_without_external_observation" })
        );
    }

    #[test]
    fn unknown_evidence_access_is_a_warning_not_an_empirical_pass() {
        let mut passport = reference_passport();
        passport.evidence_uses[0].access_status = AccessStatus::Unknown;
        let report = evaluate_passport(&passport).unwrap();
        assert!(!report.has_errors());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn missing_is_not_silently_collapsed_to_zero() {
        let mut passport = reference_passport();
        passport.indicators[0].missing_policy = MissingPolicy::UnknownNotZero;
        passport.indicators[0].zero_semantics = ZeroSemantics::ZeroIsValue;
        let report = evaluate_passport(&passport).unwrap();
        assert!(!report.has_errors());
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.code == "zero_semantics_needs_review")
        );

        passport.indicators[0].zero_semantics = ZeroSemantics::ZeroMeansNoObservation;
        let report = evaluate_passport(&passport).unwrap();
        assert!(!report.has_errors());
    }
}
