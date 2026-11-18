//! Analytic HAPS fleet service availability arithmetic.
//!
//! This crate models independent per-HAPS availability, a shared HAPS-only
//! common-mode event, and a second shared failure mode affecting terrestrial
//! and LEO fallback. It is not a Markov reliability model, event simulator,
//! fault-tree qualification, physical weather model, or measured SLA.

use serde::{Deserialize, Serialize};
use std::fmt;

const PROBABILITY_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailabilityError {
    Json(String),
    InvalidCase(String),
}

impl fmt::Display for AvailabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) | Self::InvalidCase(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AvailabilityError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub analysis_horizon_h: f64,
    pub inputs: AvailabilityInputs,
    pub maintenance_policy: MaintenancePolicy,
    pub scenarios: Vec<AvailabilityScenario>,
    pub source_evidence_refs: Option<Vec<String>>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityInputs {
    pub per_haps_independent_availability: f64,
    pub shared_haps_weather_outage_probability: f64,
    pub terrestrial_availability: f64,
    pub leo_backhaul_availability: f64,
    pub shared_external_common_mode_outage_probability: f64,
    pub independence_contract: IndependenceContract,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependenceContract {
    pub haps_platforms_independent_conditioned_on_no_common_mode: bool,
    pub terrestrial_independent_of_haps_weather: bool,
    pub availability_includes_planned_maintenance: bool,
    pub external_common_mode_independent_of_haps_weather: bool,
    pub terrestrial_and_leo_independent_conditioned_on_no_external_common_mode: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenancePolicy {
    pub policy: String,
    pub max_simultaneous_planned_maintenance: usize,
    pub maintenance_already_accounted_in_individual_availability: bool,
    pub maintenance_windows_staggered: bool,
    pub baseline_fleet_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityScenario {
    pub scenario_id: String,
    pub shared_weather_outage_probability: f64,
    pub shared_external_outage_probability: f64,
    pub haps_platform_counts: Vec<usize>,
    pub expected: Vec<ExpectedAvailability>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedAvailability {
    pub haps_count: usize,
    pub haps_only_availability: f64,
    pub haps_plus_terrestrial_availability: f64,
    pub haps_plus_terrestrial_plus_leo_availability: f64,
    pub haps_only_downtime_h_per_year: f64,
    pub cross_layer_downtime_h_per_year: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetAvailabilityResult {
    pub haps_count: usize,
    pub haps_only_availability: f64,
    pub haps_plus_terrestrial_availability: f64,
    pub haps_plus_terrestrial_plus_leo_availability: f64,
    pub haps_only_downtime_h_per_year: f64,
    pub cross_layer_downtime_h_per_year: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScenarioAvailabilityReport {
    pub scenario_id: String,
    pub shared_weather_outage_probability: f64,
    pub shared_external_outage_probability: f64,
    pub results: Vec<FleetAvailabilityResult>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AvailabilityReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub analysis_horizon_h: f64,
    pub scenarios: Vec<ScenarioAvailabilityReport>,
}

/// Parse and validate a versioned HAPS fleet availability case.
pub fn parse_case_json(bytes: &[u8]) -> Result<AvailabilityCase, AvailabilityError> {
    let case: AvailabilityCase = serde_json::from_slice(bytes)
        .map_err(|error| AvailabilityError::Json(error.to_string()))?;
    validate_case(&case)?;
    Ok(case)
}

/// Validate input domains and the assumptions required by the product model.
pub fn validate_case(case: &AvailabilityCase) -> Result<(), AvailabilityError> {
    if case.schema_version != "haps-fleet-availability-case/v1" {
        return Err(AvailabilityError::InvalidCase(
            "unsupported schema_version".to_owned(),
        ));
    }
    if case.case_id.trim().is_empty() || case.revision == 0 {
        return Err(AvailabilityError::InvalidCase(
            "case_id must be non-empty and revision must be positive".to_owned(),
        ));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_reliability_calculation"
    {
        return Err(AvailabilityError::InvalidCase(
            "this initial evaluator accepts synthetic reliability cases only".to_owned(),
        ));
    }
    if !case.analysis_horizon_h.is_finite() || case.analysis_horizon_h <= 0.0 {
        return Err(AvailabilityError::InvalidCase(
            "analysis_horizon_h must be finite and positive".to_owned(),
        ));
    }

    for (name, value) in [
        ("per_haps_independent_availability", case.inputs.per_haps_independent_availability),
        ("shared_haps_weather_outage_probability", case.inputs.shared_haps_weather_outage_probability),
        ("terrestrial_availability", case.inputs.terrestrial_availability),
        ("leo_backhaul_availability", case.inputs.leo_backhaul_availability),
        ("shared_external_common_mode_outage_probability", case.inputs.shared_external_common_mode_outage_probability),
    ] {
        validate_probability(name, value)?;
    }

    let contract = &case.inputs.independence_contract;
    if !contract.haps_platforms_independent_conditioned_on_no_common_mode
        || !contract.terrestrial_independent_of_haps_weather
        || !contract.availability_includes_planned_maintenance
        || !contract.external_common_mode_independent_of_haps_weather
        || !contract.terrestrial_and_leo_independent_conditioned_on_no_external_common_mode
    {
        return Err(AvailabilityError::InvalidCase(
            "declared independence/maintenance contract is not satisfied".to_owned(),
        ));
    }

    let maintenance = &case.maintenance_policy;
    if maintenance.baseline_fleet_size == 0
        || maintenance.max_simultaneous_planned_maintenance > maintenance.baseline_fleet_size
        || !maintenance.maintenance_already_accounted_in_individual_availability
    {
        return Err(AvailabilityError::InvalidCase(
            "invalid maintenance policy or planned maintenance would be double-counted".to_owned(),
        ));
    }
    if maintenance.policy == "staggered" && !maintenance.maintenance_windows_staggered {
        return Err(AvailabilityError::InvalidCase(
            "staggered maintenance policy requires staggered maintenance windows".to_owned(),
        ));
    }

    if case.scenarios.is_empty() {
        return Err(AvailabilityError::InvalidCase(
            "at least one availability scenario is required".to_owned(),
        ));
    }
    for scenario in &case.scenarios {
        if scenario.scenario_id.trim().is_empty() || scenario.haps_platform_counts.is_empty() {
            return Err(AvailabilityError::InvalidCase(
                "scenario_id and haps_platform_counts must be non-empty".to_owned(),
            ));
        }
        validate_probability(
            "scenario.shared_weather_outage_probability",
            scenario.shared_weather_outage_probability,
        )?;
        validate_probability(
            "scenario.shared_external_outage_probability",
            scenario.shared_external_outage_probability,
        )?;
        if scenario.expected.len() != scenario.haps_platform_counts.len() {
            return Err(AvailabilityError::InvalidCase(format!(
                "scenario {} has a different number of expected rows and fleet sizes",
                scenario.scenario_id
            )));
        }
        for (index, count) in scenario.haps_platform_counts.iter().enumerate() {
            if *count == 0 || scenario.expected[index].haps_count != *count {
                return Err(AvailabilityError::InvalidCase(format!(
                    "scenario {} has an invalid or mismatched HAPS platform count",
                    scenario.scenario_id
                )));
            }
        }
    }
    Ok(())
}

fn validate_probability(name: &str, value: f64) -> Result<(), AvailabilityError> {
    if !value.is_finite() || !(-PROBABILITY_TOLERANCE..=1.0 + PROBABILITY_TOLERANCE).contains(&value) {
        return Err(AvailabilityError::InvalidCase(format!(
            "{name} must be finite and within [0, 1]"
        )));
    }
    Ok(())
}

/// Evaluate one fleet size under the declared common-mode probabilities.
pub fn evaluate_fleet_size(
    case: &AvailabilityCase,
    haps_count: usize,
    shared_weather_outage_probability: f64,
    shared_external_outage_probability: f64,
) -> Result<FleetAvailabilityResult, AvailabilityError> {
    if haps_count == 0 {
        return Err(AvailabilityError::InvalidCase(
            "haps_count must be positive".to_owned(),
        ));
    }
    validate_probability("shared_weather_outage_probability", shared_weather_outage_probability)?;
    validate_probability("shared_external_outage_probability", shared_external_outage_probability)?;

    let p = case.inputs.per_haps_independent_availability;
    let g = case.inputs.terrestrial_availability;
    let s = case.inputs.leo_backhaul_availability;
    let q = shared_weather_outage_probability;
    let r = shared_external_outage_probability;

    // Conditional on no shared HAPS-only event, all n HAPS fail independently.
    let haps_unavailability = q + (1.0 - q) * (1.0 - p).powf(haps_count as f64);
    let haps_availability = 1.0 - haps_unavailability;

    // r disables both external fallback layers. Otherwise, ground and LEO fail
    // independently with probabilities (1-g) and (1-s).
    let ground_unavailability = r + (1.0 - r) * (1.0 - g);
    let ground_leo_unavailability =
        r + (1.0 - r) * (1.0 - g) * (1.0 - s);

    let haps_ground_availability =
        1.0 - haps_unavailability * ground_unavailability;
    let haps_ground_leo_availability =
        1.0 - haps_unavailability * ground_leo_unavailability;

    let annual_hours = 8760.0;
    Ok(FleetAvailabilityResult {
        haps_count,
        haps_only_availability: clamp_probability(haps_availability),
        haps_plus_terrestrial_availability: clamp_probability(haps_ground_availability),
        haps_plus_terrestrial_plus_leo_availability:
            clamp_probability(haps_ground_leo_availability),
        haps_only_downtime_h_per_year: haps_unavailability * annual_hours,
        cross_layer_downtime_h_per_year:
            haps_unavailability * ground_leo_unavailability * annual_hours,
    })
}

fn clamp_probability(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Evaluate every scenario and fleet-size row in a validated case.
pub fn evaluate_case(
    case: &AvailabilityCase,
) -> Result<AvailabilityReport, AvailabilityError> {
    validate_case(case)?;
    let mut scenario_reports = Vec::with_capacity(case.scenarios.len());
    for scenario in &case.scenarios {
        let mut results = Vec::with_capacity(scenario.haps_platform_counts.len());
        for count in &scenario.haps_platform_counts {
            results.push(evaluate_fleet_size(
                case,
                *count,
                scenario.shared_weather_outage_probability,
                scenario.shared_external_outage_probability,
            )?);
        }
        scenario_reports.push(ScenarioAvailabilityReport {
            scenario_id: scenario.scenario_id.clone(),
            shared_weather_outage_probability: scenario.shared_weather_outage_probability,
            shared_external_outage_probability: scenario.shared_external_outage_probability,
            results,
        });
    }
    Ok(AvailabilityReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        analysis_horizon_h: case.analysis_horizon_h,
        scenarios: scenario_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FLEET_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_availability_case_v1.json"
    );

    fn fixture() -> AvailabilityCase {
        parse_case_json(FLEET_CASE.as_bytes()).expect("fleet fixture parses and validates")
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "actual {actual:.15}, expected {expected:.15}"
        );
    }

    #[test]
    fn all_forty_five_fixture_rows_match_formula_oracle() {
        let case = fixture();
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.scenarios.len(), 9);
        let mut rows = 0;
        for (scenario, calculated) in case.scenarios.iter().zip(report.scenarios.iter()) {
            assert_eq!(scenario.scenario_id, calculated.scenario_id);
            for (expected, actual) in scenario.expected.iter().zip(calculated.results.iter()) {
                assert_eq!(expected.haps_count, actual.haps_count);
                close(expected.haps_only_availability, actual.haps_only_availability);
                close(
                    expected.haps_plus_terrestrial_availability,
                    actual.haps_plus_terrestrial_availability,
                );
                close(
                    expected.haps_plus_terrestrial_plus_leo_availability,
                    actual.haps_plus_terrestrial_plus_leo_availability,
                );
                close(
                    expected.haps_only_downtime_h_per_year,
                    actual.haps_only_downtime_h_per_year,
                );
                close(
                    expected.cross_layer_downtime_h_per_year,
                    actual.cross_layer_downtime_h_per_year,
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 45);
    }

    #[test]
    fn common_weather_limits_same_class_redundancy() {
        let case = fixture();
        let three = evaluate_fleet_size(&case, 3, 0.03, 0.0).unwrap();
        let ten = evaluate_fleet_size(&case, 10, 0.03, 0.0).unwrap();
        close(three.haps_only_availability, 0.969_992_24);
        close(ten.haps_only_availability, 0.97);
        assert!(ten.haps_only_availability - three.haps_only_availability < 0.000_01);
    }

    #[test]
    fn layered_fallback_beats_haps_only_under_declared_independence() {
        let case = fixture();
        let haps_only = evaluate_fleet_size(&case, 3, 0.03, 0.0).unwrap();
        assert!(
            haps_only.haps_plus_terrestrial_availability
                > haps_only.haps_only_availability
        );
        assert!(
            haps_only.haps_plus_terrestrial_plus_leo_availability
                > haps_only.haps_plus_terrestrial_availability
        );
        close(
            haps_only.cross_layer_downtime_h_per_year,
            0.013_143_398_879_589_263,
        );
    }

    #[test]
    fn common_external_failure_erodes_cross_layer_diversity() {
        let case = fixture();
        let independent = evaluate_fleet_size(&case, 3, 0.03, 0.0).unwrap();
        let shared_one_percent = evaluate_fleet_size(&case, 3, 0.03, 0.01).unwrap();
        assert!(
            shared_one_percent.haps_plus_terrestrial_plus_leo_availability
                < independent.haps_plus_terrestrial_plus_leo_availability
        );
        close(
            shared_one_percent.cross_layer_downtime_h_per_year,
            2.641_691_740_891_07,
        );
    }

    #[test]
    fn adding_platforms_never_reduces_haps_only_availability() {
        let case = fixture();
        let mut previous = 0.0;
        for count in 1..=10 {
            let result = evaluate_fleet_size(&case, count, 0.03, 0.0).unwrap();
            assert!(result.haps_only_availability >= previous);
            previous = result.haps_only_availability;
        }
    }

    #[test]
    fn invalid_common_mode_probability_is_rejected() {
        let case = fixture();
        assert!(evaluate_fleet_size(&case, 3, 1.1, 0.0).is_err());
        assert!(evaluate_fleet_size(&case, 3, 0.03, -0.1).is_err());
    }

    #[test]
    fn maintenance_must_not_be_double_counted() {
        let mut value: Value = serde_json::from_str(FLEET_CASE).unwrap();
        value["maintenance_policy"]["maintenance_already_accounted_in_individual_availability"] =
            Value::Bool(false);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(parse_case_json(&bytes).is_err());
    }

    #[test]
    fn invalid_probability_input_is_rejected() {
        let mut value: Value = serde_json::from_str(FLEET_CASE).unwrap();
        value["inputs"]["terrestrial_availability"] = Value::from(1.2);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(parse_case_json(&bytes).is_err());
    }
}
