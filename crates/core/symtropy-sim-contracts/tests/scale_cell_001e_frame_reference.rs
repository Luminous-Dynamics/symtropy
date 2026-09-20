// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001E exact reference-frame composition fixture.
//!
//! This is a synthetic exact-integer reference theorem over the #98 frame-graph
//! contract shape. It is not a production ephemeris, metric, rigid-body, camera,
//! or relativistic transform implementation.
//!
//! Frame identity and transform evidence remain separate:
//!
//! `ReferenceFrameId != transform at SimInstant`.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ReferenceFrameId, ScopeId, SimInstant, TypedDigest32,
    WorldInstanceId,
};

const SCALE_FRAME_COUNT: usize = 6;
const MAX_FRAME_EDGES: usize = 64;

type ExactPoint = [i128; 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TransformKind {
    Static,
    DeterministicLinearForcing,
}

impl TransformKind {
    const fn code(self) -> u8 {
        match self {
            Self::Static => 0,
            Self::DeterministicLinearForcing => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrameEdge {
    child: ReferenceFrameId,
    parent: ReferenceFrameId,
    kind: TransformKind,
    model_identity: TypedDigest32,
    base_offset: ExactPoint,
    velocity_per_nanosecond: ExactPoint,
}

impl FrameEdge {
    fn validate(&self) -> Result<(), RefError> {
        self.child.validate()?;
        self.parent.validate()?;
        self.model_identity.validate()?;
        if self.child == self.parent {
            return Err(RefError::SelfParent);
        }
        if self.kind == TransformKind::Static
            && self.velocity_per_nanosecond != [0_i128; 3]
        {
            return Err(RefError::StaticEdgeHasVelocity);
        }
        Ok(())
    }

    fn offset_at(&self, at: SimInstant) -> Result<ExactPoint, RefError> {
        self.validate()?;
        at.validate()?;
        let delta_ns = at.nanoseconds_since(SimInstant::GENESIS);
        let mut result = [0_i128; 3];
        for axis in 0..3 {
            let delta = self.velocity_per_nanosecond[axis]
                .checked_mul(delta_ns)
                .ok_or(RefError::CoordinateOverflow)?;
            result[axis] = self.base_offset[axis]
                .checked_add(delta)
                .ok_or(RefError::CoordinateOverflow)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrameGraph {
    root: ReferenceFrameId,
    edges: Vec<FrameEdge>,
}

impl FrameGraph {
    fn new(root: ReferenceFrameId, mut edges: Vec<FrameEdge>) -> Result<Self, RefError> {
        if edges.len() > MAX_FRAME_EDGES {
            return Err(RefError::TooManyEdges);
        }
        root.validate()?;
        edges.sort_by(|left, right| left.child.cmp(&right.child));
        let graph = Self { root, edges };
        graph.validate()?;
        Ok(graph)
    }

    fn validate(&self) -> Result<(), RefError> {
        self.root.validate()?;
        if self.edges.len() > MAX_FRAME_EDGES {
            return Err(RefError::TooManyEdges);
        }

        let mut known = BTreeSet::new();
        known.insert(self.root.clone());
        let mut by_child = BTreeMap::new();
        for edge in &self.edges {
            edge.validate()?;
            if edge.child == self.root {
                return Err(RefError::RootHasParent);
            }
            if by_child.insert(edge.child.clone(), edge).is_some() {
                return Err(RefError::DuplicateChildFrame);
            }
            known.insert(edge.child.clone());
        }

        for edge in &self.edges {
            if !known.contains(&edge.parent) {
                return Err(RefError::UnknownParentFrame);
            }
        }

        for frame in by_child.keys() {
            let mut cursor = frame.clone();
            let mut seen = BTreeSet::new();
            while cursor != self.root {
                if !seen.insert(cursor.clone()) {
                    return Err(RefError::Cycle);
                }
                let edge = by_child.get(&cursor).ok_or(RefError::UnknownParentFrame)?;
                cursor = edge.parent.clone();
            }
        }
        Ok(())
    }

    fn has_frame(&self, frame: &ReferenceFrameId) -> bool {
        frame == &self.root || self.edges.iter().any(|edge| &edge.child == frame)
    }

    fn edge_for(&self, child: &ReferenceFrameId) -> Result<&FrameEdge, RefError> {
        self.edges
            .iter()
            .find(|edge| &edge.child == child)
            .ok_or(RefError::UnknownFrame)
    }

    fn origin_in_root(&self, frame: &ReferenceFrameId, at: SimInstant) -> Result<ExactPoint, RefError> {
        self.validate()?;
        frame.validate()?;
        at.validate()?;
        if !self.has_frame(frame) {
            return Err(RefError::UnknownFrame);
        }

        let mut origin = [0_i128; 3];
        let mut cursor = frame.clone();
        while cursor != self.root {
            let edge = self.edge_for(&cursor)?;
            origin = add_point(origin, edge.offset_at(at)?)?;
            cursor = edge.parent.clone();
        }
        Ok(origin)
    }

    fn evaluate(
        &self,
        source: &ReferenceFrameId,
        target: &ReferenceFrameId,
        at: SimInstant,
    ) -> Result<ExactTransform, RefError> {
        let source_origin = self.origin_in_root(source, at)?;
        let target_origin = self.origin_in_root(target, at)?;
        let offset = sub_point(source_origin, target_origin)?;
        let transform = ExactTransform {
            source: source.clone(),
            target: target.clone(),
            at,
            graph_identity: self.digest()?,
            offset,
        };
        transform.validate()?;
        Ok(transform)
    }

    fn transform_point(
        &self,
        point: ExactPoint,
        source: &ReferenceFrameId,
        target: &ReferenceFrameId,
        at: SimInstant,
    ) -> Result<ExactPoint, RefError> {
        let transform = self.evaluate(source, target, at)?;
        add_point(point, transform.offset)
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001e.frame-graph.v1\0");
        push_string(&mut bytes, self.root.as_str());
        bytes.extend_from_slice(&(self.edges.len() as u32).to_le_bytes());
        for edge in &self.edges {
            push_string(&mut bytes, edge.child.as_str());
            push_string(&mut bytes, edge.parent.as_str());
            bytes.push(edge.kind.code());
            push_digest(&mut bytes, &edge.model_identity);
            push_point(&mut bytes, edge.base_offset);
            push_point(&mut bytes, edge.velocity_per_nanosecond);
        }
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001e.frame-graph.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactTransform {
    source: ReferenceFrameId,
    target: ReferenceFrameId,
    at: SimInstant,
    graph_identity: TypedDigest32,
    offset: ExactPoint,
}

impl ExactTransform {
    fn validate(&self) -> Result<(), RefError> {
        self.source.validate()?;
        self.target.validate()?;
        self.at.validate()?;
        self.graph_identity.validate()?;
        Ok(())
    }

    fn evidence_digest(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001e.transform-evidence.v1\0");
        push_string(&mut bytes, self.source.as_str());
        push_string(&mut bytes, self.target.as_str());
        push_instant(&mut bytes, self.at);
        push_digest(&mut bytes, &self.graph_identity);
        push_point(&mut bytes, self.offset);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001e.transform-evidence.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScopeFrameBinding {
    scope: ScopeId,
    frame: ReferenceFrameId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaleFrameMap {
    world: WorldInstanceId,
    bindings: Vec<ScopeFrameBinding>,
}

impl ScaleFrameMap {
    fn new(
        world: WorldInstanceId,
        mut bindings: Vec<ScopeFrameBinding>,
        graph: &FrameGraph,
    ) -> Result<Self, RefError> {
        world.validate()?;
        if bindings.len() != SCALE_FRAME_COUNT {
            return Err(RefError::WrongBindingCount);
        }
        bindings.sort();

        let mut scopes = BTreeSet::new();
        let mut frames = BTreeSet::new();
        for binding in &bindings {
            binding.scope.validate()?;
            binding.frame.validate()?;
            if !graph.has_frame(&binding.frame) {
                return Err(RefError::UnknownFrame);
            }
            if !scopes.insert(binding.scope.clone()) {
                return Err(RefError::DuplicateScopeBinding);
            }
            if !frames.insert(binding.frame.clone()) {
                return Err(RefError::DuplicateFrameBinding);
            }
        }

        Ok(Self { world, bindings })
    }

    fn frame_for(&self, scope: &ScopeId) -> Result<&ReferenceFrameId, RefError> {
        scope.validate()?;
        self.bindings
            .iter()
            .find(|binding| &binding.scope == scope)
            .map(|binding| &binding.frame)
            .ok_or(RefError::UnknownScope)
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        self.world.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001e.scope-frame-map.v1\0");
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&(self.bindings.len() as u32).to_le_bytes());
        for binding in &self.bindings {
            push_string(&mut bytes, binding.scope.as_str());
            push_string(&mut bytes, binding.frame.as_str());
        }
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001e.scope-frame-map.identity.v1",
            1,
            &bytes,
        )?)
    }
}

fn continuation_frame_context(
    continuation_identity: &TypedDigest32,
    graph: &FrameGraph,
    mapping: &ScaleFrameMap,
) -> Result<TypedDigest32, RefError> {
    continuation_identity.validate()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"symtropy.scale-cell-001e.continuation-frame-context.v1\0");
    push_digest(&mut bytes, continuation_identity);
    push_digest(&mut bytes, &graph.digest()?);
    push_digest(&mut bytes, &mapping.digest()?);
    Ok(TypedDigest32::sha256(
        "symtropy.scale-cell-001e.continuation-frame-context.identity.v1",
        1,
        &bytes,
    )?)
}

fn add_point(left: ExactPoint, right: ExactPoint) -> Result<ExactPoint, RefError> {
    let mut out = [0_i128; 3];
    for axis in 0..3 {
        out[axis] = left[axis]
            .checked_add(right[axis])
            .ok_or(RefError::CoordinateOverflow)?;
    }
    Ok(out)
}

fn sub_point(left: ExactPoint, right: ExactPoint) -> Result<ExactPoint, RefError> {
    let mut out = [0_i128; 3];
    for axis in 0..3 {
        out[axis] = left[axis]
            .checked_sub(right[axis])
            .ok_or(RefError::CoordinateOverflow)?;
    }
    Ok(out)
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_instant(bytes: &mut Vec<u8>, value: SimInstant) {
    bytes.extend_from_slice(&value.seconds_from_genesis.to_le_bytes());
    bytes.extend_from_slice(&value.nanos.to_le_bytes());
}

fn push_point(bytes: &mut Vec<u8>, value: ExactPoint) {
    for axis in value {
        bytes.extend_from_slice(&axis.to_le_bytes());
    }
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    TooManyEdges,
    SelfParent,
    StaticEdgeHasVelocity,
    RootHasParent,
    DuplicateChildFrame,
    UnknownParentFrame,
    Cycle,
    UnknownFrame,
    CoordinateOverflow,
    WrongBindingCount,
    DuplicateScopeBinding,
    DuplicateFrameBinding,
    UnknownScope,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn frame(value: &str) -> ReferenceFrameId {
    ReferenceFrameId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, value).unwrap()
}

fn static_edge(child: &str, parent: &str, base_offset: ExactPoint) -> FrameEdge {
    FrameEdge {
        child: frame(child),
        parent: frame(parent),
        kind: TransformKind::Static,
        model_identity: digest("scale-cell.frame-model.static.v1", child.as_bytes()),
        base_offset,
        velocity_per_nanosecond: [0, 0, 0],
    }
}

fn moving_body_edge() -> FrameEdge {
    FrameEdge {
        child: frame("frame:body:a"),
        parent: frame("frame:system"),
        kind: TransformKind::DeterministicLinearForcing,
        model_identity: digest("scale-cell.frame-model.linear.v1", b"body-a-ephemeris-v1"),
        base_offset: [1_000, 0, 0],
        velocity_per_nanosecond: [1, 0, 0],
    }
}

fn graph() -> FrameGraph {
    FrameGraph::new(
        frame("frame:system"),
        vec![
            moving_body_edge(),
            static_edge("frame:region:r0", "frame:body:a", [0, 2_000, 0]),
            static_edge("frame:local:l0", "frame:region:r0", [0, 0, 3_000]),
            static_edge("frame:structure:s0", "frame:local:l0", [40, 50, 60]),
            static_edge("frame:micro:m0", "frame:structure:s0", [7, 8, 9]),
        ],
    )
    .unwrap()
}

fn mapping(graph: &FrameGraph) -> ScaleFrameMap {
    ScaleFrameMap::new(
        WorldInstanceId::parse("world:scale-cell-001").unwrap(),
        vec![
            ScopeFrameBinding {
                scope: scope("system:fixture"),
                frame: frame("frame:system"),
            },
            ScopeFrameBinding {
                scope: scope("body:a"),
                frame: frame("frame:body:a"),
            },
            ScopeFrameBinding {
                scope: scope("region:r0"),
                frame: frame("frame:region:r0"),
            },
            ScopeFrameBinding {
                scope: scope("local:l0"),
                frame: frame("frame:local:l0"),
            },
            ScopeFrameBinding {
                scope: scope("structure:s0"),
                frame: frame("frame:structure:s0"),
            },
            ScopeFrameBinding {
                scope: scope("micro:m0"),
                frame: frame("frame:micro:m0"),
            },
        ],
        graph,
    )
    .unwrap()
}

#[test]
fn six_scope_frame_mapping_is_explicit_and_complete() {
    let graph = graph();
    let map = mapping(&graph);
    assert_eq!(map.bindings.len(), SCALE_FRAME_COUNT);
    assert_eq!(
        map.frame_for(&scope("micro:m0")).unwrap(),
        &frame("frame:micro:m0")
    );
    assert_ne!(scope("micro:m0").as_str(), frame("frame:micro:m0").as_str());
}

#[test]
fn frame_edge_input_order_cannot_change_graph_identity() {
    let first = graph();
    let mut reversed = first.edges.clone();
    reversed.reverse();
    let second = FrameGraph::new(frame("frame:system"), reversed).unwrap();
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn macro_micro_transform_round_trip_is_exact() {
    let graph = graph();
    let at = SimInstant::new(10, 123).unwrap();
    let point = [11_i128, -22, 33];
    let system = frame("frame:system");
    let micro = frame("frame:micro:m0");

    let in_system = graph.transform_point(point, &micro, &system, at).unwrap();
    let round_trip = graph
        .transform_point(in_system, &system, &micro, at)
        .unwrap();
    assert_eq!(round_trip, point);
}

#[test]
fn composed_paths_agree_exactly() {
    let graph = graph();
    let at = SimInstant::new(3, 250).unwrap();
    let point = [4_i128, 5, 6];
    let micro = frame("frame:micro:m0");
    let region = frame("frame:region:r0");
    let system = frame("frame:system");

    let direct = graph.transform_point(point, &micro, &system, at).unwrap();
    let through_region = graph
        .transform_point(point, &micro, &region, at)
        .and_then(|value| graph.transform_point(value, &region, &system, at))
        .unwrap();
    assert_eq!(direct, through_region);
}

#[test]
fn deterministic_forcing_depends_on_siminstant_not_wall_clock() {
    let graph = graph();
    let body = frame("frame:body:a");
    let system = frame("frame:system");
    let first_time = SimInstant::new(2, 0).unwrap();
    let second_time = SimInstant::new(3, 0).unwrap();

    let first = graph.evaluate(&body, &system, first_time).unwrap();
    let repeat = graph.evaluate(&body, &system, first_time).unwrap();
    let second = graph.evaluate(&body, &system, second_time).unwrap();

    assert_eq!(first, repeat);
    assert_ne!(first.offset, second.offset);
    assert_ne!(first.evidence_digest().unwrap(), second.evidence_digest().unwrap());
}

#[test]
fn changed_model_identity_changes_graph_and_transform_evidence() {
    let first = graph();
    let mut edges = first.edges.clone();
    let body = edges
        .iter_mut()
        .find(|edge| edge.child == frame("frame:body:a"))
        .unwrap();
    body.model_identity = digest("scale-cell.frame-model.linear.v1", b"body-a-ephemeris-v2");
    let second = FrameGraph::new(frame("frame:system"), edges).unwrap();

    let at = SimInstant::new(4, 0).unwrap();
    let source = frame("frame:micro:m0");
    let target = frame("frame:system");
    assert_ne!(first.digest().unwrap(), second.digest().unwrap());
    assert_ne!(
        first.evaluate(&source, &target, at).unwrap().evidence_digest().unwrap(),
        second.evaluate(&source, &target, at).unwrap().evidence_digest().unwrap()
    );
}

#[test]
fn unknown_parent_cycle_and_invalid_static_velocity_fail_closed() {
    let unknown_parent = FrameGraph::new(
        frame("frame:root"),
        vec![static_edge("frame:child", "frame:missing", [0, 0, 0])],
    );
    assert_eq!(unknown_parent, Err(RefError::UnknownParentFrame));

    let cycle = FrameGraph::new(
        frame("frame:root"),
        vec![
            static_edge("frame:a", "frame:b", [0, 0, 0]),
            static_edge("frame:b", "frame:a", [0, 0, 0]),
        ],
    );
    assert_eq!(cycle, Err(RefError::Cycle));

    let mut invalid_static = static_edge("frame:a", "frame:root", [0, 0, 0]);
    invalid_static.velocity_per_nanosecond = [1, 0, 0];
    assert_eq!(
        FrameGraph::new(frame("frame:root"), vec![invalid_static]),
        Err(RefError::StaticEdgeHasVelocity)
    );
}

#[test]
fn unknown_frame_transform_fails_closed() {
    let graph = graph();
    assert_eq!(
        graph.evaluate(
            &frame("frame:missing"),
            &frame("frame:system"),
            SimInstant::GENESIS,
        ),
        Err(RefError::UnknownFrame)
    );
}

#[test]
fn frame_context_is_continuation_significant_but_not_scope_identity() {
    let first_graph = graph();
    let first_map = mapping(&first_graph);
    let resume = digest("symtropy.scale-cell-001d.continuation-identity.v1", b"resume-r1");
    let first_context = continuation_frame_context(&resume, &first_graph, &first_map).unwrap();

    let mut edges = first_graph.edges.clone();
    edges
        .iter_mut()
        .find(|edge| edge.child == frame("frame:body:a"))
        .unwrap()
        .model_identity = digest("scale-cell.frame-model.linear.v1", b"body-a-ephemeris-v2");
    let second_graph = FrameGraph::new(frame("frame:system"), edges).unwrap();
    let second_map = mapping(&second_graph);
    let second_context = continuation_frame_context(&resume, &second_graph, &second_map).unwrap();

    assert_ne!(first_context, second_context);
    assert_eq!(
        first_map.bindings[5].scope,
        second_map.bindings[5].scope,
        "changing frame semantics must not rename the semantic scope"
    );
}
