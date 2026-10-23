use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreInput {
    pub schema_version: String,
    pub artifact_kind: String,
    pub evidence_class: String,
    pub status: String,
    pub episode: Episode,
    pub policy_runs: Vec<PolicyRun>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Episode {
    pub id: String,
    pub location_context: String,
    pub time_step_hours: f64,
    pub service_value_semantics: String,
    pub service_contracts: Vec<ServiceContract>,
    pub steps: Vec<StepDemand>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceContract {
    pub id: String,
    pub criticality_weight: f64,
    pub minimum_service_fraction: f64,
    pub unit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepDemand {
    pub demand: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRun {
    pub policy_id: String,
    pub method_family: String,
    pub fuel_unit: String,
    pub fuel_units: f64,
    pub steps: Vec<StepDelivery>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepDelivery {
    pub delivered: BTreeMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkReport {
    pub schema_version: &'static str,
    pub report_kind: &'static str,
    pub evidence_class: &'static str,
    pub episode_id: String,
    pub result_scope: &'static str,
    pub results: Vec<PolicyScore>,
}

#[derive(Debug, Serialize)]
pub struct ServiceScore {
    pub service_id: String,
    pub unit: String,
    pub demanded_unit_hours: f64,
    pub delivered_unit_hours: f64,
    pub service_fraction: Option<f64>,
    pub floor_breach_service_hours: f64,
}

#[derive(Debug, Serialize)]
pub struct PolicyScore {
    pub policy_id: String,
    pub method_family: String,
    pub fuel_unit: String,
    /// Weighted mean of per-service delivered/demanded unit-hours ratios.
    /// Units are normalized within each service before criticality weighting.
    pub weighted_service_fraction: f64,
    /// Dimensionless per-service delivery fractions integrated over time and weighted by criticality.
    pub weighted_delivered_service_fraction_hours: f64,
    /// Dimensionless per-service unmet fractions integrated over time and weighted by criticality.
    pub weighted_unserved_service_fraction_hours: f64,
    /// Sum of timestep hours for each service below its declared floor.
    pub floor_breach_service_hours: f64,
    pub gate_pass: bool,
    /// Fuel units divided by weighted delivered service-fraction-hours; comparable only under a shared fuel unit and service contract.
    pub fuel_units_per_weighted_delivered_service_fraction_hour: Option<f64>,
    /// Physical unit-hours stay separated by service; they are never summed across unlike units.
    pub services: Vec<ServiceScore>,
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn exact_keys<T>(
    values: &BTreeMap<String, T>,
    expected: &BTreeSet<String>,
    context: &str,
) -> Result<(), String> {
    if values.len() != expected.len() || values.keys().any(|key| !expected.contains(key)) {
        return Err(format!("{context}: keys must exactly match declared service IDs"));
    }
    Ok(())
}

pub fn score(input: &ScoreInput) -> Result<BenchmarkReport, String> {
    if input.schema_version != "0.1.0"
        || input.artifact_kind != "polar-resilience-score-input"
        || input.status != "synthetic_fixture"
        || input.evidence_class != "synthetic_only"
    {
        return Err(
            "only schema 0.1.0 synthetic_fixture inputs with synthetic_only evidence are accepted"
                .to_string(),
        );
    }

    let episode = &input.episode;
    if episode.id.trim().is_empty() || episode.location_context != "toy_not_site_data" {
        return Err("episode must be named and explicitly marked toy_not_site_data".to_string());
    }
    if !episode.time_step_hours.is_finite() || episode.time_step_hours <= 0.0 {
        return Err("time_step_hours must be finite and positive".to_string());
    }
    if episode.service_value_semantics != "rate_per_hour_integrated_to_unit_hours" {
        return Err("service_value_semantics must be rate_per_hour_integrated_to_unit_hours".to_string());
    }
    if episode.service_contracts.is_empty() || episode.steps.is_empty() {
        return Err("episode requires at least one service contract and time step".to_string());
    }

    let mut service_ids = BTreeSet::new();
    for service in &episode.service_contracts {
        if service.id.trim().is_empty() || !service_ids.insert(service.id.clone()) {
            return Err("service IDs must be non-empty and unique".to_string());
        }
        if !service.criticality_weight.is_finite() || service.criticality_weight <= 0.0 {
            return Err(format!("service {}: criticality weight must be finite and positive", service.id));
        }
        if !service.minimum_service_fraction.is_finite()
            || !(0.0..=1.0).contains(&service.minimum_service_fraction)
        {
            return Err(format!("service {}: minimum_service_fraction must be in [0, 1]", service.id));
        }
        if service.unit.trim().is_empty() {
            return Err(format!("service {}: unit must be explicit", service.id));
        }
    }

    // Keep physical quantities by service. Different units (e.g. heat and water)
    // must never be added together into one purportedly meaningful physical total.
    let mut demanded_by_service: BTreeMap<String, f64> =
        episode.service_contracts.iter().map(|s| (s.id.clone(), 0.0)).collect();
    for (step_index, step) in episode.steps.iter().enumerate() {
        exact_keys(&step.demand, &service_ids, &format!("demand step {step_index}"))?;
        for service in &episode.service_contracts {
            let demand = step.demand[&service.id];
            if !finite_nonnegative(demand) {
                return Err(format!("demand step {step_index}/{} must be finite and nonnegative", service.id));
            }
            let total = demanded_by_service.get_mut(&service.id).expect("service ID declared");
            *total += demand * episode.time_step_hours;
            if !total.is_finite() {
                return Err(format!("service {} demand total is non-finite", service.id));
            }
        }
    }
    let active_services: Vec<&ServiceContract> = episode.service_contracts.iter()
        .filter(|s| demanded_by_service[&s.id] > 0.0)
        .collect();
    if active_services.is_empty() {
        return Err("at least one service must have positive total demand".to_string());
    }
    let total_active_weight: f64 = active_services.iter().map(|s| s.criticality_weight).sum();
    if !total_active_weight.is_finite() || total_active_weight <= 0.0 {
        return Err("total criticality weight must be finite and positive".to_string());
    }

    if input.policy_runs.is_empty() {
        return Err("at least one policy run is required".to_string());
    }

    let mut policy_ids = BTreeSet::new();
    let mut shared_fuel_unit: Option<String> = None;
    let mut results = Vec::with_capacity(input.policy_runs.len());
    for policy in &input.policy_runs {
        if policy.policy_id.trim().is_empty() || !policy_ids.insert(policy.policy_id.clone()) {
            return Err("policy IDs must be non-empty and unique".to_string());
        }
        if policy.method_family.trim().is_empty() {
            return Err(format!("policy {}: method_family must be explicit", policy.policy_id));
        }
        if policy.fuel_unit.trim().is_empty() {
            return Err(format!("policy {}: fuel_unit must be explicit", policy.policy_id));
        }
        if let Some(expected) = &shared_fuel_unit {
            if expected != &policy.fuel_unit {
                return Err("all policies in an episode must declare the same fuel_unit".to_string());
            }
        } else {
            shared_fuel_unit = Some(policy.fuel_unit.clone());
        }
        if !finite_nonnegative(policy.fuel_units) {
            return Err(format!("policy {}: fuel_units must be finite and nonnegative", policy.policy_id));
        }
        if policy.steps.len() != episode.steps.len() {
            return Err(format!(
                "policy {}: {} delivery steps do not match {} demand steps",
                policy.policy_id,
                policy.steps.len(),
                episode.steps.len()
            ));
        }

        let mut delivered_by_service: BTreeMap<String, f64> =
            episode.service_contracts.iter().map(|s| (s.id.clone(), 0.0)).collect();
        let mut weighted_delivered_fraction_hours = 0.0;
        let mut weighted_unserved_fraction_hours = 0.0;
        let mut floor_breach_hours = 0.0;

        for (step_index, (demand_step, delivery_step)) in
            episode.steps.iter().zip(&policy.steps).enumerate()
        {
            exact_keys(
                &delivery_step.delivered,
                &service_ids,
                &format!("policy {} delivery step {step_index}", policy.policy_id),
            )?;

            for service in &episode.service_contracts {
                let demand = demand_step.demand[&service.id];
                let delivered_raw = delivery_step.delivered[&service.id];
                if !finite_nonnegative(delivered_raw) {
                    return Err(format!(
                        "policy {}/step {step_index}/{} must be finite and nonnegative",
                        policy.policy_id, service.id
                    ));
                }

                // Cap delivery at demand so over-delivery cannot inflate a score.
                let delivered = delivered_raw.min(demand);
                let unit_hours = delivered * episode.time_step_hours;
                let total = delivered_by_service.get_mut(&service.id).expect("service ID declared");
                *total += unit_hours;
                if !total.is_finite() {
                    return Err(format!("policy {}/{} delivered total is non-finite", policy.policy_id, service.id));
                }

                if demand > 0.0 {
                    let fraction = delivered / demand;
                    let weight_hours = service.criticality_weight * episode.time_step_hours;
                    weighted_delivered_fraction_hours += fraction * weight_hours;
                    weighted_unserved_fraction_hours += (1.0 - fraction) * weight_hours;

                    if fraction < service.minimum_service_fraction {
                        floor_breach_hours += episode.time_step_hours;
                    }
                }
            }
        }

        let mut weighted_service_fraction = 0.0;
        let mut service_results = Vec::with_capacity(episode.service_contracts.len());
        for service in &episode.service_contracts {
            let demanded = demanded_by_service[&service.id];
            let delivered = delivered_by_service[&service.id];
            let fraction = if demanded > 0.0 {
                let ratio = (delivered / demanded).clamp(0.0, 1.0);
                weighted_service_fraction += service.criticality_weight * ratio;
                Some(ratio)
            } else {
                None
            };
            service_results.push(ServiceScore {
                service_id: service.id.clone(),
                unit: service.unit.clone(),
                demanded_unit_hours: demanded,
                delivered_unit_hours: delivered,
                service_fraction: fraction,
                floor_breach_service_hours: episode.steps.iter().zip(&policy.steps).fold(0.0, |sum, (demand_step, delivery_step)| {
                    let demand = demand_step.demand[&service.id];
                    let delivered = delivery_step.delivered[&service.id].min(demand);
                    if demand > 0.0 && delivered / demand < service.minimum_service_fraction {
                        sum + episode.time_step_hours
                    } else {
                        sum
                    }
                }),
            });
        }
        weighted_service_fraction /= total_active_weight;

        for value in [
            weighted_service_fraction,
            weighted_delivered_fraction_hours,
            weighted_unserved_fraction_hours,
            floor_breach_hours,
        ] {
            if !value.is_finite() {
                return Err(format!("policy {} produced non-finite metrics", policy.policy_id));
            }
        }

        results.push(PolicyScore {
            policy_id: policy.policy_id.clone(),
            method_family: policy.method_family.clone(),
            fuel_unit: policy.fuel_unit.clone(),
            weighted_service_fraction,
            weighted_delivered_service_fraction_hours: weighted_delivered_fraction_hours,
            weighted_unserved_service_fraction_hours: weighted_unserved_fraction_hours,
            floor_breach_service_hours: floor_breach_hours,
            gate_pass: floor_breach_hours == 0.0,
            fuel_units_per_weighted_delivered_service_fraction_hour: if weighted_delivered_fraction_hours > 0.0 {
                Some(policy.fuel_units / weighted_delivered_fraction_hours)
            } else {
                None
            },
            services: service_results,
        });
    }

    Ok(BenchmarkReport {
        schema_version: "0.1.0",
        report_kind: "polar-resilience-score-report",
        evidence_class: "synthetic_only",
        episode_id: episode.id.clone(),
        result_scope: "metric_calculation_only_not_physical_validation",
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ScoreInput {
        serde_json::from_str(include_str!("../fixtures/toy-episode.json"))
            .expect("checked-in fixture must be valid JSON")
    }

    #[test]
    fn scores_synthetic_fixture_without_lifting_claim_ceiling() {
        let report = score(&fixture()).expect("fixture should score");
        assert_eq!(report.evidence_class, "synthetic_only");
        assert_eq!(report.result_scope, "metric_calculation_only_not_physical_validation");
        assert_eq!(report.results.len(), 4);
        assert!(report.results.iter().all(|r| r.fuel_unit == "normalized_fuel_units"));

        let candidate = report.results.iter()
            .find(|r| r.policy_id == "candidate_full_service")
            .expect("candidate trace should exist");
        assert!(candidate.gate_pass);
        assert!((candidate.weighted_service_fraction - 1.0).abs() < 1e-12);
        assert_eq!(candidate.floor_breach_service_hours, 0.0);
        assert_eq!(candidate.services.len(), 3);
    }

    #[test]
    fn low_fuel_does_not_compensate_for_a_service_floor_breach() {
        let report = score(&fixture()).expect("fixture should score");
        let defective = report.results.iter()
            .find(|r| r.policy_id == "defective_low_fuel_control")
            .expect("negative control should exist");
        let candidate = report.results.iter()
            .find(|r| r.policy_id == "candidate_full_service")
            .expect("candidate trace should exist");

        assert!(!defective.gate_pass);
        assert_eq!(defective.floor_breach_service_hours, 1.0);
        assert!(
            defective.fuel_units_per_weighted_delivered_service_fraction_hour.unwrap()
                < candidate.fuel_units_per_weighted_delivered_service_fraction_hour.unwrap()
        );
    }

    #[test]
    fn physically_different_service_units_are_not_summed() {
        let mut input = fixture();
        input.episode.service_contracts[0].unit = "kW".to_string();
        input.episode.service_contracts[1].unit = "litres_per_hour".to_string();
        input.episode.service_contracts[2].unit = "connected-service-count".to_string();
        let report = score(&input).expect("heterogeneous service units should be supported");
        let candidate = report.results.iter().find(|r| r.policy_id == "candidate_full_service").unwrap();
        assert!((candidate.weighted_service_fraction - 1.0).abs() < 1e-12);
        assert!(candidate.services.iter().any(|s| s.unit == "kW" && s.delivered_unit_hours > 0.0));
        assert!(candidate.services.iter().any(|s| s.unit == "litres_per_hour" && s.delivered_unit_hours > 0.0));
    }

    #[test]
    fn inactive_service_is_not_counted_as_a_zero_fraction() {
        let mut input = fixture();
        for step in &mut input.episode.steps {
            step.demand.insert("communications".to_string(), 0.0);
        }
        for policy in &mut input.policy_runs {
            for step in &mut policy.steps {
                step.delivered.insert("communications".to_string(), 0.0);
            }
        }
        let report = score(&input).expect("inactive service is explicitly excluded");
        let candidate = report.results.iter().find(|r| r.policy_id == "candidate_full_service").unwrap();
        assert!((candidate.weighted_service_fraction - 1.0).abs() < 1e-12);
        assert!(candidate.services.iter().find(|s| s.service_id == "communications").unwrap().service_fraction.is_none());
    }

    #[test]
    fn incumbent_and_rule_based_traces_preserve_breach_counts() {
        let report = score(&fixture()).expect("fixture should score");
        let incumbent = report.results.iter().find(|r| r.policy_id == "incumbent_reference").unwrap();
        let rules = report.results.iter().find(|r| r.policy_id == "transparent_rules").unwrap();
        assert_eq!(incumbent.floor_breach_service_hours, 2.0);
        assert_eq!(rules.floor_breach_service_hours, 1.0);
        assert!(rules.weighted_service_fraction > incumbent.weighted_service_fraction);
    }

    #[test]
    fn rejects_mixed_fuel_units_between_compared_policies() {
        let mut input = fixture();
        input.policy_runs[1].fuel_unit = "litres".to_string();
        assert!(score(&input).unwrap_err().contains("same fuel_unit"));
    }

    #[test]
    fn rejects_unknown_service_value_semantics() {
        let mut input = fixture();
        input.episode.service_value_semantics = "unspecified".to_string();
        assert!(score(&input).unwrap_err().contains("service_value_semantics"));
    }

    #[test]
    fn rejects_non_synthetic_evidence_claims() {
        let mut input = fixture();
        input.evidence_class = "domain_validated".to_string();
        assert!(score(&input).unwrap_err().contains("synthetic_only"));
    }

    #[test]
    fn rejects_missing_service_fields() {
        let mut input = fixture();
        input.episode.steps[0].demand.remove("water");
        assert!(score(&input).unwrap_err().contains("keys must exactly match"));
    }

    #[test]
    fn rejects_delivery_series_length_mismatch() {
        let mut input = fixture();
        input.policy_runs[0].steps.pop();
        assert!(score(&input).unwrap_err().contains("do not match"));
    }

    #[test]
    fn rejects_negative_measurements() {
        let mut input = fixture();
        input.episode.steps[0].demand.insert("heat".to_string(), -1.0);
        assert!(score(&input).unwrap_err().contains("finite and nonnegative"));
    }

    #[test]
    fn caps_overdelivery_at_required_demand() {
        let mut input = fixture();
        for step in &mut input.policy_runs[0].steps {
            for delivered in step.delivered.values_mut() {
                *delivered = 1000.0;
            }
        }
        let report = score(&input).expect("over-delivery is valid but cannot inflate the metric");
        let result = &report.results[0];
        assert!((result.weighted_service_fraction - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_duplicate_service_and_policy_ids() {
        let mut duplicate_service = fixture();
        duplicate_service.episode.service_contracts[1].id = "heat".to_string();
        assert!(score(&duplicate_service).unwrap_err().contains("service IDs"));
        let mut duplicate_policy = fixture();
        duplicate_policy.policy_runs[1].policy_id = "incumbent_reference".to_string();
        assert!(score(&duplicate_policy).unwrap_err().contains("policy IDs"));
    }

    #[test]
    fn rejects_invalid_fraction_and_zero_total_demand() {
        let mut invalid_floor = fixture();
        invalid_floor.episode.service_contracts[0].minimum_service_fraction = 1.1;
        assert!(score(&invalid_floor).unwrap_err().contains("must be in [0, 1]"));
        let mut zero_demand = fixture();
        for step in &mut zero_demand.episode.steps {
            for demand in step.demand.values_mut() {
                *demand = 0.0;
            }
        }
        assert!(score(&zero_demand).unwrap_err().contains("at least one service"));
    }
}
