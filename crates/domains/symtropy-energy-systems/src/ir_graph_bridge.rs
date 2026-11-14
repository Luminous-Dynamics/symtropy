// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit, fail-closed bridge from validated Power Fabric IR to ServiceGraph.
//! It never infers semantic mappings or promotes claim maturity automatically.

use crate::{
    ClaimDomain, ClaimState, GraphEvidence, GraphEvidenceState, GraphMaturity,
    GraphRevision, IrObject, IrRelationship, NamespacedType, PowerFabricDocument,
    RelationshipDirection, ServiceEdge, ServiceEdgeKind, ServiceGraph,
    ModuleIdentity, ServiceGraphError, ServiceNode, ServiceNodeKind,
};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphCompileContext {
    graph_id: String,
    graph_revision_id: String,
    graph_content_digest: String,
    scope_statement: String,
    maturity: GraphMaturity,
    maturity_evidence: GraphEvidence,
    current: bool,
}

impl GraphCompileContext {
    pub fn new(
        graph_id: impl Into<String>,
        graph_revision_id: impl Into<String>,
        graph_content_digest: impl Into<String>,
        scope_statement: impl Into<String>,
        maturity: GraphMaturity,
        maturity_evidence_state: GraphEvidenceState,
        maturity_evidence_id: Option<String>,
        current: bool,
    ) -> Result<Self, GraphBridgeError> {
        let graph_id = graph_id.into();
        let graph_revision_id = graph_revision_id.into();
        let graph_content_digest = graph_content_digest.into();
        let scope_statement = scope_statement.into();
        for (field, value) in [
            ("graph_id", graph_id.as_str()),
            ("graph_revision_id", graph_revision_id.as_str()),
            ("graph_content_digest", graph_content_digest.as_str()),
            ("scope_statement", scope_statement.as_str()),
        ] {
            require_non_empty(field, value)?;
        }
        let maturity_evidence = GraphEvidence::new(maturity_evidence_state, maturity_evidence_id)
            .map_err(GraphBridgeError::ServiceGraph)?;
        Ok(Self {
            graph_id, graph_revision_id, graph_content_digest, scope_statement,
            maturity, maturity_evidence, current,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrGraphMappingProfile {
    profile_id: String,
    profile_version: String,
    #[serde(serialize_with = "serialize_object_kind_map")]
    object_kind_map: BTreeMap<NamespacedType, ServiceNodeKind>,
    #[serde(serialize_with = "serialize_relationship_kind_map")]
    relationship_kind_map: BTreeMap<NamespacedType, ServiceEdgeKind>,
    #[serde(serialize_with = "serialize_domain_evidence_map")]
    domain_evidence_map: BTreeMap<ClaimDomain, GraphEvidenceState>,
    object_evidence_claim_ids: BTreeMap<String, String>,
    object_module_identities: BTreeMap<String, ModuleIdentity>,
}

impl IrGraphMappingProfile {
    pub fn new(
        profile_id: impl Into<String>,
        profile_version: impl Into<String>,
        object_kind_map: BTreeMap<NamespacedType, ServiceNodeKind>,
        relationship_kind_map: BTreeMap<NamespacedType, ServiceEdgeKind>,
        domain_evidence_map: BTreeMap<ClaimDomain, GraphEvidenceState>,
        object_evidence_claim_ids: BTreeMap<String, String>,
        object_module_identities: BTreeMap<String, ModuleIdentity>,
    ) -> Result<Self, GraphBridgeError> {
        let profile_id = profile_id.into();
        let profile_version = profile_version.into();
        require_non_empty("profile_id", &profile_id)?;
        require_non_empty("profile_version", &profile_version)?;
        if object_kind_map.is_empty() { return Err(GraphBridgeError::EmptyObjectKindMap); }
        if relationship_kind_map.is_empty() { return Err(GraphBridgeError::EmptyRelationshipKindMap); }
        for (domain, state) in &domain_evidence_map {
            if *state == GraphEvidenceState::Unknown {
                return Err(GraphBridgeError::UnknownEvidenceStateMapping { domain: *domain });
            }
        }
        for (object_id, claim_id) in &object_evidence_claim_ids {
            require_non_empty("evidence_selection_object_id", object_id)?;
            require_non_empty("evidence_selection_claim_id", claim_id)?;
        }
        for object_id in object_module_identities.keys() {
            require_non_empty("module_binding_object_id", object_id)?;
        }
        Ok(Self {
            profile_id, profile_version, object_kind_map, relationship_kind_map,
            domain_evidence_map, object_evidence_claim_ids, object_module_identities,
        })
    }

    pub fn profile_id(&self) -> &str { &self.profile_id }
    pub fn profile_version(&self) -> &str { &self.profile_version }
}


#[derive(Serialize)]
struct ObjectKindMapEntry<'a> {
    source: &'a NamespacedType,
    target: ServiceNodeKind,
}

fn serialize_object_kind_map<S>(
    value: &BTreeMap<NamespacedType, ServiceNodeKind>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.iter()
        .map(|(source, target)| ObjectKindMapEntry { source, target: *target })
        .collect::<Vec<_>>()
        .serialize(serializer)
}

#[derive(Serialize)]
struct RelationshipKindMapEntry<'a> {
    source: &'a NamespacedType,
    target: ServiceEdgeKind,
}

fn serialize_relationship_kind_map<S>(
    value: &BTreeMap<NamespacedType, ServiceEdgeKind>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.iter()
        .map(|(source, target)| RelationshipKindMapEntry { source, target: *target })
        .collect::<Vec<_>>()
        .serialize(serializer)
}

#[derive(Serialize)]
struct DomainEvidenceMapEntry {
    source: ClaimDomain,
    target: GraphEvidenceState,
}

fn serialize_domain_evidence_map<S>(
    value: &BTreeMap<ClaimDomain, GraphEvidenceState>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.iter()
        .map(|(source, target)| DomainEvidenceMapEntry { source: *source, target: *target })
        .collect::<Vec<_>>()
        .serialize(serializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectMappingReceipt {
    object_id: String,
    source_kind: NamespacedType,
    target_kind: ServiceNodeKind,
    selected_evidence_claim_id: Option<String>,
    evidence_state: GraphEvidenceState,
    module_identity: Option<ModuleIdentity>,
}
impl ObjectMappingReceipt {
    pub fn object_id(&self) -> &str { &self.object_id }
    pub fn source_kind(&self) -> &NamespacedType { &self.source_kind }
    pub const fn target_kind(&self) -> ServiceNodeKind { self.target_kind }
    pub fn selected_evidence_claim_id(&self) -> Option<&str> { self.selected_evidence_claim_id.as_deref() }
    pub const fn evidence_state(&self) -> GraphEvidenceState { self.evidence_state }
    pub fn module_identity(&self) -> Option<&ModuleIdentity> { self.module_identity.as_ref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationshipMappingReceipt {
    relationship_id: String,
    source_kind: NamespacedType,
    target_kind: ServiceEdgeKind,
    source_direction: RelationshipDirection,
    evidence_state: GraphEvidenceState,
}
impl RelationshipMappingReceipt {
    pub fn relationship_id(&self) -> &str { &self.relationship_id }
    pub fn source_kind(&self) -> &NamespacedType { &self.source_kind }
    pub const fn target_kind(&self) -> ServiceEdgeKind { self.target_kind }
    pub const fn source_direction(&self) -> RelationshipDirection { self.source_direction }
    pub const fn evidence_state(&self) -> GraphEvidenceState { self.evidence_state }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGapReason {
    NoEvidenceClaimSelected,
    ClaimUnknown,
    ClaimUnavailable,
    ClaimNotApplicable,
    ClaimHasNoEvidenceReference,
    ClaimDomainNotMapped,
    RelationshipHasNoEvidenceReference,
    RelationshipDomainNotMapped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceGap {
    entity_id: String,
    reason: EvidenceGapReason,
    selected_claim_id: Option<String>,
}
impl EvidenceGap {
    pub fn entity_id(&self) -> &str { &self.entity_id }
    pub const fn reason(&self) -> EvidenceGapReason { self.reason }
    pub fn selected_claim_id(&self) -> Option<&str> { self.selected_claim_id.as_deref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrGraphCompilationReport {
    document_id: String,
    document_revision_id: String,
    mapping_profile_id: String,
    mapping_profile_version: String,
    graph_id: String,
    graph_revision_id: String,
    graph_content_digest: String,
    object_mappings: Vec<ObjectMappingReceipt>,
    relationship_mappings: Vec<RelationshipMappingReceipt>,
    evidence_gaps: Vec<EvidenceGap>,
}
impl IrGraphCompilationReport {
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn document_revision_id(&self) -> &str { &self.document_revision_id }
    pub fn mapping_profile_id(&self) -> &str { &self.mapping_profile_id }
    pub fn mapping_profile_version(&self) -> &str { &self.mapping_profile_version }
    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn graph_revision_id(&self) -> &str { &self.graph_revision_id }
    pub fn graph_content_digest(&self) -> &str { &self.graph_content_digest }
    pub fn object_mappings(&self) -> &[ObjectMappingReceipt] { &self.object_mappings }
    pub fn relationship_mappings(&self) -> &[RelationshipMappingReceipt] { &self.relationship_mappings }
    pub fn evidence_gaps(&self) -> &[EvidenceGap] { &self.evidence_gaps }
    pub const fn has_evidence_gaps(&self) -> bool { !self.evidence_gaps.is_empty() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrGraphCompilation {
    graph: ServiceGraph,
    report: IrGraphCompilationReport,
}
impl IrGraphCompilation {
    pub fn graph(&self) -> &ServiceGraph { &self.graph }
    pub fn report(&self) -> &IrGraphCompilationReport { &self.report }
}

pub fn compile_ir_to_service_graph(
    document: &PowerFabricDocument,
    profile: &IrGraphMappingProfile,
    context: GraphCompileContext,
) -> Result<IrGraphCompilation, GraphBridgeError> {
    for object_id in profile.object_module_identities.keys() {
        if !document.objects().contains_key(object_id) {
            return Err(GraphBridgeError::ModuleBindingObjectUnknown { object_id: object_id.clone() });
        }
    }
    for (object_id, claim_id) in &profile.object_evidence_claim_ids {
        let object = document.objects().get(object_id).ok_or_else(|| {
            GraphBridgeError::EvidenceSelectionObjectUnknown { object_id: object_id.clone() }
        })?;
        if !object.claims().contains_key(claim_id) {
            return Err(GraphBridgeError::EvidenceSelectionClaimUnknown {
                object_id: object_id.clone(), claim_id: claim_id.clone(),
            });
        }
    }

    let mut nodes = Vec::with_capacity(document.objects().len());
    let mut object_mappings = Vec::with_capacity(document.objects().len());
    let mut evidence_gaps = Vec::new();
    for (object_id, object) in document.objects() {
        let target_kind = *profile.object_kind_map.get(object.kind()).ok_or_else(|| {
            GraphBridgeError::UnmappedObjectKind {
                object_id: object_id.clone(),
                namespace: object.kind().namespace().to_owned(),
                local_name: object.kind().local_name().to_owned(),
            }
        })?;
        let (evidence, selected_claim_id, gap) = map_object_evidence(object, object_id, profile)?;
        if let Some(gap) = gap { evidence_gaps.push(gap); }
        let mut node = ServiceNode::new(
            object.identity().stable_id(), target_kind, object.identity().stable_id(), evidence.clone(),
        ).map_err(GraphBridgeError::ServiceGraph)?;
        let module_identity = profile.object_module_identities.get(object_id).cloned();
        if let Some(identity) = module_identity.clone() {
            node = node.with_module_identity(identity);
        }
        nodes.push(node);
        object_mappings.push(ObjectMappingReceipt {
            object_id: object_id.clone(),
            source_kind: object.kind().clone(),
            target_kind,
            selected_evidence_claim_id: selected_claim_id,
            evidence_state: evidence.state(),
            module_identity,
        });
    }

    let mut edges = Vec::with_capacity(document.relationships().len());
    let mut relationship_mappings = Vec::with_capacity(document.relationships().len());
    for (relationship_id, relationship) in document.relationships() {
        let target_kind = *profile.relationship_kind_map.get(relationship.kind()).ok_or_else(|| {
            GraphBridgeError::UnmappedRelationshipKind {
                relationship_id: relationship_id.clone(),
                namespace: relationship.kind().namespace().to_owned(),
                local_name: relationship.kind().local_name().to_owned(),
            }
        })?;
        if relationship.direction() != RelationshipDirection::Directed {
            return Err(GraphBridgeError::UnsupportedRelationshipDirection {
                relationship_id: relationship_id.clone(),
                direction: relationship.direction(),
            });
        }
        let (evidence, gap) = map_relationship_evidence(relationship, profile)?;
        if let Some(gap) = gap { evidence_gaps.push(gap); }
        edges.push(ServiceEdge::new(
            relationship.relationship_id(),
            relationship.from_object_id(),
            relationship.to_object_id(),
            target_kind,
            evidence.clone(),
        ).map_err(GraphBridgeError::ServiceGraph)?);
        relationship_mappings.push(RelationshipMappingReceipt {
            relationship_id: relationship_id.clone(),
            source_kind: relationship.kind().clone(),
            target_kind,
            source_direction: relationship.direction(),
            evidence_state: evidence.state(),
        });
    }

    require_graph_maturity(context.maturity, &nodes, &edges)?;
    let revision = GraphRevision::new(
        context.graph_id.clone(), context.graph_revision_id.clone(),
        context.graph_content_digest.clone(), context.scope_statement,
        context.maturity, context.maturity_evidence, context.current,
    ).map_err(GraphBridgeError::ServiceGraph)?;
    let graph = ServiceGraph::new(revision, nodes, edges).map_err(GraphBridgeError::ServiceGraph)?;

    object_mappings.sort_by(|a,b| a.object_id.cmp(&b.object_id));
    relationship_mappings.sort_by(|a,b| a.relationship_id.cmp(&b.relationship_id));
    evidence_gaps.sort_by(|a,b| a.entity_id.cmp(&b.entity_id).then_with(|| (a.reason as u8).cmp(&(b.reason as u8))));

    let report = IrGraphCompilationReport {
        document_id: document.document_id().to_owned(),
        document_revision_id: document.revision_id().to_owned(),
        mapping_profile_id: profile.profile_id.clone(),
        mapping_profile_version: profile.profile_version.clone(),
        graph_id: context.graph_id,
        graph_revision_id: context.graph_revision_id,
        graph_content_digest: context.graph_content_digest,
        object_mappings,
        relationship_mappings,
        evidence_gaps,
    };
    Ok(IrGraphCompilation { graph, report })
}

fn map_object_evidence(
    object: &IrObject,
    object_id: &str,
    profile: &IrGraphMappingProfile,
) -> Result<(GraphEvidence, Option<String>, Option<EvidenceGap>), GraphBridgeError> {
    let Some(claim_id) = profile.object_evidence_claim_ids.get(object_id) else {
        return Ok((GraphEvidence::unknown(), None, Some(EvidenceGap {
            entity_id: object_id.to_owned(), reason: EvidenceGapReason::NoEvidenceClaimSelected,
            selected_claim_id: None,
        })));
    };
    let claim = object.claims().get(claim_id).ok_or_else(|| GraphBridgeError::EvidenceSelectionClaimUnknown {
        object_id: object_id.to_owned(), claim_id: claim_id.clone(),
    })?;
    let gap = |reason| Some(EvidenceGap {
        entity_id: object_id.to_owned(), reason, selected_claim_id: Some(claim_id.clone()),
    });
    match claim.state() {
        ClaimState::Unknown => Ok((GraphEvidence::unknown(), Some(claim_id.clone()), gap(EvidenceGapReason::ClaimUnknown))),
        ClaimState::Unavailable => Ok((GraphEvidence::unknown(), Some(claim_id.clone()), gap(EvidenceGapReason::ClaimUnavailable))),
        ClaimState::NotApplicable => Ok((GraphEvidence::unknown(), Some(claim_id.clone()), gap(EvidenceGapReason::ClaimNotApplicable))),
        ClaimState::Contradictory { .. } => Err(GraphBridgeError::ContradictoryEvidenceClaim {
            object_id: object_id.to_owned(), claim_id: claim_id.clone(),
        }),
        ClaimState::Value(_) => {
            let Some(evidence_id) = claim.provenance().evidence_id() else {
                return Ok((GraphEvidence::unknown(), Some(claim_id.clone()), gap(EvidenceGapReason::ClaimHasNoEvidenceReference)));
            };
            let Some(state) = profile.domain_evidence_map.get(&claim.domain()).copied() else {
                return Ok((GraphEvidence::unknown(), Some(claim_id.clone()), gap(EvidenceGapReason::ClaimDomainNotMapped)));
            };
            let evidence = GraphEvidence::new(state, Some(evidence_id.to_owned())).map_err(GraphBridgeError::ServiceGraph)?;
            Ok((evidence, Some(claim_id.clone()), None))
        }
    }
}

fn map_relationship_evidence(
    relationship: &IrRelationship,
    profile: &IrGraphMappingProfile,
) -> Result<(GraphEvidence, Option<EvidenceGap>), GraphBridgeError> {
    let gap = |reason| Some(EvidenceGap {
        entity_id: relationship.relationship_id().to_owned(), reason, selected_claim_id: None,
    });
    let Some(evidence_id) = relationship.provenance().evidence_id() else {
        return Ok((GraphEvidence::unknown(), gap(EvidenceGapReason::RelationshipHasNoEvidenceReference)));
    };
    let Some(state) = profile.domain_evidence_map.get(&relationship.domain()).copied() else {
        return Ok((GraphEvidence::unknown(), gap(EvidenceGapReason::RelationshipDomainNotMapped)));
    };
    let evidence = GraphEvidence::new(state, Some(evidence_id.to_owned())).map_err(GraphBridgeError::ServiceGraph)?;
    Ok((evidence, None))
}

fn require_graph_maturity(
    maturity: GraphMaturity,
    nodes: &[ServiceNode],
    edges: &[ServiceEdge],
) -> Result<(), GraphBridgeError> {
    if maturity == GraphMaturity::DesignOnly { return Ok(()); }
    for (entity_id, state) in nodes.iter().map(|n| (n.node_id(), n.evidence().state()))
        .chain(edges.iter().map(|e| (e.edge_id(), e.evidence().state())))
    {
        let enough = match maturity {
            GraphMaturity::DesignOnly => true,
            GraphMaturity::FieldReconciled => matches!(state, GraphEvidenceState::FieldObserved | GraphEvidenceState::IndependentlyReviewed),
            GraphMaturity::IndependentlyReviewed => state == GraphEvidenceState::IndependentlyReviewed,
        };
        if !enough {
            return Err(GraphBridgeError::EvidenceInsufficientForGraphMaturity {
                entity_id: entity_id.to_owned(), maturity, evidence_state: state,
            });
        }
    }
    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), GraphBridgeError> {
    if value.trim().is_empty() { Err(GraphBridgeError::EmptyField { field }) } else { Ok(()) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphBridgeError {
    EmptyField { field: &'static str },
    EmptyObjectKindMap,
    EmptyRelationshipKindMap,
    UnknownEvidenceStateMapping { domain: ClaimDomain },
    EvidenceSelectionObjectUnknown { object_id: String },
    EvidenceSelectionClaimUnknown { object_id: String, claim_id: String },
    ModuleBindingObjectUnknown { object_id: String },
    UnmappedObjectKind { object_id: String, namespace: String, local_name: String },
    UnmappedRelationshipKind { relationship_id: String, namespace: String, local_name: String },
    UnsupportedRelationshipDirection { relationship_id: String, direction: RelationshipDirection },
    ContradictoryEvidenceClaim { object_id: String, claim_id: String },
    EvidenceInsufficientForGraphMaturity { entity_id: String, maturity: GraphMaturity, evidence_state: GraphEvidenceState },
    ServiceGraph(ServiceGraphError),
}
impl core::fmt::Display for GraphBridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for GraphBridgeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildMetadata, ClaimProvenance, IrObject, IrRelationship, IrSchemaVersion,
        ObjectIdentity, PropertyClaim, PropertyValue, Sha256Digest,
        StandardProfilePin, StandardPublicationStatus, StandardReference,
        UpstreamArtifactPin, compile_ir_to_service_graph_reproducibly,
    };
    use std::collections::BTreeMap;

    fn ns(value: &str) -> NamespacedType { NamespacedType::new("urn:pf:test", value).unwrap() }
    fn doc(direction: RelationshipDirection) -> PowerFabricDocument {
        let claim = |id: &str, text: &str| PropertyClaim::new(
            id, "label", ClaimDomain::Declared,
            ClaimState::Value(PropertyValue::Text { value: text.into() }),
            ClaimProvenance::new("fixture", Some(format!("evidence:{id}")), None).unwrap()
        ).unwrap();
        let source = IrObject::new(ObjectIdentity::new("source:grid", BTreeMap::new()).unwrap(), ns("utility_source"), vec![claim("claim:source","Grid")]).unwrap();
        let load = IrObject::new(ObjectIdentity::new("load:lighting", BTreeMap::new()).unwrap(), ns("load"), vec![claim("claim:load","Lighting")]).unwrap();
        let relationship = IrRelationship::new(
            "edge:grid-lighting", "source:grid", "load:lighting", ns("power_path"),
            direction, ClaimDomain::Declared,
            ClaimProvenance::new("fixture", Some("evidence:path".into()), None).unwrap()
        ).unwrap();
        PowerFabricDocument::new(IrSchemaVersion::current(), "document:1", "revision:ir:1",
            vec![source,load], vec![relationship], Vec::new()).unwrap()
    }
    fn profile() -> IrGraphMappingProfile {
        IrGraphMappingProfile::new("profile:home","0.1",
            BTreeMap::from([(ns("utility_source"),ServiceNodeKind::UtilitySource),(ns("load"),ServiceNodeKind::Load)]),
            BTreeMap::from([(ns("power_path"),ServiceEdgeKind::PowerPath)]),
            BTreeMap::from([(ClaimDomain::Declared,GraphEvidenceState::DesignDeclared)]),
            BTreeMap::from([("source:grid".into(),"claim:source".into()),("load:lighting".into(),"claim:load".into())]),
            BTreeMap::new()
        ).unwrap()
    }
    fn ctx(maturity: GraphMaturity, state: GraphEvidenceState) -> GraphCompileContext {
        GraphCompileContext::new("graph:home","revision:1","sha256:declared","synthetic scope",
            maturity,state,Some("evidence:revision".into()),true).unwrap()
    }


    fn build_metadata(include_amd1: bool) -> BuildMetadata {
        build_metadata_with_lock(include_amd1, b"synthetic Cargo.lock")
    }

    fn build_metadata_with_lock(include_amd1: bool, lock_contents: &[u8]) -> BuildMetadata {
        let standard = StandardReference::new(
            "IEC 60364-8-82", Some("8-82".into()), "2022",
            if include_amd1 { vec!["AMD1:2026".into()] } else { Vec::new() },
            StandardPublicationStatus::Published, None,
        ).unwrap();
        let standard_pin = StandardProfilePin::new(
            "profile:prosumer-pei", "0.1", standard,
            Sha256Digest::from_bytes(b"synthetic registry record"),
            Sha256Digest::from_bytes(b"synthetic applicability rules"),
        ).unwrap();
        let upstream = UpstreamArtifactPin::new(
            "artifact:module-catalogue", "revision:catalogue:1",
            "module identity catalogue", Sha256Digest::from_bytes(b"synthetic module catalogue"),
        ).unwrap();
        BuildMetadata::new(
            "symtropy-energy-systems", "0.1.0",
            "rustc:1.96.0;target:x86_64-unknown-linux-gnu",
            Sha256Digest::from_bytes(lock_contents),
            vec![standard_pin], vec![upstream],
        ).unwrap()
    }

    #[test]
    fn mapping_profile_serializes_complex_maps_as_ordered_structured_arrays() {
        let value = serde_json::to_value(profile()).unwrap();
        assert!(value["object_kind_map"].is_array());
        assert!(value["relationship_kind_map"].is_array());
        assert!(value["domain_evidence_map"].is_array());
        assert_eq!(value["object_kind_map"][0]["source"]["namespace"], "urn:pf:test");
    }

    #[test]
    fn reproducible_compilation_emits_stable_digest_bound_build_receipt() {
        let document = doc(RelationshipDirection::Directed);
        let mapping_profile = profile();
        let first = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let second = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let receipt = first.receipt();
        assert_eq!(receipt.receipt_version(), 1);
        assert_eq!(receipt.digest_algorithm(), "sha256");
        assert_eq!(receipt.source_document_revision_id(), "revision:ir:1");
        assert_eq!(receipt.mapping_profile_id(), "profile:home");
        assert_eq!(receipt.graph_revision_id(), "revision:1");
        assert_eq!(receipt.graph_maturity(), GraphMaturity::DesignOnly);
        assert!(receipt.source_document_digest().as_str().starts_with("sha256:"));
        assert!(receipt.mapping_profile_digest().as_str().starts_with("sha256:"));
        assert!(receipt.service_graph_payload_digest().as_str().starts_with("sha256:"));
        assert!(receipt.build_lock_digest().as_str().starts_with("sha256:"));
        assert_eq!(receipt.build_lock_digest(), second.receipt().build_lock_digest());
        assert_eq!(receipt.standard_profiles().len(), 1);
        assert_eq!(receipt.upstream_artifacts().len(), 1);
    }

    #[test]
    fn changed_standard_profile_changes_build_lock_digest() {
        let document = doc(RelationshipDirection::Directed);
        let mapping_profile = profile();
        let a = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let b = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(false),
        ).unwrap();
        assert_ne!(a.receipt().build_lock_digest(), b.receipt().build_lock_digest());
    }

    #[test]
    fn changed_mapping_profile_changes_profile_and_build_lock_digests() {
        let document = doc(RelationshipDirection::Directed);
        let original_profile = profile();
        let mut changed_profile = profile();
        changed_profile.domain_evidence_map.insert(
            ClaimDomain::Declared, GraphEvidenceState::FieldObserved,
        );
        let original = compile_ir_to_service_graph_reproducibly(
            &document, &original_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let changed = compile_ir_to_service_graph_reproducibly(
            &document, &changed_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        assert_ne!(original.receipt().mapping_profile_digest(), changed.receipt().mapping_profile_digest());
        assert_ne!(original.receipt().build_lock_digest(), changed.receipt().build_lock_digest());
    }


    #[test]
    fn changed_module_configuration_changes_mapping_graph_and_build_lock_digests() {
        let document = doc(RelationshipDirection::Directed);
        let original_profile = profile();
        let mut changed_profile = profile();
        changed_profile.object_module_identities.insert(
            "source:grid".into(),
            crate::ModuleIdentity::new(
                "module:grid-inverter",
                "hardware:R2",
                "sha256:replacement-configuration",
                true,
            ).unwrap(),
        );

        let original = compile_ir_to_service_graph_reproducibly(
            &document,
            &original_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let changed = compile_ir_to_service_graph_reproducibly(
            &document,
            &changed_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();

        assert_eq!(
            original.receipt().source_document_digest(),
            changed.receipt().source_document_digest()
        );
        assert_ne!(
            original.receipt().mapping_profile_digest(),
            changed.receipt().mapping_profile_digest()
        );
        assert_ne!(
            original.receipt().service_graph_payload_digest(),
            changed.receipt().service_graph_payload_digest()
        );
        assert_ne!(
            original.receipt().build_lock_digest(),
            changed.receipt().build_lock_digest()
        );
        assert_eq!(
            changed.compilation().graph().node("source:grid").unwrap()
                .module_identity().unwrap().configuration_digest(),
            "sha256:replacement-configuration"
        );
    }

    #[test]
    fn changed_dependency_lock_digest_changes_build_lock_digest() {
        let document = doc(RelationshipDirection::Directed);
        let mapping_profile = profile();
        let first = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata_with_lock(true, b"Cargo.lock revision A"),
        ).unwrap();
        let second = compile_ir_to_service_graph_reproducibly(
            &document, &mapping_profile,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata_with_lock(true, b"Cargo.lock revision B"),
        ).unwrap();
        assert_ne!(first.receipt().build_lock_digest(), second.receipt().build_lock_digest());
    }

    #[test]
    fn changed_source_revision_changes_source_digest_and_build_lock() {
        let first_document = doc(RelationshipDirection::Directed);
        let source = first_document.objects()["source:grid"].clone();
        let load = first_document.objects()["load:lighting"].clone();
        let edge = first_document.relationships()["edge:grid-lighting"].clone();
        let second_document = PowerFabricDocument::new(
            IrSchemaVersion::current(), "document:1", "revision:ir:changed",
            vec![source, load], vec![edge], Vec::new(),
        ).unwrap();
        let first = compile_ir_to_service_graph_reproducibly(
            &first_document, &profile(),
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        let second = compile_ir_to_service_graph_reproducibly(
            &second_document, &profile(),
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            build_metadata(true),
        ).unwrap();
        assert_ne!(first.receipt().source_document_digest(), second.receipt().source_document_digest());
        assert_ne!(first.receipt().build_lock_digest(), second.receipt().build_lock_digest());
    }

    #[test]
    fn explicitly_mapped_ir_compiles_all_objects_and_edges() {
        let result = compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&profile(),ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)).unwrap();
        assert_eq!(result.graph().nodes().len(),2);
        assert_eq!(result.graph().edges().len(),1);
        assert_eq!(result.report().object_mappings().len(),2);
        assert_eq!(result.report().relationship_mappings().len(),1);
        assert!(!result.report().has_evidence_gaps());
    }
    #[test]
    fn stable_ids_and_revision_provenance_are_preserved() {
        let result = compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&profile(),ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)).unwrap();
        assert!(result.graph().node("source:grid").is_some());
        assert_eq!(result.graph().edges()[0].edge_id(),"edge:grid-lighting");
        assert_eq!(result.report().document_revision_id(),"revision:ir:1");
        assert_eq!(result.report().graph_revision_id(),"revision:1");
    }
    #[test]
    fn unmapped_object_semantics_block_graph_compilation() {
        let mut p=profile(); p.object_kind_map.remove(&ns("utility_source"));
        assert!(matches!(compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&p,ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)),Err(GraphBridgeError::UnmappedObjectKind{..})));
    }
    #[test]
    fn unmapped_relationship_semantics_block_graph_compilation() {
        let mut p=profile(); p.relationship_kind_map.clear();
        assert!(matches!(compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&p,ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)),Err(GraphBridgeError::UnmappedRelationshipKind{..})));
    }
    #[test]
    fn undirected_relationships_are_not_coerced_to_directed_edges() {
        assert!(matches!(compile_ir_to_service_graph(&doc(RelationshipDirection::Undirected),&profile(),ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)),Err(GraphBridgeError::UnsupportedRelationshipDirection{..})));
    }
    #[test]
    fn missing_evidence_selection_is_unknown_and_reported() {
        let mut p=profile(); p.object_evidence_claim_ids.remove("source:grid");
        let r=compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&p,ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)).unwrap();
        assert_eq!(r.graph().node("source:grid").unwrap().evidence().state(),GraphEvidenceState::Unknown);
        assert!(r.report().evidence_gaps().iter().any(|g|g.reason()==EvidenceGapReason::NoEvidenceClaimSelected));
    }
    #[test]
    fn field_reconciled_maturity_rejects_design_declared_node_evidence() {
        assert!(matches!(compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&profile(),ctx(GraphMaturity::FieldReconciled,GraphEvidenceState::FieldObserved)),Err(GraphBridgeError::EvidenceInsufficientForGraphMaturity{..})));
    }
    #[test]
    fn explicit_domain_mapping_controls_evidence_state_without_default_promotion() {
        let mut p=profile(); p.domain_evidence_map.insert(ClaimDomain::Declared,GraphEvidenceState::FieldObserved);
        let r=compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&p,ctx(GraphMaturity::FieldReconciled,GraphEvidenceState::FieldObserved)).unwrap();
        assert_eq!(r.graph().node("source:grid").unwrap().evidence().state(),GraphEvidenceState::FieldObserved);
    }
    #[test]
    fn unknown_claim_state_remains_unknown() {
        let source=IrObject::new(ObjectIdentity::new("source:grid",BTreeMap::new()).unwrap(),ns("utility_source"),
            vec![PropertyClaim::new("claim:source","label",ClaimDomain::Declared,ClaimState::Unknown,
                ClaimProvenance::new("fixture",None,None).unwrap()).unwrap()]).unwrap();
        let load=d.objects()["load:lighting"].clone();
        let edge=d.relationships()["edge:grid-lighting"].clone();
        d=PowerFabricDocument::new(IrSchemaVersion::current(),"document:1","revision:ir:2",vec![source,load],vec![edge],Vec::new()).unwrap();
        let r=compile_ir_to_service_graph(&d,&profile(),ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)).unwrap();
        assert_eq!(r.graph().node("source:grid").unwrap().evidence().state(),GraphEvidenceState::Unknown);
        assert!(r.report().evidence_gaps().iter().any(|g|g.reason()==EvidenceGapReason::ClaimUnknown));
    }

    #[test]
    fn verified_claim_domain_is_not_implicitly_promoted_to_independent_review() {
        let original = doc(RelationshipDirection::Directed);
        let verified_source = IrObject::new(
            ObjectIdentity::new("source:grid", BTreeMap::new()).unwrap(),
            ns("utility_source"),
            vec![PropertyClaim::new(
                "claim:source",
                "label",
                ClaimDomain::Verified,
                ClaimState::Value(PropertyValue::Text { value: "Grid".into() }),
                ClaimProvenance::new("fixture", Some("evidence:verified".into()), None).unwrap(),
            ).unwrap()],
        ).unwrap();
        let load = original.objects()["load:lighting"].clone();
        let edge = original.relationships()["edge:grid-lighting"].clone();
        let changed = PowerFabricDocument::new(
            IrSchemaVersion::current(),
            "document:1",
            "revision:ir:verified",
            vec![verified_source, load],
            vec![edge],
            Vec::new(),
        ).unwrap();

        // The profile maps Declared only. Verified remains unmapped/unknown.
        let result = compile_ir_to_service_graph(
            &changed,
            &profile(),
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
        ).unwrap();
        assert_eq!(
            result.graph().node("source:grid").unwrap().evidence().state(),
            GraphEvidenceState::Unknown
        );
        assert!(result.report().evidence_gaps().iter().any(|gap| {
            gap.reason() == EvidenceGapReason::ClaimDomainNotMapped
                && gap.selected_claim_id() == Some("claim:source")
        }));
    }

    #[test]
    fn relationship_without_evidence_remains_unknown_and_is_reported() {
        let d=doc(RelationshipDirection::Directed);
        let source=d.objects()["source:grid"].clone();
        let load=d.objects()["load:lighting"].clone();
        let edge=IrRelationship::new("edge:grid-lighting","source:grid","load:lighting",ns("power_path"),
            RelationshipDirection::Directed,ClaimDomain::Declared,
            ClaimProvenance::new("fixture",None,None).unwrap()).unwrap();
        let d=PowerFabricDocument::new(IrSchemaVersion::current(),"document:1","revision:ir:3",vec![source,load],vec![edge],Vec::new()).unwrap();
        let r=compile_ir_to_service_graph(&d,&profile(),ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)).unwrap();
        assert_eq!(r.graph().edges()[0].evidence().state(),GraphEvidenceState::Unknown);
        assert!(r.report().evidence_gaps().iter().any(|g|g.reason()==EvidenceGapReason::RelationshipHasNoEvidenceReference));
    }
    #[test]
    fn unknown_claim_domain_evidence_mapping_is_rejected() {
        assert!(matches!(IrGraphMappingProfile::new("p","v",
            BTreeMap::from([(ns("load"),ServiceNodeKind::Load)]),
            BTreeMap::from([(ns("association"),ServiceEdgeKind::PhysicalAssociation)]),
            BTreeMap::from([(ClaimDomain::Declared,GraphEvidenceState::Unknown)]),BTreeMap::new(),BTreeMap::new()),
            Err(GraphBridgeError::UnknownEvidenceStateMapping{..})));
    }

    #[test]
    fn module_configuration_identity_is_preserved_and_stale_state_surfaces() {
        let mut p = profile();
        p.object_module_identities.insert(
            "source:grid".into(),
            ModuleIdentity::new(
                "module:hybrid-inverter",
                "hardware:R2",
                "sha256:old-config",
                false,
            ).unwrap(),
        );
        let result = compile_ir_to_service_graph(
            &doc(RelationshipDirection::Directed),
            &p,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
        ).unwrap();

        let node = result.graph().node("source:grid").unwrap();
        let identity = node.module_identity().unwrap();
        assert_eq!(identity.module_id(), "module:hybrid-inverter");
        assert_eq!(identity.hardware_revision(), "hardware:R2");
        assert_eq!(identity.configuration_digest(), "sha256:old-config");
        assert!(!identity.configuration_current());
        assert_eq!(
            result.report().object_mappings().iter()
                .find(|receipt| receipt.object_id() == "source:grid")
                .unwrap().module_identity().unwrap().module_id(),
            "module:hybrid-inverter"
        );

        let assessment = result.graph().assess_declared_source_paths();
        assert_eq!(assessment.stale_module_configurations().len(), 1);
        assert_eq!(
            assessment.stale_module_configurations()[0].module_id(),
            "module:hybrid-inverter"
        );
    }

    #[test]
    fn module_binding_to_missing_ir_object_blocks_compilation() {
        let mut p = profile();
        p.object_module_identities.insert(
            "asset:missing".into(),
            ModuleIdentity::new("module:missing", "hw:1", "sha256:config", true).unwrap(),
        );
        assert!(matches!(
            compile_ir_to_service_graph(
                &doc(RelationshipDirection::Directed),
                &p,
                ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
            ),
            Err(GraphBridgeError::ModuleBindingObjectUnknown { .. })
        ));
    }

    #[test]
    fn current_module_configuration_is_not_reported_as_stale() {
        let mut p = profile();
        p.object_module_identities.insert(
            "source:grid".into(),
            ModuleIdentity::new("module:inverter", "hw:R3", "sha256:current", true).unwrap(),
        );
        let result = compile_ir_to_service_graph(
            &doc(RelationshipDirection::Directed),
            &p,
            ctx(GraphMaturity::DesignOnly, GraphEvidenceState::DesignDeclared),
        ).unwrap();
        assert!(result.graph().assess_declared_source_paths().stale_module_configurations().is_empty());
    }

    #[test]
    fn invalid_evidence_selection_object_is_rejected() {
        let mut p=profile(); p.object_evidence_claim_ids.insert("missing".into(),"claim".into());
        assert!(matches!(compile_ir_to_service_graph(&doc(RelationshipDirection::Directed),&p,ctx(GraphMaturity::DesignOnly,GraphEvidenceState::DesignDeclared)),Err(GraphBridgeError::EvidenceSelectionObjectUnknown{..})));
    }
}
