//! Exact static availability enumeration for HAPS fleets.
//!
//! This tool deliberately models independent Bernoulli node states,
//! independent common-cause events, and independent shared dependencies.
//! It is not a semi-Markov, maintenance, flight, weather-forecast, or
//! operational service model.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Maximum number of binary variables accepted by exact enumeration.
/// 20 bits caps the evaluator at 1,048,576 scenarios.
pub const MAX_ENUMERATION_BITS: usize = 20;
const PROBABILITY_MASS_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliabilityError(String);

impl ReliabilityError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ReliabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ReliabilityError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReliabilityCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub analysis_method: String,
    pub time_window_s: f64,
    pub nodes: Vec<FleetNode>,
    pub common_cause_events: Vec<CommonCauseEvent>,
    pub shared_dependencies: Vec<SharedDependency>,
    pub service_contract: FleetServiceContract,
    pub expected_oracle: ExpectedOracle,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetNode {
    pub node_id: String,
    pub platform_class: String,
    pub independent_availability_probability: f64,
    pub service_region_ref: String,
    #[serde(default)]
    pub maintenance_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonCauseEvent {
    pub event_id: String,
    pub event_class: String,
    pub event_probability: f64,
    pub affected_node_ids: Vec<String>,
    #[serde(default)]
    pub source_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedDependency {
    pub dependency_id: String,
    pub dependency_class: String,
    pub availability_probability: f64,
    pub dependency_scope: String,
    #[serde(default)]
    pub source_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServiceContract {
    pub minimum_operational_node_count: usize,
    pub required_shared_dependency_ids: Vec<String>,
    pub single_node_reference_id: String,
    pub service_region_ref: String,
    #[serde(default)]
    pub target_service_availability_fraction: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOracle {
    pub oracle_status: String,
    pub enumerated_scenario_count: usize,
    pub single_node_service_availability: f64,
    pub fleet_k_of_n_service_availability: f64,
    pub naive_independent_k_of_n_service_availability: f64,
    pub absolute_service_availability_gain_vs_single_node: f64,
    pub common_cause_overstatement_probability_points: f64,
    pub enumeration_verified: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetReliabilityReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub analysis_method: String,
    pub enumerated_scenario_count: usize,
    pub probability_mass: f64,
    pub probability_mass_error: f64,
    pub single_node_service_availability: f64,
    pub fleet_k_of_n_service_availability: f64,
    pub naive_independent_k_of_n_service_availability: f64,
    pub absolute_service_availability_gain_vs_single_node: f64,
    pub common_cause_overstatement_probability_points: f64,
    pub target_service_availability_fraction: Option<f64>,
    pub fleet_meets_target: Option<bool>,
}

pub fn parse_case_json(bytes: &[u8]) -> Result<FleetReliabilityCase, ReliabilityError> {
    let case: FleetReliabilityCase = serde_json::from_slice(bytes)
        .map_err(|error| ReliabilityError::new(format!("invalid fleet case JSON: {error}")))?;
    validate_case(&case)?;
    Ok(case)
}

pub fn validate_case(case: &FleetReliabilityCase) -> Result<(), ReliabilityError> {
    if case.schema_version != "haps-fleet-reliability-case/v1" {
        return Err(ReliabilityError::new(
            "schema_version must be haps-fleet-reliability-case/v1",
        ));
    }
    if case.case_id.trim().is_empty() || case.revision == 0 {
        return Err(ReliabilityError::new(
            "case_id must be non-empty and revision must be positive",
        ));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_static_availability_oracle"
    {
        return Err(ReliabilityError::new(
            "this exact enumerator accepts only synthetic static-availability cases",
        ));
    }
    if case.analysis_method != "static_bernoulli_exact_enumeration" {
        return Err(ReliabilityError::new(
            "analysis_method must be static_bernoulli_exact_enumeration; temporal/semi-Markov cases are not evaluated by this tool",
        ));
    }
    if !case.time_window_s.is_finite() || case.time_window_s <= 0.0 {
        return Err(ReliabilityError::new(
            "time_window_s must be finite and positive",
        ));
    }
    if case.nodes.is_empty() {
        return Err(ReliabilityError::new("at least one fleet node is required"));
    }

    let mut node_ids = HashSet::with_capacity(case.nodes.len());
    for node in &case.nodes {
        if node.node_id.trim().is_empty() || !node_ids.insert(node.node_id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "node_id is empty or duplicated: {}",
                node.node_id
            )));
        }
        if node.service_region_ref.trim().is_empty() {
            return Err(ReliabilityError::new(format!(
                "node {} has an empty service_region_ref",
                node.node_id
            )));
        }
        if node.service_region_ref != case.service_contract.service_region_ref {
            return Err(ReliabilityError::new(format!(
                "node {} does not declare the fleet service region",
                node.node_id
            )));
        }
        if node.maintenance_profile_ref.is_some() {
            return Err(ReliabilityError::new(
                "maintenance_profile_ref is not consumed by the static Bernoulli evaluator; use the temporal reliability owner",
            ));
        }
        if !matches!(
            node.platform_class.as_str(),
            "solar_fixed_wing_haps"
                | "buoyant_free_balloon"
                | "buoyant_airship"
                | "unknown"
        ) {
            return Err(ReliabilityError::new(format!(
                "node {} has an unrecognized platform_class",
                node.node_id
            )));
        }
        validate_probability(
            node.independent_availability_probability,
            &format!("node {} independent_availability_probability", node.node_id),
        )?;
    }

    let mut event_ids = HashSet::with_capacity(case.common_cause_events.len());
    for event in &case.common_cause_events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "common-cause event_id is empty or duplicated: {}",
                event.event_id
            )));
        }
        if event.affected_node_ids.is_empty() {
            return Err(ReliabilityError::new(format!(
                "common-cause event {} affects no nodes",
                event.event_id
            )));
        }
        if !matches!(
            event.event_class.as_str(),
            "weather_grounding"
                | "regional_airspace_closure"
                | "shared_launch_site_loss"
                | "common_configuration_defect"
                | "shared_maintenance_supply_failure"
                | "other"
        ) {
            return Err(ReliabilityError::new(format!(
                "common-cause event {} has an unrecognized event_class",
                event.event_id
            )));
        }
        validate_probability(
            event.event_probability,
            &format!("common-cause event {} probability", event.event_id),
        )?;
        let mut affected_ids = HashSet::with_capacity(event.affected_node_ids.len());
        for node_id in &event.affected_node_ids {
            if !node_ids.contains(node_id.as_str()) {
                return Err(ReliabilityError::new(format!(
                    "common-cause event {} references unknown node {}",
                    event.event_id, node_id
                )));
            }
            if !affected_ids.insert(node_id.as_str()) {
                return Err(ReliabilityError::new(format!(
                    "common-cause event {} repeats affected node {}",
                    event.event_id, node_id
                )));
            }
        }
    }

    let mut dependency_ids = HashSet::with_capacity(case.shared_dependencies.len());
    for dependency in &case.shared_dependencies {
        if dependency.dependency_id.trim().is_empty()
            || !dependency_ids.insert(dependency.dependency_id.as_str())
        {
            return Err(ReliabilityError::new(format!(
                "dependency_id is empty or duplicated: {}",
                dependency.dependency_id
            )));
        }
        if !matches!(
            dependency.dependency_class.as_str(),
            "terrestrial_gateway"
                | "satellite_backhaul"
                | "fso_backhaul"
                | "spectrum_authority"
                | "ground_control_site"
                | "launch_recovery_site"
                | "maintenance_supply"
                | "other"
        ) {
            return Err(ReliabilityError::new(format!(
                "dependency {} has an unrecognized dependency_class",
                dependency.dependency_id
            )));
        }
        validate_probability(
            dependency.availability_probability,
            &format!("dependency {} availability_probability", dependency.dependency_id),
        )?;
        if !matches!(
            dependency.dependency_scope.as_str(),
            "fleet_wide" | "regional" | "single_node"
        ) {
            return Err(ReliabilityError::new(format!(
                "dependency {} has an unrecognized dependency_scope",
                dependency.dependency_id
            )));
        }
    }

    let contract = &case.service_contract;
    if contract.minimum_operational_node_count == 0
        || contract.minimum_operational_node_count > case.nodes.len()
    {
        return Err(ReliabilityError::new(
            "minimum_operational_node_count must be in 1..=nodes.len()",
        ));
    }
    if !node_ids.contains(contract.single_node_reference_id.as_str()) {
        return Err(ReliabilityError::new(
            "single_node_reference_id does not identify a declared node",
        ));
    }
    if contract.service_region_ref.trim().is_empty() {
        return Err(ReliabilityError::new(
            "service_contract.service_region_ref must be non-empty",
        ));
    }
    if let Some(target) = contract.target_service_availability_fraction {
        validate_probability(target, "target_service_availability_fraction")?;
    }
    let mut required_dependencies = HashSet::with_capacity(
        contract.required_shared_dependency_ids.len(),
    );
    for dependency_id in &contract.required_shared_dependency_ids {
        if !dependency_ids.contains(dependency_id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "required shared dependency {} is undeclared",
                dependency_id
            )));
        }
        if !required_dependencies.insert(dependency_id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "required shared dependency {} is duplicated",
                dependency_id
            )));
        }
    }

    let random_bits = case
        .nodes
        .len()
        .checked_add(case.common_cause_events.len())
        .and_then(|value| value.checked_add(case.shared_dependencies.len()))
        .ok_or_else(|| ReliabilityError::new("random-variable count overflow"))?;
    if random_bits > MAX_ENUMERATION_BITS {
        return Err(ReliabilityError::new(format!(
            "exact enumeration requires {random_bits} binary variables; limit is {MAX_ENUMERATION_BITS}"
        )));
    }
    Ok(())
}

fn validate_probability(value: f64, label: &str) -> Result<(), ReliabilityError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(ReliabilityError::new(format!(
            "{label} must be finite and lie in [0, 1]"
        )));
    }
    Ok(())
}

pub fn evaluate_case(
    case: &FleetReliabilityCase,
) -> Result<FleetReliabilityReport, ReliabilityError> {
    validate_case(case)?;
    let node_count = case.nodes.len();
    let event_count = case.common_cause_events.len();
    let dependency_count = case.shared_dependencies.len();
    let random_bits = node_count + event_count + dependency_count;
    let scenario_count = 1usize
        .checked_shl(random_bits as u32)
        .ok_or_else(|| ReliabilityError::new("scenario count overflow"))?;

    let required_dependency_indices: Vec<usize> = case
        .service_contract
        .required_shared_dependency_ids
        .iter()
        .map(|dependency_id| {
            case.shared_dependencies
                .iter()
                .position(|dependency| dependency.dependency_id == *dependency_id)
                .expect("validate_case checked required dependency references")
        })
        .collect();
    let single_node_index = case
        .nodes
        .iter()
        .position(|node| node.node_id == case.service_contract.single_node_reference_id)
        .expect("validate_case checked single node reference");

    let mut probability_mass = 0.0;
    let mut single_node_availability = 0.0;
    let mut fleet_availability = 0.0;

    for mask in 0..scenario_count {
        let mut bit_index = 0usize;
        let mut scenario_probability = 1.0;
        let mut independent_node_up = vec![false; node_count];

        for (index, node) in case.nodes.iter().enumerate() {
            let is_up = mask & (1usize << bit_index) != 0;
            bit_index += 1;
            independent_node_up[index] = is_up;
            scenario_probability *= if is_up {
                node.independent_availability_probability
            } else {
                1.0 - node.independent_availability_probability
            };
        }

        let mut common_cause_node_down = vec![false; node_count];
        for event in &case.common_cause_events {
            let event_active = mask & (1usize << bit_index) != 0;
            bit_index += 1;
            scenario_probability *= if event_active {
                event.event_probability
            } else {
                1.0 - event.event_probability
            };
            if event_active {
                for node_id in &event.affected_node_ids {
                    let node_index = case
                        .nodes
                        .iter()
                        .position(|node| node.node_id == *node_id)
                        .expect("validate_case checked common-cause node references");
                    common_cause_node_down[node_index] = true;
                }
            }
        }

        let mut required_dependencies_up = true;
        for (index, dependency) in case.shared_dependencies.iter().enumerate() {
            let is_up = mask & (1usize << bit_index) != 0;
            bit_index += 1;
            scenario_probability *= if is_up {
                dependency.availability_probability
            } else {
                1.0 - dependency.availability_probability
            };
            if required_dependency_indices.contains(&index) && !is_up {
                required_dependencies_up = false;
            }
        }

        let operational_count = (0..node_count)
            .filter(|index| {
                independent_node_up[*index] && !common_cause_node_down[*index]
            })
            .count();
        let reference_node_up =
            independent_node_up[single_node_index] && !common_cause_node_down[single_node_index];

        probability_mass += scenario_probability;
        if reference_node_up && required_dependencies_up {
            single_node_availability += scenario_probability;
        }
        if operational_count >= case.service_contract.minimum_operational_node_count
            && required_dependencies_up
        {
            fleet_availability += scenario_probability;
        }
    }

    if !probability_mass.is_finite()
        || (probability_mass - 1.0).abs() > PROBABILITY_MASS_TOLERANCE
    {
        return Err(ReliabilityError::new(format!(
            "enumerated scenario probability mass is {}, expected 1.0",
            probability_mass
        )));
    }

    let naive_availability =
        naive_independent_k_out_of_n_availability(case, &required_dependency_indices);
    let probability_mass_error = (probability_mass - 1.0).abs();

    Ok(FleetReliabilityReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        analysis_method: case.analysis_method.clone(),
        enumerated_scenario_count: scenario_count,
        probability_mass,
        probability_mass_error,
        single_node_service_availability: single_node_availability,
        fleet_k_of_n_service_availability: fleet_availability,
        naive_independent_k_of_n_service_availability: naive_availability,
        absolute_service_availability_gain_vs_single_node: fleet_availability
            - single_node_availability,
        common_cause_overstatement_probability_points: naive_availability - fleet_availability,
        target_service_availability_fraction: case.service_contract.target_service_availability_fraction,
        fleet_meets_target: case
            .service_contract
            .target_service_availability_fraction
            .map(|target| fleet_availability >= target),
    })
}

fn naive_independent_k_out_of_n_availability(
    case: &FleetReliabilityCase,
    required_dependency_indices: &[usize],
) -> f64 {
    let mut distribution = vec![1.0];
    for node in &case.nodes {
        let mut next = vec![0.0; distribution.len() + 1];
        for (operational_count, probability) in distribution.iter().enumerate() {
            next[operational_count] +=
                probability * (1.0 - node.independent_availability_probability);
            next[operational_count + 1] +=
                probability * node.independent_availability_probability;
        }
        distribution = next;
    }

    let k = case.service_contract.minimum_operational_node_count;
    let k_out_of_n = distribution.iter().skip(k).sum::<f64>();
    let dependency_probability = required_dependency_indices
        .iter()
        .map(|index| case.shared_dependencies[*index].availability_probability)
        .product::<f64>();
    k_out_of_n * dependency_probability
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMON_CAUSE_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_common_cause_reliability_v1.json"
    );
    const DIVERSE_WEATHER_CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_diverse_weather_reliability_v1.json"
    );

    fn parse_fixture(value: &str) -> FleetReliabilityCase {
        parse_case_json(value.as_bytes()).expect("fixture parses and validates")
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-10,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn fully_correlated_weather_case_matches_oracle() {
        let case = parse_fixture(COMMON_CAUSE_CASE);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.enumerated_scenario_count, 32);
        close(report.probability_mass, 1.0);
        close(report.single_node_service_availability, 0.838856);
        close(report.fleet_k_of_n_service_availability, 0.8952271232);
        close(report.naive_independent_k_of_n_service_availability, 0.95236928);
        close(
            report.common_cause_overstatement_probability_points,
            0.0571421568,
        );
        assert_eq!(report.fleet_meets_target, Some(false));
    }

    #[test]
    fn weather_diversity_case_matches_oracle() {
        let case = parse_fixture(DIVERSE_WEATHER_CASE);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.enumerated_scenario_count, 32);
        close(report.probability_mass, 1.0);
        close(report.single_node_service_availability, 0.838856);
        close(report.fleet_k_of_n_service_availability, 0.9444876032);
        close(report.naive_independent_k_of_n_service_availability, 0.95236928);
        close(
            report.common_cause_overstatement_probability_points,
            0.0078816768,
        );
        assert_eq!(report.fleet_meets_target, Some(false));
    }

    #[test]
    fn diverse_weather_mapping_improves_service_availability() {
        let common = evaluate_case(&parse_fixture(COMMON_CAUSE_CASE)).unwrap();
        let diverse = evaluate_case(&parse_fixture(DIVERSE_WEATHER_CASE)).unwrap();
        close(
            diverse.fleet_k_of_n_service_availability
                - common.fleet_k_of_n_service_availability,
            0.04926048,
        );
    }

    #[test]
    fn probability_mass_is_conserved() {
        let report = evaluate_case(&parse_fixture(COMMON_CAUSE_CASE)).unwrap();
        close(report.probability_mass, 1.0);
        close(report.probability_mass_error, 0.0);
    }

    #[test]
    fn parser_rejects_unknown_common_cause_node() {
        let mut case = parse_fixture(COMMON_CAUSE_CASE);
        case.common_cause_events[0].affected_node_ids.push("haps-unknown".to_owned());
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_probability_outside_unit_interval() {
        let mut case = parse_fixture(COMMON_CAUSE_CASE);
        case.nodes[0].independent_availability_probability = 1.01;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn temporal_method_is_not_silently_treated_as_static() {
        let mut case = parse_fixture(COMMON_CAUSE_CASE);
        case.analysis_method = "semi_markov_time_dependent".to_owned();
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn enumeration_limit_rejects_excessive_scenario_spaces() {
        let mut case = parse_fixture(COMMON_CAUSE_CASE);
        let original = case.nodes.clone();
        for index in 0..18 {
            let mut node = original[index % original.len()].clone();
            node.node_id = format!("extra-node-{index}");
            case.nodes.push(node);
        }
        case.common_cause_events[0]
            .affected_node_ids
            .truncate(1);
        case.service_contract.minimum_operational_node_count = 1;
        assert!(matches!(
            validate_case(&case),
            Err(error) if error.to_string().contains("limit is")
        ));
    }
}
