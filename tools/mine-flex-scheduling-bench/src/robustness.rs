use super::{
    enumerate_vectors, evaluate_schedule, independently_reconstruct_score, is_lower_hex, score_order,
    sha256, Check, PeriodInput, ProcessModel, ScenarioExpectation, ScenarioInput, ScenarioStatus,
    ScheduleScore,
};
use serde::Serialize;
use std::{env, process::ExitCode};

const GENERATOR_VERSION: &str = "mine-flex-generated-disturbances-v0.1.0";
const VALIDATION_SEEDS: std::ops::RangeInclusive<u64> = 1..=32;
const HELD_OUT_SEEDS: std::ops::RangeInclusive<u64> = 10_001..=10_032;
const WORLDS_PER_SEED: usize = 5;
const PRODUCTION_TARGET: u32 = 4;
const FIXED_BASELINE: [u32; 3] = [2, 1, 1];
const ROBUST_POLICY_ID: &str = "bounded_minimax_cost";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Split {
    Validation,
    HeldOut,
}

#[derive(Debug, Clone, Serialize)]
struct GeneratedWorld {
    realization_id: String,
    scenario: ScenarioInput,
}

#[derive(Debug, Clone, Serialize)]
struct GeneratedCase {
    seed: u64,
    split: Split,
    forecast: ScenarioInput,
    uncertainty_envelope: Vec<GeneratedWorld>,
}

#[derive(Debug, Clone, Copy)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn range(&mut self, upper_exclusive: u64) -> u64 {
        self.next_u64() % upper_exclusive
    }
}

#[derive(Debug, Clone, Serialize)]
struct PolicyEvaluation {
    policy_id: String,
    schedule_batches: Option<Vec<u32>>,
    feasible: bool,
    rejection_reason: Option<String>,
    grid_cost_units: Option<u64>,
    grid_carbon_units: Option<u64>,
    regret_vs_realization_oracle_units: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct WorldResult {
    seed: u64,
    split: Split,
    realization_id: String,
    oracle_schedule_batches: Vec<u32>,
    oracle_grid_cost_units: u64,
    oracle_grid_carbon_units: u64,
    fixed_baseline: PolicyEvaluation,
    nominal_policy: PolicyEvaluation,
    robust_policy: PolicyEvaluation,
}

#[derive(Debug, Clone, Serialize)]
struct TailMetrics {
    sample_count: usize,
    mean_regret_units_floor: Option<u64>,
    worst_regret_units: Option<u64>,
    p95_regret_units_nearest_rank: Option<u64>,
    cvar95_regret_units_worst_five_percent_mean_floor: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct PolicySummary {
    split: Split,
    policy_id: String,
    evaluated_worlds: usize,
    feasible_worlds: usize,
    infeasible_worlds: usize,
    feasible_rate_basis_points: u32,
    hard_constraint_violations: usize,
    mean_cost_units_floor: Option<u64>,
    mean_carbon_units_floor: Option<u64>,
    cost_regret: TailMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct PairedComparison {
    split: Split,
    left_policy_id: String,
    right_policy_id: String,
    paired_worlds_both_feasible: usize,
    mean_right_minus_left_grid_cost_units_signed_floor: Option<i64>,
    right_policy_feasible_when_left_infeasible: usize,
    left_policy_feasible_when_right_infeasible: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RobustnessReceipt {
    result: String,
    suite_id: String,
    suite_version: String,
    generator_version: String,
    source_commit: String,
    generated_case_set_sha256: String,
    validation_seed_count: usize,
    held_out_seed_count: usize,
    uncertainty_worlds_per_seed: usize,
    validation_world_result_count: usize,
    held_out_world_result_count: usize,
    total_world_result_count: usize,
    hard_constraint_violations: usize,
    robust_policy_feasible_worlds: usize,
    robust_policy_infeasible_worlds: usize,
    summaries: Vec<PolicySummary>,
    paired_comparisons: Vec<PairedComparison>,
    claim_ceiling: String,
    world_results: Vec<WorldResult>,
}

fn expectation_placeholder() -> ScenarioExpectation {
    ScenarioExpectation {
        status: ScenarioStatus::Feasible,
        candidate_schedules_evaluated: 0,
        feasible_schedule_count: 0,
        best_schedule_batches: None,
        best_grid_cost_units: None,
        best_grid_carbon_units: None,
        fixed_baseline_grid_cost_units: None,
        fixed_baseline_grid_carbon_units: None,
        pareto_frontier: Vec::new(),
    }
}

fn scenario(id: String, periods: Vec<PeriodInput>) -> ScenarioInput {
    ScenarioInput {
        scenario_id: id,
        production_target_batches: PRODUCTION_TARGET,
        periods,
        fixed_baseline_batches: FIXED_BASELINE.to_vec(),
        expected: expectation_placeholder(),
    }
}

fn generated_forecast(seed: u64) -> ScenarioInput {
    let mut rng = DeterministicRng::new(seed);
    let p0_price = 25 + rng.range(31) as u32;
    let p1_price = 8 + rng.range(13) as u32;
    let p2_price = 50 + rng.range(41) as u32;
    let p0_carbon = 8 + rng.range(13) as u32;
    let p1_carbon = 15 + rng.range(16) as u32;
    let p2_carbon = 2 + rng.range(9) as u32;
    let p0_water = 4 + rng.range(2) as u32;
    let p1_water = 5 + rng.range(2) as u32;
    let p2_water = 4 + rng.range(2) as u32;

    scenario(
        format!("forecast-seed-{seed}"),
        vec![
            PeriodInput {
                period_id: "P0".to_owned(),
                safety_electricity_units: 4,
                renewable_electricity_units: 2,
                grid_import_cap_units: 6,
                grid_price_units_per_import_unit: p0_price,
                grid_carbon_units_per_import_unit: p0_carbon,
                water_available_units: p0_water,
                protected_water_reserve_units: 2,
                max_process_batches: 2,
            },
            PeriodInput {
                period_id: "P1".to_owned(),
                safety_electricity_units: 4,
                renewable_electricity_units: 5,
                grid_import_cap_units: 5,
                grid_price_units_per_import_unit: p1_price,
                grid_carbon_units_per_import_unit: p1_carbon,
                water_available_units: p1_water,
                protected_water_reserve_units: 2,
                max_process_batches: 3,
            },
            PeriodInput {
                period_id: "P2".to_owned(),
                safety_electricity_units: 4,
                renewable_electricity_units: 1,
                grid_import_cap_units: 7,
                grid_price_units_per_import_unit: p2_price,
                grid_carbon_units_per_import_unit: p2_carbon,
                water_available_units: p2_water,
                protected_water_reserve_units: 2,
                max_process_batches: 2,
            },
        ],
    )
}

fn changed_period(period: &PeriodInput, changes: &[(u8, u32)]) -> PeriodInput {
    let mut updated = period.clone();
    for (field, value) in changes {
        match field {
            0 => updated.renewable_electricity_units = *value,
            1 => updated.grid_import_cap_units = *value,
            2 => updated.water_available_units = *value,
            3 => updated.grid_price_units_per_import_unit = *value,
            4 => updated.grid_carbon_units_per_import_unit = *value,
            _ => unreachable!("generator uses only declared field selectors"),
        }
    }
    updated
}

fn apply_changes(
    forecast: &ScenarioInput,
    realization_id: &str,
    changes: &[(usize, u8, u32)],
) -> ScenarioInput {
    let mut periods = forecast.periods.clone();
    for (period_index, field, value) in changes {
        periods[*period_index] = changed_period(&periods[*period_index], &[(*field, *value)]);
    }
    scenario(
        format!("{}-{realization_id}", forecast.scenario_id),
        periods,
    )
}

fn generate_case(seed: u64, split: Split) -> GeneratedCase {
    let forecast = generated_forecast(seed);
    let nominal = GeneratedWorld {
        realization_id: "nominal".to_owned(),
        scenario: forecast.clone(),
    };
    let p0_tight = GeneratedWorld {
        realization_id: "p0-grid-import-tight".to_owned(),
        scenario: apply_changes(&forecast, "p0-grid-import-tight", &[(0, 0, 1), (0, 1, 5)]),
    };
    let p1_power = GeneratedWorld {
        realization_id: "p1-renewables-shortfall".to_owned(),
        scenario: apply_changes(&forecast, "p1-renewables-shortfall", &[(1, 0, 2), (1, 1, 4)]),
    };
    let p1_water = GeneratedWorld {
        realization_id: "p1-water-shortfall".to_owned(),
        scenario: apply_changes(&forecast, "p1-water-shortfall", &[(1, 2, 4)]),
    };
    let correlated = GeneratedWorld {
        realization_id: "correlated-p0-p1-and-p2-price-spike".to_owned(),
        scenario: apply_changes(
            &forecast,
            "correlated-p0-p1-and-p2-price-spike",
            &[
                (0, 0, 1),
                (0, 1, 5),
                (1, 0, 2),
                (1, 1, 4),
                (1, 2, 4),
                (
                    2,
                    3,
                    forecast.periods[2]
                        .grid_price_units_per_import_unit
                        .saturating_add(20),
                ),
                (
                    2,
                    4,
                    forecast.periods[2]
                        .grid_carbon_units_per_import_unit
                        .saturating_add(8),
                ),
            ],
        ),
    };
    GeneratedCase {
        seed,
        split,
        forecast,
        uncertainty_envelope: vec![nominal, p0_tight, p1_power, p1_water, correlated],
    }
}

fn generate_cases() -> Vec<GeneratedCase> {
    VALIDATION_SEEDS
        .clone()
        .map(|seed| generate_case(seed, Split::Validation))
        .chain(
            HELD_OUT_SEEDS
                .clone()
                .map(|seed| generate_case(seed, Split::HeldOut)),
        )
        .collect()
}

fn candidate_schedules(
    model: &ProcessModel,
    scenario_input: &ScenarioInput,
) -> Check<Vec<ScheduleScore>> {
    let mut candidates = Vec::new();
    for batches in enumerate_vectors(&scenario_input.periods)? {
        if batches.iter().map(|count| u64::from(*count)).sum::<u64>()
            != u64::from(scenario_input.production_target_batches)
        {
            continue;
        }
        if let Ok(score) = evaluate_schedule(model, scenario_input, &batches) {
            let reconstructed =
                independently_reconstruct_score(model, scenario_input, &batches)?;
            if reconstructed != score {
                return Err("candidate schedule disagrees with independent balance reconstruction".to_owned());
            }
            candidates.push(score);
        }
    }
    Ok(candidates)
}

// The nominal policy receives a single forecast only. It is not passed a realized world.
fn choose_nominal_schedule(
    model: &ProcessModel,
    forecast: &ScenarioInput,
) -> Check<Vec<u32>> {
    candidate_schedules(model, forecast)?
        .into_iter()
        .min_by(score_order)
        .map(|score| score.batches)
        .ok_or_else(|| "nominal forecast has no feasible schedule".to_owned())
}

// The robust policy receives a published uncertainty envelope, but not a current-world index.
// It must find one schedule feasible under every member of that envelope.
fn choose_robust_schedule(
    model: &ProcessModel,
    forecast: &ScenarioInput,
    envelope: &[GeneratedWorld],
) -> Check<Vec<u32>> {
    let mut best: Option<(u64, u64, Vec<u32>)> = None;
    for batches in enumerate_vectors(&forecast.periods)? {
        if batches.iter().map(|count| u64::from(*count)).sum::<u64>()
            != u64::from(forecast.production_target_batches)
        {
            continue;
        }
        let mut worst_cost = 0_u64;
        let mut worst_carbon = 0_u64;
        let mut feasible_everywhere = true;
        for world in envelope {
            match evaluate_schedule(model, &world.scenario, &batches) {
                Ok(score) => {
                    let reconstructed =
                        independently_reconstruct_score(model, &world.scenario, &batches)?;
                    if reconstructed != score {
                        return Err("robust candidate disagrees with independent balance reconstruction".to_owned());
                    }
                    worst_cost = worst_cost.max(score.total_grid_cost_units);
                    worst_carbon = worst_carbon.max(score.total_grid_carbon_units);
                }
                Err(_) => {
                    feasible_everywhere = false;
                    break;
                }
            }
        }
        if !feasible_everywhere {
            continue;
        }
        let candidate = (worst_cost, worst_carbon, batches);
        let replace = best.as_ref().is_none_or(|current| {
            (candidate.0, candidate.1, &candidate.2) < (current.0, current.1, &current.2)
        });
        if replace {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, batches)| batches)
        .ok_or_else(|| "uncertainty envelope has no universally feasible schedule".to_owned())
}

// This perfect-information schedule belongs only to the evaluator; it is never passed to either
// selectable policy function.
fn clairvoyant_schedule(
    model: &ProcessModel,
    realized_world: &ScenarioInput,
) -> Check<ScheduleScore> {
    candidate_schedules(model, realized_world)?
        .into_iter()
        .min_by(score_order)
        .ok_or_else(|| "realized world has no feasible oracle schedule".to_owned())
}

fn policy_evaluation(
    policy_id: &str,
    model: &ProcessModel,
    realized_world: &ScenarioInput,
    schedule_batches: Option<&[u32]>,
    oracle_cost_units: u64,
    hard_constraint_violations: &mut usize,
) -> PolicyEvaluation {
    let Some(batches) = schedule_batches else {
        return PolicyEvaluation {
            policy_id: policy_id.to_owned(),
            schedule_batches: None,
            feasible: false,
            rejection_reason: Some("POLICY_NO_SCHEDULE".to_owned()),
            grid_cost_units: None,
            grid_carbon_units: None,
            regret_vs_realization_oracle_units: None,
        };
    };
    match evaluate_schedule(model, realized_world, batches) {
        Ok(score) => {
            let reconstructed =
                independently_reconstruct_score(model, realized_world, batches);
            if !matches!(reconstructed.as_ref(), Ok(independent) if independent == &score) {
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
            if score.total_grid_cost_units < oracle_cost_units {
                *hard_constraint_violations += 1;
                return PolicyEvaluation {
                    policy_id: policy_id.to_owned(),
                    schedule_batches: Some(batches.to_vec()),
                    feasible: false,
                    rejection_reason: Some("POLICY_BEATS_ORACLE_COST".to_owned()),
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
                regret_vs_realization_oracle_units: Some(
                    score.total_grid_cost_units - oracle_cost_units,
                ),
            }
        }
        Err(error) => PolicyEvaluation {
            policy_id: policy_id.to_owned(),
            schedule_batches: Some(batches.to_vec()),
            feasible: false,
            rejection_reason: Some(error),
            grid_cost_units: None,
            grid_carbon_units: None,
            regret_vs_realization_oracle_units: None,
        },
    }
}

fn evaluate_cases(model: &ProcessModel, cases: &[GeneratedCase]) -> Check<(Vec<WorldResult>, usize)> {
    let mut results = Vec::new();
    let mut hard_constraint_violations = 0;
    for case in cases {
        let nominal_schedule = choose_nominal_schedule(model, &case.forecast)?;
        let robust_schedule =
            choose_robust_schedule(model, &case.forecast, &case.uncertainty_envelope)?;
        for world in &case.uncertainty_envelope {
            let oracle = clairvoyant_schedule(model, &world.scenario)?;
            if !independently_reconstruct_score(model, &world.scenario, &oracle.batches)
                .is_ok_and(|reconstructed| reconstructed == oracle)
            {
                hard_constraint_violations += 1;
            }
            let fixed = policy_evaluation(
                "fixed_baseline",
                model,
                &world.scenario,
                Some(&FIXED_BASELINE),
                oracle.total_grid_cost_units,
                &mut hard_constraint_violations,
            );
            let nominal = policy_evaluation(
                "nominal_forecast_cost_minimizer",
                model,
                &world.scenario,
                Some(&nominal_schedule),
                oracle.total_grid_cost_units,
                &mut hard_constraint_violations,
            );
            let robust = policy_evaluation(
                ROBUST_POLICY_ID,
                model,
                &world.scenario,
                Some(&robust_schedule),
                oracle.total_grid_cost_units,
                &mut hard_constraint_violations,
            );
            results.push(WorldResult {
                seed: case.seed,
                split: case.split,
                realization_id: world.realization_id.clone(),
                oracle_schedule_batches: oracle.batches,
                oracle_grid_cost_units: oracle.total_grid_cost_units,
                oracle_grid_carbon_units: oracle.total_grid_carbon_units,
                fixed_baseline: fixed,
                nominal_policy: nominal,
                robust_policy: robust,
            });
        }
    }
    Ok((results, hard_constraint_violations))
}

fn tail_metrics(regrets: &[u64]) -> TailMetrics {
    if regrets.is_empty() {
        return TailMetrics {
            sample_count: 0,
            mean_regret_units_floor: None,
            worst_regret_units: None,
            p95_regret_units_nearest_rank: None,
            cvar95_regret_units_worst_five_percent_mean_floor: None,
        };
    }
    let sum: u128 = regrets.iter().map(|value| u128::from(*value)).sum();
    let mut sorted = regrets.to_vec();
    sorted.sort_unstable();
    let p95_index = ((95 * sorted.len() + 99) / 100).saturating_sub(1);
    let worst_count = ((5 * sorted.len() + 99) / 100).max(1);
    let worst_slice = &sorted[sorted.len() - worst_count..];
    let worst_sum: u128 = worst_slice.iter().map(|value| u128::from(*value)).sum();
    TailMetrics {
        sample_count: sorted.len(),
        mean_regret_units_floor: Some((sum / sorted.len() as u128) as u64),
        worst_regret_units: sorted.last().copied(),
        p95_regret_units_nearest_rank: sorted.get(p95_index).copied(),
        cvar95_regret_units_worst_five_percent_mean_floor: Some(
            (worst_sum / worst_count as u128) as u64,
        ),
    }
}

fn result_for_policy<'a>(world: &'a WorldResult, policy_id: &str) -> &'a PolicyEvaluation {
    match policy_id {
        "fixed_baseline" => &world.fixed_baseline,
        "nominal_forecast_cost_minimizer" => &world.nominal_policy,
        ROBUST_POLICY_ID => &world.robust_policy,
        _ => unreachable!("summary requests a declared policy"),
    }
}

fn summarize_policy(
    worlds: &[WorldResult],
    split: Split,
    policy_id: &str,
    hard_constraint_violations: usize,
) -> PolicySummary {
    let selected: Vec<&WorldResult> = worlds.iter().filter(|row| row.split == split).collect();
    let evaluations: Vec<&PolicyEvaluation> = selected
        .iter()
        .map(|row| result_for_policy(row, policy_id))
        .collect();
    let costs: Vec<u64> = evaluations
        .iter()
        .filter_map(|evaluation| evaluation.grid_cost_units)
        .collect();
    let carbons: Vec<u64> = evaluations
        .iter()
        .filter_map(|evaluation| evaluation.grid_carbon_units)
        .collect();
    let regrets: Vec<u64> = evaluations
        .iter()
        .filter_map(|evaluation| evaluation.regret_vs_realization_oracle_units)
        .collect();
    let feasible_worlds = evaluations.iter().filter(|evaluation| evaluation.feasible).count();
    let sum_cost: u128 = costs.iter().map(|value| u128::from(*value)).sum();
    let sum_carbon: u128 = carbons.iter().map(|value| u128::from(*value)).sum();
    let total = evaluations.len();
    PolicySummary {
        split,
        policy_id: policy_id.to_owned(),
        evaluated_worlds: total,
        feasible_worlds,
        infeasible_worlds: total - feasible_worlds,
        feasible_rate_basis_points: if total == 0 {
            0
        } else {
            ((feasible_worlds as u128 * 10_000) / total as u128) as u32
        },
        hard_constraint_violations,
        mean_cost_units_floor: if costs.is_empty() {
            None
        } else {
            Some((sum_cost / costs.len() as u128) as u64)
        },
        mean_carbon_units_floor: if carbons.is_empty() {
            None
        } else {
            Some((sum_carbon / carbons.len() as u128) as u64)
        },
        cost_regret: tail_metrics(&regrets),
    }
}

fn paired_comparison(worlds: &[WorldResult], split: Split) -> PairedComparison {
    let selected: Vec<&WorldResult> = worlds.iter().filter(|row| row.split == split).collect();
    let both: Vec<(&PolicyEvaluation, &PolicyEvaluation)> = selected
        .iter()
        .filter_map(|row| {
            if row.nominal_policy.feasible && row.robust_policy.feasible {
                Some((&row.nominal_policy, &row.robust_policy))
            } else {
                None
            }
        })
        .collect();
    let delta_sum: i128 = both
        .iter()
        .map(|(left, right)| {
            i128::from(right.grid_cost_units.expect("feasible robust has cost"))
                - i128::from(left.grid_cost_units.expect("feasible nominal has cost"))
        })
        .sum();
    let right_when_left_bad = selected
        .iter()
        .filter(|row| row.robust_policy.feasible && !row.nominal_policy.feasible)
        .count();
    let left_when_right_bad = selected
        .iter()
        .filter(|row| row.nominal_policy.feasible && !row.robust_policy.feasible)
        .count();
    PairedComparison {
        split,
        left_policy_id: "nominal_forecast_cost_minimizer".to_owned(),
        right_policy_id: ROBUST_POLICY_ID.to_owned(),
        paired_worlds_both_feasible: both.len(),
        mean_right_minus_left_grid_cost_units_signed_floor: if both.is_empty() {
            None
        } else {
            Some((delta_sum / both.len() as i128) as i64)
        },
        right_policy_feasible_when_left_infeasible: right_when_left_bad,
        left_policy_feasible_when_right_infeasible: left_when_right_bad,
    }
}

fn make_receipt(source_commit: &str) -> Check<RobustnessReceipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a lowercase 40-character hex SHA".to_owned());
    }
    let model = ProcessModel {
        electricity_units_per_batch: 2,
        water_units_per_batch: 1,
    };
    let cases = generate_cases();
    let serialized_cases = serde_json::to_vec(&cases)
        .map_err(|error| format!("cannot serialize generated case set: {error}"))?;
    let (world_results, hard_constraint_violations) = evaluate_cases(&model, &cases)?;
    let summaries = [
        Split::Validation,
        Split::HeldOut,
    ]
    .into_iter()
    .flat_map(|split| {
        [
            "fixed_baseline",
            "nominal_forecast_cost_minimizer",
            ROBUST_POLICY_ID,
        ]
        .into_iter()
        .map(move |policy_id| {
            summarize_policy(&world_results, split, policy_id, hard_constraint_violations)
        })
    })
    .collect::<Vec<_>>();
    let paired_comparisons = [Split::Validation, Split::HeldOut]
        .into_iter()
        .map(|split| paired_comparison(&world_results, split))
        .collect::<Vec<_>>();
    let robust_policy_feasible_worlds = world_results
        .iter()
        .filter(|row| row.robust_policy.feasible)
        .count();
    let robust_policy_infeasible_worlds = world_results.len() - robust_policy_feasible_worlds;
    let validation_seed_count = VALIDATION_SEEDS.clone().count();
    let held_out_seed_count = HELD_OUT_SEEDS.clone().count();
    let validation_world_result_count = world_results
        .iter()
        .filter(|row| row.split == Split::Validation)
        .count();
    let held_out_world_result_count = world_results
        .iter()
        .filter(|row| row.split == Split::HeldOut)
        .count();
    let all_worlds_accounted = world_results.len()
        == (validation_seed_count + held_out_seed_count) * WORLDS_PER_SEED;
    let robust_policy_meets_floor = robust_policy_infeasible_worlds == 0;
    let summaries_consistent = summaries.iter().all(|summary| {
        summary.evaluated_worlds
            == if summary.split == Split::Validation {
                validation_world_result_count
            } else {
                held_out_world_result_count
            }
            && summary.feasible_worlds + summary.infeasible_worlds == summary.evaluated_worlds
            && summary.feasible_rate_basis_points
                == ((summary.feasible_worlds as u128 * 10_000)
                    / summary.evaluated_worlds as u128) as u32
    });
    let result = if hard_constraint_violations == 0
        && robust_policy_meets_floor
        && all_worlds_accounted
        && summaries_consistent
    {
        "validated"
    } else {
        "control_mismatch"
    };
    Ok(RobustnessReceipt {
        result: result.to_owned(),
        suite_id: "MINE-FLEX-ROBUST-001".to_owned(),
        suite_version: "0.1.0".to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        generated_case_set_sha256: sha256(&serialized_cases),
        validation_seed_count,
        held_out_seed_count,
        uncertainty_worlds_per_seed: WORLDS_PER_SEED,
        validation_world_result_count,
        held_out_world_result_count,
        total_world_result_count: world_results.len(),
        hard_constraint_violations,
        robust_policy_feasible_worlds,
        robust_policy_infeasible_worlds,
        summaries,
        paired_comparisons,
        claim_ceiling: "Generated public synthetic disturbance tournament only; no mine-site cost, carbon, water sufficiency, safety, forecast calibration or industrial performance claim.".to_owned(),
        world_results,
    })
}

fn run_cli(args: &[String]) -> Check<ExitCode> {
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: mine-flex-scheduling-bench --robustness --source-commit <40-hex-sha>".to_owned());
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize robustness receipt: {error}"))?
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
    fn generated_scenario_set_is_deterministic_and_seed_splits_are_disjoint() {
        let first = generate_cases();
        let second = generate_cases();
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        let validation: std::collections::BTreeSet<u64> = first
            .iter()
            .filter(|case| case.split == Split::Validation)
            .map(|case| case.seed)
            .collect();
        let heldout: std::collections::BTreeSet<u64> = first
            .iter()
            .filter(|case| case.split == Split::HeldOut)
            .map(|case| case.seed)
            .collect();
        assert_eq!(32, validation.len());
        assert_eq!(32, heldout.len());
        assert!(validation.is_disjoint(&heldout));
        assert!(first.iter().all(|case| case.uncertainty_envelope.len() == WORLDS_PER_SEED));
    }

    #[test]
    fn robust_policy_is_feasible_across_all_declared_disturbances() {
        for case in generate_cases() {
            let schedule = choose_robust_schedule(&model(), &case.forecast, &case.uncertainty_envelope)
                .expect("a universally feasible reference schedule exists");
            for world in &case.uncertainty_envelope {
                assert!(evaluate_schedule(&model(), &world.scenario, &schedule).is_ok());
            }
        }
    }

    #[test]
    fn nominal_policy_receives_only_forecast_and_fixed_baseline_is_not_tuned_to_realization() {
        let case = generate_case(42, Split::Validation);
        let nominal_a = choose_nominal_schedule(&model(), &case.forecast).expect("nominal schedule");
        let mut reversed = case.uncertainty_envelope.clone();
        reversed.reverse();
        let robust_a = choose_robust_schedule(&model(), &case.forecast, &case.uncertainty_envelope)
            .expect("robust schedule");
        let robust_b = choose_robust_schedule(&model(), &case.forecast, &reversed)
            .expect("robust schedule must not depend on envelope iteration order");
        let nominal_b = choose_nominal_schedule(&model(), &case.forecast).expect("nominal schedule");
        assert_eq!(nominal_a, nominal_b);
        assert_eq!(robust_a, robust_b);
        assert_eq!(FIXED_BASELINE.to_vec(), case.forecast.fixed_baseline_batches);
    }

    #[test]
    fn every_feasible_policy_and_oracle_score_matches_independent_reconstruction() {
        let cases = generate_cases();
        let (rows, violations) = evaluate_cases(&model(), &cases).expect("tournament evaluates");
        assert_eq!(0, violations);
        assert_eq!(320, rows.len());
        for row in &rows {
            assert!(row.robust_policy.feasible);
            assert_eq!(
                Some(row.oracle_grid_cost_units),
                row.robust_policy.regret_vs_realization_oracle_units
                    .map(|regret| row.robust_policy.grid_cost_units.expect("feasible cost") - regret)
            );
            for policy in [&row.fixed_baseline, &row.nominal_policy, &row.robust_policy] {
                if policy.feasible {
                    let scenario = cases
                        .iter()
                        .find(|case| case.seed == row.seed && case.split == row.split)
                        .and_then(|case| {
                            case.uncertainty_envelope
                                .iter()
                                .find(|world| world.realization_id == row.realization_id)
                        })
                        .expect("world is in the frozen case set");
                    let batches = policy.schedule_batches.as_ref().expect("feasible schedule");
                    assert_eq!(
                        evaluate_schedule(&model(), &scenario.scenario, batches)
                            .expect("reported feasible")
                            .total_grid_cost_units,
                        policy.grid_cost_units.expect("feasible cost")
                    );
                }
            }
        }
    }

    #[test]
    fn receipt_keeps_validation_and_heldout_scores_separate_and_reports_tails() {
        let receipt_a = make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("valid source commit");
        let receipt_b = make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("same receipt");
        assert_eq!(
            serde_json::to_vec(&receipt_a).expect("serialize"),
            serde_json::to_vec(&receipt_b).expect("serialize")
        );
        assert_eq!("validated", receipt_a.result);
        assert_eq!(0, receipt_a.hard_constraint_violations);
        assert_eq!(320, receipt_a.total_world_result_count);
        assert_eq!(160, receipt_a.validation_world_result_count);
        assert_eq!(160, receipt_a.held_out_world_result_count);
        assert_eq!(320, receipt_a.robust_policy_feasible_worlds);
        assert_eq!(0, receipt_a.robust_policy_infeasible_worlds);
        for summary in &receipt_a.summaries {
            assert_eq!(160, summary.evaluated_worlds);
            assert_eq!(summary.evaluated_worlds, summary.feasible_worlds + summary.infeasible_worlds);
            if summary.cost_regret.sample_count > 0 {
                assert!(summary.cost_regret.mean_regret_units_floor.is_some());
                assert!(summary.cost_regret.p95_regret_units_nearest_rank.is_some());
                assert!(summary.cost_regret.cvar95_regret_units_worst_five_percent_mean_floor.is_some());
            }
        }
        for pair in &receipt_a.paired_comparisons {
            assert_eq!(Split::Validation == pair.split, pair.split == Split::Validation);
            assert!(pair.paired_worlds_both_feasible > 0);
        }
    }

    #[test]
    fn tail_metrics_use_nearest_rank_and_worst_five_percent() {
        let metrics = tail_metrics(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100]);
        assert_eq!(Some(100), metrics.worst_regret_units);
        assert_eq!(Some(100), metrics.p95_regret_units_nearest_rank);
        assert_eq!(Some(100), metrics.cvar95_regret_units_worst_five_percent_mean_floor);
        assert_eq!(Some(20), tail_metrics(&[10, 10, 20]).cvar95_regret_units_worst_five_percent_mean_floor);
    }
}
