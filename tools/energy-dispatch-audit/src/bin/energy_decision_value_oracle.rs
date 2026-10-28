//! Exact synthetic counterexample for forecast-error ranking versus dispatch value.
//!
//! Research/qualification tool only. This model is abstract and does not
//! represent a physical grid, field forecaster, or deployable controller.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs};

const PANEL_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-decision-value-panel-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-decision-value-experiment-v0.1.json");
const PANEL_SCHEMA: &str = "energy-decision-value-panel-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-decision-value-experiment-v0.1";
const REPORT_SCHEMA: &str = "energy-decision-value-report-v0.1";
const LOWER_MODEL: &str = "lower_mae_tail_blind";
const HIGHER_MODEL: &str = "higher_mae_critical_reserve";
const NO_FORECAST: &str = "no_forecast_conservative";
const PERFECT_INFO: &str = "perfect_information_upper_bound";
const EXPECTED_SCENARIOS: [&str; 4] = [
    "no_future_critical",
    "single_late_critical",
    "persistent_critical_load",
    "single_early_critical",
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
    model: Model,
    forecast_target: String,
    decision_time_index: usize,
    observable_contexts: Vec<String>,
    forecast_models: BTreeMap<String, ForecastModel>,
    scenarios: Vec<Scenario>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    initial_soc_quanta: u32,
    capacity_quanta: u32,
    max_discharge_quanta_per_interval: u32,
    decision_rule: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastModel {
    description: String,
    forecast_by_observable_context: BTreeMap<String, u32>,
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
    required_known_answers: KnownAnswers,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCoverage {
    scenarios: usize,
    intervals_per_scenario: usize,
    forecast_models: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    lower_mae_tail_blind_mae: f64,
    higher_mae_critical_reserve_mae: f64,
    lower_mae_total_essential_unserved: u32,
    higher_mae_total_essential_unserved: u32,
    higher_mae_total_flexible_unserved: u32,
    conservative_total_flexible_unserved: u32,
    perfect_information_total_flexible_unserved: u32,
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
struct ScenarioOutcome {
    scenario_id: String,
    observable_context: String,
    forecast_reserve_quanta: u32,
    actions: Vec<String>,
    steps: Vec<StepTrace>,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyOutcome {
    policy_id: String,
    scenario_rows: Vec<ScenarioOutcome>,
    total_essential_unserved: u32,
    worst_case_essential_unserved: u32,
    total_flexible_unserved: u32,
    worst_case_flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct ScenarioError {
    scenario_id: String,
    observable_context: String,
    true_future_essential_reserve_quanta: u32,
    predicted_future_essential_reserve_quanta: u32,
    absolute_error_quanta: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct ForecastErrorMetrics {
    absolute_error_sum_quanta: u32,
    scenario_count: usize,
    mae_unweighted_finite_panel: f64,
    absolute_errors_by_scenario: Vec<ScenarioError>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct RankInversion {
    lower_error_model: String,
    lower_error_mae: f64,
    higher_error_model: String,
    higher_error_mae: f64,
    lower_error_model_total_essential_unserved: u32,
    higher_error_model_total_essential_unserved: u32,
    claim: String,
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
    forecast_error_metrics: BTreeMap<String, ForecastErrorMetrics>,
    policy_outcomes: BTreeMap<String, PolicyOutcome>,
    rank_inversion: RankInversion,
    physical_dispatch_audit: &'static str,
    perfect_information_reference: &'static str,
    interpretation: String,
}

fn validate_panel(panel: &Panel) -> Result<(), String> {
    if panel.schema_version != PANEL_SCHEMA {
        return Err(format!("unsupported panel schema: {}", panel.schema_version));
    }
    if !panel.claim_ceiling.to_ascii_lowercase().contains("synthetic") {
        return Err("synthetic claim ceiling is required".to_owned());
    }
    if panel.probability_model.is_some() {
        return Err("probability-weighted scoring is unsupported by this fixture".to_owned());
    }
    if panel.model.initial_soc_quanta != 2
        || panel.model.capacity_quanta != 2
        || panel.model.max_discharge_quanta_per_interval != 1
        || panel.decision_time_index != 0
    {
        return Err("model or decision boundary differs from frozen two-quantum fixture".to_owned());
    }
    if panel.model.decision_rule.is_empty() || panel.forecast_target.is_empty() {
        return Err("decision rule and forecast target must be explicit".to_owned());
    }
    let expected_contexts_ordered = ["north", "east", "south"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if panel.observable_contexts != expected_contexts_ordered {
        return Err("observable context registry changed".to_owned());
    }
    if panel.forecast_models.len() != 2
        || !panel.forecast_models.contains_key(LOWER_MODEL)
        || !panel.forecast_models.contains_key(HIGHER_MODEL)
    {
        return Err("forecast model registry mismatch".to_owned());
    }
    let expected_contexts = panel.observable_contexts.iter().cloned().collect::<BTreeSet<_>>();
    for (model_id, model) in &panel.forecast_models {
        if model.description.is_empty() {
            return Err(format!("{model_id}: forecast description missing"));
        }
        if model.forecast_by_observable_context.keys().cloned().collect::<BTreeSet<_>>() != expected_contexts {
            return Err(format!("{model_id}: forecast must cover each observable context exactly"));
        }
        if model.forecast_by_observable_context.values().any(|value| *value > panel.model.capacity_quanta) {
            return Err(format!("{model_id}: forecast reserve outside supported range"));
        }
    }
    if panel.scenarios.len() != EXPECTED_SCENARIOS.len() {
        return Err("expected exactly four scenarios".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut truth_by_context = BTreeMap::<String, u32>::new();
    for (scenario, expected_id) in panel.scenarios.iter().zip(EXPECTED_SCENARIOS) {
        if scenario.scenario_id != expected_id || !ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!("scenario identity/order mismatch: expected {expected_id}"));
        }
        if !expected_contexts.contains(&scenario.observable_context) {
            return Err(format!("{}: unknown observable context", scenario.scenario_id));
        }
        if scenario.intervals.len() != 3 {
            return Err(format!("{}: exactly three intervals required", scenario.scenario_id));
        }
        for (expected_time, interval) in scenario.intervals.iter().enumerate() {
            if interval.time_index != expected_time {
                return Err(format!("{}: missing, duplicate, or reordered interval", scenario.scenario_id));
            }
        }
        let first = &scenario.intervals[0];
        if first.grid_available || first.essential_demand != 0 || first.flexible_demand != 1 {
            return Err(format!("{}: t0 decision state differs from the frozen fixture", scenario.scenario_id));
        }
        let derived_truth = scenario.intervals.iter().skip(1)
            .filter(|interval| !interval.grid_available)
            .map(|interval| interval.essential_demand.min(panel.model.max_discharge_quanta_per_interval))
            .sum::<u32>();
        if scenario.true_future_essential_reserve_quanta != derived_truth {
            return Err(format!("{}: truth label contradicts future essential-reserve need", scenario.scenario_id));
        }
        if let Some(existing) = truth_by_context.insert(
            scenario.observable_context.clone(),
            scenario.true_future_essential_reserve_quanta,
        ) {
            if existing != scenario.true_future_essential_reserve_quanta {
                return Err("same observable context has contradictory forecast target".to_owned());
            }
        }
    }
    if panel.required_checks.len() < 7 {
        return Err("source panel required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn validate_experiment(
    experiment: &Experiment,
    panel: &Panel,
    panel_bytes: &[u8],
) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA {
        return Err(format!("unsupported experiment schema: {}", experiment.schema_version));
    }
    if experiment.benchmark_id != panel.benchmark_id {
        return Err("experiment/panel benchmark identity mismatch".to_owned());
    }
    if experiment.source_panel_path
        != "tools/energy-dispatch-audit/fixtures/energy-decision-value-panel-v0.1.json"
    {
        return Err("source panel path differs from the pinned contract".to_owned());
    }
    let actual_digest = sha256_hex(panel_bytes);
    if experiment.source_panel_sha256 != actual_digest {
        return Err(format!(
            "source panel digest mismatch: expected {}, got {}",
            experiment.source_panel_sha256, actual_digest
        ));
    }
    if experiment.expected_coverage.scenarios != panel.scenarios.len()
        || experiment.expected_coverage.intervals_per_scenario != 3
        || experiment.expected_coverage.forecast_models != panel.forecast_models.len()
    {
        return Err("forecast decision-value fixture coverage changed".to_owned());
    }
    if experiment.required_checks.len() < 7
        || !experiment.claim_ceiling.to_ascii_lowercase().contains("synthetic")
    {
        return Err("experiment claim ceiling or required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn selected_forecast_reserve(
    panel: &Panel,
    scenario: &Scenario,
    policy_id: &str,
) -> Result<u32, String> {
    match policy_id {
        LOWER_MODEL | HIGHER_MODEL => panel.forecast_models.get(policy_id)
            .and_then(|model| model.forecast_by_observable_context.get(&scenario.observable_context))
            .copied()
            .ok_or_else(|| format!("{policy_id}: forecast missing for observable context {}", scenario.observable_context)),
        NO_FORECAST => Ok(panel.model.capacity_quanta),
        PERFECT_INFO => Ok(scenario.true_future_essential_reserve_quanta),
        other => Err(format!("unknown policy ID: {other}")),
    }
}

fn simulate(
    panel: &Panel,
    scenario: &Scenario,
    policy_id: &str,
) -> Result<ScenarioOutcome, String> {
    let forecast_reserve = selected_forecast_reserve(panel, scenario, policy_id)?;
    let mut soc = panel.model.initial_soc_quanta;
    let mut outcomes = Vec::with_capacity(scenario.intervals.len());
    let mut actions = Vec::with_capacity(scenario.intervals.len());
    let mut total_essential = 0u32;
    let mut total_flexible = 0u32;

    for (time_index, interval) in scenario.intervals.iter().enumerate() {
        let soc_before = soc;
        let mut essential_served = 0u32;
        let mut flexible_served = 0u32;
        let mut discharge = 0u32;
        let action = if interval.grid_available {
            essential_served = interval.essential_demand;
            flexible_served = interval.flexible_demand;
            "hold"
        } else if interval.essential_demand > 0 && soc > 0 {
            discharge = 1;
            soc -= discharge;
            essential_served = 1;
            "serve_essential"
        } else if time_index == panel.decision_time_index
            && interval.flexible_demand > 0
            && soc > 0
            && soc.saturating_sub(1) >= forecast_reserve
        {
            discharge = 1;
            soc -= discharge;
            flexible_served = 1;
            "serve_flexible"
        } else if interval.flexible_demand > 0 && soc > 0 {
            // Later flexible actions are permitted only after essential demand
            // has been checked; this frozen panel has no later flexible demand.
            discharge = 1;
            soc -= discharge;
            flexible_served = 1;
            "serve_flexible"
        } else {
            "hold"
        };
        if discharge > panel.model.max_discharge_quanta_per_interval
            || soc > panel.model.capacity_quanta
        {
            return Err("battery state/discharge invariant failed".to_owned());
        }
        let essential_unserved = interval.essential_demand - essential_served;
        let flexible_unserved = interval.flexible_demand - flexible_served;
        total_essential += essential_unserved;
        total_flexible += flexible_unserved;
        actions.push(action.to_owned());
        outcomes.push(StepTrace {
            time_index,
            action: action.to_owned(),
            soc_before,
            soc_after: soc,
            discharge_quanta: discharge,
            essential_unserved,
            flexible_unserved,
        });
    }

    Ok(ScenarioOutcome {
        scenario_id: scenario.scenario_id.clone(),
        observable_context: scenario.observable_context.clone(),
        forecast_reserve_quanta: forecast_reserve,
        actions,
        steps: outcomes,
        essential_unserved: total_essential,
        flexible_unserved: total_flexible,
    })
}

fn policy_outcome(panel: &Panel, policy_id: &str) -> Result<PolicyOutcome, String> {
    let scenario_rows = panel.scenarios.iter()
        .map(|scenario| simulate(panel, scenario, policy_id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PolicyOutcome {
        policy_id: policy_id.to_owned(),
        total_essential_unserved: scenario_rows.iter().map(|row| row.essential_unserved).sum(),
        worst_case_essential_unserved: scenario_rows.iter().map(|row| row.essential_unserved).max().unwrap_or(0),
        total_flexible_unserved: scenario_rows.iter().map(|row| row.flexible_unserved).sum(),
        worst_case_flexible_unserved: scenario_rows.iter().map(|row| row.flexible_unserved).max().unwrap_or(0),
        scenario_rows,
    })
}

fn forecast_error_metrics(panel: &Panel, model_id: &str) -> Result<ForecastErrorMetrics, String> {
    let model = panel.forecast_models.get(model_id)
        .ok_or_else(|| format!("forecast model missing: {model_id}"))?;
    let mut absolute_errors_by_scenario = Vec::with_capacity(panel.scenarios.len());
    let mut sum = 0u32;
    for scenario in &panel.scenarios {
        let predicted = *model.forecast_by_observable_context.get(&scenario.observable_context)
            .ok_or_else(|| format!("context {} has no forecast", scenario.observable_context))?;
        let absolute = predicted.abs_diff(scenario.true_future_essential_reserve_quanta);
        sum += absolute;
        absolute_errors_by_scenario.push(ScenarioError {
            scenario_id: scenario.scenario_id.clone(),
            observable_context: scenario.observable_context.clone(),
            true_future_essential_reserve_quanta: scenario.true_future_essential_reserve_quanta,
            predicted_future_essential_reserve_quanta: predicted,
            absolute_error_quanta: absolute,
        });
    }
    let count = absolute_errors_by_scenario.len();
    Ok(ForecastErrorMetrics {
        absolute_error_sum_quanta: sum,
        scenario_count: count,
        mae_unweighted_finite_panel: f64::from(sum) / count as f64,
        absolute_errors_by_scenario,
    })
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

fn build_report(panel_bytes: &[u8], experiment_bytes: &[u8]) -> Result<Report, String> {
    let panel: Panel = serde_json::from_slice(panel_bytes)
        .map_err(|error| format!("invalid decision-value panel JSON: {error}"))?;
    let experiment: Experiment = serde_json::from_slice(experiment_bytes)
        .map_err(|error| format!("invalid decision-value experiment JSON: {error}"))?;
    validate_panel(&panel)?;
    validate_experiment(&experiment, &panel, panel_bytes)?;

    let mut forecast_error = BTreeMap::new();
    for model_id in [LOWER_MODEL, HIGHER_MODEL] {
        forecast_error.insert(model_id.to_owned(), forecast_error_metrics(&panel, model_id)?);
    }
    let mut policies = BTreeMap::new();
    for policy_id in [LOWER_MODEL, HIGHER_MODEL, NO_FORECAST, PERFECT_INFO] {
        policies.insert(policy_id.to_owned(), policy_outcome(&panel, policy_id)?);
    }
    let low_metrics = forecast_error.get(LOWER_MODEL).unwrap();
    let high_metrics = forecast_error.get(HIGHER_MODEL).unwrap();
    let low_outcome = policies.get(LOWER_MODEL).unwrap();
    let high_outcome = policies.get(HIGHER_MODEL).unwrap();
    let conservative = policies.get(NO_FORECAST).unwrap();
    let perfect = policies.get(PERFECT_INFO).unwrap();
    let known = &experiment.required_known_answers;

    let low_mae = low_metrics.mae_unweighted_finite_panel;
    let high_mae = high_metrics.mae_unweighted_finite_panel;
    let low_essential = low_outcome.total_essential_unserved;
    let high_essential = high_outcome.total_essential_unserved;
    if !close(low_mae, known.lower_mae_tail_blind_mae)
        || !close(high_mae, known.higher_mae_critical_reserve_mae)
        || low_essential != known.lower_mae_total_essential_unserved
        || high_essential != known.higher_mae_total_essential_unserved
        || high_outcome.total_flexible_unserved != known.higher_mae_total_flexible_unserved
        || conservative.total_flexible_unserved != known.conservative_total_flexible_unserved
        || perfect.total_flexible_unserved != known.perfect_information_total_flexible_unserved
        || low_mae >= high_mae
        || low_essential <= high_essential
    {
        return Err("frozen forecast-error/decision-value rank-inversion known answer changed".to_owned());
    }

    let rank_inversion = RankInversion {
        lower_error_model: LOWER_MODEL.to_owned(),
        lower_error_mae: low_mae,
        higher_error_model: HIGHER_MODEL.to_owned(),
        higher_error_mae: high_mae,
        lower_error_model_total_essential_unserved: low_essential,
        higher_error_model_total_essential_unserved: high_essential,
        claim: "Finite synthetic counterexample: the lower-MAE model has worse critical-service outcomes; this is not a generalization or calibrated-performance claim.".to_owned(),
    };

    Ok(Report {
        schema_version: REPORT_SCHEMA,
        benchmark_id: experiment.benchmark_id.clone(),
        status: "PASS_SYNTHETIC_DECISION_VALUE_COUNTEREXAMPLE_ONLY",
        claim_ceiling: experiment.claim_ceiling.clone(),
        source_panel_sha256: sha256_hex(panel_bytes),
        experiment_fixture_sha256: sha256_hex(experiment_bytes),
        probability_weighted_metrics: "NOT_COMPUTED_NO_SUPPORTED_PROBABILITIES",
        forecast_error_metrics: forecast_error,
        policy_outcomes: policies,
        rank_inversion,
        physical_dispatch_audit: "NOT_RUN; abstract one-quantum battery model",
        perfect_information_reference: "NON_DEPLOYABLE_UPPER_BOUND_ONLY",
        interpretation: "Four fixed synthetic scenarios; unweighted MAE describes this finite panel only and is not an expectation or a probability estimate. Critical and flexible service trade-offs remain separately visible.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: energy_decision_value_oracle [report.json]".to_owned());
    }
    let report = build_report(PANEL_BYTES, EXPERIMENT_BYTES)?;
    if let Some(path) = args.get(1) {
        let bytes = serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))?;
        fs::write(path, bytes).map_err(|error| format!("cannot write report: {error}"))?;
    }
    println!("PASS synthetic forecast-error versus decision-value oracle");
    println!("source_panel_sha256={}", report.source_panel_sha256);
    println!("experiment_fixture_sha256={}", report.experiment_fixture_sha256);
    println!("rank_inversion={}", serde_json::to_string(&report.rank_inversion).map_err(|e| e.to_string())?);
    println!("physical_dispatch_audit=NOT_RUN");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy decision-value oracle failed closed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden() -> Report {
        build_report(PANEL_BYTES, EXPERIMENT_BYTES).expect("frozen decision-value fixture must pass")
    }

    #[test]
    fn lower_mae_forecast_has_worse_critical_service() {
        let report = golden();
        let low_mae = &report.forecast_error_metrics[LOWER_MODEL];
        let high_mae = &report.forecast_error_metrics[HIGHER_MODEL];
        assert!(close(low_mae.mae_unweighted_finite_panel, 0.25));
        assert!(close(high_mae.mae_unweighted_finite_panel, 0.5));
        assert_eq!(report.policy_outcomes[LOWER_MODEL].total_essential_unserved, 1);
        assert_eq!(report.policy_outcomes[HIGHER_MODEL].total_essential_unserved, 0);
    }

    #[test]
    fn flexible_service_tradeoff_and_conservative_baseline_are_preserved() {
        let report = golden();
        assert_eq!(report.policy_outcomes[HIGHER_MODEL].total_flexible_unserved, 2);
        assert_eq!(report.policy_outcomes[NO_FORECAST].total_essential_unserved, 0);
        assert_eq!(report.policy_outcomes[NO_FORECAST].total_flexible_unserved, 4);
        assert_eq!(report.policy_outcomes[PERFECT_INFO].total_essential_unserved, 0);
        assert_eq!(report.policy_outcomes[PERFECT_INFO].total_flexible_unserved, 1);
    }

    #[test]
    fn perfect_information_reference_is_explicitly_non_deployable() {
        assert_eq!(golden().perfect_information_reference, "NON_DEPLOYABLE_UPPER_BOUND_ONLY");
    }

    #[test]
    fn forecast_selection_uses_observable_context_not_scenario_identity() {
        let panel: Panel = serde_json::from_slice(PANEL_BYTES).unwrap();
        let scenario = &panel.scenarios[0];
        let mut renamed = scenario.clone();
        renamed.scenario_id = "evaluator-only-renamed-id".to_owned();
        let original = simulate(&panel, scenario, LOWER_MODEL).unwrap();
        let changed_id = simulate(&panel, &renamed, LOWER_MODEL).unwrap();
        assert_eq!(original.actions, changed_id.actions);
        assert_eq!(original.forecast_reserve_quanta, changed_id.forecast_reserve_quanta);
    }

    #[test]
    fn source_digest_tampering_fails_closed() {
        let mut modified: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        modified["claim_ceiling"] = Value::String("Synthetic modified fixture".to_owned());
        let changed = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&changed, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("source panel digest mismatch"));
    }

    #[test]
    fn reordered_interval_fails_closed() {
        let mut modified: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        modified["scenarios"][0]["intervals"].as_array_mut().unwrap().swap(0, 1);
        let changed = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&changed, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("missing, duplicate, or reordered interval"));
    }

    #[test]
    fn unsupported_probability_weights_fail_closed() {
        let mut modified: Value = serde_json::from_slice(PANEL_BYTES).unwrap();
        modified["probability_model"] = serde_json::json!({"uniform":true});
        let changed = serde_json::to_vec(&modified).unwrap();
        let error = build_report(&changed, EXPERIMENT_BYTES).unwrap_err();
        assert!(error.contains("probability-weighted scoring is unsupported"));
    }

    #[test]
    fn report_replay_is_deterministic() {
        let first = golden();
        let second = golden();
        assert_eq!(serde_json::to_vec(&first).unwrap(), serde_json::to_vec(&second).unwrap());
    }
}
