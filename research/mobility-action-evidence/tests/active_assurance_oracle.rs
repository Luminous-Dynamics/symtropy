//! Independent reference checks for the active-assurance Pareto selector.
//!
//! This oracle intentionally recomputes feasibility and dominance using a
//! separate, simple formulation. It does not call production helpers.

use mobility_action_evidence::Digest;
use mobility_action_evidence::active_assurance::{
    AssuranceContext, AssuranceDecision, ObservationBudget, ObservationCandidate,
    ObservationQualification, select_observation_frontier,
};

fn digest(value: u8) -> Digest {
    Digest([value; 32])
}

fn option(id: &str, risk: u16, latency: u64, energy: u64, cost: u64) -> ObservationCandidate {
    ObservationCandidate {
        candidate_id: id.into(),
        predicate_id: "object-secured".into(),
        source_id: format!("observer-{id}"),
        observer_failure_domains: vec![
            format!("observer-{id}/sensor"),
            format!("observer-{id}/processor"),
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

fn reference_frontier(
    current_risk: u16,
    budget: ObservationBudget,
    candidates: &[ObservationCandidate],
) -> Vec<String> {
    let feasible: Vec<&ObservationCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.expected_residual_risk_upper_bound_bps < current_risk
                && candidate.expected_residual_risk_upper_bound_bps
                    <= budget.maximum_residual_risk_bps
                && candidate.latency_upper_bound_ms <= budget.maximum_latency_ms
                && candidate.energy_upper_bound_millijoules <= budget.maximum_energy_millijoules
                && candidate.cost_upper_bound_microcredits <= budget.maximum_cost_microcredits
        })
        .collect();

    let mut frontier = Vec::new();
    for candidate in &feasible {
        let mut has_better_or_equal = false;
        let mut has_strict_advantage = false;
        for other in &feasible {
            if other.candidate_id == candidate.candidate_id {
                continue;
            }
            let coordinates_other = [
                u64::from(other.expected_residual_risk_upper_bound_bps),
                other.latency_upper_bound_ms,
                other.energy_upper_bound_millijoules,
                other.cost_upper_bound_microcredits,
            ];
            let coordinates_candidate = [
                u64::from(candidate.expected_residual_risk_upper_bound_bps),
                candidate.latency_upper_bound_ms,
                candidate.energy_upper_bound_millijoules,
                candidate.cost_upper_bound_microcredits,
            ];
            let coordinatewise_no_worse = coordinates_other
                .iter()
                .zip(coordinates_candidate.iter())
                .all(|(left, right)| left <= right);
            let some_coordinate_better = coordinates_other
                .iter()
                .zip(coordinates_candidate.iter())
                .any(|(left, right)| left < right);
            has_better_or_equal = coordinatewise_no_worse;
            has_strict_advantage = some_coordinate_better;
            if has_better_or_equal && has_strict_advantage {
                break;
            }
            has_better_or_equal = false;
            has_strict_advantage = false;
        }
        if !(has_better_or_equal && has_strict_advantage) {
            frontier.push(candidate.candidate_id.clone());
        }
    }
    frontier.sort();
    frontier
}

fn context(current_risk: u16) -> AssuranceContext {
    AssuranceContext {
        predicate_id: "object-secured".into(),
        current_risk_upper_bound_bps: current_risk,
        execution_failure_domains: vec![
            "arm-controller".into(),
            "robot-power".into(),
            "vehicle-bus".into(),
        ],
    }
}

#[test]
fn active_assurance_frontier_matches_independent_reference_matrix() {
    let budget = ObservationBudget {
        maximum_residual_risk_bps: 1_000,
        maximum_latency_ms: 500,
        maximum_energy_millijoules: 20_000,
        maximum_cost_microcredits: 10_000,
    };

    let matrix = vec![
        vec![
            option("a", 250, 400, 18_000, 9_000),
            option("b", 400, 100, 5_000, 2_000),
            option("c", 600, 50, 4_000, 1_000),
            option("d", 700, 200, 8_000, 4_000),
        ],
        vec![
            option("a", 900, 480, 19_000, 9_500),
            option("b", 800, 490, 18_500, 9_000),
            option("c", 700, 300, 8_000, 2_000),
            option("d", 1_100, 100, 2_000, 100),
        ],
        vec![
            option("a", 100, 500, 20_000, 10_000),
            option("b", 100, 100, 20_000, 10_000),
            option("c", 100, 100, 10_000, 10_000),
            option("d", 100, 100, 10_000, 1_000),
        ],
        vec![
            option("a", 500, 100, 5_000, 5_000),
            option("b", 500, 100, 5_000, 1_000),
            option("c", 500, 100, 5_000, 1_000),
            option("d", 950, 450, 19_000, 9_900),
        ],
    ];

    for candidates in matrix {
        let expected = reference_frontier(6_000, budget, &candidates);
        let actual = select_observation_frontier(&context(6_000), budget, &candidates)
            .expect("valid observation set");
        let actual_ids = match actual {
            AssuranceDecision::AcquireObservation {
                pareto_frontier, ..
            } => pareto_frontier
                .iter()
                .map(|candidate| candidate.candidate_id.clone())
                .collect::<Vec<_>>(),
            AssuranceDecision::Hold { .. } => Vec::new(),
        };
        assert_eq!(actual_ids, expected);
    }
}

#[test]
fn no_feasible_option_matches_reference_hold() {
    let budget = ObservationBudget {
        maximum_residual_risk_bps: 100,
        maximum_latency_ms: 10,
        maximum_energy_millijoules: 100,
        maximum_cost_microcredits: 100,
    };
    let candidates = vec![
        option("slow", 50, 100, 10, 10),
        option("risky", 500, 10, 10, 10),
    ];
    let expected = reference_frontier(1_000, budget, &candidates);
    assert!(expected.is_empty());
    assert_eq!(
        select_observation_frontier(&context(1_000), budget, &candidates),
        Ok(AssuranceDecision::Hold {
            reason: mobility_action_evidence::active_assurance::HoldReason::NoCandidateMeetsRiskAndResourceBudgets,
            filtered_out_candidates: vec![
                mobility_action_evidence::active_assurance::FilteredObservationCandidate {
                    candidate_id: "risky".into(),
                    failed_gates: vec![
                        mobility_action_evidence::active_assurance::BudgetGate::ResidualRisk,
                    ],
                },
                mobility_action_evidence::active_assurance::FilteredObservationCandidate {
                    candidate_id: "slow".into(),
                    failed_gates: vec![
                        mobility_action_evidence::active_assurance::BudgetGate::Latency,
                    ],
                },
            ],
        })
    );
}
