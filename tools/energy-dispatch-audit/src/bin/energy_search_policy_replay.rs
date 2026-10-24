//! Matched unique-proposal budget replay over the frozen 39-candidate domain.
//!
//! This consumes outcomes from the already-qualified end-to-end matrix. It is
//! a discovery-quality replay, not a wall-clock benchmark: it does not rerun
//! dispatch, and it must not be presented as evidence that an ordering policy
//! improves actual runtime. Full-domain envelopes provide the finite-domain
//! oracle for frontier recall and axis-wise gaps.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs,
    io,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const LEDGER_SCHEMA: &str = "energy-studio-benchmark-report-v0.6";
const ROBUSTNESS_SCHEMA: &str = "energy-candidate-robustness-report-v0.1";
const MATRIX_SCHEMA: &str = "energy-candidate-scenario-matrix-report-v0.2";
const BUDGETS: [usize; 6] = [1, 3, 6, 12, 24, 39];

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}
fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("missing string field {key}")))
}
fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("missing unsigned field {key}")))
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct AxisBest {
    essential_unserved_quanta: u64,
    flexible_unserved_quanta: u64,
    operating_cost_units: u64,
    grid_carbon_grams: u64,
    curtailed_pv_quanta: u64,
    terminal_soc_quanta: u16,
}

#[derive(Clone, Debug, Deserialize)]
struct ScenarioProfile {
    scenario_id: String,
    frontier_count: usize,
    frontier_objectives: Vec<Value>,
    axis_best: AxisBest,
}

#[derive(Clone, Debug, Deserialize)]
struct CandidateProfile {
    candidate_id: String,
    scenario_profiles: Vec<ScenarioProfile>,
    worst_case_of_per_scenario_best_axis_value: AxisBest,
}

#[derive(Clone, Debug, Serialize)]
struct AxisGaps {
    essential_unserved_quanta_gap: u64,
    flexible_unserved_quanta_gap: u64,
    operating_cost_units_gap: u64,
    grid_carbon_grams_gap: u64,
    curtailed_pv_quanta_gap: u64,
    terminal_soc_quanta_gap: u64,
    semantics: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct BudgetPoint {
    unique_proposal_budget: usize,
    unique_proposals: usize,
    duplicate_proposals: usize,
    selected_candidate_ids: Vec<String>,
    discovered_envelope_frontier_vector_count: usize,
    full_domain_oracle_frontier_vector_count: usize,
    oracle_frontier_recall_numerator: usize,
    oracle_frontier_recall_denominator: usize,
    axis_best_gap: AxisGaps,
    best_discovered_worst_case_essential_unserved_quanta: u64,
}

#[derive(Clone, Debug, Serialize)]
struct PolicyRun {
    policy_id: String,
    seed: Option<u64>,
    candidate_order_sha256: String,
    candidate_order: Vec<String>,
    budget_curve: Vec<BudgetPoint>,
}

#[derive(Clone, Debug, Serialize)]
struct QuantileSummaryU64 {
    min: u64,
    p10: u64,
    p50: u64,
    p90: u64,
    max: u64,
}

#[derive(Clone, Debug, Serialize)]
struct RandomSeedBudgetSummary {
    unique_proposal_budget: usize,
    random_seed_count: usize,
    oracle_frontier_recall_numerator: QuantileSummaryU64,
    worst_case_essential_unserved_quanta: QuantileSummaryU64,
    essential_unserved_axis_gap: QuantileSummaryU64,
    operating_cost_axis_gap: QuantileSummaryU64,
}

#[derive(Clone, Debug, Serialize)]
struct SearchPolicyReplayReport {
    schema_version: &'static str,
    status: &'static str,
    benchmark_id: &'static str,
    source_ledger_sha256: String,
    source_matrix_sha256: String,
    source_robustness_report_sha256: String,
    source_design_space_manifest_sha256: String,
    candidate_count: usize,
    scenario_count: usize,
    proposal_budget_tiers: Vec<usize>,
    full_domain_oracle_frontier_vector_count: usize,
    random_seed_budget_summary: Vec<RandomSeedBudgetSummary>,
    policy_runs: Vec<PolicyRun>,
    measurement_scope: &'static str,
    claim_ceiling: &'static str,
}

fn expected_candidate_ids() -> Vec<String> {
    let mut ids = vec![
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned(),
    ];
    for pv in [1, 2, 3] {
        for capacity in [1, 2, 3] {
            for battery_power in [1, 2] {
                for grid_power in [1, 2] {
                    ids.push(format!(
                        "synthetic-parametric-pv-{pv}kw-battery-{capacity}kwh-battery-power-{battery_power}kw-grid-{grid_power}kw"
                    ));
                }
            }
        }
    }
    ids.sort();
    ids
}

fn dominates(left: &AxisBest, right: &AxisBest) -> bool {
    let no_worse = left.essential_unserved_quanta <= right.essential_unserved_quanta
        && left.flexible_unserved_quanta <= right.flexible_unserved_quanta
        && left.operating_cost_units <= right.operating_cost_units
        && left.grid_carbon_grams <= right.grid_carbon_grams
        && left.curtailed_pv_quanta <= right.curtailed_pv_quanta
        && left.terminal_soc_quanta >= right.terminal_soc_quanta;
    let strict = left.essential_unserved_quanta < right.essential_unserved_quanta
        || left.flexible_unserved_quanta < right.flexible_unserved_quanta
        || left.operating_cost_units < right.operating_cost_units
        || left.grid_carbon_grams < right.grid_carbon_grams
        || left.curtailed_pv_quanta < right.curtailed_pv_quanta
        || left.terminal_soc_quanta > right.terminal_soc_quanta;
    no_worse && strict
}

fn unique_envelope_frontier(envelopes: &[AxisBest]) -> BTreeSet<AxisBest> {
    let unique = envelopes.iter().cloned().collect::<BTreeSet<_>>();
    unique.iter()
        .filter(|candidate| !unique.iter().any(|other| dominates(other, candidate)))
        .cloned()
        .collect()
}

fn axis_best_gaps(selected: &[AxisBest], oracle: &[AxisBest]) -> Result<AxisGaps, Box<dyn Error>> {
    if selected.is_empty() || oracle.is_empty() {
        return Err(invalid("axis-gap calculation requires selected candidates and a non-empty oracle"));
    }
    let selected_min = |f: fn(&AxisBest) -> u64| selected.iter().map(f).min().unwrap();
    let oracle_min = |f: fn(&AxisBest) -> u64| oracle.iter().map(f).min().unwrap();
    let selected_essential = selected_min(|x| x.essential_unserved_quanta);
    let oracle_essential = oracle_min(|x| x.essential_unserved_quanta);
    let selected_flexible = selected_min(|x| x.flexible_unserved_quanta);
    let oracle_flexible = oracle_min(|x| x.flexible_unserved_quanta);
    let selected_cost = selected_min(|x| x.operating_cost_units);
    let oracle_cost = oracle_min(|x| x.operating_cost_units);
    let selected_carbon = selected_min(|x| x.grid_carbon_grams);
    let oracle_carbon = oracle_min(|x| x.grid_carbon_grams);
    let selected_curtailment = selected_min(|x| x.curtailed_pv_quanta);
    let oracle_curtailment = oracle_min(|x| x.curtailed_pv_quanta);
    let selected_soc = selected.iter().map(|x| x.terminal_soc_quanta).max().unwrap();
    let oracle_soc = oracle.iter().map(|x| x.terminal_soc_quanta).max().unwrap();

    Ok(AxisGaps {
        essential_unserved_quanta_gap: selected_essential - oracle_essential,
        flexible_unserved_quanta_gap: selected_flexible - oracle_flexible,
        operating_cost_units_gap: selected_cost - oracle_cost,
        grid_carbon_grams_gap: selected_carbon - oracle_carbon,
        curtailed_pv_quanta_gap: selected_curtailment - oracle_curtailment,
        terminal_soc_quanta_gap: u64::from(oracle_soc - selected_soc),
        semantics: "Coordinate-wise gaps against the full finite-domain envelope oracle; different objective gaps can be attained by different candidates and do not identify one jointly optimal design.",
    })
}

fn curve_for_order(
    order: &[String],
    envelopes: &BTreeMap<String, AxisBest>,
    oracle_frontier: &BTreeSet<AxisBest>,
    oracle_envelopes: &[AxisBest],
) -> Result<Vec<BudgetPoint>, Box<dyn Error>> {
    let mut curve = Vec::with_capacity(BUDGETS.len());
    for budget in BUDGETS {
        let selected_ids = order.iter().take(budget).cloned().collect::<Vec<_>>();
        if selected_ids.len() != budget {
            return Err(invalid("policy order is shorter than the requested proposal budget"));
        }
        let selected = selected_ids.iter()
            .map(|id| envelopes.get(id).cloned()
                .ok_or_else(|| invalid(format!("policy proposed unknown candidate {id}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let discovered_frontier = unique_envelope_frontier(&selected);
        let recall_numerator = discovered_frontier.intersection(oracle_frontier).count();
        let axis_best_gap = axis_best_gaps(&selected, oracle_envelopes)?;
        let best_essential = selected.iter()
            .map(|value| value.essential_unserved_quanta).min()
            .ok_or_else(|| invalid("selected proposal set is empty"))?;
        curve.push(BudgetPoint {
            unique_proposal_budget: budget,
            unique_proposals: selected.len(),
            duplicate_proposals: selected.len() - selected_ids.iter().collect::<BTreeSet<_>>().len(),
            selected_candidate_ids: selected_ids,
            discovered_envelope_frontier_vector_count: discovered_frontier.len(),
            full_domain_oracle_frontier_vector_count: oracle_frontier.len(),
            oracle_frontier_recall_numerator: recall_numerator,
            oracle_frontier_recall_denominator: oracle_frontier.len(),
            axis_best_gap,
            best_discovered_worst_case_essential_unserved_quanta: best_essential,
        });
    }
    Ok(curve)
}

fn quantile_summary(mut values: Vec<u64>) -> Result<QuantileSummaryU64, Box<dyn Error>> {
    if values.is_empty() {
        return Err(invalid("cannot summarize an empty random-seed result set"));
    }
    values.sort_unstable();
    let nearest_rank = |percent: usize| -> u64 {
        let rank = (percent * values.len() + 99) / 100;
        values[rank.saturating_sub(1).min(values.len() - 1)]
    };
    Ok(QuantileSummaryU64 {
        min: values[0],
        p10: nearest_rank(10),
        p50: nearest_rank(50),
        p90: nearest_rank(90),
        max: *values.last().unwrap(),
    })
}

fn random_seed_summaries(policy_runs: &[PolicyRun]) -> Result<Vec<RandomSeedBudgetSummary>, Box<dyn Error>> {
    let random_runs = policy_runs.iter()
        .filter(|run| run.policy_id.starts_with("seeded_random_without_replacement_seed_"))
        .collect::<Vec<_>>();
    if random_runs.len() != 30 {
        return Err(invalid(format!("expected 30 preregistered random seeds, observed {}", random_runs.len())));
    }
    let mut summaries = Vec::with_capacity(BUDGETS.len());
    for budget in BUDGETS {
        let points = random_runs.iter()
            .map(|run| run.budget_curve.iter().find(|point| point.unique_proposal_budget == budget)
                .ok_or_else(|| invalid(format!("random policy {} missing budget {budget}", run.policy_id))))
            .collect::<Result<Vec<_>, _>>()?;
        summaries.push(RandomSeedBudgetSummary {
            unique_proposal_budget: budget,
            random_seed_count: points.len(),
            oracle_frontier_recall_numerator: quantile_summary(points.iter()
                .map(|point| point.oracle_frontier_recall_numerator as u64).collect())?,
            worst_case_essential_unserved_quanta: quantile_summary(points.iter()
                .map(|point| point.best_discovered_worst_case_essential_unserved_quanta).collect())?,
            essential_unserved_axis_gap: quantile_summary(points.iter()
                .map(|point| point.axis_best_gap.essential_unserved_quanta_gap).collect())?,
            operating_cost_axis_gap: quantile_summary(points.iter()
                .map(|point| point.axis_best_gap.operating_cost_units_gap).collect())?,
        });
    }
    Ok(summaries)
}

fn shuffled_order(items: &[String], seed: u64) -> Vec<String> {
    let mut values = items.to_vec();
    let mut state = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
    for index in (1..values.len()).rev() {
        // Fixed xorshift64* stream; no RNG crate/version dependence.
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let choice = (state % (index as u64 + 1)) as usize;
        values.swap(index, choice);
    }
    values
}

fn validate_order(order: &[String], expected: &BTreeSet<String>) -> Result<(), Box<dyn Error>> {
    let observed = order.iter().cloned().collect::<BTreeSet<_>>();
    if order.len() != expected.len() || observed.len() != order.len() || &observed != expected {
        return Err(invalid("search policy must propose every expected candidate exactly once"));
    }
    Ok(())
}

fn run_report(root: &std::path::Path, output: &std::path::Path) -> Result<SearchPolicyReplayReport, Box<dyn Error>> {
    let manifest_bytes = fs::read(root.join("energy-design-space-manifest.json"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let ledger_bytes = fs::read(root.join("energy-benchmark-report.json"))?;
    let ledger: Value = serde_json::from_slice(&ledger_bytes)?;
    let matrix_bytes = fs::read(root.join("candidate-scenario-matrix-context/matrix-report.json"))?;
    let matrix: Value = serde_json::from_slice(&matrix_bytes)?;
    let robustness_bytes = fs::read(root.join("candidate-scenario-matrix-context/robustness-report.json"))?;
    let robustness: Value = serde_json::from_slice(&robustness_bytes)?;

    let manifest_digest = sha256_hex(&manifest_bytes);
    let ledger_digest = sha256_hex(&ledger_bytes);
    let matrix_digest = sha256_hex(&matrix_bytes);
    let robustness_digest = sha256_hex(&robustness_bytes);

    if required_str(&ledger, "schema_version")? != LEDGER_SCHEMA
        || required_str(&ledger, "benchmark_id")? != "energy-studio-candidate-robustness-envelope-benchmark-v0.6"
        || required_str(&matrix, "schema_version")? != MATRIX_SCHEMA
        || required_str(&robustness, "schema_version")? != ROBUSTNESS_SCHEMA
        || required_str(&robustness, "status")? != "pass"
        || required_str(&robustness, "matrix_id")? != "energy-candidate-scenario-matrix-v0.1"
        || required_str(&robustness, "source_matrix_sha256")? != matrix_digest.as_str()
        || required_str(&robustness, "source_design_space_manifest_sha256")? != manifest_digest.as_str()
        || required_str(&matrix, "design_space_manifest_sha256")? != manifest_digest
        || matrix["all_passed"].as_bool() != Some(true)
        || required_u64(&matrix, "expected_row_count")? != 195
        || required_u64(&matrix, "observed_row_count")? != 195
        || required_u64(&matrix, "passing_row_count")? != 195
        || required_u64(&matrix, "failed_or_unknown_row_count")? != 0
        || required_u64(&robustness, "expected_row_count")? != 195
        || required_u64(&robustness, "passed_row_count")? != 195
        || required_u64(&manifest, "requested_candidate_count")? != 36
        || manifest["candidates"].as_array().map(Vec::len) != Some(36)
        || manifest["rejected_candidates"].as_array().map(Vec::len) != Some(0)
        || required_u64(&robustness, "candidate_count")? != 39
        || required_u64(&robustness, "scenario_count")? != 5
        || required_u64(&robustness, "observed_row_count")? != 195
        || required_u64(&robustness, "failed_or_unknown_row_count")? != 0
        || required_str(&ledger["candidate_scenario_matrix"], "robustness_report_sha256")? != robustness_digest.as_str()
        || required_str(&ledger["candidate_scenario_matrix"], "matrix_report_sha256")? != matrix_digest.as_str()
        || required_str(&ledger["candidate_scenario_matrix"], "robustness_source_matrix_sha256")? != matrix_digest.as_str()
        || ledger["candidate_scenario_matrix"]["all_passed"].as_bool() != Some(true)
        || required_u64(&ledger["candidate_scenario_matrix"], "expected_row_count")? != 195
        || required_u64(&ledger["candidate_scenario_matrix"], "passing_row_count")? != 195
        || required_u64(&ledger["candidate_scenario_matrix"], "failed_or_unknown_row_count")? != 0
        || required_u64(&ledger["candidate_scenario_matrix"], "robustness_candidate_count")? != 39
        || required_u64(&ledger["candidate_scenario_matrix"], "robustness_scenario_count")? != 5
    {
        return Err(invalid("search replay inputs are not the digest-bound passing v0.6 ledger/matrix/robustness artifacts"));
    }

    let mut manifest_ids = manifest["candidates"].as_array()
        .ok_or_else(|| invalid("design-space manifest candidates missing"))?
        .iter()
        .map(|item| required_str(item, "candidate_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    if manifest_ids.iter().collect::<BTreeSet<_>>().len() != 36 {
        return Err(invalid("design-space manifest has duplicate candidate IDs"));
    }
    let expected_ids = expected_candidate_ids();
    let expected_set = expected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let manifest_parametric_set = manifest_ids.iter().cloned().collect::<BTreeSet<_>>();
    let expected_parametric_set = expected_ids.iter()
        .filter(|id| id.starts_with("synthetic-parametric-"))
        .cloned().collect::<BTreeSet<_>>();
    if manifest_parametric_set != expected_parametric_set {
        return Err(invalid("manifest candidate domain differs from the pinned 36-tuple oracle set"));
    }

    let candidate_rows = robustness["candidates"].as_array()
        .ok_or_else(|| invalid("robustness report candidate list missing"))?;
    if candidate_rows.len() != 39 {
        return Err(invalid("robustness report must expose exactly 39 candidate profiles"));
    }
    let mut envelopes = BTreeMap::<String, AxisBest>::new();
    let scenario_ids = robustness["deterministic_scenario_ids"].as_array()
        .ok_or_else(|| invalid("robustness scenario IDs missing"))?
        .iter().map(|item| item.as_str().map(str::to_owned)
            .ok_or_else(|| invalid("non-string scenario ID"))).collect::<Result<Vec<_>, _>>()?;
    let expected_scenarios = ["nominal", "pv_total_loss", "outage_early", "essential_demand_2x", "terminal_reserve_1"]
        .iter().map(|value| (*value).to_owned()).collect::<BTreeSet<_>>();
    if scenario_ids.iter().cloned().collect::<BTreeSet<_>>() != expected_scenarios
        || scenario_ids.len() != 5
    {
        return Err(invalid("robustness report scenario registry differs from the pinned five-scenario panel"));
    }

    for candidate in candidate_rows {
        let candidate_id = required_str(candidate, "candidate_id")?.to_owned();
        if !expected_set.contains(&candidate_id) {
            return Err(invalid(format!("unexpected candidate in robustness report: {candidate_id}")));
        }
        let scenarios: Vec<ScenarioProfile> = serde_json::from_value(candidate["scenario_profiles"].clone())?;
        if scenarios.len() != 5 {
            return Err(invalid(format!("candidate {candidate_id} lacks five scenarios")));
        }
        let mut ids = BTreeSet::new();
        for scenario in &scenarios {
            if !expected_scenarios.contains(&scenario.scenario_id)
                || !ids.insert(scenario.scenario_id.clone())
                || scenario.frontier_count == 0
                || scenario.frontier_count != scenario.frontier_objectives.len()
            {
                return Err(invalid(format!("candidate {candidate_id} has an invalid scenario profile")));
            }
        }
        let envelope: AxisBest = serde_json::from_value(
            candidate["worst_case_of_per_scenario_best_axis_value"].clone()
        )?;
        if envelopes.insert(candidate_id.clone(), envelope).is_some() {
            return Err(invalid(format!("duplicate candidate profile {candidate_id}")));
        }
    }
    if envelopes.keys().cloned().collect::<BTreeSet<_>>() != expected_set {
        return Err(invalid("robustness report candidate set is incomplete"));
    }

    let mut template_order = vec![
        "synthetic-grid-only".to_owned(),
        "synthetic-pv-grid".to_owned(),
        "synthetic-pv-battery-grid".to_owned(),
    ];
    template_order.append(&mut manifest_ids);
    validate_order(&template_order, &expected_set)?;

    let mut lexical_order = expected_ids.clone();
    lexical_order.sort();
    let mut battery_first = expected_ids.clone();
    battery_first.sort_by_key(|id| (
        if id.contains("battery") { 0_u8 } else { 1_u8 },
        if id.contains("pv") { 0_u8 } else { 1_u8 },
        id.clone(),
    ));

    let envelope_values = expected_ids.iter()
        .map(|id| envelopes.get(id).cloned()
            .ok_or_else(|| invalid(format!("missing envelope for {id}"))))
        .collect::<Result<Vec<_>, _>>()?;
    let oracle_frontier = unique_envelope_frontier(&envelope_values);
    if oracle_frontier.is_empty() {
        return Err(invalid("full-domain envelope oracle produced an empty frontier"));
    }

    let mut policy_runs = Vec::new();
    let mut arms: Vec<(String, Option<u64>, Vec<String>)> = vec![
        ("symthaea_export_manifest_order_plus_reference_templates".to_owned(), None, template_order),
        ("lexicographic_candidate_id".to_owned(), None, lexical_order),
        ("battery_then_pv_structural_heuristic".to_owned(), None, battery_first),
    ];
    for seed in 1_u64..=30 {
        arms.push((format!("seeded_random_without_replacement_seed_{seed}"), Some(seed), shuffled_order(&expected_ids, seed)));
    }

    for (policy_id, seed, order) in arms {
        validate_order(&order, &expected_set)?;
        let order_bytes = serde_json::to_vec(&order)?;
        let curve = curve_for_order(&order, &envelopes, &oracle_frontier, &envelope_values)?;
        policy_runs.push(PolicyRun {
            policy_id,
            seed,
            candidate_order_sha256: sha256_hex(&order_bytes),
            candidate_order: order,
            budget_curve: curve,
        });
    }

    let random_seed_budget_summary = random_seed_summaries(&policy_runs)?;
    let report = SearchPolicyReplayReport {
        schema_version: "energy-search-policy-replay-report-v0.1",
        status: "pass",
        benchmark_id: "energy-search-policy-replay-v0.1",
        source_ledger_sha256: ledger_digest,
        source_matrix_sha256: matrix_digest,
        source_robustness_report_sha256: robustness_digest,
        source_design_space_manifest_sha256: manifest_digest,
        candidate_count: 39,
        scenario_count: 5,
        proposal_budget_tiers: BUDGETS.to_vec(),
        full_domain_oracle_frontier_vector_count: oracle_frontier.len(),
        random_seed_budget_summary,
        policy_runs,
        measurement_scope: "Cached-outcome discovery-quality replay only. Unique-proposal count is the budget; all scenario dispatch outcomes are read from the already-qualified matrix. This report does not measure online search runtime, dispatch runtime, or policy computation time.",
        claim_ceiling: "Synthetic 39-candidate, five-scenario finite domain only. Envelope-frontier recall and axis-wise gaps operate on coordinate-wise worst-case-of-per-scenario-best envelopes, which are not necessarily jointly attainable dispatch schedules. No probability-weighted expectations, real-site recommendation, or industry-outperformance claim.",
    };
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(output, &output_bytes)?;
    println!("PASS: replayed {} policy orderings over six unique-proposal budgets and summarized {} random seeds",
        report.policy_runs.len(), 30);
    println!("source_ledger_sha256={}", report.source_ledger_sha256);
    println!("source_robustness_report_sha256={}", report.source_robustness_report_sha256);
    println!("report_sha256={}", sha256_hex(&output_bytes));
    println!("measurement_scope={}", report.measurement_scope);
    Ok(report)
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: energy_search_policy_replay <energy-studio-root> <output-report.json>");
        std::process::exit(2);
    }
    if let Err(error) = run_report(std::path::Path::new(&args[1]), std::path::Path::new(&args[2])) {
        eprintln!("energy search policy replay failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(essential: u64, flexible: u64, cost: u64, carbon: u64, curtailment: u64, soc: u16) -> AxisBest {
        AxisBest {
            essential_unserved_quanta: essential,
            flexible_unserved_quanta: flexible,
            operating_cost_units: cost,
            grid_carbon_grams: carbon,
            curtailed_pv_quanta: curtailment,
            terminal_soc_quanta: soc,
        }
    }

    #[test]
    fn envelope_frontier_deduplicates_vectors_and_removes_dominated_vectors() {
        let vectors = vec![
            env(0, 0, 5, 5, 2, 8),
            env(1, 1, 7, 8, 4, 5),
            env(0, 0, 5, 5, 2, 8),
            env(2, 0, 2, 3, 1, 9),
        ];
        let frontier = unique_envelope_frontier(&vectors);
        assert_eq!(frontier.len(), 2);
        assert!(frontier.contains(&env(0, 0, 5, 5, 2, 8)));
        assert!(frontier.contains(&env(2, 0, 2, 3, 1, 9)));
    }

    #[test]
    fn full_domain_budget_recovers_exact_oracle_frontier_and_zero_axis_gaps() {
        let ids = expected_candidate_ids();
        let oracle = vec![
            env(0, 1, 10, 10, 2, 4),
            env(1, 0, 4, 3, 0, 9),
            env(2, 2, 2, 2, 1, 3),
        ];
        let envelopes = ids.iter().enumerate().map(|(index, id)| {
            (id.clone(), oracle[index % oracle.len()].clone())
        }).collect::<BTreeMap<_, _>>();
        let values = ids.iter().map(|id| envelopes[id].clone()).collect::<Vec<_>>();
        let frontier = unique_envelope_frontier(&values);
        let curve = curve_for_order(&ids, &envelopes, &frontier, &values).unwrap();
        let last = curve.last().unwrap();
        assert_eq!(last.unique_proposal_budget, 39);
        assert_eq!(last.unique_proposals, 39);
        assert_eq!(last.oracle_frontier_recall_numerator, last.oracle_frontier_recall_denominator);
        assert_eq!(last.axis_best_gap.essential_unserved_quanta_gap, 0);
        assert_eq!(last.axis_best_gap.operating_cost_units_gap, 0);
        assert_eq!(last.axis_best_gap.terminal_soc_quanta_gap, 0);
    }

    #[test]
    fn seeded_order_is_repeatable_and_duplicate_free() {
        let ids = expected_candidate_ids();
        let first = shuffled_order(&ids, 42);
        let second = shuffled_order(&ids, 42);
        assert_eq!(first, second);
        assert_eq!(first.len(), 39);
        assert_eq!(first.iter().collect::<BTreeSet<_>>().len(), 39);
        assert_ne!(first, shuffled_order(&ids, 17));
    }

    #[test]
    fn quantiles_use_deterministic_nearest_rank() {
        let summary = quantile_summary((1_u64..=30).collect()).unwrap();
        assert_eq!(summary.min, 1);
        assert_eq!(summary.p10, 3);
        assert_eq!(summary.p50, 15);
        assert_eq!(summary.p90, 27);
        assert_eq!(summary.max, 30);
        assert!(quantile_summary(Vec::new()).is_err());
    }

    #[test]
    fn thirty_seed_summary_requires_every_budget_for_every_seed() {
        let mut runs = Vec::new();
        for seed in 1_u64..=30 {
            let mut curve = Vec::new();
            for budget in BUDGETS {
                curve.push(BudgetPoint {
                    unique_proposal_budget: budget,
                    unique_proposals: budget,
                    duplicate_proposals: 0,
                    selected_candidate_ids: Vec::new(),
                    discovered_envelope_frontier_vector_count: 1,
                    full_domain_oracle_frontier_vector_count: 2,
                    oracle_frontier_recall_numerator: (seed % 3) as usize,
                    oracle_frontier_recall_denominator: 2,
                    axis_best_gap: AxisGaps {
                        essential_unserved_quanta_gap: seed,
                        flexible_unserved_quanta_gap: 0,
                        operating_cost_units_gap: seed * 2,
                        grid_carbon_grams_gap: 0,
                        curtailed_pv_quanta_gap: 0,
                        terminal_soc_quanta_gap: 0,
                        semantics: "test-only",
                    },
                    best_discovered_worst_case_essential_unserved_quanta: seed + 10,
                });
            }
            runs.push(PolicyRun {
                policy_id: format!("seeded_random_without_replacement_seed_{seed}"),
                seed: Some(seed),
                candidate_order_sha256: "0".repeat(64),
                candidate_order: Vec::new(),
                budget_curve: curve,
            });
        }
        let summaries = random_seed_summaries(&runs).unwrap();
        assert_eq!(summaries.len(), 6);
        assert_eq!(summaries[0].random_seed_count, 30);
        assert_eq!(summaries[0].worst_case_essential_unserved_quanta.p50, 25);
        assert_eq!(summaries[0].operating_cost_gap.p90, 54);
        assert!(random_seed_summaries(&runs[..29]).is_err());
    }

    #[test]
    fn invalid_or_duplicate_proposal_order_fails_closed() {
        let ids = expected_candidate_ids();
        let expected = ids.iter().cloned().collect::<BTreeSet<_>>();
        assert!(validate_order(&ids[..38], &expected).is_err());
        let mut duplicate = ids.clone();
        duplicate[0] = duplicate[1].clone();
        assert!(validate_order(&duplicate, &expected).is_err());
    }
}
