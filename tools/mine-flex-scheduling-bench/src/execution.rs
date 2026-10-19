use super::{
    candidate_schedules, enumerate_vectors, evaluate_schedule, independently_reconstruct_score,
    is_lower_hex, sha256, Check, GeneratedCase, ProcessModel, ScheduleScore, Split,
    WORLDS_PER_SEED, generate_cases,
};
use serde::Serialize;
use std::process::ExitCode;

const SUITE_ID: &str = "MINE-FLEX-EXEC-001";
const SUITE_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExecutionEnvelope {
    max_ramp_up_batches_per_period: u32,
    max_ramp_down_batches_per_period: u32,
    minimum_up_periods_after_start: usize,
    maximum_starts_per_horizon: usize,
}

fn execution_envelope() -> ExecutionEnvelope {
    ExecutionEnvelope {
        max_ramp_up_batches_per_period: 1,
        max_ramp_down_batches_per_period: 1,
        minimum_up_periods_after_start: 2,
        maximum_starts_per_horizon: 1,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExecutionViolation {
    code: String,
    period_index: Option<usize>,
    observed_batches: Option<u32>,
    limit_batches: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExecutionAudit {
    executable: bool,
    startup_count: usize,
    violations: Vec<ExecutionViolation>,
}

fn violation(
    code: &str,
    period_index: Option<usize>,
    observed_batches: Option<u32>,
    limit_batches: Option<u32>,
) -> ExecutionViolation {
    ExecutionViolation {
        code: code.to_owned(),
        period_index,
        observed_batches,
        limit_batches,
    }
}

// This checker is intentionally separate from the resource-accounting optimizer: a schedule can
// satisfy the electricity/water ledger and still violate a physical transition constraint.
fn audit_execution(
    batches: &[u32],
    envelope: &ExecutionEnvelope,
) -> ExecutionAudit {
    let mut violations = Vec::new();
    let mut startup_count = 0_usize;
    if batches.is_empty() || envelope.minimum_up_periods_after_start == 0 {
        violations.push(violation("MALFORMED_EXECUTION_PROFILE", None, None, None));
        return ExecutionAudit {
            executable: false,
            startup_count,
            violations,
        };
    }

    for index in 1..batches.len() {
        let previous = batches[index - 1];
        let current = batches[index];
        if current.saturating_sub(previous) > envelope.max_ramp_up_batches_per_period {
            violations.push(violation(
                "RAMP_UP_EXCEEDED",
                Some(index),
                Some(current - previous),
                Some(envelope.max_ramp_up_batches_per_period),
            ));
        }
        if previous.saturating_sub(current) > envelope.max_ramp_down_batches_per_period {
            violations.push(violation(
                "RAMP_DOWN_EXCEEDED",
                Some(index),
                Some(previous - current),
                Some(envelope.max_ramp_down_batches_per_period),
            ));
        }
    }

    for index in 0..batches.len() {
        let previous = if index == 0 { 0 } else { batches[index - 1] };
        if previous == 0 && batches[index] > 0 {
            startup_count += 1;
            let required_end = index.saturating_add(envelope.minimum_up_periods_after_start);
            if required_end > batches.len() {
                violations.push(violation(
                    "MINIMUM_UPTIME_EXCEEDS_HORIZON",
                    Some(index),
                    Some(batches[index]),
                    Some(envelope.minimum_up_periods_after_start as u32),
                ));
            } else if batches[index..required_end].contains(&0) {
                violations.push(violation(
                    "MINIMUM_UPTIME_NOT_MET",
                    Some(index),
                    Some(batches[index]),
                    Some(envelope.minimum_up_periods_after_start as u32),
                ));
            }
        }
    }

    if startup_count > envelope.maximum_starts_per_horizon {
        violations.push(violation(
            "STARTUP_LIMIT_EXCEEDED",
            None,
            Some(u32::try_from(startup_count).unwrap_or(u32::MAX)),
            Some(u32::try_from(envelope.maximum_starts_per_horizon).unwrap_or(u32::MAX)),
        ));
    }

    ExecutionAudit {
        executable: violations.is_empty(),
        startup_count,
        violations,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RobustCandidate {
    batches: Vec<u32>,
    worst_case_cost_units: u64,
    worst_case_carbon_units: u64,
}

fn candidate_order(left: &RobustCandidate, right: &RobustCandidate) -> std::cmp::Ordering {
    left.worst_case_cost_units
        .cmp(&right.worst_case_cost_units)
        .then_with(|| left.worst_case_carbon_units.cmp(&right.worst_case_carbon_units))
        .then_with(|| left.batches.cmp(&right.batches))
}

fn model() -> ProcessModel {
    ProcessModel {
        electricity_units_per_batch: 2,
        water_units_per_batch: 1,
    }
}

fn robust_candidates(process: &ProcessModel, case: &GeneratedCase) -> Check<Vec<RobustCandidate>> {
    let mut candidates = Vec::new();
    for batches in enumerate_vectors(&case.forecast.periods)? {
        if batches.iter().map(|count| u64::from(*count)).sum::<u64>()
            != u64::from(case.forecast.production_target_batches)
        {
            continue;
        }

        let mut worst_case_cost_units = 0_u64;
        let mut worst_case_carbon_units = 0_u64;
        let mut feasible_everywhere = true;
        for world in &case.uncertainty_envelope {
            let score = match evaluate_schedule(process, &world.scenario, &batches) {
                Ok(score) => score,
                Err(_) => {
                    feasible_everywhere = false;
                    break;
                }
            };
            let independent =
                independently_reconstruct_score(process, &world.scenario, &batches)?;
            if independent != score {
                return Err(format!(
                    "independent resource reconstruction mismatch for seed {} world {}",
                    case.seed, world.realization_id
                ));
            }
            worst_case_cost_units = worst_case_cost_units.max(score.total_grid_cost_units);
            worst_case_carbon_units =
                worst_case_carbon_units.max(score.total_grid_carbon_units);
        }
        if feasible_everywhere {
            candidates.push(RobustCandidate {
                batches,
                worst_case_cost_units,
                worst_case_carbon_units,
            });
        }
    }
    candidates.sort_by(candidate_order);
    Ok(candidates)
}

#[derive(Debug, Clone, Serialize)]
struct CaseDecision {
    seed: u64,
    split: Split,
    resource_robust_candidate_count: usize,
    execution_robust_candidate_count: usize,
    resource_only_min_cost_batches: Vec<u32>,
    resource_only_worst_case_cost_units: u64,
    resource_only_execution_audit: ExecutionAudit,
    execution_constrained_batches: Vec<u32>,
    execution_constrained_worst_case_cost_units: u64,
    execution_constrained_worst_case_carbon_units: u64,
    execution_constrained_execution_audit: ExecutionAudit,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyWorldOutcome {
    policy_id: String,
    schedule_batches: Vec<u32>,
    resource_feasible: bool,
    execution_audit: ExecutionAudit,
    grid_cost_units: Option<u64>,
    grid_carbon_units: Option<u64>,
    oracle_grid_cost_units: Option<u64>,
    cost_regret_vs_realization_oracle_units: Option<u64>,
    rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WorldEvaluation {
    seed: u64,
    split: Split,
    realization_id: String,
    resource_only: PolicyWorldOutcome,
    execution_constrained: PolicyWorldOutcome,
}

#[derive(Debug, Clone, Serialize)]
struct PolicySummary {
    split: Split,
    policy_id: String,
    evaluated_worlds: usize,
    resource_feasible_worlds: usize,
    physically_executable_worlds: usize,
    execution_violation_worlds: usize,
    feasibility_rate_basis_points: u32,
    mean_realized_cost_units_floor: Option<u64>,
    mean_realized_carbon_units_floor: Option<u64>,
    mean_cost_regret_units_floor: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionReceipt {
    result: String,
    suite_id: String,
    suite_version: String,
    source_commit: String,
    generated_case_set_sha256: String,
    execution_envelope: ExecutionEnvelope,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    case_count: usize,
    world_evaluation_count: usize,
    resource_only_optimum_rejected_by_execution_cases: usize,
    execution_constrained_cases: usize,
    execution_constrained_non_executable_cases: usize,
    execution_constrained_non_executable_worlds: usize,
    independent_resource_accounting_violations: usize,
    summaries: Vec<PolicySummary>,
    case_decisions: Vec<CaseDecision>,
    world_evaluations: Vec<WorldEvaluation>,
    claim_ceiling: String,
}

fn summarize(
    worlds: &[WorldEvaluation],
    split: Split,
    policy_id: &str,
) -> PolicySummary {
    let outcomes = worlds
        .iter()
        .filter(|world| world.split == split)
        .map(|world| {
            if policy_id == "resource_only_robust_min_cost" {
                &world.resource_only
            } else {
                &world.execution_constrained
            }
        })
        .collect::<Vec<_>>();
    let resource_feasible_worlds = outcomes
        .iter()
        .filter(|outcome| outcome.resource_feasible)
        .count();
    let physically_executable_worlds = outcomes
        .iter()
        .filter(|outcome| outcome.execution_audit.executable)
        .count();
    let execution_violation_worlds = outcomes
        .iter()
        .filter(|outcome| !outcome.execution_audit.executable)
        .count();
    let costs = outcomes
        .iter()
        .filter_map(|outcome| outcome.grid_cost_units)
        .collect::<Vec<_>>();
    let carbons = outcomes
        .iter()
        .filter_map(|outcome| outcome.grid_carbon_units)
        .collect::<Vec<_>>();
    let regrets = outcomes
        .iter()
        .filter_map(|outcome| outcome.cost_regret_vs_realization_oracle_units)
        .collect::<Vec<_>>();
    PolicySummary {
        split,
        policy_id: policy_id.to_owned(),
        evaluated_worlds: outcomes.len(),
        resource_feasible_worlds,
        physically_executable_worlds,
        execution_violation_worlds,
        feasibility_rate_basis_points: if outcomes.is_empty() {
            0
        } else {
            ((physically_executable_worlds as u128 * 10_000) / outcomes.len() as u128) as u32
        },
        mean_realized_cost_units_floor: mean_floor(&costs),
        mean_realized_carbon_units_floor: mean_floor(&carbons),
        mean_cost_regret_units_floor: mean_floor(&regrets),
    }
}

fn mean_floor(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        None
    } else {
        Some(
            (values.iter().map(|value| u128::from(*value)).sum::<u128>()
                / values.len() as u128) as u64,
        )
    }
}

fn realized_outcome(
    process: &ProcessModel,
    world: &super::GeneratedWorld,
    seed: u64,
    split: Split,
    candidate: &RobustCandidate,
    policy_id: &str,
    execution_profile: &ExecutionEnvelope,
    independent_resource_accounting_violations: &mut usize,
) -> Check<PolicyWorldOutcome> {
    let execution_audit = audit_execution(&candidate.batches, execution_profile);
    let oracle_cost = candidate_schedules(process, &world.scenario)?
        .into_iter()
        .min_by(|left, right| {
            left.total_grid_cost_units
                .cmp(&right.total_grid_cost_units)
                .then_with(|| left.total_grid_carbon_units.cmp(&right.total_grid_carbon_units))
                .then_with(|| left.batches.cmp(&right.batches))
        })
        .map(|score| score.total_grid_cost_units);

    match evaluate_schedule(process, &world.scenario, &candidate.batches) {
        Ok(score) => {
            let independent =
                independently_reconstruct_score(process, &world.scenario, &candidate.batches)?;
            if independent != score {
                *independent_resource_accounting_violations += 1;
                return Ok(PolicyWorldOutcome {
                    policy_id: policy_id.to_owned(),
                    schedule_batches: candidate.batches.clone(),
                    resource_feasible: false,
                    execution_audit,
                    grid_cost_units: None,
                    grid_carbon_units: None,
                    oracle_grid_cost_units: oracle_cost,
                    cost_regret_vs_realization_oracle_units: None,
                    rejection_reason: Some("INDEPENDENT_RESOURCE_RECONSTRUCTION_MISMATCH".to_owned()),
                });
            }
            Ok(PolicyWorldOutcome {
                policy_id: policy_id.to_owned(),
                schedule_batches: candidate.batches.clone(),
                resource_feasible: true,
                execution_audit,
                grid_cost_units: Some(score.total_grid_cost_units),
                grid_carbon_units: Some(score.total_grid_carbon_units),
                oracle_grid_cost_units: oracle_cost,
                cost_regret_vs_realization_oracle_units: oracle_cost
                    .map(|oracle| score.total_grid_cost_units.saturating_sub(oracle)),
                rejection_reason: None,
            })
        }
        Err(_) => {
            *independent_resource_accounting_violations += 1;
            Ok(PolicyWorldOutcome {
                policy_id: policy_id.to_owned(),
                schedule_batches: candidate.batches.clone(),
                resource_feasible: false,
                execution_audit,
                grid_cost_units: None,
                grid_carbon_units: None,
                oracle_grid_cost_units: oracle_cost,
                cost_regret_vs_realization_oracle_units: None,
                rejection_reason: Some("ROBUST_SCHEDULE_NOT_FEASIBLE_IN_REALIZED_WORLD".to_owned()),
            })
        }
    }
}

fn make_receipt(source_commit: &str) -> Check<ExecutionReceipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a lowercase 40-character hex SHA".to_owned());
    }
    let cases = generate_cases();
    let manifest_bytes =
        serde_json::to_vec(&cases).map_err(|error| format!("cannot serialize case manifest: {error}"))?;
    let process = model();
    let profile = execution_envelope();
    let mut decisions = Vec::new();
    let mut worlds = Vec::new();
    let mut independent_resource_accounting_violations = 0_usize;
    let mut resource_only_optimum_rejected_by_execution_cases = 0_usize;
    let mut execution_constrained_non_executable_cases = 0_usize;
    let mut execution_constrained_non_executable_worlds = 0_usize;

    for case in &cases {
        let candidates = robust_candidates(&process, case)?;
        let resource_only = candidates
            .first()
            .ok_or_else(|| format!("no resource-robust schedule for seed {}", case.seed))?;
        let executable_candidates = candidates
            .iter()
            .filter(|candidate| audit_execution(&candidate.batches, &profile).executable)
            .collect::<Vec<_>>();
        let execution_constrained = executable_candidates.first().ok_or_else(|| {
            format!(
                "no physically executable robust schedule under declared envelope for seed {}",
                case.seed
            )
        })?;

        let resource_only_execution_audit = audit_execution(&resource_only.batches, &profile);
        if !resource_only_execution_audit.executable {
            resource_only_optimum_rejected_by_execution_cases += 1;
        }
        let execution_constrained_execution_audit =
            audit_execution(&execution_constrained.batches, &profile);
        if !execution_constrained_execution_audit.executable {
            execution_constrained_non_executable_cases += 1;
        }

        decisions.push(CaseDecision {
            seed: case.seed,
            split: case.split,
            resource_robust_candidate_count: candidates.len(),
            execution_robust_candidate_count: executable_candidates.len(),
            resource_only_min_cost_batches: resource_only.batches.clone(),
            resource_only_worst_case_cost_units: resource_only.worst_case_cost_units,
            resource_only_execution_audit,
            execution_constrained_batches: execution_constrained.batches.clone(),
            execution_constrained_worst_case_cost_units: execution_constrained.worst_case_cost_units,
            execution_constrained_worst_case_carbon_units: execution_constrained.worst_case_carbon_units,
            execution_constrained_execution_audit,
        });

        for world in &case.uncertainty_envelope {
            let resource_outcome = realized_outcome(
                &process,
                world,
                case.seed,
                case.split,
                resource_only,
                "resource_only_robust_min_cost",
                &profile,
                &mut independent_resource_accounting_violations,
            )?;
            let constrained_outcome = realized_outcome(
                &process,
                world,
                case.seed,
                case.split,
                execution_constrained,
                "execution_constrained_robust_min_cost",
                &profile,
                &mut independent_resource_accounting_violations,
            )?;
            if !constrained_outcome.execution_audit.executable {
                execution_constrained_non_executable_worlds += 1;
            }
            worlds.push(WorldEvaluation {
                seed: case.seed,
                split: case.split,
                realization_id: world.realization_id.clone(),
                resource_only: resource_outcome,
                execution_constrained: constrained_outcome,
            });
        }
    }

    let summaries = [
        summarize(&worlds, Split::Validation, "resource_only_robust_min_cost"),
        summarize(&worlds, Split::HeldOut, "resource_only_robust_min_cost"),
        summarize(&worlds, Split::Validation, "execution_constrained_robust_min_cost"),
        summarize(&worlds, Split::HeldOut, "execution_constrained_robust_min_cost"),
    ];
    let expected_worlds = cases.len() * WORLDS_PER_SEED;
    let all_resource_worlds_feasible = worlds.iter().all(|world| {
        world.resource_only.resource_feasible && world.execution_constrained.resource_feasible
    });
    let all_selected_executions_valid = decisions.iter().all(|decision| {
        decision.execution_constrained_execution_audit.executable
    });
    let all_summaries_consistent = summaries.iter().all(|summary| {
        summary.evaluated_worlds == 32 * WORLDS_PER_SEED
            && summary.resource_feasible_worlds == summary.evaluated_worlds
            && summary.physically_executable_worlds == summary.evaluated_worlds
            && summary.execution_violation_worlds == 0
    });
    let result = if cases.len() == 64
        && worlds.len() == expected_worlds
        && resource_only_optimum_rejected_by_execution_cases > 0
        && execution_constrained_non_executable_cases == 0
        && execution_constrained_non_executable_worlds == 0
        && independent_resource_accounting_violations == 0
        && all_resource_worlds_feasible
        && all_selected_executions_valid
        && all_summaries_consistent
    {
        "validated"
    } else {
        "control_mismatch"
    };

    Ok(ExecutionReceipt {
        result: result.to_owned(),
        suite_id: SUITE_ID.to_owned(),
        suite_version: SUITE_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        generated_case_set_sha256: sha256(&manifest_bytes),
        execution_envelope: profile,
        validation_seed_count: cases.iter().filter(|case| case.split == Split::Validation).count(),
        held_out_seed_count: cases.iter().filter(|case| case.split == Split::HeldOut).count(),
        case_count: cases.len(),
        world_evaluation_count: worlds.len(),
        resource_only_optimum_rejected_by_execution_cases,
        execution_constrained_cases: decisions.len(),
        execution_constrained_non_executable_cases,
        execution_constrained_non_executable_worlds,
        independent_resource_accounting_violations,
        summaries: summaries.to_vec(),
        case_decisions: decisions,
        world_evaluations: worlds,
        claim_ceiling: "Finite exact static scheduling with coarse authored batch-rate ramp and minimum-up constraints. All units are synthetic; no equipment response model, site-calibrated executable envelope, operational safety claim, or real-mine cost/carbon claim is implied.".to_owned(),
    })
}

pub(super) fn run_cli(args: &[String]) -> Check<ExitCode> {
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: mine-flex-scheduling-bench --executability --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize execution receipt: {error}"))?
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
    fn execution_audit_checks_ramps_and_start_constraints() {
        let envelope = execution_envelope();
        assert!(audit_execution(&[1, 2, 1], &envelope).executable);
        let ramp_up = audit_execution(&[0, 2, 2], &envelope);
        assert!(!ramp_up.executable);
        assert!(ramp_up.violations.iter().any(|v| v.code == "RAMP_UP_EXCEEDED"));

        let ramp_down = audit_execution(&[2, 0, 2], &envelope);
        assert!(!ramp_down.executable);
        assert!(ramp_down.violations.iter().any(|v| v.code == "RAMP_DOWN_EXCEEDED"));
        assert!(ramp_down.violations.iter().any(|v| v.code == "STARTUP_LIMIT_EXCEEDED"));
    }

    #[test]
    fn minimum_up_time_cannot_be_satisfied_by_horizon_truncation() {
        let envelope = execution_envelope();
        let stopped_early = audit_execution(&[1, 0, 3], &envelope);
        assert!(stopped_early.violations.iter().any(|v| v.code == "MINIMUM_UPTIME_NOT_MET"));
        let start_too_late = audit_execution(&[0, 0, 1], &envelope);
        assert!(start_too_late.violations.iter().any(|v| v.code == "MINIMUM_UPTIME_EXCEEDS_HORIZON"));
    }

    #[test]
    fn resource_feasible_schedule_can_fail_the_independent_execution_envelope() {
        let process = model();
        let envelope = execution_envelope();
        let batches = [0, 2, 2];
        assert!(!audit_execution(&batches, &envelope).executable);
        for case in generate_cases() {
            for world in &case.uncertainty_envelope {
                let score = evaluate_schedule(&process, &world.scenario, &batches)
                    .expect("control is resource-feasible in each declared envelope world");
                assert_eq!(
                    score,
                    independently_reconstruct_score(&process, &world.scenario, &batches)
                        .expect("independent resource accounting")
                );
            }
        }
    }

    #[test]
    fn exact_head_receipt_is_deterministic_and_selects_only_executable_plans() {
        let head = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let first = make_receipt(head).expect("receipt");
        let second = make_receipt(head).expect("replay");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        assert_eq!("validated", first.result);
        assert_eq!(64, first.case_count);
        assert_eq!(320, first.world_evaluation_count);
        assert!(first.resource_only_optimum_rejected_by_execution_cases > 0);
        assert_eq!(64, first.execution_constrained_cases);
        assert_eq!(0, first.execution_constrained_non_executable_cases);
        assert_eq!(0, first.execution_constrained_non_executable_worlds);
        assert_eq!(0, first.independent_resource_accounting_violations);
        assert!(make_receipt("not-a-commit").is_err());
    }
}
