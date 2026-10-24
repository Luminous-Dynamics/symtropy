use polar_resilience_score::{active_service_ids, score, service_contracts_match, PolicyScore, ScoreInput, ServiceContract};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RobustSetInput {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    scenarios: Vec<RobustScenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RobustScenario {
    scenario_id: String,
    score_input: ScoreInput,
}

#[derive(Debug, Serialize)]
struct RobustSetReport {
    schema_version: &'static str,
    report_kind: &'static str,
    evidence_class: &'static str,
    scenario_count: usize,
    result_scope: &'static str,
    /// Complete only when every policy has fuel-intensity data for every scenario.
    pareto_comparison_status: &'static str,
    policy_results: Vec<RobustPolicyScore>,
}

#[derive(Debug, Clone, Serialize)]
struct RobustPolicyScore {
    policy_id: String,
    method_family: String,
    fuel_unit: Option<String>,
    service_floor_breach_scenario_count: usize,
    maximum_system_floor_breach_episode_count_across_scenarios: usize,
    passed_all_declared_service_floors: bool,
    worst_case_service_shortfall: f64,
    unweighted_mean_shortfall_across_stress_suite: f64,
    worst_case_weighted_unserved_service_fraction_hours: f64,
    unweighted_mean_weighted_unserved_service_fraction_hours: f64,
    worst_floor_breach_service_hours: f64,
    worst_case_longest_consecutive_system_floor_breach_hours: f64,
    /// Earliest observed breach onset across scenarios; None means no scenario breached within its trace.
    earliest_observed_time_to_first_system_floor_breach_hours: Option<f64>,
    maximum_fuel_intensity_across_scenarios: Option<f64>,
    /// None means the frontier is incomplete because some fuel-intensity data is missing.
    pareto_nondominated: Option<bool>,
    service_results: Vec<RobustServiceScore>,
    scenario_outcomes: Vec<ScenarioOutcome>,
}

#[derive(Debug, Clone, Serialize)]
struct RobustServiceScore {
    service_id: String,
    rate_unit: String,
    integrated_unit: String,
    active_in_all_scenarios: bool,
    service_floor_breach_scenario_count: usize,
    passed_all_declared_service_floors: Option<bool>,
    worst_case_service_shortfall: Option<f64>,
    unweighted_mean_service_shortfall_across_stress_suite: Option<f64>,
    worst_floor_breach_service_hours: f64,
    maximum_floor_breach_episode_count_across_scenarios: usize,
    worst_case_longest_consecutive_floor_breach_hours: f64,
    earliest_observed_time_to_first_floor_breach_hours: Option<f64>,
    scenario_outcomes: Vec<ScenarioServiceOutcome>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioServiceOutcome {
    scenario_id: String,
    scenario_duration_hours: f64,
    service_fraction: Option<f64>,
    service_shortfall: Option<f64>,
    floor_breach: bool,
    demanded_unit_hours: f64,
    delivered_unit_hours: f64,
    floor_breach_service_hours: f64,
    floor_breach_episode_count: usize,
    longest_consecutive_floor_breach_hours: f64,
    time_to_first_floor_breach_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioOutcome {
    scenario_id: String,
    scenario_duration_hours: f64,
    weighted_service_fraction: f64,
    fuel_unit: String,
    service_shortfall: f64,
    weighted_unserved_service_fraction_hours: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    system_floor_breach_episode_count: usize,
    longest_consecutive_system_floor_breach_hours: f64,
    time_to_first_system_floor_breach_hours: Option<f64>,
    fuel_intensity: Option<f64>,
}

#[derive(Debug)]
struct Observation {
    scenario_id: String,
    scenario_duration_hours: f64,
    weighted_service_fraction: f64,
    fuel_unit: String,
    shortfall: f64,
    weighted_unserved_service_fraction_hours: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    system_floor_breach_episode_count: usize,
    longest_consecutive_system_floor_breach_hours: f64,
    time_to_first_system_floor_breach_hours: Option<f64>,
    fuel_intensity: Option<f64>,
}

/// Compare an observed-time-to-first-breach objective where larger is better.
/// None means no breach was observed during the common, declared stress-test horizon,
/// so it beats any observed breach but is never reported as infinite duration.
fn maximize_observed_breach_onset(a: Option<f64>, b: Option<f64>, eps: f64) -> (bool, bool) {
    match (a, b) {
        (None, None) => (true, false),
        (None, Some(_)) => (true, true),
        (Some(_), None) => (false, false),
        (Some(a), Some(b)) => (a + eps >= b, a > b + eps),
    }
}

fn dominates(a: &RobustPolicyScore, b: &RobustPolicyScore) -> bool {
    // A Pareto comparison needs a measured fuel objective for both options.
    // Missing is neither zero nor infinity, and cannot establish dominance.
    let (Some(af), Some(bf)) = (
        a.maximum_fuel_intensity_across_scenarios,
        b.maximum_fuel_intensity_across_scenarios,
    ) else {
        return false;
    };
    let mut av = vec![
        a.service_floor_breach_scenario_count as f64,
        a.worst_case_service_shortfall,
        a.unweighted_mean_shortfall_across_stress_suite,
        a.worst_case_weighted_unserved_service_fraction_hours,
        a.unweighted_mean_weighted_unserved_service_fraction_hours,
        a.worst_floor_breach_service_hours,
        a.maximum_system_floor_breach_episode_count_across_scenarios as f64,
        a.worst_case_longest_consecutive_system_floor_breach_hours,
        af,
    ];
    let mut bv = vec![
        b.service_floor_breach_scenario_count as f64,
        b.worst_case_service_shortfall,
        b.unweighted_mean_shortfall_across_stress_suite,
        b.worst_case_weighted_unserved_service_fraction_hours,
        b.unweighted_mean_weighted_unserved_service_fraction_hours,
        b.worst_floor_breach_service_hours,
        b.maximum_system_floor_breach_episode_count_across_scenarios as f64,
        b.worst_case_longest_consecutive_system_floor_breach_hours,
        bf,
    ];

    // A system-wide average must not erase a per-service regression. For each
    // active service, compare breach-scenario count, worst shortfall, breach
    // service-hours, maximum breach-window count, and longest contiguous breach.
    let eps = 1e-12;
    let mut service_time_no_worse = true;
    let mut service_time_strictly_better = false;

    if a.service_results.len() != b.service_results.len() {
        return false;
    }
    for a_service in &a.service_results {
        let Some(b_service) = b.service_results.iter()
            .find(|service| service.service_id == a_service.service_id) else {
            return false;
        };
        if a_service.active_in_all_scenarios != b_service.active_in_all_scenarios {
            return false;
        }
        if !a_service.active_in_all_scenarios {
            continue;
        }
        let (Some(a_shortfall), Some(b_shortfall), Some(a_mean_shortfall), Some(b_mean_shortfall)) = (
            a_service.worst_case_service_shortfall,
            b_service.worst_case_service_shortfall,
            a_service.unweighted_mean_service_shortfall_across_stress_suite,
            b_service.unweighted_mean_service_shortfall_across_stress_suite,
        ) else {
            return false;
        };
        let (time_no_worse, time_strictly_better) = maximize_observed_breach_onset(
            a_service.earliest_observed_time_to_first_floor_breach_hours,
            b_service.earliest_observed_time_to_first_floor_breach_hours,
            eps,
        );
        service_time_no_worse &= time_no_worse;
        service_time_strictly_better |= time_strictly_better;
        av.extend([
            a_service.service_floor_breach_scenario_count as f64,
            a_shortfall,
            a_mean_shortfall,
            a_service.worst_floor_breach_service_hours,
            a_service.maximum_floor_breach_episode_count_across_scenarios as f64,
            a_service.worst_case_longest_consecutive_floor_breach_hours,
        ]);
        bv.extend([
            b_service.service_floor_breach_scenario_count as f64,
            b_shortfall,
            b_mean_shortfall,
            b_service.worst_floor_breach_service_hours,
            b_service.maximum_floor_breach_episode_count_across_scenarios as f64,
            b_service.worst_case_longest_consecutive_floor_breach_hours,
        ]);
    }

    let no_worse = (0..av.len()).all(|i| av[i] <= bv[i] + eps);
    let strictly_better = (0..av.len()).any(|i| av[i] + eps < bv[i]);
    let (system_time_no_worse, system_time_strictly_better) = maximize_observed_breach_onset(
        a.earliest_observed_time_to_first_system_floor_breach_hours,
        b.earliest_observed_time_to_first_system_floor_breach_hours,
        eps,
    );
    no_worse
        && service_time_no_worse
        && system_time_no_worse
        && (strictly_better || service_time_strictly_better || system_time_strictly_better)
}


/// Summarize each declared service separately across a probability-free stress set.
/// Inactive services remain explicit with null shortfall fields rather than being counted as zero.
fn aggregate_service_results(
    service_contracts: &[ServiceContract],
    policy_id: &str,
    scored_scenarios: &[(String, f64, Vec<PolicyScore>)],
) -> Result<Vec<RobustServiceScore>, String> {
    let mut results = Vec::with_capacity(service_contracts.len());

    for contract in service_contracts {
        let mut outcomes = Vec::with_capacity(scored_scenarios.len());
        for (scenario_id, scenario_duration_hours, policy_scores) in scored_scenarios {
            let policy = policy_scores.iter().find(|score| score.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from scenario {scenario_id}"))?;
            let service = policy.services.iter().find(|service| service.service_id == contract.id)
                .ok_or_else(|| format!("service {} missing for policy {policy_id} in scenario {scenario_id}", contract.id))?;
            if service.rate_unit != contract.rate_unit || service.integrated_unit != contract.integrated_unit {
                return Err(format!("service {} changes declared units for policy {policy_id}", contract.id));
            }

            outcomes.push(ScenarioServiceOutcome {
                scenario_id: scenario_id.clone(),
                scenario_duration_hours: *scenario_duration_hours,
                service_fraction: service.service_fraction,
                service_shortfall: service.service_fraction.map(|fraction| (1.0 - fraction).clamp(0.0, 1.0)),
                floor_breach: service.floor_breach_episode_count > 0,
                demanded_unit_hours: service.demanded_unit_hours,
                delivered_unit_hours: service.delivered_unit_hours,
                floor_breach_service_hours: service.floor_breach_service_hours,
                floor_breach_episode_count: service.floor_breach_episode_count,
                longest_consecutive_floor_breach_hours: service.longest_consecutive_floor_breach_hours,
                time_to_first_floor_breach_hours: service.time_to_first_floor_breach_hours,
            });
        }

        let active = outcomes.iter().all(|outcome| outcome.service_fraction.is_some());
        if !active && outcomes.iter().any(|outcome| outcome.service_fraction.is_some()) {
            return Err(format!("service {} changes activity across scenarios", contract.id));
        }
        let breach_scenario_count = outcomes.iter().filter(|outcome| outcome.floor_breach).count();
        let worst_case_shortfall = if active {
            outcomes.iter().filter_map(|outcome| outcome.service_shortfall).reduce(f64::max)
        } else {
            None
        };
        let mean_shortfall = if active {
            Some(outcomes.iter().filter_map(|outcome| outcome.service_shortfall).sum::<f64>() / outcomes.len() as f64)
        } else {
            None
        };
        let worst_floor_breach_service_hours = outcomes.iter()
            .map(|outcome| outcome.floor_breach_service_hours).fold(0.0_f64, f64::max);
        let maximum_floor_breach_episode_count_across_scenarios = outcomes.iter()
            .map(|outcome| outcome.floor_breach_episode_count).max().unwrap_or(0);
        let worst_case_longest_consecutive_floor_breach_hours = outcomes.iter()
            .map(|outcome| outcome.longest_consecutive_floor_breach_hours).fold(0.0_f64, f64::max);
        let earliest_observed_time_to_first_floor_breach_hours = outcomes.iter()
            .filter_map(|outcome| outcome.time_to_first_floor_breach_hours).reduce(f64::min);

        for value in [
            worst_floor_breach_service_hours,
            worst_case_longest_consecutive_floor_breach_hours,
            worst_case_shortfall.unwrap_or(0.0),
            mean_shortfall.unwrap_or(0.0),
            earliest_observed_time_to_first_floor_breach_hours.unwrap_or(0.0),
        ] {
            if !value.is_finite() {
                return Err(format!("service {} for policy {policy_id} produced non-finite robust-set metrics", contract.id));
            }
        }

        results.push(RobustServiceScore {
            service_id: contract.id.clone(),
            rate_unit: contract.rate_unit.clone(),
            integrated_unit: contract.integrated_unit.clone(),
            active_in_all_scenarios: active,
            service_floor_breach_scenario_count: breach_scenario_count,
            passed_all_declared_service_floors: active.then_some(breach_scenario_count == 0),
            worst_case_service_shortfall: worst_case_shortfall,
            unweighted_mean_service_shortfall_across_stress_suite: mean_shortfall,
            worst_floor_breach_service_hours,
            maximum_floor_breach_episode_count_across_scenarios,
            worst_case_longest_consecutive_floor_breach_hours,
            earliest_observed_time_to_first_floor_breach_hours,
            scenario_outcomes: outcomes,
        });
    }

    Ok(results)
}

fn aggregate(input: &RobustSetInput) -> Result<RobustSetReport, String> {
    if input.schema_version != "0.1.0"
        || input.artifact_kind != "polar-resilience-robust-set-input"
        || input.evidence_class != "synthetic_only"
        || input.status != "synthetic_fixture"
    {
        return Err("only schema 0.1.0 synthetic_fixture inputs with synthetic_only evidence are accepted".to_string());
    }
    if input.scenarios.is_empty() {
        return Err("at least one enumerated scenario is required".to_string());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut episode_ids = BTreeSet::new();
    let mut scored: Vec<(String, Vec<PolicyScore>)> = Vec::with_capacity(input.scenarios.len());
    let mut policy_families: Option<BTreeMap<String, (String, String)>> = None;
    let mut baseline: Option<&ScoreInput> = None;

    for scenario in &input.scenarios {
        if scenario.scenario_id.trim().is_empty() || !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_string());
        }
        if !episode_ids.insert(scenario.score_input.episode.id.clone()) {
            return Err("episode IDs must also be unique across enumerated scenarios".to_string());
        }
        if let Some(first) = baseline {
            if !service_contracts_match(first, &scenario.score_input) {
                return Err(format!("scenario {} changes time-step or service contract semantics", scenario.scenario_id));
            }
            if active_service_ids(first) != active_service_ids(&scenario.score_input) {
                return Err(format!("scenario {} changes the active-service set", scenario.scenario_id));
            }
        } else {
            baseline = Some(&scenario.score_input);
        }

        let report = score(&scenario.score_input)?;
        let families: BTreeMap<String, (String, String)> = report.results.iter()
            .map(|p| (p.policy_id.clone(), (p.method_family.clone(), p.fuel_unit.clone())))
            .collect();
        if families.len() != report.results.len() {
            return Err(format!("scenario {} contains duplicate policy IDs", scenario.scenario_id));
        }
        if let Some(expected) = &policy_families {
            if expected != &families {
                return Err(format!("scenario {} does not contain the same policy IDs, method families and fuel units", scenario.scenario_id));
            }
        } else {
            policy_families = Some(families);
        }
        scored.push((scenario.scenario_id.clone(), report.episode_duration_hours, report.results));
    }

    let families = policy_families.unwrap_or_default();
    let service_contracts = baseline
        .map(|input| input.episode.service_contracts.as_slice())
        .ok_or_else(|| "at least one enumerated scenario is required".to_string())?;
    let mut results = Vec::with_capacity(families.len());

    for (policy_id, (method_family, fuel_unit)) in families {
        let mut observations = Vec::with_capacity(scored.len());
        for (scenario_id, scenario_duration_hours, scores) in &scored {
            let policy = scores.iter().find(|p| p.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from scenario {scenario_id}"))?;
            observations.push(Observation {
                scenario_id: scenario_id.clone(),
                scenario_duration_hours: *scenario_duration_hours,
                weighted_service_fraction: policy.weighted_service_fraction,
                fuel_unit: policy.fuel_unit.clone(),
                shortfall: (1.0 - policy.weighted_service_fraction).clamp(0.0, 1.0),
                weighted_unserved_service_fraction_hours: policy.weighted_unserved_service_fraction_hours,
                gate_pass: policy.gate_pass,
                floor_breach_service_hours: policy.floor_breach_service_hours,
                system_floor_breach_episode_count: policy.system_floor_breach_episode_count,
                longest_consecutive_system_floor_breach_hours: policy.longest_consecutive_system_floor_breach_hours,
                time_to_first_system_floor_breach_hours: policy.time_to_first_system_floor_breach_hours,
                fuel_intensity: policy.fuel_units_per_weighted_delivered_service_fraction_hour,
            });
        }

        let breach_count = observations.iter().filter(|o| !o.gate_pass).count();
        let maximum_system_floor_breach_episode_count_across_scenarios = observations.iter()
            .map(|o| o.system_floor_breach_episode_count).max().unwrap_or(0);
        let worst_case_shortfall = observations.iter().map(|o| o.shortfall).fold(0.0_f64, f64::max);
        let unweighted_mean_shortfall = observations.iter().map(|o| o.shortfall).sum::<f64>() / observations.len() as f64;
        let worst_case_weighted_unserved_service_fraction_hours = observations.iter()
            .map(|o| o.weighted_unserved_service_fraction_hours).fold(0.0_f64, f64::max);
        let unweighted_mean_weighted_unserved_service_fraction_hours = observations.iter()
            .map(|o| o.weighted_unserved_service_fraction_hours).sum::<f64>() / observations.len() as f64;
        let worst_floor_breach_hours = observations.iter().map(|o| o.floor_breach_service_hours).fold(0.0_f64, f64::max);
        let worst_case_longest_consecutive_system_floor_breach_hours = observations.iter()
            .map(|o| o.longest_consecutive_system_floor_breach_hours).fold(0.0_f64, f64::max);
        let earliest_observed_time_to_first_system_floor_breach_hours = observations.iter()
            .filter_map(|o| o.time_to_first_system_floor_breach_hours).reduce(f64::min);
        let maximum_fuel_intensity = if observations.iter().all(|o| o.fuel_intensity.is_some()) {
            observations.iter().filter_map(|o| o.fuel_intensity).reduce(f64::max)
        } else {
            None
        };
        for value in [worst_case_shortfall, unweighted_mean_shortfall, worst_case_weighted_unserved_service_fraction_hours, unweighted_mean_weighted_unserved_service_fraction_hours, worst_floor_breach_hours, worst_case_longest_consecutive_system_floor_breach_hours, earliest_observed_time_to_first_system_floor_breach_hours.unwrap_or(0.0)] {
            if !value.is_finite() {
                return Err(format!("policy {policy_id} produced non-finite robust-set metrics"));
            }
        }

        let service_results = aggregate_service_results(service_contracts, &policy_id, &scored)?;
        let scenario_outcomes = observations.iter().map(|o| ScenarioOutcome {
            scenario_id: o.scenario_id.clone(),
            scenario_duration_hours: o.scenario_duration_hours,
            weighted_service_fraction: o.weighted_service_fraction,
            fuel_unit: o.fuel_unit.clone(),
            service_shortfall: o.shortfall,
            weighted_unserved_service_fraction_hours: o.weighted_unserved_service_fraction_hours,
            gate_pass: o.gate_pass,
            floor_breach_service_hours: o.floor_breach_service_hours,
            system_floor_breach_episode_count: o.system_floor_breach_episode_count,
            longest_consecutive_system_floor_breach_hours: o.longest_consecutive_system_floor_breach_hours,
            time_to_first_system_floor_breach_hours: o.time_to_first_system_floor_breach_hours,
            fuel_intensity: o.fuel_intensity,
        }).collect();

        results.push(RobustPolicyScore {
            policy_id,
            method_family,
            fuel_unit: if maximum_fuel_intensity.is_some() { Some(fuel_unit) } else { None },
            service_floor_breach_scenario_count: breach_count,
            maximum_system_floor_breach_episode_count_across_scenarios,
            passed_all_declared_service_floors: breach_count == 0,
            worst_case_service_shortfall: worst_case_shortfall,
            unweighted_mean_shortfall_across_stress_suite: unweighted_mean_shortfall,
            worst_case_weighted_unserved_service_fraction_hours,
            unweighted_mean_weighted_unserved_service_fraction_hours,
            worst_floor_breach_service_hours: worst_floor_breach_hours,
            worst_case_longest_consecutive_system_floor_breach_hours,
            earliest_observed_time_to_first_system_floor_breach_hours,
            maximum_fuel_intensity_across_scenarios: maximum_fuel_intensity,
            pareto_nondominated: None,
            service_results,
            scenario_outcomes,
        });
    }

    let pareto_comparison_complete = results.iter()
        .all(|policy| policy.maximum_fuel_intensity_across_scenarios.is_some());
    if pareto_comparison_complete {
        for i in 0..results.len() {
            let nondominated = !(0..results.len())
                .filter(|&j| j != i)
                .any(|j| dominates(&results[j], &results[i]));
            results[i].pareto_nondominated = Some(nondominated);
        }
    }

    Ok(RobustSetReport {
        schema_version: "0.4.0",
        report_kind: "polar-resilience-robust-set-report",
        evidence_class: "synthetic_only",
        scenario_count: input.scenarios.len(),
        result_scope: "unweighted_enumerated_stress_set_not_probability_estimate",
        pareto_comparison_status: if pareto_comparison_complete {
            "complete"
        } else {
            "incomplete_missing_fuel_data"
        },
        policy_results: results,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: robust_set <synthetic-robust-set-input.json>")
    })?;
    let raw = fs::read_to_string(path)?;
    let input: RobustSetInput = serde_json::from_str(&raw)?;
    let report = aggregate(&input)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("robust_set: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> RobustSetInput {
        serde_json::from_str(include_str!("../../fixtures/toy-robust-set.json"))
            .expect("robust set fixture must be valid JSON")
    }

    #[test]
    fn evaluates_enumerated_scenarios_without_probabilities() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert_eq!(report.schema_version, "0.4.0");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.result_scope, "unweighted_enumerated_stress_set_not_probability_estimate");
        assert_eq!(report.scenario_count, 3);
        let candidate = report.policy_results.iter().find(|p| p.policy_id == "candidate_full_service").unwrap();
        assert_eq!(candidate.scenario_outcomes.len(), 3);
        assert!(candidate.scenario_outcomes.iter().any(|o| o.scenario_id == "compound_supply_and_communications" && !o.gate_pass));
    }

    #[test]
    fn candidate_does_not_pass_robust_gate_just_because_nominal_episode_passes() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let candidate = report.policy_results.iter().find(|p| p.policy_id == "candidate_full_service").unwrap();
        assert!(!candidate.passed_all_declared_service_floors);
        assert!(candidate.service_floor_breach_scenario_count > 0);
    }

    #[test]
    fn low_fuel_control_is_not_promoted_over_service_gates() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let defective = report.policy_results.iter().find(|p| p.policy_id == "defective_low_fuel_control").unwrap();
        assert_eq!(defective.fuel_unit.as_deref(), Some("normalized_fuel_units"));
        assert!(!defective.passed_all_declared_service_floors);
        assert!(defective.maximum_fuel_intensity_across_scenarios.unwrap()
            < report.policy_results.iter().find(|p| p.policy_id == "candidate_full_service").unwrap()
                .maximum_fuel_intensity_across_scenarios.unwrap());
    }

    #[test]
    fn service_specific_stress_metrics_expose_water_failure_separately_from_system_average() {
        let report = aggregate(&fixture()).expect("synthetic stress set should aggregate");
        let candidate = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let water = candidate.service_results.iter()
            .find(|service| service.service_id == "water").unwrap();

        assert_eq!(candidate.service_results.len(), 3);
        assert!(water.active_in_all_scenarios);
        assert!(water.service_floor_breach_scenario_count > 0);
        assert_eq!(water.passed_all_declared_service_floors, Some(false));
        assert!(water.worst_case_service_shortfall.unwrap() > candidate.worst_case_service_shortfall);
        assert_eq!(water.scenario_outcomes.len(), 3);
        assert!(water.scenario_outcomes.iter().any(|outcome| {
            outcome.scenario_id == "compound_supply_and_communications"
                && outcome.floor_breach
                && outcome.service_shortfall.unwrap() > 0.0
        }));
    }

    #[test]
    fn inactive_service_has_null_shortfall_in_the_probability_free_stress_set() {
        let mut input = fixture();
        for scenario in &mut input.scenarios {
            for step in &mut scenario.score_input.episode.steps {
                step.demand.insert("communications".to_string(), 0.0);
            }
            for policy in &mut scenario.score_input.policy_runs {
                for step in &mut policy.steps {
                    step.delivered.insert("communications".to_string(), 0.0);
                }
            }
        }

        let report = aggregate(&input).expect("inactive service remains explicitly represented");
        let candidate = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let communications = candidate.service_results.iter()
            .find(|service| service.service_id == "communications").unwrap();
        assert!(!communications.active_in_all_scenarios);
        assert!(communications.worst_case_service_shortfall.is_none());
        assert!(communications.unweighted_mean_service_shortfall_across_stress_suite.is_none());
        assert_eq!(communications.service_floor_breach_scenario_count, 0);
        assert_eq!(communications.passed_all_declared_service_floors, None);
        assert_eq!(communications.scenario_outcomes.len(), 3);
        assert!(communications.scenario_outcomes.iter().all(|outcome| outcome.service_fraction.is_none()));
    }

    #[test]
    fn probability_free_frontier_preserves_time_integrated_service_deficit() {
        let report = aggregate(&fixture()).expect("synthetic fixture should aggregate");
        let candidate = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        assert!((candidate.worst_case_weighted_unserved_service_fraction_hours - 4.7).abs() < 1e-12);
        assert!((candidate.unweighted_mean_weighted_unserved_service_fraction_hours - (7.4 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn reports_longest_contiguous_service_failure_across_stress_scenarios() {
        let mut input = fixture();
        let scenario_id = input.scenarios[0].scenario_id.clone();
        let candidate = input.scenarios[0].score_input.policy_runs.iter_mut()
            .find(|p| p.policy_id == "candidate_full_service").unwrap();
        candidate.steps[0].delivered.insert("heat".to_string(), 0.0);
        candidate.steps[1].delivered.insert("heat".to_string(), 0.0);

        let report = aggregate(&input).expect("modified synthetic stress set should aggregate");
        let result = report.policy_results.iter()
            .find(|p| p.policy_id == "candidate_full_service").unwrap();
        let scenario = result.scenario_outcomes.iter()
            .find(|outcome| outcome.scenario_id == scenario_id).unwrap();
        assert!(scenario.system_floor_breach_episode_count > 0);
        assert!(scenario.weighted_unserved_service_fraction_hours > 0.0);
        assert!(result.worst_case_weighted_unserved_service_fraction_hours >= scenario.weighted_unserved_service_fraction_hours);
        assert!(result.unweighted_mean_weighted_unserved_service_fraction_hours <= result.worst_case_weighted_unserved_service_fraction_hours);
        assert_eq!(scenario.time_to_first_system_floor_breach_hours, Some(0.0));
        assert!(scenario.longest_consecutive_system_floor_breach_hours >= 2.0);
        assert!(result.maximum_system_floor_breach_episode_count_across_scenarios > 0);
        assert!(result.worst_case_longest_consecutive_system_floor_breach_hours >= 2.0);
        assert_eq!(result.earliest_observed_time_to_first_system_floor_breach_hours, Some(0.0));
        assert_eq!(
            result.worst_case_longest_consecutive_system_floor_breach_hours,
            result.scenario_outcomes.iter()
                .map(|outcome| outcome.longest_consecutive_system_floor_breach_hours)
                .fold(0.0_f64, f64::max)
        );
    }

    #[test]
    fn rejects_probability_fields_and_other_unknown_json_fields() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!("../../fixtures/toy-robust-set.json")).unwrap();
        value["scenarios"][0]["probability"] = serde_json::json!(0.9);
        assert!(serde_json::from_value::<RobustSetInput>(value).is_err());
    }

    #[test]
    fn rejects_evidence_claim_escalation() {
        let mut input = fixture();
        input.evidence_class = "domain_validated".to_string();
        assert!(aggregate(&input).unwrap_err().contains("synthetic_only"));
    }

    #[test]
    fn rejects_changed_service_contracts() {
        let mut input = fixture();
        input.scenarios[1].score_input.episode.service_contracts[0].minimum_service_fraction = 0.5;
        assert!(aggregate(&input).unwrap_err().contains("service contract semantics"));
    }

    #[test]
    fn rejects_duplicate_scenario_and_episode_ids() {
        let mut duplicate_scenario = fixture();
        duplicate_scenario.scenarios[1].scenario_id = duplicate_scenario.scenarios[0].scenario_id.clone();
        assert!(aggregate(&duplicate_scenario).unwrap_err().contains("scenario IDs"));

        let mut duplicate_episode = fixture();
        duplicate_episode.scenarios[1].score_input.episode.id = duplicate_episode.scenarios[0].score_input.episode.id.clone();
        assert!(aggregate(&duplicate_episode).unwrap_err().contains("episode IDs"));
    }

    #[test]
    fn rejects_rate_and_integrated_unit_drift_between_scenarios() {
        let mut rate_drift = fixture();
        rate_drift.scenarios[1].score_input.episode.service_contracts[0].rate_unit =
            "incompatible_rate_unit".to_string();
        assert!(aggregate(&rate_drift).unwrap_err().contains("service contract semantics"));

        let mut integrated_drift = fixture();
        integrated_drift.scenarios[1].score_input.episode.service_contracts[0].integrated_unit =
            "incompatible_integrated_unit".to_string();
        assert!(aggregate(&integrated_drift).unwrap_err().contains("service contract semantics"));
    }

    #[test]
    fn rejects_active_service_set_drift() {
        let mut input = fixture();
        let scenario = &mut input.scenarios[1].score_input;
        for step in &mut scenario.episode.steps {
            step.demand.insert("communications".to_string(), 0.0);
        }
        for policy in &mut scenario.policy_runs {
            for step in &mut policy.steps {
                step.delivered.insert("communications".to_string(), 0.0);
            }
        }
        assert!(aggregate(&input).unwrap_err().contains("active-service set"));
    }

    #[test]
    fn rejects_fuel_unit_mismatch_between_scenarios() {
        let mut input = fixture();
        for policy in &mut input.scenarios[1].score_input.policy_runs {
            policy.fuel_unit = "litres".to_string();
        }
        assert!(aggregate(&input).unwrap_err().contains("fuel units"));
    }

    #[test]
    fn rejects_policy_set_mismatch() {
        let mut input = fixture();
        input.scenarios[1].score_input.policy_runs.pop();
        assert!(aggregate(&input).unwrap_err().contains("same policy IDs"));
    }

    #[test]
    fn no_policy_is_claimed_to_dominate_on_one_combined_scalar() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert_eq!(report.pareto_comparison_status, "complete");
        assert!(report.policy_results.iter().any(|p| p.pareto_nondominated == Some(true)));
        // Eligibility is a separate boolean; Pareto status is not safety approval.
        assert!(report.policy_results.iter().any(|p| p.pareto_nondominated == Some(true) && !p.passed_all_declared_service_floors));
    }

    #[test]
    fn pareto_dominance_cannot_hide_a_per_service_shortfall_regression() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let template = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let mut higher_water_loss = template.clone();
        let mut lower_water_loss = template.clone();

        for policy in [&mut higher_water_loss, &mut lower_water_loss] {
            policy.service_floor_breach_scenario_count = 1;
            policy.worst_case_service_shortfall = 0.1;
            policy.worst_floor_breach_service_hours = 1.0;
            policy.maximum_system_floor_breach_episode_count_across_scenarios = 1;
            policy.worst_case_longest_consecutive_system_floor_breach_hours = 1.0;
            policy.maximum_fuel_intensity_across_scenarios = Some(1.0);
        }
        lower_water_loss.service_results = higher_water_loss.service_results.clone();
        higher_water_loss.service_results.iter_mut()
            .find(|service| service.service_id == "water").unwrap()
            .worst_case_service_shortfall = Some(0.4);
        lower_water_loss.service_results.iter_mut()
            .find(|service| service.service_id == "water").unwrap()
            .worst_case_service_shortfall = Some(0.2);

        assert!(!dominates(&higher_water_loss, &lower_water_loss));
        assert!(dominates(&lower_water_loss, &higher_water_loss));
    }

    #[test]
    fn pareto_frontier_preserves_mean_vs_worst_case_tradeoffs() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let template = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let mut strong_worst_case = template.clone();
        let mut strong_mean = template.clone();

        // All other objectives stay equal. A better worst case must not hide
        // a worse unweighted stress-suite mean.
        strong_worst_case.worst_case_service_shortfall = 0.1;
        strong_worst_case.unweighted_mean_shortfall_across_stress_suite = 0.4;
        strong_mean.worst_case_service_shortfall = 0.2;
        strong_mean.unweighted_mean_shortfall_across_stress_suite = 0.3;

        assert!(!dominates(&strong_worst_case, &strong_mean));
        assert!(!dominates(&strong_mean, &strong_worst_case));

        let mut service_better_worst = template.clone();
        let mut service_better_mean = template.clone();
        let water_a = service_better_worst.service_results.iter_mut()
            .find(|service| service.service_id == "water").unwrap();
        water_a.worst_case_service_shortfall = Some(0.1);
        water_a.unweighted_mean_service_shortfall_across_stress_suite = Some(0.4);
        let water_b = service_better_mean.service_results.iter_mut()
            .find(|service| service.service_id == "water").unwrap();
        water_b.worst_case_service_shortfall = Some(0.2);
        water_b.unweighted_mean_service_shortfall_across_stress_suite = Some(0.3);

        assert!(!dominates(&service_better_worst, &service_better_mean));
        assert!(!dominates(&service_better_mean, &service_better_worst));
    }

    #[test]
    fn pareto_frontier_prefers_later_breach_onset_and_no_observed_breach() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let template = report.policy_results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let mut later = template.clone();
        let mut earlier = template.clone();
        later.earliest_observed_time_to_first_system_floor_breach_hours = Some(2.0);
        earlier.earliest_observed_time_to_first_system_floor_breach_hours = Some(0.0);
        for service in &mut later.service_results {
            service.earliest_observed_time_to_first_floor_breach_hours = Some(2.0);
        }
        for service in &mut earlier.service_results {
            service.earliest_observed_time_to_first_floor_breach_hours = Some(0.0);
        }

        assert!(dominates(&later, &earlier));
        assert!(!dominates(&earlier, &later));
        assert_eq!(maximize_observed_breach_onset(None, Some(0.0), 1e-12), (true, true));
        assert_eq!(maximize_observed_breach_onset(Some(0.0), None, 1e-12), (false, false));
    }

    #[test]
    fn missing_fuel_intensity_makes_the_whole_frontier_incomplete() {
        let mut input = fixture();
        for scenario in &mut input.scenarios {
            let policy = scenario.score_input.policy_runs.iter_mut()
                .find(|p| p.policy_id == "defective_low_fuel_control")
                .unwrap();
            for step in &mut policy.steps {
                for delivered in step.delivered.values_mut() {
                    *delivered = 0.0;
                }
            }
        }

        let report = aggregate(&input).expect("zero delivery is valid but cannot establish fuel intensity");
        let defective = report.policy_results.iter()
            .find(|p| p.policy_id == "defective_low_fuel_control")
            .unwrap();
        assert!(defective.maximum_fuel_intensity_across_scenarios.is_none());
        assert_eq!(report.pareto_comparison_status, "incomplete_missing_fuel_data");
        assert!(report.policy_results.iter().all(|p| p.pareto_nondominated.is_none()));
    }
}
