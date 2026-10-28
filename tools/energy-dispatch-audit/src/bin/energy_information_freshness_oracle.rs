//! Forecast freshness and information-latency oracle.
//!
//! Synthetic research qualification only. This binary does not model
//! electrical network physics and never claims physical dispatch feasibility.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

const PANEL_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-information-freshness-panel-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-information-freshness-experiment-v0.1.json");
const PANEL_SCHEMA: &str = "energy-information-freshness-panel-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-information-freshness-experiment-v0.1";
const REPORT_SCHEMA: &str = "energy-information-freshness-report-v0.1";
const FRESH: &str = "fresh_forecast";
const DELAYED: &str = "delayed_forecast_fail_closed";
const STALE_SAFE: &str = "stale_forecast_fail_closed";
const STALE_BLIND: &str = "stale_blind_counterfactual";
const EXPECTED_SCENARIOS: [&str; 4] = [
    "no_future_critical",
    "single_late_critical",
    "persistent_critical_load",
    "single_early_critical",
];
const EXPECTED_CONTEXTS: [&str; 3] = ["north", "east", "south"];

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
    model: Model,
    observable_contexts: Vec<String>,
    forecast_target: String,
    forecast_sources: BTreeMap<String, ForecastSource>,
    scenarios: Vec<Scenario>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    initial_soc_quanta: u32,
    capacity_quanta: u32,
    max_discharge_quanta_per_interval: u32,
    decision_time_index: usize,
    unknown_forecast_fallback: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastSource {
    freshness_status: String,
    issued_at_time_index: i32,
    valid_until_time_index: i32,
    available_at_time_index: i32,
    reserve_by_observable_context: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    observable_context: String,
    true_future_essential_reserve_quanta: u32,
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
    source_panel_sha256: String,
    expected_coverage: ExpectedCoverage,
    known_answers: KnownAnswers,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCoverage {
    scenarios: usize,
    intervals_per_scenario: usize,
    qualified_policies: usize,
    counterfactual_policies: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    fresh_forecast_total_essential_unserved: u32,
    fresh_forecast_total_flexible_unserved: u32,
    delayed_forecast_total_essential_unserved: u32,
    delayed_forecast_total_flexible_unserved: u32,
    stale_fail_closed_total_essential_unserved: u32,
    stale_fail_closed_total_flexible_unserved: u32,
    stale_blind_counterfactual_total_essential_unserved: u32,
    stale_blind_counterfactual_total_flexible_unserved: u32,
    minimum_stale_case_essential_gap: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct StepTrace {
    time_index: usize,
    action: String,
    soc_before: u32,
    soc_after: u32,
    discharge_quanta: u32,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct ScenarioRow {
    scenario_id: String,
    observable_context: String,
    forecast_status_at_decision: String,
    reserve_value_used: u32,
    actions: Vec<String>,
    steps: Vec<StepTrace>,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyResult {
    policy_id: String,
    qualification_status: String,
    essential_unserved_total: u32,
    flexible_unserved_total: u32,
    worst_case_essential_unserved: u32,
    worst_case_flexible_unserved: u32,
    rows: Vec<ScenarioRow>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Report {
    schema_version: &'static str,
    benchmark_id: String,
    status: &'static str,
    claim_ceiling: String,
    source_panel_sha256: String,
    experiment_fixture_sha256: String,
    probability_weighted_metrics: &'static str,
    physical_dispatch_audit: &'static str,
    scenario_count: usize,
    interval_count: usize,
    qualified_policy_count: usize,
    counterfactual_policy_count: usize,
    policy_results: BTreeMap<String, PolicyResult>,
    interpretation: String,
}

fn validate_panel(panel: &Panel) -> Result<(), String> {
    if panel.schema_version != PANEL_SCHEMA {
        return Err(format!("unsupported panel schema: {}", panel.schema_version));
    }
    if panel.benchmark_id != "forecast-freshness-and-information-latency-v0.1" {
        return Err("panel benchmark identity mismatch".to_owned());
    }
    if !panel.claim_ceiling.to_ascii_lowercase().contains("synthetic") {
        return Err("synthetic claim ceiling is required".to_owned());
    }
    if panel.probability_model.is_some() {
        return Err("probability-weighted scoring is unsupported by this panel".to_owned());
    }
    if panel.model.initial_soc_quanta != 2
        || panel.model.capacity_quanta != 2
        || panel.model.max_discharge_quanta_per_interval != 1
        || panel.model.decision_time_index != 0
    {
        return Err("model differs from frozen two-quantum domain".to_owned());
    }
    if panel.model.unknown_forecast_fallback
        != "reserve_full_capacity_and_do_not_serve_discretionary_flexible_load"
    {
        return Err("unknown-forecast fallback rule changed".to_owned());
    }
    if panel.forecast_target.is_empty() {
        return Err("forecast target missing".to_owned());
    }
    let expected_contexts = EXPECTED_CONTEXTS
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if panel.observable_contexts != expected_contexts {
        return Err("observable context registry changed".to_owned());
    }
    for source_id in ["fresh_context_forecast", "delayed_context_forecast", "stale_context_forecast"] {
        let source = panel.forecast_sources.get(source_id)
            .ok_or_else(|| format!("forecast source missing: {source_id}"))?;
        let keys = source.reserve_by_observable_context.keys().cloned().collect::<BTreeSet<_>>();
        let contexts = panel.observable_contexts.iter().cloned().collect::<BTreeSet<_>>();
        if keys != contexts {
            return Err(format!("{source_id}: forecast context coverage mismatch"));
        }
        if source.reserve_by_observable_context.values().any(|reserve| *reserve > panel.model.capacity_quanta) {
            return Err(format!("{source_id}: reserve value exceeds battery capacity"));
        }
    }
    let expected_source_timing = [
        ("fresh_context_forecast", "fresh", 0, 0, 0),
        ("delayed_context_forecast", "fresh_when_arrived", 1, 1, 1),
        ("stale_context_forecast", "stale", -3, -1, -3),
    ];
    for (source_id, expected_status, issued_at, valid_until, available_at) in expected_source_timing {
        let source = panel.forecast_sources.get(source_id)
            .ok_or_else(|| format!("forecast source missing: {source_id}"))?;
        if source.freshness_status != expected_status
            || source.issued_at_time_index != issued_at
            || source.valid_until_time_index != valid_until
            || source.available_at_time_index != available_at
        {
            return Err(format!("forecast source temporal contract changed: {source_id}"));
        }
    }
    if panel.forecast_sources.len() != 3 {
        return Err("unexpected forecast source added or removed".to_owned());
    }
    if panel.scenarios.len() != EXPECTED_SCENARIOS.len() {
        return Err("expected exactly four scenarios".to_owned());
    }
    let mut seen = BTreeSet::new();
    for (scenario, expected_id) in panel.scenarios.iter().zip(EXPECTED_SCENARIOS) {
        if scenario.scenario_id != expected_id || !seen.insert(scenario.scenario_id.as_str()) {
            return Err(format!("missing, duplicate, unknown or reordered scenario; expected {expected_id}"));
        }
        if !panel.observable_contexts.contains(&scenario.observable_context) {
            return Err(format!("{}: unknown observable context", scenario.scenario_id));
        }
        if scenario.intervals.len() != 3 {
            return Err(format!("{}: exactly three intervals required", scenario.scenario_id));
        }
        for (expected_time, interval) in scenario.intervals.iter().enumerate() {
            if interval.time_index != expected_time {
                return Err(format!("{}: missing, duplicate or reordered interval", scenario.scenario_id));
            }
        }
        let first = &scenario.intervals[0];
        if first.grid_available || first.essential_demand != 0 || first.flexible_demand != 1 {
            return Err(format!("{}: frozen t0 state changed", scenario.scenario_id));
        }
        let derived_reserve = scenario.intervals.iter().skip(1)
            .filter(|step| !step.grid_available)
            .map(|step| step.essential_demand.min(panel.model.max_discharge_quanta_per_interval))
            .sum::<u32>();
        if derived_reserve != scenario.true_future_essential_reserve_quanta {
            return Err(format!("{}: reserve truth label contradicts interval data", scenario.scenario_id));
        }
    }
    if panel.required_checks.len() < 8 {
        return Err("panel required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn validate_experiment(experiment: &Experiment, panel: &Panel, panel_bytes: &[u8]) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA {
        return Err(format!("unsupported experiment schema: {}", experiment.schema_version));
    }
    if experiment.benchmark_id != panel.benchmark_id {
        return Err("panel/experiment benchmark identity mismatch".to_owned());
    }
    if experiment.source_panel_path
        != "tools/energy-dispatch-audit/fixtures/energy-information-freshness-panel-v0.1.json"
    {
        return Err("source panel path mismatch".to_owned());
    }
    let actual = sha256_hex(panel_bytes);
    if actual != experiment.source_panel_sha256 {
        return Err(format!("raw source panel SHA-256 mismatch: expected {}, got {actual}", experiment.source_panel_sha256));
    }
    if experiment.expected_coverage.scenarios != panel.scenarios.len()
        || experiment.expected_coverage.intervals_per_scenario != 3
        || experiment.expected_coverage.qualified_policies != 3
        || experiment.expected_coverage.counterfactual_policies != 1
    {
        return Err("experiment coverage contract changed".to_owned());
    }
    if !experiment.claim_ceiling.to_ascii_lowercase().contains("synthetic")
        || experiment.required_checks.len() < 7
    {
        return Err("experiment claim ceiling or required-check registry incomplete".to_owned());
    }
    Ok(())
}

fn available_forecast(
    panel: &Panel,
    scenario: &Scenario,
    source_id: &str,
    now: i32,
) -> (String, u32) {
    let source = match panel.forecast_sources.get(source_id) {
        Some(source) => source,
        None => return ("missing_source".to_owned(), panel.model.capacity_quanta),
    };
    if source.freshness_status == "stale" {
        return ("ignored_stale".to_owned(), panel.model.capacity_quanta);
    }
    if source.available_at_time_index > now {
        return ("not_yet_available".to_owned(), panel.model.capacity_quanta);
    }
    if source.issued_at_time_index > now {
        return ("issued_in_future".to_owned(), panel.model.capacity_quanta);
    }
    if source.valid_until_time_index < now {
        return ("expired".to_owned(), panel.model.capacity_quanta);
    }
    match source.reserve_by_observable_context.get(&scenario.observable_context) {
        Some(reserve) => ("usable_fresh".to_owned(), *reserve),
        None => ("missing_context".to_owned(), panel.model.capacity_quanta),
    }
}

fn simulate(panel: &Panel, scenario: &Scenario, policy_id: &str) -> Result<ScenarioRow, String> {
    let now = panel.model.decision_time_index as i32;
    let (freshness, reserve, qualified) = match policy_id {
        FRESH => {
            let (freshness, reserve) = available_forecast(panel, scenario, "fresh_context_forecast", now);
            (freshness, reserve, true)
        }
        DELAYED => {
            let (freshness, reserve) = available_forecast(panel, scenario, "delayed_context_forecast", now);
            (freshness, reserve, true)
        }
        STALE_SAFE => {
            let (freshness, reserve) = available_forecast(panel, scenario, "stale_context_forecast", now);
            (freshness, reserve, true)
        }
        STALE_BLIND => {
            let source = panel.forecast_sources.get("stale_context_forecast")
                .ok_or_else(|| "stale source missing".to_owned())?;
            let reserve = *source.reserve_by_observable_context.get(&scenario.observable_context)
                .ok_or_else(|| "stale counterfactual has no value for observable context".to_owned())?;
            ("STALE_USED_UNSAFELY".to_owned(), reserve, false)
        }
        _ => return Err(format!("unknown policy id: {policy_id}")),
    };

    if qualified && freshness == "STALE_USED_UNSAFELY" {
        return Err("stale forecast cannot be qualified".to_owned());
    }

    let mut soc = panel.model.initial_soc_quanta;
    let mut actions = Vec::with_capacity(scenario.intervals.len());
    let mut steps = Vec::with_capacity(scenario.intervals.len());
    let mut essential_total = 0u32;
    let mut flexible_total = 0u32;
    for interval in &scenario.intervals {
        let time = interval.time_index;
        let before = soc;
        let mut discharge = 0u32;
        let mut essential_served = 0u32;
        let mut flexible_served = 0u32;

        let action = if interval.grid_available {
            essential_served = interval.essential_demand;
            flexible_served = interval.flexible_demand;
            "hold"
        } else if interval.essential_demand > 0 && soc > 0 {
            discharge = 1;
            soc -= discharge;
            essential_served = 1;
            "serve_essential"
        } else if time == panel.model.decision_time_index && interval.flexible_demand > 0 {
            if soc > 0 && soc.saturating_sub(1) >= reserve {
                discharge = 1;
                soc -= discharge;
                flexible_served = 1;
                "serve_flexible"
            } else {
                "hold"
            }
        } else if interval.flexible_demand > 0 && soc > 0 {
            discharge = 1;
            soc -= discharge;
            flexible_served = 1;
            "serve_flexible"
        } else {
            "hold"
        };
        if discharge > panel.model.max_discharge_quanta_per_interval || soc > panel.model.capacity_quanta {
            return Err("battery capacity/discharge invariant failed".to_owned());
        }
        let essential_unserved = interval.essential_demand - essential_served;
        let flexible_unserved = interval.flexible_demand - flexible_served;
        essential_total += essential_unserved;
        flexible_total += flexible_unserved;
        actions.push(action.to_owned());
        steps.push(StepTrace {
            time_index: time,
            action: action.to_owned(),
            soc_before: before,
            soc_after: soc,
            discharge_quanta: discharge,
            essential_unserved,
            flexible_unserved,
        });
    }

    Ok(ScenarioRow {
        scenario_id: scenario.scenario_id.clone(),
        observable_context: scenario.observable_context.clone(),
        forecast_status_at_decision: freshness,
        reserve_value_used: reserve,
        actions,
        steps,
        essential_unserved: essential_total,
        flexible_unserved: flexible_total,
    })
}

fn evaluate_policy(panel: &Panel, policy_id: &str) -> Result<PolicyResult, String> {
    let rows = panel.scenarios.iter()
        .map(|scenario| simulate(panel, scenario, policy_id))
        .collect::<Result<Vec<_>, _>>()?;
    let is_qualified = policy_id != STALE_BLIND;
    Ok(PolicyResult {
        policy_id: policy_id.to_owned(),
        qualification_status: if is_qualified {
            "QUALIFIED_SYNTHETIC_POLICY".to_owned()
        } else {
            "DISQUALIFIED_UNSAFE_COUNTERFACTUAL".to_owned()
        },
        essential_unserved_total: rows.iter().map(|row| row.essential_unserved).sum(),
        flexible_unserved_total: rows.iter().map(|row| row.flexible_unserved).sum(),
        worst_case_essential_unserved: rows.iter().map(|row| row.essential_unserved).max().unwrap_or(0),
        worst_case_flexible_unserved: rows.iter().map(|row| row.flexible_unserved).max().unwrap_or(0),
        rows,
    })
}

fn build_report(panel_bytes: &[u8], experiment_bytes: &[u8]) -> Result<Report, String> {
    let panel: Panel = serde_json::from_slice(panel_bytes)
        .map_err(|error| format!("invalid source panel JSON: {error}"))?;
    let experiment: Experiment = serde_json::from_slice(experiment_bytes)
        .map_err(|error| format!("invalid experiment JSON: {error}"))?;
    validate_panel(&panel)?;
    validate_experiment(&experiment, &panel, panel_bytes)?;

    let mut results = BTreeMap::new();
    for policy_id in [FRESH, DELAYED, STALE_SAFE, STALE_BLIND] {
        results.insert(policy_id.to_owned(), evaluate_policy(&panel, policy_id)?);
    }
    let known = &experiment.known_answers;
    let fresh = results.get(FRESH).unwrap();
    let delayed = results.get(DELAYED).unwrap();
    let stale_safe = results.get(STALE_SAFE).unwrap();
    let stale_blind = results.get(STALE_BLIND).unwrap();
    let fresh_essential = fresh.essential_unserved_total;
    let fresh_flexible = fresh.flexible_unserved_total;
    let delayed_essential = delayed.essential_unserved_total;
    let delayed_flexible = delayed.flexible_unserved_total;
    let stale_safe_essential = stale_safe.essential_unserved_total;
    let stale_safe_flexible = stale_safe.flexible_unserved_total;
    let stale_blind_essential = stale_blind.essential_unserved_total;
    let stale_blind_flexible = stale_blind.flexible_unserved_total;

    if fresh_essential != known.fresh_forecast_total_essential_unserved
        || fresh_flexible != known.fresh_forecast_total_flexible_unserved
        || delayed_essential != known.delayed_forecast_total_essential_unserved
        || delayed_flexible != known.delayed_forecast_total_flexible_unserved
        || stale_safe_essential != known.stale_fail_closed_total_essential_unserved
        || stale_safe_flexible != known.stale_fail_closed_total_flexible_unserved
        || stale_blind_essential != known.stale_blind_counterfactual_total_essential_unserved
        || stale_blind_flexible != known.stale_blind_counterfactual_total_flexible_unserved
        || stale_blind_essential < known.minimum_stale_case_essential_gap
        || stale_blind.qualification_status != "DISQUALIFIED_UNSAFE_COUNTERFACTUAL"
        || stale_safe.qualification_status != "QUALIFIED_SYNTHETIC_POLICY"
    {
        return Err("forecast freshness frozen known answer changed".to_owned());
    }

    Ok(Report {
        schema_version: REPORT_SCHEMA,
        benchmark_id: experiment.benchmark_id.clone(),
        status: "PASS_SYNTHETIC_INFORMATION_FRESHNESS_ONLY",
        claim_ceiling: experiment.claim_ceiling.clone(),
        source_panel_sha256: sha256_hex(panel_bytes),
        experiment_fixture_sha256: sha256_hex(experiment_bytes),
        probability_weighted_metrics: "NOT_COMPUTED_NO_SUPPORTED_PROBABILITIES",
        physical_dispatch_audit: "NOT_RUN_ABSTRACT_BATTERY_MODEL",
        scenario_count: panel.scenarios.len(),
        interval_count: panel.scenarios.iter().map(|scenario| scenario.intervals.len()).sum(),
        qualified_policy_count: 3,
        counterfactual_policy_count: 1,
        policy_results: results,
        interpretation: "Availability, issuance time and validity window are part of the decision information state. The stale counterfactual is disqualified even though it serves more flexible load, because it worsens essential-service outcomes. Synthetic finite-panel result only.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: energy_information_freshness_oracle [report.json]".to_owned());
    }
    let report = build_report(PANEL_BYTES, EXPERIMENT_BYTES)?;
    if let Some(path) = args.get(1) {
        let output = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
        fs::write(path, output).map_err(|error| format!("cannot write report: {error}"))?;
    }
    println!("PASS synthetic information freshness benchmark");
    println!("source_panel_sha256={}", report.source_panel_sha256);
    println!("experiment_fixture_sha256={}", report.experiment_fixture_sha256);
    for result in report.policy_results.values() {
        println!(
            "policy={} qualification={} essential_unserved={} flexible_unserved={}",
            result.policy_id,
            result.qualification_status,
            result.essential_unserved_total,
            result.flexible_unserved_total
        );
    }
    println!("physical_dispatch_audit=NOT_RUN");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy information freshness oracle failed closed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden() -> Report {
        build_report(PANEL_BYTES, EXPERIMENT_BYTES).expect("frozen information-freshness fixture must pass")
    }

    #[test]
    fn frozen_known_answers_and_exact_coverage() {
        let report = golden();
        assert_eq!((report.scenario_count, report.interval_count), (4, 12));
        let fresh = &report.policy_results[FRESH];
        assert_eq!((fresh.essential_unserved_total, fresh.flexible_unserved_total), (0, 1));
        let delayed = &report.policy_results[DELAYED];
        assert_eq!((delayed.essential_unserved_total, delayed.flexible_unserved_total), (0, 4));
        let stale = &report.policy_results[STALE_SAFE];
        assert_eq!((stale.essential_unserved_total, stale.flexible_unserved_total), (0, 4));
        let blind = &report.policy_results[STALE_BLIND];
        assert_eq!((blind.essential_unserved_total, blind.flexible_unserved_total), (1, 0));
    }

    #[test]
    fn delayed_and_stale_sources_fail_closed_at_t0() {
        let report = golden();
        for policy_id in [DELAYED, STALE_SAFE] {
            let result = &report.policy_results[policy_id];
            assert!(result.rows.iter().all(|row| row.actions[0] == "hold"));
            assert!(result.rows.iter().all(|row| row.forecast_status_at_decision != "usable_fresh"));
        }
    }

    #[test]
    fn stale_blind_counterfactual_is_never_qualified() {
        let blind = &golden().policy_results[STALE_BLIND];
        assert_eq!(blind.qualification_status, "DISQUALIFIED_UNSAFE_COUNTERFACTUAL");
        assert_eq!(blind.essential_unserved_total, 1);
    }

    #[test]
    fn scenario_ids_do_not_change_forecast_selection() {
        let panel: Panel = serde_json::from_slice(PANEL_BYTES).unwrap();
        let source = &panel.scenarios[0];
        let original = simulate(&panel, source, FRESH).unwrap();
        let mut renamed = source.clone();
        renamed.scenario_id = "hidden-renamed-evaluator-id".to_owned();
        let changed = simulate(&panel, &renamed, FRESH).unwrap();
        assert_eq!(original.actions, changed.actions);
        assert_eq!(original.reserve_value_used, changed.reserve_value_used);
    }

    #[test]
    fn raw_source_digest_mismatch_fails_closed() {
        let mut panel: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        panel["claim_ceiling"] = Value::String("Synthetic edited fixture".to_owned());
        let changed = serde_json::to_vec(&panel).unwrap();
        let error = build_report(&changed, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn unsupported_probability_model_fails_closed() {
        let mut panel: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        panel["probability_model"] = serde_json::json!({"uniform": true});
        let changed = serde_json::to_vec(&panel).unwrap();
        let error = build_report(&changed, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("probability-weighted scoring is unsupported"));
    }

    #[test]
    fn reordered_scenario_fails_closed_after_digest_is_rebound() {
        let mut panel: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        panel["scenarios"].as_array_mut().unwrap().reverse();
        let changed_panel = serde_json::to_vec(&panel).unwrap();
        let mut experiment: Value = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        experiment["source_panel_sha256"] = Value::String(sha256_hex(&changed_panel));
        let changed_experiment = serde_json::to_vec(&experiment).unwrap();
        let error = build_report(&changed_panel, &changed_experiment).unwrap_err();
        assert!(error.contains("reordered scenario"));
    }

    #[test]
    fn stale_freshness_metadata_change_is_detected_by_known_answer() {
        let mut panel: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        panel["forecast_sources"]["fresh_context_forecast"]["valid_until_time_index"] = Value::from(-1);
        let changed_panel = serde_json::to_vec(&panel).unwrap();
        let mut experiment: Value = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        experiment["source_panel_sha256"] = Value::String(sha256_hex(&changed_panel));
        let changed_experiment = serde_json::to_vec(&experiment).unwrap();
        let error = build_report(&changed_panel, &changed_experiment).unwrap_err();
        assert!(error.contains("temporal contract changed"));
    }
}
