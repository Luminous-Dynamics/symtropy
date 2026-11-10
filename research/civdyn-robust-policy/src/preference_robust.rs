// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Policy qualification across an explicitly declared finite preference polytope.
//!
//! This module does not infer stakeholder values from simulation outputs and
//! does not assign probabilities to preference vertices. Each vertex is a
//! separately identified utility contract. Hard household service floors are
//! non-compensatory and are applied before any utility comparison.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use civdyn_cross_scale_resilience::{InterventionPolicy, ShockKind};
use civdyn_resilience_uq::{OutcomeMetric, RunMetrics};

use crate::{PolicyRun, ServiceConstraints, analyze_policy_runs};

const WEIGHT_SUM_TOLERANCE: f64 = 1e-9;
const COMPARISON_TOLERANCE: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

impl ObjectiveDirection {
    const fn sign(self) -> f64 {
        match self {
            Self::Minimize => -1.0,
            Self::Maximize => 1.0,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceCriterion {
    pub metric: OutcomeMetric,
    pub direction: ObjectiveDirection,
    /// Externally declared positive scale in the metric's native unit.
    /// It must not be estimated from the evaluated challenge outcomes.
    pub reference_scale: f64,
    /// Provenance for the scale/normalizer (e.g. service standard or approved
    /// reference range). This digest is an identity, not proof of legitimacy.
    pub scale_evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceVertex {
    pub profile_id: String,
    pub stakeholder_group_id: String,
    /// Non-negative criterion weights in the contract's exact criterion order.
    /// The vector is a declared polytope vertex, not a probability distribution.
    pub weights: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceContract {
    pub contract_id: String,
    pub version: String,
    pub contract_digest: String,
    pub criteria: Vec<PreferenceCriterion>,
    /// Declared extreme points of the finite preference polytope.
    pub vertices: Vec<PreferenceVertex>,
    pub service_constraints: ServiceConstraints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyScenarioStatus {
    Comparable,
    NoFeasiblePolicy,
    IncompleteComparison,
}

impl PolicyScenarioStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Comparable => "comparable",
            Self::NoFeasiblePolicy => "no_feasible_policy",
            Self::IncompleteComparison => "incomplete_comparison",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEligibility {
    Feasible,
    Infeasible,
    Unknown,
}

impl PolicyEligibility {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Feasible => "feasible",
            Self::Infeasible => "infeasible",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceScenarioResult {
    pub design_point_id: String,
    pub shock_kind: String,
    pub profile_id: String,
    pub policy: String,
    pub eligibility: PolicyEligibility,
    /// Utility is present for measured outcomes even when the policy violates
    /// a hard constraint; infeasible policies never receive regret or a win.
    pub normalized_utility: Option<f64>,
    pub regret: Option<f64>,
    pub tied_best: bool,
    pub pareto_nondominated: Option<bool>,
    pub feasible_policy_count: usize,
    pub scenario_status: PolicyScenarioStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceProfilePolicySummary {
    pub profile_id: String,
    pub stakeholder_group_id: String,
    pub policy: String,
    pub scenario_count: usize,
    pub feasible_count: usize,
    pub infeasible_count: usize,
    pub unknown_count: usize,
    pub comparable_count: usize,
    pub best_scenario_count: usize,
    pub mean_regret: Option<f64>,
    pub p95_regret: Option<f64>,
    pub worst_regret: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyPreferenceSummary {
    pub policy: String,
    pub scenario_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub unknown_count: usize,
    /// None means no failure was observed but evidence is incomplete.
    pub finite_design_robust_feasible: Option<bool>,
    pub preference_vertices_best_count: usize,
    pub preference_vertices_considered: usize,
    /// Worst regret is only reported for a policy feasible on the entire
    /// finite design and only when every policy comparison is complete.
    pub worst_case_regret_across_preference_vertices: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceScenarioWinner {
    pub design_point_id: String,
    pub shock_kind: String,
    pub profile_id: String,
    pub status: PolicyScenarioStatus,
    pub feasible_policy_count: usize,
    pub unknown_policy_count: usize,
    pub best_policy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceScenarioConsensus {
    pub design_point_id: String,
    pub shock_kind: String,
    pub profile_count: usize,
    pub profiles_with_winner: usize,
    pub distinct_winner_policy_count: usize,
    pub profiles_with_different_winners: usize,
    pub unanimous_single_winner: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceRobustReport {
    pub contract_id: String,
    pub contract_version: String,
    pub contract_digest: String,
    pub criterion_count: usize,
    pub preference_vertex_count: usize,
    pub design_point_count: usize,
    pub scenario_count: usize,
    pub finite_design_robust_feasible_policy_count: usize,
    pub no_feasible_scenario_profile_count: usize,
    pub incomplete_scenario_profile_count: usize,
    pub results: Vec<PreferenceScenarioResult>,
    pub profile_policy_summaries: Vec<PreferenceProfilePolicySummary>,
    pub policy_summaries: Vec<PolicyPreferenceSummary>,
    pub scenario_winners: Vec<PreferenceScenarioWinner>,
    pub scenario_consensus: Vec<PreferenceScenarioConsensus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreferenceRobustError {
    InvalidIdentity { field: &'static str },
    InvalidCriterionCount,
    DuplicateCriterion { metric: &'static str },
    InvalidReferenceScale { metric: &'static str },
    MissingScaleEvidence { metric: &'static str },
    EmptyPreferenceVertices,
    DuplicateProfileId { profile_id: String },
    EmptyStakeholderGroup { profile_id: String },
    PreferenceWeightCountMismatch { profile_id: String },
    InvalidPreferenceWeight { profile_id: String },
    PreferenceWeightsDoNotSumToOne { profile_id: String },
    DuplicatePreferenceVertex { profile_id: String },
    Decision(String),
    NonFiniteUtility,
}

impl fmt::Display for PreferenceRobustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PreferenceRobustError {}

impl From<crate::DecisionError> for PreferenceRobustError {
    fn from(value: crate::DecisionError) -> Self {
        Self::Decision(value.to_string())
    }
}

impl PreferenceContract {
    pub fn validate(&self) -> Result<(), PreferenceRobustError> {
        for (value, field) in [
            (self.contract_id.as_str(), "contract_id"),
            (self.version.as_str(), "version"),
            (self.contract_digest.as_str(), "contract_digest"),
        ] {
            if value.trim().is_empty() {
                return Err(PreferenceRobustError::InvalidIdentity { field });
            }
        }
        self.service_constraints
            .validate()
            .map_err(PreferenceRobustError::from)?;

        if self.criteria.len() < 2 {
            return Err(PreferenceRobustError::InvalidCriterionCount);
        }
        let mut metric_ids = BTreeSet::new();
        for criterion in &self.criteria {
            if !metric_ids.insert(criterion.metric.name()) {
                return Err(PreferenceRobustError::DuplicateCriterion {
                    metric: criterion.metric.name(),
                });
            }
            if !criterion.reference_scale.is_finite() || criterion.reference_scale <= 0.0 {
                return Err(PreferenceRobustError::InvalidReferenceScale {
                    metric: criterion.metric.name(),
                });
            }
            if criterion.scale_evidence_digest.trim().is_empty() {
                return Err(PreferenceRobustError::MissingScaleEvidence {
                    metric: criterion.metric.name(),
                });
            }
        }
        if self.vertices.is_empty() {
            return Err(PreferenceRobustError::EmptyPreferenceVertices);
        }

        let mut profile_ids = BTreeSet::new();
        let mut weight_vectors = BTreeSet::new();
        for vertex in &self.vertices {
            if vertex.profile_id.trim().is_empty() {
                return Err(PreferenceRobustError::InvalidIdentity {
                    field: "profile_id",
                });
            }
            if !profile_ids.insert(vertex.profile_id.as_str()) {
                return Err(PreferenceRobustError::DuplicateProfileId {
                    profile_id: vertex.profile_id.clone(),
                });
            }
            if vertex.stakeholder_group_id.trim().is_empty() {
                return Err(PreferenceRobustError::EmptyStakeholderGroup {
                    profile_id: vertex.profile_id.clone(),
                });
            }
            if vertex.weights.len() != self.criteria.len() {
                return Err(PreferenceRobustError::PreferenceWeightCountMismatch {
                    profile_id: vertex.profile_id.clone(),
                });
            }
            if vertex.weights.iter().any(|weight| !weight.is_finite() || *weight < 0.0) {
                return Err(PreferenceRobustError::InvalidPreferenceWeight {
                    profile_id: vertex.profile_id.clone(),
                });
            }
            let total = vertex.weights.iter().sum::<f64>();
            if !total.is_finite() || (total - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
                return Err(PreferenceRobustError::PreferenceWeightsDoNotSumToOne {
                    profile_id: vertex.profile_id.clone(),
                });
            }
            // Canonical integer quantization only detects exact duplicate vectors
            // at the declared tolerance; the contract never treats counts as
            // probabilities or averages stakeholder groups.
            let key = vertex
                .weights
                .iter()
                .map(|weight| (weight / WEIGHT_SUM_TOLERANCE).round() as i64)
                .collect::<Vec<_>>();
            if !weight_vectors.insert(key) {
                return Err(PreferenceRobustError::DuplicatePreferenceVertex {
                    profile_id: vertex.profile_id.clone(),
                });
            }
        }
        Ok(())
    }
}

fn policy_eligibility(
    metrics: Option<RunMetrics>,
    constraints: ServiceConstraints,
) -> PolicyEligibility {
    let Some(metrics) = metrics else {
        return PolicyEligibility::Unknown;
    };
    let margins = [
        metrics.minimum_water_coverage - constraints.minimum_water_coverage,
        metrics.minimum_energy_coverage - constraints.minimum_energy_coverage,
        constraints.maximum_water_breach_fraction - metrics.water_floor_breach_fraction,
        constraints.maximum_energy_breach_fraction - metrics.energy_floor_breach_fraction,
    ];
    if margins.iter().any(|margin| *margin < -COMPARISON_TOLERANCE) {
        PolicyEligibility::Infeasible
    } else {
        PolicyEligibility::Feasible
    }
}

fn normalized_utility(
    metrics: RunMetrics,
    criterion_values: &[PreferenceCriterion],
    weights: &[f64],
) -> Result<f64, PreferenceRobustError> {
    let mut utility = 0.0;
    for (criterion, weight) in criterion_values.iter().zip(weights) {
        let value = metrics.value(criterion.metric);
        let term = criterion.direction.sign() * (value / criterion.reference_scale) * weight;
        if !term.is_finite() {
            return Err(PreferenceRobustError::NonFiniteUtility);
        }
        utility += term;
    }
    if !utility.is_finite() {
        return Err(PreferenceRobustError::NonFiniteUtility);
    }
    Ok(utility)
}

fn dominates(
    candidate: RunMetrics,
    other: RunMetrics,
    criteria: &[PreferenceCriterion],
) -> bool {
    let mut strictly_better = false;
    for criterion in criteria {
        let candidate_value = criterion.direction.sign() * candidate.value(criterion.metric);
        let other_value = criterion.direction.sign() * other.value(criterion.metric);
        if candidate_value + COMPARISON_TOLERANCE < other_value {
            return false;
        }
        if candidate_value > other_value + COMPARISON_TOLERANCE {
            strictly_better = true;
        }
    }
    strictly_better
}

fn pareto_flags(
    feasible: &[(String, RunMetrics)],
    criteria: &[PreferenceCriterion],
) -> BTreeMap<String, bool> {
    let mut flags = BTreeMap::new();
    for (policy, metrics) in feasible {
        let dominated = feasible.iter().any(|(other_policy, other_metrics)| {
            other_policy != policy && dominates(*other_metrics, *metrics, criteria)
        });
        flags.insert(policy.clone(), !dominated);
    }
    flags
}

fn percentile(sorted: &[f64], probability: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0]);
    }
    let position = probability * (sorted.len() - 1) as f64;
    let low = position.floor() as usize;
    let high = position.ceil() as usize;
    if low == high {
        Some(sorted[low])
    } else {
        let fraction = position - low as f64;
        Some(sorted[low] * (1.0 - fraction) + sorted[high] * fraction)
    }
}

/// Analyze policy recommendations over the complete, matched finite design and
/// the declared preference-polytope vertices. LHS scenarios and preference
/// vertices have no inferred probability weights.
pub fn analyze_preference_robustness(
    contract: &PreferenceContract,
    runs: &[PolicyRun],
) -> Result<PreferenceRobustReport, PreferenceRobustError> {
    contract.validate()?;
    let _base_report = analyze_policy_runs(0, contract.service_constraints, runs)?;

    let mut design_points = BTreeMap::new();
    let mut records = BTreeMap::new();
    for run in runs {
        design_points.insert(run.design_point.design_id.clone(), run.design_point.clone());
        records.insert(
            (
                run.design_point.design_id.clone(),
                run.shock_kind.name().to_string(),
                run.policy.name().to_string(),
            ),
            run,
        );
    }

    let design_ids = design_points.keys().cloned().collect::<Vec<_>>();
    let mut results = Vec::new();
    let mut scenario_winners = Vec::new();
    let mut scenario_consensus = Vec::new();
    let mut scenario_keys = Vec::new();
    let mut scenario_policy_eligibility = BTreeMap::new();
    let mut scenario_profile_regrets = BTreeMap::new();
    let mut scenario_profile_winners = BTreeMap::new();
    let mut no_feasible_scenario_profile_count = 0;
    let mut incomplete_scenario_profile_count = 0;

    for design_id in &design_ids {
        for shock in ShockKind::ALL {
            let scenario_key = (design_id.clone(), shock.name().to_string());
            scenario_keys.push(scenario_key.clone());
            let mut statuses = BTreeMap::new();
            let mut utilities = BTreeMap::new();
            for policy in InterventionPolicy::ALL {
                let run = records[&(
                    design_id.clone(),
                    shock.name().to_string(),
                    policy.name().to_string(),
                )];
                let eligibility = policy_eligibility(run.metrics, contract.service_constraints);
                statuses.insert(policy.name().to_string(), eligibility);
                scenario_policy_eligibility.insert(
                    (
                        design_id.clone(),
                        shock.name().to_string(),
                        policy.name().to_string(),
                    ),
                    eligibility,
                );
            }
            let feasible_metrics = InterventionPolicy::ALL
                .iter()
                .filter_map(|policy| {
                    let run = records[&(
                        design_id.clone(),
                        shock.name().to_string(),
                        policy.name().to_string(),
                    )];
                    (policy_eligibility(run.metrics, contract.service_constraints)
                        == PolicyEligibility::Feasible)
                        .then(|| (policy.name().to_string(), run.metrics.expect("feasible metrics")))
                })
                .collect::<Vec<_>>();
            let pareto = pareto_flags(&feasible_metrics, &contract.criteria);

            for profile in &contract.vertices {
                let feasible_count = statuses
                    .values()
                    .filter(|status| **status == PolicyEligibility::Feasible)
                    .count();
                let unknown_count = statuses
                    .values()
                    .filter(|status| **status == PolicyEligibility::Unknown)
                    .count();
                let scenario_status = if unknown_count > 0 {
                    PolicyScenarioStatus::IncompleteComparison
                } else if feasible_count > 0 {
                    PolicyScenarioStatus::Comparable
                } else {
                    PolicyScenarioStatus::NoFeasiblePolicy
                };
                if scenario_status == PolicyScenarioStatus::NoFeasiblePolicy {
                    no_feasible_scenario_profile_count += 1;
                }
                if scenario_status == PolicyScenarioStatus::IncompleteComparison {
                    incomplete_scenario_profile_count += 1;
                }

                let mut profile_utilities = BTreeMap::new();
                for policy in InterventionPolicy::ALL {
                    let run = records[&(
                        design_id.clone(),
                        shock.name().to_string(),
                        policy.name().to_string(),
                    )];
                    let utility = run
                        .metrics
                        .map(|metrics| normalized_utility(metrics, &contract.criteria, &profile.weights))
                        .transpose()?;
                    if let Some(value) = utility {
                        profile_utilities.insert(policy.name().to_string(), value);
                    }
                }
                let best_utility = if scenario_status == PolicyScenarioStatus::Comparable {
                    feasible_metrics
                        .iter()
                        .map(|(policy, _)| profile_utilities[policy])
                        .max_by(f64::total_cmp)
                } else {
                    None
                };
                let mut best_policy_ids = Vec::new();
                for policy in InterventionPolicy::ALL {
                    let policy_id = policy.name().to_string();
                    let eligibility = statuses[&policy_id];
                    let utility = profile_utilities.get(&policy_id).copied();
                    let regret = if eligibility == PolicyEligibility::Feasible {
                        best_utility.map(|best| {
                            (best - utility.expect("feasible policy utility"))
                                .max(0.0)
                        })
                    } else {
                        None
                    };
                    let tied_best = regret.is_some_and(|value| value <= COMPARISON_TOLERANCE);
                    if tied_best {
                        best_policy_ids.push(policy_id.clone());
                    }
                    let profile_result_key = (
                        profile.profile_id.clone(),
                        policy_id.clone(),
                    );
                    if let Some(value) = regret {
                        scenario_profile_regrets
                            .entry(profile_result_key)
                            .or_insert_with(Vec::new)
                            .push(value);
                    }
                    results.push(PreferenceScenarioResult {
                        design_point_id: design_id.clone(),
                        shock_kind: shock.name().to_string(),
                        profile_id: profile.profile_id.clone(),
                        policy: policy_id.clone(),
                        eligibility,
                        normalized_utility: utility,
                        regret,
                        tied_best,
                        pareto_nondominated: if eligibility == PolicyEligibility::Feasible
                            && scenario_status == PolicyScenarioStatus::Comparable
                        {
                            pareto.get(&policy_id).copied()
                        } else {
                            None
                        },
                        feasible_policy_count: feasible_count,
                        scenario_status,
                    });
                }
                scenario_winners.push(PreferenceScenarioWinner {
                    design_point_id: design_id.clone(),
                    shock_kind: shock.name().to_string(),
                    profile_id: profile.profile_id.clone(),
                    status: scenario_status,
                    feasible_policy_count: feasible_count,
                    unknown_policy_count: unknown_count,
                    best_policy_ids: best_policy_ids.clone(),
                });
                scenario_profile_winners.insert(
                    (
                        design_id.clone(),
                        shock.name().to_string(),
                        profile.profile_id.clone(),
                    ),
                    best_policy_ids,
                );
            }

            let mut union_winners = BTreeSet::new();
            let mut profiles_with_different_winners = 0;
            let mut first_winner_set: Option<Vec<String>> = None;
            let mut unanimous_single = true;
            let mut profiles_with_winner = 0;
            for profile in &contract.vertices {
                let winners = &scenario_profile_winners[&(
                    design_id.clone(),
                    shock.name().to_string(),
                    profile.profile_id.clone(),
                )];
                if !winners.is_empty() {
                    profiles_with_winner += 1;
                    union_winners.extend(winners.iter().cloned());
                }
                match &first_winner_set {
                    None => first_winner_set = Some(winners.clone()),
                    Some(previous) if previous == winners => {}
                    Some(_) => profiles_with_different_winners += 1,
                }
                if winners.len() != 1 {
                    unanimous_single = false;
                }
            }
            let unanimous_single_winner = if unanimous_single
                && profiles_with_winner == contract.vertices.len()
            {
                first_winner_set.and_then(|winners| winners.into_iter().next())
            } else {
                None
            };
            scenario_consensus.push(PreferenceScenarioConsensus {
                design_point_id: design_id.clone(),
                shock_kind: shock.name().to_string(),
                profile_count: contract.vertices.len(),
                profiles_with_winner,
                distinct_winner_policy_count: union_winners.len(),
                profiles_with_different_winners,
                unanimous_single_winner,
            });
        }
    }

    let mut profile_policy_summaries = Vec::new();
    let mut policy_summaries = Vec::new();
    let scenario_count = scenario_keys.len();
    let mut robust_feasible_policy_count = 0;
    for policy in InterventionPolicy::ALL {
        let policy_id = policy.name().to_string();
        let mut pass = 0;
        let mut fail = 0;
        let mut unknown = 0;
        for scenario in &scenario_keys {
            match scenario_policy_eligibility[&(
                scenario.0.clone(),
                scenario.1.clone(),
                policy_id.clone(),
            )] {
                PolicyEligibility::Feasible => pass += 1,
                PolicyEligibility::Infeasible => fail += 1,
                PolicyEligibility::Unknown => unknown += 1,
            }
        }
        let robust_feasible = if fail > 0 {
            Some(false)
        } else if unknown > 0 {
            None
        } else {
            Some(true)
        };
        if robust_feasible == Some(true) {
            robust_feasible_policy_count += 1;
        }

        let mut best_count = 0;
        let mut profiles_considered = 0;
        let mut worst_regrets = Vec::new();
        for profile in &contract.vertices {
            let regrets = scenario_profile_regrets
                .get(&(profile.profile_id.clone(), policy_id.clone()))
                .cloned()
                .unwrap_or_default();
            let best_scenarios = results.iter().filter(|result| {
                result.profile_id == profile.profile_id
                    && result.policy == policy_id
                    && result.tied_best
            }).count();
            best_count += best_scenarios;
            profiles_considered += scenario_count;
            let mut sorted = regrets.clone();
            sorted.sort_by(f64::total_cmp);
            profile_policy_summaries.push(PreferenceProfilePolicySummary {
                profile_id: profile.profile_id.clone(),
                stakeholder_group_id: profile.stakeholder_group_id.clone(),
                policy: policy_id.clone(),
                scenario_count,
                feasible_count: results.iter().filter(|result| {
                    result.profile_id == profile.profile_id
                        && result.policy == policy_id
                        && result.eligibility == PolicyEligibility::Feasible
                }).count(),
                infeasible_count: results.iter().filter(|result| {
                    result.profile_id == profile.profile_id
                        && result.policy == policy_id
                        && result.eligibility == PolicyEligibility::Infeasible
                }).count(),
                unknown_count: results.iter().filter(|result| {
                    result.profile_id == profile.profile_id
                        && result.policy == policy_id
                        && result.eligibility == PolicyEligibility::Unknown
                }).count(),
                comparable_count: regrets.len(),
                best_scenario_count: best_scenarios,
                mean_regret: (!regrets.is_empty()).then(|| {
                    regrets.iter().sum::<f64>() / regrets.len() as f64
                }),
                p95_regret: percentile(&sorted, 0.95),
                worst_regret: sorted.last().copied(),
            });
            worst_regrets.extend(regrets);
        }
        worst_regrets.sort_by(f64::total_cmp);
        let all_comparisons_complete = results.iter().all(|result| {
            result.eligibility != PolicyEligibility::Unknown
                && result.scenario_status == PolicyScenarioStatus::Comparable
        });
        policy_summaries.push(PolicyPreferenceSummary {
            policy: policy_id,
            scenario_count,
            pass_count: pass,
            fail_count: fail,
            unknown_count: unknown,
            finite_design_robust_feasible: robust_feasible,
            preference_vertices_best_count: best_count,
            preference_vertices_considered: profiles_considered,
            worst_case_regret_across_preference_vertices: if robust_feasible == Some(true)
                && all_comparisons_complete
            {
                worst_regrets.last().copied()
            } else {
                None
            },
        });
    }

    // Remove any rows accidentally inserted while constructing the panel; keep
    // vectors canonical so replay and downstream CSV receipts are deterministic.
    results.sort_by(|left, right| {
        (
            &left.design_point_id,
            &left.shock_kind,
            &left.profile_id,
            &left.policy,
        )
            .cmp(&(
                &right.design_point_id,
                &right.shock_kind,
                &right.profile_id,
                &right.policy,
            ))
    });
    scenario_winners.sort_by(|left, right| {
        (
            &left.design_point_id,
            &left.shock_kind,
            &left.profile_id,
        )
            .cmp(&(
                &right.design_point_id,
                &right.shock_kind,
                &right.profile_id,
            ))
    });
    scenario_consensus.sort_by(|left, right| {
        (&left.design_point_id, &left.shock_kind).cmp(&(&right.design_point_id, &right.shock_kind))
    });

    Ok(PreferenceRobustReport {
        contract_id: contract.contract_id.clone(),
        contract_version: contract.version.clone(),
        contract_digest: contract.contract_digest.clone(),
        criterion_count: contract.criteria.len(),
        preference_vertex_count: contract.vertices.len(),
        design_point_count: design_ids.len(),
        scenario_count,
        finite_design_robust_feasible_policy_count: robust_feasible_policy_count,
        no_feasible_scenario_profile_count,
        incomplete_scenario_profile_count,
        results,
        profile_policy_summaries,
        policy_summaries,
        scenario_winners,
        scenario_consensus,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_resilience_uq::{ParameterSet, RunMetrics};

    const CRITERIA: [OutcomeMetric; 5] = [
        OutcomeMetric::TotalUnmetWaterM3,
        OutcomeMetric::TotalUnmetEnergyKwh,
        OutcomeMetric::MinimumWaterCoverage,
        OutcomeMetric::MinimumEnergyCoverage,
        OutcomeMetric::RecoverySuccess,
    ];

    fn contract(vertices: Vec<PreferenceVertex>) -> PreferenceContract {
        PreferenceContract {
            contract_id: "municipal-service-preferences".to_string(),
            version: "v0.1".to_string(),
            contract_digest: "sha256-preference-contract".to_string(),
            criteria: CRITERIA
                .iter()
                .map(|metric| PreferenceCriterion {
                    metric: *metric,
                    direction: match metric {
                        OutcomeMetric::TotalUnmetWaterM3
                        | OutcomeMetric::TotalUnmetEnergyKwh => ObjectiveDirection::Minimize,
                        _ => ObjectiveDirection::Maximize,
                    },
                    reference_scale: match metric {
                        OutcomeMetric::TotalUnmetWaterM3 => 100.0,
                        OutcomeMetric::TotalUnmetEnergyKwh => 100.0,
                        _ => 1.0,
                    },
                    scale_evidence_digest: format!("sha256-scale-{}", metric.name()),
                })
                .collect(),
            vertices,
            service_constraints: ServiceConstraints::default(),
        }
    }

    fn two_preference_vertices() -> Vec<PreferenceVertex> {
        vec![
            PreferenceVertex {
                profile_id: "water-priority".to_string(),
                stakeholder_group_id: "water-dependent-households".to_string(),
                weights: vec![0.80, 0.05, 0.05, 0.05, 0.05],
            },
            PreferenceVertex {
                profile_id: "energy-priority".to_string(),
                stakeholder_group_id: "energy-dependent-households".to_string(),
                weights: vec![0.05, 0.80, 0.05, 0.05, 0.05],
            },
        ]
    }

    fn point(id: &str, coordinate: f64) -> DesignPoint {
        let unit_coordinates = [coordinate; 7];
        DesignPoint {
            design_id: id.to_string(),
            unit_coordinates,
            parameters: ParameterSet::from_unit_coordinates(unit_coordinates).unwrap(),
        }
    }

    fn metrics(
        water: f64,
        energy: f64,
        water_coverage: f64,
        energy_coverage: f64,
        recovery: f64,
    ) -> RunMetrics {
        RunMetrics {
            total_unmet_water_m3: water,
            total_unmet_energy_kwh: energy,
            water_deficit_area_household_steps: water / 10.0,
            energy_deficit_area_household_steps: energy / 10.0,
            minimum_water_coverage: water_coverage,
            minimum_energy_coverage: energy_coverage,
            water_floor_breach_fraction: 1.0 - water_coverage,
            energy_floor_breach_fraction: 1.0 - energy_coverage,
            recovery_success: recovery,
        }
    }

    fn tradeoff_fixture() -> Vec<PolicyRun> {
        let point = point("lhs-00", 0.5);
        let values = [
            (InterventionPolicy::NoTransfer, metrics(50.0, 50.0, 0.97, 0.97, 0.70)),
            (InterventionPolicy::ProportionalSolidarity, metrics(35.0, 35.0, 0.98, 0.98, 0.80)),
            (InterventionPolicy::ReserveProtected, metrics(10.0, 45.0, 0.99, 0.97, 0.85)),
            (InterventionPolicy::DeficitFirst, metrics(45.0, 10.0, 0.97, 0.99, 0.85)),
        ];
        ShockKind::ALL
            .iter()
            .flat_map(|shock| {
                values.iter().map({
                    let point = point.clone();
                    move |(policy, run_metrics)| PolicyRun {
                        design_point: point.clone(),
                        shock_kind: *shock,
                        policy: *policy,
                        metrics: Some(*run_metrics),
                    }
                })
            })
            .collect()
    }

    #[test]
    fn stakeholder_preference_vertices_can_reverse_policy_winners() {
        let report =
            analyze_preference_robustness(&contract(two_preference_vertices()), &tradeoff_fixture())
                .unwrap();
        assert!(report.scenario_winners.iter().any(|row| {
            row.profile_id == "water-priority" && row.best_policy_ids == vec!["reserve_protected"]
        }));
        assert!(report.scenario_winners.iter().any(|row| {
            row.profile_id == "energy-priority" && row.best_policy_ids == vec!["deficit_first"]
        }));
        assert!(report.scenario_consensus.iter().all(|row| {
            row.distinct_winner_policy_count >= 2 && row.unanimous_single_winner.is_none()
        }));
    }

    #[test]
    fn hard_service_constraints_block_a_high_utility_infeasible_policy() {
        let mut runs = tradeoff_fixture();
        let target = runs.iter_mut().find(|run| {
            run.policy == InterventionPolicy::ReserveProtected
                && run.shock_kind == ShockKind::WaterSupply
        }).unwrap();
        target.metrics = Some(metrics(0.0, 0.0, 0.70, 0.99, 1.0));
        let report = analyze_preference_robustness(&contract(two_preference_vertices()), &runs)
            .unwrap();
        let result = report.results.iter().find(|row| {
            row.policy == "reserve_protected" && row.shock_kind == "water_supply"
                && row.profile_id == "water-priority"
        }).unwrap();
        assert_eq!(result.eligibility, PolicyEligibility::Infeasible);
        assert_eq!(result.regret, None);
        assert_eq!(result.tied_best, false);
    }

    #[test]
    fn identical_feasible_policies_tie_with_zero_regret() {
        let mut runs = tradeoff_fixture();
        let shared = metrics(20.0, 20.0, 0.98, 0.98, 0.9);
        for run in &mut runs {
            run.metrics = Some(shared);
        }
        let report = analyze_preference_robustness(&contract(two_preference_vertices()), &runs)
            .unwrap();
        assert!(report.results.iter().all(|row| row.tied_best && row.regret == Some(0.0)));
        assert!(report.scenario_consensus.iter().all(|row| {
            row.distinct_winner_policy_count == 4 && row.unanimous_single_winner.is_none()
        }));
    }

    #[test]
    fn no_feasible_policy_is_an_abstention_not_a_utility_winner() {
        let mut runs = tradeoff_fixture();
        for run in &mut runs {
            run.metrics = Some(metrics(1.0, 1.0, 0.5, 0.5, 1.0));
        }
        let report = analyze_preference_robustness(&contract(two_preference_vertices()), &runs)
            .unwrap();
        assert_eq!(report.no_feasible_scenario_profile_count, 6);
        assert!(report.results.iter().all(|row| {
            row.scenario_status == PolicyScenarioStatus::NoFeasiblePolicy
                && row.regret.is_none()
                && !row.tied_best
        }));
    }

    #[test]
    fn missing_outcome_remains_unknown_without_fake_regret() {
        let mut runs = tradeoff_fixture();
        runs.iter_mut().find(|run| {
            run.policy == InterventionPolicy::NoTransfer
                && run.shock_kind == ShockKind::WaterSupply
        }).unwrap().metrics = None;
        let report = analyze_preference_robustness(&contract(two_preference_vertices()), &runs)
            .unwrap();
        assert_eq!(report.incomplete_scenario_profile_count, 2);
        assert!(report.results.iter().filter(|row| {
            row.shock_kind == "water_supply" && row.policy == "no_transfer"
        }).all(|row| {
            row.eligibility == PolicyEligibility::Unknown
                && row.regret.is_none()
                && row.normalized_utility.is_none()
        }));
    }

    #[test]
    fn common_unit_rescaling_with_matching_reference_scale_preserves_utility() {
        let first = contract(two_preference_vertices());
        let first_report = analyze_preference_robustness(&first, &tradeoff_fixture()).unwrap();

        let mut scaled_runs = tradeoff_fixture();
        for run in &mut scaled_runs {
            if let Some(metrics) = run.metrics.as_mut() {
                metrics.total_unmet_water_m3 *= 10.0;
            }
        }
        let mut scaled_contract = contract(two_preference_vertices());
        scaled_contract.criteria[0].reference_scale *= 10.0;
        let scaled_report = analyze_preference_robustness(&scaled_contract, &scaled_runs).unwrap();
        assert_eq!(
            first_report.results.iter().map(|row| (row.profile_id.clone(), row.policy.clone(), row.normalized_utility))
                .collect::<Vec<_>>(),
            scaled_report.results.iter().map(|row| (row.profile_id.clone(), row.policy.clone(), row.normalized_utility))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn invalid_preference_contracts_fail_closed() {
        let mut duplicate_id = contract(two_preference_vertices());
        duplicate_id.vertices[1].profile_id = duplicate_id.vertices[0].profile_id.clone();
        assert!(matches!(
            duplicate_id.validate(),
            Err(PreferenceRobustError::DuplicateProfileId { .. })
        ));

        let mut negative_weight = contract(two_preference_vertices());
        negative_weight.vertices[0].weights[0] = -0.1;
        assert!(matches!(
            negative_weight.validate(),
            Err(PreferenceRobustError::InvalidPreferenceWeight { .. })
        ));

        let mut bad_sum = contract(two_preference_vertices());
        bad_sum.vertices[0].weights[0] = 0.5;
        assert!(matches!(
            bad_sum.validate(),
            Err(PreferenceRobustError::PreferenceWeightsDoNotSumToOne { .. })
        ));

        let mut duplicate_criterion = contract(two_preference_vertices());
        duplicate_criterion.criteria[1].metric = duplicate_criterion.criteria[0].metric;
        assert!(matches!(
            duplicate_criterion.validate(),
            Err(PreferenceRobustError::DuplicateCriterion { .. })
        ));
    }

    #[test]
    fn incomplete_or_mismatched_scenario_design_fails_closed() {
        let mut missing = tradeoff_fixture();
        missing.pop();
        assert!(analyze_preference_robustness(&contract(two_preference_vertices()), &missing).is_err());

        let mut inconsistent = tradeoff_fixture();
        inconsistent[0].design_point.unit_coordinates = [0.1; 7];
        inconsistent[0].design_point.parameters = ParameterSet::from_unit_coordinates([0.1; 7]).unwrap();
        assert!(analyze_preference_robustness(&contract(two_preference_vertices()), &inconsistent).is_err());
    }
}
