#![forbid(unsafe_code)]

//! Machine-checkable target-trial protocol and identification gate for
//! observational maritime maintenance records.
//!
//! This module does not estimate causal effects. It checks whether the target
//! trial is sufficiently specified, temporally aligned and supported by its
//! declared diagnostics to be considered by a separately qualified estimator.
//! It must return NOT_IDENTIFIED when the data or assumptions do not support
//! the requested causal contrast.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TARGET_TRIAL_SCHEMA: &str = "symtropy-marine-observational-target-trial-v0.1";
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EmulationDesign {
    ParallelGroup,
    SequentialTrial,
    CloneCensorWeight,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CausalContrast {
    AssignmentEffect,
    PerProtocolEffect,
    TreatmentPolicyEffect,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterventionKind {
    PhysicalComponentRepair,
    AlternativeRepair,
    SensorOnlyCorrection,
    DeferredOrUsualCare,
    DynamicMaintenanceStrategy,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CovariateTiming {
    BaselinePreTreatment,
    TimeVaryingPreTreatment,
    PostTreatment,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionDomain {
    Consistency,
    ConditionalExchangeability,
    Positivity,
    NoInterference,
    CensoringExchangeability,
    MeasurementValidity,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Supported,
    Violated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FollowupEndReason {
    Outcome,
    Censoring,
    CompetingEvent,
    AdministrativeEnd,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TreatmentStrategySpec {
    pub strategy_id: String,
    pub strategy_version_sha256: String,
    pub intervention_kind: InterventionKind,
    pub strategy_definition_sha256: String,
    pub grace_period_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineCovariateSpec {
    pub covariate_id: String,
    pub definition_sha256: String,
    pub timing: CovariateTiming,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrimaryOutcomeSpec {
    pub outcome_id: String,
    pub metric_id: String,
    pub unit: String,
    pub definition_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IdentificationThresholds {
    pub minimum_common_support_fraction: f64,
    pub minimum_arm_effective_sample_size: f64,
    pub maximum_absolute_standardized_mean_difference: f64,
    pub maximum_stabilized_weight: f64,
    pub minimum_followup_completeness_fraction: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetTrialProtocol {
    pub schema_version: String,
    pub trial_id: String,
    pub target_population_id: String,
    pub eligibility_rule_sha256: String,
    pub assignment_rule_sha256: String,
    pub causal_graph_sha256: String,
    pub analysis_plan_sha256: String,
    pub cluster_group_key: String,
    pub cluster_grouping_rule_sha256: String,
    pub cluster_robust_inference_declared: bool,
    pub emulation_design: EmulationDesign,
    pub causal_contrast: CausalContrast,
    pub align_eligibility_assignment_followup_at_time_zero: bool,
    pub followup_horizon_seconds: f64,
    pub followup_exposure_unit: String,
    pub treatment_strategies: Vec<TreatmentStrategySpec>,
    pub baseline_covariates: Vec<BaselineCovariateSpec>,
    pub primary_outcomes: Vec<PrimaryOutcomeSpec>,
    pub thresholds: IdentificationThresholds,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StrategyArmCount {
    pub strategy_id: String,
    pub emulation_entry_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssumptionAssessment {
    pub domain: AssumptionDomain,
    pub status: AssessmentStatus,
    pub evidence_sha256: String,
    pub reviewer_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverlapDiagnostics {
    pub common_support_fraction: f64,
    pub minimum_arm_effective_sample_size: f64,
    pub maximum_absolute_standardized_mean_difference: f64,
    pub maximum_stabilized_weight: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineCovariateEvidence {
    pub covariate_id: String,
    pub available_at_seconds: f64,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmulationUnitTimeline {
    /// Unique emulation entry; a physical asset may enter sequential trial emulations.
    pub emulation_unit_id: String,
    pub asset_id: String,
    pub strategy_id: String,
    pub eligible_at_seconds: f64,
    pub strategy_assignment_at_seconds: f64,
    pub followup_start_at_seconds: f64,
    pub treatment_start_at_seconds: Option<f64>,
    pub followup_end_at_seconds: f64,
    pub followup_end_reason: FollowupEndReason,
    pub baseline_covariates: Vec<BaselineCovariateEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationalEmulationDiagnostics {
    pub trial_id: String,
    pub data_snapshot_sha256: String,
    pub eligible_emulation_entry_count: usize,
    pub independent_cluster_count: usize,
    pub strategy_arm_counts: Vec<StrategyArmCount>,
    pub overlap: OverlapDiagnostics,
    pub followup_completeness_fraction: f64,
    pub assumption_assessments: Vec<AssumptionAssessment>,
    pub negative_control_status: AssessmentStatus,
    pub sensitivity_analysis_status: AssessmentStatus,
    pub unit_timelines: Vec<EmulationUnitTimeline>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetTrialDisposition {
    ProtocolIncomplete,
    NotIdentified,
    DesignQualifiedForEstimator,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TargetTrialValidationReport {
    pub schema_version: String,
    pub trial_id: String,
    pub disposition: TargetTrialDisposition,
    pub qualified_emulation_entry_count: usize,
    pub blockers: Vec<String>,
    pub assumptions_supported: usize,
    pub assumptions_unknown_or_violated: usize,
    pub causal_effect_estimated: bool,
    pub maintenance_authorized: bool,
    pub physical_restoration_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn validate_protocol(protocol: &TargetTrialProtocol) -> Vec<String> {
    let mut blockers = Vec::new();
    if protocol.schema_version != TARGET_TRIAL_SCHEMA {
        blockers.push("unexpected target-trial schema version".into());
    }
    if protocol.trial_id.trim().is_empty() || protocol.target_population_id.trim().is_empty() {
        blockers.push("trial and target-population identities must be non-empty".into());
    }
    for (name, digest) in [
        ("eligibility rule", &protocol.eligibility_rule_sha256),
        ("assignment rule", &protocol.assignment_rule_sha256),
        ("causal graph", &protocol.causal_graph_sha256),
        ("analysis plan", &protocol.analysis_plan_sha256),
    ] {
        if !is_sha256_hex(digest) {
            blockers.push(format!("{name} digest is invalid"));
        }
    }
    if protocol.cluster_group_key != "asset_id"
        || !is_sha256_hex(&protocol.cluster_grouping_rule_sha256)
        || !protocol.cluster_robust_inference_declared
    {
        blockers.push(
            "cluster unit, grouping rule or cluster-robust uncertainty plan is missing".into(),
        );
    }
    if !protocol.align_eligibility_assignment_followup_at_time_zero {
        blockers.push(
            "eligibility, strategy assignment and follow-up are not aligned at time zero".into(),
        );
    }
    if !finite_non_negative(protocol.followup_horizon_seconds)
        || protocol.followup_horizon_seconds <= 0.0
        || protocol.followup_exposure_unit.trim().is_empty()
    {
        blockers.push("follow-up horizon or exposure unit is invalid".into());
    }
    if protocol.treatment_strategies.len() < 2 {
        blockers.push("at least two explicit treatment strategies are required".into());
    }
    if protocol.primary_outcomes.is_empty() {
        blockers.push("at least one primary outcome is required".into());
    }
    if protocol.baseline_covariates.is_empty() {
        blockers.push("baseline confounder set is not declared".into());
    }

    let mut strategy_ids = BTreeSet::new();
    let mut strategies_with_grace = false;
    for strategy in &protocol.treatment_strategies {
        if strategy.strategy_id.trim().is_empty()
            || !strategy_ids.insert(strategy.strategy_id.clone())
            || !is_sha256_hex(&strategy.strategy_version_sha256)
            || !is_sha256_hex(&strategy.strategy_definition_sha256)
            || !finite_non_negative(strategy.grace_period_seconds)
        {
            blockers.push(format!(
                "treatment strategy identity, version, definition or grace period is invalid: {}",
                strategy.strategy_id
            ));
        }
        strategies_with_grace |= strategy.grace_period_seconds > 0.0;
    }
    if strategies_with_grace && protocol.emulation_design != EmulationDesign::CloneCensorWeight {
        blockers.push(
            "a treatment grace period requires an explicit clone-censor-weight emulation design"
                .into(),
        );
    }

    let mut covariate_ids = BTreeSet::new();
    for covariate in &protocol.baseline_covariates {
        if covariate.covariate_id.trim().is_empty()
            || !covariate_ids.insert(covariate.covariate_id.clone())
            || !is_sha256_hex(&covariate.definition_sha256)
        {
            blockers.push(format!(
                "baseline covariate identity or definition is invalid: {}",
                covariate.covariate_id
            ));
        }
        if covariate.timing != CovariateTiming::BaselinePreTreatment {
            blockers.push(format!(
                "covariate {} is not an admissible baseline pre-treatment covariate",
                covariate.covariate_id
            ));
        }
    }

    let mut outcome_ids = BTreeSet::new();
    for outcome in &protocol.primary_outcomes {
        if outcome.outcome_id.trim().is_empty()
            || outcome.metric_id.trim().is_empty()
            || outcome.unit.trim().is_empty()
            || !outcome_ids.insert(outcome.outcome_id.clone())
            || !is_sha256_hex(&outcome.definition_sha256)
        {
            blockers.push(format!(
                "primary outcome identity, metric, unit or definition is invalid: {}",
                outcome.outcome_id
            ));
        }
    }

    let thresholds = &protocol.thresholds;
    if !thresholds.minimum_common_support_fraction.is_finite()
        || !(0.0..=1.0).contains(&thresholds.minimum_common_support_fraction)
        || !finite_non_negative(thresholds.minimum_arm_effective_sample_size)
        || !finite_non_negative(thresholds.maximum_absolute_standardized_mean_difference)
        || !finite_non_negative(thresholds.maximum_stabilized_weight)
        || !thresholds.minimum_followup_completeness_fraction.is_finite()
        || !(0.0..=1.0).contains(&thresholds.minimum_followup_completeness_fraction)
    {
        blockers.push("identification thresholds are invalid".into());
    }
    blockers
}

fn validate_assumptions(
    diagnostics: &ObservationalEmulationDiagnostics,
) -> (usize, usize, Vec<String>) {
    let required = [
        AssumptionDomain::Consistency,
        AssumptionDomain::ConditionalExchangeability,
        AssumptionDomain::Positivity,
        AssumptionDomain::NoInterference,
        AssumptionDomain::CensoringExchangeability,
        AssumptionDomain::MeasurementValidity,
    ];
    let mut by_domain = BTreeMap::new();
    let mut blockers = Vec::new();
    for assessment in &diagnostics.assumption_assessments {
        if !is_sha256_hex(&assessment.evidence_sha256) || assessment.reviewer_ref.trim().is_empty()
        {
            blockers.push(format!(
                "assumption assessment lacks evidence or reviewer: {:?}",
                assessment.domain
            ));
        }
        if by_domain.insert(assessment.domain, assessment.status).is_some() {
            blockers.push(format!(
                "assumption is assessed more than once: {:?}",
                assessment.domain
            ));
        }
    }
    let mut supported = 0;
    let mut unknown_or_violated = 0;
    for domain in required {
        match by_domain.get(&domain) {
            Some(&AssessmentStatus::Supported) => supported += 1,
            Some(&AssessmentStatus::Unknown) | Some(&AssessmentStatus::Violated) | None => {
                unknown_or_violated += 1;
                blockers.push(format!(
                    "identifying assumption is unknown, violated or missing: {domain:?}"
                ));
            }
        }
    }
    (supported, unknown_or_violated, blockers)
}

fn validate_timelines(
    protocol: &TargetTrialProtocol,
    diagnostics: &ObservationalEmulationDiagnostics,
) -> Vec<String> {
    let mut blockers = Vec::new();
    let strategy_map = protocol
        .treatment_strategies
        .iter()
        .map(|strategy| (strategy.strategy_id.as_str(), strategy))
        .collect::<BTreeMap<_, _>>();
    let covariate_ids = protocol
        .baseline_covariates
        .iter()
        .map(|covariate| covariate.covariate_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut entry_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    let mut arm_counts = BTreeMap::<String, usize>::new();

    for timeline in &diagnostics.unit_timelines {
        if timeline.emulation_unit_id.trim().is_empty()
            || timeline.asset_id.trim().is_empty()
            || !entry_ids.insert(timeline.emulation_unit_id.clone())
        {
            blockers.push(format!(
                "emulation unit identity is empty or duplicated: {}",
                timeline.emulation_unit_id
            ));
        }
        let Some(strategy) = strategy_map.get(timeline.strategy_id.as_str()) else {
            blockers.push(format!(
                "unit {} is assigned to an undeclared treatment strategy",
                timeline.emulation_unit_id
            ));
            continue;
        };
        *arm_counts.entry(timeline.strategy_id.clone()).or_default() += 1;
        let asset_was_seen = !asset_ids.insert(timeline.asset_id.clone());
        if asset_was_seen && protocol.emulation_design == EmulationDesign::ParallelGroup {
            blockers.push(format!(
                "asset {} appears more than once in a parallel-group emulation",
                timeline.asset_id
            ));
        }

        let times = [
            timeline.eligible_at_seconds,
            timeline.strategy_assignment_at_seconds,
            timeline.followup_start_at_seconds,
            timeline.followup_end_at_seconds,
        ];
        if !times.iter().all(|value| finite_non_negative(*value)) {
            blockers.push(format!(
                "unit {} contains non-finite or negative timeline values",
                timeline.emulation_unit_id
            ));
            continue;
        }
        let time_zero = timeline.eligible_at_seconds;
        if (timeline.strategy_assignment_at_seconds - time_zero).abs() > TIME_TOLERANCE_SECONDS
            || (timeline.followup_start_at_seconds - time_zero).abs()
                > TIME_TOLERANCE_SECONDS
        {
            blockers.push(format!(
                "unit {} does not align eligibility, assignment and follow-up at time zero",
                timeline.emulation_unit_id
            ));
        }
        if timeline.followup_end_at_seconds < timeline.followup_start_at_seconds {
            blockers.push(format!(
                "unit {} has follow-up ending before it starts",
                timeline.emulation_unit_id
            ));
        }
        if timeline.followup_end_reason == FollowupEndReason::AdministrativeEnd
            && timeline.followup_end_at_seconds
                < timeline.followup_start_at_seconds + protocol.followup_horizon_seconds
                    - TIME_TOLERANCE_SECONDS
        {
            blockers.push(format!(
                "unit {} has premature administrative follow-up end",
                timeline.emulation_unit_id
            ));
        }
        if strategy.intervention_kind == InterventionKind::DeferredOrUsualCare
            && timeline.treatment_start_at_seconds.is_some()
        {
            blockers.push(format!(
                "unit {} records a treatment start for a deferred/usual-care strategy",
                timeline.emulation_unit_id
            ));
        }
        if let Some(treatment_start) = timeline.treatment_start_at_seconds {
            if !finite_non_negative(treatment_start)
                || treatment_start < time_zero - TIME_TOLERANCE_SECONDS
                || treatment_start
                    > time_zero + strategy.grace_period_seconds + TIME_TOLERANCE_SECONDS
                || treatment_start
                    > timeline.followup_end_at_seconds + TIME_TOLERANCE_SECONDS
            {
                blockers.push(format!(
                    "unit {} has treatment start outside its declared strategy window",
                    timeline.emulation_unit_id
                ));
            }
        } else if matches!(
            strategy.intervention_kind,
            InterventionKind::PhysicalComponentRepair
                | InterventionKind::AlternativeRepair
                | InterventionKind::SensorOnlyCorrection
                | InterventionKind::DynamicMaintenanceStrategy
        ) {
            blockers.push(format!(
                "unit {} lacks treatment-start evidence for its assigned strategy",
                timeline.emulation_unit_id
            ));
        }

        let mut observed_covariates = BTreeSet::new();
        for covariate in &timeline.baseline_covariates {
            if !covariate_ids.contains(covariate.covariate_id.as_str())
                || !observed_covariates.insert(covariate.covariate_id.as_str())
                || !is_sha256_hex(&covariate.evidence_sha256)
                || !finite_non_negative(covariate.available_at_seconds)
            {
                blockers.push(format!(
                    "unit {} has missing, duplicate or invalid baseline covariate evidence",
                    timeline.emulation_unit_id
                ));
                continue;
            }
            if covariate.available_at_seconds > time_zero + TIME_TOLERANCE_SECONDS {
                blockers.push(format!(
                    "unit {} uses covariate {} that became available after time zero",
                    timeline.emulation_unit_id,
                    covariate.covariate_id
                ));
            }
        }
        if observed_covariates.len() != covariate_ids.len() {
            blockers.push(format!(
                "unit {} lacks one or more declared baseline covariates",
                timeline.emulation_unit_id
            ));
        }
    }

    if diagnostics.unit_timelines.len() != diagnostics.eligible_emulation_entry_count {
        blockers.push(
            "timeline row count does not match the declared emulation-entry count".into(),
        );
    }
    if asset_ids.len() != diagnostics.independent_cluster_count {
        blockers.push(
            "distinct cluster-key count does not match the declared independent-cluster count"
                .into(),
        );
    }
    for strategy in &protocol.treatment_strategies {
        if arm_counts.get(&strategy.strategy_id).copied().unwrap_or(0) == 0 {
            blockers.push(format!(
                "no eligible emulation entries are assigned to strategy {}",
                strategy.strategy_id
            ));
        }
    }

    let mut declared_counts = BTreeMap::<&str, usize>::new();
    for entry in &diagnostics.strategy_arm_counts {
        if !strategy_map.contains_key(entry.strategy_id.as_str())
            || declared_counts
                .insert(entry.strategy_id.as_str(), entry.emulation_entry_count)
                .is_some()
        {
            blockers.push(format!(
                "strategy-arm count is unknown or duplicated: {}",
                entry.strategy_id
            ));
        }
    }
    for strategy in &protocol.treatment_strategies {
        let observed = arm_counts.get(&strategy.strategy_id).copied().unwrap_or(0);
        let declared = declared_counts
            .get(strategy.strategy_id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        if observed != declared {
            blockers.push(format!(
                "declared and timeline strategy-arm counts differ for {}",
                strategy.strategy_id
            ));
        }
    }
    blockers
}

/// Evaluate a protocol and its observational diagnostics without estimating an effect.
/// Design qualification is not causal identification or a field validation result.
pub fn evaluate_observational_target_trial(
    protocol: &TargetTrialProtocol,
    diagnostics: &ObservationalEmulationDiagnostics,
) -> TargetTrialValidationReport {
    let mut blockers = validate_protocol(protocol);
    let protocol_incomplete = !blockers.is_empty();

    if diagnostics.trial_id != protocol.trial_id
        || !is_sha256_hex(&diagnostics.data_snapshot_sha256)
        || diagnostics.eligible_emulation_entry_count == 0
        || diagnostics.independent_cluster_count == 0
        || diagnostics.independent_cluster_count > diagnostics.eligible_emulation_entry_count
    {
        blockers.push("diagnostic snapshot identity or emulation-entry count is invalid".into());
    }

    let (assumptions_supported, assumptions_unknown_or_violated, assumption_blockers) =
        validate_assumptions(diagnostics);
    blockers.extend(assumption_blockers);
    blockers.extend(validate_timelines(protocol, diagnostics));

    let overlap = &diagnostics.overlap;
    if !overlap.common_support_fraction.is_finite()
        || !(0.0..=1.0).contains(&overlap.common_support_fraction)
        || !finite_non_negative(overlap.minimum_arm_effective_sample_size)
        || !finite_non_negative(overlap.maximum_absolute_standardized_mean_difference)
        || !finite_non_negative(overlap.maximum_stabilized_weight)
    {
        blockers.push("overlap diagnostics contain invalid numeric values".into());
    } else {
        if overlap.common_support_fraction
            < protocol.thresholds.minimum_common_support_fraction
        {
            blockers.push("common-support coverage is below the preregistered threshold".into());
        }
        if overlap.minimum_arm_effective_sample_size
            < protocol.thresholds.minimum_arm_effective_sample_size
        {
            blockers.push("effective sample size is below the preregistered threshold".into());
        }
        if overlap.maximum_absolute_standardized_mean_difference
            > protocol.thresholds.maximum_absolute_standardized_mean_difference
        {
            blockers.push("covariate balance is outside the preregistered threshold".into());
        }
        if overlap.maximum_stabilized_weight > protocol.thresholds.maximum_stabilized_weight {
            blockers.push("stabilized weights exceed the preregistered threshold".into());
        }
    }

    if !diagnostics.followup_completeness_fraction.is_finite()
        || !(0.0..=1.0).contains(&diagnostics.followup_completeness_fraction)
    {
        blockers.push("follow-up completeness is invalid".into());
    } else if diagnostics.followup_completeness_fraction
        < protocol.thresholds.minimum_followup_completeness_fraction
    {
        blockers.push("follow-up completeness is below the preregistered threshold".into());
    }

    if diagnostics.negative_control_status != AssessmentStatus::Supported {
        blockers.push("negative control is unknown or violates its preregistered expectation".into());
    }
    if diagnostics.sensitivity_analysis_status != AssessmentStatus::Supported {
        blockers.push("sensitivity analysis is missing, unknown or incomplete".into());
    }

    let disposition = if protocol_incomplete {
        TargetTrialDisposition::ProtocolIncomplete
    } else if !blockers.is_empty() {
        TargetTrialDisposition::NotIdentified
    } else {
        TargetTrialDisposition::DesignQualifiedForEstimator
    };

    TargetTrialValidationReport {
        schema_version: TARGET_TRIAL_SCHEMA.to_owned(),
        trial_id: protocol.trial_id.clone(),
        disposition,
        qualified_emulation_entry_count: if disposition
            == TargetTrialDisposition::DesignQualifiedForEstimator
        {
            diagnostics.eligible_emulation_entry_count
        } else {
            0
        },
        blockers,
        assumptions_supported,
        assumptions_unknown_or_violated,
        causal_effect_estimated: false,
        maintenance_authorized: false,
        physical_restoration_authorized: false,
        operating_restriction_clearance_authorized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn protocol() -> TargetTrialProtocol {
        TargetTrialProtocol {
            schema_version: TARGET_TRIAL_SCHEMA.into(),
            trial_id: "synthetic-target-trial-v1".into(),
            target_population_id: "eligible-engine-population-v1".into(),
            eligibility_rule_sha256: digest('a'),
            assignment_rule_sha256: digest('b'),
            causal_graph_sha256: digest('c'),
            analysis_plan_sha256: digest('d'),
            cluster_group_key: "asset_id".into(),
            cluster_grouping_rule_sha256: digest('f'),
            cluster_robust_inference_declared: true,
            emulation_design: EmulationDesign::ParallelGroup,
            causal_contrast: CausalContrast::AssignmentEffect,
            align_eligibility_assignment_followup_at_time_zero: true,
            followup_horizon_seconds: 1_000.0,
            followup_exposure_unit: "engine_hours".into(),
            treatment_strategies: vec![
                TreatmentStrategySpec {
                    strategy_id: "physical-repair".into(),
                    strategy_version_sha256: digest('e'),
                    intervention_kind: InterventionKind::PhysicalComponentRepair,
                    strategy_definition_sha256: digest('f'),
                    grace_period_seconds: 0.0,
                },
                TreatmentStrategySpec {
                    strategy_id: "deferred-care".into(),
                    strategy_version_sha256: digest('1'),
                    intervention_kind: InterventionKind::DeferredOrUsualCare,
                    strategy_definition_sha256: digest('2'),
                    grace_period_seconds: 0.0,
                },
            ],
            baseline_covariates: vec![
                BaselineCovariateSpec {
                    covariate_id: "baseline-fault-severity".into(),
                    definition_sha256: digest('3'),
                    timing: CovariateTiming::BaselinePreTreatment,
                },
                BaselineCovariateSpec {
                    covariate_id: "baseline-load-regime".into(),
                    definition_sha256: digest('4'),
                    timing: CovariateTiming::BaselinePreTreatment,
                },
            ],
            primary_outcomes: vec![PrimaryOutcomeSpec {
                outcome_id: "propulsion-restoration".into(),
                metric_id: "normalized-propulsion-capability".into(),
                unit: "normalized_capability".into(),
                definition_sha256: digest('5'),
            }],
            thresholds: IdentificationThresholds {
                minimum_common_support_fraction: 0.8,
                minimum_arm_effective_sample_size: 20.0,
                maximum_absolute_standardized_mean_difference: 0.1,
                maximum_stabilized_weight: 10.0,
                minimum_followup_completeness_fraction: 0.9,
            },
        }
    }

    fn diagnostics() -> ObservationalEmulationDiagnostics {
        let mut timelines = Vec::new();
        for index in 0..40 {
            let strategy_id = if index < 20 {
                "physical-repair"
            } else {
                "deferred-care"
            };
            let treatment_start = if index < 20 { Some(100.0) } else { None };
            timelines.push(EmulationUnitTimeline {
                emulation_unit_id: format!("entry-{index}"),
                asset_id: format!("engine-{index}"),
                strategy_id: strategy_id.into(),
                eligible_at_seconds: 100.0,
                strategy_assignment_at_seconds: 100.0,
                followup_start_at_seconds: 100.0,
                treatment_start_at_seconds: treatment_start,
                followup_end_at_seconds: 1_100.0,
                followup_end_reason: FollowupEndReason::AdministrativeEnd,
                baseline_covariates: vec![
                    BaselineCovariateEvidence {
                        covariate_id: "baseline-fault-severity".into(),
                        available_at_seconds: 90.0,
                        evidence_sha256: digest('6'),
                    },
                    BaselineCovariateEvidence {
                        covariate_id: "baseline-load-regime".into(),
                        available_at_seconds: 95.0,
                        evidence_sha256: digest('7'),
                    },
                ],
            });
        }
        ObservationalEmulationDiagnostics {
            trial_id: "synthetic-target-trial-v1".into(),
            data_snapshot_sha256: digest('8'),
            eligible_emulation_entry_count: 40,
            independent_cluster_count: 40,
            strategy_arm_counts: vec![
                StrategyArmCount {
                    strategy_id: "physical-repair".into(),
                    emulation_entry_count: 20,
                },
                StrategyArmCount {
                    strategy_id: "deferred-care".into(),
                    emulation_entry_count: 20,
                },
            ],
            overlap: OverlapDiagnostics {
                common_support_fraction: 0.95,
                minimum_arm_effective_sample_size: 25.0,
                maximum_absolute_standardized_mean_difference: 0.05,
                maximum_stabilized_weight: 3.0,
            },
            followup_completeness_fraction: 0.96,
            assumption_assessments: vec![
                AssumptionAssessment {
                    domain: AssumptionDomain::Consistency,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('9'),
                    reviewer_ref: "review-consistency".into(),
                },
                AssumptionAssessment {
                    domain: AssumptionDomain::ConditionalExchangeability,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('a'),
                    reviewer_ref: "review-exchangeability".into(),
                },
                AssumptionAssessment {
                    domain: AssumptionDomain::Positivity,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('b'),
                    reviewer_ref: "review-positivity".into(),
                },
                AssumptionAssessment {
                    domain: AssumptionDomain::NoInterference,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('c'),
                    reviewer_ref: "review-interference".into(),
                },
                AssumptionAssessment {
                    domain: AssumptionDomain::CensoringExchangeability,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('d'),
                    reviewer_ref: "review-censoring".into(),
                },
                AssumptionAssessment {
                    domain: AssumptionDomain::MeasurementValidity,
                    status: AssessmentStatus::Supported,
                    evidence_sha256: digest('e'),
                    reviewer_ref: "review-measurement".into(),
                },
            ],
            negative_control_status: AssessmentStatus::Supported,
            sensitivity_analysis_status: AssessmentStatus::Supported,
            unit_timelines: timelines,
        }
    }

    #[test]
    fn fully_supported_target_trial_is_only_design_qualified_not_estimated() {
        let report = evaluate_observational_target_trial(&protocol(), &diagnostics());
        assert_eq!(
            report.disposition,
            TargetTrialDisposition::DesignQualifiedForEstimator
        );
        assert_eq!(report.qualified_emulation_entry_count, 40);
        assert!(!report.causal_effect_estimated);
        assert!(!report.maintenance_authorized);
    }

    #[test]
    fn misaligned_time_zero_returns_not_identified() {
        let mut data = diagnostics();
        data.unit_timelines[0].followup_start_at_seconds = 101.0;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("time zero")));
    }

    #[test]
    fn post_time_zero_covariate_is_rejected_as_baseline() {
        let mut data = diagnostics();
        data.unit_timelines[0].baseline_covariates[0].available_at_seconds = 105.0;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("after time zero")));
    }

    #[test]
    fn post_treatment_covariate_cannot_be_declared_as_baseline_adjustment() {
        let mut spec = protocol();
        spec.baseline_covariates[0].timing = CovariateTiming::PostTreatment;
        let report = evaluate_observational_target_trial(&spec, &diagnostics());
        assert_eq!(report.disposition, TargetTrialDisposition::ProtocolIncomplete);
        assert!(report.blockers.iter().any(|item| item.contains("not an admissible baseline")));
    }

    #[test]
    fn unknown_exchangeability_or_positivity_assumption_forces_not_identified() {
        let mut data = diagnostics();
        data.assumption_assessments[1].status = AssessmentStatus::Unknown;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert_eq!(report.assumptions_unknown_or_violated, 1);
    }

    #[test]
    fn poor_common_support_or_effective_sample_size_forces_not_identified() {
        let mut data = diagnostics();
        data.overlap.common_support_fraction = 0.4;
        data.overlap.minimum_arm_effective_sample_size = 8.0;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("common-support")));
        assert!(report.blockers.iter().any(|item| item.contains("effective sample size")));
    }

    #[test]
    fn grace_period_requires_clone_censor_weight_design() {
        let mut spec = protocol();
        spec.treatment_strategies[0].grace_period_seconds = 300.0;
        let report = evaluate_observational_target_trial(&spec, &diagnostics());
        assert_eq!(report.disposition, TargetTrialDisposition::ProtocolIncomplete);
        assert!(report.blockers.iter().any(|item| item.contains("clone-censor-weight")));
    }

    #[test]
    fn parallel_group_does_not_allow_one_asset_in_both_strategies() {
        let mut data = diagnostics();
        data.unit_timelines[20].asset_id = data.unit_timelines[0].asset_id.clone();
        data.independent_cluster_count = 39;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("parallel-group")));
    }

    #[test]
    fn administrative_followup_cannot_end_before_the_preregistered_horizon() {
        let mut data = diagnostics();
        data.unit_timelines[0].followup_end_at_seconds = 500.0;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("premature administrative")));
    }

    #[test]
    fn cluster_group_key_must_match_the_asset_identity_in_the_timeline() {
        let mut spec = protocol();
        spec.cluster_group_key = "telemetry_row_id".into();
        let report = evaluate_observational_target_trial(&spec, &diagnostics());
        assert_eq!(report.disposition, TargetTrialDisposition::ProtocolIncomplete);
        assert!(report.blockers.iter().any(|item| item.contains("cluster unit")));
    }

    #[test]
    fn cluster_robust_uncertainty_must_be_declared() {
        let mut spec = protocol();
        spec.cluster_robust_inference_declared = false;
        let report = evaluate_observational_target_trial(&spec, &diagnostics());
        assert_eq!(report.disposition, TargetTrialDisposition::ProtocolIncomplete);
        assert!(report.blockers.iter().any(|item| item.contains("cluster-robust")));
    }

    #[test]
    fn missing_or_invalid_negative_control_prevents_design_qualification() {
        let mut data = diagnostics();
        data.negative_control_status = AssessmentStatus::Unknown;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
    }

    #[test]
    fn missing_sensitivity_analysis_prevents_design_qualification() {
        let mut data = diagnostics();
        data.sensitivity_analysis_status = AssessmentStatus::Unknown;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("sensitivity analysis")));
    }

    #[test]
    fn deferred_strategy_must_not_record_a_physical_treatment_start() {
        let mut data = diagnostics();
        data.unit_timelines[20].treatment_start_at_seconds = Some(100.0);
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("deferred/usual-care")));
    }

    #[test]
    fn duplicate_strategy_arm_count_metadata_is_rejected() {
        let mut data = diagnostics();
        data.strategy_arm_counts.push(data.strategy_arm_counts[0].clone());
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("unknown or duplicated")));
    }

    #[test]
    fn followup_completeness_below_preregistered_threshold_prevents_qualification() {
        let mut data = diagnostics();
        data.followup_completeness_fraction = 0.6;
        let report = evaluate_observational_target_trial(&protocol(), &data);
        assert_eq!(report.disposition, TargetTrialDisposition::NotIdentified);
        assert!(report.blockers.iter().any(|item| item.contains("follow-up completeness")));
    }

    #[test]
    fn sequential_entries_may_reuse_asset_but_have_distinct_emulation_ids() {
        let mut spec = protocol();
        spec.emulation_design = EmulationDesign::SequentialTrial;
        let mut data = diagnostics();
        data.unit_timelines[1].asset_id = data.unit_timelines[0].asset_id.clone();
        data.unit_timelines[1].emulation_unit_id = "entry-1-sequential-reentry".into();
        data.independent_cluster_count = 39;
        let report = evaluate_observational_target_trial(&spec, &data);
        assert_eq!(
            report.disposition,
            TargetTrialDisposition::DesignQualifiedForEstimator
        );
    }

    #[test]
    fn schema_and_case_manifest_are_machine_readable() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../target-trial-protocol.schema.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../target-trial-cases-v0.1.json")).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 16);
    }
}
