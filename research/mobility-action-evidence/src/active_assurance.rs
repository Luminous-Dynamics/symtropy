//! Active assurance: select observation actions under independent risk and resource budgets.
//!
//! This module does not estimate risk or information gain. It accepts
//! caller-supplied conservative forecasts, validates observer qualification,
//! and returns a Pareto frontier without collapsing risk and cost into a score.

use super::Digest;
use std::collections::BTreeSet;

const MAX_RISK_BPS: u16 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationQualification {
    pub calibration_commitment: Digest,
    pub risk_model_commitment: Digest,
    pub clock_correlation_commitment: Digest,
    pub independence_policy_commitment: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationCandidate {
    pub candidate_id: String,
    pub predicate_id: String,
    pub source_id: String,
    pub observer_failure_domains: Vec<String>,
    pub qualification: ObservationQualification,
    pub expected_residual_risk_upper_bound_bps: u16,
    pub latency_upper_bound_ms: u64,
    pub energy_upper_bound_millijoules: u64,
    pub cost_upper_bound_microcredits: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceContext {
    pub predicate_id: String,
    pub current_risk_upper_bound_bps: u16,
    pub execution_failure_domains: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationBudget {
    pub maximum_residual_risk_bps: u16,
    pub maximum_latency_ms: u64,
    pub maximum_energy_millijoules: u64,
    pub maximum_cost_microcredits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssuranceError {
    EmptyPredicate,
    PredicateMismatch,
    MissingQualificationCommitment,
    InvalidCurrentRiskBound,
    InvalidMaximumRiskBound,
    EmptyFailureDomain,
    DuplicateFailureDomain,
    EmptyCandidateId,
    EmptySourceId,
    DuplicateCandidateId,
    MissingObserverFailureDomains,
    CommonCauseObserverExecutorOverlap,
    InvalidResidualRiskBound,
    NoExpectedRiskReduction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    NoObservationCandidates,
    NoCandidateMeetsRiskAndResourceBudgets,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetGate {
    ResidualRisk,
    Latency,
    Energy,
    Cost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilteredObservationCandidate {
    pub candidate_id: String,
    pub failed_gates: Vec<BudgetGate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssuranceDecision {
    AcquireObservation {
        pareto_frontier: Vec<ObservationCandidate>,
        filtered_out_candidates: Vec<FilteredObservationCandidate>,
        dominated_candidate_ids: Vec<String>,
    },
    Hold {
        reason: HoldReason,
        filtered_out_candidates: Vec<FilteredObservationCandidate>,
    },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_domains(domains: &[String]) -> Result<(), AssuranceError> {
    if domains.is_empty() {
        return Err(AssuranceError::EmptyFailureDomain);
    }
    let mut seen = BTreeSet::new();
    for domain in domains {
        if !nonempty(domain) {
            return Err(AssuranceError::EmptyFailureDomain);
        }
        if !seen.insert(domain.as_str()) {
            return Err(AssuranceError::DuplicateFailureDomain);
        }
    }
    Ok(())
}

fn dominates(left: &ObservationCandidate, right: &ObservationCandidate) -> bool {
    let no_worse = left.expected_residual_risk_upper_bound_bps
        <= right.expected_residual_risk_upper_bound_bps
        && left.latency_upper_bound_ms <= right.latency_upper_bound_ms
        && left.energy_upper_bound_millijoules <= right.energy_upper_bound_millijoules
        && left.cost_upper_bound_microcredits <= right.cost_upper_bound_microcredits;
    let strictly_better = left.expected_residual_risk_upper_bound_bps
        < right.expected_residual_risk_upper_bound_bps
        || left.latency_upper_bound_ms < right.latency_upper_bound_ms
        || left.energy_upper_bound_millijoules < right.energy_upper_bound_millijoules
        || left.cost_upper_bound_microcredits < right.cost_upper_bound_microcredits;
    no_worse && strictly_better
}

/// Validate candidate provenance, apply hard risk/resource gates and return
/// every non-dominated observation option. It never computes a weighted score.
///
/// Malformed candidate declarations fail the entire request. Well-formed
/// candidates that exceed a budget are listed as filtered out.
pub fn select_observation_frontier(
    context: &AssuranceContext,
    budget: ObservationBudget,
    candidates: &[ObservationCandidate],
) -> Result<AssuranceDecision, AssuranceError> {
    if !nonempty(&context.predicate_id) {
        return Err(AssuranceError::EmptyPredicate);
    }
    if context.current_risk_upper_bound_bps > MAX_RISK_BPS {
        return Err(AssuranceError::InvalidCurrentRiskBound);
    }
    if budget.maximum_residual_risk_bps > MAX_RISK_BPS {
        return Err(AssuranceError::InvalidMaximumRiskBound);
    }
    validate_domains(&context.execution_failure_domains)?;

    if candidates.is_empty() {
        return Ok(AssuranceDecision::Hold {
            reason: HoldReason::NoObservationCandidates,
            filtered_out_candidates: Vec::new(),
        });
    }

    let execution_domains: BTreeSet<&str> = context
        .execution_failure_domains
        .iter()
        .map(String::as_str)
        .collect();
    let mut candidate_ids = BTreeSet::new();
    let mut feasible = Vec::new();
    let mut filtered_out = Vec::new();

    for candidate in candidates {
        if !nonempty(&candidate.candidate_id) {
            return Err(AssuranceError::EmptyCandidateId);
        }
        if !nonempty(&candidate.predicate_id) || candidate.predicate_id != context.predicate_id {
            return Err(AssuranceError::PredicateMismatch);
        }
        if !nonempty(&candidate.source_id) {
            return Err(AssuranceError::EmptySourceId);
        }
        let qualification = &candidate.qualification;
        if qualification.calibration_commitment.0 == [0; 32]
            || qualification.risk_model_commitment.0 == [0; 32]
            || qualification.clock_correlation_commitment.0 == [0; 32]
            || qualification.independence_policy_commitment.0 == [0; 32]
        {
            return Err(AssuranceError::MissingQualificationCommitment);
        }
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(AssuranceError::DuplicateCandidateId);
        }
        if candidate.observer_failure_domains.is_empty() {
            return Err(AssuranceError::MissingObserverFailureDomains);
        }
        validate_domains(&candidate.observer_failure_domains)?;

        let observer_domains: BTreeSet<&str> = candidate
            .observer_failure_domains
            .iter()
            .map(String::as_str)
            .collect();
        if !observer_domains.is_disjoint(&execution_domains) {
            return Err(AssuranceError::CommonCauseObserverExecutorOverlap);
        }
        if candidate.expected_residual_risk_upper_bound_bps > MAX_RISK_BPS {
            return Err(AssuranceError::InvalidResidualRiskBound);
        }
        if candidate.expected_residual_risk_upper_bound_bps >= context.current_risk_upper_bound_bps
        {
            return Err(AssuranceError::NoExpectedRiskReduction);
        }

        let mut failed_gates = Vec::new();
        if candidate.expected_residual_risk_upper_bound_bps > budget.maximum_residual_risk_bps {
            failed_gates.push(BudgetGate::ResidualRisk);
        }
        if candidate.latency_upper_bound_ms > budget.maximum_latency_ms {
            failed_gates.push(BudgetGate::Latency);
        }
        if candidate.energy_upper_bound_millijoules > budget.maximum_energy_millijoules {
            failed_gates.push(BudgetGate::Energy);
        }
        if candidate.cost_upper_bound_microcredits > budget.maximum_cost_microcredits {
            failed_gates.push(BudgetGate::Cost);
        }

        if failed_gates.is_empty() {
            feasible.push(candidate.clone());
        } else {
            filtered_out.push(FilteredObservationCandidate {
                candidate_id: candidate.candidate_id.clone(),
                failed_gates,
            });
        }
    }

    filtered_out.sort_by(
        |left: &FilteredObservationCandidate, right: &FilteredObservationCandidate| {
            left.candidate_id.cmp(&right.candidate_id)
        },
    );
    if feasible.is_empty() {
        return Ok(AssuranceDecision::Hold {
            reason: HoldReason::NoCandidateMeetsRiskAndResourceBudgets,
            filtered_out_candidates: filtered_out,
        });
    }

    let mut frontier = Vec::new();
    let mut dominated_candidate_ids = Vec::new();
    for (index, candidate) in feasible.iter().enumerate() {
        let is_dominated = feasible
            .iter()
            .enumerate()
            .any(|(other_index, other)| other_index != index && dominates(other, candidate));
        if is_dominated {
            dominated_candidate_ids.push(candidate.candidate_id.clone());
        } else {
            frontier.push(candidate.clone());
        }
    }
    frontier.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    dominated_candidate_ids.sort();

    Ok(AssuranceDecision::AcquireObservation {
        pareto_frontier: frontier,
        filtered_out_candidates: filtered_out,
        dominated_candidate_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest([byte; 32])
    }

    fn context() -> AssuranceContext {
        AssuranceContext {
            predicate_id: "predicate.object-in-gripper".into(),
            current_risk_upper_bound_bps: 6_000,
            execution_failure_domains: vec![
                "arm-controller".into(),
                "robot-power".into(),
                "vehicle-bus".into(),
            ],
        }
    }

    fn budget() -> ObservationBudget {
        ObservationBudget {
            maximum_residual_risk_bps: 1_000,
            maximum_latency_ms: 500,
            maximum_energy_millijoules: 20_000,
            maximum_cost_microcredits: 10_000,
        }
    }

    fn candidate(
        id: &str,
        risk: u16,
        latency: u64,
        energy: u64,
        cost: u64,
        source: &str,
    ) -> ObservationCandidate {
        ObservationCandidate {
            candidate_id: id.into(),
            predicate_id: "predicate.object-in-gripper".into(),
            source_id: source.into(),
            observer_failure_domains: vec![
                format!("{source}/sensor"),
                format!("{source}/processor"),
                format!("{source}/power"),
            ],
            qualification: ObservationQualification {
                calibration_commitment: digest(1),
                risk_model_commitment: digest(2),
                clock_correlation_commitment: digest(3),
                independence_policy_commitment: digest(4),
            },
            expected_residual_risk_upper_bound_bps: risk,
            latency_upper_bound_ms: latency,
            energy_upper_bound_millijoules: energy,
            cost_upper_bound_microcredits: cost,
        }
    }

    #[test]
    fn a_valid_candidate_is_eligible_when_every_hard_budget_is_met() {
        let option = candidate("gripper-camera", 400, 100, 5_000, 2_000, "camera-a");
        assert_eq!(
            select_observation_frontier(&context(), budget(), std::slice::from_ref(&option)),
            Ok(AssuranceDecision::AcquireObservation {
                pareto_frontier: vec![option],
                filtered_out_candidates: vec![],
                dominated_candidate_ids: vec![],
            })
        );
    }

    #[test]
    fn dominated_option_is_removed_but_independent_tradeoff_is_retained() {
        let best = candidate("a-best", 300, 100, 4_000, 1_000, "sensor-a");
        let dominated = candidate("b-dominated", 500, 200, 5_000, 2_000, "sensor-b");
        let low_latency = candidate("c-low-latency", 600, 50, 3_000, 3_000, "sensor-c");
        assert_eq!(
            select_observation_frontier(
                &context(),
                budget(),
                &[dominated, low_latency.clone(), best.clone()]
            ),
            Ok(AssuranceDecision::AcquireObservation {
                pareto_frontier: vec![best, low_latency],
                filtered_out_candidates: vec![],
                dominated_candidate_ids: vec!["b-dominated".into()],
            })
        );
    }

    #[test]
    fn common_cause_overlap_with_executor_blocks_the_entire_selection() {
        let mut option = candidate("shared-power", 300, 100, 4_000, 1_000, "sensor-a");
        option.observer_failure_domains.push("robot-power".into());
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[option]),
            Err(AssuranceError::CommonCauseObserverExecutorOverlap)
        );
    }

    #[test]
    fn duplicate_observer_failure_domains_are_rejected() {
        let mut option = candidate("duplicate", 300, 100, 4_000, 1_000, "sensor-a");
        option
            .observer_failure_domains
            .push("sensor-a/sensor".into());
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[option]),
            Err(AssuranceError::DuplicateFailureDomain)
        );
    }

    #[test]
    fn option_without_expected_risk_reduction_is_rejected() {
        let option = candidate("no-information", 6_000, 100, 4_000, 1_000, "sensor-a");
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[option]),
            Err(AssuranceError::NoExpectedRiskReduction)
        );
    }

    #[test]
    fn risk_and_resource_budget_failures_hold_without_a_success_claim() {
        let slow = candidate("too-slow", 300, 501, 4_000, 1_000, "sensor-a");
        let risky = candidate("risk-too-high", 1_001, 100, 4_000, 1_000, "sensor-b");
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[slow, risky]),
            Ok(AssuranceDecision::Hold {
                reason: HoldReason::NoCandidateMeetsRiskAndResourceBudgets,
                filtered_out_candidates: vec![
                    FilteredObservationCandidate {
                        candidate_id: "risk-too-high".into(),
                        failed_gates: vec![BudgetGate::ResidualRisk],
                    },
                    FilteredObservationCandidate {
                        candidate_id: "too-slow".into(),
                        failed_gates: vec![BudgetGate::Latency],
                    },
                ],
            })
        );
    }

    #[test]
    fn resource_tradeoffs_are_not_collapsed_into_a_weighted_score() {
        let low_risk = candidate("high-information", 200, 450, 19_000, 9_000, "sensor-a");
        let low_cost = candidate("cheap", 900, 90, 4_000, 100, "sensor-b");
        assert_eq!(
            select_observation_frontier(
                &context(),
                budget(),
                &[low_risk.clone(), low_cost.clone()]
            ),
            Ok(AssuranceDecision::AcquireObservation {
                pareto_frontier: vec![low_cost, low_risk],
                filtered_out_candidates: vec![],
                dominated_candidate_ids: vec![],
            })
        );
    }

    #[test]
    fn empty_candidate_set_holds_for_reconciliation() {
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[]),
            Ok(AssuranceDecision::Hold {
                reason: HoldReason::NoObservationCandidates,
                filtered_out_candidates: vec![],
            })
        );
    }

    #[test]
    fn candidate_must_target_the_same_predicate() {
        let mut option = candidate("wrong-predicate", 300, 100, 4_000, 1_000, "sensor-a");
        option.predicate_id = "predicate.different".into();
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[option]),
            Err(AssuranceError::PredicateMismatch)
        );
    }

    #[test]
    fn zero_commitment_cannot_be_used_as_a_qualification_placeholder() {
        let mut option = candidate("empty-qualification", 300, 100, 4_000, 1_000, "sensor-a");
        option.qualification.clock_correlation_commitment = digest(0);
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[option]),
            Err(AssuranceError::MissingQualificationCommitment)
        );
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let first = candidate("same-id", 300, 100, 4_000, 1_000, "sensor-a");
        let second = candidate("same-id", 400, 90, 3_000, 2_000, "sensor-b");
        assert_eq!(
            select_observation_frontier(&context(), budget(), &[first, second]),
            Err(AssuranceError::DuplicateCandidateId)
        );
    }
}
