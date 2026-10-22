use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const MAX_PERIODS: usize = 8;
const MAX_SEARCH_SPACE: usize = 200_000;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ScenarioStatus {
    Feasible,
    InfeasibleSafetyFloor,
    InfeasibleProtectedWaterReserve,
    InfeasibleNoSchedule,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    process_model: ProcessModel,
    scenarios: Vec<ScenarioInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProcessModel {
    electricity_units_per_batch: u32,
    water_units_per_batch: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScenarioInput {
    scenario_id: String,
    production_target_batches: u32,
    periods: Vec<PeriodInput>,
    fixed_baseline_batches: Vec<u32>,
    expected: ScenarioExpectation,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PeriodInput {
    period_id: String,
    safety_electricity_units: u32,
    renewable_electricity_units: u32,
    grid_import_cap_units: u32,
    grid_price_units_per_import_unit: u32,
    grid_carbon_units_per_import_unit: u32,
    water_available_units: u32,
    protected_water_reserve_units: u32,
    max_process_batches: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScenarioExpectation {
    status: ScenarioStatus,
    candidate_schedules_evaluated: usize,
    feasible_schedule_count: usize,
    best_schedule_batches: Option<Vec<u32>>,
    best_grid_cost_units: Option<u64>,
    best_grid_carbon_units: Option<u64>,
    fixed_baseline_grid_cost_units: Option<u64>,
    fixed_baseline_grid_carbon_units: Option<u64>,
    pareto_frontier: Vec<FrontierPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FrontierPoint {
    batches: Vec<u32>,
    grid_cost_units: u64,
    grid_carbon_units: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PeriodBalance {
    period_id: String,
    safety_electricity_units: u64,
    process_electricity_units: u64,
    total_electricity_demand_units: u64,
    renewable_electricity_used_units: u64,
    grid_import_units: u64,
    grid_cost_units: u64,
    grid_carbon_units: u64,
    water_available_units: u64,
    protected_water_reserve_units: u64,
    process_water_units: u64,
    remaining_water_units: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ScheduleScore {
    batches: Vec<u32>,
    period_balances: Vec<PeriodBalance>,
    total_grid_cost_units: u64,
    total_grid_carbon_units: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ScenarioOutcome {
    scenario_id: String,
    status: ScenarioStatus,
    reason_code: Option<String>,
    search_space_size: usize,
    candidate_schedules_evaluated: usize,
    feasible_schedule_count: usize,
    best_schedule: Option<ScheduleScore>,
    fixed_baseline_schedule: Option<ScheduleScore>,
    feasible_schedules: Vec<ScheduleScore>,
    pareto_frontier: Vec<ScheduleScore>,
    expected_control_matched: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    evaluator_version: String,
    source_commit: String,
    corpus_sha256: String,
    scenario_count: usize,
    expected_controls_matched: usize,
    independent_checks_passed: bool,
    hard_constraint_violations: usize,
    scenarios: Vec<ScenarioOutcome>,
    claim_ceiling: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn search_space_size(periods: &[PeriodInput]) -> Check<usize> {
    periods.iter().try_fold(1_usize, |product, period| {
        product
            .checked_mul(period.max_process_batches as usize + 1)
            .filter(|value| *value <= MAX_SEARCH_SPACE)
            .ok_or_else(|| "search space exceeds the bounded exact-oracle limit".to_owned())
    })
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.suite_id != "MINE-FLEX-001"
        || corpus.suite_version != VERSION
        || corpus.evidence_class != "public_authored_synthetic_dispatch_case"
        || !nonempty(&corpus.claim_ceiling)
        || corpus.scenarios.len() != 1
        || corpus.process_model.electricity_units_per_batch == 0
        || corpus.process_model.water_units_per_batch == 0
    {
        return Err(
            "invalid suite identity, evidence class, process model, or frozen scenario count"
                .to_owned(),
        );
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if !nonempty(&scenario.scenario_id)
            || !scenario_ids.insert(scenario.scenario_id.clone())
            || scenario.production_target_batches == 0
            || scenario.periods.len() < 2
            || scenario.periods.len() > MAX_PERIODS
            || scenario.fixed_baseline_batches.len() != scenario.periods.len()
        {
            return Err(
                "scenario ID, production target, period count, or baseline length is invalid"
                    .to_owned(),
            );
        }

        let mut period_ids = BTreeSet::new();
        for period in &scenario.periods {
            if !nonempty(&period.period_id)
                || !period_ids.insert(period.period_id.clone())
                || period.safety_electricity_units == 0
                || period.max_process_batches > 32
            {
                return Err(
                    "period ID, declared safety floor, or process-batch limit is invalid"
                        .to_owned(),
                );
            }
        }
        let _ = search_space_size(&scenario.periods)?;
    }
    Ok(())
}

fn enumerate_vectors_recursive(
    periods: &[PeriodInput],
    index: usize,
    current: &mut Vec<u32>,
    out: &mut Vec<Vec<u32>>,
) {
    if index == periods.len() {
        out.push(current.clone());
        return;
    }
    for batches in 0..=periods[index].max_process_batches {
        current.push(batches);
        enumerate_vectors_recursive(periods, index + 1, current, out);
        current.pop();
    }
}

fn enumerate_vectors(periods: &[PeriodInput]) -> Check<Vec<Vec<u32>>> {
    let expected = search_space_size(periods)?;
    let mut out = Vec::with_capacity(expected);
    enumerate_vectors_recursive(periods, 0, &mut Vec::new(), &mut out);
    if out.len() != expected {
        return Err("internal enumeration count mismatch".to_owned());
    }
    Ok(out)
}

fn evaluate_schedule(
    model: &ProcessModel,
    scenario: &ScenarioInput,
    batches: &[u32],
) -> Check<ScheduleScore> {
    if batches.len() != scenario.periods.len() {
        return Err("schedule length does not match the declared period count".to_owned());
    }
    let total_batches: u64 = batches.iter().map(|value| u64::from(*value)).sum();
    if total_batches != u64::from(scenario.production_target_batches) {
        return Err("schedule does not meet the exact production target".to_owned());
    }

    let mut total_cost = 0_u64;
    let mut total_carbon = 0_u64;
    let mut balances = Vec::with_capacity(scenario.periods.len());

    for (period, batch_count) in scenario.periods.iter().zip(batches.iter().copied()) {
        if batch_count > period.max_process_batches {
            return Err(format!("process capacity exceeded in {}", period.period_id));
        }
        if period.water_available_units < period.protected_water_reserve_units {
            return Err(format!(
                "protected water reserve unavailable in {}",
                period.period_id
            ));
        }

        let process_electricity = u64::from(batch_count)
            .checked_mul(u64::from(model.electricity_units_per_batch))
            .ok_or_else(|| "process electricity multiplication overflows".to_owned())?;
        let demand = u64::from(period.safety_electricity_units)
            .checked_add(process_electricity)
            .ok_or_else(|| "electricity demand overflows".to_owned())?;
        let renewable = u64::from(period.renewable_electricity_units);
        let grid_import = demand.saturating_sub(renewable);
        if grid_import > u64::from(period.grid_import_cap_units) {
            return Err(format!("grid import cap exceeded in {}", period.period_id));
        }

        let process_water = u64::from(batch_count)
            .checked_mul(u64::from(model.water_units_per_batch))
            .ok_or_else(|| "process water multiplication overflows".to_owned())?;
        let available_for_process = u64::from(period.water_available_units)
            - u64::from(period.protected_water_reserve_units);
        if process_water > available_for_process {
            return Err(format!(
                "protected water reserve would be diverted in {}",
                period.period_id
            ));
        }

        let cost = grid_import
            .checked_mul(u64::from(period.grid_price_units_per_import_unit))
            .ok_or_else(|| "grid cost multiplication overflows".to_owned())?;
        let carbon = grid_import
            .checked_mul(u64::from(period.grid_carbon_units_per_import_unit))
            .ok_or_else(|| "grid carbon multiplication overflows".to_owned())?;
        total_cost = total_cost
            .checked_add(cost)
            .ok_or_else(|| "total grid cost overflows".to_owned())?;
        total_carbon = total_carbon
            .checked_add(carbon)
            .ok_or_else(|| "total grid carbon overflows".to_owned())?;

        balances.push(PeriodBalance {
            period_id: period.period_id.clone(),
            safety_electricity_units: u64::from(period.safety_electricity_units),
            process_electricity_units: process_electricity,
            total_electricity_demand_units: demand,
            renewable_electricity_used_units: demand.min(renewable),
            grid_import_units: grid_import,
            grid_cost_units: cost,
            grid_carbon_units: carbon,
            water_available_units: u64::from(period.water_available_units),
            protected_water_reserve_units: u64::from(period.protected_water_reserve_units),
            process_water_units: process_water,
            remaining_water_units: u64::from(period.water_available_units) - process_water,
        });
    }

    Ok(ScheduleScore {
        batches: batches.to_vec(),
        period_balances: balances,
        total_grid_cost_units: total_cost,
        total_grid_carbon_units: total_carbon,
    })
}

fn score_order(a: &ScheduleScore, b: &ScheduleScore) -> std::cmp::Ordering {
    a.total_grid_cost_units
        .cmp(&b.total_grid_cost_units)
        .then_with(|| a.total_grid_carbon_units.cmp(&b.total_grid_carbon_units))
        .then_with(|| a.batches.cmp(&b.batches))
}

fn frontier_schedules(feasible: &[ScheduleScore]) -> Vec<ScheduleScore> {
    let mut frontier = feasible
        .iter()
        .filter(|candidate| {
            !feasible.iter().any(|other| {
                if candidate.batches == other.batches {
                    return false;
                }
                let no_worse = other.total_grid_cost_units <= candidate.total_grid_cost_units
                    && other.total_grid_carbon_units <= candidate.total_grid_carbon_units;
                let strictly_better = other.total_grid_cost_units < candidate.total_grid_cost_units
                    || other.total_grid_carbon_units < candidate.total_grid_carbon_units
                    || other.batches < candidate.batches;
                no_worse && strictly_better
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    frontier.sort_by(score_order);
    frontier
}

fn early_infeasibility(scenario: &ScenarioInput) -> Option<(ScenarioStatus, String)> {
    if scenario
        .periods
        .iter()
        .any(|period| period.water_available_units < period.protected_water_reserve_units)
    {
        return Some((
            ScenarioStatus::InfeasibleProtectedWaterReserve,
            "PROTECTED_WATER_RESERVE_UNMET".to_owned(),
        ));
    }
    if scenario.periods.iter().any(|period| {
        u64::from(period.safety_electricity_units)
            > u64::from(period.renewable_electricity_units)
                + u64::from(period.grid_import_cap_units)
    }) {
        return Some((
            ScenarioStatus::InfeasibleSafetyFloor,
            "SAFETY_FLOOR_UNSERVICEABLE".to_owned(),
        ));
    }
    None
}

fn expected_matches(expected: &ScenarioExpectation, outcome: &ScenarioOutcome) -> bool {
    let best_batches = outcome
        .best_schedule
        .as_ref()
        .map(|score| score.batches.clone());
    let best_cost = outcome
        .best_schedule
        .as_ref()
        .map(|score| score.total_grid_cost_units);
    let best_carbon = outcome
        .best_schedule
        .as_ref()
        .map(|score| score.total_grid_carbon_units);
    let baseline_cost = outcome
        .fixed_baseline_schedule
        .as_ref()
        .map(|score| score.total_grid_cost_units);
    let baseline_carbon = outcome
        .fixed_baseline_schedule
        .as_ref()
        .map(|score| score.total_grid_carbon_units);
    let frontier = outcome
        .pareto_frontier
        .iter()
        .map(|score| FrontierPoint {
            batches: score.batches.clone(),
            grid_cost_units: score.total_grid_cost_units,
            grid_carbon_units: score.total_grid_carbon_units,
        })
        .collect::<Vec<_>>();

    expected.status == outcome.status
        && expected.candidate_schedules_evaluated == outcome.candidate_schedules_evaluated
        && expected.feasible_schedule_count == outcome.feasible_schedule_count
        && expected.best_schedule_batches == best_batches
        && expected.best_grid_cost_units == best_cost
        && expected.best_grid_carbon_units == best_carbon
        && expected.fixed_baseline_grid_cost_units == baseline_cost
        && expected.fixed_baseline_grid_carbon_units == baseline_carbon
        && expected.pareto_frontier == frontier
}

fn evaluate_scenario(model: &ProcessModel, scenario: &ScenarioInput) -> Check<ScenarioOutcome> {
    let search_size = search_space_size(&scenario.periods)?;
    if let Some((status, reason)) = early_infeasibility(scenario) {
        let mut outcome = ScenarioOutcome {
            scenario_id: scenario.scenario_id.clone(),
            status,
            reason_code: Some(reason),
            search_space_size: search_size,
            candidate_schedules_evaluated: 0,
            feasible_schedule_count: 0,
            best_schedule: None,
            fixed_baseline_schedule: None,
            feasible_schedules: Vec::new(),
            pareto_frontier: Vec::new(),
            expected_control_matched: false,
        };
        outcome.expected_control_matched = expected_matches(&scenario.expected, &outcome);
        return Ok(outcome);
    }

    let vectors = enumerate_vectors(&scenario.periods)?;
    let mut candidate_schedules_evaluated = 0_usize;
    let mut feasible_schedules = Vec::new();
    for vector in vectors {
        let total: u64 = vector.iter().map(|value| u64::from(*value)).sum();
        if total != u64::from(scenario.production_target_batches) {
            continue;
        }
        candidate_schedules_evaluated += 1;
        if let Ok(score) = evaluate_schedule(model, scenario, &vector) {
            feasible_schedules.push(score);
        }
    }
    feasible_schedules.sort_by(|a, b| a.batches.cmp(&b.batches));

    if feasible_schedules.is_empty() {
        let mut outcome = ScenarioOutcome {
            scenario_id: scenario.scenario_id.clone(),
            status: ScenarioStatus::InfeasibleNoSchedule,
            reason_code: Some("NO_FEASIBLE_SCHEDULE".to_owned()),
            search_space_size: search_size,
            candidate_schedules_evaluated,
            feasible_schedule_count: 0,
            best_schedule: None,
            fixed_baseline_schedule: evaluate_schedule(
                model,
                scenario,
                &scenario.fixed_baseline_batches,
            )
            .ok(),
            feasible_schedules,
            pareto_frontier: Vec::new(),
            expected_control_matched: false,
        };
        outcome.expected_control_matched = expected_matches(&scenario.expected, &outcome);
        return Ok(outcome);
    }

    let best_schedule = feasible_schedules
        .iter()
        .min_by(|a, b| score_order(a, b))
        .cloned();
    let fixed_baseline_schedule =
        evaluate_schedule(model, scenario, &scenario.fixed_baseline_batches).ok();
    let pareto_frontier = frontier_schedules(&feasible_schedules);
    let mut outcome = ScenarioOutcome {
        scenario_id: scenario.scenario_id.clone(),
        status: ScenarioStatus::Feasible,
        reason_code: None,
        search_space_size: search_size,
        candidate_schedules_evaluated,
        feasible_schedule_count: feasible_schedules.len(),
        best_schedule,
        fixed_baseline_schedule,
        feasible_schedules,
        pareto_frontier,
        expected_control_matched: false,
    };
    outcome.expected_control_matched = expected_matches(&scenario.expected, &outcome);
    Ok(outcome)
}

// This verifier reconstructs the schedule ledger from the original input, rather than trusting
// the optimizer's precomputed PeriodBalance records.
fn independently_reconstruct_score(
    model: &ProcessModel,
    scenario: &ScenarioInput,
    batches: &[u32],
) -> Check<ScheduleScore> {
    if batches.len() != scenario.periods.len()
        || batches.iter().map(|value| u64::from(*value)).sum::<u64>()
            != u64::from(scenario.production_target_batches)
    {
        return Err("independent reconstruction rejected schedule shape/production".to_owned());
    }
    let mut balances = Vec::new();
    let mut cost_total = 0_u64;
    let mut carbon_total = 0_u64;
    for (input, count) in scenario.periods.iter().zip(batches.iter().copied()) {
        if count > input.max_process_batches
            || input.water_available_units < input.protected_water_reserve_units
        {
            return Err(
                "independent reconstruction found a period hard-constraint failure".to_owned(),
            );
        }
        let safety = u64::from(input.safety_electricity_units);
        let process_power = u64::from(count) * u64::from(model.electricity_units_per_batch);
        let demand = safety + process_power;
        let renewable_available = u64::from(input.renewable_electricity_units);
        let grid = demand.saturating_sub(renewable_available);
        if grid > u64::from(input.grid_import_cap_units)
            || safety > renewable_available + u64::from(input.grid_import_cap_units)
        {
            return Err(
                "independent reconstruction found an electrical capacity violation".to_owned(),
            );
        }
        let process_water = u64::from(count) * u64::from(model.water_units_per_batch);
        let total_water = u64::from(input.water_available_units);
        let protected_water = u64::from(input.protected_water_reserve_units);
        if process_water > total_water - protected_water {
            return Err("independent reconstruction found a water reserve violation".to_owned());
        }
        let period_cost = grid * u64::from(input.grid_price_units_per_import_unit);
        let period_carbon = grid * u64::from(input.grid_carbon_units_per_import_unit);
        cost_total += period_cost;
        carbon_total += period_carbon;
        balances.push(PeriodBalance {
            period_id: input.period_id.clone(),
            safety_electricity_units: safety,
            process_electricity_units: process_power,
            total_electricity_demand_units: demand,
            renewable_electricity_used_units: demand.min(renewable_available),
            grid_import_units: grid,
            grid_cost_units: period_cost,
            grid_carbon_units: period_carbon,
            water_available_units: total_water,
            protected_water_reserve_units: protected_water,
            process_water_units: process_water,
            remaining_water_units: total_water - process_water,
        });
    }
    Ok(ScheduleScore {
        batches: batches.to_vec(),
        period_balances: balances,
        total_grid_cost_units: cost_total,
        total_grid_carbon_units: carbon_total,
    })
}

fn independently_verify_score(
    model: &ProcessModel,
    scenario: &ScenarioInput,
    score: &ScheduleScore,
) -> bool {
    independently_reconstruct_score(model, scenario, &score.batches)
        .is_ok_and(|reconstructed| reconstructed == *score)
}

fn hard_constraint_violation_count(
    model: &ProcessModel,
    scenarios: &[ScenarioInput],
    outcomes: &[ScenarioOutcome],
) -> usize {
    let mut violations = 0;
    for (scenario, outcome) in scenarios.iter().zip(outcomes) {
        let mut scores = Vec::new();
        if let Some(score) = &outcome.best_schedule {
            scores.push(score);
        }
        if let Some(score) = &outcome.fixed_baseline_schedule {
            scores.push(score);
        }
        scores.extend(outcome.feasible_schedules.iter());
        scores.extend(outcome.pareto_frontier.iter());
        for score in scores {
            if !independently_verify_score(model, scenario, score) {
                violations += 1;
            }
        }
        if outcome.status == ScenarioStatus::Feasible
            && (outcome.best_schedule.is_none()
                || outcome.feasible_schedule_count != outcome.feasible_schedules.len())
        {
            violations += 1;
        }
        if outcome.status != ScenarioStatus::Feasible
            && (outcome.best_schedule.is_some() || !outcome.pareto_frontier.is_empty())
        {
            violations += 1;
        }
    }
    violations
}

fn make_receipt(corpus: &Corpus, raw_bytes: &[u8], source_commit: &str) -> Check<Receipt> {
    validate_corpus(corpus)?;
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a lowercase 40-character hex SHA".to_owned());
    }
    let scenarios = corpus
        .scenarios
        .iter()
        .map(|scenario| evaluate_scenario(&corpus.process_model, scenario))
        .collect::<Check<Vec<_>>>()?;
    let expected_controls_matched = scenarios
        .iter()
        .filter(|outcome| outcome.expected_control_matched)
        .count();
    let hard_constraint_violations =
        hard_constraint_violation_count(&corpus.process_model, &corpus.scenarios, &scenarios);
    let independent_checks_passed = hard_constraint_violations == 0;
    let all_controls_matched = expected_controls_matched == corpus.scenarios.len();
    Ok(Receipt {
        result: if all_controls_matched && independent_checks_passed {
            "validated"
        } else {
            "control_mismatch"
        }
        .to_owned(),
        suite_id: corpus.suite_id.clone(),
        suite_version: corpus.suite_version.clone(),
        evaluator_version: VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        corpus_sha256: sha256(raw_bytes),
        scenario_count: scenarios.len(),
        expected_controls_matched,
        independent_checks_passed,
        hard_constraint_violations,
        scenarios,
        claim_ceiling: corpus.claim_ceiling.clone(),
    })
}

fn run() -> Check<ExitCode> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: mine-flex-scheduling-bench <corpus.json> --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let corpus_path = &args[0];
    let source_commit = &args[2];
    let bytes = fs::read(corpus_path)
        .map_err(|error| format!("cannot read corpus {corpus_path}: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    let receipt = make_receipt(&corpus, &bytes, source_commit)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );
    Ok(if receipt.result == "validated" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        serde_json::from_str(include_str!(
            "../../../docs/tech/mine-flex-scheduling-corpus-v0.1.json"
        ))
        .expect("frozen synthetic flex scheduling corpus should deserialize")
    }

    #[test]
    fn frozen_known_answer_matches_exact_cost_carbon_and_frontier() {
        let corpus = corpus();
        validate_corpus(&corpus).expect("fixture contract should validate");
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(ScenarioStatus::Feasible, outcome.status);
        assert_eq!(8, outcome.candidate_schedules_evaluated);
        assert_eq!(5, outcome.feasible_schedule_count);
        let best = outcome.best_schedule.expect("feasible optimum");
        assert_eq!(vec![1, 3, 0], best.batches);
        assert_eq!(410, best.total_grid_cost_units);
        assert_eq!(152, best.total_grid_carbon_units);
        let baseline = outcome
            .fixed_baseline_schedule
            .expect("fixed baseline is feasible");
        assert_eq!(vec![2, 1, 1], baseline.batches);
        assert_eq!(590, baseline.total_grid_cost_units);
        assert_eq!(100, baseline.total_grid_carbon_units);
        assert!(outcome.expected_control_matched);
        assert_eq!(4, outcome.pareto_frontier.len());
    }

    #[test]
    fn every_reported_feasible_schedule_reconstructs_independently() {
        let corpus = corpus();
        let scenario = &corpus.scenarios[0];
        let outcome = evaluate_scenario(&corpus.process_model, scenario).expect("scenario");
        for score in &outcome.feasible_schedules {
            assert!(independently_verify_score(
                &corpus.process_model,
                scenario,
                score
            ));
        }
        for score in &outcome.pareto_frontier {
            assert!(independently_verify_score(
                &corpus.process_model,
                scenario,
                score
            ));
            assert!(score.period_balances.iter().all(|balance| {
                balance.remaining_water_units >= balance.protected_water_reserve_units
            }));
        }
    }

    #[test]
    fn each_frontier_member_is_not_dominated_by_another_member() {
        let corpus = corpus();
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        for candidate in &outcome.pareto_frontier {
            for other in &outcome.pareto_frontier {
                if candidate.batches == other.batches {
                    continue;
                }
                assert!(
                    !(other.total_grid_cost_units <= candidate.total_grid_cost_units
                        && other.total_grid_carbon_units <= candidate.total_grid_carbon_units
                        && (other.total_grid_cost_units < candidate.total_grid_cost_units
                            || other.total_grid_carbon_units < candidate.total_grid_carbon_units
                            || other.batches < candidate.batches)),
                    "frontier contains a dominated member"
                );
            }
        }
    }

    #[test]
    fn grid_import_cap_shortfall_changes_the_feasible_frontier() {
        let mut corpus = corpus();
        corpus.scenarios[0].periods[1].grid_import_cap_units = 3;
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(ScenarioStatus::Feasible, outcome.status);
        assert_eq!(3, outcome.feasible_schedule_count);
        assert_eq!(
            vec![2, 2, 0],
            outcome.best_schedule.expect("still feasible").batches
        );
    }

    #[test]
    fn water_shortfall_never_borrows_from_the_protected_reserve() {
        let mut corpus = corpus();
        corpus.scenarios[0].periods[1].water_available_units = 4;
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(ScenarioStatus::Feasible, outcome.status);
        assert_eq!(3, outcome.feasible_schedule_count);
        assert!(outcome
            .feasible_schedules
            .iter()
            .all(|score| score.period_balances.iter().all(|balance| {
                balance.remaining_water_units >= balance.protected_water_reserve_units
            })));
    }

    #[test]
    fn unserviceable_safety_load_is_reported_as_infeasible_not_shed() {
        let mut corpus = corpus();
        corpus.scenarios[0].periods[2].renewable_electricity_units = 0;
        corpus.scenarios[0].periods[2].grid_import_cap_units = 3;
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(ScenarioStatus::InfeasibleSafetyFloor, outcome.status);
        assert_eq!(
            Some("SAFETY_FLOOR_UNSERVICEABLE".to_owned()),
            outcome.reason_code
        );
        assert!(outcome.best_schedule.is_none());
    }

    #[test]
    fn insufficient_water_for_the_production_target_is_explicitly_infeasible() {
        let mut corpus = corpus();
        corpus.scenarios[0].periods[1].water_available_units = 3;
        corpus.scenarios[0].periods[2].water_available_units = 2;
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(ScenarioStatus::InfeasibleNoSchedule, outcome.status);
        assert_eq!(Some("NO_FEASIBLE_SCHEDULE".to_owned()), outcome.reason_code);
        assert!(outcome.best_schedule.is_none());
        assert!(outcome.pareto_frontier.is_empty());
    }

    #[test]
    fn unmet_protected_water_reserve_is_a_hard_stop() {
        let mut corpus = corpus();
        corpus.scenarios[0].periods[2].water_available_units = 1;
        let outcome =
            evaluate_scenario(&corpus.process_model, &corpus.scenarios[0]).expect("scenario");
        assert_eq!(
            ScenarioStatus::InfeasibleProtectedWaterReserve,
            outcome.status
        );
        assert_eq!(
            Some("PROTECTED_WATER_RESERVE_UNMET".to_owned()),
            outcome.reason_code
        );
    }

    #[test]
    fn unknown_corpus_fields_fail_closed() {
        let contents = include_str!("../../../docs/tech/mine-flex-scheduling-corpus-v0.1.json")
            .replace(
                "\"suite_id\": \"MINE-FLEX-001\"",
                "\"suite_id\": \"MINE-FLEX-001\", \"unrecognized\": true",
            );
        assert!(serde_json::from_str::<Corpus>(&contents).is_err());
    }
}
