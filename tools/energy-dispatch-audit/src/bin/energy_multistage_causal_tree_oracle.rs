//! Exact, bounded reference oracle for a multi-stage causal energy-policy tree.
//!
//! Research/qualification tool only. It is not a physical dispatch solver and
//! never treats a scenario-wise wait-and-see schedule as a deployable policy.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

const SCHEMA: &str = "energy-multistage-causal-tree-v0.1";
const FIXTURE: &str = include_str!("../../fixtures/energy-multistage-causal-tree-v0.1.json");
const OBJECTIVE_ORDER: [&str; 4] = [
    "worst_case_essential_unserved_quanta",
    "worst_case_flexible_unserved_quanta",
    "total_essential_unserved_quanta",
    "total_flexible_unserved_quanta",
];
const EXPECTED_SCENARIOS: [&str; 4] = [
    "early_outage_clears",
    "early_outage_persists",
    "late_outage",
    "no_late_outage",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    benchmark_id: String,
    status: String,
    claim_ceiling: String,
    model: Model,
    forecast_contract: ForecastContract,
    scenarios: Vec<Scenario>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    initial_battery_soc_quanta: u32,
    battery_capacity_quanta: u32,
    max_battery_discharge_quanta_per_interval: u32,
    actions: Vec<String>,
    grid_rule: String,
    objective_directions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastContract {
    scenario_probabilities: Option<Value>,
    note: String,
    unknown_forecasts_available_at_t0: BTreeMap<String, Forecast>,
    unknown_forecasts_available_at_t1: BTreeMap<String, Forecast>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Forecast {
    value: Value,
    available_at_time_index: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    intervals: Vec<Interval>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interval {
    time_index: usize,
    observable_state: ObservableState,
    #[serde(default)]
    available_forecast: BTreeMap<String, Forecast>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservableState {
    grid_available: bool,
    essential_demand_quanta: u32,
    flexible_demand_quanta: u32,
}

#[derive(Clone, Debug)]
struct StepRecord {
    soc_before: u32,
    soc_after: u32,
    essential_unserved: u32,
    flexible_unserved: u32,
    discharge: u32,
}

#[derive(Clone, Debug)]
struct Schedule {
    actions: Vec<String>,
    steps: Vec<StepRecord>,
    essential_unserved: u32,
    flexible_unserved: u32,
}

impl Schedule {
    fn total_discharge(&self) -> u32 {
        self.steps.iter().map(|step| step.discharge).sum()
    }
}

#[derive(Clone, Debug, Serialize)]
struct OracleReport {
    schema_version: &'static str,
    benchmark_id: String,
    status: &'static str,
    claim_ceiling: String,
    scenario_count: usize,
    interval_count: usize,
    feasible_schedule_count_by_scenario: BTreeMap<String, usize>,
    feasible_wait_and_see_joint_profile_count: usize,
    causal_joint_policy_count: usize,
    causal_policy_pareto_frontier: Vec<[u32; 4]>,
    wait_and_see_pareto_frontier: Vec<[u32; 4]>,
    objective_order: Vec<&'static str>,
    selected_wait_and_see_reference_deployable: bool,
    selected_wait_and_see_reference_objectives: [u32; 4],
    service_first_causal_frontier_objectives: [u32; 4],
    signed_clairvoyance_gap_causal_minus_wait_and_see: [i64; 4],
    probability_weighted_metrics: &'static str,
    physical_dispatch_audit: &'static str,
    source_fixture_sha256: String,
}

fn parse_fixture(bytes: &str) -> Result<Fixture, String> {
    serde_json::from_str(bytes).map_err(|error| format!("invalid fixture JSON: {error}"))
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != SCHEMA {
        return Err(format!("unsupported schema version: {}", fixture.schema_version));
    }
    if !fixture.claim_ceiling.to_ascii_lowercase().contains("no real site") {
        return Err("synthetic-only claim ceiling missing".to_owned());
    }
    if !fixture.status.to_ascii_lowercase().contains("synthetic") {
        return Err("fixture status must remain explicitly synthetic".to_owned());
    }
    if fixture.model.battery_capacity_quanta != 2
        || fixture.model.initial_battery_soc_quanta > fixture.model.battery_capacity_quanta
        || fixture.model.max_battery_discharge_quanta_per_interval != 1
    {
        return Err("model is outside the registered one-quantum battery oracle".to_owned());
    }
    let expected_actions = ["hold", "serve_essential", "serve_flexible"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if fixture.model.actions != expected_actions {
        return Err("action catalog mismatch".to_owned());
    }
    if fixture.model.grid_rule.is_empty() {
        return Err("grid rule missing".to_owned());
    }
    if fixture.model.objective_directions.len() != OBJECTIVE_ORDER.len()
        || !fixture.model.objective_directions.values().all(|direction| direction == "minimize")
    {
        return Err("objective directions must define four minimized service-gap axes".to_owned());
    }
    if fixture.forecast_contract.scenario_probabilities.is_some() {
        return Err("scenario probabilities are not justified by this deterministic fixture".to_owned());
    }
    if fixture.forecast_contract.note.is_empty() {
        return Err("forecast contract note missing".to_owned());
    }
    if fixture.forecast_contract.unknown_forecasts_available_at_t0.len() != 2
        || fixture.forecast_contract.unknown_forecasts_available_at_t1.len() != 1
    {
        return Err("forecast availability contract does not match the frozen tree".to_owned());
    }
    for forecast in fixture.forecast_contract.unknown_forecasts_available_at_t0.values() {
        if forecast.value != json!("unknown") || forecast.available_at_time_index > 0 {
            return Err("t0 future forecast must remain unknown and available only from t0".to_owned());
        }
    }
    for forecast in fixture.forecast_contract.unknown_forecasts_available_at_t1.values() {
        if forecast.value != json!("unknown") || forecast.available_at_time_index > 1 {
            return Err("t1 future forecast must remain unknown and available only from t1".to_owned());
        }
    }
    if fixture.scenarios.len() != EXPECTED_SCENARIOS.len() {
        return Err("scenario count differs from the frozen four-future panel".to_owned());
    }
    for (scenario, expected_id) in fixture.scenarios.iter().zip(EXPECTED_SCENARIOS) {
        if scenario.scenario_id != expected_id {
            return Err(format!("scenario order/identity mismatch: expected {expected_id}"));
        }
        if scenario.intervals.len() != 3 {
            return Err(format!("{}: expected exactly three intervals", scenario.scenario_id));
        }
        for (expected_time, interval) in scenario.intervals.iter().enumerate() {
            if interval.time_index != expected_time {
                return Err(format!(
                    "{}: missing, duplicate, or reordered interval at index {expected_time}",
                    scenario.scenario_id
                ));
            }
            for (name, forecast) in &interval.available_forecast {
                if forecast.available_at_time_index > interval.time_index {
                    return Err(format!(
                        "{} t{}: forecast {name} is unavailable until t{}",
                        scenario.scenario_id, interval.time_index, forecast.available_at_time_index
                    ));
                }
            }
        }
    }
    if fixture.required_checks.len() < 8 {
        return Err("required-check registry is incomplete".to_owned());
    }
    Ok(())
}

fn simulate_schedule(
    fixture: &Fixture,
    scenario: &Scenario,
    actions: &[String],
) -> Option<Schedule> {
    if actions.len() != scenario.intervals.len() {
        return None;
    }
    let mut soc = fixture.model.initial_battery_soc_quanta;
    let capacity = fixture.model.battery_capacity_quanta;
    let mut steps = Vec::with_capacity(actions.len());
    let mut total_essential_unserved = 0;
    let mut total_flexible_unserved = 0;

    for (interval, action) in scenario.intervals.iter().zip(actions) {
        let state = &interval.observable_state;
        let soc_before = soc;
        let mut essential_served = 0;
        let mut flexible_served = 0;
        let mut discharge = 0;
        if state.grid_available {
            if action != "hold" {
                return None;
            }
            essential_served = state.essential_demand_quanta;
            flexible_served = state.flexible_demand_quanta;
        } else {
            match action.as_str() {
                "hold" => {}
                "serve_essential" => {
                    if state.essential_demand_quanta == 0 || soc == 0 {
                        return None;
                    }
                    discharge = 1;
                    soc -= discharge;
                    essential_served = 1;
                }
                "serve_flexible" => {
                    if state.flexible_demand_quanta == 0 || soc == 0 {
                        return None;
                    }
                    discharge = 1;
                    soc -= discharge;
                    flexible_served = 1;
                }
                _ => return None,
            }
        }
        let essential_unserved = state.essential_demand_quanta - essential_served;
        let flexible_unserved = state.flexible_demand_quanta - flexible_served;
        total_essential_unserved += essential_unserved;
        total_flexible_unserved += flexible_unserved;
        if soc > capacity || discharge > fixture.model.max_battery_discharge_quanta_per_interval {
            return None;
        }
        steps.push(StepRecord {
            soc_before,
            soc_after: soc,
            essential_unserved,
            flexible_unserved,
            discharge,
        });
    }

    Some(Schedule {
        actions: actions.to_vec(),
        steps,
        essential_unserved: total_essential_unserved,
        flexible_unserved: total_flexible_unserved,
    })
}

fn expand_action_sequences(
    catalog: &[String],
    horizon: usize,
    prefix: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    if prefix.len() == horizon {
        output.push(prefix.clone());
        return;
    }
    for action in catalog {
        prefix.push(action.clone());
        expand_action_sequences(catalog, horizon, prefix, output);
        prefix.pop();
    }
}

fn enumerate_schedules(fixture: &Fixture, scenario: &Scenario) -> Vec<Schedule> {
    let mut sequences = Vec::new();
    expand_action_sequences(
        &fixture.model.actions,
        scenario.intervals.len(),
        &mut Vec::new(),
        &mut sequences,
    );
    sequences
        .iter()
        .filter_map(|actions| simulate_schedule(fixture, scenario, actions))
        .collect()
}

fn schedules_by_scenario(fixture: &Fixture) -> Result<Vec<Vec<Schedule>>, String> {
    let groups = fixture
        .scenarios
        .iter()
        .map(|scenario| enumerate_schedules(fixture, scenario))
        .collect::<Vec<_>>();
    if groups.iter().any(Vec::is_empty) {
        return Err("at least one scenario has no feasible complete schedule".to_owned());
    }
    Ok(groups)
}

fn cartesian_profiles(schedule_groups: &[Vec<Schedule>]) -> Vec<Vec<Schedule>> {
    let mut profiles = vec![Vec::new()];
    for group in schedule_groups {
        let mut next = Vec::with_capacity(profiles.len() * group.len());
        for prefix in &profiles {
            for schedule in group {
                let mut candidate = prefix.clone();
                candidate.push(schedule.clone());
                next.push(candidate);
            }
        }
        profiles = next;
    }
    profiles
}

fn information_key(scenario: &Scenario, schedule: &Schedule, time: usize) -> Result<String, String> {
    let mut history = Vec::with_capacity(time + 1);
    for index in 0..=time {
        let interval = &scenario.intervals[index];
        let step = &schedule.steps[index];
        let mut item = json!({
            "time_index": index,
            "observable_state": &interval.observable_state,
            "soc_before_action": step.soc_before,
            "available_forecast": &interval.available_forecast,
        });
        if index < time {
            item["committed_action"] = json!(&schedule.actions[index]);
            item["soc_after_action"] = json!(step.soc_after);
        }
        history.push(item);
    }
    serde_json::to_string(&json!({"observable_history": history}))
        .map_err(|error| format!("could not serialize observable history: {error}"))
}

fn profile_is_causal(scenarios: &[Scenario], profile: &[Schedule]) -> Result<bool, String> {
    if scenarios.len() != profile.len() {
        return Err("joint profile has wrong scenario cardinality".to_owned());
    }
    let mut action_by_information_set: BTreeMap<(usize, String), String> = BTreeMap::new();
    for (scenario, schedule) in scenarios.iter().zip(profile) {
        for time in 0..schedule.actions.len() {
            let key = (time, information_key(scenario, schedule, time)?);
            let action = schedule.actions[time].clone();
            if let Some(existing) = action_by_information_set.get(&key) {
                if existing != &action {
                    return Ok(false);
                }
            } else {
                action_by_information_set.insert(key, action);
            }
        }
    }
    Ok(true)
}

fn objective_vector(profile: &[Schedule]) -> [u32; 4] {
    let worst_essential = profile.iter().map(|schedule| schedule.essential_unserved).max().unwrap_or(0);
    let worst_flexible = profile.iter().map(|schedule| schedule.flexible_unserved).max().unwrap_or(0);
    let total_essential = profile.iter().map(|schedule| schedule.essential_unserved).sum();
    let total_flexible = profile.iter().map(|schedule| schedule.flexible_unserved).sum();
    [worst_essential, worst_flexible, total_essential, total_flexible]
}

fn dominates(left: &[u32; 4], right: &[u32; 4]) -> bool {
    left.iter().zip(right).all(|(a, b)| a <= b)
        && left.iter().zip(right).any(|(a, b)| a < b)
}

fn pareto_frontier(vectors: impl IntoIterator<Item = [u32; 4]>) -> Vec<[u32; 4]> {
    let unique = vectors.into_iter().collect::<BTreeSet<_>>();
    let points = unique.iter().copied().collect::<Vec<_>>();
    points.iter().copied()
        .filter(|point| !points.iter().any(|other| dominates(other, point)))
        .collect()
}

fn compare_schedules(left: &Schedule, right: &Schedule) -> Ordering {
    left.essential_unserved
        .cmp(&right.essential_unserved)
        .then_with(|| left.flexible_unserved.cmp(&right.flexible_unserved))
        .then_with(|| left.total_discharge().cmp(&right.total_discharge()))
        .then_with(|| left.actions.cmp(&right.actions))
}

fn selected_wait_and_see_reference(groups: &[Vec<Schedule>]) -> Result<Vec<Schedule>, String> {
    groups.iter().map(|group| {
        group.iter().min_by(|left, right| compare_schedules(left, right))
            .cloned()
            .ok_or_else(|| "scenario has no wait-and-see reference schedule".to_owned())
    }).collect()
}

fn build_report(fixture: &Fixture, source_bytes: &[u8]) -> Result<OracleReport, String> {
    validate_fixture(fixture)?;
    let groups = schedules_by_scenario(fixture)?;
    let schedule_counts = fixture.scenarios.iter().zip(&groups)
        .map(|(scenario, schedules)| (scenario.scenario_id.clone(), schedules.len()))
        .collect::<BTreeMap<_, _>>();
    let profiles = cartesian_profiles(&groups);
    if profiles.len() != 224 {
        return Err(format!("registered wait-and-see profile count changed: {}", profiles.len()));
    }
    let wait_and_see_frontier = pareto_frontier(profiles.iter().map(|profile| objective_vector(profile)));
    let mut causal_profiles = Vec::new();
    for profile in &profiles {
        if profile_is_causal(&fixture.scenarios, profile)? {
            causal_profiles.push(profile.clone());
        }
    }
    if causal_profiles.len() != 14 {
        return Err(format!("registered causal-policy count changed: {}", causal_profiles.len()));
    }
    let causal_frontier = pareto_frontier(
        causal_profiles.iter().map(|profile| objective_vector(profile)),
    );
    let wait_reference = selected_wait_and_see_reference(&groups)?;
    if profile_is_causal(&fixture.scenarios, &wait_reference)? {
        return Err("wait-and-see reference unexpectedly became deployable".to_owned());
    }
    let wait_objectives = objective_vector(&wait_reference);
    let service_first = causal_frontier.iter()
        .min_by_key(|point| (point[0], point[2], point[1], point[3]))
        .copied()
        .ok_or_else(|| "causal policy frontier is empty".to_owned())?;
    let gap = std::array::from_fn(|index| service_first[index] as i64 - wait_objectives[index] as i64);
    let digest = format!("{:x}", Sha256::digest(source_bytes));

    Ok(OracleReport {
        schema_version: "energy-multistage-causal-tree-oracle-report-v0.1",
        benchmark_id: fixture.benchmark_id.clone(),
        status: "PASS_REFERENCE_ORACLE_ONLY",
        claim_ceiling: fixture.claim_ceiling.clone(),
        scenario_count: fixture.scenarios.len(),
        interval_count: fixture.scenarios.first().map_or(0, |scenario| scenario.intervals.len()),
        feasible_schedule_count_by_scenario: schedule_counts,
        feasible_wait_and_see_joint_profile_count: profiles.len(),
        causal_joint_policy_count: causal_profiles.len(),
        causal_policy_pareto_frontier: causal_frontier,
        wait_and_see_pareto_frontier: wait_and_see_frontier,
        objective_order: OBJECTIVE_ORDER.to_vec(),
        selected_wait_and_see_reference_deployable: false,
        selected_wait_and_see_reference_objectives: wait_objectives,
        service_first_causal_frontier_objectives: service_first,
        signed_clairvoyance_gap_causal_minus_wait_and_see: gap,
        probability_weighted_metrics: "NOT_COMPUTED_NO_EVIDENCE_BACKED_SCENARIO_PROBABILITIES",
        physical_dispatch_audit: "NOT_RUN; abstract information-set oracle only",
        source_fixture_sha256: digest,
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: energy_multistage_causal_tree_oracle [report.json]".to_owned());
    }
    let fixture = parse_fixture(FIXTURE)?;
    let report = build_report(&fixture, FIXTURE.as_bytes())?;
    if let Some(path) = args.get(1) {
        let bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
        fs::write(path, bytes).map_err(|error| format!("could not write report: {error}"))?;
    }
    println!(
        "PASS reference oracle: scenarios={} intervals={} wait-and-see-profiles={} causal-policies={} causal-frontier={} wait-and-see-frontier={}",
        report.scenario_count,
        report.interval_count,
        report.feasible_wait_and_see_joint_profile_count,
        report.causal_joint_policy_count,
        report.causal_policy_pareto_frontier.len(),
        report.wait_and_see_pareto_frontier.len()
    );
    println!("source_fixture_sha256={}", report.source_fixture_sha256);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("multi-stage causal tree oracle failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        parse_fixture(FIXTURE).expect("frozen fixture parses")
    }

    #[test]
    fn exact_registered_counts_frontiers_and_clairvoyance_gap_match_reference() {
        let report = build_report(&fixture(), FIXTURE.as_bytes()).unwrap();
        assert_eq!(report.feasible_schedule_count_by_scenario["early_outage_clears"], 4);
        assert_eq!(report.feasible_schedule_count_by_scenario["early_outage_persists"], 7);
        assert_eq!(report.feasible_schedule_count_by_scenario["late_outage"], 4);
        assert_eq!(report.feasible_schedule_count_by_scenario["no_late_outage"], 2);
        assert_eq!(report.feasible_wait_and_see_joint_profile_count, 224);
        assert_eq!(report.causal_joint_policy_count, 14);
        assert_eq!(
            report.causal_policy_pareto_frontier,
            vec![[0, 1, 0, 4], [1, 0, 1, 0]]
        );
        assert_eq!(
            report.wait_and_see_pareto_frontier,
            vec![[0, 1, 0, 1], [1, 0, 1, 0]]
        );
        assert_eq!(report.signed_clairvoyance_gap_causal_minus_wait_and_see, [0, 0, 0, 3]);
        assert!(!report.selected_wait_and_see_reference_deployable);
        assert_eq!(
            report.probability_weighted_metrics,
            "NOT_COMPUTED_NO_EVIDENCE_BACKED_SCENARIO_PROBABILITIES"
        );
        assert_eq!(report.physical_dispatch_audit, "NOT_RUN; abstract information-set oracle only");
    }

    #[test]
    fn post_reveal_recourse_is_allowed_without_hidden_future_lookahead() {
        let fixture = fixture();
        let groups = schedules_by_scenario(&fixture).unwrap();
        let profiles = cartesian_profiles(&groups);
        let mut found = false;
        for profile in profiles {
            if !profile_is_causal(&fixture.scenarios, &profile).unwrap() {
                continue;
            }
            let clears = &profile[0].actions;
            let persists = &profile[1].actions;
            if clears[0] == persists[0] && clears[1] == persists[1] && clears[2] != persists[2] {
                found = true;
                break;
            }
        }
        assert!(found, "identical histories must share actions until the later outage reveal");
    }

    #[test]
    fn every_registered_causal_profile_respects_full_observable_prefixes() {
        let fixture = fixture();
        let profiles = cartesian_profiles(&schedules_by_scenario(&fixture).unwrap());
        let causal = profiles.iter()
            .filter(|profile| profile_is_causal(&fixture.scenarios, profile).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(causal.len(), 14);
        assert!(causal.iter().all(|profile| profile_is_causal(&fixture.scenarios, profile).unwrap()));
    }

    #[test]
    fn scenario_wise_ideal_is_diagnostic_not_deployable() {
        let fixture = fixture();
        let groups = schedules_by_scenario(&fixture).unwrap();
        let reference = selected_wait_and_see_reference(&groups).unwrap();
        assert!(!profile_is_causal(&fixture.scenarios, &reference).unwrap());
        assert_ne!(reference[0].actions[0], reference[1].actions[0]);
    }

    #[test]
    fn unavailable_future_forecast_fails_closed() {
        let mut fixture = fixture();
        fixture.scenarios[0].intervals[1]
            .available_forecast.get_mut("grid_state_at_t2").unwrap()
            .available_at_time_index = 2;
        assert!(validate_fixture(&fixture).unwrap_err().contains("unavailable"));
    }

    #[test]
    fn scenario_probabilities_are_not_invented() {
        let mut fixture = fixture();
        fixture.forecast_contract.scenario_probabilities = Some(json!([0.25, 0.25, 0.25, 0.25]));
        assert!(validate_fixture(&fixture).unwrap_err().contains("probabilities"));
    }

    #[test]
    fn duplicate_scenario_identity_is_rejected() {
        let mut fixture = fixture();
        fixture.scenarios[1].scenario_id = fixture.scenarios[0].scenario_id.clone();
        assert!(validate_fixture(&fixture).unwrap_err().contains("order/identity"));
    }

    #[test]
    fn missing_or_reordered_interval_fails_closed() {
        let mut fixture = fixture();
        fixture.scenarios[0].intervals.remove(1);
        assert!(validate_fixture(&fixture).unwrap_err().contains("exactly three"));
        let mut fixture = fixture();
        fixture.scenarios[0].intervals.swap(0, 1);
        assert!(validate_fixture(&fixture).unwrap_err().contains("reordered"));
    }

    #[test]
    fn unknown_action_catalog_is_rejected() {
        let mut fixture = fixture();
        fixture.model.actions[0] = "clairvoyant".to_owned();
        assert!(validate_fixture(&fixture).unwrap_err().contains("action catalog"));
    }

    #[test]
    fn invalid_battery_state_is_rejected() {
        let mut fixture = fixture();
        fixture.model.initial_battery_soc_quanta = 3;
        assert!(validate_fixture(&fixture).unwrap_err().contains("model"));
    }

    #[test]
    fn grid_connected_intervals_do_not_accept_battery_service_actions() {
        let fixture = fixture();
        let scenario = &fixture.scenarios[0];
        assert!(simulate_schedule(
            &fixture,
            scenario,
            &["hold".to_owned(), "hold".to_owned(), "serve_essential".to_owned()]
        ).is_none());
    }
}
