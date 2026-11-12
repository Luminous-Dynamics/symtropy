// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Order-independent composition audit for multi-hop Scan-to-BIM representation paths.
//!
//! This module consumes the existing per-transition continuity receipt. It
//! verifies that the declared transitions form a coherent artifact graph and
//! that conservative uncertainty bounds do not silently reset between hops.
//! It does not authenticate referenced artifact bytes or recalculate survey
//! covariance.

use crate::continuity::{
    EvidenceRole, RepresentationContinuityReceipt, RepresentationKind, UncertaintyStatus,
    UncertaintyUpdateBasis, audit_representation_continuity_bytes,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_TRANSITIONS: usize = 1_024;
const BOUND_EPSILON_M: f64 = 1.0e-12;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RepresentationChainNode {
    pub transition_id: String,
    pub component_id: String,
    pub predecessor_transition_ids: Vec<String>,
    pub successor_transition_ids: Vec<String>,
    pub source_artifact_ids: Vec<String>,
    pub output_artifact_ids: Vec<String>,
    pub is_root_transition: bool,
    pub is_terminal_transition: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RepresentationChainReport {
    pub schema_version: String,
    pub receipt_id: String,
    pub input_sha256: String,
    pub transition_count: usize,
    pub dependency_edge_count: usize,
    pub artifact_handoff_count: usize,
    pub root_artifact_count: usize,
    pub terminal_artifact_count: usize,
    pub root_transition_count: usize,
    pub terminal_transition_count: usize,
    pub root_to_terminal_path_count: u64,
    pub connected_component_count: usize,
    pub topological_blocked_transition_ids: Vec<String>,
    pub uncertainty_handoff_count: usize,
    pub unknown_uncertainty_handoff_count: usize,
    pub evidence_conditioned_uncertainty_handoff_count: usize,
    pub composition_state: String,
    pub transitions: Vec<RepresentationChainNode>,
}

#[derive(Debug)]
struct Producer {
    transition_index: usize,
    sha256: String,
    role: EvidenceRole,
    representation: RepresentationKind,
}

pub fn audit_representation_chain_bytes(bytes: &[u8]) -> Result<RepresentationChainReport, String> {
    // Keep the per-transition contract authoritative. This layer adds only
    // graph composition and cross-hop handoff checks.
    audit_representation_continuity_bytes(bytes)?;
    let receipt: RepresentationContinuityReceipt = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid representation chain receipt: {error}"))?;
    audit_representation_chain(&receipt, bytes)
}

fn audit_representation_chain(
    receipt: &RepresentationContinuityReceipt,
    exact_input_bytes: &[u8],
) -> Result<RepresentationChainReport, String> {
    let transitions = &receipt.transitions;
    if transitions.is_empty() {
        return Err("representation chain must contain at least one transition".to_owned());
    }
    if transitions.len() > MAX_TRANSITIONS {
        return Err(format!(
            "representation chain exceeds the {MAX_TRANSITIONS}-transition ceiling"
        ));
    }

    // Index every producer before examining any consumer. This is deliberately
    // independent of the order in which transitions appear in the JSON array.
    let mut producers = BTreeMap::<String, Producer>::new();
    for (index, transition) in transitions.iter().enumerate() {
        for output in &transition.output_artifacts {
            let producer = Producer {
                transition_index: index,
                sha256: output.sha256.clone(),
                role: output.role,
                representation: transition.to_representation,
            };
            if producers.insert(output.artifact_id.clone(), producer).is_some() {
                return Err(format!(
                    "artifact {} has more than one transition producer",
                    output.artifact_id
                ));
            }
        }
    }

    let count = transitions.len();
    let mut successors = vec![BTreeSet::<usize>::new(); count];
    let mut predecessors = vec![BTreeSet::<usize>::new(); count];
    let mut artifact_users = BTreeMap::<String, BTreeSet<usize>>::new();
    let mut consumed_output_ids = BTreeSet::<String>::new();
    let mut root_artifact_ids = BTreeSet::<String>::new();
    let mut handoff_artifact_count = 0usize;

    for (consumer_index, transition) in transitions.iter().enumerate() {
        for source in &transition.source_artifacts {
            artifact_users
                .entry(source.artifact_id.clone())
                .or_default()
                .insert(consumer_index);

            match producers.get(&source.artifact_id) {
                Some(producer) => {
                    if producer.sha256 != source.sha256 || producer.role != source.role {
                        return Err(format!(
                            "transition {} consumes artifact {} with bytes or role that differ from its producer",
                            transition.transition_id, source.artifact_id
                        ));
                    }
                    let producer_index = producer.transition_index;
                    if producer_index == consumer_index {
                        successors[producer_index].insert(consumer_index);
                        predecessors[consumer_index].insert(producer_index);
                    } else {
                        successors[producer_index].insert(consumer_index);
                        predecessors[consumer_index].insert(producer_index);
                    }
                    consumed_output_ids.insert(source.artifact_id.clone());
                    handoff_artifact_count += 1;

                    if is_representation_payload(source.role)
                        && producer.representation != transition.from_representation
                    {
                        return Err(format!(
                            "transition {} expects {:?} input but consumes artifact {} produced as {:?}",
                            transition.transition_id,
                            transition.from_representation,
                            source.artifact_id,
                            producer.representation
                        ));
                    }
                }
                None => {
                    root_artifact_ids.insert(source.artifact_id.clone());
                }
            }
        }

        for output in &transition.output_artifacts {
            artifact_users
                .entry(output.artifact_id.clone())
                .or_default()
                .insert(consumer_index);
        }
    }

    let dependency_edge_count = (0..count)
        .map(|index| successors[index].len())
        .sum::<usize>();
    let terminal_artifact_ids: BTreeSet<String> = producers
        .keys()
        .filter(|artifact_id| !consumed_output_ids.contains(*artifact_id))
        .cloned()
        .collect();

    // Union transitions that share any artifact node. This represents a
    // bipartite artifact/transition graph, so two branches from the same root
    // remain one connected composition even before they reconverge.
    let mut union = DisjointSet::new(count);
    for users in artifact_users.values() {
        if let Some(first) = users.iter().next().copied() {
            for other in users.iter().copied().skip(1) {
                union.union(first, other);
            }
        }
    }
    let mut components = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..count {
        let root = union.find(index);
        components.entry(root).or_default().push(index);
    }
    let connected_component_count = components.len();
    let mut component_ids = vec![String::new(); count];
    for indexes in components.values() {
        let component_id = indexes
            .iter()
            .map(|index| transitions[*index].transition_id.as_str())
            .min()
            .ok_or_else(|| "empty transition component".to_owned())?
            .to_owned();
        for index in indexes {
            component_ids[*index] = component_id.clone();
        }
    }

    let mut indegree: Vec<usize> = predecessors.iter().map(BTreeSet::len).collect();
    let root_transition_indexes: Vec<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let terminal_transition_indexes: Vec<usize> = successors
        .iter()
        .enumerate()
        .filter_map(|(index, next)| next.is_empty().then_some(index))
        .collect();

    let mut queue = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut topological_order = Vec::with_capacity(count);
    while let Some(index) = queue.pop_front() {
        topological_order.push(index);
        for next in successors[index].iter().copied() {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push_back(next);
            }
        }
    }
    let mut topological_blocked_transition_ids: Vec<String> = (0..count)
        .filter(|index| indegree[*index] > 0)
        .map(|index| transitions[index].transition_id.clone())
        .collect();
    topological_blocked_transition_ids.sort();

    // Count distinct root-to-terminal transition paths in the DAG. Saturation
    // avoids overflow from highly branching, but still valid, representation graphs.
    let mut paths_from_root = vec![0u64; count];
    for index in &topological_order {
        if predecessors[*index].is_empty() {
            paths_from_root[*index] = 1;
        }
        let paths = paths_from_root[*index];
        for next in successors[*index].iter().copied() {
            paths_from_root[next] = paths_from_root[next].saturating_add(paths);
        }
    }
    let root_to_terminal_path_count = if topological_order.len() == count {
        terminal_transition_indexes
            .iter()
            .map(|index| paths_from_root[*index])
            .fold(0u64, u64::saturating_add)
    } else {
        0
    };

    let mut uncertainty_handoff_count = 0usize;
    let mut unknown_uncertainty_handoff_count = 0usize;
    let mut evidence_conditioned_uncertainty_handoff_count = 0usize;
    for consumer_index in 0..count {
        for producer_index in predecessors[consumer_index].iter().copied() {
            uncertainty_handoff_count += 1;
            let producer_after = &transitions[producer_index].uncertainty.after;
            let consumer_uncertainty = &transitions[consumer_index].uncertainty;
            let consumer_before = &consumer_uncertainty.before;

            match (producer_after.status, consumer_before.status) {
                (UncertaintyStatus::Known, UncertaintyStatus::Known) => {
                    let upstream_bound = producer_after.absolute_bound_m.ok_or_else(|| {
                        "known producer uncertainty is missing its absolute bound".to_owned()
                    })?;
                    let downstream_before = consumer_before.absolute_bound_m.ok_or_else(|| {
                        "known consumer before-uncertainty is missing its absolute bound"
                            .to_owned()
                    })?;
                    if downstream_before + BOUND_EPSILON_M < upstream_bound {
                        if consumer_uncertainty.update_basis
                            == UncertaintyUpdateBasis::EvidenceConditioned
                            && has_additional_checkpoint(
                                consumer_uncertainty,
                                &transitions[consumer_index].source_artifacts,
                            )
                        {
                            evidence_conditioned_uncertainty_handoff_count += 1;
                        } else {
                            return Err(format!(
                                "transition {} silently resets uncertainty from {:.9} m to {:.9} m after transition {}",
                                transitions[consumer_index].transition_id,
                                upstream_bound,
                                downstream_before,
                                transitions[producer_index].transition_id
                            ));
                        }
                    }
                }
                (UncertaintyStatus::Known, UncertaintyStatus::Unknown)
                | (UncertaintyStatus::Unknown, UncertaintyStatus::Unknown) => {
                    unknown_uncertainty_handoff_count += 1;
                }
                (UncertaintyStatus::Unknown, UncertaintyStatus::Known) => {
                    if consumer_uncertainty.update_basis
                        == UncertaintyUpdateBasis::EvidenceConditioned
                        && has_additional_checkpoint(
                            consumer_uncertainty,
                            &transitions[consumer_index].source_artifacts,
                        )
                    {
                        evidence_conditioned_uncertainty_handoff_count += 1;
                    } else {
                        return Err(format!(
                            "transition {} converts an unknown upstream bound into a known downstream bound without an evidence-conditioned checkpoint",
                            transitions[consumer_index].transition_id
                        ));
                    }
                }
            }
        }
    }

    let transitions_report: Vec<RepresentationChainNode> = (0..count)
        .map(|index| {
            let mut predecessor_transition_ids: Vec<String> = predecessors[index]
                .iter()
                .map(|item| transitions[*item].transition_id.clone())
                .collect();
            let mut successor_transition_ids: Vec<String> = successors[index]
                .iter()
                .map(|item| transitions[*item].transition_id.clone())
                .collect();
            let mut source_artifact_ids: Vec<String> = transitions[index]
                .source_artifacts
                .iter()
                .map(|item| item.artifact_id.clone())
                .collect();
            let mut output_artifact_ids: Vec<String> = transitions[index]
                .output_artifacts
                .iter()
                .map(|item| item.artifact_id.clone())
                .collect();
            predecessor_transition_ids.sort();
            successor_transition_ids.sort();
            source_artifact_ids.sort();
            output_artifact_ids.sort();
            RepresentationChainNode {
                transition_id: transitions[index].transition_id.clone(),
                component_id: component_ids[index].clone(),
                predecessor_transition_ids,
                successor_transition_ids,
                source_artifact_ids,
                output_artifact_ids,
                is_root_transition: predecessors[index].is_empty(),
                is_terminal_transition: successors[index].is_empty(),
            }
        })
        .collect();
    let mut transitions_report = transitions_report;
    transitions_report.sort_by(|left, right| left.transition_id.cmp(&right.transition_id));

    let composition_state = if !topological_blocked_transition_ids.is_empty() {
        "invalid"
    } else if connected_component_count > 1 {
        "fragmented"
    } else if unknown_uncertainty_handoff_count > 0 {
        "unresolved"
    } else {
        "composed"
    };

    let mut digest = Sha256::new();
    digest.update(exact_input_bytes);
    let input_sha256 = format!("{:x}", digest.finalize());

    Ok(RepresentationChainReport {
        schema_version: receipt.schema_version.clone(),
        receipt_id: receipt.receipt_id.clone(),
        input_sha256,
        transition_count: count,
        dependency_edge_count,
        artifact_handoff_count: handoff_artifact_count,
        root_artifact_count: root_artifact_ids.len(),
        terminal_artifact_count: terminal_artifact_ids.len(),
        root_transition_count: root_transition_indexes.len(),
        terminal_transition_count: terminal_transition_indexes.len(),
        root_to_terminal_path_count,
        connected_component_count,
        topological_blocked_transition_ids,
        uncertainty_handoff_count,
        unknown_uncertainty_handoff_count,
        evidence_conditioned_uncertainty_handoff_count,
        composition_state: composition_state.to_owned(),
        transitions: transitions_report,
    })
}

fn is_representation_payload(role: EvidenceRole) -> bool {
    matches!(
        role,
        EvidenceRole::RegisteredObservation
            | EvidenceRole::GeometryOutput
            | EvidenceRole::RepresentationOutput
    )
}

fn has_additional_checkpoint(
    uncertainty: &crate::continuity::UncertaintyTransition,
    source_artifacts: &[crate::continuity::ArtifactReference],
) -> bool {
    uncertainty.conditioning_evidence.iter().any(|evidence| {
        evidence.role == EvidenceRole::IndependentCheckpoint
            && !source_artifacts.iter().any(|source| {
                source.artifact_id == evidence.artifact_id || source.sha256 == evidence.sha256
            })
    })
}

#[derive(Debug)]
struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, item: usize) -> usize {
        if self.parent[item] != item {
            let root = self.find(self.parent[item]);
            self.parent[item] = root;
        }
        self.parent[item]
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left_root = self.find(left);
        let mut right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        if self.rank[left_root] < self.rank[right_root] {
            std::mem::swap(&mut left_root, &mut right_root);
        }
        self.parent[right_root] = left_root;
        if self.rank[left_root] == self.rank[right_root] {
            self.rank[left_root] += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RepresentationChainReport, audit_representation_chain_bytes};
    use serde_json::{Value, json};

    fn valid_receipt() -> Value {
        serde_json::from_str::<Value>(include_str!(
            "../../../research/scan-bim-representation-continuity-001/fixtures/known_answers_v0_1.json"
        ))
        .expect("continuity fixture JSON")["canonical_receipt"]
            .clone()
    }

    fn add_surface_to_graph_transition(value: &mut Value) {
        let mut next = value["transitions"][0].clone();
        next["transition_id"] = json!("SURFACE-TO-GRAPH-001");
        next["from_representation"] = json!("surface_mesh");
        next["to_representation"] = json!("spatial_graph");
        next["source_artifacts"] = json!([
            {
                "artifact_id": "MESH-001",
                "sha256": "c".repeat(64),
                "role": "geometry_output"
            },
            {
                "artifact_id": "RELATION-INVENTORY-GRAPH-001",
                "sha256": "1".repeat(64),
                "role": "relation_inventory"
            }
        ]);
        next["output_artifacts"] = json!([{
            "artifact_id": "GRAPH-001",
            "sha256": "8".repeat(64),
            "role": "geometry_output"
        }]);
        next["correspondences"] = json!([
            {
                "event_id": "CORR-GRAPH-WALL-001",
                "kind": "one_to_one",
                "source_entity_ids": ["WALL-SURFACE-001"],
                "target_entity_ids": ["WALL-GRAPH-001"],
                "identity_receipt_sha256": "e".repeat(64)
            },
            {
                "event_id": "CORR-GRAPH-OPENING-001",
                "kind": "one_to_one",
                "source_entity_ids": ["OPENING-SURFACE-001"],
                "target_entity_ids": ["OPENING-GRAPH-001"],
                "identity_receipt_sha256": "f".repeat(64)
            }
        ]);
        next["relation_audit"] = json!({
            "inventory_artifact_id": "RELATION-INVENTORY-GRAPH-001",
            "completeness": "complete",
            "relations": [{
                "source_relation_id": "REL-MESH-WALL-OPENING-001",
                "relation_kind": "host_opening",
                "source_relation_type": "SurfaceGraph.hosts_opening",
                "source_subject_entity_id": "WALL-SURFACE-001",
                "source_object_entity_id": "OPENING-SURFACE-001",
                "directed": true,
                "disposition": "preserved",
                "target_relations": [{
                    "relation_id": "REL-GRAPH-WALL-OPENING-001",
                    "relation_kind": "host_opening",
                    "relation_type": "SpatialGraph.hosts_opening",
                    "subject_entity_id": "WALL-GRAPH-001",
                    "object_entity_id": "OPENING-GRAPH-001"
                }],
                "reason": null
            }]
        });
        next["claims"] = json!([]);
        next["uncertainty"]["approximation_error_bound_m"] = json!(0.0005);
        next["uncertainty"]["before"]["absolute_bound_m"] = json!(0.0025);
        next["uncertainty"]["after"]["absolute_bound_m"] = json!(0.003);
        value["transitions"].as_array_mut().unwrap().push(next);
    }

    fn audit(value: &Value) -> Result<RepresentationChainReport, String> {
        let bytes = serde_json::to_vec(value).expect("serialize chain receipt");
        audit_representation_chain_bytes(&bytes)
    }

    #[test]
    fn accepts_composed_two_hop_path_and_accumulated_uncertainty() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        let report = audit(&value).expect("well-formed two-hop path");
        assert_eq!(report.transition_count, 2);
        assert_eq!(report.dependency_edge_count, 1);
        assert_eq!(report.artifact_handoff_count, 1);
        assert_eq!(report.root_transition_count, 1);
        assert_eq!(report.terminal_transition_count, 1);
        assert_eq!(report.root_to_terminal_path_count, 1);
        assert_eq!(report.connected_component_count, 1);
        assert_eq!(report.uncertainty_handoff_count, 1);
        assert_eq!(report.composition_state, "composed");
    }

    #[test]
    fn transition_order_does_not_change_semantic_chain_report() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        let original = audit(&value).expect("forward transition serialization");
        value["transitions"].as_array_mut().unwrap().reverse();
        let reversed = audit(&value).expect("reverse transition serialization");
        assert_eq!(original.transitions, reversed.transitions);
        assert_eq!(
            original.dependency_edge_count,
            reversed.dependency_edge_count
        );
        assert_eq!(
            original.root_to_terminal_path_count,
            reversed.root_to_terminal_path_count
        );
        assert_eq!(original.composition_state, reversed.composition_state);
        assert_ne!(original.input_sha256, reversed.input_sha256);
    }

    #[test]
    fn rejects_intermediate_digest_mismatch_even_when_consumer_appears_first() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        value["transitions"][1]["source_artifacts"][0]["sha256"] = json!("9".repeat(64));
        value["transitions"].as_array_mut().unwrap().reverse();
        let error = audit(&value).expect_err("producer index is independent of transition order");
        assert!(error.contains("bytes or role that differ from its producer"));
    }

    #[test]
    fn rejects_cross_hop_uncertainty_reset() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        value["transitions"][1]["uncertainty"]["before"]["absolute_bound_m"] = json!(0.002);
        value["transitions"][1]["uncertainty"]["after"]["absolute_bound_m"] = json!(0.003);
        let error = audit(&value).expect_err("downstream bound cannot reset below prior output");
        assert!(error.contains("silently resets uncertainty"));
    }

    #[test]
    fn rejects_unknown_to_known_handoff_without_conditioning_evidence() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        value["transitions"][0]["uncertainty"]["after"] = json!({
            "status": "unknown",
            "metric_id": null,
            "absolute_bound_m": null
        });
        value["transitions"][1]["uncertainty"]["update_basis"] = json!("representation_only");
        let error = audit(&value).expect_err("unknown cannot become known by conversion");
        assert!(error.contains("converts an unknown upstream bound into a known downstream bound"));
    }

    #[test]
    fn reports_disconnected_transition_components_without_claiming_one_path() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        value["transitions"][1]["source_artifacts"][0] = json!({
            "artifact_id": "UNRELATED-MESH-001",
            "sha256": "9".repeat(64),
            "role": "geometry_output"
        });
        value["transitions"][1]["uncertainty"]["before"]["absolute_bound_m"] = json!(0.002);
        let report = audit(&value).expect("disconnected component remains explicitly reported");
        assert_eq!(report.connected_component_count, 2);
        assert_eq!(report.composition_state, "fragmented");
        assert_eq!(report.root_to_terminal_path_count, 2);
    }

    #[test]
    fn detects_transition_cycles() {
        let mut value = valid_receipt();
        value["transitions"][0]["source_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "SELF-LOOP-TRANSFORM",
                "sha256": "abab".repeat(32),
                "role": "transform"
            }));
        value["transitions"][0]["output_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "SELF-LOOP-TRANSFORM",
                "sha256": "abab".repeat(32),
                "role": "transform"
            }));
        let report = audit(&value).expect("cycle is reported as invalid composition");
        assert_eq!(
            report.topological_blocked_transition_ids,
            vec!["RAW-TO-SURFACE-001"]
        );
        assert_eq!(report.composition_state, "invalid");
    }

    #[test]
    fn accepts_evidence_conditioned_cross_hop_bound_update() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        value["transitions"][1]["uncertainty"]["before"]["absolute_bound_m"] = json!(0.002);
        value["transitions"][1]["uncertainty"]["after"]["absolute_bound_m"] = json!(0.0025);
        value["transitions"][1]["uncertainty"]["update_basis"] = json!("evidence_conditioned");
        value["transitions"][1]["uncertainty"]["conditioning_evidence"] = json!([{
            "artifact_id": "CHECKPOINT-NEW-001",
            "sha256": "2".repeat(64),
            "role": "independent_checkpoint"
        }]);
        value["transitions"][1]["uncertainty"]["estimator_id"] = json!("weighted-plane-update-v2");
        value["transitions"][1]["uncertainty"]["calibration_profile_id"] =
            json!("held-out-metrology-profile-v2");

        let report = audit(&value).expect("new independent checkpoint conditions downstream bound");
        assert_eq!(report.evidence_conditioned_uncertainty_handoff_count, 1);
        assert_eq!(report.unknown_uncertainty_handoff_count, 0);
        assert_eq!(report.composition_state, "composed");
    }

    #[test]
    fn reports_shared_root_branching_as_one_component_with_two_terminal_paths() {
        let mut value = valid_receipt();
        add_surface_to_graph_transition(&mut value);
        let mut branch = value["transitions"][1].clone();
        branch["transition_id"] = json!("SURFACE-TO-PARAMETRIC-001");
        branch["to_representation"] = json!("parametric_model");
        branch["output_artifacts"] = json!([{
            "artifact_id": "PARAMETRIC-001",
            "sha256": "7".repeat(64),
            "role": "representation_output"
        }]);
        branch["correspondences"][0]["event_id"] = json!("CORR-PARAM-WALL-001");
        branch["correspondences"][0]["target_entity_ids"] = json!(["WALL-PARAM-001"]);
        branch["correspondences"][1]["event_id"] = json!("CORR-PARAM-OPENING-001");
        branch["correspondences"][1]["target_entity_ids"] = json!(["OPENING-PARAM-001"]);
        branch["relation_audit"]["relations"][0]["source_relation_id"] =
            json!("REL-PARAM-WALL-OPENING-001");
        branch["relation_audit"]["relations"][0]["target_relations"][0]["relation_id"] =
            json!("REL-PARAM-WALL-OPENING-001");
        branch["relation_audit"]["relations"][0]["target_relations"][0]["subject_entity_id"] =
            json!("WALL-PARAM-001");
        branch["relation_audit"]["relations"][0]["target_relations"][0]["object_entity_id"] =
            json!("OPENING-PARAM-001");
        value["transitions"].as_array_mut().unwrap().push(branch);

        let report = audit(&value).expect("two consumers branch from one shared mesh artifact");
        assert_eq!(report.connected_component_count, 1);
        assert_eq!(report.root_to_terminal_path_count, 2);
        assert_eq!(report.terminal_transition_count, 2);
        assert_eq!(report.terminal_artifact_count, 2);
        assert_eq!(report.composition_state, "composed");
    }

    #[test]
    fn rejects_ambiguous_multiple_producers_for_one_artifact() {
        let mut value = valid_receipt();
        let mut second = value["transitions"][0].clone();
        second["transition_id"] = json!("SECOND-PRODUCER-001");
        second["from_representation"] = json!("raw_observations");
        second["to_representation"] = json!("surface_mesh");
        second["output_artifacts"][0]["sha256"] = json!("c".repeat(64));
        second["output_artifacts"][0]["artifact_id"] = json!("MESH-001");
        second["uncertainty"]["after"]["absolute_bound_m"] = json!(0.0025);
        value["transitions"].as_array_mut().unwrap().push(second);
        let error = audit(&value).expect_err("an artifact has exactly one declared producer");
        assert!(error.contains("more than one transition producer"));
    }
}
