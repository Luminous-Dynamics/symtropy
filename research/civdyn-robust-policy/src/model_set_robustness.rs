// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision stability across structurally distinct, adequacy-qualified models.
//!
//! This is a model-set layer over the existing robust-policy evaluator. It
//! does not simulate the world, assign probabilities to candidate models, or
//! collapse unlike objectives into a universal score. Its conclusions are
//! bounded by the declared candidate model set and matched finite cases.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelQualification {
    /// No primary discrepancy was detected by the declared finite adequacy
    /// protocol. This is eligibility for a bounded comparison, not proof that
    /// the model is true.
    NoPrimaryDiscrepancyDetected,
    PrimaryDiscrepancyDetected,
    NotAssessed,
    OutOfDomain,
}

impl ModelQualification {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoPrimaryDiscrepancyDetected => "no_primary_discrepancy_detected",
            Self::PrimaryDiscrepancyDetected => "primary_discrepancy_detected",
            Self::NotAssessed => "not_assessed",
            Self::OutOfDomain => "out_of_domain",
        }
    }

    const fn is_eligible(self) -> bool {
        matches!(self, Self::NoPrimaryDiscrepancyDetected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateModel {
    pub model_id: String,
    pub model_family_id: String,
    pub model_digest: String,
    pub adequacy_receipt_digest: String,
    pub qualification: ModelQualification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionCase {
    pub case_id: String,
    pub case_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionPolicy {
    pub policy_id: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

impl ObjectiveDirection {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstraintComparison {
    AtLeast,
    AtMost,
}

impl ConstraintComparison {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AtLeast => "at_least",
            Self::AtMost => "at_most",
        }
    }

    fn passes(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::AtLeast => value >= threshold,
            Self::AtMost => value <= threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecisionMetricRole {
    Objective {
        direction: ObjectiveDirection,
    },
    HardConstraint {
        comparison: ConstraintComparison,
        threshold: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionMetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub role: DecisionMetricRole,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelSetManifest {
    pub model_set_id: String,
    pub manifest_digest: String,
    pub adequacy_registry_digest: String,
    pub case_manifest_digest: String,
    pub policy_manifest_digest: String,
    pub metric_registry_digest: String,
    pub models: Vec<CandidateModel>,
    pub cases: Vec<DecisionCase>,
    pub policies: Vec<DecisionPolicy>,
    pub metrics: Vec<DecisionMetricSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionMetricValue {
    pub metric_id: String,
    pub unit: String,
    /// None means unavailable; it is never coerced to zero.
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPolicyOutcome {
    pub model_id: String,
    pub model_digest: String,
    pub case_id: String,
    pub case_digest: String,
    pub policy_id: String,
    pub policy_digest: String,
    pub outcome_digest: String,
    pub values: Vec<DecisionMetricValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeasibilityStatus {
    Pass,
    Fail,
    Unknown,
}

impl FeasibilityStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseComparisonStatus {
    Assessed,
    NoFeasiblePolicy,
    NotAssessed,
}

impl CaseComparisonStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Assessed => "assessed",
            Self::NoFeasiblePolicy => "no_feasible_policy",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSetCoverage {
    AllDeclaredModelsQualified,
    PartialQualifiedModelSet,
    NoQualifiedModels,
}

impl ModelSetCoverage {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AllDeclaredModelsQualified => "all_declared_models_qualified",
            Self::PartialQualifiedModelSet => "partial_qualified_model_set",
            Self::NoQualifiedModels => "no_qualified_models",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelQualificationReceipt {
    pub model_id: String,
    pub model_family_id: String,
    pub qualification: ModelQualification,
    pub eligible_for_comparison: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyCaseFeasibility {
    pub model_id: String,
    pub case_id: String,
    pub policy_id: String,
    pub status: FeasibilityStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyRobustFeasibility {
    pub policy_id: String,
    pub expected_model_case_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub unknown_count: usize,
    /// False if any qualified model/case has a known violation; None if no
    /// violation is known but at least one comparison is incomplete.
    pub feasible_across_qualified_models: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseObjectiveAssessment {
    pub model_id: String,
    pub case_id: String,
    pub objective_id: String,
    pub status: CaseComparisonStatus,
    pub feasible_policy_ids: Vec<String>,
    pub best_policy_ids: Vec<String>,
    pub best_value: Option<f64>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveStabilitySummary {
    pub objective_id: String,
    pub unit: String,
    pub expected_model_case_count: usize,
    pub assessed_model_case_count: usize,
    pub no_feasible_policy_count: usize,
    pub not_assessed_count: usize,
    pub distinct_winner_policy_count: usize,
    /// Non-empty only if at least one policy is a best policy in every
    /// assessed model/case. This is an intersection, not a weighted vote.
    pub best_policy_intersection: Vec<String>,
    pub best_policy_stable_across_all_model_cases: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyObjectiveRegret {
    pub policy_id: String,
    pub objective_id: String,
    pub unit: String,
    pub feasible_across_qualified_models: Option<bool>,
    pub expected_model_case_count: usize,
    pub assessed_model_case_count: usize,
    pub unknown_model_case_count: usize,
    /// Set only for a robust-feasible policy with complete comparisons across
    /// every qualified model/case.
    pub mean_regret: Option<f64>,
    pub worst_case_regret: Option<f64>,
    pub worst_case_model_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinimaxRegretSummary {
    pub objective_id: String,
    pub unit: String,
    pub minimax_policy_ids: Vec<String>,
    pub minimum_worst_case_regret: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelSetDecisionReport {
    pub model_set_id: String,
    pub manifest_digest: String,
    pub adequacy_registry_digest: String,
    pub case_manifest_digest: String,
    pub policy_manifest_digest: String,
    pub metric_registry_digest: String,
    pub coverage: ModelSetCoverage,
    pub declared_model_count: usize,
    pub qualified_model_count: usize,
    pub excluded_model_count: usize,
    pub distinct_qualified_model_family_count: usize,
    pub qualification_receipts: Vec<ModelQualificationReceipt>,
    pub policy_case_feasibility: Vec<PolicyCaseFeasibility>,
    pub policy_robust_feasibility: Vec<PolicyRobustFeasibility>,
    pub case_objectives: Vec<CaseObjectiveAssessment>,
    pub objective_stability: Vec<ObjectiveStabilitySummary>,
    pub policy_objective_regret: Vec<PolicyObjectiveRegret>,
    pub minimax_regret: Vec<MinimaxRegretSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelSetError {
    InvalidManifest,
    DuplicateModel {
        model_id: String,
    },
    DuplicateCase {
        case_id: String,
    },
    DuplicatePolicy {
        policy_id: String,
    },
    DuplicateMetric {
        metric_id: String,
    },
    MissingObjective,
    MissingHardConstraint,
    InvalidMetric {
        metric_id: String,
    },
    InvalidConstraintThreshold {
        metric_id: String,
        threshold: f64,
    },
    UnknownModel {
        model_id: String,
    },
    UnknownCase {
        case_id: String,
    },
    UnknownPolicy {
        policy_id: String,
    },
    DuplicateOutcome {
        model_id: String,
        case_id: String,
        policy_id: String,
    },
    ModelDigestMismatch {
        model_id: String,
    },
    CaseDigestMismatch {
        case_id: String,
    },
    PolicyDigestMismatch {
        policy_id: String,
    },
    EmptyOutcomeDigest,
    UnknownMetric {
        metric_id: String,
    },
    DuplicateMetricValue {
        metric_id: String,
    },
    MetricUnitMismatch {
        metric_id: String,
    },
    NonFiniteMetricValue {
        metric_id: String,
        value: f64,
    },
}

impl fmt::Display for ModelSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ModelSetError {}

impl ModelSetManifest {
    pub fn validate(&self) -> Result<(), ModelSetError> {
        if self.model_set_id.trim().is_empty()
            || self.manifest_digest.trim().is_empty()
            || self.adequacy_registry_digest.trim().is_empty()
            || self.case_manifest_digest.trim().is_empty()
            || self.policy_manifest_digest.trim().is_empty()
            || self.metric_registry_digest.trim().is_empty()
            || self.models.is_empty()
            || self.cases.is_empty()
            || self.policies.is_empty()
            || self.metrics.is_empty()
        {
            return Err(ModelSetError::InvalidManifest);
        }

        let mut model_ids = BTreeSet::new();
        for model in &self.models {
            if model.model_id.trim().is_empty()
                || model.model_family_id.trim().is_empty()
                || model.model_digest.trim().is_empty()
                || model.adequacy_receipt_digest.trim().is_empty()
            {
                return Err(ModelSetError::InvalidManifest);
            }
            if !model_ids.insert(model.model_id.as_str()) {
                return Err(ModelSetError::DuplicateModel {
                    model_id: model.model_id.clone(),
                });
            }
        }

        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if case.case_id.trim().is_empty() || case.case_digest.trim().is_empty() {
                return Err(ModelSetError::InvalidManifest);
            }
            if !case_ids.insert(case.case_id.as_str()) {
                return Err(ModelSetError::DuplicateCase {
                    case_id: case.case_id.clone(),
                });
            }
        }

        let mut policy_ids = BTreeSet::new();
        for policy in &self.policies {
            if policy.policy_id.trim().is_empty() || policy.policy_digest.trim().is_empty() {
                return Err(ModelSetError::InvalidManifest);
            }
            if !policy_ids.insert(policy.policy_id.as_str()) {
                return Err(ModelSetError::DuplicatePolicy {
                    policy_id: policy.policy_id.clone(),
                });
            }
        }

        let mut metric_ids = BTreeSet::new();
        let mut objective_count = 0usize;
        let mut constraint_count = 0usize;
        for metric in &self.metrics {
            if metric.metric_id.trim().is_empty() || metric.unit.trim().is_empty() {
                return Err(ModelSetError::InvalidMetric {
                    metric_id: metric.metric_id.clone(),
                });
            }
            if !metric_ids.insert(metric.metric_id.as_str()) {
                return Err(ModelSetError::DuplicateMetric {
                    metric_id: metric.metric_id.clone(),
                });
            }
            match metric.role {
                DecisionMetricRole::Objective { .. } => objective_count += 1,
                DecisionMetricRole::HardConstraint {
                    threshold,
                    comparison: _,
                } => {
                    constraint_count += 1;
                    if !threshold.is_finite() {
                        return Err(ModelSetError::InvalidConstraintThreshold {
                            metric_id: metric.metric_id.clone(),
                            threshold,
                        });
                    }
                }
            }
        }
        if objective_count == 0 {
            return Err(ModelSetError::MissingObjective);
        }
        if constraint_count == 0 {
            return Err(ModelSetError::MissingHardConstraint);
        }
        Ok(())
    }
}

/// Evaluate matched policy outcomes across the subset of candidate models
/// whose declared adequacy status has no detected primary discrepancy.
///
/// Excluded models remain in the report. No model weights are inferred, and
/// no result is claimed beyond the declared candidate set and matched cases.
pub fn analyze_model_set_decisions(
    manifest: &ModelSetManifest,
    outcomes: &[ModelPolicyOutcome],
) -> Result<ModelSetDecisionReport, ModelSetError> {
    manifest.validate()?;

    let models: BTreeMap<&str, &CandidateModel> = manifest
        .models
        .iter()
        .map(|model| (model.model_id.as_str(), model))
        .collect();
    let cases: BTreeMap<&str, &DecisionCase> = manifest
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    let policies: BTreeMap<&str, &DecisionPolicy> = manifest
        .policies
        .iter()
        .map(|policy| (policy.policy_id.as_str(), policy))
        .collect();
    let metrics: BTreeMap<&str, &DecisionMetricSpec> = manifest
        .metrics
        .iter()
        .map(|metric| (metric.metric_id.as_str(), metric))
        .collect();

    let mut rows = BTreeMap::<(String, String, String), &ModelPolicyOutcome>::new();
    for outcome in outcomes {
        let model =
            models
                .get(outcome.model_id.as_str())
                .ok_or_else(|| ModelSetError::UnknownModel {
                    model_id: outcome.model_id.clone(),
                })?;
        let case =
            cases
                .get(outcome.case_id.as_str())
                .ok_or_else(|| ModelSetError::UnknownCase {
                    case_id: outcome.case_id.clone(),
                })?;
        let policy = policies.get(outcome.policy_id.as_str()).ok_or_else(|| {
            ModelSetError::UnknownPolicy {
                policy_id: outcome.policy_id.clone(),
            }
        })?;

        if outcome.model_digest != model.model_digest {
            return Err(ModelSetError::ModelDigestMismatch {
                model_id: outcome.model_id.clone(),
            });
        }
        if outcome.case_digest != case.case_digest {
            return Err(ModelSetError::CaseDigestMismatch {
                case_id: outcome.case_id.clone(),
            });
        }
        if outcome.policy_digest != policy.policy_digest {
            return Err(ModelSetError::PolicyDigestMismatch {
                policy_id: outcome.policy_id.clone(),
            });
        }
        if outcome.outcome_digest.trim().is_empty() {
            return Err(ModelSetError::EmptyOutcomeDigest);
        }

        let mut seen_metrics = BTreeSet::new();
        for value in &outcome.values {
            let spec = metrics.get(value.metric_id.as_str()).ok_or_else(|| {
                ModelSetError::UnknownMetric {
                    metric_id: value.metric_id.clone(),
                }
            })?;
            if !seen_metrics.insert(value.metric_id.as_str()) {
                return Err(ModelSetError::DuplicateMetricValue {
                    metric_id: value.metric_id.clone(),
                });
            }
            if value.unit != spec.unit {
                return Err(ModelSetError::MetricUnitMismatch {
                    metric_id: value.metric_id.clone(),
                });
            }
            if let Some(number) = value.value {
                if !number.is_finite() {
                    return Err(ModelSetError::NonFiniteMetricValue {
                        metric_id: value.metric_id.clone(),
                        value: number,
                    });
                }
            }
        }

        let key = (
            outcome.model_id.clone(),
            outcome.case_id.clone(),
            outcome.policy_id.clone(),
        );
        if rows.insert(key, outcome).is_some() {
            return Err(ModelSetError::DuplicateOutcome {
                model_id: outcome.model_id.clone(),
                case_id: outcome.case_id.clone(),
                policy_id: outcome.policy_id.clone(),
            });
        }
    }

    let qualification_receipts = manifest
        .models
        .iter()
        .map(|model| ModelQualificationReceipt {
            model_id: model.model_id.clone(),
            model_family_id: model.model_family_id.clone(),
            qualification: model.qualification,
            eligible_for_comparison: model.qualification.is_eligible(),
        })
        .collect::<Vec<_>>();

    let eligible_models = manifest
        .models
        .iter()
        .filter(|model| model.qualification.is_eligible())
        .collect::<Vec<_>>();
    let excluded_model_count = manifest.models.len() - eligible_models.len();
    let coverage = if eligible_models.is_empty() {
        ModelSetCoverage::NoQualifiedModels
    } else if excluded_model_count > 0 {
        ModelSetCoverage::PartialQualifiedModelSet
    } else {
        ModelSetCoverage::AllDeclaredModelsQualified
    };

    let distinct_families = eligible_models
        .iter()
        .map(|model| model.model_family_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    let mut policy_case_feasibility = Vec::new();
    for model in &eligible_models {
        for case in &manifest.cases {
            for policy in &manifest.policies {
                let outcome = rows.get(&(
                    model.model_id.clone(),
                    case.case_id.clone(),
                    policy.policy_id.clone(),
                ));
                policy_case_feasibility.push(PolicyCaseFeasibility {
                    model_id: model.model_id.clone(),
                    case_id: case.case_id.clone(),
                    policy_id: policy.policy_id.clone(),
                    status: constraint_status(outcome.copied(), &manifest.metrics),
                });
            }
        }
    }

    let mut policy_robust_feasibility = Vec::new();
    for policy in &manifest.policies {
        let statuses = policy_case_feasibility
            .iter()
            .filter(|row| row.policy_id == policy.policy_id)
            .map(|row| row.status)
            .collect::<Vec<_>>();
        let pass_count = statuses
            .iter()
            .filter(|status| **status == FeasibilityStatus::Pass)
            .count();
        let fail_count = statuses
            .iter()
            .filter(|status| **status == FeasibilityStatus::Fail)
            .count();
        let unknown_count = statuses
            .iter()
            .filter(|status| **status == FeasibilityStatus::Unknown)
            .count();
        let feasible = if fail_count > 0 {
            Some(false)
        } else if unknown_count > 0 || statuses.is_empty() {
            None
        } else {
            Some(true)
        };
        policy_robust_feasibility.push(PolicyRobustFeasibility {
            policy_id: policy.policy_id.clone(),
            expected_model_case_count: eligible_models.len() * manifest.cases.len(),
            pass_count,
            fail_count,
            unknown_count,
            feasible_across_qualified_models: feasible,
        });
    }

    let objectives = manifest
        .metrics
        .iter()
        .filter(|metric| matches!(metric.role, DecisionMetricRole::Objective { .. }))
        .collect::<Vec<_>>();
    let mut case_objectives = Vec::new();

    for model in &eligible_models {
        for case in &manifest.cases {
            let policy_rows = manifest
                .policies
                .iter()
                .map(|policy| {
                    (
                        policy,
                        rows.get(&(
                            model.model_id.clone(),
                            case.case_id.clone(),
                            policy.policy_id.clone(),
                        ))
                        .copied(),
                    )
                })
                .collect::<Vec<_>>();

            let statuses = policy_rows
                .iter()
                .map(|(_, outcome)| constraint_status(*outcome, &manifest.metrics))
                .collect::<Vec<_>>();
            let constraint_unknown = statuses
                .iter()
                .any(|status| *status == FeasibilityStatus::Unknown);
            let feasible_policy_indices = statuses
                .iter()
                .enumerate()
                .filter(|(_, status)| **status == FeasibilityStatus::Pass)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();

            for objective in &objectives {
                let unit = objective.unit.clone();
                let (status, feasible_policy_ids, best_policy_ids, best_value) =
                    if constraint_unknown {
                        (
                            CaseComparisonStatus::NotAssessed,
                            Vec::new(),
                            Vec::new(),
                            None,
                        )
                    } else if feasible_policy_indices.is_empty() {
                        (
                            CaseComparisonStatus::NoFeasiblePolicy,
                            Vec::new(),
                            Vec::new(),
                            None,
                        )
                    } else {
                        let mut candidate_values = Vec::new();
                        let mut missing_value = false;
                        for index in &feasible_policy_indices {
                            let (policy, outcome) = policy_rows[*index];
                            let value = metric_value(outcome, &objective.metric_id);
                            if let Some(value) = value {
                                candidate_values.push((policy.policy_id.as_str(), value));
                            } else {
                                missing_value = true;
                            }
                        }

                        if missing_value {
                            (
                                CaseComparisonStatus::NotAssessed,
                                feasible_policy_indices
                                    .iter()
                                    .map(|index| policy_rows[*index].0.policy_id.clone())
                                    .collect(),
                                Vec::new(),
                                None,
                            )
                        } else {
                            let direction = if let DecisionMetricRole::Objective { direction } =
                                objective.role
                            {
                                direction
                            } else {
                                continue;
                            };
                            let best = best_metric_value(
                                direction,
                                &candidate_values
                                    .iter()
                                    .map(|(_, value)| *value)
                                    .collect::<Vec<_>>(),
                            );
                            let winners = candidate_values
                                .iter()
                                .filter(|(_, value)| (*value - best).abs() <= 1e-12)
                                .map(|(policy_id, _)| (*policy_id).to_string())
                                .collect::<Vec<_>>();
                            (
                                CaseComparisonStatus::Assessed,
                                feasible_policy_indices
                                    .iter()
                                    .map(|index| policy_rows[*index].0.policy_id.clone())
                                    .collect(),
                                winners,
                                Some(best),
                            )
                        }
                    };

                case_objectives.push(CaseObjectiveAssessment {
                    model_id: model.model_id.clone(),
                    case_id: case.case_id.clone(),
                    objective_id: objective.metric_id.clone(),
                    status,
                    feasible_policy_ids,
                    best_policy_ids,
                    best_value,
                    unit,
                });
            }
        }
    }

    let mut objective_stability = Vec::new();
    let mut policy_objective_regret = Vec::new();
    let mut minimax_regret = Vec::new();

    for objective in &objectives {
        let objective_cases = case_objectives
            .iter()
            .filter(|row| row.objective_id == objective.metric_id)
            .collect::<Vec<_>>();
        let assessed = objective_cases
            .iter()
            .filter(|row| row.status == CaseComparisonStatus::Assessed)
            .count();
        let no_feasible = objective_cases
            .iter()
            .filter(|row| row.status == CaseComparisonStatus::NoFeasiblePolicy)
            .count();
        let not_assessed = objective_cases
            .iter()
            .filter(|row| row.status == CaseComparisonStatus::NotAssessed)
            .count();
        let distinct_winners = objective_cases
            .iter()
            .flat_map(|row| row.best_policy_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let intersection = if objective_cases.is_empty() {
            BTreeSet::new()
        } else {
            let mut current: Option<BTreeSet<String>> = None;
            for row in &objective_cases {
                if row.status != CaseComparisonStatus::Assessed {
                    current = Some(BTreeSet::new());
                    break;
                }
                let winners = row.best_policy_ids.iter().cloned().collect::<BTreeSet<_>>();
                current = Some(match current {
                    Some(previous) => previous.intersection(&winners).cloned().collect(),
                    None => winners,
                });
            }
            current.unwrap_or_default()
        };
        let expected_model_case_count = eligible_models.len() * manifest.cases.len();
        let complete = expected_model_case_count > 0 && assessed == expected_model_case_count;
        let best_policy_stable_across_all_model_cases =
            complete.then_some(!intersection.is_empty());

        objective_stability.push(ObjectiveStabilitySummary {
            objective_id: objective.metric_id.clone(),
            unit: objective.unit.clone(),
            expected_model_case_count,
            assessed_model_case_count: assessed,
            no_feasible_policy_count: no_feasible,
            not_assessed_count: not_assessed,
            distinct_winner_policy_count: distinct_winners.len(),
            best_policy_intersection: intersection.into_iter().collect(),
            best_policy_stable_across_all_model_cases,
        });

        for policy in &manifest.policies {
            let robust_status = policy_robust_feasibility
                .iter()
                .find(|row| row.policy_id == policy.policy_id)
                .map(|row| row.feasible_across_qualified_models)
                .unwrap_or(None);
            let policy_cases = objective_cases.iter().collect::<Vec<_>>();
            let mut regrets = Vec::new();
            let mut unknown_count = 0usize;
            let mut worst_model_id: Option<String> = None;
            let mut worst_value = f64::NEG_INFINITY;

            if robust_status == Some(true) {
                for assessment in &policy_cases {
                    if assessment.status != CaseComparisonStatus::Assessed {
                        unknown_count += 1;
                        continue;
                    }
                    let outcome = rows
                        .get(&(
                            assessment.model_id.clone(),
                            assessment.case_id.clone(),
                            policy.policy_id.clone(),
                        ))
                        .copied();
                    let value = metric_value(outcome, &objective.metric_id);
                    let Some(value) = value else {
                        unknown_count += 1;
                        continue;
                    };
                    let Some(best) = assessment.best_value else {
                        unknown_count += 1;
                        continue;
                    };
                    let regret = objective_regret(objective.role, value, best);
                    regrets.push(regret);
                    if regret > worst_value {
                        worst_value = regret;
                        worst_model_id = Some(assessment.model_id.clone());
                    }
                }
            } else {
                unknown_count = expected_model_case_count;
            }

            let complete_regret = robust_status == Some(true)
                && unknown_count == 0
                && regrets.len() == expected_model_case_count;
            policy_objective_regret.push(PolicyObjectiveRegret {
                policy_id: policy.policy_id.clone(),
                objective_id: objective.metric_id.clone(),
                unit: objective.unit.clone(),
                feasible_across_qualified_models: robust_status,
                expected_model_case_count,
                assessed_model_case_count: regrets.len(),
                unknown_model_case_count: unknown_count,
                mean_regret: complete_regret
                    .then(|| regrets.iter().sum::<f64>() / regrets.len() as f64),
                worst_case_regret: complete_regret.then_some(worst_value),
                worst_case_model_id: complete_regret.then_some(worst_model_id).flatten(),
            });
        }

        let mut eligible_regrets = policy_objective_regret
            .iter()
            .filter(|row| row.objective_id == objective.metric_id)
            .filter_map(|row| {
                row.worst_case_regret
                    .map(|regret| (row.policy_id.clone(), regret))
            })
            .collect::<Vec<_>>();
        eligible_regrets.sort_by(|left, right| left.1.total_cmp(&right.1));
        let minimax_value = eligible_regrets.first().map(|(_, value)| *value);
        let minimax_policy_ids = minimax_value
            .map(|best| {
                eligible_regrets
                    .iter()
                    .filter(|(_, value)| (*value - best).abs() <= 1e-12)
                    .map(|(policy_id, _)| policy_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        minimax_regret.push(MinimaxRegretSummary {
            objective_id: objective.metric_id.clone(),
            unit: objective.unit.clone(),
            minimax_policy_ids,
            minimum_worst_case_regret: minimax_value,
        });
    }

    Ok(ModelSetDecisionReport {
        model_set_id: manifest.model_set_id.clone(),
        manifest_digest: manifest.manifest_digest.clone(),
        adequacy_registry_digest: manifest.adequacy_registry_digest.clone(),
        case_manifest_digest: manifest.case_manifest_digest.clone(),
        policy_manifest_digest: manifest.policy_manifest_digest.clone(),
        metric_registry_digest: manifest.metric_registry_digest.clone(),
        coverage,
        declared_model_count: manifest.models.len(),
        qualified_model_count: eligible_models.len(),
        excluded_model_count,
        distinct_qualified_model_family_count: distinct_families,
        qualification_receipts,
        policy_case_feasibility,
        policy_robust_feasibility,
        case_objectives,
        objective_stability,
        policy_objective_regret,
        minimax_regret,
    })
}

fn metric_value(outcome: Option<&ModelPolicyOutcome>, metric_id: &str) -> Option<f64> {
    outcome?
        .values
        .iter()
        .find(|value| value.metric_id == metric_id)
        .and_then(|value| value.value)
}

fn constraint_status(
    outcome: Option<&ModelPolicyOutcome>,
    metric_specs: &[DecisionMetricSpec],
) -> FeasibilityStatus {
    let Some(outcome) = outcome else {
        return FeasibilityStatus::Unknown;
    };

    let mut unknown = false;
    for spec in metric_specs {
        if let DecisionMetricRole::HardConstraint {
            comparison,
            threshold,
        } = spec.role
        {
            let value = metric_value(Some(outcome), &spec.metric_id);
            match value {
                Some(value) if comparison.passes(value, threshold) => {}
                Some(_) => return FeasibilityStatus::Fail,
                None => unknown = true,
            }
        }
    }

    if unknown {
        FeasibilityStatus::Unknown
    } else {
        FeasibilityStatus::Pass
    }
}

fn best_metric_value(direction: ObjectiveDirection, values: &[f64]) -> f64 {
    match direction {
        ObjectiveDirection::Minimize => values.iter().copied().fold(f64::INFINITY, f64::min),
        ObjectiveDirection::Maximize => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

fn objective_regret(role: DecisionMetricRole, value: f64, best: f64) -> f64 {
    match role {
        DecisionMetricRole::Objective {
            direction: ObjectiveDirection::Minimize,
        } => (value - best).max(0.0),
        DecisionMetricRole::Objective {
            direction: ObjectiveDirection::Maximize,
        } => (best - value).max(0.0),
        DecisionMetricRole::HardConstraint { .. } => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ModelSetManifest {
        ModelSetManifest {
            model_set_id: "model-set-v1".to_string(),
            manifest_digest: "sha256-manifest".to_string(),
            adequacy_registry_digest: "sha256-adequacy-registry".to_string(),
            case_manifest_digest: "sha256-cases".to_string(),
            policy_manifest_digest: "sha256-policies".to_string(),
            metric_registry_digest: "sha256-metrics".to_string(),
            models: vec![
                CandidateModel {
                    model_id: "model-a".to_string(),
                    model_family_id: "family-a".to_string(),
                    model_digest: "sha256-model-a".to_string(),
                    adequacy_receipt_digest: "sha256-adequacy-a".to_string(),
                    qualification: ModelQualification::NoPrimaryDiscrepancyDetected,
                },
                CandidateModel {
                    model_id: "model-b".to_string(),
                    model_family_id: "family-b".to_string(),
                    model_digest: "sha256-model-b".to_string(),
                    adequacy_receipt_digest: "sha256-adequacy-b".to_string(),
                    qualification: ModelQualification::NoPrimaryDiscrepancyDetected,
                },
            ],
            cases: vec![
                DecisionCase {
                    case_id: "drought".to_string(),
                    case_digest: "sha256-drought".to_string(),
                },
                DecisionCase {
                    case_id: "energy-shock".to_string(),
                    case_digest: "sha256-energy-shock".to_string(),
                },
            ],
            policies: vec![
                DecisionPolicy {
                    policy_id: "reserve".to_string(),
                    policy_digest: "sha256-reserve".to_string(),
                },
                DecisionPolicy {
                    policy_id: "demand-first".to_string(),
                    policy_digest: "sha256-demand-first".to_string(),
                },
                DecisionPolicy {
                    policy_id: "growth-first".to_string(),
                    policy_digest: "sha256-growth-first".to_string(),
                },
            ],
            metrics: vec![
                DecisionMetricSpec {
                    metric_id: "unmet-water".to_string(),
                    unit: "m3".to_string(),
                    role: DecisionMetricRole::Objective {
                        direction: ObjectiveDirection::Minimize,
                    },
                },
                DecisionMetricSpec {
                    metric_id: "essential-coverage".to_string(),
                    unit: "fraction".to_string(),
                    role: DecisionMetricRole::HardConstraint {
                        comparison: ConstraintComparison::AtLeast,
                        threshold: 0.95,
                    },
                },
            ],
        }
    }

    fn outcome(
        model_id: &str,
        model_digest: &str,
        case_id: &str,
        case_digest: &str,
        policy_id: &str,
        policy_digest: &str,
        unmet_water: Option<f64>,
        coverage: Option<f64>,
    ) -> ModelPolicyOutcome {
        ModelPolicyOutcome {
            model_id: model_id.to_string(),
            model_digest: model_digest.to_string(),
            case_id: case_id.to_string(),
            case_digest: case_digest.to_string(),
            policy_id: policy_id.to_string(),
            policy_digest: policy_digest.to_string(),
            outcome_digest: format!("sha256-{model_id}-{case_id}-{policy_id}"),
            values: vec![
                DecisionMetricValue {
                    metric_id: "unmet-water".to_string(),
                    unit: "m3".to_string(),
                    value: unmet_water,
                },
                DecisionMetricValue {
                    metric_id: "essential-coverage".to_string(),
                    unit: "fraction".to_string(),
                    value: coverage,
                },
            ],
        }
    }

    fn complete_outcomes() -> Vec<ModelPolicyOutcome> {
        let mut rows = Vec::new();
        for (model_id, model_digest, cases) in [
            (
                "model-a",
                "sha256-model-a",
                [
                    [
                        ("reserve", "sha256-reserve", 10.0, 0.99),
                        ("demand-first", "sha256-demand-first", 8.0, 0.98),
                        ("growth-first", "sha256-growth-first", 2.0, 0.90),
                    ],
                    [
                        ("reserve", "sha256-reserve", 7.0, 0.98),
                        ("demand-first", "sha256-demand-first", 6.0, 0.97),
                        ("growth-first", "sha256-growth-first", 1.0, 0.91),
                    ],
                ],
            ),
            (
                "model-b",
                "sha256-model-b",
                [
                    [
                        ("reserve", "sha256-reserve", 9.0, 0.98),
                        ("demand-first", "sha256-demand-first", 1.0, 0.90),
                        ("growth-first", "sha256-growth-first", 6.0, 0.99),
                    ],
                    [
                        ("reserve", "sha256-reserve", 8.0, 0.97),
                        ("demand-first", "sha256-demand-first", 2.0, 0.92),
                        ("growth-first", "sha256-growth-first", 5.0, 0.98),
                    ],
                ],
            ),
        ] {
            let case_ids = ["drought", "energy-shock"];
            let case_digests = ["sha256-drought", "sha256-energy-shock"];
            for (case_index, case_rows) in cases.into_iter().enumerate() {
                for (policy_id, policy_digest, unmet, coverage) in case_rows {
                    rows.push(outcome(
                        model_id,
                        model_digest,
                        case_ids[case_index],
                        case_digests[case_index],
                        policy_id,
                        policy_digest,
                        Some(unmet),
                        Some(coverage),
                    ));
                }
            }
        }
        rows
    }

    #[test]
    fn model_qualification_exclusions_remain_visible_and_reduce_scope() {
        let mut candidate = manifest();
        candidate.models.push(CandidateModel {
            model_id: "model-c".to_string(),
            model_family_id: "family-c".to_string(),
            model_digest: "sha256-model-c".to_string(),
            adequacy_receipt_digest: "sha256-adequacy-c".to_string(),
            qualification: ModelQualification::PrimaryDiscrepancyDetected,
        });
        let report = analyze_model_set_decisions(&candidate, &complete_outcomes()).unwrap();
        assert_eq!(report.coverage, ModelSetCoverage::PartialQualifiedModelSet);
        assert_eq!(report.declared_model_count, 3);
        assert_eq!(report.qualified_model_count, 2);
        assert_eq!(report.excluded_model_count, 1);
        assert!(report.qualification_receipts.iter().any(|row| {
            row.model_id == "model-c"
                && row.qualification == ModelQualification::PrimaryDiscrepancyDetected
                && !row.eligible_for_comparison
        }));
    }

    #[test]
    fn winner_reversals_are_reported_but_minimax_regret_stays_metric_specific() {
        let report = analyze_model_set_decisions(&manifest(), &complete_outcomes()).unwrap();
        let stability = &report.objective_stability[0];
        assert_eq!(stability.distinct_winner_policy_count, 2);
        assert_eq!(
            stability.best_policy_stable_across_all_model_cases,
            Some(false)
        );
        assert!(stability.best_policy_intersection.is_empty());

        let reserve = report
            .policy_robust_feasibility
            .iter()
            .find(|row| row.policy_id == "reserve")
            .unwrap();
        assert_eq!(reserve.feasible_across_qualified_models, Some(true));
        let demand = report
            .policy_robust_feasibility
            .iter()
            .find(|row| row.policy_id == "demand-first")
            .unwrap();
        assert_eq!(demand.feasible_across_qualified_models, Some(false));

        let minimax = &report.minimax_regret[0];
        assert_eq!(minimax.minimax_policy_ids, vec!["reserve".to_string()]);
        assert!((minimax.minimum_worst_case_regret.unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn missing_eligible_model_outcome_prevents_partial_oracle_and_minimax_claim() {
        let mut rows = complete_outcomes();
        rows.retain(|row| {
            !(row.model_id == "model-b"
                && row.case_id == "energy-shock"
                && row.policy_id == "reserve")
        });
        let report = analyze_model_set_decisions(&manifest(), &rows).unwrap();
        assert!(
            report.objective_stability[0]
                .best_policy_stable_across_all_model_cases
                .is_none()
        );
        assert!(report.minimax_regret[0].minimum_worst_case_regret.is_none());
        assert!(report.policy_robust_feasibility.iter().any(|row| {
            row.policy_id == "reserve" && row.feasible_across_qualified_models.is_none()
        }));
    }

    #[test]
    fn no_qualified_models_returns_not_assessed_without_using_excluded_outcomes() {
        let mut candidate = manifest();
        for model in &mut candidate.models {
            model.qualification = ModelQualification::NotAssessed;
        }
        let report = analyze_model_set_decisions(&candidate, &complete_outcomes()).unwrap();
        assert_eq!(report.coverage, ModelSetCoverage::NoQualifiedModels);
        assert_eq!(report.qualified_model_count, 0);
        assert!(
            report
                .policy_robust_feasibility
                .iter()
                .all(|row| { row.feasible_across_qualified_models.is_none() })
        );
        assert!(
            report
                .minimax_regret
                .iter()
                .all(|row| { row.minimum_worst_case_regret.is_none() })
        );
    }

    #[test]
    fn any_known_hard_constraint_violation_blocks_robust_feasibility() {
        let mut rows = complete_outcomes();
        let target = rows
            .iter_mut()
            .find(|row| {
                row.model_id == "model-b"
                    && row.case_id == "energy-shock"
                    && row.policy_id == "reserve"
            })
            .unwrap();
        target.values[1].value = Some(0.80);
        let report = analyze_model_set_decisions(&manifest(), &rows).unwrap();
        let reserve = report
            .policy_robust_feasibility
            .iter()
            .find(|row| row.policy_id == "reserve")
            .unwrap();
        assert_eq!(reserve.feasible_across_qualified_models, Some(false));
        assert!(report.minimax_regret[0].minimum_worst_case_regret.is_none());
    }

    #[test]
    fn metric_unit_substitution_fails_closed() {
        let mut rows = complete_outcomes();
        rows[0].values[0].unit = "kWh".to_string();
        assert!(matches!(
            analyze_model_set_decisions(&manifest(), &rows),
            Err(ModelSetError::MetricUnitMismatch { .. })
        ));
    }

    #[test]
    fn exact_model_case_policy_provenance_is_required() {
        let mut rows = complete_outcomes();
        rows[0].model_digest = "sha256-wrong-model".to_string();
        assert!(matches!(
            analyze_model_set_decisions(&manifest(), &rows),
            Err(ModelSetError::ModelDigestMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_outcomes_are_rejected() {
        let mut rows = complete_outcomes();
        rows.push(rows[0].clone());
        assert!(matches!(
            analyze_model_set_decisions(&manifest(), &rows),
            Err(ModelSetError::DuplicateOutcome { .. })
        ));
    }

    #[test]
    fn maximize_objectives_use_the_opposite_regret_direction() {
        let mut candidate = manifest();
        candidate.metrics.push(DecisionMetricSpec {
            metric_id: "reserve-stock".to_string(),
            unit: "m3".to_string(),
            role: DecisionMetricRole::Objective {
                direction: ObjectiveDirection::Maximize,
            },
        });
        let mut rows = complete_outcomes();
        for row in &mut rows {
            row.values.push(DecisionMetricValue {
                metric_id: "reserve-stock".to_string(),
                unit: "m3".to_string(),
                value: Some(match row.policy_id.as_str() {
                    "reserve" => 100.0,
                    "demand-first" => 80.0,
                    _ => 60.0,
                }),
            });
        }
        let report = analyze_model_set_decisions(&candidate, &rows).unwrap();
        let reserve_regret = report
            .policy_objective_regret
            .iter()
            .find(|item| item.policy_id == "reserve" && item.objective_id == "reserve-stock")
            .unwrap();
        assert_eq!(reserve_regret.worst_case_regret, Some(0.0));
    }

    #[test]
    fn tied_winners_are_preserved_as_a_set() {
        let mut rows = complete_outcomes();
        for row in &mut rows {
            row.values[0].value = Some(match row.policy_id.as_str() {
                "reserve" | "demand-first" => 10.0,
                _ => 12.0,
            });
        }
        let report = analyze_model_set_decisions(&manifest(), &rows).unwrap();
        assert!(report.case_objectives.iter().any(|item| {
            item.model_id == "model-a"
                && item.case_id == "drought"
                && item.best_policy_ids.len() == 2
        }));
    }

    #[test]
    fn model_family_count_does_not_treat_duplicate_family_as_independent_evidence() {
        let mut candidate = manifest();
        let shared_family_id = candidate.models[0].model_family_id.clone();
        candidate.models[1].model_family_id = shared_family_id;
        let report = analyze_model_set_decisions(&candidate, &complete_outcomes()).unwrap();
        assert_eq!(report.qualified_model_count, 2);
        assert_eq!(report.distinct_qualified_model_family_count, 1);
    }
}
