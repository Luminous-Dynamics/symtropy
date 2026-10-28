//! Exhaustive leave-subset-out sensitivity oracle for a set-valued energy reserve policy.
//!
//! Research/qualification tool only. This is an abstract battery-only model,
//! not a physical dispatch solver or a production controller.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs};

const PANEL_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-robust-policy-stress-panel-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-uncertainty-set-omission-experiment-v0.1.json");
const PANEL_SCHEMA: &str = "energy-robust-policy-stress-panel-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-uncertainty-set-omission-experiment-v0.1";
const REPORT_SCHEMA: &str = "energy-uncertainty-set-omission-report-v0.1";
const POLICY: &str = "set_valued_essential_reserve";
const EXPECTED_TRAINING: [&str; 4] = [
    "early_outage_clears",
    "early_outage_persists",
    "late_outage",
    "no_late_outage",
];
const EXPECTED_HELD_OUT: [&str; 4] = [
    "heldout_critical_and_flexible_now",
    "heldout_long_outage_flex_surge",
    "heldout_critical_demand_surge",
    "heldout_late_double_critical",
];

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Panel {
    schema_version: String,
    benchmark_id: String,
    claim_ceiling: String,
    probability_model: Option<Value>,
    probability_note: String,
    model: Model,
    forecast_contract: Value,
    training_scenarios: Vec<Scenario>,
    held_out_stress_scenarios: Vec<Scenario>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    initial_soc_quanta: u32,
    capacity_quanta: u32,
    max_discharge_quanta_per_interval: u32,
    actions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    #[serde(default)]
    stress_family: Option<String>,
    intervals: Vec<Interval>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interval {
    time_index: usize,
    grid_available: bool,
    essential_demand: u32,
    flexible_demand: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    schema_version: String,
    benchmark_id: String,
    source_panel_path: String,
    source_panel_git_blob_sha1: String,
    source_panel_sha256: String,
    source_panel_hashing_rule: String,
    policy: String,
    subset_enumeration: String,
    held_out_evaluation: String,
    expected_coverage: ExpectedCoverage,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCoverage {
    training_subsets: usize,
    held_out_scenarios_per_subset: usize,
    subset_held_out_pairs: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct StepTrace {
    time_index: usize,
    action: String,
    soc_before: u32,
    soc_after: u32,
    reserve_quanta: u32,
    essential_unserved: u32,
    flexible_unserved: u32,
    discharge_quanta: u32,
    training_prefix_miss: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct HeldOutRow {
    scenario_id: String,
    actions: Vec<String>,
    steps: Vec<StepTrace>,
    essential_unserved: u32,
    flexible_unserved: u32,
    prefix_misses: usize,
    essential_unserved_delta_vs_full_set: i64,
    flexible_unserved_delta_vs_full_set: i64,
    first_action_changed_vs_full_set: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct SubsetResult {
    subset_id: String,
    training_scenario_count: usize,
    kept_training_scenario_ids: Vec<String>,
    omitted_training_scenario_ids: Vec<String>,
    held_out_total_essential_unserved: u32,
    held_out_worst_case_essential_unserved: u32,
    held_out_total_flexible_unserved: u32,
    held_out_worst_case_flexible_unserved: u32,
    held_out_prefix_misses: usize,
    held_out_first_action_changes_vs_full_set: usize,
    held_out_essential_degradation_scenario_count: usize,
    held_out_scenario_rows: Vec<HeldOutRow>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct CardinalitySummary {
    training_scenario_count: usize,
    subset_count: usize,
    min_worst_case_essential_unserved: u32,
    max_worst_case_essential_unserved: u32,
    min_total_essential_unserved: u32,
    max_total_essential_unserved: u32,
    min_total_flexible_unserved: u32,
    max_total_flexible_unserved: u32,
    note: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct SummaryCounts {
    subset_scenario_pairs_with_essential_degradation_vs_full_set: usize,
    subset_scenario_pairs_with_flexible_service_degradation_vs_full_set: usize,
    subset_scenario_pairs_with_first_action_change_vs_full_set: usize,
    subset_scenario_pairs_total: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct CriticalOmission {
    minimum_omitted_scenario_count: usize,
    omitted_training_scenario_ids: Vec<String>,
    subset_id: String,
    held_out_total_essential_unserved: u32,
    held_out_worst_case_essential_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Report {
    schema_version: &'static str,
    benchmark_id: String,
    status: &'static str,
    claim_ceiling: String,
    source_panel_sha256: String,
    experiment_fixture_sha256: String,
    policy: &'static str,
    probability_weighted_metrics: &'static str,
    physical_dispatch_audit: &'static str,
    training_scenario_count: usize,
    held_out_scenario_count: usize,
    training_subsets_enumerated: usize,
    subset_held_out_pairs: usize,
    full_training_set_reference: SubsetResult,
    empty_training_set_fail_closed_reference: SubsetResult,
    sensitivity_by_training_set_cardinality: Vec<CardinalitySummary>,
    minimum_critical_omissions: Vec<CriticalOmission>,
    all_subset_results: Vec<SubsetResult>,
    summary_counts: SummaryCounts,
    interpretation: String,
}

#[derive(Clone, Debug)]
struct Simulation {
    scenario_id: String,
    actions: Vec<String>,
    steps: Vec<StepTrace>,
    essential_unserved: u32,
    flexible_unserved: u32,
    prefix_misses: usize,
}

type Observable = (bool, u32, u32);

fn observable(interval: &Interval) -> Observable {
    (
        interval.grid_available,
        interval.essential_demand,
        interval.flexible_demand,
    )
}

fn validate_panel(panel: &Panel) -> Result<(), String> {
    if panel.schema_version != PANEL_SCHEMA {
        return Err(format!("unsupported panel schema: {}", panel.schema_version));
    }
    if !panel.claim_ceiling.to_ascii_lowercase().contains("synthetic") {
        return Err("synthetic claim ceiling is required".to_owned());
    }
    if panel.probability_model.is_some() {
        return Err("scenario probabilities are unsupported by the registered panel".to_owned());
    }
    if panel.probability_note.is_empty() || panel.forecast_contract.is_null() {
        return Err("probability or forecast contract is missing".to_owned());
    }
    if panel.model.actions != ["hold", "serve_essential", "serve_flexible"] {
        return Err("action catalog mismatch".to_owned());
    }
    if panel.model.capacity_quanta != 2
        || panel.model.initial_soc_quanta > panel.model.capacity_quanta
        || panel.model.max_discharge_quanta_per_interval != 1
    {
        return Err("model is outside the frozen one-quantum battery domain".to_owned());
    }
    validate_scenarios(&panel.training_scenarios, &EXPECTED_TRAINING, "training")?;
    validate_scenarios(&panel.held_out_stress_scenarios, &EXPECTED_HELD_OUT, "held-out")?;
    if panel.required_checks.len() < 8 {
        return Err("source panel required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn validate_scenarios(
    scenarios: &[Scenario],
    expected_ids: &[&str],
    group: &str,
) -> Result<(), String> {
    if scenarios.len() != expected_ids.len() {
        return Err(format!("{group} scenario count mismatch"));
    }
    let mut ids = BTreeSet::new();
    for (scenario, expected_id) in scenarios.iter().zip(expected_ids) {
        if scenario.scenario_id != *expected_id || !ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!("{group} scenario identity/order mismatch at {}", expected_id));
        }
        if scenario.intervals.len() != 3 {
            return Err(format!("{}: exactly three intervals required", scenario.scenario_id));
        }
        for (expected_time, interval) in scenario.intervals.iter().enumerate() {
            if interval.time_index != expected_time {
                return Err(format!(
                    "{}: missing, duplicate, or reordered interval at {expected_time}",
                    scenario.scenario_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_experiment(
    experiment: &Experiment,
    panel_bytes: &[u8],
) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA {
        return Err(format!(
            "unsupported experiment schema: {}",
            experiment.schema_version
        ));
    }
    if experiment.benchmark_id.is_empty()
        || experiment.claim_ceiling.to_ascii_lowercase().find("synthetic").is_none()
    {
        return Err("experiment identity or synthetic claim ceiling missing".to_owned());
    }
    if experiment.policy != POLICY {
        return Err("policy mismatch".to_owned());
    }
    if experiment.source_panel_path !=
        "tools/energy-dispatch-audit/fixtures/energy-robust-policy-stress-panel-v0.1.json"
    {
        return Err("source panel path differs from the pinned research contract".to_owned());
    }
    if experiment.source_panel_git_blob_sha1.len() != 40
        || !experiment.source_panel_git_blob_sha1.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err("source panel Git blob identity is malformed".to_owned());
    }
    if !experiment.source_panel_hashing_rule.to_ascii_lowercase().contains("raw bytes")
        || !experiment.subset_enumeration.to_ascii_lowercase().contains("all subsets")
        || !experiment.held_out_evaluation.to_ascii_lowercase().contains("all four")
    {
        return Err("experiment does not require exhaustive raw-byte-bound evaluation".to_owned());
    }
    let actual_digest = sha256_hex(panel_bytes);
    if experiment.source_panel_sha256 != actual_digest {
        return Err(format!(
            "source panel digest mismatch: expected {}, got {}",
            experiment.source_panel_sha256, actual_digest
        ));
    }
    let n = EXPECTED_TRAINING.len();
    let expected_subsets = 1usize << n;
    let pairs = expected_subsets * EXPECTED_HELD_OUT.len();
    if experiment.expected_coverage.training_subsets != expected_subsets
        || experiment.expected_coverage.held_out_scenarios_per_subset != EXPECTED_HELD_OUT.len()
        || experiment.expected_coverage.subset_held_out_pairs != pairs
    {
        return Err("expected Cartesian coverage must be 16 subsets by 4 held-out scenarios".to_owned());
    }
    if experiment.required_checks.len() < 9 {
        return Err("experiment required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn matching_scenarios<'a>(
    training: &'a [Scenario],
    history: &[Observable],
) -> Vec<&'a Scenario> {
    training
        .iter()
        .filter(|scenario| {
            scenario.intervals[..history.len()]
                .iter()
                .map(observable)
                .eq(history.iter().copied())
        })
        .collect()
}

fn future_essential_reserve(
    matches: &[&Scenario],
    time_index: usize,
    limit: u32,
    capacity: u32,
) -> u32 {
    if matches.is_empty() {
        return capacity;
    }
    let worst_required = matches
        .iter()
        .map(|scenario| {
            scenario.intervals[time_index + 1..]
                .iter()
                .filter(|interval| !interval.grid_available)
                .map(|interval| interval.essential_demand.min(limit))
                .sum::<u32>()
        })
        .max()
        .unwrap_or(0);
    capacity.min(worst_required)
}

fn decide(
    interval: &Interval,
    soc: u32,
    time_index: usize,
    history: &[Observable],
    training: &[Scenario],
    model: &Model,
) -> (String, bool, u32) {
    let matches = matching_scenarios(training, history);
    let prefix_miss = matches.is_empty();
    let reserve = future_essential_reserve(
        &matches,
        time_index,
        model.max_discharge_quanta_per_interval,
        model.capacity_quanta,
    );
    if interval.grid_available {
        return ("hold".to_owned(), prefix_miss, reserve);
    }
    if interval.essential_demand > 0 {
        return (
            if soc > 0 { "serve_essential" } else { "hold" }.to_owned(),
            prefix_miss,
            reserve,
        );
    }
    if interval.flexible_demand == 0 || soc == 0 {
        return ("hold".to_owned(), prefix_miss, reserve);
    }
    let can_serve_flexible = soc.saturating_sub(1) >= reserve;
    (
        if can_serve_flexible {
            "serve_flexible"
        } else {
            "hold"
        }
        .to_owned(),
        prefix_miss,
        reserve,
    )
}

fn simulate(
    scenario: &Scenario,
    training: &[Scenario],
    model: &Model,
) -> Result<Simulation, String> {
    let mut soc = model.initial_soc_quanta;
    let mut history = Vec::<Observable>::new();
    let mut actions = Vec::<String>::new();
    let mut steps = Vec::<StepTrace>::new();
    let mut essential_total = 0u32;
    let mut flexible_total = 0u32;
    let mut prefix_misses = 0usize;

    for (time_index, interval) in scenario.intervals.iter().enumerate() {
        history.push(observable(interval));
        let (action, prefix_miss, reserve) =
            decide(interval, soc, time_index, &history, training, model);
        prefix_misses += usize::from(prefix_miss);
        let soc_before = soc;
        let mut essential_served = 0u32;
        let mut flexible_served = 0u32;
        let mut discharge = 0u32;

        if interval.grid_available {
            if action != "hold" {
                return Err("controller attempted battery action while grid is available".to_owned());
            }
            essential_served = interval.essential_demand;
            flexible_served = interval.flexible_demand;
        } else {
            match action.as_str() {
                "hold" => {}
                "serve_essential" => {
                    if soc == 0 || interval.essential_demand == 0 {
                        return Err("controller emitted infeasible essential-service action".to_owned());
                    }
                    discharge = 1;
                    soc -= discharge;
                    essential_served = 1;
                }
                "serve_flexible" => {
                    if soc == 0 || interval.flexible_demand == 0 {
                        return Err("controller emitted infeasible flexible-service action".to_owned());
                    }
                    discharge = 1;
                    soc -= discharge;
                    flexible_served = 1;
                }
                _ => return Err(format!("unknown controller action: {action}")),
            }
        }
        if discharge > model.max_discharge_quanta_per_interval || soc > model.capacity_quanta {
            return Err("battery state/discharge invariant failed".to_owned());
        }
        let essential_unserved = interval.essential_demand - essential_served;
        let flexible_unserved = interval.flexible_demand - flexible_served;
        essential_total += essential_unserved;
        flexible_total += flexible_unserved;
        actions.push(action.clone());
        steps.push(StepTrace {
            time_index,
            action,
            soc_before,
            soc_after: soc,
            reserve_quanta: reserve,
            essential_unserved,
            flexible_unserved,
            discharge_quanta: discharge,
            training_prefix_miss: prefix_miss,
        });
    }

    Ok(Simulation {
        scenario_id: scenario.scenario_id.clone(),
        actions,
        steps,
        essential_unserved: essential_total,
        flexible_unserved: flexible_total,
        prefix_misses,
    })
}

fn subset_id(kept: &[String]) -> String {
    if kept.is_empty() {
        "subset-empty".to_owned()
    } else {
        format!("subset-{}", kept.join("-"))
    }
}

fn enumerate_subsets<'a>(items: &'a [Scenario]) -> Vec<Vec<&'a Scenario>> {
    fn combinations<'a>(
        items: &'a [Scenario],
        start: usize,
        remaining: usize,
        current: &mut Vec<&'a Scenario>,
        output: &mut Vec<Vec<&'a Scenario>>,
    ) {
        if remaining == 0 {
            output.push(current.clone());
            return;
        }
        for index in start..=items.len().saturating_sub(remaining) {
            current.push(&items[index]);
            combinations(items, index + 1, remaining - 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    for size in 0..=items.len() {
        combinations(items, 0, size, &mut Vec::new(), &mut output);
    }
    output
}

fn evaluate_subset(
    subset: &[&Scenario],
    panel: &Panel,
    full_rows: &[Simulation],
) -> Result<SubsetResult, String> {
    let kept = subset
        .iter()
        .map(|s| s.scenario_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let all_ids = panel
        .training_scenarios
        .iter()
        .map(|s| s.scenario_id.clone())
        .collect::<BTreeSet<_>>();
    let kept_set = kept.iter().cloned().collect::<BTreeSet<_>>();
    let omitted = all_ids.difference(&kept_set).cloned().collect::<Vec<_>>();
    let training = subset.iter().copied().cloned().collect::<Vec<_>>();
    let mut total_essential = 0u32;
    let mut total_flexible = 0u32;
    let mut worst_essential = 0u32;
    let mut worst_flexible = 0u32;
    let mut total_prefix_misses = 0usize;
    let mut first_action_changes = 0usize;
    let mut essential_degradation_count = 0usize;
    let mut held_out_rows = Vec::with_capacity(panel.held_out_stress_scenarios.len());

    for (scenario, full) in panel.held_out_stress_scenarios.iter().zip(full_rows) {
        let row = simulate(scenario, &training, &panel.model)?;
        let essential_delta = i64::from(row.essential_unserved) - i64::from(full.essential_unserved);
        let flexible_delta = i64::from(row.flexible_unserved) - i64::from(full.flexible_unserved);
        let first_action_changed = row.actions.first() != full.actions.first();
        first_action_changes += usize::from(first_action_changed);
        essential_degradation_count += usize::from(essential_delta > 0);
        total_prefix_misses += row.prefix_misses;
        total_essential += row.essential_unserved;
        total_flexible += row.flexible_unserved;
        worst_essential = worst_essential.max(row.essential_unserved);
        worst_flexible = worst_flexible.max(row.flexible_unserved);
        held_out_rows.push(HeldOutRow {
            scenario_id: row.scenario_id,
            actions: row.actions,
            steps: row.steps,
            essential_unserved: row.essential_unserved,
            flexible_unserved: row.flexible_unserved,
            prefix_misses: row.prefix_misses,
            essential_unserved_delta_vs_full_set: essential_delta,
            flexible_unserved_delta_vs_full_set: flexible_delta,
            first_action_changed_vs_full_set: first_action_changed,
        });
    }

    Ok(SubsetResult {
        subset_id: subset_id(&kept),
        training_scenario_count: kept.len(),
        kept_training_scenario_ids: kept,
        omitted_training_scenario_ids: omitted,
        held_out_total_essential_unserved: total_essential,
        held_out_worst_case_essential_unserved: worst_essential,
        held_out_total_flexible_unserved: total_flexible,
        held_out_worst_case_flexible_unserved: worst_flexible,
        held_out_prefix_misses: total_prefix_misses,
        held_out_first_action_changes_vs_full_set: first_action_changes,
        held_out_essential_degradation_scenario_count: essential_degradation_count,
        held_out_scenario_rows: held_out_rows,
    })
}

fn summarize_by_cardinality(rows: &[SubsetResult]) -> Vec<CardinalitySummary> {
    (0..=EXPECTED_TRAINING.len())
        .map(|size| {
            let group = rows
                .iter()
                .filter(|row| row.training_scenario_count == size)
                .collect::<Vec<_>>();
            CardinalitySummary {
                training_scenario_count: size,
                subset_count: group.len(),
                min_worst_case_essential_unserved: group
                    .iter().map(|x| x.held_out_worst_case_essential_unserved).min().unwrap_or(0),
                max_worst_case_essential_unserved: group
                    .iter().map(|x| x.held_out_worst_case_essential_unserved).max().unwrap_or(0),
                min_total_essential_unserved: group
                    .iter().map(|x| x.held_out_total_essential_unserved).min().unwrap_or(0),
                max_total_essential_unserved: group
                    .iter().map(|x| x.held_out_total_essential_unserved).max().unwrap_or(0),
                min_total_flexible_unserved: group
                    .iter().map(|x| x.held_out_total_flexible_unserved).min().unwrap_or(0),
                max_total_flexible_unserved: group
                    .iter().map(|x| x.held_out_total_flexible_unserved).max().unwrap_or(0),
                note: "finite extrema across enumerated subsets; not a probability distribution",
            }
        })
        .collect()
}

fn build_report(panel_bytes: &[u8], experiment_bytes: &[u8]) -> Result<Report, String> {
    let panel: Panel =
        serde_json::from_slice(panel_bytes).map_err(|e| format!("invalid source panel JSON: {e}"))?;
    let experiment: Experiment = serde_json::from_slice(experiment_bytes)
        .map_err(|e| format!("invalid experiment fixture JSON: {e}"))?;
    validate_panel(&panel)?;
    validate_experiment(&experiment, panel_bytes)?;

    if experiment.benchmark_id != "energy-uncertainty-set-omission-fragility-v0.1" {
        return Err("unexpected benchmark identity".to_owned());
    }
    let full_training = panel.training_scenarios.iter().collect::<Vec<_>>();
    let full_simulations = panel
        .held_out_stress_scenarios
        .iter()
        .map(|scenario| simulate(scenario, &full_training, &panel.model))
        .collect::<Result<Vec<_>, _>>()?;
    let all_sets = enumerate_subsets(&panel.training_scenarios);
    if all_sets.len() != experiment.expected_coverage.training_subsets {
        return Err(format!("subset enumeration mismatch: {}", all_sets.len()));
    }

    let mut rows = Vec::with_capacity(all_sets.len());
    for subset in all_sets {
        rows.push(evaluate_subset(&subset, &panel, &full_simulations)?);
    }
    let expected_pairs = rows.len() * panel.held_out_stress_scenarios.len();
    let actual_pairs = rows.iter().map(|row| row.held_out_scenario_rows.len()).sum::<usize>();
    if actual_pairs != expected_pairs
        || expected_pairs != experiment.expected_coverage.subset_held_out_pairs
    {
        return Err(format!("subset/scenario coverage mismatch: expected {expected_pairs}, got {actual_pairs}"));
    }
    let empty = rows
        .iter()
        .find(|row| row.training_scenario_count == 0)
        .cloned()
        .ok_or_else(|| "empty-set result missing".to_owned())?;
    if empty.held_out_scenario_rows.iter().any(|row| row.actions.first().map(String::as_str) == Some("serve_flexible")) {
        return Err("empty-set controller did not fail closed for first decision".to_owned());
    }
    let full = rows
        .iter()
        .find(|row| row.training_scenario_count == panel.training_scenarios.len())
        .cloned()
        .ok_or_else(|| "full-set result missing".to_owned())?;
    if full.held_out_scenario_rows.iter().any(|row| {
        row.essential_unserved_delta_vs_full_set != 0
            || row.flexible_unserved_delta_vs_full_set != 0
            || row.first_action_changed_vs_full_set
    }) {
        return Err("full-set self-comparison must have zero deltas and no action changes".to_owned());
    }

    let mut all_pairs = Vec::new();
    for row in &rows {
        for scenario in &row.held_out_scenario_rows {
            all_pairs.push(scenario);
        }
    }
    let summary_counts = SummaryCounts {
        subset_scenario_pairs_with_essential_degradation_vs_full_set: all_pairs
            .iter().filter(|row| row.essential_unserved_delta_vs_full_set > 0).count(),
        subset_scenario_pairs_with_flexible_service_degradation_vs_full_set: all_pairs
            .iter().filter(|row| row.flexible_unserved_delta_vs_full_set > 0).count(),
        subset_scenario_pairs_with_first_action_change_vs_full_set: all_pairs
            .iter().filter(|row| row.first_action_changed_vs_full_set).count(),
        subset_scenario_pairs_total: all_pairs.len(),
    };
    let minimum_omission_count = rows
        .iter()
        .filter(|row| row.held_out_essential_degradation_scenario_count > 0)
        .map(|row| row.omitted_training_scenario_ids.len())
        .min()
        .unwrap_or(0);
    let minimum_critical_omissions = rows
        .iter()
        .filter(|row| {
            row.held_out_essential_degradation_scenario_count > 0
                && row.omitted_training_scenario_ids.len() == minimum_omission_count
        })
        .map(|row| CriticalOmission {
            minimum_omitted_scenario_count: minimum_omission_count,
            omitted_training_scenario_ids: row.omitted_training_scenario_ids.clone(),
            subset_id: row.subset_id.clone(),
            held_out_total_essential_unserved: row.held_out_total_essential_unserved,
            held_out_worst_case_essential_unserved: row.held_out_worst_case_essential_unserved,
        })
        .collect::<Vec<_>>();
    let critical = minimum_critical_omissions
        .iter()
        .any(|row| row.omitted_training_scenario_ids == ["early_outage_persists"]);
    if !critical || summary_counts.subset_scenario_pairs_with_essential_degradation_vs_full_set != 7
        || summary_counts.subset_scenario_pairs_with_flexible_service_degradation_vs_full_set != 0
        || summary_counts.subset_scenario_pairs_with_first_action_change_vs_full_set != 14
    {
        return Err(format!("frozen sensitivity known answer mismatch: {summary_counts:?}"));
    }

    Ok(Report {
        schema_version: REPORT_SCHEMA,
        benchmark_id: experiment.benchmark_id.clone(),
        status: "PASS_SYNTHETIC_SCENARIO_SET_SENSITIVITY_ONLY",
        claim_ceiling: experiment.claim_ceiling.clone(),
        source_panel_sha256: sha256_hex(panel_bytes),
        experiment_fixture_sha256: sha256_hex(experiment_bytes),
        policy: POLICY,
        probability_weighted_metrics: "NOT_COMPUTED_NO_SUPPORTED_PROBABILITIES",
        physical_dispatch_audit: "NOT_RUN; abstract battery-only model",
        training_scenario_count: panel.training_scenarios.len(),
        held_out_scenario_count: panel.held_out_stress_scenarios.len(),
        training_subsets_enumerated: rows.len(),
        subset_held_out_pairs: expected_pairs,
        full_training_set_reference: full,
        empty_training_set_fail_closed_reference: empty,
        sensitivity_by_training_set_cardinality: summarize_by_cardinality(&rows),
        minimum_critical_omissions,
        all_subset_results: rows,
        summary_counts,
        interpretation: "Exhaustive subset-ablation diagnostic for one frozen synthetic panel. Cardinality strata and pair counts describe this finite enumeration only; they are not independent trials, probabilities, confidence intervals, or a real-world guarantee.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: energy_uncertainty_set_omission_oracle [report.json]".to_owned());
    }
    let report = build_report(PANEL_BYTES, EXPERIMENT_BYTES)?;
    if let Some(path) = args.get(1) {
        let output = serde_json::to_vec_pretty(&report)
            .map_err(|e| format!("cannot serialize report: {e}"))?;
        fs::write(path, output).map_err(|e| format!("cannot write report: {e}"))?;
    }
    println!("PASS synthetic uncertainty-set omission sensitivity");
    println!(
        "subsets={} pairs={}",
        report.training_subsets_enumerated, report.subset_held_out_pairs
    );
    println!(
        "summary_counts={}",
        serde_json::to_string(&report.summary_counts).map_err(|e| e.to_string())?
    );
    println!("source_panel_sha256={}", report.source_panel_sha256);
    println!("experiment_fixture_sha256={}", report.experiment_fixture_sha256);
    println!("physical_dispatch_audit=NOT_RUN");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy uncertainty-set omission oracle failed closed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden() -> Report {
        build_report(PANEL_BYTES, EXPERIMENT_BYTES).expect("frozen fixture must pass")
    }

    #[test]
    fn frozen_known_answer_has_exact_subset_and_pair_coverage() {
        let report = golden();
        assert_eq!(report.training_subsets_enumerated, 16);
        assert_eq!(report.subset_held_out_pairs, 64);
        assert_eq!(report.sensitivity_by_training_set_cardinality.iter().map(|x| x.subset_count).sum::<usize>(), 16);
        assert_eq!(report.summary_counts.subset_scenario_pairs_with_essential_degradation_vs_full_set, 7);
        assert_eq!(report.summary_counts.subset_scenario_pairs_with_flexible_service_degradation_vs_full_set, 0);
        assert_eq!(report.summary_counts.subset_scenario_pairs_with_first_action_change_vs_full_set, 14);
        assert_eq!(report.summary_counts.subset_scenario_pairs_total, 64);
    }

    #[test]
    fn empty_set_fails_closed_and_full_set_has_zero_self_delta() {
        let report = golden();
        assert!(report.empty_training_set_fail_closed_reference.held_out_scenario_rows.iter()
            .all(|row| row.actions.first().map(String::as_str) != Some("serve_flexible")));
        for row in &report.full_training_set_reference.held_out_scenario_rows {
            assert_eq!(row.essential_unserved_delta_vs_full_set, 0);
            assert_eq!(row.flexible_unserved_delta_vs_full_set, 0);
            assert!(!row.first_action_changed_vs_full_set);
        }
    }

    #[test]
    fn minimum_counterexample_identifies_persistent_outage_omission() {
        let report = golden();
        assert_eq!(report.minimum_critical_omissions.len(), 1);
        let case = &report.minimum_critical_omissions[0];
        assert_eq!(case.minimum_omitted_scenario_count, 1);
        assert_eq!(case.omitted_training_scenario_ids, vec!["early_outage_persists"]);
        assert_eq!(case.held_out_total_essential_unserved, 4);
        assert_eq!(case.held_out_worst_case_essential_unserved, 2);
    }

    #[test]
    fn complete_step_traces_are_preserved_for_each_pair() {
        let report = golden();
        assert!(report.all_subset_results.iter().all(|subset| subset.held_out_scenario_rows.len() == 4
            && subset.held_out_scenario_rows.iter().all(|row| row.steps.len() == 3)));
    }

    #[test]
    fn source_digest_tampering_fails_closed() {
        let mut modified = serde_json::from_slice::<Value>(PANEL_BYTES).unwrap();
        modified["claim_ceiling"] = Value::String("Synthetic altered claim ceiling".to_owned());
        let mutated = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&mutated, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("source panel digest mismatch"));
    }

    #[test]
    fn duplicate_scenario_identity_fails_closed() {
        let mut modified = serde_json::from_slice::<Value>(PANEL_BYTES).unwrap();
        modified["training_scenarios"][1]["scenario_id"] = Value::String("early_outage_clears".to_owned());
        let mutated = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&mutated, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("identity/order mismatch"));
    }

    #[test]
    fn reordered_or_missing_interval_fails_closed() {
        let mut modified = serde_json::from_slice::<Value>(PANEL_BYTES).unwrap();
        modified["held_out_stress_scenarios"][0]["intervals"].as_array_mut().unwrap().swap(0, 1);
        let mutated = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&mutated, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("missing, duplicate, or reordered interval"));
    }

    #[test]
    fn unsupported_probabilities_fail_closed() {
        let mut modified = serde_json::from_slice::<Value>(PANEL_BYTES).unwrap();
        modified["probability_model"] = serde_json::json!({"uniform": true});
        let mutated = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&mutated, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("scenario probabilities are unsupported"));
    }

    #[test]
    fn deterministic_report_replay() {
        let first = golden();
        let second = golden();
        assert_eq!(first, second);
        let a = serde_json::to_vec(&first).unwrap();
        let b = serde_json::to_vec(&second).unwrap();
        assert_eq!(a, b);
    }
}
