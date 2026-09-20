// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001A executable observation-invariance reference oracle.
//!
//! The first scale cell deliberately proves less than promotion/collapse. It
//! freezes one six-scope canonical world and a separate observation runtime so
//! macro -> micro -> macro navigation can change residency/cache state without
//! changing canonical world commitments.
//!
//! Process demand is also read-only in 001A. It can identify that a scope needs
//! a separately qualified admission path, but cannot refine or mutate canon by
//! itself.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{ScopeId, TypedDigest32, WorldInstanceId};

const CELL_SCHEMA_VERSION: u32 = 1;
const CELL_DOMAIN: &[u8] = b"symtropy.scale-cell.reference.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScopeClass {
    System,
    Body,
    Region,
    Local,
    Structure,
    Micro,
}

impl ScopeClass {
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
struct CanonicalScopeState {
    class: ScopeClass,
    parent: Option<ScopeId>,
    exact_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellWorld {
    schema_version: u32,
    world: WorldInstanceId,
    scopes: BTreeMap<ScopeId, CanonicalScopeState>,
}

impl CellWorld {
    fn fixture(world: WorldInstanceId) -> Self {
        let system = scope("system:sol-fixture");
        let body = scope("body:a");
        let region = scope("region:r0");
        let local = scope("local:l0");
        let structure = scope("structure:s0");
        let micro = scope("micro:m0");

        let scopes = BTreeMap::from([
            (
                system.clone(),
                CanonicalScopeState {
                    class: ScopeClass::System,
                    parent: None,
                    exact_quantity: 100,
                },
            ),
            (
                body.clone(),
                CanonicalScopeState {
                    class: ScopeClass::Body,
                    parent: Some(system),
                    exact_quantity: 100,
                },
            ),
            (
                region.clone(),
                CanonicalScopeState {
                    class: ScopeClass::Region,
                    parent: Some(body),
                    exact_quantity: 100,
                },
            ),
            (
                local.clone(),
                CanonicalScopeState {
                    class: ScopeClass::Local,
                    parent: Some(region),
                    exact_quantity: 100,
                },
            ),
            (
                structure.clone(),
                CanonicalScopeState {
                    class: ScopeClass::Structure,
                    parent: Some(local),
                    exact_quantity: 100,
                },
            ),
            (
                micro,
                CanonicalScopeState {
                    class: ScopeClass::Micro,
                    parent: Some(structure),
                    exact_quantity: 100,
                },
            ),
        ]);

        Self {
            schema_version: CELL_SCHEMA_VERSION,
            world,
            scopes,
        }
    }

    fn contains_scope(&self, scope: &ScopeId) -> bool {
        self.scopes.contains_key(scope)
    }

    fn state(&self, scope: &ScopeId) -> Result<&CanonicalScopeState, CellError> {
        self.scopes
            .get(scope)
            .ok_or_else(|| CellError::UnknownScope(scope.clone()))
    }

    fn direct_children(&self, parent: &ScopeId) -> Result<Vec<ScopeId>, CellError> {
        self.state(parent)?;
        Ok(self
            .scopes
            .iter()
            .filter_map(|(scope, state)| {
                (state.parent.as_ref() == Some(parent)).then(|| scope.clone())
            })
            .collect())
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CELL_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&(self.scopes.len() as u32).to_le_bytes());

        for (scope, state) in &self.scopes {
            push_string(&mut bytes, scope.as_str());
            bytes.push(state.class.code());
            match &state.parent {
                Some(parent) => {
                    bytes.push(1);
                    push_string(&mut bytes, parent.as_str());
                }
                None => bytes.push(0),
            }
            bytes.extend_from_slice(&state.exact_quantity.to_le_bytes());
        }

        bytes
    }

    fn commitment(&self) -> TypedDigest32 {
        TypedDigest32::sha256(
            "symtropy.scale-cell.reference.v1",
            CELL_SCHEMA_VERSION,
            &self.canonical_bytes(),
        )
        .unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ObservationRuntime {
    resident_for_observation: BTreeSet<ScopeId>,
    navigation_trace: Vec<ScopeId>,
}

impl ObservationRuntime {
    fn observe_scope(
        &mut self,
        world: &CellWorld,
        scope: &ScopeId,
    ) -> Result<ObservationRef, CellError> {
        world.state(scope)?;
        self.resident_for_observation.insert(scope.clone());
        self.navigation_trace.push(scope.clone());

        Ok(ObservationRef {
            scope: scope.clone(),
            source_world_commitment: world.commitment(),
        })
    }

    fn is_observation_resident(&self, scope: &ScopeId) -> bool {
        self.resident_for_observation.contains(scope)
    }

    fn request_process(
        &self,
        world: &CellWorld,
        scope: &ScopeId,
    ) -> Result<ProcessDemand, CellError> {
        world.state(scope)?;
        Ok(ProcessDemand {
            scope: scope.clone(),
            source_world_commitment: world.commitment(),
            outcome: ProcessDemandOutcome::QualifiedAdmissionRequired,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationRef {
    scope: ScopeId,
    source_world_commitment: TypedDigest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessDemandOutcome {
    QualifiedAdmissionRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessDemand {
    scope: ScopeId,
    source_world_commitment: TypedDigest32,
    outcome: ProcessDemandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CellError {
    UnknownScope(ScopeId),
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

fn navigation_path() -> Vec<ScopeId> {
    vec![
        scope("system:sol-fixture"),
        scope("body:a"),
        scope("region:r0"),
        scope("local:l0"),
        scope("structure:s0"),
        scope("micro:m0"),
        scope("structure:s0"),
        scope("local:l0"),
        scope("region:r0"),
        scope("body:a"),
        scope("system:sol-fixture"),
    ]
}

#[test]
fn macro_micro_macro_observation_is_canonically_invariant() {
    let control = CellWorld::fixture(world("world:scale-cell"));
    let experiment = control.clone();
    let control_bytes = control.canonical_bytes();
    let control_commitment = control.commitment();
    let mut runtime = ObservationRuntime::default();

    for requested in navigation_path() {
        let observation = runtime.observe_scope(&experiment, &requested).unwrap();
        assert_eq!(observation.scope, requested);
        assert_eq!(observation.source_world_commitment, control_commitment);
    }

    assert_eq!(experiment.canonical_bytes(), control_bytes);
    assert_eq!(experiment.commitment(), control_commitment);
    assert_eq!(control.commitment(), experiment.commitment());
    assert_eq!(runtime.resident_for_observation.len(), 6);
    assert_eq!(runtime.navigation_trace.len(), 11);
}

#[test]
fn navigation_order_and_repetition_change_runtime_not_canon() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));
    let before = world_state.commitment();
    let mut first = ObservationRuntime::default();
    let mut second = ObservationRuntime::default();

    for requested in [
        scope("micro:m0"),
        scope("system:sol-fixture"),
        scope("micro:m0"),
    ] {
        first.observe_scope(&world_state, &requested).unwrap();
    }
    for requested in [
        scope("system:sol-fixture"),
        scope("micro:m0"),
        scope("structure:s0"),
    ] {
        second.observe_scope(&world_state, &requested).unwrap();
    }

    assert_ne!(first.navigation_trace, second.navigation_trace);
    assert_ne!(first.resident_for_observation, second.resident_for_observation);
    assert_eq!(world_state.commitment(), before);
}

#[test]
fn nonresident_scope_remains_semantically_present_and_discoverable() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));
    let runtime = ObservationRuntime::default();
    let micro = scope("micro:m0");

    assert!(world_state.contains_scope(&micro));
    assert!(!runtime.is_observation_resident(&micro));
    assert_eq!(
        world_state.direct_children(&scope("structure:s0")).unwrap(),
        vec![micro]
    );
}

#[test]
fn observation_of_unknown_scope_fails_without_runtime_or_world_mutation() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));
    let world_before = world_state.commitment();
    let mut runtime = ObservationRuntime::default();
    let runtime_before = runtime.clone();
    let missing = scope("micro:missing");

    assert_eq!(
        runtime.observe_scope(&world_state, &missing),
        Err(CellError::UnknownScope(missing))
    );
    assert_eq!(runtime, runtime_before);
    assert_eq!(world_state.commitment(), world_before);
}

#[test]
fn process_demand_is_read_only_and_cannot_self_admit_refinement() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));
    let before = world_state.commitment();
    let runtime = ObservationRuntime::default();
    let micro = scope("micro:m0");

    let demand = runtime.request_process(&world_state, &micro).unwrap();

    assert_eq!(demand.scope, micro);
    assert_eq!(demand.source_world_commitment, before);
    assert_eq!(
        demand.outcome,
        ProcessDemandOutcome::QualifiedAdmissionRequired
    );
    assert_eq!(world_state.commitment(), before);
    assert!(!runtime.is_observation_resident(&scope("micro:m0")));
}

#[test]
fn world_instance_is_part_of_exact_cell_commitment() {
    let first = CellWorld::fixture(world("world:scale-cell-a"));
    let second = CellWorld::fixture(world("world:scale-cell-b"));

    assert_eq!(first.scopes, second.scopes);
    assert_ne!(first.commitment(), second.commitment());
}

#[test]
fn six_scope_fixture_has_one_exact_chain() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));

    assert_eq!(world_state.scopes.len(), 6);
    assert_eq!(
        world_state.direct_children(&scope("system:sol-fixture")).unwrap(),
        vec![scope("body:a")]
    );
    assert_eq!(
        world_state.direct_children(&scope("body:a")).unwrap(),
        vec![scope("region:r0")]
    );
    assert_eq!(
        world_state.direct_children(&scope("region:r0")).unwrap(),
        vec![scope("local:l0")]
    );
    assert_eq!(
        world_state.direct_children(&scope("local:l0")).unwrap(),
        vec![scope("structure:s0")]
    );
    assert_eq!(
        world_state.direct_children(&scope("structure:s0")).unwrap(),
        vec![scope("micro:m0")]
    );
    assert!(
        world_state
            .direct_children(&scope("micro:m0"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn all_canonical_quantities_remain_unchanged_after_observation_navigation() {
    let world_state = CellWorld::fixture(world("world:scale-cell"));
    let before: Vec<_> = world_state
        .scopes
        .iter()
        .map(|(scope, state)| (scope.clone(), state.exact_quantity))
        .collect();
    let mut runtime = ObservationRuntime::default();

    for requested in navigation_path() {
        runtime.observe_scope(&world_state, &requested).unwrap();
    }

    let after: Vec<_> = world_state
        .scopes
        .iter()
        .map(|(scope, state)| (scope.clone(), state.exact_quantity))
        .collect();
    assert_eq!(before, after);
    assert!(after.iter().all(|(_, quantity)| *quantity == 100));
}
