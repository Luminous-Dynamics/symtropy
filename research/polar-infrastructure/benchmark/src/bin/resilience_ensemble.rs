use polar_resilience_score::{active_service_ids, score, service_contracts_match, PolicyScore, ScoreInput, ServiceContract};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnsembleInput {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    cvar_confidence: f64,
    scenarios: Vec<ScenarioInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioInput {
    scenario_id: String,
    probability: f64,
    score_input: ScoreInput,
}

#[derive(Debug, Serialize)]
struct EnsembleReport {
    schema_version: &'static str,
    report_kind: &'static str,
    evidence_class: &'static str,
    cvar_confidence: f64,
    scenario_count: usize,
    result_scope: &'static str,
    results: Vec<EnsemblePolicyScore>,
}

#[derive(Debug, Serialize)]
struct EnsemblePolicyScore {
    policy_id: String,
    method_family: String,
    expected_weighted_service_fraction: f64,
    expected_service_shortfall: f64,
    /// Probability-weighted criticality-weighted normalized service deficit integrated over time.
    expected_weighted_unserved_service_fraction_hours: f64,
    worst_case_weighted_unserved_service_fraction_hours: f64,
    cvar_service_shortfall: f64,
    worst_case_service_shortfall: f64,
    gate_failure_probability: f64,
    expected_floor_breach_service_hours: f64,
    expected_system_floor_breach_episode_count: f64,
    expected_longest_consecutive_system_floor_breach_hours: f64,
    worst_case_longest_consecutive_system_floor_breach_hours: f64,
    /// Earliest observed breach onset across scenarios; None means no scenario breached within its trace.
    earliest_observed_time_to_first_system_floor_breach_hours: Option<f64>,
    expected_fuel_intensity: Option<f64>,
    fuel_unit: Option<String>,
    service_results: Vec<ServiceEnsembleScore>,
    scenario_outcomes: Vec<ScenarioPolicyOutcome>,
}

#[derive(Debug, Serialize)]
struct ServiceEnsembleScore {
    service_id: String,
    rate_unit: String,
    integrated_unit: String,
    expected_service_fraction: Option<f64>,
    expected_service_shortfall: Option<f64>,
    cvar_service_shortfall: Option<f64>,
    worst_case_service_shortfall: Option<f64>,
    floor_breach_probability: f64,
    expected_floor_breach_service_hours: f64,
    expected_floor_breach_episode_count: f64,
    worst_case_longest_consecutive_floor_breach_hours: f64,
    earliest_observed_time_to_first_floor_breach_hours: Option<f64>,
    scenario_outcomes: Vec<ScenarioServiceOutcome>,
}

#[derive(Debug, Serialize)]
struct ScenarioServiceOutcome {
    scenario_id: String,
    probability: f64,
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

#[derive(Debug, Serialize)]
struct ScenarioPolicyOutcome {
    scenario_id: String,
    probability: f64,
    scenario_duration_hours: f64,
    weighted_service_fraction: f64,
    service_shortfall: f64,
    weighted_unserved_service_fraction_hours: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    system_floor_breach_episode_count: usize,
    longest_consecutive_system_floor_breach_hours: f64,
    time_to_first_system_floor_breach_hours: Option<f64>,
}

#[derive(Debug)]
struct Observation {
    scenario_id: String,
    probability: f64,
    scenario_duration_hours: f64,
    weighted_service_fraction: f64,
    shortfall: f64,
    weighted_unserved_service_fraction_hours: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    system_floor_breach_episode_count: usize,
    longest_consecutive_system_floor_breach_hours: f64,
    time_to_first_system_floor_breach_hours: Option<f64>,
    fuel_unit: String,
    fuel_intensity: Option<f64>,
}

fn cvar_upper_tail(observations: &[(f64, f64)], confidence: f64) -> Result<f64, String> {
    let tail_mass = 1.0 - confidence;
    if !tail_mass.is_finite() || tail_mass <= 0.0 {
        return Err("CVaR confidence must be in [0, 1)".to_string());
    }

    let mut sorted = observations.to_vec();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    let maximum_loss = sorted.first().map(|(_, loss)| *loss).unwrap_or(0.0);

    let mut remaining = tail_mass;
    let mut tail_loss = 0.0;
    for (probability, loss) in sorted {
        if probability <= 0.0 || !probability.is_finite() {
            return Err("scenario probabilities must be finite and positive".to_string());
        }
        if !loss.is_finite() || !(0.0..=1.0).contains(&loss) {
            return Err("service shortfall losses must be finite and in [0, 1]".to_string());
        }
        let included_mass = probability.min(remaining);
        tail_loss += included_mass * loss;
        remaining -= included_mass;
        if remaining <= 1e-12 {
            break;
        }
    }
    if remaining > 1e-9 {
        return Err("scenario probability mass does not cover CVaR tail".to_string());
    }
    // The upper-tail mean cannot exceed the maximum loss; clamp only floating-point overshoot.
    Ok((tail_loss / tail_mass).clamp(0.0, maximum_loss))
}


/// Summarize service-specific loss distributions independently of the weighted system score.
/// Active-service consistency is checked by the caller across all scenarios, so a service's
/// shortfall/CVaR is either defined in every scenario or absent in every scenario.
fn aggregate_service_results(
    service_contracts: &[ServiceContract],
    policy_id: &str,
    scored_scenarios: &[(String, f64, f64, Vec<PolicyScore>)],
    cvar_confidence: f64,
) -> Result<Vec<ServiceEnsembleScore>, String> {
    let mut results = Vec::with_capacity(service_contracts.len());

    for contract in service_contracts {
        let mut outcomes = Vec::with_capacity(scored_scenarios.len());
        for (scenario_id, probability, scenario_duration_hours, policy_scores) in scored_scenarios {
            let policy = policy_scores.iter().find(|score| score.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from scenario {scenario_id}"))?;
            let service = policy.services.iter().find(|service| service.service_id == contract.id)
                .ok_or_else(|| format!("service {} missing for policy {policy_id} in scenario {scenario_id}", contract.id))?;
            if service.rate_unit != contract.rate_unit || service.integrated_unit != contract.integrated_unit {
                return Err(format!("service {} changes declared units for policy {policy_id}", contract.id));
            }

            outcomes.push(ScenarioServiceOutcome {
                scenario_id: scenario_id.clone(),
                probability: *probability,
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
        let expected_service_fraction = if active {
            Some(outcomes.iter().map(|outcome| outcome.probability * outcome.service_fraction.unwrap()).sum::<f64>())
        } else {
            None
        };
        let expected_shortfall = if active {
            Some(outcomes.iter().map(|outcome| outcome.probability * outcome.service_shortfall.unwrap()).sum::<f64>())
        } else {
            None
        };
        let cvar_shortfall = if active {
            let tail_observations = outcomes.iter()
                .map(|outcome| (outcome.probability, outcome.service_shortfall.unwrap()))
                .collect::<Vec<_>>();
            Some(cvar_upper_tail(&tail_observations, cvar_confidence)?)
        } else {
            None
        };
        let worst_case_shortfall = if active {
            outcomes.iter().filter_map(|outcome| outcome.service_shortfall).reduce(f64::max)
        } else {
            None
        };
        let floor_breach_probability = outcomes.iter()
            .filter(|outcome| outcome.floor_breach)
            .map(|outcome| outcome.probability)
            .sum::<f64>();
        let expected_floor_breach_service_hours = outcomes.iter()
            .map(|outcome| outcome.probability * outcome.floor_breach_service_hours)
            .sum::<f64>();
        let expected_floor_breach_episode_count = outcomes.iter()
            .map(|outcome| outcome.probability * outcome.floor_breach_episode_count as f64)
            .sum::<f64>();
        let worst_case_longest_consecutive_floor_breach_hours = outcomes.iter()
            .map(|outcome| outcome.longest_consecutive_floor_breach_hours)
            .fold(0.0_f64, f64::max);
        let earliest_observed_time_to_first_floor_breach_hours = outcomes.iter()
            .filter_map(|outcome| outcome.time_to_first_floor_breach_hours)
            .reduce(f64::min);

        for value in [
            floor_breach_probability,
            expected_floor_breach_service_hours,
            expected_floor_breach_episode_count,
            worst_case_longest_consecutive_floor_breach_hours,
            expected_service_fraction.unwrap_or(0.0),
            expected_shortfall.unwrap_or(0.0),
            cvar_shortfall.unwrap_or(0.0),
            worst_case_shortfall.unwrap_or(0.0),
            earliest_observed_time_to_first_floor_breach_hours.unwrap_or(0.0),
        ] {
            if !value.is_finite() {
                return Err(format!("service {} for policy {policy_id} produced non-finite ensemble metrics", contract.id));
            }
        }

        results.push(ServiceEnsembleScore {
            service_id: contract.id.clone(),
            rate_unit: contract.rate_unit.clone(),
            integrated_unit: contract.integrated_unit.clone(),
            expected_service_fraction,
            expected_service_shortfall: expected_shortfall,
            cvar_service_shortfall: cvar_shortfall,
            worst_case_service_shortfall: worst_case_shortfall,
            floor_breach_probability,
            expected_floor_breach_service_hours,
            expected_floor_breach_episode_count,
            worst_case_longest_consecutive_floor_breach_hours,
            earliest_observed_time_to_first_floor_breach_hours,
            scenario_outcomes: outcomes,
        });
    }

    Ok(results)
}

fn aggregate(input: &EnsembleInput) -> Result<EnsembleReport, String> {
    if input.schema_version != "0.1.0"
        || input.artifact_kind != "polar-resilience-ensemble-input"
        || input.evidence_class != "synthetic_only"
        || input.status != "synthetic_fixture"
    {
        return Err("only schema 0.1.0 synthetic_fixture inputs with synthetic_only evidence are accepted".to_string());
    }
    if !input.cvar_confidence.is_finite() || !(0.0..1.0).contains(&input.cvar_confidence) {
        return Err("cvar_confidence must be finite and in [0, 1)".to_string());
    }
    if input.scenarios.is_empty() {
        return Err("at least one scenario is required".to_string());
    }

    let mut ids = BTreeSet::new();
    let mut probability_sum = 0.0;
    let mut scored_scenarios = Vec::with_capacity(input.scenarios.len());
    let mut expected_policies: Option<BTreeMap<String, (String, String)>> = None;
    let mut baseline: Option<&ScoreInput> = None;

    for scenario in &input.scenarios {
        if scenario.scenario_id.trim().is_empty() || !ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_string());
        }
        if !scenario.probability.is_finite() || scenario.probability <= 0.0 {
            return Err(format!("scenario {} probability must be finite and positive", scenario.scenario_id));
        }
        probability_sum += scenario.probability;

        if let Some(first) = baseline {
            if !service_contracts_match(first, &scenario.score_input) {
                return Err(format!("scenario {} changes service-contract semantics", scenario.scenario_id));
            }
            if active_service_ids(first) != active_service_ids(&scenario.score_input) {
                return Err(format!("scenario {} changes the active-service set", scenario.scenario_id));
            }
        } else {
            baseline = Some(&scenario.score_input);
        }

        let report = score(&scenario.score_input)?;
        let policies: BTreeMap<String, (String, String)> = report.results.iter()
            .map(|p| (p.policy_id.clone(), (p.method_family.clone(), p.fuel_unit.clone())))
            .collect();
        if policies.len() != report.results.len() {
            return Err(format!("scenario {} has duplicate policy IDs", scenario.scenario_id));
        }
        if let Some(expected) = &expected_policies {
            if expected != &policies {
                return Err(format!("scenario {} does not contain the same policy IDs, method families and fuel units", scenario.scenario_id));
            }
        } else {
            expected_policies = Some(policies);
        }
        scored_scenarios.push((scenario.scenario_id.clone(), scenario.probability, report.episode_duration_hours, report.results));
    }

    if (probability_sum - 1.0).abs() > 1e-9 {
        return Err(format!("scenario probabilities must sum to 1.0 (observed {probability_sum})"));
    }

    let policy_map = expected_policies.unwrap_or_default();
    let service_contracts = baseline
        .map(|input| input.episode.service_contracts.as_slice())
        .ok_or_else(|| "at least one scenario is required".to_string())?;
    let mut results = Vec::with_capacity(policy_map.len());

    for (policy_id, (method_family, fuel_unit)) in policy_map {
        let mut observations = Vec::with_capacity(scored_scenarios.len());
        for (scenario_id, probability, scenario_duration_hours, scores) in &scored_scenarios {
            let p: &PolicyScore = scores.iter().find(|s| s.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from a scenario"))?;
            let shortfall = (1.0 - p.weighted_service_fraction).clamp(0.0, 1.0);
            observations.push(Observation {
                scenario_id: scenario_id.clone(),
                probability: *probability,
                scenario_duration_hours: *scenario_duration_hours,
                weighted_service_fraction: p.weighted_service_fraction,
                shortfall,
                weighted_unserved_service_fraction_hours: p.weighted_unserved_service_fraction_hours,
                gate_pass: p.gate_pass,
                floor_breach_service_hours: p.floor_breach_service_hours,
                system_floor_breach_episode_count: p.system_floor_breach_episode_count,
                longest_consecutive_system_floor_breach_hours: p.longest_consecutive_system_floor_breach_hours,
                time_to_first_system_floor_breach_hours: p.time_to_first_system_floor_breach_hours,
                fuel_unit: p.fuel_unit.clone(),
                fuel_intensity: p.fuel_units_per_weighted_delivered_service_fraction_hour,
            });
        }

        let expected_shortfall: f64 = observations.iter()
            .map(|o| o.probability * o.shortfall).sum();
        let expected_service_fraction = 1.0 - expected_shortfall;
        let expected_weighted_unserved_service_fraction_hours: f64 = observations.iter()
            .map(|o| o.probability * o.weighted_unserved_service_fraction_hours).sum();
        let worst_case_weighted_unserved_service_fraction_hours = observations.iter()
            .map(|o| o.weighted_unserved_service_fraction_hours).fold(0.0_f64, f64::max);
        let worst_case_shortfall = observations.iter()
            .map(|o| o.shortfall).fold(0.0_f64, f64::max);
        let gate_failure_probability: f64 = observations.iter()
            .filter(|o| !o.gate_pass).map(|o| o.probability).sum();
        let expected_floor_breach_service_hours: f64 = observations.iter()
            .map(|o| o.probability * o.floor_breach_service_hours).sum();
        let expected_system_floor_breach_episode_count: f64 = observations.iter()
            .map(|o| o.probability * o.system_floor_breach_episode_count as f64).sum();
        let expected_longest_consecutive_system_floor_breach_hours: f64 = observations.iter()
            .map(|o| o.probability * o.longest_consecutive_system_floor_breach_hours).sum();
        let worst_case_longest_consecutive_system_floor_breach_hours: f64 = observations.iter()
            .map(|o| o.longest_consecutive_system_floor_breach_hours).fold(0.0_f64, f64::max);
        let earliest_observed_time_to_first_system_floor_breach_hours = observations.iter()
            .filter_map(|o| o.time_to_first_system_floor_breach_hours).reduce(f64::min);
        let cvar_observations: Vec<(f64, f64)> = observations.iter()
            .map(|o| (o.probability, o.shortfall)).collect();
        let cvar_service_shortfall = cvar_upper_tail(&cvar_observations, input.cvar_confidence)?;
        let expected_fuel_intensity = if observations.iter().all(|o| o.fuel_intensity.is_some()) {
            Some(observations.iter().map(|o| o.probability * o.fuel_intensity.unwrap()).sum())
        } else {
            None
        };

        for value in [expected_shortfall, expected_service_fraction, worst_case_shortfall, gate_failure_probability, expected_floor_breach_service_hours, expected_system_floor_breach_episode_count, expected_longest_consecutive_system_floor_breach_hours, worst_case_longest_consecutive_system_floor_breach_hours, earliest_observed_time_to_first_system_floor_breach_hours.unwrap_or(0.0), expected_weighted_unserved_service_fraction_hours, worst_case_weighted_unserved_service_fraction_hours, cvar_service_shortfall] {
            if !value.is_finite() {
                return Err(format!("policy {policy_id} produced non-finite ensemble metrics"));
            }
        }

        let service_results = aggregate_service_results(service_contracts, &policy_id, &scored_scenarios, input.cvar_confidence)?;
        let scenario_outcomes = observations.iter().map(|o| ScenarioPolicyOutcome {
            scenario_id: o.scenario_id.clone(),
            probability: o.probability,
            scenario_duration_hours: o.scenario_duration_hours,
            weighted_service_fraction: o.weighted_service_fraction,
            service_shortfall: o.shortfall,
            weighted_unserved_service_fraction_hours: o.weighted_unserved_service_fraction_hours,
            gate_pass: o.gate_pass,
            floor_breach_service_hours: o.floor_breach_service_hours,
            system_floor_breach_episode_count: o.system_floor_breach_episode_count,
            longest_consecutive_system_floor_breach_hours: o.longest_consecutive_system_floor_breach_hours,
            time_to_first_system_floor_breach_hours: o.time_to_first_system_floor_breach_hours,
        }).collect();

        results.push(EnsemblePolicyScore {
            policy_id,
            method_family,
            expected_weighted_service_fraction: expected_service_fraction,
            expected_service_shortfall: expected_shortfall,
            expected_weighted_unserved_service_fraction_hours,
            worst_case_weighted_unserved_service_fraction_hours,
            cvar_service_shortfall,
            worst_case_service_shortfall: worst_case_shortfall,
            gate_failure_probability,
            expected_floor_breach_service_hours,
            expected_system_floor_breach_episode_count,
            expected_longest_consecutive_system_floor_breach_hours,
            worst_case_longest_consecutive_system_floor_breach_hours,
            earliest_observed_time_to_first_system_floor_breach_hours,
            fuel_unit: if expected_fuel_intensity.is_some() { Some(fuel_unit) } else { None },
            expected_fuel_intensity,
            service_results,
            scenario_outcomes,
        });
    }

    Ok(EnsembleReport {
        schema_version: "0.4.0",
        report_kind: "polar-resilience-ensemble-report",
        evidence_class: "synthetic_only",
        cvar_confidence: input.cvar_confidence,
        scenario_count: input.scenarios.len(),
        result_scope: "weighted_synthetic_scenario_aggregation_not_physical_validation",
        results,
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: resilience_ensemble <synthetic-ensemble-input.json>")
    })?;
    let raw = fs::read_to_string(path)?;
    let input: EnsembleInput = serde_json::from_str(&raw)?;
    let report = aggregate(&input)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("resilience_ensemble: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> EnsembleInput {
        serde_json::from_str(include_str!("../../fixtures/toy-ensemble.json"))
            .expect("ensemble fixture must be valid JSON")
    }

    #[test]
    fn service_specific_tail_metrics_expose_critical_service_loss_hidden_by_system_average() {
        let report = aggregate(&fixture()).expect("synthetic ensemble should aggregate");
        let candidate = report.results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let water = candidate.service_results.iter()
            .find(|service| service.service_id == "water").unwrap();

        assert_eq!(candidate.service_results.len(), 3);
        assert!((water.floor_breach_probability - 0.1).abs() < 1e-12);
        assert!(water.cvar_service_shortfall.unwrap() > candidate.cvar_service_shortfall);
        assert!(water.cvar_service_shortfall.unwrap() >= water.expected_service_shortfall.unwrap());
        let compound = water.scenario_outcomes.iter()
            .find(|outcome| outcome.scenario_id == "toy-compound-failure").unwrap();
        assert!(compound.floor_breach);
        assert!(compound.service_shortfall.unwrap() > 0.0);
        assert_eq!(compound.delivered_unit_hours, 15.0);
        assert_eq!(compound.demanded_unit_hours, 20.0);
    }

    #[test]
    fn inactive_service_has_null_loss_distribution_not_a_fake_zero() {
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
        let candidate = report.results.iter()
            .find(|policy| policy.policy_id == "candidate_full_service").unwrap();
        let communications = candidate.service_results.iter()
            .find(|service| service.service_id == "communications").unwrap();
        assert!(communications.expected_service_fraction.is_none());
        assert!(communications.expected_service_shortfall.is_none());
        assert!(communications.cvar_service_shortfall.is_none());
        assert!(communications.worst_case_service_shortfall.is_none());
        assert_eq!(communications.floor_breach_probability, 0.0);
        assert_eq!(communications.scenario_outcomes.len(), 2);
        assert!(communications.scenario_outcomes.iter().all(|outcome| outcome.service_fraction.is_none()));
    }

    #[test]
    fn reports_temporal_persistence_of_compound_service_failures() {
        let mut input = fixture();
        let scenario_id = input.scenarios[0].scenario_id.clone();
        let candidate = input.scenarios[0].score_input.policy_runs.iter_mut()
            .find(|p| p.policy_id == "candidate_full_service").unwrap();
        candidate.steps[0].delivered.insert("heat".to_string(), 0.0);
        candidate.steps[1].delivered.insert("heat".to_string(), 0.0);

        let report = aggregate(&input).expect("modified synthetic ensemble should aggregate");
        let result = report.results.iter()
            .find(|p| p.policy_id == "candidate_full_service").unwrap();
        assert!(result.worst_case_longest_consecutive_system_floor_breach_hours >= 2.0);
        assert!(result.expected_longest_consecutive_system_floor_breach_hours > 0.0);
        assert!(result.expected_system_floor_breach_episode_count > 0.0);
        assert!((result.expected_weighted_unserved_service_fraction_hours - 0.47).abs() < 1e-12);
        assert!((result.worst_case_weighted_unserved_service_fraction_hours - 4.7).abs() < 1e-12);
        assert_eq!(result.earliest_observed_time_to_first_system_floor_breach_hours, Some(0.0));
        let first = result.scenario_outcomes.iter()
            .find(|outcome| outcome.scenario_id == scenario_id).unwrap();
        assert!(first.system_floor_breach_episode_count > 0);
        assert_eq!(first.time_to_first_system_floor_breach_hours, Some(0.0));
        assert!(first.longest_consecutive_system_floor_breach_hours >= 2.0);
    }

    #[test]
    fn aggregate_reports_expected_and_tail_risk_without_raising_evidence_tier() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert_eq!(report.schema_version, "0.4.0");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.result_scope, "weighted_synthetic_scenario_aggregation_not_physical_validation");
        assert_eq!(report.scenario_count, 2);

        let rules = report.results.iter().find(|p| p.policy_id == "transparent_rules").unwrap();
        assert_eq!(rules.fuel_unit.as_deref(), Some("normalized_fuel_units"));
        assert!(rules.cvar_service_shortfall >= rules.expected_service_shortfall);
        assert!(rules.worst_case_service_shortfall + 1e-12 >= rules.cvar_service_shortfall);
        assert!(rules.gate_failure_probability > 0.0);
        assert_eq!(rules.scenario_outcomes.len(), 2);
        assert!(rules.scenario_outcomes.iter().any(|o| o.scenario_id == "toy-compound-failure" && !o.gate_pass));
    }

    #[test]
    fn adverse_tail_is_not_hidden_by_a_mean() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        let candidate = report.results.iter().find(|p| p.policy_id == "candidate_full_service").unwrap();
        assert!(candidate.worst_case_service_shortfall > candidate.expected_service_shortfall);
        assert!(candidate.cvar_service_shortfall > candidate.expected_service_shortfall);
    }

    #[test]
    fn rejects_probability_mass_that_does_not_sum_to_one() {
        let mut input = fixture();
        input.scenarios[0].probability = 0.2;
        assert!(aggregate(&input).unwrap_err().contains("must sum to 1.0"));
    }

    #[test]
    fn rejects_duplicate_scenario_ids() {
        let mut input = fixture();
        input.scenarios[1].scenario_id = input.scenarios[0].scenario_id.clone();
        assert!(aggregate(&input).unwrap_err().contains("unique"));
    }

    #[test]
    fn rejects_claim_ceiling_escalation() {
        let mut input = fixture();
        input.evidence_class = "domain_validated".to_string();
        assert!(aggregate(&input).unwrap_err().contains("synthetic_only"));
    }

    #[test]
    fn rejects_service_contract_drift_between_scenarios() {
        let mut input = fixture();
        input.scenarios[1].score_input.episode.service_contracts[0].minimum_service_fraction = 0.5;
        assert!(aggregate(&input).unwrap_err().contains("service-contract semantics"));
    }

    #[test]
    fn rejects_active_service_set_drift_between_scenarios() {
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
    fn rejects_policy_set_mismatch_between_scenarios() {
        let mut input = fixture();
        input.scenarios[1].score_input.policy_runs.pop();
        assert!(aggregate(&input).unwrap_err().contains("same policy IDs"));
    }

    #[test]
    fn cvar_uses_fractional_mass_at_tail_boundary() {
        let cvar = cvar_upper_tail(&[(0.02, 1.0), (0.98, 0.0)], 0.95).unwrap();
        assert!((cvar - 0.4).abs() < 1e-12);
    }
}
