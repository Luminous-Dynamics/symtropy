#![forbid(unsafe_code)]

//! Domain attribution for exact lower-bound circulation infeasibility cuts.
//!
//! The kernel min-cut proves a bottleneck in the transformed graph. This module
//! maps each crossing edge back to a temporal-model constraint without
//! relabeling the cut as a causal root-cause or investment recommendation.

use crate::beneficiary_floors::{
    BeneficiaryFloor, BeneficiaryFloorReport, FLOOR_FEASIBILITY_SCHEMA,
    verify_beneficiary_floor_report,
};
use crate::temporal_profile::{TEMPORAL_PROFILE_SCHEMA, TemporalProfile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use single_commodity_flow_kernel::{CutEdge, FlowCertificateBundle, FlowEdge};
use std::collections::{BTreeMap, BTreeSet};

pub const FLOOR_CUT_EXPLANATION_SCHEMA: &str =
    "safrb-beneficiary-floor-cut-explanation-v0.1";

const AUX_SOURCE: &str = "__safrb-floor-feasibility/super-source-v0.1";
const AUX_SINK: &str = "__safrb-floor-feasibility/super-sink-v0.1";
const RETURN_EDGE_ID: &str = "__safrb-floor-feasibility/return-v0.1";
const RETURN_METADATA: &str = "safrb-beneficiary-floor-feasibility-v0.1/return";
const SUPER_SOURCE_PREFIX: &str = "__safrb-floor-feasibility/ss/";
const SUPER_SINK_PREFIX: &str = "__safrb-floor-feasibility/tt/";
const SUPER_SOURCE_META_PREFIX: &str =
    "safrb-beneficiary-floor-feasibility-v0.1/super-source/";
const SUPER_SINK_META_PREFIX: &str =
    "safrb-beneficiary-floor-feasibility-v0.1/super-sink/";

pub const FLOOR_CUT_CLAIM_CEILING: &str =
    "Exact min-cut attribution in the declared synthetic lower-bound circulation graph only; not a physical root-cause diagnosis, unique intervention set, evidence-authenticity result, legal determination, dispatch authorization or actual service claim.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorCutConstraintFamily {
    BeneficiaryRequestCeilingResidual,
    OutboundHandlingCapacity,
    BeneficiaryServiceIssueCapacity,
    EndOfPeriodStorageCarryCapacity,
    RouteCapacityAndTransit,
    ReceiptAvailabilityQuantity,
    OpeningInventoryQuantity,
    LowerBoundPositiveBalanceDemand,
    LowerBoundNegativeBalanceDischarge,
    CirculationReturnClosure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticCutEdge {
    pub auxiliary_edge_id: String,
    pub from: String,
    pub to: String,
    pub capacity: u64,
    pub metadata_ref: Option<String>,
    pub constraint_family: FloorCutConstraintFamily,
    /// Original edge ID when the cut arc corresponds to a base temporal edge.
    pub original_edge_id: Option<String>,
    /// Physical temporal node when the cut arc is a node-period gate/balance.
    pub node_id: Option<String>,
    pub period: Option<u64>,
    /// Directly associated service IDs; not a causal attribution.
    pub related_service_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorInfeasibilityExplanation {
    pub schema_version: String,
    pub case_id: String,
    pub base_graph_sha256: String,
    pub auxiliary_graph_sha256: String,
    pub required_auxiliary_flow: u64,
    pub achieved_auxiliary_flow: u64,
    pub shortfall: u64,
    pub min_cut_capacity: u64,
    pub source_side_nodes: Vec<String>,
    pub cut_edges: Vec<SemanticCutEdge>,
    pub directly_referenced_service_ids: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct EdgeAttribution {
    family: FloorCutConstraintFamily,
    original_edge_id: Option<String>,
    node_id: Option<String>,
    period: Option<u64>,
    related_service_ids: Vec<String>,
}

fn source_edge_id(kind: &str, id: &str) -> String {
    format!("{kind}/{}/{id}", id.len())
}

fn state_node_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/state/{}/{node_id}/{period}", node_id.len())
}

fn handling_node_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/handling/{}/{node_id}/{period}", node_id.len())
}

fn issue_node_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/issue/{}/{node_id}/{period}", node_id.len())
}

fn carry_edge_id(node_id: &str, period: u64) -> String {
    format!("storage-carry/{}/{node_id}/{period}", node_id.len())
}

fn handling_edge_id(node_id: &str, period: u64) -> String {
    format!("handling-gate/{}/{node_id}/{period}", node_id.len())
}

fn issue_edge_id(node_id: &str, period: u64) -> String {
    format!("service-issue-gate/{}/{node_id}/{period}", node_id.len())
}

fn service_edge_id(service_id: &str) -> String {
    source_edge_id("service", service_id)
}

fn metadata_kind(edge: &FlowEdge, kind: &str) -> bool {
    edge.metadata_ref.as_deref()
        == Some(format!("{TEMPORAL_PROFILE_SCHEMA}/{kind}/{}", edge.id).as_str())
}

fn auxiliary_graph_digest(bundle: &FlowCertificateBundle) -> Result<String, String> {
    let bytes = bundle
        .graph
        .canonical_bytes()
        .map_err(|error| format!("invalid auxiliary graph: {error}"))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn base_edge_attribution(
    profile: &TemporalProfile,
    base_edge: &FlowEdge,
) -> Result<EdgeAttribution, String> {
    let service_by_edge = profile
        .services
        .iter()
        .map(|service| (service_edge_id(&service.service_id), service))
        .collect::<BTreeMap<_, _>>();

    if let Some(service) = service_by_edge.get(base_edge.id.as_str()) {
        if !metadata_kind(base_edge, "service") {
            return Err(format!(
                "base service edge {} has unexpected metadata",
                base_edge.id
            ));
        }
        return Ok(EdgeAttribution {
            family: FloorCutConstraintFamily::BeneficiaryRequestCeilingResidual,
            original_edge_id: Some(base_edge.id.clone()),
            node_id: Some(service.node_id.clone()),
            period: Some(service.period),
            related_service_ids: vec![service.service_id.clone()],
        });
    }

    for transfer in &profile.transfers {
        let id = source_edge_id("transfer", &transfer.transfer_id);
        if id == base_edge.id {
            if !metadata_kind(base_edge, "transfer") {
                return Err(format!(
                    "base transfer edge {} has unexpected metadata",
                    base_edge.id
                ));
            }
            return Ok(EdgeAttribution {
                family: FloorCutConstraintFamily::RouteCapacityAndTransit,
                original_edge_id: Some(base_edge.id.clone()),
                node_id: Some(transfer.from_node_id.clone()),
                period: Some(transfer.departure_period),
                related_service_ids: Vec::new(),
            });
        }
    }

    for receipt in &profile.receipts {
        let id = source_edge_id("receipt", &receipt.receipt_id);
        if id == base_edge.id {
            if !metadata_kind(base_edge, "receipt") {
                return Err(format!(
                    "base receipt edge {} has unexpected metadata",
                    base_edge.id
                ));
            }
            return Ok(EdgeAttribution {
                family: FloorCutConstraintFamily::ReceiptAvailabilityQuantity,
                original_edge_id: Some(base_edge.id.clone()),
                node_id: Some(receipt.node_id.clone()),
                period: Some(receipt.available_period),
                related_service_ids: Vec::new(),
            });
        }
    }

    for node in &profile.nodes {
        let opening_id = source_edge_id("opening-inventory", &node.node_id);
        if opening_id == base_edge.id {
            if !metadata_kind(base_edge, "opening-inventory") {
                return Err(format!(
                    "base opening-inventory edge {} has unexpected metadata",
                    base_edge.id
                ));
            }
            return Ok(EdgeAttribution {
                family: FloorCutConstraintFamily::OpeningInventoryQuantity,
                original_edge_id: Some(base_edge.id.clone()),
                node_id: Some(node.node_id.clone()),
                period: Some(0),
                related_service_ids: Vec::new(),
            });
        }

        let period_count = profile
            .horizon_last_period
            .checked_add(1)
            .ok_or_else(|| "temporal horizon overflows while attributing cut".to_string())?;
        for period in 0..period_count {
            if handling_edge_id(&node.node_id, period) == base_edge.id {
                if !metadata_kind(base_edge, "handling") {
                    return Err(format!(
                        "base handling edge {} has unexpected metadata",
                        base_edge.id
                    ));
                }
                return Ok(EdgeAttribution {
                    family: FloorCutConstraintFamily::OutboundHandlingCapacity,
                    original_edge_id: Some(base_edge.id.clone()),
                    node_id: Some(node.node_id.clone()),
                    period: Some(period),
                    related_service_ids: Vec::new(),
                });
            }

            if issue_edge_id(&node.node_id, period) == base_edge.id {
                if !metadata_kind(base_edge, "service-issue") {
                    return Err(format!(
                        "base service-issue edge {} has unexpected metadata",
                        base_edge.id
                    ));
                }
                let related = profile
                    .services
                    .iter()
                    .filter(|service| service.node_id == node.node_id && service.period == period)
                    .map(|service| service.service_id.clone())
                    .collect::<Vec<_>>();
                return Ok(EdgeAttribution {
                    family: FloorCutConstraintFamily::BeneficiaryServiceIssueCapacity,
                    original_edge_id: Some(base_edge.id.clone()),
                    node_id: Some(node.node_id.clone()),
                    period: Some(period),
                    related_service_ids: related,
                });
            }

            if period < profile.horizon_last_period
                && carry_edge_id(&node.node_id, period) == base_edge.id
            {
                if !metadata_kind(base_edge, "storage-carry") {
                    return Err(format!(
                        "base storage edge {} has unexpected metadata",
                        base_edge.id
                    ));
                }
                return Ok(EdgeAttribution {
                    family: FloorCutConstraintFamily::EndOfPeriodStorageCarryCapacity,
                    original_edge_id: Some(base_edge.id.clone()),
                    node_id: Some(node.node_id.clone()),
                    period: Some(period),
                    related_service_ids: Vec::new(),
                });
            }
        }
    }

    Err(format!(
        "cut edge {} cannot be mapped to a declared temporal constraint",
        base_edge.id
    ))
}

fn node_for_balance_edge(
    profile: &TemporalProfile,
    edge: &CutEdge,
    prefix: &str,
) -> Result<String, String> {
    let metadata_prefix = if prefix == SUPER_SOURCE_PREFIX {
        SUPER_SOURCE_META_PREFIX
    } else {
        SUPER_SINK_META_PREFIX
    };
    let metadata = edge
        .metadata_ref
        .as_deref()
        .ok_or_else(|| format!("balance edge {} has no metadata", edge.id))?;
    let declared_node = metadata
        .strip_prefix(metadata_prefix)
        .ok_or_else(|| format!("balance edge {} has wrong metadata prefix", edge.id))?;
    let period_count = profile
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "temporal horizon overflows while attributing balance edge".to_string())?;
    let matches = profile
        .nodes
        .iter()
        .flat_map(|physical| {
            let mut ids = Vec::new();
            for period in 0..period_count {
                ids.push(state_node_id(&physical.node_id, period));
                ids.push(handling_node_id(&physical.node_id, period));
                ids.push(issue_node_id(&physical.node_id, period));
            }
            ids
        })
        .find(|node_id| node_id == declared_node)
        .ok_or_else(|| {
            format!(
                "balance edge {} references unknown temporal node {}",
                edge.id, declared_node
            )
        })?;

    let expected_id = format!("{prefix}{}{matches}", matches.len());
    if edge.id != expected_id {
        return Err(format!(
            "balance edge {} does not preserve its node identity",
            edge.id
        ));
    }
    Ok(matches)
}

fn balance_node_services(profile: &TemporalProfile, node_id: &str) -> Vec<String> {
    profile
        .services
        .iter()
        .filter(|service| {
            issue_node_id(&service.node_id, service.period) == node_id
        })
        .map(|service| service.service_id.clone())
        .collect()
}

fn attribute_cut_edge(
    profile: &TemporalProfile,
    floors: &[BeneficiaryFloor],
    base_graph: &single_commodity_flow_kernel::FlowGraph,
    edge: &CutEdge,
) -> Result<SemanticCutEdge, String> {
    let attribution = if let Some(base_edge) = base_graph
        .edges
        .iter()
        .find(|base_edge| base_edge.id == edge.id)
    {
        let value = base_edge_attribution(profile, base_edge)?;
        let expected_capacity = if value.family
            == FloorCutConstraintFamily::BeneficiaryRequestCeilingResidual
        {
            let service_id = value
                .related_service_ids
                .first()
                .ok_or_else(|| format!("service cut edge {} has no service identity", edge.id))?;
            let floor = floors
                .iter()
                .find(|floor| floor.service_id == *service_id)
                .ok_or_else(|| format!("service cut edge {} has no matching floor", edge.id))?;
            base_edge
                .capacity
                .checked_sub(floor.minimum_required_quantity)
                .ok_or_else(|| format!("service cut edge {} has invalid lower bound", edge.id))?
        } else {
            base_edge.capacity
        };
        if edge.from != base_edge.from
            || edge.to != base_edge.to
            || edge.capacity != expected_capacity
            || edge.metadata_ref != base_edge.metadata_ref
        {
            return Err(format!(
                "cut edge {} does not match its original graph edge and lower-bound residual",
                edge.id
            ));
        }
        value
    } else if edge.id == RETURN_EDGE_ID {
        if edge.from != AUX_SINK
            || edge.to != AUX_SOURCE
            || edge.metadata_ref.as_deref() != Some(RETURN_METADATA)
        {
            return Err("circulation return cut edge has invalid identity or endpoints".to_string());
        }
        EdgeAttribution {
            family: FloorCutConstraintFamily::CirculationReturnClosure,
            original_edge_id: None,
            node_id: None,
            period: None,
            related_service_ids: Vec::new(),
        }
    } else if edge.from == AUX_SOURCE {
        let node_id = node_for_balance_edge(profile, edge, SUPER_SOURCE_PREFIX)?;
        if edge.to != node_id
            || edge.metadata_ref.as_deref()
                != Some(format!("{SUPER_SOURCE_META_PREFIX}{node_id}").as_str())
        {
            return Err(format!("positive lower-bound balance edge {} has invalid endpoints or metadata", edge.id));
        }
        EdgeAttribution {
            family: FloorCutConstraintFamily::LowerBoundPositiveBalanceDemand,
            original_edge_id: None,
            node_id: Some(node_id.clone()),
            period: None,
            related_service_ids: balance_node_services(profile, &node_id),
        }
    } else if edge.to == AUX_SINK {
        let node_id = node_for_balance_edge(profile, edge, SUPER_SINK_PREFIX)?;
        if edge.from != node_id
            || edge.metadata_ref.as_deref()
                != Some(format!("{SUPER_SINK_META_PREFIX}{node_id}").as_str())
        {
            return Err(format!("negative lower-bound balance edge {} has invalid endpoints or metadata", edge.id));
        }
        EdgeAttribution {
            family: FloorCutConstraintFamily::LowerBoundNegativeBalanceDischarge,
            original_edge_id: None,
            node_id: Some(node_id.clone()),
            period: None,
            related_service_ids: balance_node_services(profile, &node_id),
        }
    } else {
        return Err(format!(
            "cut edge {} is not a declared lower-bound reduction edge",
            edge.id
        ));
    };

    Ok(SemanticCutEdge {
        auxiliary_edge_id: edge.id.clone(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        capacity: edge.capacity,
        metadata_ref: edge.metadata_ref.clone(),
        constraint_family: attribution.family,
        original_edge_id: attribution.original_edge_id,
        node_id: attribution.node_id,
        period: attribution.period,
        related_service_ids: attribution.related_service_ids,
    })
}

/// Produce a semantic explanation only for a genuinely infeasible floor set.
pub fn explain_beneficiary_floor_infeasibility(
    profile: &TemporalProfile,
    floors: &[BeneficiaryFloor],
    report: &BeneficiaryFloorReport,
) -> Result<FloorInfeasibilityExplanation, String> {
    verify_beneficiary_floor_report(profile, floors, report)?;
    if report.floors_jointly_feasible {
        return Err("cannot explain infeasibility for a jointly feasible floor set".to_string());
    }

    let certificate = &report.feasibility_certificate;
    certificate
        .verify()
        .map_err(|error| format!("lower-bound certificate failed verification: {error}"))?;
    let achieved = certificate.result.max_flow;
    let shortfall = report
        .required_auxiliary_flow
        .checked_sub(achieved)
        .ok_or_else(|| "infeasible report has achieved flow above required flow".to_string())?;
    if shortfall == 0 {
        return Err("infeasible report has no positive auxiliary-flow shortfall".to_string());
    }
    if certificate.result.min_cut.capacity != achieved {
        return Err("auxiliary min-cut capacity does not equal certified max flow".to_string());
    }

    let mut cut_edges = certificate
        .result
        .min_cut
        .edges
        .iter()
        .map(|edge| attribute_cut_edge(profile, &report.base_graph, edge))
        .collect::<Result<Vec<_>, _>>()?;
    cut_edges.sort_by(|left, right| left.auxiliary_edge_id.cmp(&right.auxiliary_edge_id));

    let directly_referenced_service_ids = cut_edges
        .iter()
        .flat_map(|edge| edge.related_service_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(FloorInfeasibilityExplanation {
        schema_version: FLOOR_CUT_EXPLANATION_SCHEMA.to_string(),
        case_id: profile.case_id.clone(),
        base_graph_sha256: report.base_graph_sha256.clone(),
        auxiliary_graph_sha256: auxiliary_graph_digest(certificate)?,
        required_auxiliary_flow: report.required_auxiliary_flow,
        achieved_auxiliary_flow: achieved,
        shortfall,
        min_cut_capacity: certificate.result.min_cut.capacity,
        source_side_nodes: certificate.result.source_side_nodes.clone(),
        cut_edges,
        directly_referenced_service_ids,
        claim_ceiling: FLOOR_CUT_CLAIM_CEILING.to_string(),
    })
}

/// Recompute the exact cut attribution and reject altered or incomplete explanations.
pub fn verify_beneficiary_floor_infeasibility_explanation(
    profile: &TemporalProfile,
    floors: &[BeneficiaryFloor],
    report: &BeneficiaryFloorReport,
    explanation: &FloorInfeasibilityExplanation,
) -> Result<(), String> {
    if explanation.schema_version != FLOOR_CUT_EXPLANATION_SCHEMA {
        return Err("unsupported floor-cut explanation schema".to_string());
    }
    if explanation.claim_ceiling != FLOOR_CUT_CLAIM_CEILING {
        return Err("floor-cut explanation claim ceiling mismatch".to_string());
    }

    let expected = explain_beneficiary_floor_infeasibility(profile, floors, report)?;
    if *explanation != expected {
        return Err(
            "floor-cut explanation does not match the independently reconstructed min-cut"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beneficiary_floors::{
        BeneficiaryFloor, evaluate_beneficiary_floors,
    };
    use crate::temporal_profile::{
        TemporalNode, TemporalReceipt, TemporalService, TemporalTransfer,
    };

    fn node(id: &str, storage: u64, handling: u64, issue: u64) -> TemporalNode {
        TemporalNode {
            node_id: id.to_string(),
            storage_capacity: storage,
            handling_capacity: handling,
            service_issue_capacity: issue,
            opening_inventory: 0,
        }
    }

    fn profile(
        handling_capacity: u64,
        route_a_capacity: u64,
        route_b_capacity: u64,
    ) -> TemporalProfile {
        TemporalProfile {
            schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
            case_id: "floor-cut-explanation-test".to_string(),
            commodity_ref: "diesel".to_string(),
            capacity_unit_ref: "litre".to_string(),
            quantity_boundary_ref: "beneficiary-deliverable-litres".to_string(),
            period_timebase_ref: "synthetic:daily-periods".to_string(),
            horizon_last_period: 1,
            nodes: vec![
                node("origin-yard", 0, handling_capacity, 0),
                node("beneficiary-a", 0, 0, 15),
                node("beneficiary-b", 0, 0, 15),
            ],
            receipts: vec![TemporalReceipt {
                receipt_id: "receipt-20".to_string(),
                node_id: "origin-yard".to_string(),
                available_period: 0,
                quantity: 20,
            }],
            transfers: vec![
                TemporalTransfer {
                    transfer_id: "route-a".to_string(),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-a".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: route_a_capacity,
                },
                TemporalTransfer {
                    transfer_id: "route-b".to_string(),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: "beneficiary-b".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: route_b_capacity,
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

    fn floors(a: u64, b: u64) -> Vec<BeneficiaryFloor> {
        vec![
            BeneficiaryFloor {
                service_id: "service-a".to_string(),
                minimum_required_quantity: a,
            },
            BeneficiaryFloor {
                service_id: "service-b".to_string(),
                minimum_required_quantity: b,
            },
        ]
    }

    #[test]
    fn shared_handling_cut_is_mapped_to_the_exact_node_period_gate() {
        let model = profile(15, 10, 10);
        let floor_set = floors(8, 8);
        let report = evaluate_beneficiary_floors(&model, &floor_set).unwrap();
        assert!(!report.floors_jointly_feasible);

        let explanation =
            explain_beneficiary_floor_infeasibility(&model, &floor_set, &report).unwrap();
        assert_eq!(explanation.required_auxiliary_flow, 16);
        assert_eq!(explanation.achieved_auxiliary_flow, 15);
        assert_eq!(explanation.shortfall, 1);
        assert_eq!(explanation.min_cut_capacity, 15);
        assert!(explanation.cut_edges.iter().any(|edge| {
            edge.constraint_family == FloorCutConstraintFamily::OutboundHandlingCapacity
                && edge.node_id.as_deref() == Some("origin-yard")
                && edge.period == Some(0)
        }));
        verify_beneficiary_floor_infeasibility_explanation(
            &model,
            &floor_set,
            &report,
            &explanation,
        )
        .unwrap();
    }

    #[test]
    fn route_cut_is_mapped_to_its_exact_transfer_identity() {
        let model = profile(20, 4, 20);
        let floor_set = floors(8, 8);
        let report = evaluate_beneficiary_floors(&model, &floor_set).unwrap();
        assert!(!report.floors_jointly_feasible);

        let explanation =
            explain_beneficiary_floor_infeasibility(&model, &floor_set, &report).unwrap();
        assert!(explanation.cut_edges.iter().any(|edge| {
            edge.constraint_family == FloorCutConstraintFamily::RouteCapacityAndTransit
                && edge.original_edge_id.as_deref() == Some("transfer/7/route-a")
                && edge.node_id.as_deref() == Some("origin-yard")
                && edge.period == Some(0)
        }));
        verify_beneficiary_floor_infeasibility_explanation(
            &model,
            &floor_set,
            &report,
            &explanation,
        )
        .unwrap();
    }

    #[test]
    fn explanation_is_refused_for_feasible_floors() {
        let model = profile(20, 10, 10);
        let floor_set = floors(5, 5);
        let report = evaluate_beneficiary_floors(&model, &floor_set).unwrap();
        assert!(report.floors_jointly_feasible);
        assert!(
            explain_beneficiary_floor_infeasibility(&model, &floor_set, &report)
                .unwrap_err()
                .contains("jointly feasible")
        );
    }

    #[test]
    fn mutated_cut_attribution_is_rejected() {
        let model = profile(15, 10, 10);
        let floor_set = floors(8, 8);
        let report = evaluate_beneficiary_floors(&model, &floor_set).unwrap();
        let mut explanation =
            explain_beneficiary_floor_infeasibility(&model, &floor_set, &report).unwrap();
        explanation.cut_edges[0].capacity = explanation.cut_edges[0].capacity.saturating_add(1);
        assert!(
            verify_beneficiary_floor_infeasibility_explanation(
                &model,
                &floor_set,
                &report,
                &explanation,
            )
            .unwrap_err()
            .contains("does not match")
        );
    }

    #[test]
    fn published_floor_cut_schema_is_valid_and_strict() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/beneficiary-floor-cut-explanation-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], FLOOR_CUT_EXPLANATION_SCHEMA);
        assert_eq!(schema["properties"]["cut_edges"]["type"], "array");
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn portable_explanation_round_trip_and_unknown_fields_are_checked() {
        let model = profile(15, 10, 10);
        let floor_set = floors(8, 8);
        let report = evaluate_beneficiary_floors(&model, &floor_set).unwrap();
        let explanation =
            explain_beneficiary_floor_infeasibility(&model, &floor_set, &report).unwrap();
        let encoded = serde_json::to_vec(&explanation).unwrap();
        let decoded: FloorInfeasibilityExplanation = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, explanation);
        verify_beneficiary_floor_infeasibility_explanation(
            &model,
            &floor_set,
            &report,
            &decoded,
        )
        .unwrap();

        let mut unknown = serde_json::to_value(&explanation).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<FloorInfeasibilityExplanation>(unknown).is_err());
    }
}
