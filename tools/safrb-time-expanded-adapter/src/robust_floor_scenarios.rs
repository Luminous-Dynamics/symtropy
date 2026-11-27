#![forbid(unsafe_code)]

//! Joint beneficiary-floor assurance across a supplied finite scenario set.
//!
//! Each complete scenario is evaluated independently using the exact
//! lower-bound circulation proof in beneficiary_floors. A set is guaranteed
//! only if every supplied scenario admits the same floors; possible if at
//! least one does; and infeasible within the supplied set if none does.
//!
//! This module does not authenticate scenario-set completeness, authority,
//! probabilities, common-cause relationships or physical observations.

use crate::beneficiary_floors::{
    BeneficiaryFloor, BeneficiaryFloorReport, evaluate_beneficiary_floors,
    verify_beneficiary_floor_report,
};
use crate::temporal_profile::{TEMPORAL_PROFILE_SCHEMA, TemporalProfile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ROBUST_FLOOR_SCENARIO_SCHEMA: &str = "safrb-robust-beneficiary-floors-v0.1";
pub const ROBUST_FLOOR_CLAIM_CEILING: &str = "Joint mandatory-floor feasibility across every supplied finite synthetic scenario, assuming the scenario is observed before scenario-specific allocation (full recourse); no scenario-set completeness, evidence authority, probability calibration, physical qualification, dispatch authority or actual beneficiary-service claim.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioRecourseAssumption {
    /// Allocation may adapt after the operating scenario has been identified.
    ScenarioObservedBeforeAllocation,
}

impl ScenarioRecourseAssumption {
    fn schema_token(self) -> &'static str {
        match self {
            Self::ScenarioObservedBeforeAllocation => "scenario_observed_before_allocation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorScenario {
    pub scenario_id: String,
    pub profile: TemporalProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobustFloorScenarioInput {
    pub scenario_set_id: String,
    pub recourse_assumption: ScenarioRecourseAssumption,
    pub scenarios: Vec<FloorScenario>,
    pub floors: Vec<BeneficiaryFloor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobustFloorDisposition {
    GuaranteedAcrossSuppliedScenarios,
    PossibleInAtLeastOneSuppliedScenario,
    NotFeasibleInAnySuppliedScenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorScenarioOutcome {
    pub scenario_id: String,
    pub case_id: String,
    pub floors_jointly_feasible: bool,
    pub report: BeneficiaryFloorReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobustFloorScenarioReport {
    pub schema_version: String,
    pub scenario_set_id: String,
    pub recourse_assumption: ScenarioRecourseAssumption,
    pub scenario_set_sha256: String,
    pub floor_set_sha256: String,
    pub assurance_input_sha256: String,
    /// Sorted by scenario_id, independent of input order.
    pub outcomes: Vec<FloorScenarioOutcome>,
    pub disposition: RobustFloorDisposition,
    pub guaranteed_scenario_count: usize,
    pub total_scenario_count: usize,
    pub claim_ceiling: String,
}

fn require_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn push_length_prefixed(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let length = u64::try_from(value.len())
        .map_err(|_| "robust floor digest field exceeds u64 length".to_string())?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn canonical_profile_bytes(profile: &TemporalProfile) -> Result<Vec<u8>, String> {
    let mut canonical = profile.clone();
    canonical
        .nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    canonical
        .receipts
        .sort_by(|left, right| left.receipt_id.cmp(&right.receipt_id));
    canonical
        .transfers
        .sort_by(|left, right| left.transfer_id.cmp(&right.transfer_id));
    canonical
        .services
        .sort_by(|left, right| left.service_id.cmp(&right.service_id));
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical temporal profile: {error}"))
}

fn scenario_set_digest(set_id: &str, scenarios: &[FloorScenario]) -> Result<String, String> {
    let mut ordered = scenarios.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let mut bytes = b"LUMINOUS-SAFRB-ROBUST-FLOOR-SCENARIOS-V1\0".to_vec();
    push_length_prefixed(&mut bytes, set_id.as_bytes())?;
    for scenario in ordered {
        push_length_prefixed(&mut bytes, scenario.scenario_id.as_bytes())?;
        let profile_bytes = canonical_profile_bytes(&scenario.profile)?;
        push_length_prefixed(&mut bytes, &profile_bytes)?;
    }
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn floor_set_digest(floors: &[BeneficiaryFloor]) -> Result<String, String> {
    let mut ordered = floors.to_vec();
    ordered.sort_by(|left, right| left.service_id.cmp(&right.service_id));
    let mut bytes = b"LUMINOUS-SAFRB-BENEFICIARY-FLOORS-V1\0".to_vec();
    for floor in ordered {
        require_nonempty(&floor.service_id, "service_id")?;
        push_length_prefixed(&mut bytes, floor.service_id.as_bytes())?;
        bytes.extend_from_slice(&floor.minimum_required_quantity.to_be_bytes());
    }
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn assurance_digest(
    set_id: &str,
    recourse_assumption: ScenarioRecourseAssumption,
    scenario_digest: &str,
    floor_digest: &str,
) -> Result<String, String> {
    let mut bytes = b"LUMINOUS-SAFRB-ROBUST-FLOOR-ASSURANCE-V1\0".to_vec();
    push_length_prefixed(&mut bytes, ROBUST_FLOOR_SCENARIO_SCHEMA.as_bytes())?;
    push_length_prefixed(&mut bytes, set_id.as_bytes())?;
    push_length_prefixed(&mut bytes, recourse_assumption.schema_token().as_bytes())?;
    push_length_prefixed(&mut bytes, scenario_digest.as_bytes())?;
    push_length_prefixed(&mut bytes, floor_digest.as_bytes())?;
    push_length_prefixed(&mut bytes, ROBUST_FLOOR_CLAIM_CEILING.as_bytes())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_input(input: &RobustFloorScenarioInput) -> Result<Vec<BeneficiaryFloor>, String> {
    require_nonempty(&input.scenario_set_id, "scenario_set_id")?;
    if input.scenarios.is_empty() {
        return Err("robust floor scenario set must not be empty".to_string());
    }
    if input.floors.is_empty() {
        return Err("robust floor declarations must not be empty".to_string());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut case_ids = BTreeSet::new();
    for scenario in &input.scenarios {
        require_nonempty(&scenario.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!(
                "duplicate floor scenario ID {}",
                scenario.scenario_id
            ));
        }
        require_nonempty(&scenario.profile.case_id, "case_id")?;
        if !case_ids.insert(scenario.profile.case_id.as_str()) {
            return Err(format!(
                "duplicate floor scenario case ID {}",
                scenario.profile.case_id
            ));
        }
        if scenario.profile.schema_version != TEMPORAL_PROFILE_SCHEMA {
            return Err(format!(
                "scenario {} uses unsupported temporal schema",
                scenario.scenario_id
            ));
        }
    }

    // Scenarios must describe comparable conserved quantities and calendars.
    let first = &input.scenarios[0].profile;
    for scenario in input.scenarios.iter().skip(1) {
        let profile = &scenario.profile;
        if profile.commodity_ref != first.commodity_ref
            || profile.capacity_unit_ref != first.capacity_unit_ref
            || profile.quantity_boundary_ref != first.quantity_boundary_ref
            || profile.period_timebase_ref != first.period_timebase_ref
            || profile.horizon_last_period != first.horizon_last_period
            || profile.schema_version != first.schema_version
        {
            return Err(format!(
                "scenario {} has incomparable commodity/unit/boundary/timebase/horizon context",
                scenario.scenario_id
            ));
        }
    }

    let mut floors = input.floors.clone();
    floors.sort_by(|left, right| left.service_id.cmp(&right.service_id));
    let mut floor_ids = BTreeSet::new();
    for floor in &floors {
        require_nonempty(&floor.service_id, "service_id")?;
        if !floor_ids.insert(floor.service_id.as_str()) {
            return Err(format!("duplicate beneficiary floor {}", floor.service_id));
        }
    }

    // Each floor must continue to refer to the same beneficiary node and
    // service period in every scenario. Capacity and request ceilings may vary,
    // but silently moving a service ID to another beneficiary or window would
    // compare different obligations and could create a false robust verdict.
    let reference_services = first
        .services
        .iter()
        .map(|service| (service.service_id.as_str(), service))
        .collect::<BTreeMap<_, _>>();
    for floor in &floors {
        let reference = reference_services
            .get(floor.service_id.as_str())
            .ok_or_else(|| {
                format!(
                    "reference scenario omits floor service {}",
                    floor.service_id
                )
            })?;
        if floor.minimum_required_quantity > reference.requested_quantity {
            return Err(format!(
                "reference scenario request ceiling is below mandatory floor for {}",
                floor.service_id
            ));
        }
        for scenario in &input.scenarios {
            let services = scenario
                .profile
                .services
                .iter()
                .map(|service| (service.service_id.as_str(), service))
                .collect::<BTreeMap<_, _>>();
            let service = services.get(floor.service_id.as_str()).ok_or_else(|| {
                format!(
                    "scenario {} omits floor service {}",
                    scenario.scenario_id, floor.service_id
                )
            })?;
            if service.node_id != reference.node_id || service.period != reference.period {
                return Err(format!(
                    "scenario {} changes beneficiary node or service window for {}",
                    scenario.scenario_id, floor.service_id
                ));
            }
            if floor.minimum_required_quantity > service.requested_quantity {
                return Err(format!(
                    "scenario {} request ceiling is below mandatory floor for {}",
                    scenario.scenario_id, floor.service_id
                ));
            }
        }
    }
    Ok(floors)
}

fn disposition(outcomes: &[FloorScenarioOutcome]) -> RobustFloorDisposition {
    let feasible = outcomes
        .iter()
        .filter(|outcome| outcome.floors_jointly_feasible)
        .count();
    if feasible == outcomes.len() {
        RobustFloorDisposition::GuaranteedAcrossSuppliedScenarios
    } else if feasible > 0 {
        RobustFloorDisposition::PossibleInAtLeastOneSuppliedScenario
    } else {
        RobustFloorDisposition::NotFeasibleInAnySuppliedScenario
    }
}

/// Evaluate the same mandatory floors in every supplied complete scenario.
/// This never combines capacities from different scenario profiles.
pub fn evaluate_robust_floor_scenarios(
    input: &RobustFloorScenarioInput,
) -> Result<RobustFloorScenarioReport, String> {
    let floors = validate_input(input)?;
    let mut scenarios = input.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut outcomes = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let result = evaluate_beneficiary_floors(&scenario.profile, &floors).map_err(|error| {
            format!(
                "scenario {} floor evaluation failed: {error}",
                scenario.scenario_id
            )
        })?;
        outcomes.push(FloorScenarioOutcome {
            scenario_id: scenario.scenario_id.clone(),
            case_id: scenario.profile.case_id.clone(),
            floors_jointly_feasible: result.floors_jointly_feasible,
            report: result,
        });
    }

    let scenario_digest = scenario_set_digest(&input.scenario_set_id, &input.scenarios)?;
    let floors_digest = floor_set_digest(&floors)?;
    let input_digest = assurance_digest(
        &input.scenario_set_id,
        input.recourse_assumption,
        &scenario_digest,
        &floors_digest,
    )?;
    let guaranteed_count = outcomes
        .iter()
        .filter(|item| item.floors_jointly_feasible)
        .count();

    let report = RobustFloorScenarioReport {
        schema_version: ROBUST_FLOOR_SCENARIO_SCHEMA.to_string(),
        scenario_set_id: input.scenario_set_id.clone(),
        recourse_assumption: input.recourse_assumption,
        scenario_set_sha256: scenario_digest,
        floor_set_sha256: floors_digest,
        assurance_input_sha256: input_digest,
        disposition: disposition(&outcomes),
        guaranteed_scenario_count: guaranteed_count,
        total_scenario_count: outcomes.len(),
        claim_ceiling: ROBUST_FLOOR_CLAIM_CEILING.to_string(),
        outcomes,
    };
    verify_robust_floor_scenario_report(input, &report)?;
    Ok(report)
}

/// Independently verify the common inputs, all scenario-specific lower-bound
/// certificates and the aggregate disposition/counts.
pub fn verify_robust_floor_scenario_report(
    input: &RobustFloorScenarioInput,
    report: &RobustFloorScenarioReport,
) -> Result<(), String> {
    if report.schema_version != ROBUST_FLOOR_SCENARIO_SCHEMA {
        return Err("unsupported robust floor scenario schema".to_string());
    }
    if report.claim_ceiling != ROBUST_FLOOR_CLAIM_CEILING {
        return Err("robust floor scenario claim ceiling mismatch".to_string());
    }
    if report.scenario_set_id != input.scenario_set_id {
        return Err("robust floor scenario set identity mismatch".to_string());
    }
    if report.recourse_assumption != input.recourse_assumption {
        return Err("robust floor recourse assumption mismatch".to_string());
    }
    let floors = validate_input(input)?;
    let expected_scenario_digest = scenario_set_digest(&input.scenario_set_id, &input.scenarios)?;
    let expected_floor_digest = floor_set_digest(&floors)?;
    let expected_input_digest = assurance_digest(
        &input.scenario_set_id,
        input.recourse_assumption,
        &expected_scenario_digest,
        &expected_floor_digest,
    )?;
    if report.scenario_set_sha256 != expected_scenario_digest
        || report.floor_set_sha256 != expected_floor_digest
        || report.assurance_input_sha256 != expected_input_digest
    {
        return Err("robust floor scenario input digest mismatch".to_string());
    }
    if report.total_scenario_count != input.scenarios.len()
        || report.outcomes.len() != input.scenarios.len()
    {
        return Err("robust floor scenario receipt count mismatch".to_string());
    }

    let mut scenarios = input.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for (scenario, outcome) in scenarios.iter().zip(report.outcomes.iter()) {
        if outcome.scenario_id != scenario.scenario_id
            || outcome.case_id != scenario.profile.case_id
        {
            return Err(format!(
                "robust floor scenario receipt identity mismatch for {}",
                scenario.scenario_id
            ));
        }
        if outcome.floors_jointly_feasible != outcome.report.floors_jointly_feasible {
            return Err(format!(
                "robust floor disposition mismatch for {}",
                scenario.scenario_id
            ));
        }
        verify_beneficiary_floor_report(&scenario.profile, &floors, &outcome.report).map_err(
            |error| {
                format!(
                    "scenario {} floor certificate failed independent verification: {error}",
                    scenario.scenario_id
                )
            },
        )?;
    }

    let expected_disposition = disposition(&report.outcomes);
    let guaranteed_count = report
        .outcomes
        .iter()
        .filter(|item| item.floors_jointly_feasible)
        .count();
    if report.disposition != expected_disposition
        || report.guaranteed_scenario_count != guaranteed_count
    {
        return Err("robust floor scenario summary does not match verified receipts".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal_profile::{
        TemporalNode, TemporalReceipt, TemporalService, TemporalTransfer,
    };

    fn node(id: &str, handling: u64, issue: u64) -> TemporalNode {
        TemporalNode {
            node_id: id.to_string(),
            storage_capacity: 0,
            handling_capacity: handling,
            service_issue_capacity: issue,
            opening_inventory: 0,
        }
    }

    fn profile(case_id: &str, handling: u64) -> TemporalProfile {
        TemporalProfile {
            schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
            case_id: case_id.to_string(),
            commodity_ref: "diesel".to_string(),
            capacity_unit_ref: "litre".to_string(),
            quantity_boundary_ref: "beneficiary-deliverable-litres".to_string(),
            period_timebase_ref: "synthetic:daily-periods".to_string(),
            horizon_last_period: 1,
            nodes: vec![
                node("origin-yard", handling, 0),
                node("beneficiary-a", 0, 15),
                node("beneficiary-b", 0, 15),
            ],
            receipts: vec![TemporalReceipt {
                receipt_id: format!("receipt-{case_id}"),
                node_id: "origin-yard".to_string(),
                available_period: 0,
                quantity: 20,
            }],
            transfers: vec![
                TemporalTransfer {
                    transfer_id: format!("route-a-{case_id}"),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-a".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: 10,
                },
                TemporalTransfer {
                    transfer_id: format!("route-b-{case_id}"),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-b".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: 10,
                },
            ],
            services: vec![
                TemporalService {
                    service_id: "service-a".to_string(),
                    node_id: "beneficiary-a".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
                TemporalService {
                    service_id: "service-b".to_string(),
                    node_id: "beneficiary-b".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
            ],
        }
    }

    fn input(a_handling: u64, b_handling: u64) -> RobustFloorScenarioInput {
        RobustFloorScenarioInput {
            scenario_set_id: "diesel-operating-modes-v1".to_string(),
            recourse_assumption: ScenarioRecourseAssumption::ScenarioObservedBeforeAllocation,
            scenarios: vec![
                FloorScenario {
                    scenario_id: "mode-a".to_string(),
                    profile: profile("case-a", a_handling),
                },
                FloorScenario {
                    scenario_id: "mode-b".to_string(),
                    profile: profile("case-b", b_handling),
                },
            ],
            floors: vec![
                BeneficiaryFloor {
                    service_id: "service-a".to_string(),
                    minimum_required_quantity: 5,
                },
                BeneficiaryFloor {
                    service_id: "service-b".to_string(),
                    minimum_required_quantity: 5,
                },
            ],
        }
    }

    #[test]
    fn all_scenarios_feasible_yields_guaranteed_within_supplied_set() {
        let request = input(10, 20);
        let report = evaluate_robust_floor_scenarios(&request).unwrap();
        assert_eq!(
            report.disposition,
            RobustFloorDisposition::GuaranteedAcrossSuppliedScenarios
        );
        assert_eq!(report.guaranteed_scenario_count, 2);
        verify_robust_floor_scenario_report(&request, &report).unwrap();
    }

    #[test]
    fn one_feasible_scenario_yields_possible_not_guaranteed() {
        let request = input(10, 9);
        let report = evaluate_robust_floor_scenarios(&request).unwrap();
        assert_eq!(
            report.disposition,
            RobustFloorDisposition::PossibleInAtLeastOneSuppliedScenario
        );
        assert_eq!(report.guaranteed_scenario_count, 1);
        assert!(!report.outcomes[1].floors_jointly_feasible);
        verify_robust_floor_scenario_report(&request, &report).unwrap();
    }

    #[test]
    fn no_scenario_feasible_yields_not_possible_within_supplied_set() {
        let request = input(9, 8);
        let report = evaluate_robust_floor_scenarios(&request).unwrap();
        assert_eq!(
            report.disposition,
            RobustFloorDisposition::NotFeasibleInAnySuppliedScenario
        );
        assert_eq!(report.guaranteed_scenario_count, 0);
        verify_robust_floor_scenario_report(&request, &report).unwrap();
    }

    #[test]
    fn scenario_and_floor_substitution_are_rejected_by_input_digests() {
        let request = input(10, 10);
        let report = evaluate_robust_floor_scenarios(&request).unwrap();

        let mut changed_scenario = request.clone();
        changed_scenario.scenarios[0].profile.transfers[0].route_capacity = 4;
        assert!(
            verify_robust_floor_scenario_report(&changed_scenario, &report)
                .unwrap_err()
                .contains("input digest mismatch")
        );

        let mut changed_floor = request;
        changed_floor.floors[0].minimum_required_quantity = 6;
        assert!(
            verify_robust_floor_scenario_report(&changed_floor, &report)
                .unwrap_err()
                .contains("input digest mismatch")
        );
    }

    #[test]
    fn scenarios_cannot_rebind_a_floor_to_another_beneficiary_or_period() {
        let mut request = input(10, 10);
        request.scenarios[1].profile.services[0].node_id = "beneficiary-b".to_string();
        assert!(
            evaluate_robust_floor_scenarios(&request)
                .unwrap_err()
                .contains("changes beneficiary node or service window")
        );

        let mut request = input(10, 10);
        request.scenarios[1].profile.services[0].period = 0;
        assert!(
            evaluate_robust_floor_scenarios(&request)
                .unwrap_err()
                .contains("changes beneficiary node or service window")
        );
    }

    #[test]
    fn incomparable_units_or_calendars_are_rejected() {
        let mut request = input(10, 10);
        request.scenarios[1].profile.capacity_unit_ref = "gallon".to_string();
        assert!(
            evaluate_robust_floor_scenarios(&request)
                .unwrap_err()
                .contains("incomparable")
        );

        let mut request = input(10, 10);
        request.scenarios[1].profile.period_timebase_ref = "synthetic:weekly".to_string();
        assert!(
            evaluate_robust_floor_scenarios(&request)
                .unwrap_err()
                .contains("incomparable")
        );
    }

    #[test]
    fn scenario_order_and_floor_order_do_not_change_input_identity() {
        let mut request = input(10, 9);
        let report = evaluate_robust_floor_scenarios(&request).unwrap();
        request.scenarios.reverse();
        request.floors.reverse();
        let reordered = evaluate_robust_floor_scenarios(&request).unwrap();
        assert_eq!(report.scenario_set_sha256, reordered.scenario_set_sha256);
        assert_eq!(report.floor_set_sha256, reordered.floor_set_sha256);
        assert_eq!(
            report.assurance_input_sha256,
            reordered.assurance_input_sha256
        );
        assert_eq!(report, reordered);
    }

    #[test]
    fn published_robust_floor_schemas_are_valid_json_documents() {
        let input_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/robust-beneficiary-floors-input-v0.1.schema.json"
        ))
        .unwrap();
        let report_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/robust-beneficiary-floors-report-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(input_schema["properties"]["floors"]["type"], "array");
        assert_eq!(
            input_schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "recourse_assumption"),
            true
        );
        assert_eq!(
            input_schema["properties"]["recourse_assumption"]["const"],
            "scenario_observed_before_allocation"
        );
        assert_eq!(
            report_schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "recourse_assumption"),
            true
        );
        assert_eq!(
            report_schema["properties"]["recourse_assumption"]["const"],
            "scenario_observed_before_allocation"
        );
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            ROBUST_FLOOR_SCENARIO_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["disposition"]["enum"][0],
            "GuaranteedAcrossSuppliedScenarios"
        );
    }

    fn next_small(state: &mut u64, bound: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) % bound
    }

    #[test]
    fn seeded_scenario_sets_match_independent_physical_robustness_oracle() {
        // Each scenario has two 10 L routes, 20 L of supply, 15 L issue
        // capacity per beneficiary and floors of 5 L each. Therefore the
        // physical feasibility oracle is exact and simple: shared origin
        // handling must support at least the 10 L total mandatory floor.
        // This oracle does not call the temporal graph compiler or floor
        // circulation reduction.
        for seed in 0..250_u64 {
            let mut rng = 0x05AF_B202_6D15_CA11_u64 ^ seed;
            let scenario_count = 2 + next_small(&mut rng, 3) as usize;
            let mut scenarios = Vec::with_capacity(scenario_count);
            let mut feasible_count = 0_usize;

            for index in 0..scenario_count {
                let handling = next_small(&mut rng, 21);
                let is_feasible = handling >= 10;
                feasible_count += usize::from(is_feasible);
                scenarios.push(FloorScenario {
                    scenario_id: format!("mode-{seed}-{index}"),
                    profile: profile(&format!("case-{seed}-{index}"), handling),
                });
            }

            let request = RobustFloorScenarioInput {
                scenario_set_id: format!("seeded-robust-floor-set-{seed}"),
                recourse_assumption: ScenarioRecourseAssumption::ScenarioObservedBeforeAllocation,
                scenarios,
                floors: vec![
                    BeneficiaryFloor {
                        service_id: "service-a".to_string(),
                        minimum_required_quantity: 5,
                    },
                    BeneficiaryFloor {
                        service_id: "service-b".to_string(),
                        minimum_required_quantity: 5,
                    },
                ],
            };

            let expected = if feasible_count == scenario_count {
                RobustFloorDisposition::GuaranteedAcrossSuppliedScenarios
            } else if feasible_count > 0 {
                RobustFloorDisposition::PossibleInAtLeastOneSuppliedScenario
            } else {
                RobustFloorDisposition::NotFeasibleInAnySuppliedScenario
            };

            let report = evaluate_robust_floor_scenarios(&request).unwrap();
            assert_eq!(
                report.disposition, expected,
                "seed {seed}: physical scenario-set oracle mismatch"
            );
            assert_eq!(
                report.guaranteed_scenario_count, feasible_count,
                "seed {seed}: feasible-scenario count mismatch"
            );
            verify_robust_floor_scenario_report(&request, &report).unwrap();
        }
    }

    #[test]
    fn strict_json_round_trip_and_unknown_fields_are_enforced() {
        let request = input(10, 9);
        assert_eq!(
            request.recourse_assumption,
            ScenarioRecourseAssumption::ScenarioObservedBeforeAllocation
        );
        let input_bytes = serde_json::to_vec(&request).unwrap();
        let decoded_input: RobustFloorScenarioInput = serde_json::from_slice(&input_bytes).unwrap();
        assert_eq!(decoded_input, request);

        let mut unsupported_recourse = serde_json::to_value(&request).unwrap();
        unsupported_recourse["recourse_assumption"] = serde_json::json!("here_and_now");
        assert!(serde_json::from_value::<RobustFloorScenarioInput>(unsupported_recourse).is_err());

        let report = evaluate_robust_floor_scenarios(&decoded_input).unwrap();
        assert_eq!(
            report.recourse_assumption,
            decoded_input.recourse_assumption
        );
        let encoded = serde_json::to_vec(&report).unwrap();
        let decoded: RobustFloorScenarioReport = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, report);
        verify_robust_floor_scenario_report(&decoded_input, &decoded).unwrap();

        let mut unknown = serde_json::to_value(&report).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<RobustFloorScenarioReport>(unknown).is_err());
    }
}
