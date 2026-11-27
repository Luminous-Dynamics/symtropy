#![forbid(unsafe_code)]

//! Whole-scenario evaluation for temporal infrastructure capacity envelopes.
//!
//! Each scenario is compiled, solved and independently verified as one complete
//! profile. This module never constructs a component-wise maximum graph by
//! combining per-resource maxima from different scenarios.
//!
//! Important claim ceiling: per-scenario maximum aggregate requested delivery
//! is not a beneficiary-floor guarantee, scenario probability, evidence
//! admission, or proof that the supplied scenario set is complete.

use crate::temporal_profile::{TemporalProfile, compile_temporal_profile};
use crate::temporal_verify::verify_temporal_certificate;
use single_commodity_flow_kernel::FlowCertificateBundle;
use std::collections::BTreeSet;

pub const SCENARIO_SET_CLAIM_CEILING: &str =
    "Maximum aggregate requested delivery within each supplied coherent synthetic scenario only; no scenario-set completeness, evidence authority, probability, beneficiary-floor guarantee, physical qualification, dispatch authority or actual service claim.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoherentScenario {
    /// Identifier for one complete, internally coherent operating condition.
    pub scenario_id: String,
    pub profile: TemporalProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoherentScenarioOutcome {
    pub scenario_id: String,
    pub case_id: String,
    pub graph_sha256: String,
    /// Maximum flow to the request ceilings for this single complete scenario.
    /// It is not a mandatory-floor allocation or guarantee.
    pub maximum_requested_delivery: u64,
    pub certificate: FlowCertificateBundle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoherentScenarioSetReport {
    pub scenario_set_id: String,
    /// Sorted by scenario_id, independent of input ordering.
    pub outcomes: Vec<CoherentScenarioOutcome>,
    /// Minimum of the per-scenario aggregate maximum requested deliveries.
    /// This is a descriptive range endpoint, not a robust beneficiary guarantee.
    pub minimum_scenario_maximum: u64,
    /// Maximum of the per-scenario aggregate maximum requested deliveries.
    /// This is not evidence that the maximum capacity components co-occur.
    pub maximum_scenario_maximum: u64,
    pub claim_ceiling: &'static str,
}

fn require_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn validate_scenario_identities(scenarios: &[CoherentScenario]) -> Result<(), String> {
    if scenarios.is_empty() {
        return Err("coherent scenario set must contain at least one scenario".to_string());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut case_ids = BTreeSet::new();
    for scenario in scenarios {
        require_nonempty(&scenario.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!(
                "duplicate coherent scenario identifier: {}",
                scenario.scenario_id
            ));
        }
        require_nonempty(&scenario.profile.case_id, "case_id")?;
        if !case_ids.insert(scenario.profile.case_id.as_str()) {
            return Err(format!(
                "duplicate temporal profile case identifier: {}",
                scenario.profile.case_id
            ));
        }
    }
    Ok(())
}

/// Evaluate each supplied coherent scenario independently.
///
/// The caller is responsible for supplying the entire admitted scenario set
/// and for authenticating its issuer, source revision, freshness, exclusivity
/// constraints and common-cause dependencies. This function assumes none of
/// those properties and never forms a synthetic graph from per-edge extrema.
pub fn evaluate_coherent_scenario_set(
    scenario_set_id: &str,
    scenarios: &[CoherentScenario],
) -> Result<CoherentScenarioSetReport, String> {
    require_nonempty(scenario_set_id, "scenario_set_id")?;
    validate_scenario_identities(scenarios)?;

    let mut ordered = scenarios.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut outcomes = Vec::with_capacity(ordered.len());
    for scenario in ordered {
        let graph = compile_temporal_profile(&scenario.profile).map_err(|error| {
            format!(
                "scenario {} failed temporal model compilation: {error}",
                scenario.scenario_id
            )
        })?;
        let result = graph.solve().map_err(|error| {
            format!(
                "scenario {} failed max-flow solve: {error}",
                scenario.scenario_id
            )
        })?;
        let certificate = FlowCertificateBundle::new(graph, result).map_err(|error| {
            format!(
                "scenario {} failed graph-bound certificate creation: {error}",
                scenario.scenario_id
            )
        })?;
        verify_temporal_certificate(&scenario.profile, &certificate).map_err(|error| {
            format!(
                "scenario {} failed independent temporal verification: {error}",
                scenario.scenario_id
            )
        })?;

        outcomes.push(CoherentScenarioOutcome {
            scenario_id: scenario.scenario_id.clone(),
            case_id: scenario.profile.case_id.clone(),
            graph_sha256: certificate.result.graph_sha256.clone(),
            maximum_requested_delivery: certificate.result.max_flow,
            certificate,
        });
    }

    let minimum_scenario_maximum = outcomes
        .iter()
        .map(|outcome| outcome.maximum_requested_delivery)
        .min()
        .ok_or_else(|| "coherent scenario set unexpectedly has no outcomes".to_string())?;
    let maximum_scenario_maximum = outcomes
        .iter()
        .map(|outcome| outcome.maximum_requested_delivery)
        .max()
        .ok_or_else(|| "coherent scenario set unexpectedly has no outcomes".to_string())?;

    Ok(CoherentScenarioSetReport {
        scenario_set_id: scenario_set_id.to_string(),
        outcomes,
        minimum_scenario_maximum,
        maximum_scenario_maximum,
        claim_ceiling: SCENARIO_SET_CLAIM_CEILING,
    })
}

/// Independently verify all scenario receipts and the reported range.
///
/// This does not authenticate scenario-set completeness, probabilities,
/// scenario exclusivity, evidence issuer authority or physical common-cause
/// relationships. It only checks that each supplied complete profile is
/// separately bound to its own valid temporal flow certificate.
pub fn verify_coherent_scenario_set(
    scenario_set_id: &str,
    scenarios: &[CoherentScenario],
    report: &CoherentScenarioSetReport,
) -> Result<(), String> {
    require_nonempty(scenario_set_id, "scenario_set_id")?;
    validate_scenario_identities(scenarios)?;

    if report.scenario_set_id != scenario_set_id {
        return Err("scenario-set identifier mismatch".to_string());
    }
    if report.claim_ceiling != SCENARIO_SET_CLAIM_CEILING {
        return Err("scenario-set claim ceiling mismatch".to_string());
    }
    if report.outcomes.len() != scenarios.len() {
        return Err("scenario-set receipt count does not match supplied scenarios".to_string());
    }

    let mut ordered = scenarios.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut observed = Vec::with_capacity(ordered.len());
    for (scenario, outcome) in ordered.iter().zip(&report.outcomes) {
        if outcome.scenario_id != scenario.scenario_id
            || outcome.case_id != scenario.profile.case_id
        {
            return Err(format!(
                "scenario receipt identity mismatch for {}",
                scenario.scenario_id
            ));
        }
        verify_temporal_certificate(&scenario.profile, &outcome.certificate).map_err(
            |error| {
                format!(
                    "scenario {} certificate failed independent verification: {error}",
                    scenario.scenario_id
                )
            },
        )?;
        if outcome.graph_sha256 != outcome.certificate.result.graph_sha256
            || outcome.maximum_requested_delivery != outcome.certificate.result.max_flow
        {
            return Err(format!(
                "scenario {} summary does not match its certificate",
                scenario.scenario_id
            ));
        }
        observed.push(outcome.maximum_requested_delivery);
    }

    let minimum = observed
        .iter()
        .copied()
        .min()
        .ok_or_else(|| "scenario report has no outcomes".to_string())?;
    let maximum = observed
        .iter()
        .copied()
        .max()
        .ok_or_else(|| "scenario report has no outcomes".to_string())?;
    if report.minimum_scenario_maximum != minimum
        || report.maximum_scenario_maximum != maximum
    {
        return Err("scenario maximum range does not match verified receipts".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal_profile::{
        TemporalNode, TemporalReceipt, TemporalService, TemporalTransfer, TEMPORAL_PROFILE_SCHEMA,
        compile_temporal_profile,
    };

    fn node(
        node_id: &str,
        storage_capacity: u64,
        handling_capacity: u64,
        service_issue_capacity: u64,
    ) -> TemporalNode {
        TemporalNode {
            node_id: node_id.to_string(),
            storage_capacity,
            handling_capacity,
            service_issue_capacity,
            opening_inventory: 0,
        }
    }

    fn profile(
        case_id: &str,
        route_a_capacity: u64,
        route_b_capacity: u64,
    ) -> TemporalProfile {
        TemporalProfile {
            schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
            case_id: case_id.to_string(),
            commodity_ref: "diesel".to_string(),
            capacity_unit_ref: "litre".to_string(),
            quantity_boundary_ref: "beneficiary-deliverable-litres".to_string(),
            period_timebase_ref: "synthetic:daily-periods".to_string(),
            horizon_last_period: 1,
            nodes: vec![
                node("origin-yard", 0, 15, 0),
                node("beneficiary-a", 0, 0, 15),
                node("beneficiary-b", 0, 0, 15),
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
                    route_capacity: route_a_capacity,
                },
                TemporalTransfer {
                    transfer_id: format!("route-b-{case_id}"),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-b".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: route_b_capacity,
                },
            ],
            services: vec![
                TemporalService {
                    service_id: format!("service-a-{case_id}"),
                    node_id: "beneficiary-a".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
                TemporalService {
                    service_id: format!("service-b-{case_id}"),
                    node_id: "beneficiary-b".to_string(),
                    period: 1,
                    requested_quantity: 15,
                },
            ],
        }
    }

    #[test]
    fn mutually_exclusive_route_maxima_are_never_spliced_into_a_scenario() {
        // In scenario A, only route A can carry 10 L. In scenario B, only
        // route B can carry 10 L. Each complete scenario shares a 15 L
        // handling gate, but the two route maxima cannot co-occur.
        let scenario_a = CoherentScenario {
            scenario_id: "pump-mode-a".to_string(),
            profile: profile("case-a", 10, 0),
        };
        let scenario_b = CoherentScenario {
            scenario_id: "pump-mode-b".to_string(),
            profile: profile("case-b", 0, 10),
        };
        let scenarios = vec![scenario_a.clone(), scenario_b.clone()];

        let report = evaluate_coherent_scenario_set("pump-modes-v1", &scenarios).unwrap();
        verify_coherent_scenario_set("pump-modes-v1", &scenarios, &report).unwrap();

        assert_eq!(report.minimum_scenario_maximum, 10);
        assert_eq!(report.maximum_scenario_maximum, 10);
        assert_eq!(report.outcomes.len(), 2);

        // This deliberately invalid component-wise splice gives each route
        // its individual maximum simultaneously and reports 15 L. It is a
        // mathematical outer relaxation, not a coherent operating scenario.
        let spliced_profile = profile("spliced-outer-bound", 10, 10);
        let spliced_graph = compile_temporal_profile(&spliced_profile).unwrap();
        let spliced_maximum = spliced_graph.solve().unwrap().max_flow;
        assert_eq!(spliced_maximum, 15);
        assert!(
            spliced_maximum > report.maximum_scenario_maximum,
            "component-wise optimistic capacities must not be mislabeled as a coherent possibility"
        );
    }

    #[test]
    fn scenario_order_does_not_change_report_order_or_range() {
        let a = CoherentScenario {
            scenario_id: "mode-a".to_string(),
            profile: profile("case-a", 10, 0),
        };
        let b = CoherentScenario {
            scenario_id: "mode-b".to_string(),
            profile: profile("case-b", 0, 10),
        };
        let forward =
            evaluate_coherent_scenario_set("set-v1", &[a.clone(), b.clone()]).unwrap();
        let reverse =
            evaluate_coherent_scenario_set("set-v1", &[b, a]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn duplicate_scenario_and_case_identities_are_rejected() {
        let a = CoherentScenario {
            scenario_id: "mode-a".to_string(),
            profile: profile("case-a", 10, 0),
        };
        let duplicate_scenario = CoherentScenario {
            scenario_id: "mode-a".to_string(),
            profile: profile("case-b", 0, 10),
        };
        assert!(
            evaluate_coherent_scenario_set("set-v1", &[a.clone(), duplicate_scenario])
                .unwrap_err()
                .contains("duplicate coherent scenario")
        );

        let duplicate_case = CoherentScenario {
            scenario_id: "mode-b".to_string(),
            profile: profile("case-a", 0, 10),
        };
        assert!(
            evaluate_coherent_scenario_set("set-v1", &[a, duplicate_case])
                .unwrap_err()
                .contains("duplicate temporal profile case")
        );
    }

    #[test]
    fn verifier_rejects_tampered_scenario_range() {
        let scenarios = vec![
            CoherentScenario {
                scenario_id: "mode-a".to_string(),
                profile: profile("case-a", 10, 0),
            },
            CoherentScenario {
                scenario_id: "mode-b".to_string(),
                profile: profile("case-b", 0, 10),
            },
        ];
        let mut report = evaluate_coherent_scenario_set("set-v1", &scenarios).unwrap();
        report.minimum_scenario_maximum = 9;
        assert!(
            verify_coherent_scenario_set("set-v1", &scenarios, &report)
                .unwrap_err()
                .contains("range does not match")
        );
    }
}
