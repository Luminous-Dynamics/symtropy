// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-bounded animal replay scorecards and deterministic result receipts.
//!
//! A receipt proves what was evaluated and which inputs were admissible. It does
//! not prove biological fidelity, causality, or cross-taxon validity by itself.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symtropy_animal_observation::{
    ClaimKind, GeneralizationTarget, ManifestAssessment, ManifestDisposition,
    ReferencePackManifest, SplitRecord, SplitRole, manifest_sha256, validate_manifest,
};
use symtropy_fauna_core::{
    AnimalProfile, EvidenceClass, ProfileKind, ValidationDisposition, ValidationResult,
    corpus_sha256, sha256_hex, validate_profile,
};

pub const REPLAY_RECEIPT_SCHEMA_VERSION: &str = "ANIMAL-REPLAY-RESULT-V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMode {
    /// Synthetic known-answer/oracle evaluation. Never empirical qualification.
    SyntheticOracle,
    /// Evaluation against an eligible reference pack and taxon-bound profile.
    EmpiricalReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayDisposition {
    /// The evidence boundary is admissible; this is not equivalent to model success.
    Eligible,
    /// Evidence is incomplete or conflicted, so it cannot support a qualified comparison.
    Indeterminate,
    /// A hard evidence, identity, schema, or protocol constraint was violated.
    Reject,
    /// Required result/baseline data are missing; no comparison is qualified.
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayReason {
    EmptyTaskId,
    EmptyTaskVersion,
    EmptyClaimCeiling,
    EmptyTargetTaxon,
    TaskClaimNotRequested,
    TaskGeneralizationMismatch,
    TargetTaxonNotBound,
    TargetTaxonAbsentFromTest,
    SyntheticEvidenceForEmpirical,
    SyntheticOracleRequiresSyntheticInputs,
    RealTaxonProfileRequired,
    ProfileTaxonMismatch,
    ProfileValidationIndeterminate,
    ProfileValidationRejected,
    ManifestValidationIndeterminate,
    ManifestValidationRejected,
    InvalidRunIdentity,
    InvalidCodeHead,
    InvalidConfigDigest,
    InvalidWorkingTreeDigest,
    MissingDeterministicSeed,
    ComparisonGroupMismatch,
    EmptyMetricId,
    DuplicateMetricId,
    EmptyMetricUnit,
    EmptyMetricStratum,
    InvalidOverallStratumId,
    TaxonStratumUnbound,
    InvalidMetricObservation,
    InvalidConfidenceInterval,
    InvalidThreshold,
    ThresholdMetricMismatch,
    MissingPrimaryMetric,
    IncompleteMetricObservation,
    NoBaseline,
    EmptyBaselineId,
    DuplicateBaselineId,
    InvalidBaselineMetadata,
    InvalidBaselineCodeHead,
    InvalidBaselineConfigDigest,
    BaselineTrainingScopeMismatch,
    BaselineComputeBudgetMismatch,
    BaselineComparisonGroupMismatch,
    BaselineMetricMismatch,
    BaselineUnitMismatch,
    BaselineStratumMismatch,
    BaselineMetricNotReported,
    DuplicateBaselineMetric,
    MissingBaselineStratumMetric,
    MissingBaselineComparison,
    DuplicateBaselineComparison,
    UnknownBaselineForComparison,
    InvalidComparisonMetadata,
    ComparisonGroupMismatchForBaseline,
    BaselineComparisonStratumMismatch,
    BaselineComparisonUnitMismatch,
    MissingPairedSamples,
    MissingPairedDifference,
    MissingPairedInterval,
    MissingPairedUnitManifestDigest,
    MalformedPairedUnitManifestDigest,
    InvalidPairedDifference,
    PairedDifferenceDoesNotMatchScores,
    InvalidPairedInterval,
    InvalidEffectMargins,
    InvalidRequiredConfidenceLevel,
    MissingUncertaintyMethod,
    UncertaintyProtocolMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdDirection {
    AtLeast,
    AtMost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdOutcome {
    MeetsThreshold,
    BelowThreshold,
    Uncertain,
    NotAssessed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricState {
    Reported,
    Missing,
    Abstained,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricStratumKind {
    Overall,
    Taxon,
    LifeStage,
    Condition,
    Study,
    Site,
    Occlusion,
    GeneralizationGroup,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Versioned estimator/resampling procedure that produced this interval.
    pub method_id: String,
    pub confidence_percent: u8,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricObservation {
    pub metric_id: String,
    /// Stable subgroup kind: overall, taxon, stage, study/site, condition, etc.
    pub stratum_kind: MetricStratumKind,
    /// Stable key within the declared pack; "all" is used by the overall row.
    pub stratum_id: String,
    pub unit: String,
    pub state: MetricState,
    pub value: Option<f64>,
    /// Number of independent analysis units used, not raw frames by default.
    pub sample_count: u64,
    pub missing_count: u64,
    pub abstained_count: u64,
    pub failure_reason: Option<String>,
    pub interval: Option<ConfidenceInterval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionThreshold {
    pub metric_id: String,
    pub direction: ThresholdDirection,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonBudgetPolicy {
    /// Candidate and baseline use identical declared data and compute budgets.
    MatchedBudget,
    /// The task is an explicit performance-cost frontier comparison; budgets may differ.
    DeclaredBudgetTradeoff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayTaskSpec {
    pub task_id: String,
    pub task_version: String,
    pub claim: ClaimKind,
    /// Present for a taxon-specific reference evaluation. Synthetic oracle IDs
    /// are explicitly marked test identifiers and cannot imply empirical claims.
    pub target_taxon_id: Option<String>,
    pub generalization: GeneralizationTarget,
    pub primary_metric_id: String,
    pub budget_policy: ComparisonBudgetPolicy,
    pub required_confidence_percent: u8,
    /// Versioned method used for metric-score uncertainty intervals.
    pub metric_uncertainty_method_id: String,
    /// Versioned method used for paired candidate-minus-baseline effect intervals.
    pub paired_comparison_method_id: String,
    /// Identifies the preregistered task/split/protocol comparison group.
    pub comparison_group: String,
    pub threshold: Option<DecisionThreshold>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdentity {
    pub run_id: String,
    /// Full Git commit ID (40 hex characters; 64 is accepted for SHA-256 Git).
    pub code_head: String,
    pub config_sha256: String,
    pub working_tree_clean: bool,
    pub working_tree_sha256: Option<String>,
    pub evaluator_version: String,
    pub toolchain: String,
    pub platform: String,
    pub comparison_group: String,
    pub training_data_scope: String,
    pub compute_budget: String,
    pub deterministic_replay: bool,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricObjective {
    HigherIsBetter,
    LowerIsBetter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineComparisonOutcome {
    OutperformsByMargin,
    NonInferior,
    RegressesBeyondMargin,
    Uncertain,
    NotAssessed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedBaselineComparison {
    pub baseline_id: String,
    pub metric_id: String,
    pub stratum_kind: MetricStratumKind,
    pub stratum_id: String,
    pub comparison_group: String,
    /// Digest of the exact canonical paired-unit membership manifest (e.g., animal/episode IDs).
    pub paired_unit_manifest_sha256: String,
    /// Paired metric difference is model minus baseline; objective controls sign interpretation.
    pub objective: MetricObjective,
    /// What counts as an independent paired unit (animal, recording, or study; not frames by default).
    pub pairing_unit: String,
    pub independent_unit_count: u64,
    pub mean_difference: Option<f64>,
    pub difference_interval: Option<ConfidenceInterval>,
    pub minimum_effect_size: f64,
    pub non_inferiority_margin: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineComparisonResult {
    pub baseline_id: String,
    pub metric_id: String,
    pub stratum_kind: MetricStratumKind,
    pub stratum_id: String,
    pub outcome: BaselineComparisonOutcome,
    /// Effect is oriented so positive values always favor the candidate model.
    pub normalized_mean_difference: Option<f64>,
    pub normalized_interval: Option<ConfidenceInterval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineReference {
    pub baseline_id: String,
    /// Human/model release label, retained alongside immutable execution identity.
    pub revision: String,
    pub run_id: String,
    pub code_head: String,
    pub config_sha256: String,
    pub evaluator_version: String,
    pub toolchain: String,
    pub platform: String,
    pub comparison_group: String,
    pub training_data_scope: String,
    pub compute_budget: String,
    /// Primary overall metric for this task.
    pub primary_metric: MetricObservation,
    /// Additional baseline score rows for matching taxon/stage/site/condition strata.
    pub additional_metrics: Vec<MetricObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayAssessment {
    pub disposition: ReplayDisposition,
    pub reason_codes: Vec<ReplayReason>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReceiptPayload {
    pub schema_version: String,
    pub mode: EvaluationMode,
    /// Makes the non-claim explicit for an otherwise valid synthetic receipt.
    pub qualification_scope: String,
    pub task: ReplayTaskSpec,
    pub target_taxon_ids: Vec<String>,
    pub profile_id: String,
    pub profile_sha256: String,
    pub profile_validation: ValidationResult,
    pub pack_id: String,
    pub manifest_sha256: String,
    pub split_sha256: String,
    pub manifest_validation: ManifestAssessment,
    pub run: RunIdentity,
    pub metrics: Vec<MetricObservation>,
    pub baselines: Vec<BaselineReference>,
    pub paired_baseline_comparisons: Vec<PairedBaselineComparison>,
    pub baseline_comparison_results: Vec<BaselineComparisonResult>,
    pub assessment: ReplayAssessment,
    pub threshold_outcome: ThresholdOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReceipt {
    pub payload: ReplayReceiptPayload,
    /// SHA-256 of canonical JSON serialization of payload (excluding this field).
    pub receipt_sha256: String,
}

#[derive(Debug)]
pub struct ReplayEvaluationInput<'a> {
    pub profile: &'a AnimalProfile,
    pub manifest: &'a ReferencePackManifest,
    pub mode: EvaluationMode,
    pub task: ReplayTaskSpec,
    pub run: RunIdentity,
    pub metrics: Vec<MetricObservation>,
    pub baselines: Vec<BaselineReference>,
    pub paired_baseline_comparisons: Vec<PairedBaselineComparison>,
}

/// Build an evidence-bounded scorecard receipt. Arrays with canonical ordering
/// are sorted before hashing; the source manifest is digested by its own
/// canonicalizer. This function never downloads data or grants source rights.
pub fn build_replay_receipt(
    input: ReplayEvaluationInput<'_>,
) -> Result<ReplayReceipt, serde_json::Error> {
    let ReplayEvaluationInput {
        profile,
        manifest,
        mode,
        task,
        run,
        mut metrics,
        mut baselines,
        mut paired_baseline_comparisons,
    } = input;
    let profile_validation = validate_profile(profile);
    let manifest_validation = validate_manifest(manifest);
    let mut rejected = BTreeSet::new();
    let mut indeterminate = BTreeSet::new();
    let mut incomplete = BTreeSet::new();

    if task.task_id.trim().is_empty() {
        rejected.insert(ReplayReason::EmptyTaskId);
    }
    if task.task_version.trim().is_empty() {
        rejected.insert(ReplayReason::EmptyTaskVersion);
    }
    if task.primary_metric_id.trim().is_empty() {
        rejected.insert(ReplayReason::MissingPrimaryMetric);
    }
    if task.required_confidence_percent == 0 || task.required_confidence_percent >= 100 {
        rejected.insert(ReplayReason::InvalidRequiredConfidenceLevel);
    }
    if task.metric_uncertainty_method_id.trim().is_empty()
        || task.paired_comparison_method_id.trim().is_empty()
    {
        rejected.insert(ReplayReason::MissingUncertaintyMethod);
    }
    if task.claim_ceiling.trim().is_empty() {
        rejected.insert(ReplayReason::EmptyClaimCeiling);
    }
    if task.comparison_group.trim().is_empty()
        || run.comparison_group.trim().is_empty()
        || task.comparison_group != run.comparison_group
    {
        rejected.insert(ReplayReason::ComparisonGroupMismatch);
    }
    if task.generalization != manifest.generalization {
        rejected.insert(ReplayReason::TaskGeneralizationMismatch);
    }
    if !manifest.requested_claims.contains(&task.claim) {
        rejected.insert(ReplayReason::TaskClaimNotRequested);
    }

    match mode {
        EvaluationMode::SyntheticOracle => {
            let records_are_synthetic = manifest
                .records
                .iter()
                .all(|record| record.provenance == EvidenceClass::Synthetic);
            if profile.kind != ProfileKind::SyntheticArchetype
                || profile.provenance != EvidenceClass::Synthetic
                || manifest.provenance != EvidenceClass::Synthetic
                || !records_are_synthetic
            {
                rejected.insert(ReplayReason::SyntheticOracleRequiresSyntheticInputs);
            }
        }
        EvaluationMode::EmpiricalReference => {
            let has_synthetic_evidence = profile.provenance == EvidenceClass::Synthetic
                || manifest.provenance == EvidenceClass::Synthetic
                || manifest
                    .records
                    .iter()
                    .any(|record| record.provenance == EvidenceClass::Synthetic);
            if has_synthetic_evidence {
                rejected.insert(ReplayReason::SyntheticEvidenceForEmpirical);
            } else if profile.kind != ProfileKind::RealTaxonBound {
                rejected.insert(ReplayReason::RealTaxonProfileRequired);
            }
            if task.target_taxon_id.is_none() {
                rejected.insert(ReplayReason::EmptyTargetTaxon);
            }
            if let (Some(target_taxon), Some(binding)) =
                (task.target_taxon_id.as_deref(), profile.taxon.as_ref())
                && binding.taxon_id != target_taxon
            {
                rejected.insert(ReplayReason::ProfileTaxonMismatch);
            }
        }
    }

    if let Some(target_taxon) = task.target_taxon_id.as_deref() {
        if target_taxon.trim().is_empty() {
            rejected.insert(ReplayReason::EmptyTargetTaxon);
        } else if !manifest
            .taxonomy
            .taxon_ids
            .iter()
            .any(|id| id == target_taxon)
        {
            rejected.insert(ReplayReason::TargetTaxonNotBound);
        } else if !manifest
            .records
            .iter()
            .any(|record| record.taxon_id == target_taxon && record.role == SplitRole::Test)
        {
            incomplete.insert(ReplayReason::TargetTaxonAbsentFromTest);
        }
    }

    match profile_validation.disposition {
        ValidationDisposition::Eligible => {}
        ValidationDisposition::Indeterminate => {
            indeterminate.insert(ReplayReason::ProfileValidationIndeterminate);
        }
        ValidationDisposition::Reject => {
            rejected.insert(ReplayReason::ProfileValidationRejected);
        }
    }
    match manifest_validation.disposition {
        ManifestDisposition::Eligible => {}
        ManifestDisposition::Indeterminate => {
            indeterminate.insert(ReplayReason::ManifestValidationIndeterminate);
        }
        ManifestDisposition::Reject => {
            rejected.insert(ReplayReason::ManifestValidationRejected);
        }
    }

    if run.run_id.trim().is_empty()
        || run.evaluator_version.trim().is_empty()
        || run.toolchain.trim().is_empty()
        || run.platform.trim().is_empty()
        || run.training_data_scope.trim().is_empty()
        || run.compute_budget.trim().is_empty()
    {
        rejected.insert(ReplayReason::InvalidRunIdentity);
    }
    if !is_lower_hex(&run.code_head, &[40, 64]) {
        rejected.insert(ReplayReason::InvalidCodeHead);
    }
    if !is_lower_hex(&run.config_sha256, &[64]) {
        rejected.insert(ReplayReason::InvalidConfigDigest);
    }
    if let Some(tree_digest) = run.working_tree_sha256.as_deref()
        && !is_lower_hex(tree_digest, &[64])
    {
        rejected.insert(ReplayReason::InvalidWorkingTreeDigest);
    }
    if !run.working_tree_clean
        && run
            .working_tree_sha256
            .as_deref()
            .is_none_or(|digest| !is_lower_hex(digest, &[64]))
    {
        rejected.insert(ReplayReason::InvalidWorkingTreeDigest);
    }
    if run.deterministic_replay && run.seed.is_none() {
        indeterminate.insert(ReplayReason::MissingDeterministicSeed);
    }

    metrics.sort_by(|left, right| {
        left.metric_id
            .cmp(&right.metric_id)
            .then_with(|| left.stratum_kind.cmp(&right.stratum_kind))
            .then_with(|| left.stratum_id.cmp(&right.stratum_id))
            .then_with(|| canonical_sort_key(left).cmp(&canonical_sort_key(right)))
    });
    let mut metric_keys = BTreeSet::new();
    for metric in &metrics {
        if metric.metric_id.trim().is_empty() {
            rejected.insert(ReplayReason::EmptyMetricId);
        }
        if metric.stratum_id.trim().is_empty() {
            rejected.insert(ReplayReason::EmptyMetricStratum);
        }
        if metric.stratum_kind == MetricStratumKind::Overall && metric.stratum_id != "all" {
            rejected.insert(ReplayReason::InvalidOverallStratumId);
        }
        if metric.stratum_kind == MetricStratumKind::Taxon
            && !manifest
                .taxonomy
                .taxon_ids
                .iter()
                .any(|id| id == &metric.stratum_id)
        {
            rejected.insert(ReplayReason::TaxonStratumUnbound);
        }
        if !metric_keys.insert((
            metric.metric_id.clone(),
            metric.stratum_kind,
            metric.stratum_id.clone(),
        )) {
            rejected.insert(ReplayReason::DuplicateMetricId);
        }
        if metric.unit.trim().is_empty() {
            rejected.insert(ReplayReason::EmptyMetricUnit);
        }
        match validate_metric(metric) {
            MetricValidation::Valid => {}
            MetricValidation::InvalidInterval => {
                rejected.insert(ReplayReason::InvalidConfidenceInterval);
            }
            MetricValidation::InvalidValue => {
                rejected.insert(ReplayReason::InvalidMetricObservation);
            }
            MetricValidation::Incomplete => {
                incomplete.insert(ReplayReason::IncompleteMetricObservation);
            }
        }
        if let Some(interval) = metric.interval.as_ref()
            && (interval.confidence_percent != task.required_confidence_percent
                || interval.method_id != task.metric_uncertainty_method_id)
        {
            rejected.insert(ReplayReason::UncertaintyProtocolMismatch);
        }
    }

    let primary_metric = metrics.iter().find(|metric| {
        metric.metric_id == task.primary_metric_id
            && metric.stratum_kind == MetricStratumKind::Overall
    });
    if primary_metric.is_none() {
        incomplete.insert(ReplayReason::MissingPrimaryMetric);
    }

    if let Some(threshold) = &task.threshold {
        if !threshold.value.is_finite() {
            rejected.insert(ReplayReason::InvalidThreshold);
        }
        if threshold.metric_id != task.primary_metric_id {
            rejected.insert(ReplayReason::ThresholdMetricMismatch);
        }
    }

    if baselines.is_empty() {
        incomplete.insert(ReplayReason::NoBaseline);
    }
    baselines.sort_by(|left, right| {
        left.baseline_id
            .cmp(&right.baseline_id)
            .then_with(|| canonical_sort_key(left).cmp(&canonical_sort_key(right)))
    });
    let mut baseline_ids = BTreeSet::new();
    for baseline in &mut baselines {
        baseline.additional_metrics.sort_by(|left, right| {
            left.metric_id
                .cmp(&right.metric_id)
                .then_with(|| left.stratum_kind.cmp(&right.stratum_kind))
                .then_with(|| left.stratum_id.cmp(&right.stratum_id))
                .then_with(|| canonical_sort_key(left).cmp(&canonical_sort_key(right)))
        });
        let mut baseline_metric_keys = BTreeSet::new();
        baseline_metric_keys.insert((
            baseline.primary_metric.metric_id.clone(),
            baseline.primary_metric.stratum_kind,
            baseline.primary_metric.stratum_id.clone(),
        ));
        for metric in &baseline.additional_metrics {
            let key = (
                metric.metric_id.clone(),
                metric.stratum_kind,
                metric.stratum_id.clone(),
            );
            if !baseline_metric_keys.insert(key) {
                rejected.insert(ReplayReason::DuplicateBaselineMetric);
            }
            if metric.stratum_id.trim().is_empty()
                || (metric.stratum_kind == MetricStratumKind::Overall && metric.stratum_id != "all")
            {
                rejected.insert(ReplayReason::BaselineStratumMismatch);
            }
            if metric.metric_id != task.primary_metric_id {
                rejected.insert(ReplayReason::BaselineMetricMismatch);
            }
            if metric.stratum_kind == MetricStratumKind::Taxon
                && !manifest
                    .taxonomy
                    .taxon_ids
                    .iter()
                    .any(|id| id == &metric.stratum_id)
            {
                rejected.insert(ReplayReason::TaxonStratumUnbound);
            }
            if matches!(
                validate_metric(metric),
                MetricValidation::InvalidInterval | MetricValidation::InvalidValue
            ) {
                rejected.insert(ReplayReason::BaselineMetricNotReported);
            } else if matches!(validate_metric(metric), MetricValidation::Incomplete) {
                incomplete.insert(ReplayReason::BaselineMetricNotReported);
            }
            if let Some(interval) = metric.interval.as_ref()
                && (interval.confidence_percent != task.required_confidence_percent
                    || interval.method_id != task.metric_uncertainty_method_id)
            {
                rejected.insert(ReplayReason::UncertaintyProtocolMismatch);
            }
        }
        if baseline.baseline_id.trim().is_empty() {
            rejected.insert(ReplayReason::EmptyBaselineId);
        }
        if !baseline_ids.insert(baseline.baseline_id.clone()) {
            rejected.insert(ReplayReason::DuplicateBaselineId);
        }
        if baseline.revision.trim().is_empty()
            || baseline.run_id.trim().is_empty()
            || baseline.evaluator_version.trim().is_empty()
            || baseline.toolchain.trim().is_empty()
            || baseline.platform.trim().is_empty()
            || baseline.training_data_scope.trim().is_empty()
            || baseline.compute_budget.trim().is_empty()
        {
            rejected.insert(ReplayReason::InvalidBaselineMetadata);
        }
        if !is_lower_hex(&baseline.code_head, &[40, 64]) {
            rejected.insert(ReplayReason::InvalidBaselineCodeHead);
        }
        if !is_lower_hex(&baseline.config_sha256, &[64]) {
            rejected.insert(ReplayReason::InvalidBaselineConfigDigest);
        }
        if task.budget_policy == ComparisonBudgetPolicy::MatchedBudget {
            if !baseline.training_data_scope.trim().is_empty()
                && baseline.training_data_scope != run.training_data_scope
            {
                rejected.insert(ReplayReason::BaselineTrainingScopeMismatch);
            }
            if !baseline.compute_budget.trim().is_empty()
                && baseline.compute_budget != run.compute_budget
            {
                rejected.insert(ReplayReason::BaselineComputeBudgetMismatch);
            }
        }
        if baseline.comparison_group != task.comparison_group {
            rejected.insert(ReplayReason::BaselineComparisonGroupMismatch);
        }
        if baseline.primary_metric.metric_id != task.primary_metric_id {
            rejected.insert(ReplayReason::BaselineMetricMismatch);
        }
        if let Some(primary) = primary_metric
            && (baseline.primary_metric.stratum_kind != primary.stratum_kind
                || baseline.primary_metric.stratum_id != primary.stratum_id)
        {
            rejected.insert(ReplayReason::BaselineStratumMismatch);
        }
        if let Some(primary) = primary_metric
            && baseline.primary_metric.unit != primary.unit
        {
            rejected.insert(ReplayReason::BaselineUnitMismatch);
        }
        match validate_metric(&baseline.primary_metric) {
            MetricValidation::Valid => {}
            MetricValidation::InvalidInterval | MetricValidation::InvalidValue => {
                rejected.insert(ReplayReason::BaselineMetricNotReported);
            }
            MetricValidation::Incomplete => {
                incomplete.insert(ReplayReason::BaselineMetricNotReported);
            }
        }
        if let Some(interval) = baseline.primary_metric.interval.as_ref()
            && (interval.confidence_percent != task.required_confidence_percent
                || interval.method_id != task.metric_uncertainty_method_id)
        {
            rejected.insert(ReplayReason::UncertaintyProtocolMismatch);
        }
    }

    // Baseline rows were normalized above; re-sort duplicates by their canonical content.
    baselines.sort_by(|left, right| {
        left.baseline_id
            .cmp(&right.baseline_id)
            .then_with(|| canonical_sort_key(left).cmp(&canonical_sort_key(right)))
    });

    paired_baseline_comparisons.sort_by(|left, right| {
        left.baseline_id
            .cmp(&right.baseline_id)
            .then_with(|| left.metric_id.cmp(&right.metric_id))
            .then_with(|| left.stratum_kind.cmp(&right.stratum_kind))
            .then_with(|| left.stratum_id.cmp(&right.stratum_id))
            .then_with(|| canonical_sort_key(left).cmp(&canonical_sort_key(right)))
    });
    let mut comparison_keys = BTreeSet::new();
    for comparison in &paired_baseline_comparisons {
        let key = (
            comparison.baseline_id.clone(),
            comparison.metric_id.clone(),
            comparison.stratum_kind,
            comparison.stratum_id.clone(),
        );
        if !comparison_keys.insert(key) {
            rejected.insert(ReplayReason::DuplicateBaselineComparison);
        }
        let Some(baseline) = baselines
            .iter()
            .find(|item| item.baseline_id == comparison.baseline_id)
        else {
            rejected.insert(ReplayReason::UnknownBaselineForComparison);
            continue;
        };
        if comparison.metric_id.trim().is_empty()
            || comparison.stratum_id.trim().is_empty()
            || comparison.pairing_unit.trim().is_empty()
            || comparison.comparison_group.trim().is_empty()
        {
            rejected.insert(ReplayReason::InvalidComparisonMetadata);
        }
        if comparison.comparison_group != task.comparison_group
            || comparison.comparison_group != baseline.comparison_group
        {
            rejected.insert(ReplayReason::ComparisonGroupMismatchForBaseline);
        }
        if !comparison.minimum_effect_size.is_finite()
            || comparison.minimum_effect_size < 0.0
            || !comparison.non_inferiority_margin.is_finite()
            || comparison.non_inferiority_margin < 0.0
        {
            rejected.insert(ReplayReason::InvalidEffectMargins);
        }
        if comparison.independent_unit_count == 0 {
            incomplete.insert(ReplayReason::MissingPairedSamples);
        }
        if comparison.paired_unit_manifest_sha256.trim().is_empty() {
            rejected.insert(ReplayReason::MissingPairedUnitManifestDigest);
        } else if !is_lower_hex(&comparison.paired_unit_manifest_sha256, &[64]) {
            rejected.insert(ReplayReason::MalformedPairedUnitManifestDigest);
        }
        let candidate_metric = metrics.iter().find(|metric| {
            metric.metric_id == comparison.metric_id
                && metric.stratum_kind == comparison.stratum_kind
                && metric.stratum_id == comparison.stratum_id
        });
        let Some(candidate_metric) = candidate_metric else {
            rejected.insert(ReplayReason::BaselineComparisonStratumMismatch);
            continue;
        };
        let baseline_metric = if comparison.stratum_kind == MetricStratumKind::Overall
            && comparison.stratum_id == "all"
        {
            Some(&baseline.primary_metric)
        } else {
            baseline.additional_metrics.iter().find(|metric| {
                metric.metric_id == comparison.metric_id
                    && metric.stratum_kind == comparison.stratum_kind
                    && metric.stratum_id == comparison.stratum_id
            })
        };
        let Some(baseline_metric) = baseline_metric else {
            rejected.insert(ReplayReason::MissingBaselineStratumMetric);
            continue;
        };
        if candidate_metric.unit != baseline_metric.unit {
            rejected.insert(ReplayReason::BaselineComparisonUnitMismatch);
        }
        if comparison.independent_unit_count > candidate_metric.sample_count
            || comparison.independent_unit_count > baseline_metric.sample_count
        {
            rejected.insert(ReplayReason::MissingPairedSamples);
        }
        match comparison.mean_difference {
            Some(value) if value.is_finite() => {
                if let (Some(candidate_value), Some(baseline_value)) =
                    (candidate_metric.value, baseline_metric.value)
                    && candidate_value.is_finite()
                    && baseline_value.is_finite()
                {
                    let observed_delta = candidate_value - baseline_value;
                    let tolerance = 1e-9 * value.abs().max(observed_delta.abs()).max(1.0);
                    if !observed_delta.is_finite() || (value - observed_delta).abs() > tolerance {
                        rejected.insert(ReplayReason::PairedDifferenceDoesNotMatchScores);
                    }
                }
            }
            Some(_) => {
                rejected.insert(ReplayReason::InvalidPairedDifference);
            }
            None => {
                incomplete.insert(ReplayReason::MissingPairedDifference);
            }
        }
        if let Some(interval) = comparison.difference_interval.as_ref()
            && (interval.confidence_percent != task.required_confidence_percent
                || (!task.paired_comparison_method_id.trim().is_empty()
                    && interval.method_id != task.paired_comparison_method_id))
        {
            rejected.insert(ReplayReason::UncertaintyProtocolMismatch);
        }
        match comparison.difference_interval.as_ref() {
            Some(interval)
                if interval.lower.is_finite()
                    && interval.upper.is_finite()
                    && interval.lower <= interval.upper
                    && comparison
                        .mean_difference
                        .is_none_or(|mean| mean >= interval.lower && mean <= interval.upper)
                    && interval.confidence_percent > 0
                    && interval.confidence_percent < 100 => {}
            Some(_) => {
                rejected.insert(ReplayReason::InvalidPairedInterval);
            }
            None => {
                incomplete.insert(ReplayReason::MissingPairedInterval);
            }
        }
    }

    for baseline in &baselines {
        for candidate_metric in metrics
            .iter()
            .filter(|metric| metric.metric_id == task.primary_metric_id)
        {
            let has_matching_comparison = paired_baseline_comparisons.iter().any(|comparison| {
                comparison.baseline_id == baseline.baseline_id
                    && comparison.metric_id == candidate_metric.metric_id
                    && comparison.stratum_kind == candidate_metric.stratum_kind
                    && comparison.stratum_id == candidate_metric.stratum_id
            });
            if !has_matching_comparison {
                incomplete.insert(ReplayReason::MissingBaselineComparison);
            }
        }
    }

    let disposition = if !rejected.is_empty() {
        ReplayDisposition::Reject
    } else if !indeterminate.is_empty() {
        ReplayDisposition::Indeterminate
    } else if !incomplete.is_empty() {
        ReplayDisposition::Incomplete
    } else {
        ReplayDisposition::Eligible
    };
    let mut reason_codes = BTreeSet::new();
    reason_codes.extend(rejected);
    reason_codes.extend(indeterminate);
    reason_codes.extend(incomplete);
    let assessment = ReplayAssessment {
        disposition,
        reason_codes: reason_codes.into_iter().collect(),
    };

    let baseline_comparison_results: Vec<BaselineComparisonResult> = paired_baseline_comparisons
        .iter()
        .map(|comparison| {
            let (outcome, normalized_mean_difference, normalized_interval) =
                assess_paired_comparison(comparison, disposition);
            BaselineComparisonResult {
                baseline_id: comparison.baseline_id.clone(),
                metric_id: comparison.metric_id.clone(),
                stratum_kind: comparison.stratum_kind,
                stratum_id: comparison.stratum_id.clone(),
                outcome,
                normalized_mean_difference,
                normalized_interval,
            }
        })
        .collect();

    let threshold_outcome = if disposition != ReplayDisposition::Eligible {
        if matches!(
            primary_metric.map(|metric| metric.state),
            Some(MetricState::Missing | MetricState::Abstained | MetricState::Failed)
        ) || primary_metric.is_none()
        {
            ThresholdOutcome::Incomplete
        } else {
            ThresholdOutcome::NotAssessed
        }
    } else if let (Some(threshold), Some(metric)) = (&task.threshold, primary_metric) {
        if metric.value.is_some() {
            match metric.interval.as_ref() {
                Some(interval) => match threshold.direction {
                    ThresholdDirection::AtLeast if interval.lower >= threshold.value => {
                        ThresholdOutcome::MeetsThreshold
                    }
                    ThresholdDirection::AtLeast if interval.upper < threshold.value => {
                        ThresholdOutcome::BelowThreshold
                    }
                    ThresholdDirection::AtMost if interval.upper <= threshold.value => {
                        ThresholdOutcome::MeetsThreshold
                    }
                    ThresholdDirection::AtMost if interval.lower > threshold.value => {
                        ThresholdOutcome::BelowThreshold
                    }
                    _ => ThresholdOutcome::Uncertain,
                },
                None => ThresholdOutcome::NotAssessed,
            }
        } else {
            ThresholdOutcome::Incomplete
        }
    } else {
        ThresholdOutcome::NotAssessed
    };

    let profile_sha256 = corpus_sha256(std::slice::from_ref(profile))?;
    let mut split_records: Vec<SplitRecord> = manifest.records.clone();
    split_records.sort();
    let split_bytes = serde_json::to_vec(&(
        manifest.generalization,
        manifest.minimum_temporal_gap,
        split_records,
    ))?;
    let split_sha256 = sha256_hex(&split_bytes);
    let canonical_manifest_sha256 = manifest_sha256(manifest)?;
    let mut target_taxon_ids = manifest.taxonomy.taxon_ids.clone();
    target_taxon_ids.sort();
    target_taxon_ids.dedup();

    let payload = ReplayReceiptPayload {
        schema_version: REPLAY_RECEIPT_SCHEMA_VERSION.to_owned(),
        mode,
        qualification_scope: match mode {
            EvaluationMode::SyntheticOracle => "synthetic_oracle_only".to_owned(),
            EvaluationMode::EmpiricalReference => "empirical_reference_evaluation".to_owned(),
        },
        task,
        target_taxon_ids,
        profile_id: profile.id.clone(),
        profile_sha256,
        profile_validation,
        pack_id: manifest.pack_id.clone(),
        manifest_sha256: canonical_manifest_sha256,
        split_sha256,
        manifest_validation,
        run,
        metrics,
        baselines,
        paired_baseline_comparisons,
        baseline_comparison_results,
        assessment,
        threshold_outcome,
    };
    let canonical_payload = serde_json::to_vec(&payload)?;
    let receipt_sha256 = sha256_hex(&canonical_payload);
    Ok(ReplayReceipt {
        payload,
        receipt_sha256,
    })
}

fn assess_paired_comparison(
    comparison: &PairedBaselineComparison,
    receipt_disposition: ReplayDisposition,
) -> (
    BaselineComparisonOutcome,
    Option<f64>,
    Option<ConfidenceInterval>,
) {
    if receipt_disposition != ReplayDisposition::Eligible {
        return (BaselineComparisonOutcome::NotAssessed, None, None);
    }
    let (Some(mean), Some(interval)) = (
        comparison.mean_difference,
        comparison.difference_interval.as_ref(),
    ) else {
        return (BaselineComparisonOutcome::Incomplete, None, None);
    };
    let (normalized_mean, lower, upper) = match comparison.objective {
        MetricObjective::HigherIsBetter => (mean, interval.lower, interval.upper),
        MetricObjective::LowerIsBetter => (-mean, -interval.upper, -interval.lower),
    };
    let outcome = if lower > comparison.minimum_effect_size {
        BaselineComparisonOutcome::OutperformsByMargin
    } else if upper < -comparison.non_inferiority_margin {
        BaselineComparisonOutcome::RegressesBeyondMargin
    } else if lower >= -comparison.non_inferiority_margin {
        BaselineComparisonOutcome::NonInferior
    } else {
        BaselineComparisonOutcome::Uncertain
    };
    (
        outcome,
        Some(normalized_mean),
        Some(ConfidenceInterval {
            method_id: interval.method_id.clone(),
            confidence_percent: interval.confidence_percent,
            lower,
            upper,
        }),
    )
}

enum MetricValidation {
    Valid,
    Incomplete,
    InvalidValue,
    InvalidInterval,
}

fn validate_metric(metric: &MetricObservation) -> MetricValidation {
    if metric.unit.trim().is_empty() {
        return MetricValidation::InvalidValue;
    }
    match metric.state {
        MetricState::Reported => {
            let Some(value) = metric.value else {
                return MetricValidation::Incomplete;
            };
            if !value.is_finite() || metric.sample_count == 0 || metric.failure_reason.is_some() {
                return MetricValidation::InvalidValue;
            }
            let Some(interval) = &metric.interval else {
                return MetricValidation::Incomplete;
            };
            if !interval.lower.is_finite()
                || !interval.upper.is_finite()
                || interval.lower > interval.upper
                || value < interval.lower
                || value > interval.upper
                || interval.method_id.trim().is_empty()
                || interval.confidence_percent == 0
                || interval.confidence_percent >= 100
            {
                return MetricValidation::InvalidInterval;
            }
            MetricValidation::Valid
        }
        MetricState::Missing => {
            if metric.value.is_some() || metric.interval.is_some() || metric.missing_count == 0 {
                MetricValidation::InvalidValue
            } else {
                MetricValidation::Incomplete
            }
        }
        MetricState::Abstained => {
            if metric.value.is_some() || metric.interval.is_some() || metric.abstained_count == 0 {
                MetricValidation::InvalidValue
            } else {
                MetricValidation::Incomplete
            }
        }
        MetricState::Failed => {
            if metric.value.is_some()
                || metric.interval.is_some()
                || metric
                    .failure_reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                MetricValidation::InvalidValue
            } else {
                MetricValidation::Incomplete
            }
        }
    }
}

fn canonical_sort_key<T: Serialize + std::fmt::Debug>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| format!("{value:?}").into_bytes())
}

fn is_lower_hex(value: &str, allowed_lengths: &[usize]) -> bool {
    allowed_lengths.contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercase_digest_validator_accepts_git_and_sha256_lengths() {
        assert!(is_lower_hex(&"a".repeat(40), &[40, 64]));
        assert!(is_lower_hex(&"b".repeat(64), &[64]));
        assert!(!is_lower_hex(&"G".repeat(40), &[40, 64]));
        assert!(!is_lower_hex("abc", &[40, 64]));
    }

    #[test]
    fn metric_validation_rejects_nan_and_invalid_intervals() {
        let mut metric = MetricObservation {
            metric_id: "pose_pck".to_owned(),
            unit: "fraction".to_owned(),
            stratum_kind: MetricStratumKind::Overall,
            stratum_id: "all".to_owned(),
            state: MetricState::Reported,
            value: Some(f64::NAN),
            sample_count: 10,
            missing_count: 0,
            abstained_count: 0,
            failure_reason: None,
            interval: None,
        };
        assert!(matches!(
            validate_metric(&metric),
            MetricValidation::InvalidValue
        ));
        metric.value = Some(0.8);
        metric.interval = Some(ConfidenceInterval {
            method_id: "synthetic-test-interval-v1".to_owned(),
            confidence_percent: 90,
            lower: 0.9,
            upper: 1.0,
        });
        assert!(matches!(
            validate_metric(&metric),
            MetricValidation::InvalidInterval
        ));
    }
}
