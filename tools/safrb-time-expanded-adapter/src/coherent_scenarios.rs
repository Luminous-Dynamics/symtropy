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
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use single_commodity_flow_kernel::FlowCertificateBundle;
use std::collections::BTreeSet;

pub const COHERENT_SCENARIO_SET_SCHEMA: &str = "safrb-coherent-scenario-set-v0.1";

pub const SCENARIO_SET_CLAIM_CEILING: &str = "Maximum aggregate requested delivery within each supplied coherent synthetic scenario only; no scenario-set completeness, evidence authority, probability, beneficiary-floor guarantee, physical qualification, dispatch authority or actual service claim.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoherentScenario {
    /// Identifier for one complete, internally coherent operating condition.
    pub scenario_id: String,
    pub profile: TemporalProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoherentScenarioSetInput {
    pub scenario_set_id: String,
    pub scenarios: Vec<CoherentScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoherentScenarioOutcome {
    pub scenario_id: String,
    pub case_id: String,
    pub graph_sha256: String,
    /// Maximum flow to the request ceilings for this single complete scenario.
    /// It is not a mandatory-floor allocation or guarantee.
    pub maximum_requested_delivery: u64,
    pub certificate: FlowCertificateBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoherentScenarioSetReport {
    pub schema_version: String,
    pub scenario_set_id: String,
    /// Digest of the sorted complete scenario profiles and scenario IDs.
    pub scenario_set_sha256: String,
    /// Sorted by scenario_id, independent of input ordering.
    pub outcomes: Vec<CoherentScenarioOutcome>,
    /// Minimum of the per-scenario aggregate maximum requested deliveries.
    /// This is a descriptive range endpoint, not a robust beneficiary guarantee.
    pub minimum_scenario_maximum: u64,
    /// Maximum of the per-scenario aggregate maximum requested deliveries.
    /// This is not evidence that the maximum capacity components co-occur.
    pub maximum_scenario_maximum: u64,
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
        .map_err(|_| "scenario-set digest field exceeds u64 length".to_string())?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn canonical_profile_bytes(profile: &TemporalProfile) -> Result<Vec<u8>, String> {
    let mut canonical = profile.clone();
    canonical.nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
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

fn scenario_set_digest(
    scenario_set_id: &str,
    scenarios: &[CoherentScenario],
) -> Result<String, String> {
    let mut ordered = scenarios.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut bytes = b"LUMINOUS-SAFRB-COHERENT-SCENARIO-SET-V1\0".to_vec();
    push_length_prefixed(&mut bytes, scenario_set_id.as_bytes())?;
    for scenario in ordered {
        push_length_prefixed(&mut bytes, scenario.scenario_id.as_bytes())?;
        let profile_bytes = canonical_profile_bytes(&scenario.profile)?;
        push_length_prefixed(&mut bytes, &profile_bytes)?;
    }

    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_scenario_identities(scenarios: &[CoherentScenario]) -> Result<(), String> {
    if scenarios.is_empty() {
        return Err("coherent scenario set must contain at least one scenario".to_string());
    }

    // A min/max range is meaningful only on one shared dimensional and
    // temporal basis. Scenario capacities may vary; the comparison contract
    // (commodity, units, quantity boundary, timebase and horizon) may not.
    let reference = &scenarios[0].profile;
    let mut scenario_ids = BTreeSet::new();
    let mut case_ids = BTreeSet::new();
    for scenario in scenarios {
        let profile = &scenario.profile;
        if profile.schema_version != reference.schema_version
            || profile.commodity_ref != reference.commodity_ref
            || profile.capacity_unit_ref != reference.capacity_unit_ref
            || profile.quantity_boundary_ref != reference.quantity_boundary_ref
            || profile.period_timebase_ref != reference.period_timebase_ref
            || profile.horizon_last_period != reference.horizon_last_period
        {
            return Err(format!(
                "scenario {} comparison context mismatch: all scenarios must share schema, commodity, unit, quantity boundary, period timebase and horizon",
                scenario.scenario_id
            ));
        }
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

    let scenario_set_sha256 = scenario_set_digest(scenario_set_id, scenarios)?;
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
        schema_version: COHERENT_SCENARIO_SET_SCHEMA.to_string(),
        scenario_set_id: scenario_set_id.to_string(),
        scenario_set_sha256,
        outcomes,
        minimum_scenario_maximum,
        maximum_scenario_maximum,
        claim_ceiling: SCENARIO_SET_CLAIM_CEILING.to_string(),
    })
}

/// Evaluate a serialized-input envelope using the same whole-scenario path.
pub fn evaluate_scenario_set_input(
    input: &CoherentScenarioSetInput,
) -> Result<CoherentScenarioSetReport, String> {
    evaluate_coherent_scenario_set(&input.scenario_set_id, &input.scenarios)
}

/// Verify a serialized-input envelope and its scenario report.
pub fn verify_scenario_set_input(
    input: &CoherentScenarioSetInput,
    report: &CoherentScenarioSetReport,
) -> Result<(), String> {
    verify_coherent_scenario_set(&input.scenario_set_id, &input.scenarios, report)
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

    if report.schema_version != COHERENT_SCENARIO_SET_SCHEMA {
        return Err("unsupported coherent scenario-set schema".to_string());
    }
    if report.scenario_set_id != scenario_set_id {
        return Err("scenario-set identifier mismatch".to_string());
    }
    let expected_scenario_set_sha256 = scenario_set_digest(scenario_set_id, scenarios)?;
    if report.scenario_set_sha256 != expected_scenario_set_sha256 {
        return Err("scenario-set profile digest mismatch".to_string());
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
        verify_temporal_certificate(&scenario.profile, &outcome.certificate).map_err(|error| {
            format!(
                "scenario {} certificate failed independent verification: {error}",
                scenario.scenario_id
            )
        })?;
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
    if report.minimum_scenario_maximum != minimum || report.maximum_scenario_maximum != maximum {
        return Err("scenario maximum range does not match verified receipts".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal_profile::{
        TEMPORAL_PROFILE_SCHEMA, TemporalNode, TemporalReceipt, TemporalService, TemporalTransfer,
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

    fn profile(case_id: &str, route_a_capacity: u64, route_b_capacity: u64) -> TemporalProfile {
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
    fn scenario_range_requires_one_shared_dimensional_and_temporal_context() {
        let pair = || {
            vec![
                CoherentScenario {
                    scenario_id: "mode-a".to_string(),
                    profile: profile("case-a", 10, 0),
                },
                CoherentScenario {
                    scenario_id: "mode-b".to_string(),
                    profile: profile("case-b", 0, 10),
                },
            ]
        };

        let mut scenarios = pair();
        scenarios[1].profile.commodity_ref = "water".to_string();
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));

        let mut scenarios = pair();
        scenarios[1].profile.capacity_unit_ref = "gallon".to_string();
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));

        let mut scenarios = pair();
        scenarios[1].profile.quantity_boundary_ref = "facility-output-litres".to_string();
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));

        let mut scenarios = pair();
        scenarios[1].profile.period_timebase_ref = "synthetic:weekly-periods".to_string();
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));

        let mut scenarios = pair();
        scenarios[1].profile.horizon_last_period = 2;
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));

        let mut scenarios = pair();
        scenarios[1].profile.schema_version = "temporal-profile-v0.2".to_string();
        assert!(evaluate_coherent_scenario_set("set-v1", &scenarios)
            .unwrap_err()
            .contains("comparison context mismatch"));
    }

    #[test]
    fn scenario_set_digest_binds_all_profiles_but_ignores_input_order() {
        let a = CoherentScenario {
            scenario_id: "mode-a".to_string(),
            profile: profile("case-a", 10, 0),
        };
        let b = CoherentScenario {
            scenario_id: "mode-b".to_string(),
            profile: profile("case-b", 0, 10),
        };
        let forward = scenario_set_digest("set-v1", &[a.clone(), b.clone()]).unwrap();
        let reverse = scenario_set_digest("set-v1", &[b.clone(), a.clone()]).unwrap();
        assert_eq!(forward, reverse);

        let mut reordered_a = a.clone();
        reordered_a.profile.nodes.reverse();
        reordered_a.profile.receipts.reverse();
        reordered_a.profile.transfers.reverse();
        reordered_a.profile.services.reverse();
        let reordered = scenario_set_digest("set-v1", &[reordered_a, b.clone()]).unwrap();
        assert_eq!(forward, reordered);

        let different_set_id =
            scenario_set_digest("different-set-id", &[a.clone(), b.clone()]).unwrap();
        assert_ne!(forward, different_set_id);

        let mut changed = b;
        changed.profile.transfers[0].route_capacity += 1;
        let mutated = scenario_set_digest("set-v1", &[a, changed]).unwrap();
        assert_ne!(forward, mutated);
    }

    #[test]
    fn verifier_rejects_profile_substitution_before_certificate_reuse() {
        let original = vec![
            CoherentScenario {
                scenario_id: "mode-a".to_string(),
                profile: profile("case-a", 10, 0),
            },
            CoherentScenario {
                scenario_id: "mode-b".to_string(),
                profile: profile("case-b", 0, 10),
            },
        ];
        let report = evaluate_coherent_scenario_set("set-v1", &original).unwrap();
        let mut substituted = original.clone();
        substituted[0].profile.transfers[0].route_capacity = 9;
        assert!(
            verify_coherent_scenario_set("set-v1", &substituted, &report)
                .unwrap_err()
                .contains("scenario-set profile digest mismatch")
        );
    }

    #[test]
    fn verifier_rejects_scenario_profile_digest_mismatch() {
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
        report.scenario_set_sha256 = "sha256:".to_string() + &"0".repeat(64);
        assert!(
            verify_coherent_scenario_set("set-v1", &scenarios, &report)
                .unwrap_err()
                .contains("scenario-set profile digest mismatch")
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
        let forward = evaluate_coherent_scenario_set("set-v1", &[a.clone(), b.clone()]).unwrap();
        let reverse = evaluate_coherent_scenario_set("set-v1", &[b, a]).unwrap();
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
    fn scenario_report_json_round_trip_is_strict_and_reverifies() {
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
        let report = evaluate_coherent_scenario_set("set-v1", &scenarios).unwrap();
        let encoded = serde_json::to_vec(&report).unwrap();
        let decoded: CoherentScenarioSetReport = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, report);
        verify_coherent_scenario_set("set-v1", &scenarios, &decoded).unwrap();

        let mut unknown = serde_json::to_value(&report).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CoherentScenarioSetReport>(unknown).is_err());
    }

    #[test]
    fn published_json_schemas_are_valid_json_documents() {
        let temporal: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/temporal-profile-v0.1.schema.json"))
                .unwrap();
        let scenario_input: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/coherent-scenario-input-v0.1.schema.json"
        ))
        .unwrap();
        let scenario_report: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/coherent-scenario-report-v0.1.schema.json"
        ))
        .unwrap();

        assert_eq!(
            temporal["properties"]["schema_version"]["const"],
            crate::temporal_profile::TEMPORAL_PROFILE_SCHEMA
        );
        assert_eq!(
            scenario_report["properties"]["schema_version"]["const"],
            COHERENT_SCENARIO_SET_SCHEMA
        );
        assert_eq!(
            scenario_report["properties"]["scenario_set_sha256"]["pattern"],
            "^sha256:[a-f0-9]{64}$"
        );
        assert_eq!(scenario_input["properties"]["scenarios"]["type"], "array");
    }

    fn next_small(state: &mut u64, bound: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) % bound
    }

    #[test]
    fn seeded_coherent_scenario_sets_match_physical_upper_bound_oracle() {
        for seed in 0..200_u64 {
            let mut state = 0x05AF_B202_6D15_CA11_u64 ^ seed;
            let scenario_count = 2 + next_small(&mut state, 3) as usize;
            let mut scenarios = Vec::new();
            let mut expected_maxima = Vec::new();

            for index in 0..scenario_count {
                let route_a = next_small(&mut state, 16);
                let route_b = next_small(&mut state, 16);
                let scenario_id = format!("mode-{seed}-{index}");
                let case_id = format!("case-{seed}-{index}");
                let per_route_upper = route_a.min(15) + route_b.min(15);
                let expected = 15_u64.min(20_u64.min(per_route_upper));
                expected_maxima.push(expected);
                scenarios.push(CoherentScenario {
                    scenario_id,
                    profile: profile(&case_id, route_a, route_b),
                });
            }

            let report =
                evaluate_coherent_scenario_set(&format!("seeded-set-{seed}"), &scenarios).unwrap();
            verify_coherent_scenario_set(&format!("seeded-set-{seed}"), &scenarios, &report)
                .unwrap();

            let expected_min = *expected_maxima.iter().min().unwrap();
            let expected_max = *expected_maxima.iter().max().unwrap();
            assert_eq!(
                report.minimum_scenario_maximum, expected_min,
                "seed {seed}: minimum scenario maximum"
            );
            assert_eq!(
                report.maximum_scenario_maximum, expected_max,
                "seed {seed}: maximum scenario maximum"
            );
            assert_eq!(report.outcomes.len(), scenario_count);
        }
    }

    #[test]
    fn scenario_input_envelope_round_trips_and_rejects_unknown_fields() {
        let input = CoherentScenarioSetInput {
            scenario_set_id: "portable-scenario-input-v1".to_string(),
            scenarios: vec![CoherentScenario {
                scenario_id: "mode-a".to_string(),
                profile: profile("case-a", 10, 0),
            }],
        };
        let encoded = serde_json::to_vec(&input).unwrap();
        let decoded: CoherentScenarioSetInput = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, input);

        let report = evaluate_scenario_set_input(&decoded).unwrap();
        verify_scenario_set_input(&decoded, &report).unwrap();

        let mut unknown = serde_json::to_value(&input).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CoherentScenarioSetInput>(unknown).is_err());
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
