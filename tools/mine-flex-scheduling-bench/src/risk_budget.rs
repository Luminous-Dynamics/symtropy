use super::{
    candidate_schedules, enumerate_vectors, evaluate_schedule, independently_reconstruct_score,
    is_lower_hex, sha256, Check, GeneratedCase, ProcessModel, ScheduleScore, Split, generate_cases,
};
use serde::Serialize;
use std::process::ExitCode;

const SUITE_ID: &str = "MINE-FLEX-PARETO-001";
const SUITE_VERSION: &str = "0.1.0";
// Relative headroom above the minimum attainable worst-case carbon across all robust-feasible
// schedules. These are policy declarations, not probabilities or empirical confidence levels.
const CARBON_HEADROOM_BPS: [u32; 6] = [0, 250, 500, 1_000, 2_500, 10_000];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RobustCandidate {
    batches: Vec<u32>,
    worst_case_cost_units: u64,
    worst_case_carbon_units: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FrontierPoint {
    batches: Vec<u32>,
    worst_case_cost_units: u64,
    worst_case_carbon_units: u64,
}

#[derive(Debug, Clone, Serialize)]
struct CaseFrontier {
    seed: u64,
    split: Split,
    robust_feasible_schedule_count: usize,
    minimum_worst_case_carbon_units: Option<u64>,
    nondominated_points: Vec<FrontierPoint>,
}

#[derive(Debug, Clone, Serialize)]
struct BudgetDecision {
    seed: u64,
    split: Split,
    carbon_headroom_basis_points: u32,
    robust_feasible_schedule_count: usize,
    minimum_worst_case_carbon_units: Option<u64>,
    carbon_cap_units: Option<u64>,
    chosen_schedule_batches: Option<Vec<u32>>,
    selected_worst_case_cost_units: Option<u64>,
    selected_worst_case_carbon_units: Option<u64>,
    nondominated_frontier_points: usize,
}

#[derive(Debug, Clone, Serialize)]
struct WorldEvaluation {
    seed: u64,
    split: Split,
    realization_id: String,
    carbon_headroom_basis_points: u32,
    selected_schedule_batches: Option<Vec<u32>>,
    feasible: bool,
    rejection_reason: Option<String>,
    realized_grid_cost_units: Option<u64>,
    realized_grid_carbon_units: Option<u64>,
    evaluator_oracle_grid_cost_units: Option<u64>,
    cost_regret_vs_realization_oracle_units: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct BudgetSummary {
    split: Split,
    carbon_headroom_basis_points: u32,
    cases_evaluated: usize,
    decision_failures: usize,
    worlds_expected: usize,
    worlds_evaluated: usize,
    feasible_worlds: usize,
    feasibility_rate_basis_points: u32,
    hard_constraint_violations: usize,
    mean_realized_cost_units_floor: Option<u64>,
    mean_realized_carbon_units_floor: Option<u64>,
    mean_cost_regret_units_floor: Option<u64>,
    max_selected_worst_case_cost_units: Option<u64>,
    max_selected_worst_case_carbon_units: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct RiskBudgetReceipt {
    result: String,
    suite_id: String,
    suite_version: String,
    source_commit: String,
    generated_case_set_sha256: String,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    carbon_headroom_basis_points: Vec<u32>,
    case_frontier_count: usize,
    budget_decision_count: usize,
    world_evaluation_count: usize,
    decision_failures: usize,
    budget_cap_violations: usize,
    budget_monotonicity_violations: usize,
    independent_hard_constraint_violations: usize,
    summaries: Vec<BudgetSummary>,
    case_frontiers: Vec<CaseFrontier>,
    decisions: Vec<BudgetDecision>,
    world_evaluations: Vec<WorldEvaluation>,
    claim_ceiling: String,
}

fn model() -> ProcessModel {
    ProcessModel {
        electricity_units_per_batch: 2,
        water_units_per_batch: 1,
    }
}

fn compare_candidate(left: &RobustCandidate, right: &RobustCandidate) -> std::cmp::Ordering {
    left.worst_case_cost_units
        .cmp(&right.worst_case_cost_units)
        .then_with(|| left.worst_case_carbon_units.cmp(&right.worst_case_carbon_units))
        .then_with(|| left.batches.cmp(&right.batches))
}

fn enumerate_robust_candidates(
    process: &ProcessModel,
    case: &GeneratedCase,
) -> Check<Vec<RobustCandidate>> {
    let mut candidates = Vec::new();
    for batches in enumerate_vectors(&case.forecast.periods)? {
        if batches.iter().map(|count| u64::from(*count)).sum::<u64>()
            != u64::from(case.forecast.production_target_batches)
        {
            continue;
        }
        let mut worst_cost = 0_u64;
        let mut worst_carbon = 0_u64;
        let mut feasible_in_every_world = true;
        for world in &case.uncertainty_envelope {
            let score = match evaluate_schedule(process, &world.scenario, &batches) {
                Ok(score) => score,
                Err(_) => {
                    feasible_in_every_world = false;
                    break;
                }
            };
            let reconstructed =
                independently_reconstruct_score(process, &world.scenario, &batches)?;
            if reconstructed != score {
                return Err(format!(
                    "independent score reconstruction mismatch for seed {} world {}",
                    case.seed, world.realization_id
                ));
            }
            worst_cost = worst_cost.max(score.total_grid_cost_units);
            worst_carbon = worst_carbon.max(score.total_grid_carbon_units);
        }
        if feasible_in_every_world {
            candidates.push(RobustCandidate {
                batches,
                worst_case_cost_units: worst_cost,
                worst_case_carbon_units: worst_carbon,
            });
        }
    }
    candidates.sort_by(compare_candidate);
    Ok(candidates)
}

fn nondominated_frontier(candidates: &[RobustCandidate]) -> Vec<FrontierPoint> {
    let mut points = candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                if other.batches == candidate.batches {
                    return false;
                }
                let no_worse = other.worst_case_cost_units <= candidate.worst_case_cost_units
                    && other.worst_case_carbon_units <= candidate.worst_case_carbon_units;
                let strictly_better = other.worst_case_cost_units
                    < candidate.worst_case_cost_units
                    || other.worst_case_carbon_units < candidate.worst_case_carbon_units;
                let same_objectives = other.worst_case_cost_units
                    == candidate.worst_case_cost_units
                    && other.worst_case_carbon_units == candidate.worst_case_carbon_units;
                no_worse
                    && (strictly_better
                        || (same_objectives && other.batches < candidate.batches))
            })
        })
        .map(|candidate| FrontierPoint {
            batches: candidate.batches.clone(),
            worst_case_cost_units: candidate.worst_case_cost_units,
            worst_case_carbon_units: candidate.worst_case_carbon_units,
        })
        .collect::<Vec<_>>();
    // If objective pairs tie, retain one canonical schedule so duplicates do not masquerade as
    // separate improvements. Candidate vectors are fully deterministic integer allocations.
    let mut canonical = Vec::new();
    points.sort_by(|left, right| {
        left.worst_case_cost_units
            .cmp(&right.worst_case_cost_units)
            .then_with(|| left.worst_case_carbon_units.cmp(&right.worst_case_carbon_units))
            .then_with(|| left.batches.cmp(&right.batches))
    });
    for point in points {
        if canonical.iter().any(|existing: &FrontierPoint| {
            existing.worst_case_cost_units == point.worst_case_cost_units
                && existing.worst_case_carbon_units == point.worst_case_carbon_units
        }) {
            continue;
        }
        canonical.push(point);
    }
    canonical
}

fn carbon_cap(minimum_worst_case_carbon: u64, headroom_bps: u32) -> Check<u64> {
    let added = (u128::from(minimum_worst_case_carbon) * u128::from(headroom_bps) + 9_999)
        / 10_000;
    let added = u64::try_from(added).map_err(|_| "carbon headroom overflow".to_owned())?;
    minimum_worst_case_carbon
        .checked_add(added)
        .ok_or_else(|| "carbon cap overflow".to_owned())
}

fn mean_floor(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    Some(
        (values.iter().map(|value| u128::from(*value)).sum::<u128>() / values.len() as u128)
            as u64,
    )
}

fn select_under_cap<'a>(
    candidates: &'a [RobustCandidate],
    carbon_cap_units: u64,
) -> Option<&'a RobustCandidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.worst_case_carbon_units <= carbon_cap_units)
        .min_by(|left, right| compare_candidate(left, right))
}

fn make_receipt(source_commit: &str) -> Check<RiskBudgetReceipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a lowercase 40-character hex SHA".to_owned());
    }
    let cases = generate_cases();
    let manifest_bytes = serde_json::to_vec(&cases)
        .map_err(|error| format!("cannot serialize case manifest: {error}"))?;
    let process = model();
    let mut case_frontiers = Vec::new();
    let mut decisions = Vec::new();
    let mut world_evaluations = Vec::new();
    let mut decision_failures = 0_usize;
    let mut budget_cap_violations = 0_usize;
    let mut independent_hard_constraint_violations = 0_usize;

    for case in &cases {
        let candidates = enumerate_robust_candidates(&process, case)?;
        let minimum_carbon = candidates
            .iter()
            .map(|candidate| candidate.worst_case_carbon_units)
            .min();
        let frontier = nondominated_frontier(&candidates);
        case_frontiers.push(CaseFrontier {
            seed: case.seed,
            split: case.split,
            robust_feasible_schedule_count: candidates.len(),
            minimum_worst_case_carbon_units: minimum_carbon,
            nondominated_points: frontier.clone(),
        });

        for headroom_bps in CARBON_HEADROOM_BPS {
            let cap = match minimum_carbon {
                Some(minimum) => Some(carbon_cap(minimum, headroom_bps)?),
                None => None,
            };
            let selected = cap.and_then(|cap| select_under_cap(&candidates, cap));
            if selected.is_none() {
                decision_failures += 1;
            }
            if let (Some(cap), Some(selected)) = (cap, selected) {
                if selected.worst_case_carbon_units > cap
                    || !frontier.iter().any(|point| point.batches == selected.batches)
                {
                    budget_cap_violations += 1;
                }
            }
            decisions.push(BudgetDecision {
                seed: case.seed,
                split: case.split,
                carbon_headroom_basis_points: headroom_bps,
                robust_feasible_schedule_count: candidates.len(),
                minimum_worst_case_carbon_units: minimum_carbon,
                carbon_cap_units: cap,
                chosen_schedule_batches: selected.map(|candidate| candidate.batches.clone()),
                selected_worst_case_cost_units: selected
                    .map(|candidate| candidate.worst_case_cost_units),
                selected_worst_case_carbon_units: selected
                    .map(|candidate| candidate.worst_case_carbon_units),
                nondominated_frontier_points: frontier.len(),
            });

            let Some(selected) = selected else {
                continue;
            };
            for world in &case.uncertainty_envelope {
                let evaluation = match evaluate_schedule(
                    &process,
                    &world.scenario,
                    &selected.batches,
                )
                {
                    Ok(score) => {
                        let reconstructed = independently_reconstruct_score(
                            &process,
                            &world.scenario,
                            &selected.batches,
                        );
                        match reconstructed {
                            Ok(independent) if independent == score => WorldEvaluation {
                                seed: case.seed,
                                split: case.split,
                                realization_id: world.realization_id.clone(),
                                carbon_headroom_basis_points: headroom_bps,
                                selected_schedule_batches: Some(selected.batches.clone()),
                                feasible: true,
                                rejection_reason: None,
                                realized_grid_cost_units: Some(score.total_grid_cost_units),
                                realized_grid_carbon_units: Some(score.total_grid_carbon_units),
                                evaluator_oracle_grid_cost_units: None,
                                cost_regret_vs_realization_oracle_units: None,
                            },
                            _ => {
                                independent_hard_constraint_violations += 1;
                                WorldEvaluation {
                                    seed: case.seed,
                                    split: case.split,
                                    realization_id: world.realization_id.clone(),
                                    carbon_headroom_basis_points: headroom_bps,
                                    selected_schedule_batches: Some(selected.batches.clone()),
                                    feasible: false,
                                    rejection_reason: Some(
                                        "INDEPENDENT_RECONSTRUCTION_MISMATCH".to_owned(),
                                    ),
                                    realized_grid_cost_units: None,
                                    realized_grid_carbon_units: None,
                                    evaluator_oracle_grid_cost_units: None,
                                    cost_regret_vs_realization_oracle_units: None,
                                }
                            }
                        }
                    }
                    Err(_) => {
                        independent_hard_constraint_violations += 1;
                        WorldEvaluation {
                            seed: case.seed,
                            split: case.split,
                            realization_id: world.realization_id.clone(),
                            carbon_headroom_basis_points: headroom_bps,
                            selected_schedule_batches: Some(selected.batches.clone()),
                            feasible: false,
                            rejection_reason: Some(
                                "ROBUST_SCHEDULE_FAILED_REALIZED_WORLD".to_owned(),
                            ),
                            realized_grid_cost_units: None,
                            realized_grid_carbon_units: None,
                            evaluator_oracle_grid_cost_units: None,
                            cost_regret_vs_realization_oracle_units: None,
                        }
                    }
                };
                let mut evaluation = evaluation;
                if evaluation.feasible {
                    let oracle = candidate_schedules(&process, &world.scenario)?
                        .into_iter()
                        .min_by(|left, right| {
                            left.total_grid_cost_units
                                .cmp(&right.total_grid_cost_units)
                                .then_with(|| {
                                    left.total_grid_carbon_units.cmp(&right.total_grid_carbon_units)
                                })
                                .then_with(|| left.batches.cmp(&right.batches))
                        });
                    if let Some(oracle) = oracle {
                        evaluation.evaluator_oracle_grid_cost_units =
                            Some(oracle.total_grid_cost_units);
                        evaluation.cost_regret_vs_realization_oracle_units = evaluation
                            .realized_grid_cost_units
                            .map(|cost| cost.saturating_sub(oracle.total_grid_cost_units));
                    }
                }
                world_evaluations.push(evaluation);
            }
        }
    }

    let mut summaries = Vec::new();
    for split in [Split::Validation, Split::HeldOut] {
        for headroom_bps in CARBON_HEADROOM_BPS {
            let case_decisions: Vec<&BudgetDecision> = decisions
                .iter()
                .filter(|decision| {
                    decision.split == split
                        && decision.carbon_headroom_basis_points == headroom_bps
                })
                .collect();
            let worlds: Vec<&WorldEvaluation> = world_evaluations
                .iter()
                .filter(|world| {
                    world.split == split && world.carbon_headroom_basis_points == headroom_bps
                })
                .collect();
            let feasible_worlds = worlds.iter().filter(|world| world.feasible).count();
            let local_violations = worlds
                .iter()
                .filter(|world| !world.feasible)
                .count();
            let costs = worlds
                .iter()
                .filter_map(|world| world.realized_grid_cost_units)
                .collect::<Vec<_>>();
            let carbons = worlds
                .iter()
                .filter_map(|world| world.realized_grid_carbon_units)
                .collect::<Vec<_>>();
            let regrets = worlds
                .iter()
                .filter_map(|world| world.cost_regret_vs_realization_oracle_units)
                .collect::<Vec<_>>();
            summaries.push(BudgetSummary {
                split,
                carbon_headroom_basis_points: headroom_bps,
                cases_evaluated: case_decisions.len(),
                decision_failures: case_decisions
                    .iter()
                    .filter(|decision| decision.chosen_schedule_batches.is_none())
                    .count(),
                worlds_expected: case_decisions.len() * 5,
                worlds_evaluated: worlds.len(),
                feasible_worlds,
                feasibility_rate_basis_points: if worlds.is_empty() {
                    0
                } else {
                    ((feasible_worlds as u128 * 10_000) / worlds.len() as u128) as u32
                },
                hard_constraint_violations: local_violations,
                mean_realized_cost_units_floor: mean_floor(&costs),
                mean_realized_carbon_units_floor: mean_floor(&carbons),
                mean_cost_regret_units_floor: mean_floor(&regrets),
                max_selected_worst_case_cost_units: case_decisions
                    .iter()
                    .filter_map(|decision| decision.selected_worst_case_cost_units)
                    .max(),
                max_selected_worst_case_carbon_units: case_decisions
                    .iter()
                    .filter_map(|decision| decision.selected_worst_case_carbon_units)
                    .max(),
            });
        }
    }

    let mut budget_monotonicity_violations = 0_usize;
    for case in &cases {
        let rows: Vec<&BudgetDecision> = decisions
            .iter()
            .filter(|decision| decision.seed == case.seed && decision.split == case.split)
            .collect();
        for pair in rows.windows(2) {
            match (
                pair[0].selected_worst_case_cost_units,
                pair[1].selected_worst_case_cost_units,
                pair[0].carbon_cap_units,
                pair[1].carbon_cap_units,
            ) {
                (Some(left_cost), Some(right_cost), Some(left_cap), Some(right_cap))
                    if right_cost <= left_cost && right_cap >= left_cap => {}
                _ => budget_monotonicity_violations += 1,
            }
        }
    }

    let expected_case_count = cases.len();
    let expected_decision_count = cases.len() * CARBON_HEADROOM_BPS.len();
    let expected_world_count = expected_decision_count * 5;
    let all_worlds_feasible = world_evaluations.len() == expected_world_count
        && world_evaluations.iter().all(|world| world.feasible);
    let all_summaries_consistent = summaries.iter().all(|summary| {
        summary.cases_evaluated == 32
            && summary.decision_failures == 0
            && summary.worlds_expected == 160
            && summary.worlds_evaluated == 160
            && summary.feasible_worlds == 160
            && summary.feasibility_rate_basis_points == 10_000
            && summary.hard_constraint_violations == 0
    });
    let result = if expected_case_count == 64
        && case_frontiers.len() == expected_case_count
        && decisions.len() == expected_decision_count
        && decision_failures == 0
        && budget_cap_violations == 0
        && budget_monotonicity_violations == 0
        && independent_hard_constraint_violations == 0
        && all_worlds_feasible
        && all_summaries_consistent
        && case_frontiers.iter().all(|frontier| !frontier.nondominated_points.is_empty())
    {
        "validated"
    } else {
        "control_mismatch"
    };

    Ok(RiskBudgetReceipt {
        result: result.to_owned(),
        suite_id: SUITE_ID.to_owned(),
        suite_version: SUITE_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        generated_case_set_sha256: sha256(&manifest_bytes),
        validation_seed_count: 32,
        held_out_seed_count: 32,
        carbon_headroom_basis_points: CARBON_HEADROOM_BPS.to_vec(),
        case_frontier_count: case_frontiers.len(),
        budget_decision_count: decisions.len(),
        world_evaluation_count: world_evaluations.len(),
        decision_failures,
        budget_cap_violations,
        budget_monotonicity_violations,
        independent_hard_constraint_violations,
        summaries,
        case_frontiers,
        decisions,
        world_evaluations,
        claim_ceiling: "Exact static robust epsilon-constraint scheduling on a finite, public, authored synthetic envelope. A schedule is chosen using the declared envelope only and is evaluated on paired validation/held-out worlds. Units are arbitrary synthetic integers; this is not a site-calibrated dispatch controller, a probability guarantee, or evidence of real-mine cost/carbon/safety outcomes.".to_owned(),
    })
}

pub(super) fn run_cli(args: &[String]) -> Check<ExitCode> {
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: mine-flex-scheduling-bench --risk-budget --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize risk-budget receipt: {error}"))?
    );
    Ok(if receipt.result == "validated" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_budget_frontier_is_deterministic_and_fail_closed() {
        let first =
            make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("valid receipt");
        let second =
            make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("deterministic replay");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        assert_eq!("validated", first.result);
        assert_eq!(64, first.case_frontier_count);
        assert_eq!(384, first.budget_decision_count);
        assert_eq!(1_920, first.world_evaluation_count);
        assert_eq!(0, first.decision_failures);
        assert_eq!(0, first.budget_cap_violations);
        assert_eq!(0, first.budget_monotonicity_violations);
        assert_eq!(0, first.independent_hard_constraint_violations);
        assert!(make_receipt("not-a-commit").is_err());
    }

    #[test]
    fn widening_the_carbon_cap_never_increases_minimum_worst_case_cost() {
        let process = model();
        for case in super::super::generate_cases() {
            let candidates = enumerate_robust_candidates(&process, &case).expect("candidates");
            let minimum_carbon = candidates
                .iter()
                .map(|candidate| candidate.worst_case_carbon_units)
                .min()
                .expect("at least one robust schedule");
            let mut prior_cost = None;
            for headroom in CARBON_HEADROOM_BPS {
                let cap = carbon_cap(minimum_carbon, headroom).expect("finite cap");
                let selected = select_under_cap(&candidates, cap).expect("budget feasible");
                if let Some(previous) = prior_cost {
                    assert!(selected.worst_case_cost_units <= previous);
                }
                assert!(selected.worst_case_carbon_units <= cap);
                prior_cost = Some(selected.worst_case_cost_units);
            }
        }
    }

    #[test]
    fn published_frontier_contains_no_strictly_dominated_objective_pair() {
        let process = model();
        for case in super::super::generate_cases() {
            let candidates = enumerate_robust_candidates(&process, &case).expect("candidates");
            let frontier = nondominated_frontier(&candidates);
            assert!(!frontier.is_empty());
            for candidate in &frontier {
                for other in &frontier {
                    if candidate.batches == other.batches {
                        continue;
                    }
                    assert!(
                        !(other.worst_case_cost_units <= candidate.worst_case_cost_units
                            && other.worst_case_carbon_units <= candidate.worst_case_carbon_units
                            && (other.worst_case_cost_units < candidate.worst_case_cost_units
                                || other.worst_case_carbon_units
                                    < candidate.worst_case_carbon_units)),
                        "frontier contains dominated point for seed {}",
                        case.seed
                    );
                }
            }
        }
    }
}
