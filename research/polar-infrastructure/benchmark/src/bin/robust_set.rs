use polar_resilience_score::{score, PolicyScore, ScoreInput};
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
    policy_results: Vec<RobustPolicyScore>,
}

#[derive(Debug, Serialize)]
struct RobustPolicyScore {
    policy_id: String,
    method_family: String,
    service_floor_breach_scenario_count: usize,
    passed_all_declared_service_floors: bool,
    worst_case_service_shortfall: f64,
    unweighted_mean_shortfall_across_stress_suite: f64,
    worst_floor_breach_service_hours: f64,
    maximum_fuel_intensity_across_scenarios: Option<f64>,
    pareto_nondominated: bool,
    scenario_outcomes: Vec<ScenarioOutcome>,
}

#[derive(Debug, Serialize)]
struct ScenarioOutcome {
    scenario_id: String,
    weighted_service_fraction: f64,
    service_shortfall: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    fuel_intensity: Option<f64>,
}

#[derive(Debug)]
struct Observation {
    scenario_id: String,
    weighted_service_fraction: f64,
    shortfall: f64,
    gate_pass: bool,
    floor_breach_service_hours: f64,
    fuel_intensity: Option<f64>,
}

fn contracts_match(a: &ScoreInput, b: &ScoreInput) -> bool {
    if a.episode.time_step_hours.to_bits() != b.episode.time_step_hours.to_bits()
        || a.episode.service_contracts.len() != b.episode.service_contracts.len()
    {
        return false;
    }
    a.episode.service_contracts.iter().zip(&b.episode.service_contracts).all(|(left, right)| {
        left.id == right.id
            && left.unit == right.unit
            && left.criticality_weight.to_bits() == right.criticality_weight.to_bits()
            && left.minimum_service_fraction.to_bits() == right.minimum_service_fraction.to_bits()
    })
}

fn dominates(a: &RobustPolicyScore, b: &RobustPolicyScore) -> bool {
    // No scalar blend: compare four disclosed outcomes. Missing fuel data is treated
    // conservatively as infinite for Pareto comparison, while remaining None in the report.
    let af = a.maximum_fuel_intensity_across_scenarios.unwrap_or(f64::INFINITY);
    let bf = b.maximum_fuel_intensity_across_scenarios.unwrap_or(f64::INFINITY);
    let av = [
        a.service_floor_breach_scenario_count as f64,
        a.worst_case_service_shortfall,
        a.worst_floor_breach_service_hours,
        af,
    ];
    let bv = [
        b.service_floor_breach_scenario_count as f64,
        b.worst_case_service_shortfall,
        b.worst_floor_breach_service_hours,
        bf,
    ];
    let eps = 1e-12;
    let no_worse = (0..av.len()).all(|i| av[i] <= bv[i] + eps);
    let strictly_better = (0..av.len()).any(|i| av[i] + eps < bv[i]);
    no_worse && strictly_better
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
    let mut policy_families: Option<BTreeMap<String, String>> = None;
    let mut baseline: Option<&ScoreInput> = None;

    for scenario in &input.scenarios {
        if scenario.scenario_id.trim().is_empty() || !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_string());
        }
        if !episode_ids.insert(scenario.score_input.episode.id.clone()) {
            return Err("episode IDs must also be unique across enumerated scenarios".to_string());
        }
        if let Some(first) = baseline {
            if !contracts_match(first, &scenario.score_input) {
                return Err(format!("scenario {} changes time-step or service contract semantics", scenario.scenario_id));
            }
        } else {
            baseline = Some(&scenario.score_input);
        }

        let report = score(&scenario.score_input)?;
        let families: BTreeMap<String, String> = report.results.iter()
            .map(|p| (p.policy_id.clone(), p.method_family.clone()))
            .collect();
        if families.len() != report.results.len() {
            return Err(format!("scenario {} contains duplicate policy IDs", scenario.scenario_id));
        }
        if let Some(expected) = &policy_families {
            if expected != &families {
                return Err(format!("scenario {} does not contain the same policy IDs and method families", scenario.scenario_id));
            }
        } else {
            policy_families = Some(families);
        }
        scored.push((scenario.scenario_id.clone(), report.results));
    }

    let families = policy_families.unwrap_or_default();
    let mut results = Vec::with_capacity(families.len());

    for (policy_id, method_family) in families {
        let mut observations = Vec::with_capacity(scored.len());
        for (scenario_id, scores) in &scored {
            let policy = scores.iter().find(|p| p.policy_id == policy_id)
                .ok_or_else(|| format!("policy {policy_id} missing from scenario {scenario_id}"))?;
            observations.push(Observation {
                scenario_id: scenario_id.clone(),
                weighted_service_fraction: policy.weighted_service_fraction,
                shortfall: (1.0 - policy.weighted_service_fraction).clamp(0.0, 1.0),
                gate_pass: policy.gate_pass,
                floor_breach_service_hours: policy.floor_breach_service_hours,
                fuel_intensity: policy.fuel_units_per_weighted_delivered_service_unit_hour,
            });
        }

        let breach_count = observations.iter().filter(|o| !o.gate_pass).count();
        let worst_case_shortfall = observations.iter().map(|o| o.shortfall).fold(0.0_f64, f64::max);
        let unweighted_mean_shortfall = observations.iter().map(|o| o.shortfall).sum::<f64>() / observations.len() as f64;
        let worst_floor_breach_hours = observations.iter().map(|o| o.floor_breach_service_hours).fold(0.0_f64, f64::max);
        let maximum_fuel_intensity = if observations.iter().all(|o| o.fuel_intensity.is_some()) {
            observations.iter().filter_map(|o| o.fuel_intensity).reduce(f64::max)
        } else {
            None
        };
        for value in [worst_case_shortfall, unweighted_mean_shortfall, worst_floor_breach_hours] {
            if !value.is_finite() {
                return Err(format!("policy {policy_id} produced non-finite robust-set metrics"));
            }
        }

        let scenario_outcomes = observations.iter().map(|o| ScenarioOutcome {
            scenario_id: o.scenario_id.clone(),
            weighted_service_fraction: o.weighted_service_fraction,
            service_shortfall: o.shortfall,
            gate_pass: o.gate_pass,
            floor_breach_service_hours: o.floor_breach_service_hours,
            fuel_intensity: o.fuel_intensity,
        }).collect();

        results.push(RobustPolicyScore {
            policy_id,
            method_family,
            service_floor_breach_scenario_count: breach_count,
            passed_all_declared_service_floors: breach_count == 0,
            worst_case_service_shortfall: worst_case_shortfall,
            unweighted_mean_shortfall_across_stress_suite: unweighted_mean_shortfall,
            worst_floor_breach_service_hours: worst_floor_breach_hours,
            maximum_fuel_intensity_across_scenarios: maximum_fuel_intensity,
            pareto_nondominated: false,
            scenario_outcomes,
        });
    }

    for i in 0..results.len() {
        let nondominated = !(0..results.len())
            .filter(|&j| j != i)
            .any(|j| dominates(&results[j], &results[i]));
        results[i].pareto_nondominated = nondominated;
    }

    Ok(RobustSetReport {
        schema_version: "0.1.0",
        report_kind: "polar-resilience-robust-set-report",
        evidence_class: "synthetic_only",
        scenario_count: input.scenarios.len(),
        result_scope: "unweighted_enumerated_stress_set_not_probability_estimate",
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
        assert!(!defective.passed_all_declared_service_floors);
        assert!(defective.maximum_fuel_intensity_across_scenarios.unwrap()
            < report.policy_results.iter().find(|p| p.policy_id == "candidate_full_service").unwrap()
                .maximum_fuel_intensity_across_scenarios.unwrap());
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
    fn rejects_policy_set_mismatch() {
        let mut input = fixture();
        input.scenarios[1].score_input.policy_runs.pop();
        assert!(aggregate(&input).unwrap_err().contains("same policy IDs"));
    }

    #[test]
    fn no_policy_is_claimed_to_dominate_on_one_combined_scalar() {
        let report = aggregate(&fixture()).expect("fixture should aggregate");
        assert!(report.policy_results.iter().any(|p| p.pareto_nondominated));
        // Eligibility is a separate boolean; Pareto status is not safety approval.
        assert!(report.policy_results.iter().any(|p| p.pareto_nondominated && !p.passed_all_declared_service_floors));
    }
}
