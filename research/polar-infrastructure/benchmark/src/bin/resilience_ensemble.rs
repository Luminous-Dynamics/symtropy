use polar_resilience_score::{active_service_ids, score, service_contracts_match, PolicyScore, ScoreInput};
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
    scenario_outcomes: Vec<ScenarioPolicyOutcome>,
}

#[derive(Debug, Serialize)]
struct ScenarioPolicyOutcome {
    scenario_id: String,
    probability: f64,
    weighted_service_fraction: f64,
    service_shortfall: f64,
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
    weighted_service_fraction: f64,
    shortfall: f64,
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
        scored_scenarios.push((scenario.scenario_id.clone(), scenario.probability, report.results));
    }

    if (probability_sum - 1.0).abs() > 1e-9 {
        return Err(format!("scenario probabilities must sum to 1.0 (observed {probability_sum})"));
    }

    let policy_map = expected_policies.unwrap_or_default();
    let mut results = Vec::with_capacity(policy_map.len());

    for (policy_id, (method_family, fuel_unit)) in policy_map {
        let mut observations = Vec::with_capacity(scored_scenarios.len());
        for (scenario_id, probability, scores) in &scored_scenarios {
            let p: &PolicyScore = scores.iter().find(|s| s.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from a scenario"))?;
            let shortfall = (1.0 - p.weighted_service_fraction).clamp(0.0, 1.0);
            observations.push(Observation {
                scenario_id: scenario_id.clone(),
                probability: *probability,
                weighted_service_fraction: p.weighted_service_fraction,
                shortfall,
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

        for value in [expected_shortfall, expected_service_fraction, worst_case_shortfall, gate_failure_probability, expected_floor_breach_service_hours, expected_system_floor_breach_episode_count, expected_longest_consecutive_system_floor_breach_hours, worst_case_longest_consecutive_system_floor_breach_hours, earliest_observed_time_to_first_system_floor_breach_hours.unwrap_or(0.0), cvar_service_shortfall] {
            if !value.is_finite() {
                return Err(format!("policy {policy_id} produced non-finite ensemble metrics"));
            }
        }

        let scenario_outcomes = observations.iter().map(|o| ScenarioPolicyOutcome {
            scenario_id: o.scenario_id.clone(),
            probability: o.probability,
            weighted_service_fraction: o.weighted_service_fraction,
            service_shortfall: o.shortfall,
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
            scenario_outcomes,
        });
    }

    Ok(EnsembleReport {
        schema_version: "0.2.0",
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
        assert_eq!(report.schema_version, "0.2.0");
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
