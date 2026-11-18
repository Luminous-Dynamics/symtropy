//! Idealized fleet-size lower bound for recurring HAPS service rotations.
//!
//! The calculation uses a deterministic duty-cycle identity. It does not
//! prove a realizable rotation schedule and does not model failures, weather,
//! maintenance queues, launch/recovery capacity, or Semi-Markov repair times.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ReliabilityError;

const FLOAT_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetRotationCapacityCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub required_service_units: usize,
    pub scenarios: Vec<RotationCapacityScenario>,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationCapacityScenario {
    pub scenario_id: String,
    pub scenario_label: String,
    pub service_active_days: f64,
    pub turnaround_components_days: TurnaroundComponents,
    pub expected_oracle: ExpectedRotationCapacity,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnaroundComponents {
    pub recovery_and_decommission_days: f64,
    pub transport_to_depot_days: f64,
    pub inspection_and_disassembly_days: f64,
    pub repair_and_refurbishment_days: f64,
    pub replenishment_days: f64,
    pub launch_preparation_days: f64,
    pub transit_to_service_area_days: f64,
}

impl TurnaroundComponents {
    fn total_days(&self) -> f64 {
        self.recovery_and_decommission_days
            + self.transport_to_depot_days
            + self.inspection_and_disassembly_days
            + self.repair_and_refurbishment_days
            + self.replenishment_days
            + self.launch_preparation_days
            + self.transit_to_service_area_days
    }

    fn named_values(&self) -> [(&'static str, f64); 7] {
        [
            ("recovery_and_decommission_days", self.recovery_and_decommission_days),
            ("transport_to_depot_days", self.transport_to_depot_days),
            ("inspection_and_disassembly_days", self.inspection_and_disassembly_days),
            ("repair_and_refurbishment_days", self.repair_and_refurbishment_days),
            ("replenishment_days", self.replenishment_days),
            ("launch_preparation_days", self.launch_preparation_days),
            ("transit_to_service_area_days", self.transit_to_service_area_days),
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedRotationCapacity {
    pub total_turnaround_days: f64,
    pub cycle_days: f64,
    pub service_duty_fraction: f64,
    pub minimum_total_platforms_lower_bound: usize,
    pub minimum_reserve_platforms_lower_bound: usize,
    pub aggregate_active_service_capacity_units: f64,
    pub surplus_active_capacity_units: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RotationCapacityResult {
    pub scenario_id: String,
    pub service_active_days: f64,
    pub turnaround_days: f64,
    pub cycle_days: f64,
    pub service_duty_fraction: f64,
    pub minimum_total_platforms_lower_bound: usize,
    pub minimum_reserve_platforms_lower_bound: usize,
    pub aggregate_active_service_capacity_units: f64,
    pub surplus_active_capacity_units: f64,
    pub lower_bound_only: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetRotationCapacityReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub required_service_units: usize,
    pub scenarios: Vec<RotationCapacityResult>,
    pub interpretation: String,
}

pub fn parse_rotation_capacity_case_json(
    bytes: &[u8],
) -> Result<FleetRotationCapacityCase, ReliabilityError> {
    let case: FleetRotationCapacityCase = serde_json::from_slice(bytes)
        .map_err(|error| ReliabilityError::new(format!("invalid fleet rotation JSON: {error}")))?;
    validate_rotation_capacity_case(&case)?;
    Ok(case)
}

pub fn validate_rotation_capacity_case(
    case: &FleetRotationCapacityCase,
) -> Result<(), ReliabilityError> {
    if case.schema_version != "haps-fleet-rotation-capacity/v1" {
        return Err(ReliabilityError::new("unsupported fleet rotation schema_version"));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_rotation_capacity_lower_bound"
    {
        return Err(ReliabilityError::new(
            "fleet rotation lower-bound evaluator accepts synthetic cases only",
        ));
    }
    if case.required_service_units == 0 {
        return Err(ReliabilityError::new("required_service_units must be positive"));
    }
    if case.scenarios.is_empty() {
        return Err(ReliabilityError::new("at least one rotation scenario is required"));
    }

    let mut scenario_ids = HashSet::with_capacity(case.scenarios.len());
    for scenario in &case.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            return Err(ReliabilityError::new(
                "rotation scenario ID is empty or duplicated",
            ));
        }
        if scenario.scenario_label.trim().is_empty() {
            return Err(ReliabilityError::new(format!(
                "scenario {} has an empty label",
                scenario.scenario_id
            )));
        }
        validate_positive_finite(
            scenario.service_active_days,
            "service_active_days",
        )?;
        for (name, value) in scenario.turnaround_components_days.named_values() {
            validate_nonnegative_finite(value, name)?;
        }
        let turnaround = scenario.turnaround_components_days.total_days();
        let cycle = scenario.service_active_days + turnaround;
        if !turnaround.is_finite() || !cycle.is_finite() || cycle <= 0.0 {
            return Err(ReliabilityError::new(format!(
                "scenario {} has an invalid turnaround/cycle sum",
                scenario.scenario_id
            )));
        }
        if scenario.expected_oracle.minimum_total_platforms_lower_bound
            < case.required_service_units
        {
            return Err(ReliabilityError::new(format!(
                "scenario {} expected fleet lower bound is below required service units",
                scenario.scenario_id
            )));
        }
    }
    Ok(())
}

pub fn evaluate_rotation_capacity_case(
    case: &FleetRotationCapacityCase,
) -> Result<FleetRotationCapacityReport, ReliabilityError> {
    validate_rotation_capacity_case(case)?;
    let mut results = Vec::new();
    results
        .try_reserve_exact(case.scenarios.len())
        .map_err(|_| ReliabilityError::new("could not reserve rotation result memory"))?;

    for scenario in &case.scenarios {
        let turnaround_days = scenario.turnaround_components_days.total_days();
        let cycle_days = scenario.service_active_days + turnaround_days;
        let service_duty_fraction = scenario.service_active_days / cycle_days;
        if !service_duty_fraction.is_finite()
            || service_duty_fraction <= 0.0
            || service_duty_fraction > 1.0
        {
            return Err(ReliabilityError::new(format!(
                "scenario {} has an invalid service duty fraction",
                scenario.scenario_id
            )));
        }

        let required_capacity = case.required_service_units as f64;
        let fleet_exact = required_capacity / service_duty_fraction;
        if !fleet_exact.is_finite() || fleet_exact > usize::MAX as f64 {
            return Err(ReliabilityError::new(format!(
                "scenario {} fleet lower bound overflows",
                scenario.scenario_id
            )));
        }
        let minimum_total_platforms_lower_bound = fleet_exact.ceil() as usize;
        let minimum_reserve_platforms_lower_bound =
            minimum_total_platforms_lower_bound - case.required_service_units;
        let aggregate_active_service_capacity_units =
            minimum_total_platforms_lower_bound as f64 * service_duty_fraction;
        let surplus_active_capacity_units =
            aggregate_active_service_capacity_units - required_capacity;

        let expected = &scenario.expected_oracle;
        for (name, actual, expected_value) in [
            ("total_turnaround_days", turnaround_days, expected.total_turnaround_days),
            ("cycle_days", cycle_days, expected.cycle_days),
            ("service_duty_fraction", service_duty_fraction, expected.service_duty_fraction),
            (
                "aggregate_active_service_capacity_units",
                aggregate_active_service_capacity_units,
                expected.aggregate_active_service_capacity_units,
            ),
            (
                "surplus_active_capacity_units",
                surplus_active_capacity_units,
                expected.surplus_active_capacity_units,
            ),
        ] {
            if (actual - expected_value).abs() > FLOAT_TOLERANCE {
                return Err(ReliabilityError::new(format!(
                    "scenario {} expected oracle mismatch for {name}: calculated {actual}, fixture {}",
                    scenario.scenario_id, expected_value
                )));
            }
        }
        if minimum_total_platforms_lower_bound
            != expected.minimum_total_platforms_lower_bound
            || minimum_reserve_platforms_lower_bound
                != expected.minimum_reserve_platforms_lower_bound
        {
            return Err(ReliabilityError::new(format!(
                "scenario {} expected oracle fleet-count mismatch",
                scenario.scenario_id
            )));
        }

        results.push(RotationCapacityResult {
            scenario_id: scenario.scenario_id.clone(),
            service_active_days: scenario.service_active_days,
            turnaround_days,
            cycle_days,
            service_duty_fraction,
            minimum_total_platforms_lower_bound,
            minimum_reserve_platforms_lower_bound,
            aggregate_active_service_capacity_units,
            surplus_active_capacity_units,
            lower_bound_only: true,
        });
    }

    Ok(FleetRotationCapacityReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        required_service_units: case.required_service_units,
        scenarios: results,
        interpretation: "Idealized aggregate fleet-size lower bound only; separate scheduling, common-cause, stochastic failure/repair, launch/recovery, maintenance-resource and service-handover qualification is required.".to_owned(),
    })
}

fn validate_positive_finite(value: f64, name: &str) -> Result<(), ReliabilityError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ReliabilityError::new(format!(
            "{name} must be finite and positive"
        )));
    }
    Ok(())
}

fn validate_nonnegative_finite(value: f64, name: &str) -> Result<(), ReliabilityError> {
    if !value.is_finite() || value < 0.0 {
        return Err(ReliabilityError::new(format!(
            "{name} must be finite and nonnegative"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_rotation_capacity_v1.json"
    );

    fn parse() -> FleetRotationCapacityCase {
        parse_rotation_capacity_case_json(CASE.as_bytes())
            .expect("synthetic fleet rotation fixture is valid")
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn rotation_fixture_matches_all_expected_lower_bounds() {
        let report = evaluate_rotation_capacity_case(&parse()).unwrap();
        assert_eq!(report.scenarios.len(), 7);
        assert_eq!(report.required_service_units, 3);
        let expected = [
            ("service-30d-turnaround-14d", 5, 2, 14.0, 30.0 / 44.0),
            ("service-30d-turnaround-30d", 6, 3, 30.0, 0.5),
            ("service-30d-turnaround-60d", 9, 6, 60.0, 1.0 / 3.0),
            ("service-30d-turnaround-90d", 12, 9, 90.0, 0.25),
            ("service-30d-turnaround-180d", 21, 18, 180.0, 1.0 / 7.0),
            ("service-180d-turnaround-14d", 4, 1, 14.0, 180.0 / 194.0),
            ("service-180d-turnaround-90d", 5, 2, 90.0, 2.0 / 3.0),
        ];
        for (result, (id, fleet, reserve, turnaround, duty)) in
            report.scenarios.iter().zip(expected)
        {
            assert_eq!(result.scenario_id, id);
            assert_eq!(result.minimum_total_platforms_lower_bound, fleet);
            assert_eq!(result.minimum_reserve_platforms_lower_bound, reserve);
            close(result.turnaround_days, turnaround);
            close(result.service_duty_fraction, duty);
            assert!(result.lower_bound_only);
        }
    }

    #[test]
    fn longer_turnaround_never_reduces_lower_bound_for_fixed_service_interval() {
        let report = evaluate_rotation_capacity_case(&parse()).unwrap();
        let fixed_service = &report.scenarios[..5];
        for pair in fixed_service.windows(2) {
            assert!(
                pair[1].minimum_total_platforms_lower_bound
                    >= pair[0].minimum_total_platforms_lower_bound
            );
            assert!(pair[1].service_duty_fraction <= pair[0].service_duty_fraction);
        }
    }

    #[test]
    fn longer_service_interval_never_increases_lower_bound_for_fixed_turnaround() {
        let report = evaluate_rotation_capacity_case(&parse()).unwrap();
        assert!(
            report.scenarios[5].minimum_total_platforms_lower_bound
                < report.scenarios[0].minimum_total_platforms_lower_bound
        );
        assert!(
            report.scenarios[6].minimum_total_platforms_lower_bound
                < report.scenarios[3].minimum_total_platforms_lower_bound
        );
    }

    #[test]
    fn parser_rejects_zero_service_interval() {
        let mut case = parse();
        case.scenarios[0].service_active_days = 0.0;
        assert!(validate_rotation_capacity_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_negative_turnaround_component() {
        let mut case = parse();
        case.scenarios[0]
            .turnaround_components_days
            .repair_and_refurbishment_days = -1.0;
        assert!(validate_rotation_capacity_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_duplicate_scenario_ids() {
        let mut case = parse();
        case.scenarios[1].scenario_id = case.scenarios[0].scenario_id.clone();
        assert!(validate_rotation_capacity_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_non_synthetic_evidence_promotion() {
        let mut case = parse();
        case.evidence_class = "field_measurement".to_owned();
        case.claim_ceiling = "measured_service_availability".to_owned();
        assert!(validate_rotation_capacity_case(&case).is_err());
    }
}
