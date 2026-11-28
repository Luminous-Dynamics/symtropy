// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact-scope, temporal change-impact analysis for AERO assurance dependencies.
//!
//! Edges are oriented from a dependent node to a dependency. A change to a
//! dependency invalidates every reachable dependent obligation. An obligation
//! is retained only when the graph declares complete dependency coverage and
//! every node in its declared dependency closure has a complete dependency
//! manifest. This evaluator verifies commitments and chronology; it does not
//! authenticate owner signatures or establish physical truth.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const ASSURANCE_DELTA_SCHEMA_V0_1: &str = "AeroAssuranceDeltaImpactV0_1";
pub const DEPENDENCY_GRAPH_SCHEMA_V0_1: &str = "AeroAssuranceDependencyGraphV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeltaReceiptStatusV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DependencyCoverageStatusV0_1 {
    #[serde(rename = "COMPLETE")]
    Complete,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssuranceDependencyNodeKindV0_1 {
    #[serde(rename = "CLAIM")]
    Claim,
    #[serde(rename = "OBLIGATION")]
    Obligation,
    #[serde(rename = "EVIDENCE")]
    Evidence,
    #[serde(rename = "SOURCE_RECORD")]
    SourceRecord,
    #[serde(rename = "MODEL")]
    Model,
    #[serde(rename = "SOLVER")]
    Solver,
    #[serde(rename = "PROCESS")]
    Process,
    #[serde(rename = "CONFIGURATION")]
    Configuration,
    #[serde(rename = "REQUIREMENT")]
    Requirement,
    #[serde(rename = "OWNER_POLICY")]
    OwnerPolicy,
    #[serde(rename = "OBSERVER")]
    Observer,
    #[serde(rename = "TOOL")]
    Tool,
    #[serde(rename = "OTHER")]
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDependencyNodeV0_1 {
    pub node_id: String,
    pub owner_namespace: String,
    pub kind: AssuranceDependencyNodeKindV0_1,
    pub content_sha256: String,
    /// True only when the source owner declares that this node's complete
    /// outgoing dependency set is represented in this graph snapshot.
    pub dependency_manifest_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDependencyEdgeV0_1 {
    /// The node whose validity depends on dependency_node_id.
    pub dependent_node_id: String,
    /// The node whose change may invalidate dependent_node_id.
    pub dependency_node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDependencyGraphV0_1 {
    pub schema: String,
    pub snapshot_id: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub coverage_status: DependencyCoverageStatusV0_1,
    pub nodes: Vec<AssuranceDependencyNodeV0_1>,
    pub edges: Vec<AssuranceDependencyEdgeV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeltaGraphApprovalReceiptV0_1 {
    pub verification_status: DeltaReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub graph_snapshot_sha256: String,
    pub receipt_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeltaChangeReceiptV0_1 {
    pub verification_status: DeltaReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub change_id: String,
    pub target_node_id: String,
    pub previous_content_sha256: String,
    pub replacement_content_sha256: String,
    pub receipt_sha256: String,
    /// The time the receiving system first possessed this change receipt.
    pub receiver_first_seen_at_unix_ms: u64,
    /// Subject/configuration effectivity interval, half-open.
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDeltaImpactInputV0_1 {
    pub schema: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub graph_snapshot_sha256: String,
    pub graph: AssuranceDependencyGraphV0_1,
    pub graph_approval_receipt: Option<DeltaGraphApprovalReceiptV0_1>,
    pub change_receipt: Option<DeltaChangeReceiptV0_1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssuranceDeltaDispositionV0_1 {
    #[serde(rename = "IMPACT_IDENTIFIED")]
    ImpactIdentified,
    #[serde(rename = "NO_AFFECTED_OBLIGATIONS")]
    NoAffectedObligations,
    #[serde(rename = "INCOMPLETE_IMPACT")]
    IncompleteImpact,
    #[serde(rename = "INVALID_INPUT")]
    InvalidInput,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AffectedObligationPathV0_1 {
    pub obligation_id: String,
    /// Ordered from the obligation through its dependencies to the changed node.
    pub dependency_path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetainedObligationClosureV0_1 {
    pub obligation_id: String,
    /// Sorted, complete dependency closure including the obligation node.
    pub dependency_node_ids: Vec<String>,
    /// Commitment to the closure's node digests and internal dependency edges.
    /// This is a reproducibility commitment, not a proof that the owner's
    /// dependency-completeness declaration is true.
    pub retained_dependency_closure_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDeltaImpactAssessmentV0_1 {
    pub disposition: AssuranceDeltaDispositionV0_1,
    pub claim_id: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub graph_snapshot_sha256: String,
    pub graph_approval_receipt_sha256: String,
    pub change_receipt_sha256: String,
    pub changed_node_id: String,
    pub previous_content_sha256: String,
    pub replacement_content_sha256: String,
    pub affected_obligations: Vec<AffectedObligationPathV0_1>,
    pub retained_obligations: Vec<RetainedObligationClosureV0_1>,
    pub incomplete_obligation_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub assurance_state_mutated: bool,
    pub engineering_change_approved: bool,
    pub physical_work_authorized: bool,
    pub engineering_acceptance_granted: bool,
}

#[derive(Serialize)]
struct GraphManifestV0_1<'a> {
    schema: &'a str,
    canonicalization_profile: &'static str,
    snapshot_id: &'a str,
    claim_id: &'a str,
    configuration_ref: &'a str,
    coverage_status: DependencyCoverageStatusV0_1,
    nodes: Vec<&'a AssuranceDependencyNodeV0_1>,
    edges: Vec<&'a AssuranceDependencyEdgeV0_1>,
}

#[derive(Serialize)]
struct RetainedNodeManifestV0_1<'a> {
    node_id: &'a str,
    content_sha256: &'a str,
}

#[derive(Serialize)]
struct RetainedEdgeManifestV0_1<'a> {
    dependent_node_id: &'a str,
    dependency_node_id: &'a str,
}

#[derive(Serialize)]
struct RetainedClosureManifestV0_1<'a> {
    schema: &'static str,
    graph_snapshot_sha256: &'a str,
    obligation_id: &'a str,
    nodes: Vec<RetainedNodeManifestV0_1<'a>>,
    edges: Vec<RetainedEdgeManifestV0_1<'a>>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "AERO_DELTA_CANONICAL_SERIALIZATION_FAILED".to_owned())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Hash the graph's exact semantic snapshot. Node and edge order is normalized;
/// every node's dependency-completeness declaration is inside the commitment.
pub fn compute_dependency_graph_sha256_v0_1(
    graph: &AssuranceDependencyGraphV0_1,
) -> Result<String, String> {
    let mut nodes = graph.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    let mut edges = graph.edges.iter().collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.dependent_node_id
            .cmp(&right.dependent_node_id)
            .then_with(|| left.dependency_node_id.cmp(&right.dependency_node_id))
    });
    let manifest = GraphManifestV0_1 {
        schema: &graph.schema,
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        snapshot_id: &graph.snapshot_id,
        claim_id: &graph.claim_id,
        configuration_ref: &graph.configuration_ref,
        coverage_status: graph.coverage_status,
        nodes,
        edges,
    };
    hash_serializable(&manifest)
}

fn adjacency(graph: &AssuranceDependencyGraphV0_1) -> BTreeMap<String, Vec<String>> {
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in &graph.edges {
        result
            .entry(edge.dependent_node_id.clone())
            .or_default()
            .push(edge.dependency_node_id.clone());
    }
    for targets in result.values_mut() {
        targets.sort();
        targets.dedup();
    }
    result
}

fn dependency_closure(
    start: &str,
    graph_adjacency: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::from([start.to_owned()]);
    let mut queue = VecDeque::from([start.to_owned()]);
    while let Some(current) = queue.pop_front() {
        if let Some(dependencies) = graph_adjacency.get(&current) {
            for dependency in dependencies {
                if visited.insert(dependency.clone()) {
                    queue.push_back(dependency.clone());
                }
            }
        }
    }
    visited
}

fn dependency_path(
    start: &str,
    target: &str,
    graph_adjacency: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    let mut queue = VecDeque::from([start.to_owned()]);
    let mut parent: BTreeMap<String, Option<String>> =
        BTreeMap::from([(start.to_owned(), None)]);
    while let Some(current) = queue.pop_front() {
        if current == target {
            let mut path = Vec::new();
            let mut cursor = Some(current);
            while let Some(node) = cursor {
                path.push(node.clone());
                cursor = parent.get(&node).cloned().flatten();
            }
            path.reverse();
            return Some(path);
        }
        if let Some(dependencies) = graph_adjacency.get(&current) {
            for dependency in dependencies {
                if !parent.contains_key(dependency) {
                    parent.insert(dependency.clone(), Some(current.clone()));
                    queue.push_back(dependency.clone());
                }
            }
        }
    }
    None
}

fn retained_closure_sha256(
    obligation_id: &str,
    closure: &BTreeSet<String>,
    graph: &AssuranceDependencyGraphV0_1,
    graph_snapshot_sha256: &str,
) -> Result<String, String> {
    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let nodes = closure
        .iter()
        .map(|node_id| {
            let node = node_map[node_id.as_str()];
            RetainedNodeManifestV0_1 {
                node_id: &node.node_id,
                content_sha256: &node.content_sha256,
            }
        })
        .collect();
    let mut retained_edges = graph
        .edges
        .iter()
        .filter(|edge| {
            closure.contains(&edge.dependent_node_id)
                && closure.contains(&edge.dependency_node_id)
        })
        .collect::<Vec<_>>();
    retained_edges.sort_by(|left, right| {
        left.dependent_node_id
            .cmp(&right.dependent_node_id)
            .then_with(|| left.dependency_node_id.cmp(&right.dependency_node_id))
    });
    let edges = retained_edges
        .into_iter()
        .map(|edge| RetainedEdgeManifestV0_1 {
            dependent_node_id: &edge.dependent_node_id,
            dependency_node_id: &edge.dependency_node_id,
        })
        .collect();
    hash_serializable(&RetainedClosureManifestV0_1 {
        schema: "AeroRetainedDependencyClosureV0_1",
        graph_snapshot_sha256,
        obligation_id,
        nodes,
        edges,
    })
}

fn validate_input(input: &AssuranceDeltaImpactInputV0_1) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let graph = &input.graph;

    if input.schema != ASSURANCE_DELTA_SCHEMA_V0_1
        || graph.schema != DEPENDENCY_GRAPH_SCHEMA_V0_1
        || !valid_identifier(&input.claim_id)
        || !valid_identifier(&input.configuration_ref)
        || !valid_identifier(&graph.snapshot_id)
        || graph.claim_id != input.claim_id
        || graph.configuration_ref != input.configuration_ref
    {
        errors.insert("AERO_DELTA_SCOPE_OR_SCHEMA_INVALID".to_owned());
    }

    if !valid_sha256(&input.graph_snapshot_sha256)
        || compute_dependency_graph_sha256_v0_1(graph).as_deref()
            != Ok(input.graph_snapshot_sha256.as_str())
    {
        errors.insert("AERO_DELTA_GRAPH_COMMITMENT_MISMATCH".to_owned());
    }

    let mut node_ids = BTreeSet::new();
    let mut claim_nodes = 0usize;
    for node in &graph.nodes {
        if !valid_identifier(&node.node_id)
            || !valid_identifier(&node.owner_namespace)
            || !valid_sha256(&node.content_sha256)
            || !node_ids.insert(node.node_id.as_str())
        {
            errors.insert("AERO_DELTA_NODE_ID_OR_DIGEST_INVALID".to_owned());
        }
        if node.kind == AssuranceDependencyNodeKindV0_1::Claim {
            claim_nodes += 1;
            if node.node_id != input.claim_id {
                errors.insert("AERO_DELTA_CLAIM_ROOT_ID_MISMATCH".to_owned());
            }
        }
    }
    if claim_nodes != 1 {
        errors.insert("AERO_DELTA_REQUIRES_ONE_CLAIM_ROOT".to_owned());
    }

    let mut edges = BTreeSet::new();
    for edge in &graph.edges {
        if !node_ids.contains(edge.dependent_node_id.as_str())
            || !node_ids.contains(edge.dependency_node_id.as_str())
            || edge.dependent_node_id == edge.dependency_node_id
            || !edges.insert((
                edge.dependent_node_id.as_str(),
                edge.dependency_node_id.as_str(),
            ))
        {
            errors.insert("AERO_DELTA_EDGE_REFERENCE_OR_DUPLICATE_INVALID".to_owned());
        }
    }

    let obligations = graph
        .nodes
        .iter()
        .filter(|node| node.kind == AssuranceDependencyNodeKindV0_1::Obligation)
        .collect::<Vec<_>>();
    if obligations.is_empty() {
        errors.insert("AERO_DELTA_NO_OBLIGATION_NODES".to_owned());
    }

    let change = input.change_receipt.as_ref();
    match change {
        None => {
            errors.insert("AERO_DELTA_CHANGE_RECEIPT_MISSING".to_owned());
        }
        Some(change) => {
            if change.verification_status != DeltaReceiptStatusV0_1::Verified {
                errors.insert("AERO_DELTA_CHANGE_RECEIPT_NOT_VERIFIED".to_owned());
            }
            if !valid_identifier(&change.owner_namespace)
                || !valid_identifier(&change.change_id)
                || !valid_identifier(&change.target_node_id)
                || !valid_sha256(&change.previous_content_sha256)
                || !valid_sha256(&change.replacement_content_sha256)
                || !valid_sha256(&change.receipt_sha256)
                || change.previous_content_sha256 == change.replacement_content_sha256
            {
                errors.insert("AERO_DELTA_CHANGE_RECEIPT_INVALID".to_owned());
            }
            if change.claim_id != input.claim_id
                || change.configuration_ref != input.configuration_ref
            {
                errors.insert("AERO_DELTA_CHANGE_SCOPE_MISMATCH".to_owned());
            }
            if change.receiver_first_seen_at_unix_ms > input.as_known_at_unix_ms {
                errors.insert("AERO_DELTA_CHANGE_NOT_KNOWN_AT_CUTOFF".to_owned());
            }
            if change.effective_from_unix_ms >= change.effective_until_unix_ms
                || change.effective_from_unix_ms > input.as_known_at_unix_ms
                || change.effective_until_unix_ms <= input.as_known_at_unix_ms
            {
                errors.insert("AERO_DELTA_CHANGE_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
            }
            match graph.nodes.iter().find(|node| node.node_id == change.target_node_id) {
                None => {
                    errors.insert("AERO_DELTA_CHANGED_NODE_NOT_IN_GRAPH".to_owned());
                }
                Some(node) if node.content_sha256 != change.previous_content_sha256 => {
                    errors.insert("AERO_DELTA_PREVIOUS_DIGEST_MISMATCH".to_owned());
                }
                Some(node) if node.owner_namespace != change.owner_namespace => {
                    errors.insert("AERO_DELTA_CHANGE_OWNER_MISMATCH".to_owned());
                }
                Some(_) => {}
            }
        }
    }

    match &input.graph_approval_receipt {
        None => {
            errors.insert("AERO_DELTA_GRAPH_APPROVAL_MISSING".to_owned());
        }
        Some(receipt) => {
            if receipt.verification_status != DeltaReceiptStatusV0_1::Verified {
                errors.insert("AERO_DELTA_GRAPH_APPROVAL_NOT_VERIFIED".to_owned());
            }
            if !valid_identifier(&receipt.owner_namespace)
                || !valid_sha256(&receipt.receipt_sha256)
                || receipt.graph_snapshot_sha256 != input.graph_snapshot_sha256
                || receipt.claim_id != input.claim_id
                || receipt.configuration_ref != input.configuration_ref
            {
                errors.insert("AERO_DELTA_GRAPH_APPROVAL_SCOPE_MISMATCH".to_owned());
            }
            if receipt.receiver_first_seen_at_unix_ms > input.as_known_at_unix_ms {
                errors.insert("AERO_DELTA_GRAPH_NOT_KNOWN_AT_CUTOFF".to_owned());
            }
            if receipt.effective_from_unix_ms >= receipt.effective_until_unix_ms
                || receipt.effective_from_unix_ms > input.as_known_at_unix_ms
                || receipt.effective_until_unix_ms <= input.as_known_at_unix_ms
            {
                errors.insert("AERO_DELTA_GRAPH_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
            }
        }
    }

    errors
}

fn invalid_result(
    input: &AssuranceDeltaImpactInputV0_1,
    reasons: BTreeSet<String>,
) -> AssuranceDeltaImpactAssessmentV0_1 {
    AssuranceDeltaImpactAssessmentV0_1 {
        disposition: AssuranceDeltaDispositionV0_1::InvalidInput,
        claim_id: input.claim_id.clone(),
        configuration_ref: input.configuration_ref.clone(),
        as_known_at_unix_ms: input.as_known_at_unix_ms,
        graph_snapshot_sha256: input.graph_snapshot_sha256.clone(),
        graph_approval_receipt_sha256: input
            .graph_approval_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_sha256.clone())
            .unwrap_or_default(),
        change_receipt_sha256: input
            .change_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_sha256.clone())
            .unwrap_or_default(),
        changed_node_id: input
            .change_receipt
            .as_ref()
            .map(|receipt| receipt.target_node_id.clone())
            .unwrap_or_default(),
        previous_content_sha256: input
            .change_receipt
            .as_ref()
            .map(|receipt| receipt.previous_content_sha256.clone())
            .unwrap_or_default(),
        replacement_content_sha256: input
            .change_receipt
            .as_ref()
            .map(|receipt| receipt.replacement_content_sha256.clone())
            .unwrap_or_default(),
        affected_obligations: Vec::new(),
        retained_obligations: Vec::new(),
        incomplete_obligation_ids: Vec::new(),
        reason_codes: reasons.into_iter().collect(),
        assurance_state_mutated: false,
        engineering_change_approved: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}

/// Evaluate the exact dependency impact of one previously received change.
///
/// The graph must already have been independently authenticated upstream.
/// This function checks the upstream "VERIFIED" dispositions, scope, digest,
/// chronology and effectivity but does not verify signatures or root trust.
pub fn evaluate_assurance_delta_impact_v0_1(
    input: &AssuranceDeltaImpactInputV0_1,
) -> AssuranceDeltaImpactAssessmentV0_1 {
    let errors = validate_input(input);
    if !errors.is_empty() {
        return invalid_result(input, errors);
    }

    let graph = &input.graph;
    let change = input.change_receipt.as_ref().expect("validated change receipt");
    let changed_node_id = change.target_node_id.as_str();
    let graph_adjacency = adjacency(graph);
    let claim_root_closure = dependency_closure(&input.claim_id, &graph_adjacency);
    let mut affected = Vec::new();
    let mut retained = Vec::new();
    let mut incomplete = Vec::new();
    let mut reason_codes = BTreeSet::new();

    let mut obligations = graph
        .nodes
        .iter()
        .filter(|node| node.kind == AssuranceDependencyNodeKindV0_1::Obligation)
        .collect::<Vec<_>>();
    obligations.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let claim_root_complete = node_map
        .get(input.claim_id.as_str())
        .is_some_and(|node| node.dependency_manifest_complete);

    for obligation in obligations {
        let reachable_from_claim = claim_root_closure.contains(&obligation.node_id);
        let closure = dependency_closure(&obligation.node_id, &graph_adjacency);
        if let Some(path) =
            dependency_path(&obligation.node_id, changed_node_id, &graph_adjacency)
        {
            affected.push(AffectedObligationPathV0_1 {
                obligation_id: obligation.node_id.clone(),
                dependency_path: path,
            });
            continue;
        }

        let closure_complete = reachable_from_claim
            && claim_root_complete
            && graph.coverage_status == DependencyCoverageStatusV0_1::Complete
            && closure.iter().all(|node_id| {
                node_map
                    .get(node_id.as_str())
                    .is_some_and(|node| node.dependency_manifest_complete)
            });

        if closure_complete {
            match retained_closure_sha256(
                &obligation.node_id,
                &closure,
                graph,
                &input.graph_snapshot_sha256,
            ) {
                Ok(digest) => retained.push(RetainedObligationClosureV0_1 {
                    obligation_id: obligation.node_id.clone(),
                    dependency_node_ids: closure.into_iter().collect(),
                    retained_dependency_closure_sha256: digest,
                }),
                Err(_) => {
                    incomplete.push(obligation.node_id.clone());
                    reason_codes.insert("AERO_DELTA_RETAINED_CLOSURE_COMMITMENT_FAILED".to_owned());
                }
            }
        } else {
            incomplete.push(obligation.node_id.clone());
            if !reachable_from_claim {
                reason_codes.insert("AERO_DELTA_OBLIGATION_NOT_REACHABLE_FROM_CLAIM_ROOT".to_owned());
            }
            if !claim_root_complete {
                reason_codes.insert("AERO_DELTA_CLAIM_ROOT_DEPENDENCY_MANIFEST_INCOMPLETE".to_owned());
            }
            if graph.coverage_status != DependencyCoverageStatusV0_1::Complete {
                reason_codes.insert("AERO_DELTA_GRAPH_DEPENDENCY_COVERAGE_INCOMPLETE".to_owned());
            } else if closure.iter().any(|node_id| {
                node_map
                    .get(node_id.as_str())
                    .is_some_and(|node| !node.dependency_manifest_complete)
            }) {
                reason_codes.insert("AERO_DELTA_NODE_DEPENDENCY_MANIFEST_INCOMPLETE".to_owned());
            }
        }
    }

    if affected.is_empty() {
        reason_codes.insert("AERO_DELTA_NO_KNOWN_AFFECTED_OBLIGATIONS".to_owned());
    } else {
        reason_codes.insert("AERO_DELTA_DEPENDENCY_PATH_INVALIDATES_OBLIGATION".to_owned());
    }
    if !incomplete.is_empty() {
        reason_codes.insert("AERO_DELTA_IMPACT_CANNOT_BE_CLOSED".to_owned());
    }

    let disposition = if !incomplete.is_empty() {
        AssuranceDeltaDispositionV0_1::IncompleteImpact
    } else if affected.is_empty() {
        AssuranceDeltaDispositionV0_1::NoAffectedObligations
    } else {
        AssuranceDeltaDispositionV0_1::ImpactIdentified
    };

    let mut affected_obligations = affected;
    affected_obligations.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    retained.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    incomplete.sort();

    AssuranceDeltaImpactAssessmentV0_1 {
        disposition,
        claim_id: input.claim_id.clone(),
        configuration_ref: input.configuration_ref.clone(),
        as_known_at_unix_ms: input.as_known_at_unix_ms,
        graph_snapshot_sha256: input.graph_snapshot_sha256.clone(),
        graph_approval_receipt_sha256: input
            .graph_approval_receipt
            .as_ref()
            .expect("validated graph receipt")
            .receipt_sha256
            .clone(),
        change_receipt_sha256: change.receipt_sha256.clone(),
        changed_node_id: change.target_node_id.clone(),
        previous_content_sha256: change.previous_content_sha256.clone(),
        replacement_content_sha256: change.replacement_content_sha256.clone(),
        affected_obligations: affected_obligations,
        retained_obligations: retained,
        incomplete_obligation_ids: incomplete,
        reason_codes: reason_codes.into_iter().collect(),
        assurance_state_mutated: false,
        engineering_change_approved: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUTOFF: u64 = 1_800_000_000_000;

    fn node(
        id: &str,
        kind: AssuranceDependencyNodeKindV0_1,
        digest_byte: char,
    ) -> AssuranceDependencyNodeV0_1 {
        AssuranceDependencyNodeV0_1 {
            node_id: id.to_owned(),
            owner_namespace: if id == "source:material-lot-record" {
                "owner:materials-authority".to_owned()
            } else {
                "owner:engineering-evidence".to_owned()
            },
            kind,
            content_sha256: digest_byte.to_string().repeat(64),
            dependency_manifest_complete: true,
        }
    }

    fn graph() -> AssuranceDependencyGraphV0_1 {
        AssuranceDependencyGraphV0_1 {
            schema: DEPENDENCY_GRAPH_SCHEMA_V0_1.to_owned(),
            snapshot_id: "graph:snapshot-r1".to_owned(),
            claim_id: "claim:wing-panel-strength".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            coverage_status: DependencyCoverageStatusV0_1::Complete,
            nodes: vec![
                node("claim:wing-panel-strength", AssuranceDependencyNodeKindV0_1::Claim, '1'),
                node("obligation:material-lot", AssuranceDependencyNodeKindV0_1::Obligation, '2'),
                node("evidence:material-certificate", AssuranceDependencyNodeKindV0_1::Evidence, '3'),
                node("source:material-lot-record", AssuranceDependencyNodeKindV0_1::SourceRecord, 'a'),
                node("obligation:solver-validation", AssuranceDependencyNodeKindV0_1::Obligation, '4'),
                node("evidence:solver-report", AssuranceDependencyNodeKindV0_1::Evidence, '5'),
                node("source:solver-validation-record", AssuranceDependencyNodeKindV0_1::SourceRecord, '6'),
            ],
            edges: vec![
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "claim:wing-panel-strength".to_owned(),
                    dependency_node_id: "obligation:material-lot".to_owned(),
                },
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "obligation:material-lot".to_owned(),
                    dependency_node_id: "evidence:material-certificate".to_owned(),
                },
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "evidence:material-certificate".to_owned(),
                    dependency_node_id: "source:material-lot-record".to_owned(),
                },
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "claim:wing-panel-strength".to_owned(),
                    dependency_node_id: "obligation:solver-validation".to_owned(),
                },
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "obligation:solver-validation".to_owned(),
                    dependency_node_id: "evidence:solver-report".to_owned(),
                },
                AssuranceDependencyEdgeV0_1 {
                    dependent_node_id: "evidence:solver-report".to_owned(),
                    dependency_node_id: "source:solver-validation-record".to_owned(),
                },
            ],
        }
    }

    fn input(graph: AssuranceDependencyGraphV0_1) -> AssuranceDeltaImpactInputV0_1 {
        let graph_sha = compute_dependency_graph_sha256_v0_1(&graph).expect("graph hash");
        let change = DeltaChangeReceiptV0_1 {
            verification_status: DeltaReceiptStatusV0_1::Verified,
            owner_namespace: "owner:materials-authority".to_owned(),
            claim_id: graph.claim_id.clone(),
            configuration_ref: graph.configuration_ref.clone(),
            change_id: "change:material-lot-correction".to_owned(),
            target_node_id: "source:material-lot-record".to_owned(),
            previous_content_sha256: "a".repeat(64),
            replacement_content_sha256: "b".repeat(64),
            receipt_sha256: "c".repeat(64),
            receiver_first_seen_at_unix_ms: CUTOFF - 10_000,
            effective_from_unix_ms: CUTOFF - 20_000,
            effective_until_unix_ms: CUTOFF + 20_000,
        };
        AssuranceDeltaImpactInputV0_1 {
            schema: ASSURANCE_DELTA_SCHEMA_V0_1.to_owned(),
            claim_id: graph.claim_id.clone(),
            configuration_ref: graph.configuration_ref.clone(),
            as_known_at_unix_ms: CUTOFF,
            graph_snapshot_sha256: graph_sha.clone(),
            graph: graph.clone(),
            graph_approval_receipt: Some(DeltaGraphApprovalReceiptV0_1 {
                verification_status: DeltaReceiptStatusV0_1::Verified,
                owner_namespace: "owner:assurance-graph-authority".to_owned(),
                claim_id: graph.claim_id,
                configuration_ref: graph.configuration_ref,
                graph_snapshot_sha256: graph_sha,
                receipt_sha256: "d".repeat(64),
                receiver_first_seen_at_unix_ms: CUTOFF - 30_000,
                effective_from_unix_ms: CUTOFF - 40_000,
                effective_until_unix_ms: CUTOFF + 40_000,
            }),
            change_receipt: Some(change),
        }
    }

    #[test]
    fn propagates_change_transitively_and_retains_disjoint_complete_obligation() {
        let assessment = evaluate_assurance_delta_impact_v0_1(&input(graph()));
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::ImpactIdentified);
        assert_eq!(assessment.affected_obligations.len(), 1);
        assert_eq!(
            assessment.affected_obligations[0].obligation_id,
            "obligation:material-lot"
        );
        assert_eq!(
            assessment.affected_obligations[0].dependency_path,
            vec![
                "obligation:material-lot",
                "evidence:material-certificate",
                "source:material-lot-record"
            ]
        );
        assert_eq!(assessment.retained_obligations.len(), 1);
        assert_eq!(
            assessment.retained_obligations[0].obligation_id,
            "obligation:solver-validation"
        );
        assert!(assessment.incomplete_obligation_ids.is_empty());
        assert!(!assessment.assurance_state_mutated);
        assert!(!assessment.engineering_change_approved);
        assert!(!assessment.physical_work_authorized);
        assert!(!assessment.engineering_acceptance_granted);
    }

    #[test]
    fn incomplete_graph_never_labels_unreached_obligations_as_retained() {
        let mut graph = graph();
        graph.coverage_status = DependencyCoverageStatusV0_1::Incomplete;
        let input = input(graph);
        let assessment = evaluate_assurance_delta_impact_v0_1(&input);
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::IncompleteImpact);
        assert_eq!(assessment.affected_obligations.len(), 1);
        assert!(assessment.retained_obligations.is_empty());
        assert_eq!(
            assessment.incomplete_obligation_ids,
            vec!["obligation:solver-validation"]
        );
    }

    #[test]
    fn incomplete_node_manifest_blocks_retained_status() {
        let mut graph = graph();
        let node = graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "evidence:solver-report")
            .expect("solver evidence node");
        node.dependency_manifest_complete = false;
        let assessment = evaluate_assurance_delta_impact_v0_1(&input(graph));
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::IncompleteImpact);
        assert!(assessment.retained_obligations.is_empty());
        assert_eq!(
            assessment.incomplete_obligation_ids,
            vec!["obligation:solver-validation"]
        );
    }

    #[test]
    fn incomplete_claim_root_manifest_blocks_retained_status() {
        let mut graph = graph();
        let claim_root = graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == AssuranceDependencyNodeKindV0_1::Claim)
            .expect("claim root");
        claim_root.dependency_manifest_complete = false;
        let assessment = evaluate_assurance_delta_impact_v0_1(&input(graph));
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::IncompleteImpact);
        assert!(assessment.retained_obligations.is_empty());
        assert_eq!(
            assessment.incomplete_obligation_ids,
            vec!["obligation:solver-validation"]
        );
        assert!(assessment.reason_codes.contains(
            &"AERO_DELTA_CLAIM_ROOT_DEPENDENCY_MANIFEST_INCOMPLETE".to_owned()
        ));
    }

    #[test]
    fn wrong_change_owner_cannot_replace_a_node_from_another_owner() {
        let mut input = input(graph());
        input
            .change_receipt
            .as_mut()
            .expect("change receipt")
            .owner_namespace = "owner:unrelated-authority".to_owned();
        let assessment = evaluate_assurance_delta_impact_v0_1(&input);
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::InvalidInput);
        assert!(assessment
            .reason_codes
            .contains(&"AERO_DELTA_CHANGE_OWNER_MISMATCH".to_owned()));
    }

    #[test]
    fn late_change_receipt_cannot_backfill_historical_knowledge() {
        let mut input = input(graph());
        input
            .change_receipt
            .as_mut()
            .expect("change receipt")
            .receiver_first_seen_at_unix_ms = CUTOFF + 1;
        let assessment = evaluate_assurance_delta_impact_v0_1(&input);
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::InvalidInput);
        assert!(assessment
            .reason_codes
            .contains(&"AERO_DELTA_CHANGE_NOT_KNOWN_AT_CUTOFF".to_owned()));
    }

    #[test]
    fn changed_node_digest_must_match_the_previous_source_commitment() {
        let mut input = input(graph());
        input
            .change_receipt
            .as_mut()
            .expect("change receipt")
            .previous_content_sha256 = "f".repeat(64);
        let assessment = evaluate_assurance_delta_impact_v0_1(&input);
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::InvalidInput);
        assert!(assessment
            .reason_codes
            .contains(&"AERO_DELTA_PREVIOUS_DIGEST_MISMATCH".to_owned()));
    }

    #[test]
    fn graph_order_permutation_preserves_digest_and_assessment() {
        let baseline = input(graph());
        let baseline_assessment = evaluate_assurance_delta_impact_v0_1(&baseline);
        let mut permuted = baseline.clone();
        permuted.graph.nodes.reverse();
        permuted.graph.edges.reverse();
        permuted.graph_snapshot_sha256 =
            compute_dependency_graph_sha256_v0_1(&permuted.graph).expect("permuted graph hash");
        let receipt = permuted.graph_approval_receipt.as_mut().expect("graph receipt");
        receipt.graph_snapshot_sha256 = permuted.graph_snapshot_sha256.clone();
        let result = evaluate_assurance_delta_impact_v0_1(&permuted);
        assert_eq!(baseline_assessment, result);
    }

    #[test]
    fn adding_a_known_dependency_cannot_reduce_the_affected_set() {
        let baseline = evaluate_assurance_delta_impact_v0_1(&input(graph()));
        let mut expanded_graph = graph();
        expanded_graph.edges.push(AssuranceDependencyEdgeV0_1 {
            dependent_node_id: "obligation:solver-validation".to_owned(),
            dependency_node_id: "source:material-lot-record".to_owned(),
        });
        let expanded = evaluate_assurance_delta_impact_v0_1(&input(expanded_graph));
        assert!(expanded.affected_obligations.len() >= baseline.affected_obligations.len());
        assert!(expanded
            .affected_obligations
            .iter()
            .any(|item| item.obligation_id == "obligation:solver-validation"));
    }

    #[test]
    fn graph_commitment_mutation_fails_closed() {
        let mut input = input(graph());
        input.graph.nodes[0].content_sha256 = "e".repeat(64);
        let assessment = evaluate_assurance_delta_impact_v0_1(&input);
        assert_eq!(assessment.disposition, AssuranceDeltaDispositionV0_1::InvalidInput);
        assert!(assessment
            .reason_codes
            .contains(&"AERO_DELTA_GRAPH_COMMITMENT_MISMATCH".to_owned()));
    }
}
