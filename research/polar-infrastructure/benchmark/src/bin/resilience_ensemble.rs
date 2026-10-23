use polar_resilience_score::{score, PolicyScore, ScoreInput};
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
    expected_fuel_intensity: Option<f64>,
}

#[derive(Debug)]
struct Observation {
    probability: f64,
    shortfall: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    fuel_intensity: Option<f64>,
}

fn cvar_upper_tail(observations: &[(f64, f64)], confidence: f64) -> Result<f64, String> {
    let tail_mass = 1.0 - confidence;
    if !tail_mass.is_finite() || tail_mass <= 0.0 {
        return Err("CVaR confidence must be in [0, 1)".to_string());
    }

    let mut sorted = observations.to_vec();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));

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
    Ok(tail_loss / tail_mass)
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
    let mut expected_policies: Option<BTreeMap<String, String>> = None;

    for scenario in &input.scenarios {
        if scenario.scenario_id.trim().is_empty() || !ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_string());
        }
        if !scenario.probability.is_finite() || scenario.probability <= 0.0 {
            return Err(format!("scenario {} probability must be finite and positive", scenario.scenario_id));
        }
        probability_sum += scenario.probability;

        let report = score(&scenario.score_input)?;
        let policies: BTreeMap<String, String> = report.results.iter()
            .map(|p| (p.policy_id.clone(), p.method_family.clone()))
            .collect();
        if policies.len() != report.results.len() {
            return Err(format!("scenario {} has duplicate policy IDs", scenario.scenario_id));
        }
        if let Some(expected) = &expected_policies {
            if expected != &policies {
                return Err(format!("scenario {} does not contain the same policy IDs and method families", scenario.scenario_id));
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

    for (policy_id, method_family) in policy_map {
        let mut observations = Vec::with_capacity(scored_scenarios.len());
        for (_scenario_id, probability, scores) in &scored_scenarios {
            let p: &PolicyScore = scores.iter().find(|s| s.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from a scenario"))?;
            let shortfall = (1.0 - p.weighted_service_fraction).clamp(0.0, 1.0);
            observations.push(Observation {
                probability: *probability,
                shortfall,
                gate_pass: p.gate_pass,
                floor_breach_service_hours: p.floor_breach_service_hours,
                fuel_intensity: p.fuel_units_per_weighted_delivered_service_unit_hour,
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
        let cvar_observations: Vec<(f64, f64)> = observations.iter()
            .map(|o| (o.probability, o.shortfall)).collect();
        let cvar_service_shortfall = cvar_upper_tail(&cvar_observations, input.cvar_confidence)?;
        let expected_fuel_intensity = if observations.iter().all(|o| o.fuel_intensity.is_some()) {
            Some(observations.iter().map(|o| o.probability * o.fuel_intensity.unwrap()).sum())
        } else {
            None
        };

        for value in [expected_shortfall, expected_service_fraction, worst_case_shortfall, gate_failure_probability, expected_floor_breach_service_hours, cvar_service_shortfall] {
            if !value.is_finite() {
                return Err(format!("policy {policy_id} produced non-finite ensemble metrics"));
            }
        }

        results.push(EnsemblePolicyScore {
            policy_id,
            method_family,
            expected_weighted_service_fraction: expected_service_fraction,
            expected_service_shortfall: expected_shortfall,
            cvar_service_shortfall,
            worst_case_service_shortfall: worst_case_shortfall,
            gate_failure_probability,
            expected_floor_breach_service_hours,
            expected_fuel_intensity,
        });
    }

    Ok(EnsembleReport {
        schema_version: "0.1.0",
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
    fn aggregate_reports_expected_and_tail_risk_without_raising_evidence_tier() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.result_scope, "weighted_synthetic_scenario_aggregation_not_physical_validation");
        assert_eq!(report.scenario_count, 2);

        let rules = report.results.iter().find(|p| p.policy_id == "transparent_rules").unwrap();
        assert!(rules.cvar_service_shortfall >= rules.expected_service_shortfall);
        assert!(rules.worst_case_service_shortfall + 1e-12 >= rules.cvar_service_shortfall);
        assert!(rules.gate_failure_probability > 0.0);
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
