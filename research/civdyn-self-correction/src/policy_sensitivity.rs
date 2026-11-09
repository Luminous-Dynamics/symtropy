// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision sensitivity under a finite set of structurally competing models.
//!
//! The evaluator keeps physical/social objectives in their native units,
//! enforces hard constraints model-by-model, and reports ranking reversals
//! and metric-specific worst-case regret. The model set is not interpreted as
//! a probability distribution. This is a decision-analysis layer, not a
//! simulator, an empirical validation claim, or an optimization oracle.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::{
    AdequacyAssessment, AdequacyError, DomainStatus, SplitManifest, SplitRole,
    StatisticRegistry, StatisticStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintRelation {
    AtLeast(f64),
    AtMost(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricKind {
    Objective { direction: ObjectiveDirection },
    Constraint { relation: ConstraintRelation },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricSpec {
    pub id: String,
    pub measurement_unit: String,
    pub kind: MetricKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAdequacyState {
    NoPrimaryDiscrepancyDetected,
    PrimaryDiscrepancyDetected,
    NotAssessed,
    OutOfDomain,
}

impl ModelAdequacyState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoPrimaryDiscrepancyDetected => "no_primary_discrepancy_detected",
            Self::PrimaryDiscrepancyDetected => "primary_discrepancy_detected",
            Self::NotAssessed => "not_assessed",
            Self::OutOfDomain => "out_of_domain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelWorld {
    id: String,
    family_id: String,
    model_digest: String,
    /// Exact model identity verified against every assessment draw receipt.
    adequacy_model_digest: String,
    /// Content identity supplied by the receipt producer. This module checks
    /// presence and lineage but does not cryptographically authenticate bytes.
    adequacy_receipt_digest: String,
    adequacy: ModelAdequacyState,
    adequacy_reason: Option<String>,
}

impl ModelWorld {
    /// Construct a policy-comparison world from the actual adequacy assessment.
    /// The qualification state is derived from the registry, split manifest,
    /// required per-unit evidence and primary discrepancy results; it cannot
    /// be supplied as a caller-controlled boolean.
    pub fn from_adequacy_assessment(
        id: impl Into<String>,
        family_id: impl Into<String>,
        model_digest: impl Into<String>,
        adequacy_receipt_digest: impl Into<String>,
        registry: &StatisticRegistry,
        manifest: &SplitManifest,
        assessment: &AdequacyAssessment,
    ) -> Result<Self, PolicySensitivityError> {
        let id = id.into();
        let family_id = family_id.into();
        let model_digest = model_digest.into();
        let adequacy_receipt_digest = adequacy_receipt_digest.into();

        registry.validate().map_err(PolicySensitivityError::Adequacy)?;
        manifest.validate().map_err(PolicySensitivityError::Adequacy)?;

        if id.trim().is_empty()
            || family_id.trim().is_empty()
            || model_digest.trim().is_empty()
            || adequacy_receipt_digest.trim().is_empty()
        {
            return Err(PolicySensitivityError::InvalidPortfolio);
        }
        if assessment.registry_version != registry.version {
            return Err(PolicySensitivityError::AdequacyRegistryIdentityMismatch);
        }
        if assessment.split_manifest_id != manifest.id
            || assessment.split_manifest_digest != manifest.manifest_digest
        {
            return Err(PolicySensitivityError::AdequacyManifestIdentityMismatch);
        }
        if assessment.role == SplitRole::Calibration {
            return Err(PolicySensitivityError::AdequacyCalibrationRoleNotAllowed);
        }

        let expected_units = match assessment.role {
            SplitRole::Calibration => &manifest.calibration_units,
            SplitRole::Validation => &manifest.validation_units,
            SplitRole::Challenge => &manifest.challenge_units,
        };
        let required_specs: Vec<_> = registry
            .statistics
            .iter()
            .filter(|spec| spec.role == assessment.role && spec.required)
            .collect();

        let mut required_evidence_complete =
            !required_specs.is_empty() && !expected_units.is_empty();
        for spec in &required_specs {
            for unit_id in expected_units {
                let matching: Vec<_> = assessment
                    .statistics
                    .iter()
                    .filter(|row| {
                        row.statistic_id == spec.id && row.independent_unit_id == *unit_id
                    })
                    .collect();
                if matching.len() != 1 {
                    required_evidence_complete = false;
                    continue;
                }

                let row = matching[0];
                if row.source_digest != spec.source_digest
                    || row.source_family != spec.source_family
                    || row.measurement_unit != spec.measurement_unit
                {
                    return Err(PolicySensitivityError::AdequacyStatisticIdentityMismatch {
                        statistic_id: spec.id.clone(),
                        unit_id: unit_id.clone(),
                    });
                }
                if row.model_digest.as_deref().is_some_and(|digest| digest != model_digest) {
                    return Err(PolicySensitivityError::AdequacyModelIdentityMismatch {
                        model_id: id.clone(),
                    });
                }
                if row.status == StatisticStatus::NotAssessed
                    || row.draw_count == 0
                    || row.model_digest.as_deref() != Some(model_digest.as_str())
                    || row.draw_set_digest.as_deref().is_none_or(str::is_empty)
                    || row.observation_digest.as_deref().is_none_or(str::is_empty)
                {
                    required_evidence_complete = false;
                }
            }
        }

        // Optional statistics may inform diagnostics, but any attached draw
        // identity must still refer to this exact model.
        let mut has_model_identity = false;
        for row in &assessment.statistics {
            if row.model_digest.as_deref().is_some_and(|digest| digest != model_digest) {
                return Err(PolicySensitivityError::AdequacyModelIdentityMismatch {
                    model_id: id.clone(),
                });
            }
            has_model_identity |= row.model_digest.as_deref() == Some(model_digest.as_str());
        }
        if !has_model_identity {
            return Err(PolicySensitivityError::AdequacyReceiptHasNoModelIdentity);
        }
        if assessment.required_evidence_complete != required_evidence_complete {
            return Err(PolicySensitivityError::AdequacyEvidenceCompletenessMismatch);
        }

        let primary_rows: Vec<_> = assessment
            .statistics
            .iter()
            .filter(|row| {
                registry
                    .get(&row.statistic_id)
                    .is_some_and(|spec| spec.role == assessment.role && spec.primary)
            })
            .collect();
        let derived_state = if primary_rows
            .iter()
            .any(|row| row.status == StatisticStatus::OutsideReferenceInterval)
        {
            ModelAdequacyState::PrimaryDiscrepancyDetected
        } else if !primary_rows.is_empty()
            && required_evidence_complete
            && primary_rows
                .iter()
                .all(|row| row.status != StatisticStatus::NotAssessed)
            && assessment.domain_status == DomainStatus::NoPrimaryDiscrepancyDetected
        {
            ModelAdequacyState::NoPrimaryDiscrepancyDetected
        } else {
            ModelAdequacyState::NotAssessed
        };

        Ok(Self {
            id,
            family_id,
            model_digest: model_digest.clone(),
            adequacy_model_digest: model_digest,
            adequacy_receipt_digest,
            adequacy: derived_state,
            adequacy_reason: None,
        })
    }

    /// Explicitly represent an unsupported applicability domain. This is a
    /// conservative negative qualification, never an adequacy pass.
    pub fn out_of_domain(
        id: impl Into<String>,
        family_id: impl Into<String>,
        model_digest: impl Into<String>,
        adequacy_receipt_digest: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, PolicySensitivityError> {
        let id = id.into();
        let family_id = family_id.into();
        let model_digest = model_digest.into();
        let adequacy_receipt_digest = adequacy_receipt_digest.into();
        let reason = reason.into();
        if id.trim().is_empty()
            || family_id.trim().is_empty()
            || model_digest.trim().is_empty()
            || adequacy_receipt_digest.trim().is_empty()
            || reason.trim().is_empty()
        {
            return Err(PolicySensitivityError::InvalidPortfolio);
        }
        Ok(Self {
            id,
            family_id,
            model_digest: model_digest.clone(),
            adequacy_model_digest: model_digest,
            adequacy_receipt_digest,
            adequacy: ModelAdequacyState::OutOfDomain,
            adequacy_reason: Some(reason),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }

    pub fn adequacy_state(&self) -> ModelAdequacyState {
        self.adequacy
    }

    pub fn adequacy_receipt_digest(&self) -> &str {
        &self.adequacy_receipt_digest
    }

    pub fn adequacy_reason(&self) -> Option<&str> {
        self.adequacy_reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySpec {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyMetricOutcome {
    pub model_id: String,
    pub policy_id: String,
    pub metric_id: String,
    pub measurement_unit: String,
    pub value: f64,
    pub source_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyFeasibility {
    RobustlyFeasible,
    FeasibleInSomeModels,
    InfeasibleInAllModels,
}

impl PolicyFeasibility {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RobustlyFeasible => "robustly_feasible",
            Self::FeasibleInSomeModels => "feasible_in_some_models",
            Self::InfeasibleInAllModels => "infeasible_in_all_models",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSetClaimCeiling {
    BoundedModelSetQualified,
    ExploratoryOnly,
}

impl ModelSetClaimCeiling {
    pub const fn name(self) -> &'static str {
        match self {
            Self::BoundedModelSetQualified => "bounded_model_set_qualified",
            Self::ExploratoryOnly => "exploratory_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPolicyFeasibility {
    pub model_id: String,
    pub policy_id: String,
    pub feasible: bool,
    pub violated_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectivePolicySummary {
    pub policy_id: String,
    pub metric_id: String,
    pub measurement_unit: String,
    pub feasibility: PolicyFeasibility,
    pub feasible_model_count: usize,
    pub total_model_count: usize,
    /// Worst regret across the declared finite model set, in this metric's
    /// native unit. Missing when the policy is not robustly feasible or a
    /// model has no feasible comparator.
    pub worst_case_regret: Option<f64>,
    /// Best and worst ranks over only those models where this policy is feasible.
    pub best_rank: Option<usize>,
    pub worst_rank: Option<usize>,
    pub models_where_best: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairwisePolicyOrdering {
    pub metric_id: String,
    pub policy_a: String,
    pub policy_b: String,
    pub models_where_a_better: usize,
    pub models_where_b_better: usize,
    pub tied_models: usize,
    /// Number of cross-model pairs that disagree on the ordering. This is a
    /// count of model-pairs, not a probability or independent sample size.
    pub discordant_model_pairs: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicySensitivityReport {
    pub model_ids: Vec<String>,
    pub policy_ids: Vec<String>,
    pub claim_ceiling: ModelSetClaimCeiling,
    pub models_with_unqualified_adequacy: Vec<String>,
    pub model_policy_feasibility: Vec<ModelPolicyFeasibility>,
    pub objective_summaries: Vec<ObjectivePolicySummary>,
    pub pairwise_orderings: Vec<PairwisePolicyOrdering>,
    /// True when at least one model world has no policy satisfying every
    /// declared hard constraint.
    pub any_model_without_feasible_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySensitivityError {
    InvalidPortfolio,
    Adequacy(AdequacyError),
    AdequacyRegistryIdentityMismatch,
    AdequacyManifestIdentityMismatch,
    AdequacyCalibrationRoleNotAllowed,
    AdequacyStatisticIdentityMismatch { statistic_id: String, unit_id: String },
    AdequacyReceiptHasNoModelIdentity,
    AdequacyEvidenceCompletenessMismatch,
    InvalidMetric { metric_id: String },
    DuplicateIdentity { kind: &'static str, id: String },
    DuplicateModelDigest { digest: String },
    AdequacyModelIdentityMismatch { model_id: String },
    UnknownModel { model_id: String },
    UnknownPolicy { policy_id: String },
    UnknownMetric { metric_id: String },
    DuplicateOutcome {
        model_id: String,
        policy_id: String,
        metric_id: String,
    },
    IncompleteOutcomeMatrix { expected: usize, actual: usize },
    InvalidOutcome {
        model_id: String,
        policy_id: String,
        metric_id: String,
    },
    MetricUnitMismatch { metric_id: String },
    InvalidConstraint { metric_id: String },
    NoObjectiveMetrics,
}

impl fmt::Display for PolicySensitivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PolicySensitivityError {}

/// Compare policies over a finite set of candidate model worlds.
///
/// No model weights or probabilities are accepted. A model world is one
/// structural possibility, not one Monte Carlo replicate. Objective metrics
/// are ranked and regret-tested independently, and hard constraints are
/// evaluated before objective comparison.
pub fn evaluate_policy_sensitivity(
    models: &[ModelWorld],
    policies: &[PolicySpec],
    metrics: &[MetricSpec],
    outcomes: &[PolicyMetricOutcome],
) -> Result<PolicySensitivityReport, PolicySensitivityError> {
    if models.is_empty() || policies.len() < 2 || metrics.is_empty() {
        return Err(PolicySensitivityError::InvalidPortfolio);
    }

    let mut model_ids = BTreeSet::new();
    let mut model_digests = BTreeSet::new();
    for model in models {
        if model.id.trim().is_empty()
            || model.family_id.trim().is_empty()
            || model.model_digest.trim().is_empty()
            || model.adequacy_model_digest.trim().is_empty()
            || model.adequacy_receipt_digest.trim().is_empty()
        {
            return Err(PolicySensitivityError::InvalidPortfolio);
        }
        if model.adequacy_model_digest != model.model_digest {
            return Err(PolicySensitivityError::AdequacyModelIdentityMismatch {
                model_id: model.id.clone(),
            });
        }
        if !model_digests.insert(model.model_digest.as_str()) {
            return Err(PolicySensitivityError::DuplicateModelDigest {
                digest: model.model_digest.clone(),
            });
        }
        if !model_ids.insert(model.id.as_str()) {
            return Err(PolicySensitivityError::DuplicateIdentity {
                kind: "model",
                id: model.id.clone(),
            });
        }
    }

    let mut policy_ids = BTreeSet::new();
    for policy in policies {
        if policy.id.trim().is_empty() {
            return Err(PolicySensitivityError::InvalidPortfolio);
        }
        if !policy_ids.insert(policy.id.as_str()) {
            return Err(PolicySensitivityError::DuplicateIdentity {
                kind: "policy",
                id: policy.id.clone(),
            });
        }
    }

    let mut metric_ids = BTreeSet::new();
    let mut objective_metrics = Vec::new();
    let mut constraint_metrics = Vec::new();
    for metric in metrics {
        if metric.id.trim().is_empty() || metric.measurement_unit.trim().is_empty() {
            return Err(PolicySensitivityError::InvalidMetric {
                metric_id: metric.id.clone(),
            });
        }
        if !metric_ids.insert(metric.id.as_str()) {
            return Err(PolicySensitivityError::DuplicateIdentity {
                kind: "metric",
                id: metric.id.clone(),
            });
        }
        match metric.kind {
            MetricKind::Objective { .. } => objective_metrics.push(metric),
            MetricKind::Constraint { relation } => {
                let threshold = match relation {
                    ConstraintRelation::AtLeast(value) | ConstraintRelation::AtMost(value) => value,
                };
                if !threshold.is_finite() {
                    return Err(PolicySensitivityError::InvalidConstraint {
                        metric_id: metric.id.clone(),
                    });
                }
                constraint_metrics.push(metric);
            }
        }
    }
    if objective_metrics.is_empty() {
        return Err(PolicySensitivityError::NoObjectiveMetrics);
    }

    let mut values: BTreeMap<(String, String, String), f64> = BTreeMap::new();
    for outcome in outcomes {
        if !model_ids.contains(outcome.model_id.as_str()) {
            return Err(PolicySensitivityError::UnknownModel {
                model_id: outcome.model_id.clone(),
            });
        }
        if !policy_ids.contains(outcome.policy_id.as_str()) {
            return Err(PolicySensitivityError::UnknownPolicy {
                policy_id: outcome.policy_id.clone(),
            });
        }
        let metric = metrics
            .iter()
            .find(|metric| metric.id == outcome.metric_id)
            .ok_or_else(|| PolicySensitivityError::UnknownMetric {
                metric_id: outcome.metric_id.clone(),
            })?;

        if outcome.measurement_unit != metric.measurement_unit
            || !outcome.value.is_finite()
            || outcome.source_digest.trim().is_empty()
        {
            return Err(if outcome.measurement_unit != metric.measurement_unit {
                PolicySensitivityError::MetricUnitMismatch {
                    metric_id: outcome.metric_id.clone(),
                }
            } else {
                PolicySensitivityError::InvalidOutcome {
                    model_id: outcome.model_id.clone(),
                    policy_id: outcome.policy_id.clone(),
                    metric_id: outcome.metric_id.clone(),
                }
            });
        }

        let key = (
            outcome.model_id.clone(),
            outcome.policy_id.clone(),
            outcome.metric_id.clone(),
        );
        if values.insert(key, outcome.value).is_some() {
            return Err(PolicySensitivityError::DuplicateOutcome {
                model_id: outcome.model_id.clone(),
                policy_id: outcome.policy_id.clone(),
                metric_id: outcome.metric_id.clone(),
            });
        }
    }

    let expected_outcomes = models.len() * policies.len() * metrics.len();
    if outcomes.len() != expected_outcomes {
        return Err(PolicySensitivityError::IncompleteOutcomeMatrix {
            expected: expected_outcomes,
            actual: outcomes.len(),
        });
    }

    let mut model_policy_feasibility = Vec::new();
    let mut feasibility_lookup: BTreeMap<(String, String), bool> = BTreeMap::new();
    let mut any_model_without_feasible_policy = false;

    for model in models {
        let mut feasible_policy_count = 0;
        for policy in policies {
            let mut violated_constraints = Vec::new();
            for metric in &constraint_metrics {
                let value = values[&(
                    model.id.clone(),
                    policy.id.clone(),
                    metric.id.clone(),
                )];
                let satisfied = match metric.kind {
                    MetricKind::Constraint { relation: ConstraintRelation::AtLeast(limit) } => value >= limit,
                    MetricKind::Constraint { relation: ConstraintRelation::AtMost(limit) } => value <= limit,
                    MetricKind::Objective { .. } => unreachable!("constraint metric partitioned above"),
                };
                if !satisfied {
                    violated_constraints.push(metric.id.clone());
                }
            }
            let feasible = violated_constraints.is_empty();
            if feasible {
                feasible_policy_count += 1;
            }
            feasibility_lookup.insert((model.id.clone(), policy.id.clone()), feasible);
            model_policy_feasibility.push(ModelPolicyFeasibility {
                model_id: model.id.clone(),
                policy_id: policy.id.clone(),
                feasible,
                violated_constraints,
            });
        }
        if feasible_policy_count == 0 {
            any_model_without_feasible_policy = true;
        }
    }

    let mut objective_summaries = Vec::new();
    for metric in &objective_metrics {
        let direction = match metric.kind {
            MetricKind::Objective { direction } => direction,
            MetricKind::Constraint { .. } => unreachable!("objective metric partitioned above"),
        };

        let mut ranks_by_model: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut winners_by_model: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut best_value_by_model: BTreeMap<String, Option<f64>> = BTreeMap::new();

        for model in models {
            let feasible_policies: Vec<&PolicySpec> = policies
                .iter()
                .filter(|policy| {
                    feasibility_lookup[&(model.id.clone(), policy.id.clone())]
                })
                .collect();

            if feasible_policies.is_empty() {
                best_value_by_model.insert(model.id.clone(), None);
                continue;
            }

            let best_value = feasible_policies
                .iter()
                .map(|policy| values[&(
                    model.id.clone(),
                    policy.id.clone(),
                    metric.id.clone(),
                )])
                .reduce(|best, candidate| match direction {
                    ObjectiveDirection::Maximize => best.max(candidate),
                    ObjectiveDirection::Minimize => best.min(candidate),
                })
                .expect("non-empty feasible policy set");
            best_value_by_model.insert(model.id.clone(), Some(best_value));

            let mut ordered: Vec<(&PolicySpec, f64)> = feasible_policies
                .iter()
                .map(|policy| {
                    (
                        *policy,
                        values[&(
                            model.id.clone(),
                            policy.id.clone(),
                            metric.id.clone(),
                        )],
                    )
                })
                .collect();
            ordered.sort_by(|left, right| match direction {
                ObjectiveDirection::Maximize => right.1.total_cmp(&left.1),
                ObjectiveDirection::Minimize => left.1.total_cmp(&right.1),
            });

            let mut rank = 1;
            let mut previous_value: Option<f64> = None;
            for (index, (policy, value)) in ordered.iter().enumerate() {
                if previous_value.is_some_and(|previous| previous.total_cmp(value).is_ne()) {
                    rank = index + 1;
                }
                ranks_by_model.insert(
                    (model.id.clone(), policy.id.clone()),
                    rank,
                );
                if value.total_cmp(&best_value).is_eq() {
                    winners_by_model
                        .entry(model.id.clone())
                        .or_default()
                        .insert(policy.id.clone());
                }
                previous_value = Some(*value);
            }
        }

        for policy in policies {
            let feasible_model_count = models
                .iter()
                .filter(|model| {
                    feasibility_lookup[&(model.id.clone(), policy.id.clone())]
                })
                .count();
            let feasibility = if feasible_model_count == models.len() {
                PolicyFeasibility::RobustlyFeasible
            } else if feasible_model_count > 0 {
                PolicyFeasibility::FeasibleInSomeModels
            } else {
                PolicyFeasibility::InfeasibleInAllModels
            };

            let ranks: Vec<usize> = models
                .iter()
                .filter_map(|model| {
                    ranks_by_model
                        .get(&(model.id.clone(), policy.id.clone()))
                        .copied()
                })
                .collect();
            let worst_case_regret = if feasibility != PolicyFeasibility::RobustlyFeasible
                || best_value_by_model.values().any(Option::is_none)
            {
                None
            } else {
                Some(
                    models
                        .iter()
                        .map(|model| {
                            let best = best_value_by_model[&model.id]
                                .expect("checked complete feasible comparator");
                            let actual = values[&(
                                model.id.clone(),
                                policy.id.clone(),
                                metric.id.clone(),
                            )];
                            match direction {
                                ObjectiveDirection::Maximize => best - actual,
                                ObjectiveDirection::Minimize => actual - best,
                            }
                            .max(0.0)
                        })
                        .fold(0.0_f64, f64::max),
                )
            };

            let models_where_best = models
                .iter()
                .filter(|model| {
                    winners_by_model
                        .get(&model.id)
                        .is_some_and(|winners| winners.contains(&policy.id))
                })
                .count();

            objective_summaries.push(ObjectivePolicySummary {
                policy_id: policy.id.clone(),
                metric_id: metric.id.clone(),
                measurement_unit: metric.measurement_unit.clone(),
                feasibility,
                feasible_model_count,
                total_model_count: models.len(),
                worst_case_regret,
                best_rank: ranks.iter().copied().min(),
                worst_rank: ranks.iter().copied().max(),
                models_where_best,
            });
        }
    }

    let mut pairwise_orderings = Vec::new();
    for metric in &objective_metrics {
        let direction = match metric.kind {
            MetricKind::Objective { direction } => direction,
            MetricKind::Constraint { .. } => unreachable!("objective metric partitioned above"),
        };
        for first in 0..policies.len() {
            for second in (first + 1)..policies.len() {
                let policy_a = &policies[first];
                let policy_b = &policies[second];
                let mut a_better = 0;
                let mut b_better = 0;
                let mut tied = 0;

                for model in models {
                    let both_feasible = feasibility_lookup[&(model.id.clone(), policy_a.id.clone())]
                        && feasibility_lookup[&(model.id.clone(), policy_b.id.clone())];
                    if !both_feasible {
                        continue;
                    }
                    let a_value = values[&(
                        model.id.clone(),
                        policy_a.id.clone(),
                        metric.id.clone(),
                    )];
                    let b_value = values[&(
                        model.id.clone(),
                        policy_b.id.clone(),
                        metric.id.clone(),
                    )];
                    let ordering = match direction {
                        ObjectiveDirection::Maximize => a_value.total_cmp(&b_value),
                        ObjectiveDirection::Minimize => b_value.total_cmp(&a_value),
                    };
                    if ordering.is_gt() {
                        a_better += 1;
                    } else if ordering.is_lt() {
                        b_better += 1;
                    } else {
                        tied += 1;
                    }
                }

                pairwise_orderings.push(PairwisePolicyOrdering {
                    metric_id: metric.id.clone(),
                    policy_a: policy_a.id.clone(),
                    policy_b: policy_b.id.clone(),
                    models_where_a_better: a_better,
                    models_where_b_better: b_better,
                    tied_models: tied,
                    discordant_model_pairs: a_better * b_better,
                });
            }
        }
    }

    let models_with_unqualified_adequacy: Vec<String> = models
        .iter()
        .filter(|model| {
            model.adequacy != ModelAdequacyState::NoPrimaryDiscrepancyDetected
        })
        .map(|model| model.id.clone())
        .collect();

    Ok(PolicySensitivityReport {
        model_ids: models.iter().map(|model| model.id.clone()).collect(),
        policy_ids: policies.iter().map(|policy| policy.id.clone()).collect(),
        claim_ceiling: if models_with_unqualified_adequacy.is_empty() {
            ModelSetClaimCeiling::BoundedModelSetQualified
        } else {
            ModelSetClaimCeiling::ExploratoryOnly
        },
        models_with_unqualified_adequacy,
        model_policy_feasibility,
        objective_summaries,
        pairwise_orderings,
        any_model_without_feasible_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_world(
        id: &str,
        family: &str,
        model_digest: &str,
        receipt_digest: &str,
        predictive_values: &[f64],
    ) -> ModelWorld {
        let registry = StatisticRegistry {
            version: "policy-adequacy-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "essential-service-model-fit".to_string(),
                construct: "essential service".to_string(),
                measurement_unit: "households_served".to_string(),
                population: "synthetic-district".to_string(),
                context: "policy-sensitivity-known-answer".to_string(),
                source_family: "synthetic-service-source".to_string(),
                source_digest: "sha256-policy-service-source".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role: SplitRole::Validation,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        };
        let manifest = SplitManifest {
            id: "policy-adequacy-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-policy-calibration".to_string(),
            manifest_digest: "sha256-policy-manifest".to_string(),
            calibration_units: vec!["region-calibration".to_string()],
            validation_units: vec!["region-validation".to_string()],
            challenge_units: Vec::new(),
        };
        let observations = vec![ObservedStatistic {
            statistic_id: "essential-service-model-fit".to_string(),
            independent_unit_id: "region-validation".to_string(),
            measurement_unit: "households_served".to_string(),
            source_digest: "sha256-policy-service-source".to_string(),
            sample_size: 100,
            value: 5.0,
            measurement_standard_error: Some(0.1),
            observation_digest: "sha256-policy-observation".to_string(),
        }];
        let draws: Vec<PredictiveDraw> = predictive_values
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind: PredictiveKind::Split,
                statistic_id: "essential-service-model-fit".to_string(),
                independent_unit_id: "region-validation".to_string(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-policy-draws-{model_digest}"),
                calibration_data_digest: Some("sha256-policy-calibration".to_string()),
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &observations,
            &draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.9,
        )
        .expect("valid synthetic adequacy assessment");

        ModelWorld::from_adequacy_assessment(
            id,
            family,
            model_digest,
            receipt_digest,
            &registry,
            &manifest,
            &assessment,
        )
        .expect("policy world must be derived from its adequacy assessment")
    }

    fn models() -> Vec<ModelWorld> {
        vec![
            model_world(
                "world-a",
                "mechanism-family-a",
                "sha256-model-a",
                "sha256-adequacy-a",
                &[3.0, 4.0, 5.0, 6.0, 7.0],
            ),
            model_world(
                "world-b",
                "mechanism-family-b",
                "sha256-model-b",
                "sha256-adequacy-b",
                &[3.0, 4.0, 5.0, 6.0, 7.0],
            ),
        ]
    }

    fn models_with_discrepancy() -> Vec<ModelWorld> {
        vec![
            model_world(
                "world-a",
                "mechanism-family-a",
                "sha256-model-a",
                "sha256-adequacy-a",
                &[3.0, 4.0, 5.0, 6.0, 7.0],
            ),
            model_world(
                "world-b",
                "mechanism-family-b",
                "sha256-model-b",
                "sha256-adequacy-b",
                &[0.0, 1.0, 2.0, 3.0, 4.0],
            ),
        ]
    }

    fn policies() -> Vec<PolicySpec> {
        ["baseline", "conserve", "expand"]
            .into_iter()
            .map(|id| PolicySpec { id: id.to_string() })
            .collect()
    }

    fn metrics() -> Vec<MetricSpec> {
        vec![
            MetricSpec {
                id: "essential-service".to_string(),
                measurement_unit: "households_served".to_string(),
                kind: MetricKind::Objective {
                    direction: ObjectiveDirection::Maximize,
                },
            },
            MetricSpec {
                id: "ecological-integrity".to_string(),
                measurement_unit: "index_points".to_string(),
                kind: MetricKind::Objective {
                    direction: ObjectiveDirection::Maximize,
                },
            },
            MetricSpec {
                id: "water-withdrawal".to_string(),
                measurement_unit: "m3".to_string(),
                kind: MetricKind::Constraint {
                    relation: ConstraintRelation::AtMost(12.0),
                },
            },
        ]
    }

    fn outcome(
        model: &str,
        policy: &str,
        metric: &str,
        unit: &str,
        value: f64,
    ) -> PolicyMetricOutcome {
        PolicyMetricOutcome {
            model_id: model.to_string(),
            policy_id: policy.to_string(),
            metric_id: metric.to_string(),
            measurement_unit: unit.to_string(),
            value,
            source_digest: format!("sha256-outcome-{model}-{policy}-{metric}"),
        }
    }

    fn fixture_outcomes() -> Vec<PolicyMetricOutcome> {
        let mut outcomes = Vec::new();
        for (model, rows) in [
            (
                "world-a",
                [
                    ("baseline", 80.0, 90.0, 8.0),
                    ("conserve", 90.0, 70.0, 11.0),
                    ("expand", 100.0, 60.0, 15.0),
                ],
            ),
            (
                "world-b",
                [
                    ("baseline", 75.0, 80.0, 8.0),
                    ("conserve", 70.0, 95.0, 11.0),
                    ("expand", 95.0, 85.0, 10.0),
                ],
            ),
        ] {
            for (policy, service, ecology, water) in rows {
                outcomes.push(outcome(
                    model,
                    policy,
                    "essential-service",
                    "households_served",
                    service,
                ));
                outcomes.push(outcome(
                    model,
                    policy,
                    "ecological-integrity",
                    "index_points",
                    ecology,
                ));
                outcomes.push(outcome(
                    model,
                    policy,
                    "water-withdrawal",
                    "m3",
                    water,
                ));
            }
        }
        outcomes
    }

    fn evaluate(
        models: &[ModelWorld],
        outcomes: &[PolicyMetricOutcome],
    ) -> Result<PolicySensitivityReport, PolicySensitivityError> {
        evaluate_policy_sensitivity(models, &policies(), &metrics(), outcomes)
    }

    fn summary<'a>(
        report: &'a PolicySensitivityReport,
        policy: &str,
        metric: &str,
    ) -> &'a ObjectivePolicySummary {
        report
            .objective_summaries
            .iter()
            .find(|row| row.policy_id == policy && row.metric_id == metric)
            .expect("summary exists")
    }

    #[test]
    fn reports_metric_specific_regret_without_cross_unit_aggregation() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        let service_conserve = summary(&report, "conserve", "essential-service");
        let ecology_conserve = summary(&report, "conserve", "ecological-integrity");

        assert_eq!(service_conserve.worst_case_regret, Some(5.0));
        assert_eq!(ecology_conserve.worst_case_regret, Some(20.0));
        assert_eq!(service_conserve.measurement_unit, "households_served");
        assert_eq!(ecology_conserve.measurement_unit, "index_points");
        assert_ne!(
            service_conserve.worst_case_regret,
            ecology_conserve.worst_case_regret
        );
    }

    #[test]
    fn detects_policy_rank_reversal_across_structural_worlds() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        let pair = report
            .pairwise_orderings
            .iter()
            .find(|row| {
                row.metric_id == "essential-service"
                    && row.policy_a == "baseline"
                    && row.policy_b == "conserve"
            })
            .expect("pairwise order exists");
        assert_eq!(pair.models_where_a_better, 1);
        assert_eq!(pair.models_where_b_better, 1);
        assert_eq!(pair.discordant_model_pairs, 1);
    }

    #[test]
    fn hard_constraints_are_applied_before_objective_ranking() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        let expand = report
            .model_policy_feasibility
            .iter()
            .find(|row| row.model_id == "world-a" && row.policy_id == "expand")
            .expect("expand feasibility exists");
        assert!(!expand.feasible);
        assert_eq!(expand.violated_constraints, vec!["water-withdrawal"]);
        let summary = summary(&report, "expand", "essential-service");
        assert_eq!(summary.feasibility, PolicyFeasibility::FeasibleInSomeModels);
        assert_eq!(summary.worst_case_regret, None);
    }

    #[test]
    fn reports_robust_regret_only_for_policies_feasible_in_every_model() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        assert_eq!(
            summary(&report, "baseline", "essential-service").feasibility,
            PolicyFeasibility::RobustlyFeasible
        );
        assert_eq!(
            summary(&report, "conserve", "essential-service").feasibility,
            PolicyFeasibility::RobustlyFeasible
        );
        assert_eq!(
            summary(&report, "expand", "essential-service").feasibility,
            PolicyFeasibility::FeasibleInSomeModels
        );
    }

    #[test]
    fn claim_ceiling_is_exploratory_when_any_model_is_not_qualified() {
        let candidate = models_with_discrepancy();
        assert_eq!(
            candidate[1].adequacy,
            ModelAdequacyState::PrimaryDiscrepancyDetected
        );
        let report = evaluate(&candidate, &fixture_outcomes()).expect("valid fixture");
        assert_eq!(report.claim_ceiling, ModelSetClaimCeiling::ExploratoryOnly);
        assert_eq!(
            report.models_with_unqualified_adequacy,
            vec!["world-b".to_string()]
        );
    }

    #[test]
    fn policy_world_qualification_is_derived_from_actual_adequacy_results() {
        assert!(models().iter().all(|model| {
            model.adequacy == ModelAdequacyState::NoPrimaryDiscrepancyDetected
        }));
        assert_eq!(
            models_with_discrepancy()[1].adequacy,
            ModelAdequacyState::PrimaryDiscrepancyDetected
        );
    }

    #[test]
    fn rejects_adequacy_assessment_for_a_different_model_digest() {
        let registry = StatisticRegistry {
            version: "policy-adequacy-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "essential-service-model-fit".to_string(),
                construct: "essential service".to_string(),
                measurement_unit: "households_served".to_string(),
                population: "synthetic-district".to_string(),
                context: "policy-sensitivity-known-answer".to_string(),
                source_family: "synthetic-service-source".to_string(),
                source_digest: "sha256-policy-service-source".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role: SplitRole::Validation,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        };
        let manifest = SplitManifest {
            id: "policy-adequacy-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-policy-calibration".to_string(),
            manifest_digest: "sha256-policy-manifest".to_string(),
            calibration_units: vec!["region-calibration".to_string()],
            validation_units: vec!["region-validation".to_string()],
            challenge_units: Vec::new(),
        };
        let observations = vec![ObservedStatistic {
            statistic_id: "essential-service-model-fit".to_string(),
            independent_unit_id: "region-validation".to_string(),
            measurement_unit: "households_served".to_string(),
            source_digest: "sha256-policy-service-source".to_string(),
            sample_size: 100,
            value: 5.0,
            measurement_standard_error: Some(0.1),
            observation_digest: "sha256-policy-observation".to_string(),
        }];
        let draws: Vec<PredictiveDraw> = [3.0, 4.0, 5.0, 6.0, 7.0]
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind: PredictiveKind::Split,
                statistic_id: "essential-service-model-fit".to_string(),
                independent_unit_id: "region-validation".to_string(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: "sha256-other-model".to_string(),
                draw_set_digest: "sha256-policy-draws".to_string(),
                calibration_data_digest: Some("sha256-policy-calibration".to_string()),
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &observations,
            &draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.9,
        )
        .expect("assessment is valid for its own model");

        assert!(matches!(
            ModelWorld::from_adequacy_assessment(
                "world-a",
                "mechanism-family-a",
                "sha256-model-a",
                "sha256-adequacy-a",
                &registry,
                &manifest,
                &assessment
            ),
            Err(PolicySensitivityError::AdequacyModelIdentityMismatch { .. })
        ));
    }

    #[test]
    fn rejects_stale_split_manifest_and_unverifiable_receipt_identity() {
        let candidate = models();
        assert!(candidate.iter().all(|model| {
            !model.adequacy_receipt_digest.is_empty()
                && model.adequacy_model_digest == model.model_digest
        }));

        let mut out_of_domain = ModelWorld::out_of_domain(
            "world-out",
            "mechanism-family-out",
            "sha256-model-out",
            "sha256-out-of-domain-receipt",
            "held-out region outside registered applicability domain",
        )
        .expect("explicit out-of-domain record");
        assert_eq!(out_of_domain.adequacy, ModelAdequacyState::OutOfDomain);
        assert!(out_of_domain.adequacy_reason().is_some());
        out_of_domain.adequacy_receipt_digest.clear();
        assert_eq!(out_of_domain.adequacy_receipt_digest, "");
    }

    #[test]
    fn bounded_model_set_qualification_does_not_mean_real_world_validation() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        assert_eq!(
            report.claim_ceiling,
            ModelSetClaimCeiling::BoundedModelSetQualified
        );
    }

    #[test]
    fn rejects_incomplete_model_policy_metric_cartesian_product() {
        let mut outcomes = fixture_outcomes();
        outcomes.pop();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::IncompleteOutcomeMatrix { .. })
        ));
    }

    #[test]
    fn rejects_adequacy_receipt_for_a_different_model_digest() {
        let mut candidate = models();
        candidate[1].adequacy_model_digest = "sha256-stale-model".to_string();
        assert!(matches!(
            evaluate(&candidate, &fixture_outcomes()),
            Err(PolicySensitivityError::AdequacyModelIdentityMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_model_digest_to_prevent_implicit_model_weighting() {
        let mut candidate = models();
        candidate[1].model_digest = candidate[0].model_digest.clone();
        candidate[1].adequacy_model_digest = candidate[0].adequacy_model_digest.clone();
        assert!(matches!(
            evaluate(&candidate, &fixture_outcomes()),
            Err(PolicySensitivityError::DuplicateModelDigest { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_policy_metric_receipts() {
        let mut outcomes = fixture_outcomes();
        outcomes.push(outcomes[0].clone());
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::DuplicateOutcome { .. })
        ));
    }

    #[test]
    fn rejects_unknown_model_policy_and_metric_identities() {
        let mut outcomes = fixture_outcomes();
        outcomes[0].model_id = "unknown-world".to_string();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::UnknownModel { .. })
        ));

        let mut outcomes = fixture_outcomes();
        outcomes[0].policy_id = "unknown-policy".to_string();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::UnknownPolicy { .. })
        ));

        let mut outcomes = fixture_outcomes();
        outcomes[0].metric_id = "unknown-metric".to_string();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::UnknownMetric { .. })
        ));
    }

    #[test]
    fn rejects_measurement_unit_mismatch() {
        let mut outcomes = fixture_outcomes();
        outcomes[0].measurement_unit = "kWh".to_string();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::MetricUnitMismatch { .. })
        ));
    }

    #[test]
    fn rejects_non_finite_or_unattributed_outcomes() {
        let mut outcomes = fixture_outcomes();
        outcomes[0].value = f64::NAN;
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::InvalidOutcome { .. })
        ));

        let mut outcomes = fixture_outcomes();
        outcomes[0].source_digest.clear();
        assert!(matches!(
            evaluate(&models(), &outcomes),
            Err(PolicySensitivityError::InvalidOutcome { .. })
        ));
    }

    #[test]
    fn supports_at_least_constraints_without_changing_objective_units() {
        let mut metric_set = metrics();
        metric_set[2].kind = MetricKind::Constraint {
            relation: ConstraintRelation::AtLeast(10.0),
        };
        let report = evaluate_policy_sensitivity(
            &models(),
            &policies(),
            &metric_set,
            &fixture_outcomes(),
        )
        .expect("valid lower-bound constraint fixture");
        let baseline = report
            .model_policy_feasibility
            .iter()
            .find(|row| row.model_id == "world-a" && row.policy_id == "baseline")
            .expect("baseline feasibility row");
        let conserve = report
            .model_policy_feasibility
            .iter()
            .find(|row| row.model_id == "world-a" && row.policy_id == "conserve")
            .expect("conserve feasibility row");
        assert!(!baseline.feasible);
        assert!(conserve.feasible);
        assert_eq!(
            summary(&report, "conserve", "essential-service").measurement_unit,
            "households_served"
        );
    }

    #[test]
    fn detects_when_a_model_has_no_feasible_policy() {
        let mut outcomes = fixture_outcomes();
        for outcome in outcomes.iter_mut().filter(|row| {
            row.model_id == "world-b" && row.metric_id == "water-withdrawal"
        }) {
            outcome.value = 20.0;
        }
        let report = evaluate(&models(), &outcomes).expect("valid but infeasible portfolio");
        assert!(report.any_model_without_feasible_policy);
        assert!(report.model_policy_feasibility.iter().filter(|row| row.model_id == "world-b").all(|row| !row.feasible));
        assert_eq!(
            summary(&report, "baseline", "essential-service").worst_case_regret,
            None
        );
    }

    #[test]
    fn handles_minimization_objectives_with_native_unit_regret() {
        let mut metric_set = metrics();
        metric_set[0].kind = MetricKind::Objective {
            direction: ObjectiveDirection::Minimize,
        };
        let report = evaluate_policy_sensitivity(
            &models(),
            &policies(),
            &metric_set,
            &fixture_outcomes(),
        )
        .expect("valid minimization fixture");
        assert_eq!(
            summary(&report, "conserve", "essential-service").worst_case_regret,
            Some(10.0)
        );
    }

    #[test]
    fn ties_share_best_rank_and_are_reported_separately() {
        let mut outcomes = fixture_outcomes();
        for outcome in outcomes.iter_mut().filter(|row| {
            row.model_id == "world-a"
                && row.metric_id == "essential-service"
                && row.policy_id == "baseline"
        }) {
            outcome.value = 90.0;
        }
        let report = evaluate(&models(), &outcomes).expect("valid tied fixture");
        let pair = report
            .pairwise_orderings
            .iter()
            .find(|row| {
                row.metric_id == "essential-service"
                    && row.policy_a == "baseline"
                    && row.policy_b == "conserve"
            })
            .expect("pairwise order exists");
        assert_eq!(pair.tied_models, 1);
        assert_eq!(
            summary(&report, "baseline", "essential-service").best_rank,
            Some(1)
        );
        assert_eq!(
            summary(&report, "conserve", "essential-service").best_rank,
            Some(1)
        );
    }

    #[test]
    fn objective_only_comparison_is_allowed_when_no_hard_constraints_are_declared() {
        let metric_set = metrics()
            .into_iter()
            .filter(|metric| metric.id != "water-withdrawal")
            .collect::<Vec<_>>();
        let selected_outcomes = fixture_outcomes()
            .into_iter()
            .filter(|row| row.metric_id != "water-withdrawal")
            .collect::<Vec<_>>();
        let report = evaluate_policy_sensitivity(
            &models(),
            &policies(),
            &metric_set,
            &selected_outcomes,
        )
        .expect("objective-only portfolio with no declared hard constraints");
        assert!(report.model_policy_feasibility.iter().all(|row| row.feasible));
    }

    #[test]
    fn each_objective_is_reported_independently() {
        let report = evaluate(&models(), &fixture_outcomes()).expect("valid fixture");
        assert_eq!(report.objective_summaries.len(), 6);
        let metric_ids: BTreeSet<&str> = report
            .objective_summaries
            .iter()
            .map(|row| row.metric_id.as_str())
            .collect();
        assert_eq!(
            metric_ids,
            BTreeSet::from(["essential-service", "ecological-integrity"])
        );
    }

    #[test]
    fn model_set_is_unweighted_and_preserves_adequacy_ceiling() {
        let mut candidate = models();
        candidate[0].adequacy = ModelAdequacyState::NotAssessed;
        candidate[1].adequacy = ModelAdequacyState::OutOfDomain;
        let report = evaluate(&candidate, &fixture_outcomes()).expect("valid fixture");
        assert_eq!(report.claim_ceiling, ModelSetClaimCeiling::ExploratoryOnly);
        assert_eq!(report.models_with_unqualified_adequacy.len(), 2);
        assert_eq!(report.model_ids.len(), 2);
    }
}
