// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001A pure six-scope reference fixture.
//!
//! This first cell proves observation/presentation isolation only. It deliberately
//! does not implement canonical refinement, collapse, fine mutation, persistence,
//! frames, or Bevy presentation.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{ContractError, ScopeId, TypedDigest32, WorldInstanceId};

const CELL_DOMAIN: &[u8] = b"symtropy.scale-cell-001a.world.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScaleKind {
    System,
    Body,
    Region,
    Local,
    Structure,
    Micro,
}

impl ScaleKind {
    const fn code(self) -> u8 {
        match self {
            Self::System => 0,
            Self::Body => 1,
            Self::Region => 2,
            Self::Local => 3,
            Self::Structure => 4,
            Self::Micro => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalNode {
    scope: ScopeId,
    parent: Option<ScopeId>,
    scale: ScaleKind,
    aggregate: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaleCell {
    world: WorldInstanceId,
    nodes: BTreeMap<ScopeId, CanonicalNode>,
}

impl ScaleCell {
    fn new(world: WorldInstanceId, nodes: Vec<CanonicalNode>) -> Result<Self, RefError> {
        world.validate()?;
        if nodes.len() != 6 {
            return Err(RefError::WrongNodeCount);
        }

        let mut by_scope = BTreeMap::new();
        for node in nodes {
            node.scope.validate()?;
            if let Some(parent) = &node.parent {
                parent.validate()?;
            }
            if by_scope.insert(node.scope.clone(), node).is_some() {
                return Err(RefError::DuplicateScope);
            }
        }

        let cell = Self {
            world,
            nodes: by_scope,
        };
        cell.validate()?;
        Ok(cell)
    }

    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        if self.nodes.len() != 6 {
            return Err(RefError::WrongNodeCount);
        }

        let roots: Vec<_> = self
            .nodes
            .values()
            .filter(|node| node.parent.is_none())
            .collect();
        if roots.len() != 1 || roots[0].scale != ScaleKind::System {
            return Err(RefError::InvalidRoot);
        }

        for node in self.nodes.values() {
            node.scope.validate()?;
            if let Some(parent) = &node.parent {
                let parent_node = self.nodes.get(parent).ok_or(RefError::UnknownParent)?;
                if parent_node.scale.code() + 1 != node.scale.code() {
                    return Err(RefError::InvalidScaleEdge);
                }
            }
        }
        Ok(())
    }

    fn lookup(&self, scope: &ScopeId) -> Result<Lookup, RefError> {
        scope.validate()?;
        Ok(match self.nodes.get(scope) {
            Some(node) => Lookup::Present {
                scope: node.scope.clone(),
                parent: node.parent.clone(),
                scale: node.scale,
            },
            None => Lookup::QualifiedNoScope,
        })
    }

    fn direct_children(&self, parent: &ScopeId) -> Result<Vec<ScopeId>, RefError> {
        parent.validate()?;
        if !self.nodes.contains_key(parent) {
            return Err(RefError::UnknownScope);
        }
        Ok(self
            .nodes
            .values()
            .filter(|node| node.parent.as_ref() == Some(parent))
            .map(|node| node.scope.clone())
            .collect())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CELL_DOMAIN);
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&(self.nodes.len() as u32).to_le_bytes());
        for node in self.nodes.values() {
            push_string(&mut bytes, node.scope.as_str());
            match &node.parent {
                Some(parent) => {
                    bytes.push(1);
                    push_string(&mut bytes, parent.as_str());
                }
                None => bytes.push(0),
            }
            bytes.push(node.scale.code());
            bytes.extend_from_slice(&node.aggregate.to_le_bytes());
        }
        Ok(bytes)
    }

    fn commitment(&self) -> Result<TypedDigest32, RefError> {
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001a.world-commitment.v1",
            1,
            &self.canonical_bytes()?,
        )?)
    }

    fn observe_scope(
        &self,
        presentation: &mut PresentationState,
        requested: &ScopeId,
        profile: ObservationProfile,
    ) -> Result<ObservationReceipt, RefError> {
        let node = self.nodes.get(requested).ok_or(RefError::UnknownScope)?;
        let source_commitment = self.commitment()?;
        if profile.first_bucket > node.aggregate {
            return Err(RefError::InvalidObservationProfile);
        }

        let derived = DerivedView {
            known_total: node.aggregate,
            conditional_detail: [profile.first_bucket, node.aggregate - profile.first_bucket],
            provenance: DetailProvenance::DerivedConditional,
            source_commitment: source_commitment.clone(),
        };
        let view_digest = derived.digest(&node.scope)?;

        presentation.resident.insert(node.scope.clone());
        presentation.viewed = Some(node.scope.clone());
        presentation.derived.insert(node.scope.clone(), derived);

        Ok(ObservationReceipt {
            scope: node.scope.clone(),
            source_commitment,
            view_digest,
        })
    }

    fn request_process(
        &self,
        requested: &ScopeId,
        information_profile: &str,
    ) -> Result<ProcessDemand, RefError> {
        requested.validate()?;
        if !self.nodes.contains_key(requested) {
            return Err(RefError::UnknownScope);
        }
        let profile = TypedDigest32::sha256(
            "symtropy.scale-cell-001a.process-information-profile.v1",
            1,
            information_profile.as_bytes(),
        )?;
        Ok(ProcessDemand::RequiresSemanticRefinement {
            scope: requested.clone(),
            source_commitment: self.commitment()?,
            information_profile: profile,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Lookup {
    Present {
        scope: ScopeId,
        parent: Option<ScopeId>,
        scale: ScaleKind,
    },
    QualifiedNoScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservationProfile {
    first_bucket: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailProvenance {
    DerivedConditional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedView {
    known_total: u64,
    conditional_detail: [u64; 2],
    provenance: DetailProvenance,
    source_commitment: TypedDigest32,
}

impl DerivedView {
    fn digest(&self, scope: &ScopeId) -> Result<TypedDigest32, RefError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001a.derived-view.v1\0");
        push_string(&mut bytes, scope.as_str());
        bytes.extend_from_slice(&self.known_total.to_le_bytes());
        bytes.extend_from_slice(&self.conditional_detail[0].to_le_bytes());
        bytes.extend_from_slice(&self.conditional_detail[1].to_le_bytes());
        bytes.push(match self.provenance {
            DetailProvenance::DerivedConditional => 1,
        });
        bytes.extend_from_slice(&self.source_commitment.value);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001a.derived-view.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PresentationState {
    resident: BTreeSet<ScopeId>,
    viewed: Option<ScopeId>,
    derived: BTreeMap<ScopeId, DerivedView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationReceipt {
    scope: ScopeId,
    source_commitment: TypedDigest32,
    view_digest: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessDemand {
    RequiresSemanticRefinement {
        scope: ScopeId,
        source_commitment: TypedDigest32,
        information_profile: TypedDigest32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    WrongNodeCount,
    DuplicateScope,
    InvalidRoot,
    UnknownParent,
    InvalidScaleEdge,
    UnknownScope,
    InvalidObservationProfile,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn node(id: &str, parent: Option<&str>, scale: ScaleKind, aggregate: u64) -> CanonicalNode {
    CanonicalNode {
        scope: scope(id),
        parent: parent.map(scope),
        scale,
        aggregate,
    }
}

fn fixture() -> ScaleCell {
    ScaleCell::new(
        world("world:scale-cell-001"),
        vec![
            node("system:fixture", None, ScaleKind::System, 100),
            node(
                "body:a",
                Some("system:fixture"),
                ScaleKind::Body,
                100,
            ),
            node("region:r0", Some("body:a"), ScaleKind::Region, 100),
            node("local:l0", Some("region:r0"), ScaleKind::Local, 100),
            node(
                "structure:s0",
                Some("local:l0"),
                ScaleKind::Structure,
                100,
            ),
            node(
                "micro:m0",
                Some("structure:s0"),
                ScaleKind::Micro,
                100,
            ),
        ],
    )
    .unwrap()
}

fn traversal() -> Vec<ScopeId> {
    [
        "system:fixture",
        "body:a",
        "region:r0",
        "local:l0",
        "structure:s0",
        "micro:m0",
        "structure:s0",
        "local:l0",
        "region:r0",
        "body:a",
        "system:fixture",
    ]
    .into_iter()
    .map(scope)
    .collect()
}

fn one_way_traversal() -> Vec<ScopeId> {
    [
        "system:fixture",
        "body:a",
        "region:r0",
        "local:l0",
        "structure:s0",
        "micro:m0",
    ]
    .into_iter()
    .map(scope)
    .collect()
}

#[test]
fn six_scope_hierarchy_is_exact_and_direct_children_are_sparse() {
    let cell = fixture();
    cell.validate().unwrap();

    assert_eq!(
        cell.direct_children(&scope("system:fixture")).unwrap(),
        vec![scope("body:a")]
    );
    assert_eq!(
        cell.direct_children(&scope("structure:s0")).unwrap(),
        vec![scope("micro:m0")]
    );

    let Lookup::Present {
        scope: found,
        parent,
        scale,
    } = cell.lookup(&scope("micro:m0")).unwrap()
    else {
        panic!("micro scope must be present");
    };
    assert_eq!(found, scope("micro:m0"));
    assert_eq!(parent, Some(scope("structure:s0")));
    assert_eq!(scale, ScaleKind::Micro);

    assert_eq!(
        cell.lookup(&scope("missing:scope")).unwrap(),
        Lookup::QualifiedNoScope
    );
}

#[test]
fn nonresident_scope_remains_present_in_world_catalog() {
    let cell = fixture();
    let presentation = PresentationState::default();

    assert!(!presentation.resident.contains(&scope("micro:m0")));
    assert!(matches!(
        cell.lookup(&scope("micro:m0")).unwrap(),
        Lookup::Present { .. }
    ));
}

#[test]
fn macro_to_micro_to_macro_observation_is_canonically_invariant() {
    let control = fixture();
    let experiment = fixture();
    let before = experiment.commitment().unwrap();
    let control_final = control.commitment().unwrap();
    let mut presentation = PresentationState::default();

    for requested in traversal() {
        let receipt = experiment
            .observe_scope(
                &mut presentation,
                &requested,
                ObservationProfile { first_bucket: 60 },
            )
            .unwrap();
        assert_eq!(receipt.source_commitment, before);
        assert_eq!(receipt.scope, requested);
        receipt.view_digest.validate().unwrap();
    }

    assert_eq!(experiment.commitment().unwrap(), before);
    assert_eq!(experiment.commitment().unwrap(), control_final);
    assert_eq!(presentation.resident.len(), 6);
    assert_eq!(presentation.viewed, Some(scope("system:fixture")));
}

#[test]
fn different_observation_detail_changes_view_not_world_truth() {
    let cell = fixture();
    let canonical = cell.commitment().unwrap();
    let mut first = PresentationState::default();
    let mut second = PresentationState::default();

    let a = cell
        .observe_scope(
            &mut first,
            &scope("micro:m0"),
            ObservationProfile { first_bucket: 60 },
        )
        .unwrap();
    let b = cell
        .observe_scope(
            &mut second,
            &scope("micro:m0"),
            ObservationProfile { first_bucket: 55 },
        )
        .unwrap();

    assert_ne!(a.view_digest, b.view_digest);
    assert_eq!(a.source_commitment, canonical);
    assert_eq!(b.source_commitment, canonical);
    assert_eq!(cell.commitment().unwrap(), canonical);

    let first_view = first.derived.get(&scope("micro:m0")).unwrap();
    let second_view = second.derived.get(&scope("micro:m0")).unwrap();
    assert_eq!(first_view.known_total, 100);
    assert_eq!(second_view.known_total, 100);
    assert_eq!(first_view.provenance, DetailProvenance::DerivedConditional);
    assert_eq!(second_view.provenance, DetailProvenance::DerivedConditional);
}

#[test]
fn observe_scope_does_not_activate_a_canonical_process() {
    let cell = fixture();
    let before = cell.commitment().unwrap();
    let mut presentation = PresentationState::default();
    let receipt = cell
        .observe_scope(
            &mut presentation,
            &scope("micro:m0"),
            ObservationProfile { first_bucket: 60 },
        )
        .unwrap();

    assert_eq!(receipt.source_commitment, before);
    assert_eq!(cell.commitment().unwrap(), before);

    let demand = cell
        .request_process(&scope("micro:m0"), "micro-exact-process-v1")
        .unwrap();
    let ProcessDemand::RequiresSemanticRefinement {
        scope: demanded_scope,
        source_commitment,
        information_profile,
    } = demand;
    assert_eq!(demanded_scope, scope("micro:m0"));
    assert_eq!(source_commitment, before);
    information_profile.validate().unwrap();
    assert_eq!(cell.commitment().unwrap(), before);
}

#[test]
fn presentation_residency_and_view_order_are_outside_world_commitment() {
    let cell = fixture();
    let canonical = cell.commitment().unwrap();
    let mut a = PresentationState::default();
    let mut b = PresentationState::default();
    let path = one_way_traversal();

    for requested in &path {
        cell.observe_scope(
            &mut a,
            requested,
            ObservationProfile { first_bucket: 60 },
        )
        .unwrap();
    }
    for requested in path.iter().rev() {
        cell.observe_scope(
            &mut b,
            requested,
            ObservationProfile { first_bucket: 60 },
        )
        .unwrap();
    }

    assert_ne!(a.viewed, b.viewed);
    assert_eq!(a.resident, b.resident);
    assert_eq!(cell.commitment().unwrap(), canonical);
}

#[test]
fn invalid_observation_detail_fails_without_changing_world_or_presentation() {
    let cell = fixture();
    let canonical = cell.commitment().unwrap();
    let mut presentation = PresentationState::default();

    assert_eq!(
        cell.observe_scope(
            &mut presentation,
            &scope("micro:m0"),
            ObservationProfile { first_bucket: 101 },
        ),
        Err(RefError::InvalidObservationProfile)
    );
    assert!(presentation.resident.is_empty());
    assert!(presentation.derived.is_empty());
    assert_eq!(cell.commitment().unwrap(), canonical);
}

#[test]
fn canonical_truth_change_changes_commitment_but_view_change_does_not() {
    let original = fixture();
    let original_commitment = original.commitment().unwrap();
    let mut changed = original.clone();
    changed.nodes.get_mut(&scope("micro:m0")).unwrap().aggregate = 99;

    assert_ne!(changed.commitment().unwrap(), original_commitment);

    let mut presentation = PresentationState::default();
    original
        .observe_scope(
            &mut presentation,
            &scope("micro:m0"),
            ObservationProfile { first_bucket: 1 },
        )
        .unwrap();
    assert_eq!(original.commitment().unwrap(), original_commitment);
}
