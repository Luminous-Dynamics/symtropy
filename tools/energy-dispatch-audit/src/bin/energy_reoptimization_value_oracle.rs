//! Exact synthetic benchmark for event-triggered re-optimization versus fixed cadence.
//!
//! The model is an abstract finite battery fixture. It measures solve counts
//! and service outcomes, not wall-clock runtime or physical network feasibility.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

const PANEL_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-reoptimization-value-panel-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-reoptimization-value-experiment-v0.1.json");
const PANEL_SCHEMA: &str = "energy-reoptimization-value-panel-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-reoptimization-value-experiment-v0.1";
const REPORT_SCHEMA: &str = "energy-reoptimization-value-report-v0.1";
const FIXED: &str = "fixed_cadence";
const EVENT: &str = "event_triggered";
const NO_REOPT: &str = "no_reoptimization";
const HIGH_THRESHOLD: &str = "too_high_threshold";
const EXPECTED_SCENARIOS: [&str; 4] = [
    "no_future_critical",
    "single_late_critical",
    "persistent_critical_load",
    "single_early_critical",
];
const EXPECTED_CONTEXTS: [&str; 4] = ["north", "east", "south", "west"];

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
    initial_forecast_reserve_by_context: BTreeMap<String, u32>,
    updated_forecast_reserve_by_context: BTreeMap<String, u32>,
    observable_residual_by_context: BTreeMap<String, f64>,
    updated_forecast_available_at_time_index: usize,
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
    event_trigger_threshold: f64,
    trigger_boundary: String,
    unknown_forecast_fallback: String,
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
    policies: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    fixed_cadence_solver_calls: usize,
    fixed_cadence_total_essential_unserved: u32,
    fixed_cadence_total_flexible_unserved: u32,
    event_triggered_solver_calls: usize,
    event_triggered_total_essential_unserved: u32,
    event_triggered_total_flexible_unserved: u32,
    no_reoptimization_solver_calls: usize,
    no_reoptimization_total_essential_unserved: u32,
    no_reoptimization_total_flexible_unserved: u32,
    too_high_threshold_solver_calls: usize,
    too_high_threshold_total_essential_unserved: u32,
    too_high_threshold_total_flexible_unserved: u32,
    event_triggered_solve_reduction_percent: f64,
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
struct ScenarioResult {
    scenario_id: String,
    observable_context: String,
    residual_at_decision: String,
    trigger_fired: bool,
    forecast_reserve_used: u32,
    solver_calls: usize,
    actions: Vec<String>,
    steps: Vec<StepTrace>,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyResult {
    policy_id: String,
    qualification_status: String,
    solver_calls_total: usize,
    essential_unserved_total: u32,
    flexible_unserved_total: u32,
    worst_case_essential_unserved: u32,
    worst_case_flexible_unserved: u32,
    scenario_rows: Vec<ScenarioResult>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
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
    policy_results: BTreeMap<String, PolicyResult>,
    event_triggered_solve_reduction_percent: f64,
    interpretation: String,
}

fn validate_panel(panel: &Panel) -> Result<(), String> {
    if panel.schema_version != PANEL_SCHEMA
        || panel.benchmark_id != "event-triggered-reoptimization-versus-fixed-cadence-v0.1"
    {
        return Err("unsupported panel schema or benchmark identity".to_owned());
    }
    if !panel.claim_ceiling.to_ascii_lowercase().contains("synthetic") {
        return Err("synthetic claim ceiling is required".to_owned());
    }
    if panel.probability_model.is_some() {
        return Err("probability-weighted scoring is unsupported".to_owned());
    }
    if panel.model.initial_soc_quanta != 2
        || panel.model.capacity_quanta != 2
        || panel.model.max_discharge_quanta_per_interval != 1
        || panel.model.decision_time_index != 0
    {
        return Err("model differs from the frozen two-quantum domain".to_owned());
    }
    if !panel.model.event_trigger_threshold.is_finite()
        || panel.model.event_trigger_threshold < 0.0
        || panel.model.trigger_boundary != "residual >= threshold triggers a re-solve before committing t0 action"
        || panel.model.unknown_forecast_fallback != "reserve_full_capacity_and_hold_discretionary_flexible_load"
    {
        return Err("trigger semantics or fail-closed fallback changed".to_owned());
    }
    if panel.updated_forecast_available_at_time_index > panel.model.decision_time_index {
        return Err("updated forecast is not available at the decision boundary".to_owned());
    }
    let expected_contexts = EXPECTED_CONTEXTS.into_iter().map(str::to_owned).collect::<Vec<_>>();
    if panel.observable_contexts != expected_contexts {
        return Err("observable context registry changed".to_owned());
    }
    let context_set = panel.observable_contexts.iter().cloned().collect::<BTreeSet<_>>();
    for (name, values) in [
        ("initial forecasts", &panel.initial_forecast_reserve_by_context),
        ("updated forecasts", &panel.updated_forecast_reserve_by_context),
    ] {
        if values.keys().cloned().collect::<BTreeSet<_>>() != context_set {
            return Err(format!("{name} do not cover each observable context exactly"));
        }
        if values.values().any(|v| *v > panel.model.capacity_quanta) {
            return Err(format!("{name} contain a reserve larger than battery capacity"));
        }
    }
    if panel.observable_residual_by_context.keys().cloned().collect::<BTreeSet<_>>() != context_set
        || panel.observable_residual_by_context.values().any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("observable residuals are missing or outside the finite nonnegative domain".to_owned());
    }
    if panel.scenarios.len() != EXPECTED_SCENARIOS.len() {
        return Err("expected exactly four scenarios".to_owned());
    }
    let mut seen = BTreeSet::new();
    for (scenario, expected_id) in panel.scenarios.iter().zip(EXPECTED_SCENARIOS) {
        if scenario.scenario_id != expected_id || !seen.insert(scenario.scenario_id.as_str()) {
            return Err(format!("missing, duplicate, unknown or reordered scenario; expected {expected_id}"));
        }
        if !context_set.contains(&scenario.observable_context) {
            return Err(format!("{}: unknown observable context", scenario.scenario_id));
        }
        if scenario.intervals.len() != 3 {
            return Err(format!("{}: exactly three intervals required", scenario.scenario_id));
        }
        for (expected_time, step) in scenario.intervals.iter().enumerate() {
            if step.time_index != expected_time {
                return Err(format!("{}: missing, duplicate or reordered interval", scenario.scenario_id));
            }
            if step.essential_demand > panel.model.max_discharge_quanta_per_interval
                || step.flexible_demand > 1
            {
                return Err(format!("{}: demand exceeds frozen interval service domain", scenario.scenario_id));
            }
        }
        let t0 = &scenario.intervals[0];
        if t0.grid_available || t0.essential_demand != 0 || t0.flexible_demand != 1 {
            return Err(format!("{}: t0 state differs from frozen domain", scenario.scenario_id));
        }
        let derived_reserve = scenario.intervals.iter().skip(1)
            .filter(|step| !step.grid_available)
            .map(|step| step.essential_demand.min(panel.model.max_discharge_quanta_per_interval))
            .sum::<u32>();
        if derived_reserve != scenario.true_future_essential_reserve_quanta {
            return Err(format!("{}: truth label contradicts future interval data", scenario.scenario_id));
        }
    }
    if panel.required_checks.len() < 8 {
        return Err("panel required-check registry incomplete".to_owned());
    }
    Ok(())
}

fn validate_experiment(experiment: &Experiment, panel: &Panel, panel_bytes: &[u8]) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA
        || experiment.benchmark_id != panel.benchmark_id
        || experiment.source_panel_path != "tools/energy-dispatch-audit/fixtures/energy-reoptimization-value-panel-v0.1.json"
    {
        return Err("experiment identity or source path mismatch".to_owned());
    }
    let actual = sha256_hex(panel_bytes);
    if actual != experiment.source_panel_sha256 {
        return Err(format!("raw source-panel SHA-256 mismatch: expected {}, got {actual}", experiment.source_panel_sha256));
    }
    if experiment.expected_coverage.scenarios != panel.scenarios.len()
        || experiment.expected_coverage.intervals_per_scenario != 3
        || experiment.expected_coverage.policies != 4
        || !experiment.claim_ceiling.to_ascii_lowercase().contains("synthetic")
        || experiment.required_checks.len() < 7
    {
        return Err("experiment coverage or claim contract changed".to_owned());
    }
    Ok(())
}

fn reserve_used(panel: &Panel, context: &str, policy: &str, threshold_override: Option<f64>) -> Result<(u32, bool, usize), String> {
    let initial = *panel.initial_forecast_reserve_by_context.get(context)
        .ok_or_else(|| format!("initial forecast missing for context {context}"))?;
    let updated = *panel.updated_forecast_reserve_by_context.get(context)
        .ok_or_else(|| format!("updated forecast missing for context {context}"))?;
    let residual = *panel.observable_residual_by_context.get(context)
        .ok_or_else(|| format!("observable residual missing for context {context}"))?;
    match policy {
        FIXED => Ok((updated, false, 3)),
        EVENT => {
            let threshold = threshold_override.unwrap_or(panel.model.event_trigger_threshold);
            let trigger = residual >= threshold;
            Ok((if trigger { updated } else { initial }, trigger, if trigger { 2 } else { 1 }))
        }
        NO_REOPT => Ok((initial, false, 1)),
        HIGH_THRESHOLD => {
            let threshold = threshold_override.unwrap_or(3.0);
            let trigger = residual >= threshold;
            Ok((if trigger { updated } else { initial }, trigger, if trigger { 2 } else { 1 }))
        }
        _ => Err(format!("unknown policy {policy}")),
    }
}

fn simulate(panel: &Panel, scenario: &Scenario, policy: &str, threshold_override: Option<f64>) -> Result<ScenarioResult, String> {
    let (reserve, trigger, calls) = reserve_used(panel, &scenario.observable_context, policy, threshold_override)?;
    let residual = *panel.observable_residual_by_context.get(&scenario.observable_context)
        .ok_or_else(|| "observable residual missing".to_owned())?;
    let mut soc = panel.model.initial_soc_quanta;
    let mut steps = Vec::with_capacity(3);
    let mut actions = Vec::with_capacity(3);
    let mut essential_total = 0u32;
    let mut flexible_total = 0u32;
    for interval in &scenario.intervals {
        let before = soc;
        let mut discharge = 0;
        let mut essential_served = 0;
        let mut flexible_served = 0;
        let action = if interval.grid_available {
            essential_served = interval.essential_demand;
            flexible_served = interval.flexible_demand;
            "hold"
        } else if interval.essential_demand > 0 && soc > 0 {
            discharge = 1;
            soc -= discharge;
            essential_served = 1;
            "serve_essential"
        } else if interval.time_index == panel.model.decision_time_index
            && interval.flexible_demand > 0
        {
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
            return Err("battery discharge or capacity invariant violated".to_owned());
        }
        let essential_unserved = interval.essential_demand - essential_served;
        let flexible_unserved = interval.flexible_demand - flexible_served;
        essential_total += essential_unserved;
        flexible_total += flexible_unserved;
        actions.push(action.to_owned());
        steps.push(StepTrace {
            time_index: interval.time_index,
            action: action.to_owned(),
            soc_before: before,
            soc_after: soc,
            discharge_quanta: discharge,
            essential_unserved,
            flexible_unserved,
        });
    }
    Ok(ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        observable_context: scenario.observable_context.clone(),
        residual_at_decision: format!("{residual:.3}"),
        trigger_fired: trigger,
        forecast_reserve_used: reserve,
        solver_calls: calls,
        actions,
        steps,
        essential_unserved: essential_total,
        flexible_unserved: flexible_total,
    })
}

fn policy_result(panel: &Panel, policy: &str, threshold_override: Option<f64>) -> Result<PolicyResult, String> {
    let rows = panel.scenarios.iter()
        .map(|s| simulate(panel, s, policy, threshold_override))
        .collect::<Result<Vec<_>, _>>()?;
    let qualified = true;
    Ok(PolicyResult {
        policy_id: policy.to_owned(),
        qualification_status: if qualified { "QUALIFIED_SYNTHETIC_POLICY" } else { "DISQUALIFIED" }.to_owned(),
        solver_calls_total: rows.iter().map(|r| r.solver_calls).sum(),
        essential_unserved_total: rows.iter().map(|r| r.essential_unserved).sum(),
        flexible_unserved_total: rows.iter().map(|r| r.flexible_unserved).sum(),
        worst_case_essential_unserved: rows.iter().map(|r| r.essential_unserved).max().unwrap_or(0),
        worst_case_flexible_unserved: rows.iter().map(|r| r.flexible_unserved).max().unwrap_or(0),
        scenario_rows: rows,
    })
}

fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

fn build_report(panel_bytes: &[u8], experiment_bytes: &[u8]) -> Result<Report, String> {
    let panel: Panel = serde_json::from_slice(panel_bytes).map_err(|e| format!("invalid panel JSON: {e}"))?;
    let experiment: Experiment = serde_json::from_slice(experiment_bytes).map_err(|e| format!("invalid experiment JSON: {e}"))?;
    validate_panel(&panel)?;
    validate_experiment(&experiment, &panel, panel_bytes)?;
    let mut policies = BTreeMap::new();
    for id in [FIXED, EVENT, NO_REOPT, HIGH_THRESHOLD] {
        policies.insert(id.to_owned(), policy_result(&panel, id, None)?);
    }
    let fixed = &policies[FIXED];
    let event = &policies[EVENT];
    let never = &policies[NO_REOPT];
    let high = &policies[HIGH_THRESHOLD];
    let known = &experiment.known_answers;
    let reduction = 100.0 * (fixed.solver_calls_total as f64 - event.solver_calls_total as f64)
        / fixed.solver_calls_total as f64;
    if fixed.solver_calls_total != known.fixed_cadence_solver_calls
        || fixed.essential_unserved_total != known.fixed_cadence_total_essential_unserved
        || fixed.flexible_unserved_total != known.fixed_cadence_total_flexible_unserved
        || event.solver_calls_total != known.event_triggered_solver_calls
        || event.essential_unserved_total != known.event_triggered_total_essential_unserved
        || event.flexible_unserved_total != known.event_triggered_total_flexible_unserved
        || never.solver_calls_total != known.no_reoptimization_solver_calls
        || never.essential_unserved_total != known.no_reoptimization_total_essential_unserved
        || never.flexible_unserved_total != known.no_reoptimization_total_flexible_unserved
        || high.solver_calls_total != known.too_high_threshold_solver_calls
        || high.essential_unserved_total != known.too_high_threshold_total_essential_unserved
        || high.flexible_unserved_total != known.too_high_threshold_total_flexible_unserved
        || !close(reduction, known.event_triggered_solve_reduction_percent)
        || event.scenario_rows.iter().filter(|r| r.trigger_fired).count() != 1
        || event.solver_calls_total >= fixed.solver_calls_total
        || event.essential_unserved_total != fixed.essential_unserved_total
        || event.flexible_unserved_total != fixed.flexible_unserved_total
    {
        return Err("fixed/event/no-reoptimization known answer changed".to_owned());
    }
    Ok(Report {
        schema_version: REPORT_SCHEMA,
        benchmark_id: experiment.benchmark_id.clone(),
        status: "PASS_SYNTHETIC_REOPTIMIZATION_VALUE_ONLY",
        claim_ceiling: experiment.claim_ceiling.clone(),
        source_panel_sha256: sha256_hex(panel_bytes),
        experiment_fixture_sha256: sha256_hex(experiment_bytes),
        probability_weighted_metrics: "NOT_COMPUTED_NO_SUPPORTED_PROBABILITIES",
        physical_dispatch_audit: "NOT_RUN_ABSTRACT_BATTERY_MODEL",
        scenario_count: panel.scenarios.len(),
        interval_count: panel.scenarios.iter().map(|s| s.intervals.len()).sum(),
        policy_results: policies,
        event_triggered_solve_reduction_percent: reduction,
        interpretation: "In this synthetic exact fixture, the residual-triggered policy matches the fixed-cadence service vector with fewer modelled solve calls. Solver-call counts are not wall-clock runtime, and this does not establish a general regret bound.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() > 2 { return Err("usage: energy_reoptimization_value_oracle [report.json]".to_owned()); }
    let report = build_report(PANEL_BYTES, EXPERIMENT_BYTES)?;
    if let Some(path) = args.get(1) {
        let bytes = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
        fs::write(path, bytes).map_err(|e| format!("cannot write report: {e}"))?;
    }
    println!("PASS synthetic event-triggered re-optimization value benchmark");
    println!("source_panel_sha256={}", report.source_panel_sha256);
    println!("experiment_fixture_sha256={}", report.experiment_fixture_sha256);
    println!("event_triggered_solve_reduction_percent={:.6}", report.event_triggered_solve_reduction_percent);
    for result in report.policy_results.values() {
        println!("policy={} solver_calls={} essential_unserved={} flexible_unserved={}",
            result.policy_id, result.solver_calls_total, result.essential_unserved_total, result.flexible_unserved_total);
    }
    println!("physical_dispatch_audit=NOT_RUN");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy re-optimization value oracle failed closed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden() -> Report {
        build_report(PANEL_BYTES, EXPERIMENT_BYTES).expect("frozen reoptimization fixture must pass")
    }

    #[test]
    fn fixed_and_event_triggered_match_service_with_fewer_calls() {
        let report = golden();
        let fixed = &report.policy_results[FIXED];
        let event = &report.policy_results[EVENT];
        assert_eq!(fixed.solver_calls_total, 12);
        assert_eq!(event.solver_calls_total, 5);
        assert_eq!((fixed.essential_unserved_total, fixed.flexible_unserved_total), (0, 1));
        assert_eq!((event.essential_unserved_total, event.flexible_unserved_total), (0, 1));
        assert!(close(report.event_triggered_solve_reduction_percent, 58.333333333333336));
    }

    #[test]
    fn no_reoptimization_and_too_high_threshold_reveal_missed_surprise_cost() {
        let report = golden();
        for id in [NO_REOPT, HIGH_THRESHOLD] {
            let r = &report.policy_results[id];
            assert_eq!(r.solver_calls_total, 4);
            assert_eq!((r.essential_unserved_total, r.flexible_unserved_total), (1, 0));
        }
    }

    #[test]
    fn trigger_boundary_is_inclusive() {
        let mut p: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        p["model"]["event_trigger_threshold"] = Value::from(2.0);
        let pbytes = serde_json::to_vec(&p).unwrap();
        let mut e: Value = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        e["source_panel_sha256"] = Value::String(sha256_hex(&pbytes));
        let ebytes = serde_json::to_vec(&e).unwrap();
        let r = build_report(&pbytes, &ebytes).expect("residual equal to threshold must trigger");
        assert_eq!(r.policy_results[EVENT].solver_calls_total, 5);
        assert_eq!(r.policy_results[EVENT].essential_unserved_total, 0);
    }

    #[test]
    fn scenario_id_does_not_change_controller_decision() {
        let p: Panel = serde_json::from_slice(PANEL_BYTES).unwrap();
        let s = &p.scenarios[2];
        let baseline = simulate(&p, s, EVENT, None).unwrap();
        let mut renamed = s.clone();
        renamed.scenario_id = "hidden-evaluator-id".to_owned();
        let altered = simulate(&p, &renamed, EVENT, None).unwrap();
        assert_eq!(baseline.actions, altered.actions);
        assert_eq!(baseline.forecast_reserve_used, altered.forecast_reserve_used);
        assert_eq!(baseline.trigger_fired, altered.trigger_fired);
    }

    #[test]
    fn source_digest_tampering_fails_closed() {
        let mut p: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        p["claim_ceiling"] = Value::String("Synthetic altered panel".to_owned());
        let pbytes = serde_json::to_vec(&p).unwrap();
        let err = build_report(&pbytes, EXPERIMENT_BYTES).unwrap_err();
        assert!(err.contains("raw source-panel SHA-256 mismatch"));
    }

    #[test]
    fn future_truth_label_mutation_fails_closed_after_digest_rebind() {
        let mut p: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        p["scenarios"][2]["true_future_essential_reserve_quanta"] = Value::from(1);
        let pbytes = serde_json::to_vec(&p).unwrap();
        let mut e: Value = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        e["source_panel_sha256"] = Value::String(sha256_hex(&pbytes));
        let ebytes = serde_json::to_vec(&e).unwrap();
        let err = build_report(&pbytes, &ebytes).unwrap_err();
        assert!(err.contains("truth label contradicts future interval data"));
    }

    #[test]
    fn unsupported_probability_model_fails_closed() {
        let mut p: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        p["probability_model"] = serde_json::json!({"uniform": true});
        let pbytes = serde_json::to_vec(&p).unwrap();
        let err = build_report(&pbytes, EXPERIMENT_BYTES).unwrap_err();
        assert!(err.contains("probability-weighted scoring is unsupported"));
    }

    #[test]
    fn deterministic_report_replay() {
        let a = golden();
        let b = golden();
        assert_eq!(serde_json::to_vec(&a).unwrap(), serde_json::to_vec(&b).unwrap());
    }
}
