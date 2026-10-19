use super::*;
use serde::Serialize;
use std::collections::BTreeMap;

const RECOURSE_VERSION: &str = "mine-flex-causal-recourse-v0.1.0";
const RECURRENT_WORLD_COUNT: usize = WORLDS_PER_SEED + 3;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WorldClass {
    InEnvelope,
    OutOfEnvelope,
}

#[derive(Debug, Clone, Serialize)]
struct EvaluationWorld {
    realization_id: String,
    class: WorldClass,
    scenario: ScenarioInput,
}

#[derive(Debug, Clone, Serialize)]
struct RecourseDecision {
    epoch: usize,
    observation_prefix_sha256: String,
    chosen_batches_this_period: u32,
    consistent_envelope_worlds: usize,
    used_conservative_fallback: bool,
    viable_actions_considered: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RecourseExecution {
    executed_batches: Vec<u32>,
    decisions: Vec<RecourseDecision>,
    completed_production_target: bool,
    rejection_reason: Option<String>,
    incurred_grid_cost_units: u64,
    incurred_grid_carbon_units: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RecourseWorldResult {
    seed: u64,
    split: Split,
    world_class: WorldClass,
    realization_id: String,
    evaluator_oracle_schedule_batches: Option<Vec<u32>>,
    evaluator_oracle_grid_cost_units: Option<u64>,
    fixed_baseline: PolicyEvaluation,
    nominal_policy: PolicyEvaluation,
    static_robust_policy: PolicyEvaluation,
    adaptive_recourse: PolicyEvaluation,
    adaptive_execution: RecourseExecution,
}

#[derive(Debug, Clone, Serialize)]
struct RecourseSummary {
    split: Split,
    world_class: WorldClass,
    policy_id: String,
    evaluated_worlds: usize,
    feasible_worlds: usize,
    infeasible_worlds: usize,
    feasible_rate_basis_points: u32,
    mean_feasible_cost_units_floor: Option<u64>,
    mean_feasible_carbon_units_floor: Option<u64>,
    mean_incurred_cost_units_per_world_floor: Option<u64>,
    cost_regret: TailMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct RecourseReceipt {
    result: String,
    suite_id: String,
    suite_version: String,
    recourse_policy_version: String,
    generator_version: String,
    source_commit: String,
    generated_evaluation_set_sha256: String,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    envelope_worlds_per_seed: usize,
    out_of_envelope_challenges_per_seed: usize,
    in_envelope_world_count: usize,
    out_of_envelope_world_count: usize,
    total_world_count: usize,
    validation_world_count: usize,
    held_out_world_count: usize,
    completed_adaptive_worlds_within_envelope: usize,
    adaptive_aborts_outside_envelope: usize,
    causal_nonanticipativity_violations: usize,
    independent_hard_constraint_violations: usize,
    summaries: Vec<RecourseSummary>,
    world_results: Vec<RecourseWorldResult>,
    claim_ceiling: String,
}

fn set_observed_prefix(scenario: &mut ScenarioInput, prefix: &[PeriodInput]) {
    for (index, observed) in prefix.iter().enumerate() {
        scenario.periods[index] = observed.clone();
    }
}

fn same_observed_period(left: &PeriodInput, right: &PeriodInput) -> bool {
    left == right
}

fn prefix_matches(candidate: &ScenarioInput, observed_prefix: &[PeriodInput]) -> bool {
    candidate.periods.len() >= observed_prefix.len()
        && candidate
            .periods
            .iter()
            .zip(observed_prefix.iter())
            .all(|(candidate, observed)| same_observed_period(candidate, observed))
}

fn schedule_prefix_matches(schedule: &[u32], prefix_batches: &[u32]) -> bool {
    schedule.len() >= prefix_batches.len()
        && schedule
            .iter()
            .zip(prefix_batches.iter())
            .all(|(actual, committed)| actual == committed)
}

fn choose_recourse_action(
    model: &ProcessModel,
    forecast: &ScenarioInput,
    envelope: &[GeneratedWorld],
    observed_prefix: &[PeriodInput],
    executed_batches: &[u32],
) -> Check<RecourseDecision> {
    if observed_prefix.is_empty()
        || observed_prefix.len() > forecast.periods.len()
        || executed_batches.len() + 1 != observed_prefix.len()
    {
        return Err("recourse received a malformed observation/action prefix".to_owned());
    }
    let epoch = observed_prefix.len() - 1;
    let consistent: Vec<&GeneratedWorld> = envelope
        .iter()
        .filter(|world| prefix_matches(&world.scenario, observed_prefix))
        .collect();
    // A support miss must not make the policy invent a probability or assume an easier future.
    // Reuse every declared future projection, overwriting only periods already observed.
    let used_conservative_fallback = consistent.is_empty();
    let projected_worlds: Vec<ScenarioInput> = if used_conservative_fallback {
        envelope
            .iter()
            .map(|world| {
                let mut projected = world.scenario.clone();
                set_observed_prefix(&mut projected, observed_prefix);
                projected
            })
            .collect()
    } else {
        consistent
            .iter()
            .map(|world| {
                let mut projected = world.scenario.clone();
                set_observed_prefix(&mut projected, observed_prefix);
                projected
            })
            .collect()
    };
    if projected_worlds.is_empty() {
        return Err("recourse has no declared future projection; fail closed".to_owned());
    }

    let full_vectors = enumerate_vectors(&forecast.periods)?;
    let mut best: Option<(u64, u64, u32)> = None;
    let mut viable_actions_considered = 0_usize;
    for chosen_now in 0..=observed_prefix[epoch].max_process_batches {
        let mut action_prefix = executed_batches.to_vec();
        action_prefix.push(chosen_now);
        let mut worst_recourse_cost = 0_u64;
        let mut worst_recourse_carbon = 0_u64;
        let mut viable_everywhere = true;

        for world in &projected_worlds {
            let mut best_completion: Option<ScheduleScore> = None;
            for vector in &full_vectors {
                if !schedule_prefix_matches(vector, &action_prefix)
                    || vector.iter().map(|count| u64::from(*count)).sum::<u64>()
                        != u64::from(forecast.production_target_batches)
                {
                    continue;
                }
                if let Ok(score) = evaluate_schedule(model, world, vector) {
                    let reconstructed =
                        independently_reconstruct_score(model, world, vector)?;
                    if reconstructed != score {
                        return Err("recourse continuation disagrees with independent resource reconstruction".to_owned());
                    }
                    if best_completion
                        .as_ref()
                        .is_none_or(|current| score_order(&score, current).is_lt())
                    {
                        best_completion = Some(score);
                    }
                }
            }
            let Some(completion) = best_completion else {
                viable_everywhere = false;
                break;
            };
            worst_recourse_cost = worst_recourse_cost.max(completion.total_grid_cost_units);
            worst_recourse_carbon = worst_recourse_carbon.max(completion.total_grid_carbon_units);
        }

        if viable_everywhere {
            viable_actions_considered += 1;
            let candidate = (worst_recourse_cost, worst_recourse_carbon, chosen_now);
            let replace = best.as_ref().is_none_or(|current| candidate < *current);
            if replace {
                best = Some(candidate);
            }
        }
    }

    let Some((_, _, chosen_batches_this_period)) = best else {
        return Err(format!(
            "NO_FEASIBLE_RECOURSE_AT_EPOCH_{epoch}: no action preserves a feasible production completion across {} declared future projections",
            projected_worlds.len()
        ));
    };
    let prefix_bytes = serde_json::to_vec(observed_prefix)
        .map_err(|error| format!("cannot serialize observed prefix: {error}"))?;
    Ok(RecourseDecision {
        epoch,
        observation_prefix_sha256: sha256(&prefix_bytes),
        chosen_batches_this_period,
        consistent_envelope_worlds: consistent.len(),
        used_conservative_fallback,
        viable_actions_considered,
    })
}

fn score_incurred_prefix(
    model: &ProcessModel,
    realized: &ScenarioInput,
    batches: &[u32],
) -> Check<(u64, u64)> {
    if batches.len() > realized.periods.len() {
        return Err("executed action prefix is longer than the scenario".to_owned());
    }
    let mut total_cost = 0_u64;
    let mut total_carbon = 0_u64;
    for (period, count) in realized.periods.iter().zip(batches.iter().copied()) {
        if count > period.max_process_batches
            || period.water_available_units < period.protected_water_reserve_units
        {
            return Err("executed action violated a hard process/water constraint".to_owned());
        }
        let safety = u64::from(period.safety_electricity_units);
        let process_power =
            u64::from(count) * u64::from(model.electricity_units_per_batch);
        let demand = safety + process_power;
        let grid = demand.saturating_sub(u64::from(period.renewable_electricity_units));
        let process_water = u64::from(count) * u64::from(model.water_units_per_batch);
        let available_for_process = u64::from(
            period.water_available_units - period.protected_water_reserve_units,
        );
        if grid > u64::from(period.grid_import_cap_units)
            || safety
                > u64::from(period.renewable_electricity_units)
                    + u64::from(period.grid_import_cap_units)
            || process_water > available_for_process
        {
            return Err("executed action violated electrical or protected-water feasibility".to_owned());
        }
        total_cost = total_cost
            .checked_add(grid * u64::from(period.grid_price_units_per_import_unit))
            .ok_or_else(|| "incurred grid cost overflow".to_owned())?;
        total_carbon = total_carbon
            .checked_add(grid * u64::from(period.grid_carbon_units_per_import_unit))
            .ok_or_else(|| "incurred grid carbon overflow".to_owned())?;
    }
    Ok((total_cost, total_carbon))
}

fn execute_adaptive_recourse(
    model: &ProcessModel,
    forecast: &ScenarioInput,
    envelope: &[GeneratedWorld],
    realized: &ScenarioInput,
) -> Check<RecourseExecution> {
    let mut executed_batches = Vec::new();
    let mut decisions = Vec::new();
    let mut rejection_reason = None;
    for epoch in 0..realized.periods.len() {
        let observed_prefix = &realized.periods[..=epoch];
        match choose_recourse_action(model, forecast, envelope, observed_prefix, &executed_batches) {
            Ok(decision) => {
                executed_batches.push(decision.chosen_batches_this_period);
                decisions.push(decision);
            }
            Err(reason) => {
                rejection_reason = Some(reason);
                break;
            }
        }
    }
    let completed_production_target = executed_batches.len() == realized.periods.len()
        && executed_batches.iter().map(|count| u64::from(*count)).sum::<u64>()
            == u64::from(realized.production_target_batches)
        && evaluate_schedule(model, realized, &executed_batches).is_ok();
    if !completed_production_target && rejection_reason.is_none() {
        rejection_reason = Some("PRODUCTION_TARGET_NOT_COMPLETED".to_owned());
    }
    let (incurred_grid_cost_units, incurred_grid_carbon_units) =
        score_incurred_prefix(model, realized, &executed_batches)?;
    if completed_production_target {
        let score = evaluate_schedule(model, realized, &executed_batches)?;
        let independent = independently_reconstruct_score(model, realized, &executed_batches)?;
        if score != independent {
            return Err("completed recourse schedule fails independent reconstruction".to_owned());
        }
    }
    Ok(RecourseExecution {
        executed_batches,
        decisions,
        completed_production_target,
        rejection_reason,
        incurred_grid_cost_units,
        incurred_grid_carbon_units,
    })
}

fn evaluation_for_schedule(
    policy_id: &str,
    model: &ProcessModel,
    scenario: &ScenarioInput,
    schedule: Option<&[u32]>,
    oracle_cost: Option<u64>,
    hard_constraint_violations: &mut usize,
) -> PolicyEvaluation {
    let Some(batches) = schedule else {
        return PolicyEvaluation {
            policy_id: policy_id.to_owned(),
            schedule_batches: None,
            feasible: false,
            rejection_reason: Some("NO_FEASIBLE_SCHEDULE".to_owned()),
            grid_cost_units: None,
            grid_carbon_units: None,
            regret_vs_realization_oracle_units: None,
        };
    };
    match evaluate_schedule(model, scenario, batches) {
        Ok(score) => {
            match independently_reconstruct_score(model, scenario, batches) {
                Ok(reconstructed) if reconstructed == score => {}
                _ => {
                    *hard_constraint_violations += 1;
                    return PolicyEvaluation {
                        policy_id: policy_id.to_owned(),
                        schedule_batches: Some(batches.to_vec()),
                        feasible: false,
                        rejection_reason: Some("INDEPENDENT_RECONSTRUCTION_MISMATCH".to_owned()),
                        grid_cost_units: None,
                        grid_carbon_units: None,
                        regret_vs_realization_oracle_units: None,
                    };
                }
            }
            if oracle_cost.is_some_and(|cost| score.total_grid_cost_units < cost) {
                *hard_constraint_violations += 1;
                return PolicyEvaluation {
                    policy_id: policy_id.to_owned(),
                    schedule_batches: Some(batches.to_vec()),
                    feasible: false,
                    rejection_reason: Some("POLICY_BEATS_EVALUATOR_ORACLE_COST".to_owned()),
                    grid_cost_units: None,
                    grid_carbon_units: None,
                    regret_vs_realization_oracle_units: None,
                };
            }
            PolicyEvaluation {
                policy_id: policy_id.to_owned(),
                schedule_batches: Some(batches.to_vec()),
                feasible: true,
                rejection_reason: None,
                grid_cost_units: Some(score.total_grid_cost_units),
                grid_carbon_units: Some(score.total_grid_carbon_units),
                regret_vs_realization_oracle_units: oracle_cost
                    .map(|cost| score.total_grid_cost_units - cost),
            }
        }
        Err(reason) => PolicyEvaluation {
            policy_id: policy_id.to_owned(),
            schedule_batches: Some(batches.to_vec()),
            feasible: false,
            rejection_reason: Some(reason),
            grid_cost_units: None,
            grid_carbon_units: None,
            regret_vs_realization_oracle_units: None,
        },
    }
}

fn add_out_of_envelope_worlds(case: &GeneratedCase) -> Vec<EvaluationWorld> {
    let forecast = &case.forecast;
    let p1_moderate_water = apply_changes(
        forecast,
        "ood-p1-water-three",
        &[(1, 2, 3)],
    );
    let p1_severe_correlated = apply_changes(
        forecast,
        "ood-p1-water-floor-with-renewable-loss",
        &[(1, 0, 0), (1, 1, 4), (1, 2, 2)],
    );
    let p2_last_moment_water = apply_changes(
        forecast,
        "ood-p2-last-moment-water-floor",
        &[(2, 2, 2)],
    );
    vec![
        EvaluationWorld {
            realization_id: "ood-p1-water-three".to_owned(),
            class: WorldClass::OutOfEnvelope,
            scenario: p1_moderate_water,
        },
        EvaluationWorld {
            realization_id: "ood-p1-water-floor-with-renewable-loss".to_owned(),
            class: WorldClass::OutOfEnvelope,
            scenario: p1_severe_correlated,
        },
        EvaluationWorld {
            realization_id: "ood-p2-last-moment-water-floor".to_owned(),
            class: WorldClass::OutOfEnvelope,
            scenario: p2_last_moment_water,
        },
    ]
}

fn policy_for_world(
    seed: u64,
    split: Split,
    class: WorldClass,
    realization_id: String,
    world: &ScenarioInput,
    model: &ProcessModel,
    case: &GeneratedCase,
    nominal_schedule: &[u32],
    static_robust_schedule: Option<&[u32]>,
    hard_constraint_violations: &mut usize,
) -> Check<(RecourseWorldResult, Option<ScheduleScore>)> {
    let oracle = candidate_schedules(model, world)?
        .into_iter()
        .min_by(score_order);
    let oracle_cost = oracle.as_ref().map(|score| score.total_grid_cost_units);
    let fixed = evaluation_for_schedule(
        "fixed_baseline",
        model,
        world,
        Some(&FIXED_BASELINE),
        oracle_cost,
        hard_constraint_violations,
    );
    let nominal = evaluation_for_schedule(
        "nominal_forecast_cost_minimizer",
        model,
        world,
        Some(nominal_schedule),
        oracle_cost,
        hard_constraint_violations,
    );
    let robust = evaluation_for_schedule(
        ROBUST_POLICY_ID,
        model,
        world,
        static_robust_schedule,
        oracle_cost,
        hard_constraint_violations,
    );
    let adaptive_execution = execute_adaptive_recourse(
        model,
        &case.forecast,
        &case.uncertainty_envelope,
        world,
    )?;
    let adaptive_schedule = if adaptive_execution.completed_production_target {
        Some(adaptive_execution.executed_batches.as_slice())
    } else {
        None
    };
    let mut adaptive = evaluation_for_schedule(
        "causal_adaptive_recourse",
        model,
        world,
        adaptive_schedule,
        oracle_cost,
        hard_constraint_violations,
    );
    if !adaptive_execution.completed_production_target {
        adaptive.rejection_reason = adaptive_execution.rejection_reason.clone();
        // Keep actual incurred cost separate from feasible cost in the summary.
        adaptive.grid_cost_units = None;
        adaptive.grid_carbon_units = None;
    }
    let row = RecourseWorldResult {
        seed,
        split,
        world_class: class,
        realization_id,
        evaluator_oracle_schedule_batches: oracle.as_ref().map(|score| score.batches.clone()),
        evaluator_oracle_grid_cost_units: oracle_cost,
        fixed_baseline: fixed,
        nominal_policy: nominal,
        static_robust_policy: robust,
        adaptive_recourse: adaptive,
        adaptive_execution,
    };
    Ok((row, oracle))
}

fn result_policy(row: &RecourseWorldResult, policy_id: &str) -> &PolicyEvaluation {
    match policy_id {
        "fixed_baseline" => &row.fixed_baseline,
        "nominal_forecast_cost_minimizer" => &row.nominal_policy,
        ROBUST_POLICY_ID => &row.static_robust_policy,
        "causal_adaptive_recourse" => &row.adaptive_recourse,
        _ => unreachable!("summary asks only for a declared policy"),
    }
}

fn summarize(
    rows: &[RecourseWorldResult],
    split: Split,
    world_class: WorldClass,
    policy_id: &str,
) -> RecourseSummary {
    let selected: Vec<&RecourseWorldResult> = rows
        .iter()
        .filter(|row| row.split == split && row.world_class == world_class)
        .collect();
    let outcomes: Vec<&PolicyEvaluation> = selected
        .iter()
        .map(|row| result_policy(row, policy_id))
        .collect();
    let feasible: Vec<&PolicyEvaluation> = outcomes
        .iter()
        .copied()
        .filter(|outcome| outcome.feasible)
        .collect();
    let costs: Vec<u64> = feasible
        .iter()
        .filter_map(|outcome| outcome.grid_cost_units)
        .collect();
    let carbons: Vec<u64> = feasible
        .iter()
        .filter_map(|outcome| outcome.grid_carbon_units)
        .collect();
    let regrets: Vec<u64> = feasible
        .iter()
        .filter_map(|outcome| outcome.regret_vs_realization_oracle_units)
        .collect();
    let incurred: Vec<u64> = selected
        .iter()
        .filter_map(|row| {
            if policy_id == "causal_adaptive_recourse" {
                Some(row.adaptive_execution.incurred_grid_cost_units)
            } else {
                result_policy(row, policy_id)
                    .grid_cost_units
                    .or_else(|| Some(0))
            }
        })
        .collect();
    let sum_cost: u128 = costs.iter().map(|value| u128::from(*value)).sum();
    let sum_carbon: u128 = carbons.iter().map(|value| u128::from(*value)).sum();
    let sum_incurred: u128 = incurred.iter().map(|value| u128::from(*value)).sum();
    let feasible_count = feasible.len();
    let total = outcomes.len();
    RecourseSummary {
        split,
        world_class,
        policy_id: policy_id.to_owned(),
        evaluated_worlds: total,
        feasible_worlds: feasible_count,
        infeasible_worlds: total - feasible_count,
        feasible_rate_basis_points: if total == 0 {
            0
        } else {
            ((feasible_count as u128 * 10_000) / total as u128) as u32
        },
        mean_feasible_cost_units_floor: if costs.is_empty() {
            None
        } else {
            Some((sum_cost / costs.len() as u128) as u64)
        },
        mean_feasible_carbon_units_floor: if carbons.is_empty() {
            None
        } else {
            Some((sum_carbon / carbons.len() as u128) as u64)
        },
        mean_incurred_cost_units_per_world_floor: if incurred.is_empty() {
            None
        } else {
            Some((sum_incurred / incurred.len() as u128) as u64)
        },
        cost_regret: tail_metrics(&regrets),
    }
}

fn audit_nonanticipativity(rows: &[RecourseWorldResult]) -> usize {
    let mut actions: BTreeMap<(u64, String, usize, String), u32> = BTreeMap::new();
    let mut violations = 0;
    for row in rows {
        for decision in &row.adaptive_execution.decisions {
            let key = (
                row.seed,
                format!("{:?}", row.split),
                decision.epoch,
                decision.observation_prefix_sha256.clone(),
            );
            if actions
                .insert(key, decision.chosen_batches_this_period)
                .is_some_and(|previous| previous != decision.chosen_batches_this_period)
            {
                violations += 1;
            }
        }
    }
    violations
}

fn make_recourse_receipt(source_commit: &str) -> Check<RecourseReceipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a lowercase 40-character hex SHA".to_owned());
    }
    let model = ProcessModel {
        electricity_units_per_batch: 2,
        water_units_per_batch: 1,
    };
    let cases = generate_cases();
    let mut case_manifest = Vec::new();
    let mut rows = Vec::new();
    let mut hard_constraint_violations = 0_usize;

    for case in &cases {
        let nominal = choose_nominal_schedule(&model, &case.forecast)?;
        let robust = choose_robust_schedule(&model, &case.forecast, &case.uncertainty_envelope).ok();
        let mut worlds: Vec<EvaluationWorld> = case
            .uncertainty_envelope
            .iter()
            .map(|world| EvaluationWorld {
                realization_id: world.realization_id.clone(),
                class: WorldClass::InEnvelope,
                scenario: world.scenario.clone(),
            })
            .collect();
        worlds.extend(add_out_of_envelope_worlds(case));
        case_manifest.push((case.seed, case.split, case.forecast.clone(), worlds.clone()));
        for world in worlds {
            let (row, _) = policy_for_world(
                case.seed,
                case.split,
                world.class,
                world.realization_id,
                &world.scenario,
                &model,
                case,
                &nominal,
                robust.as_deref(),
                &mut hard_constraint_violations,
            )?;
            if row.adaptive_recourse.feasible
                && (row.adaptive_execution.executed_batches.len() != world.scenario.periods.len()
                    || !independently_reconstruct_score(
                        &model,
                        &world.scenario,
                        &row.adaptive_execution.executed_batches,
                    )
                    .is_ok_and(|score| {
                        Some(score.total_grid_cost_units) == row.adaptive_recourse.grid_cost_units
                    }))
            {
                hard_constraint_violations += 1;
            }
            rows.push(row);
        }
    }

    let nonanticipativity_violations = audit_nonanticipativity(&rows);
    let mut summaries = Vec::new();
    for split in [Split::Validation, Split::HeldOut] {
        for class in [WorldClass::InEnvelope, WorldClass::OutOfEnvelope] {
            for policy in [
                "fixed_baseline",
                "nominal_forecast_cost_minimizer",
                ROBUST_POLICY_ID,
                "causal_adaptive_recourse",
            ] {
                summaries.push(summarize(&rows, split, class, policy));
            }
        }
    }
    let in_envelope_world_count = rows.iter().filter(|row| row.world_class == WorldClass::InEnvelope).count();
    let out_of_envelope_world_count = rows.iter().filter(|row| row.world_class == WorldClass::OutOfEnvelope).count();
    let validation_world_count = rows.iter().filter(|row| row.split == Split::Validation).count();
    let held_out_world_count = rows.iter().filter(|row| row.split == Split::HeldOut).count();
    let completed_adaptive_worlds_within_envelope = rows
        .iter()
        .filter(|row| {
            row.world_class == WorldClass::InEnvelope
                && row.adaptive_execution.completed_production_target
                && row.adaptive_recourse.feasible
        })
        .count();
    let adaptive_aborts_outside_envelope = rows
        .iter()
        .filter(|row| {
            row.world_class == WorldClass::OutOfEnvelope
                && !row.adaptive_execution.completed_production_target
        })
        .count();
    let all_worlds_accounted = rows.len()
        == (VALIDATION_SEEDS.clone().count() + HELD_OUT_SEEDS.clone().count())
            * RECURRENT_WORLD_COUNT;
    let summaries_consistent = summaries.iter().all(|summary| {
        summary.feasible_worlds + summary.infeasible_worlds == summary.evaluated_worlds
            && (summary.evaluated_worlds == 0 || summary.feasible_rate_basis_points
                == ((summary.feasible_worlds as u128 * 10_000) / summary.evaluated_worlds as u128) as u32)
    });
    let in_envelope_completion_control =
        completed_adaptive_worlds_within_envelope == in_envelope_world_count;
    let result = if hard_constraint_violations == 0
        && nonanticipativity_violations == 0
        && in_envelope_completion_control
        && all_worlds_accounted
        && summaries_consistent
    {
        "validated"
    } else {
        "control_mismatch"
    };
    let manifest_bytes = serde_json::to_vec(&case_manifest)
        .map_err(|error| format!("cannot serialize recourse case manifest: {error}"))?;
    Ok(RecourseReceipt {
        result: result.to_owned(),
        suite_id: "MINE-FLEX-RECOURSE-001".to_owned(),
        suite_version: "0.1.0".to_owned(),
        recourse_policy_version: RECOURSE_VERSION.to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        generated_evaluation_set_sha256: sha256(&manifest_bytes),
        validation_seed_count: VALIDATION_SEEDS.clone().count(),
        held_out_seed_count: HELD_OUT_SEEDS.clone().count(),
        envelope_worlds_per_seed: WORLDS_PER_SEED,
        out_of_envelope_challenges_per_seed: 3,
        in_envelope_world_count,
        out_of_envelope_world_count,
        total_world_count: rows.len(),
        validation_world_count,
        held_out_world_count,
        completed_adaptive_worlds_within_envelope,
        adaptive_aborts_outside_envelope,
        causal_nonanticipativity_violations: nonanticipativity_violations,
        independent_hard_constraint_violations: hard_constraint_violations,
        summaries,
        world_results: rows,
        claim_ceiling: "Causal staged recourse on a finite public synthetic uncertainty set plus authored out-of-envelope stresses; no real-mine safety, economic, environmental or forecast-calibration claim.".to_owned(),
    })
}

pub(super) fn run_cli(args: &[String]) -> Check<ExitCode> {
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: mine-flex-scheduling-bench --recourse --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let receipt = make_recourse_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize causal-recourse receipt: {error}"))?
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

    fn model() -> ProcessModel {
        ProcessModel {
            electricity_units_per_batch: 2,
            water_units_per_batch: 1,
        }
    }

    #[test]
    fn recourse_completes_every_in_envelope_world() {
        for case in generate_cases() {
            for world in &case.uncertainty_envelope {
                let outcome = execute_adaptive_recourse(
                    &model(),
                    &case.forecast,
                    &case.uncertainty_envelope,
                    &world.scenario,
                )
                .expect("recourse execution");
                assert!(outcome.completed_production_target, "seed {} world {}", case.seed, world.realization_id);
                assert_eq!(3, outcome.executed_batches.len());
            }
        }
    }

    #[test]
    fn future_only_perturbation_does_not_change_first_action() {
        let case = generate_case(17, Split::Validation);
        let base = case.uncertainty_envelope[0].scenario.clone();
        let changed = apply_changes(
            &base,
            "only-future-fields-changed",
            &[(1, 0, 0), (1, 1, 4), (2, 2, 2)],
        );
        let base_action = choose_recourse_action(
            &model(),
            &case.forecast,
            &case.uncertainty_envelope,
            &base.periods[..=0],
            &[],
        )
        .expect("base first action");
        let changed_action = choose_recourse_action(
            &model(),
            &case.forecast,
            &case.uncertainty_envelope,
            &changed.periods[..=0],
            &[],
        )
        .expect("future-perturbed first action");
        assert_eq!(
            base_action.chosen_batches_this_period,
            changed_action.chosen_batches_this_period
        );
        assert_eq!(
            base_action.observation_prefix_sha256,
            changed_action.observation_prefix_sha256
        );
    }

    #[test]
    fn out_of_envelope_shocks_are_never_marked_feasible_after_an_abort() {
        for case in generate_cases() {
            for world in add_out_of_envelope_worlds(&case) {
                let outcome = execute_adaptive_recourse(
                    &model(),
                    &case.forecast,
                    &case.uncertainty_envelope,
                    &world.scenario,
                )
                .expect("out-of-envelope execution records a safe disposition");
                if !outcome.completed_production_target {
                    assert!(outcome.rejection_reason.is_some());
                    assert!(outcome.executed_batches.len() < world.scenario.periods.len());
                }
            }
        }
    }

    #[test]
    fn same_seed_replays_byte_identically_and_prefix_action_audit_is_zero() {
        let first = make_recourse_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("valid source commit");
        let second = make_recourse_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("replay");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first receipt"),
            serde_json::to_vec(&second).expect("serialize second receipt")
        );
        assert_eq!("validated", first.result);
        assert_eq!(0, first.causal_nonanticipativity_violations);
        assert_eq!(0, first.independent_hard_constraint_violations);
        assert_eq!(320, first.in_envelope_world_count);
        assert_eq!(192, first.out_of_envelope_world_count);
        assert_eq!(512, first.total_world_count);
        assert_eq!(320, first.completed_adaptive_worlds_within_envelope);
        assert_eq!(512, first.validation_world_count + first.held_out_world_count);
        assert!(first.adaptive_aborts_outside_envelope <= 192);
    }
}
