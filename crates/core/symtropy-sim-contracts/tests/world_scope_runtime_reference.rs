// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! WORLD-SCOPE-RUNTIME-00A executable reference oracle.
//!
//! This deliberately reuses `WorldInstanceId` and `ScopeId`. It is an external
//! test oracle, not a public runtime authority API. It owns no domain state,
//! reference-frame placement, residency, networking, rendering, or fidelity.
//!
//! The first theorem freezes a bounded one-root/single-parent hierarchy plus a
//! prepare/commit reparent transition. The hierarchy generation participates in
//! exact identity so A -> B -> A cannot recreate an old current hierarchy
//! identity.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{ContractError, ScopeId, TypedDigest32, WorldInstanceId};

const SCHEMA_VERSION: u32 = 1;
const MAX_ENTRIES: usize = 4_096;
const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.scope-hierarchy.reference.v1\0";
const REPARENT_DOMAIN: &[u8] = b"symtropy.scope-hierarchy.reparent-reference.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generation(u64);

impl Generation {
    const GENESIS: Self = Self(0);

    fn next(self) -> Result<Self, RefError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(RefError::GenerationOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    scope: ScopeId,
    parent: Option<ScopeId>,
}

impl Entry {
    fn root(scope: ScopeId) -> Self {
        Self {
            scope,
            parent: None,
        }
    }

    fn child(scope: ScopeId, parent: ScopeId) -> Self {
        Self {
            scope,
            parent: Some(parent),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    schema_version: u32,
    world: WorldInstanceId,
    generation: Generation,
    entries: Vec<Entry>,
}

impl Snapshot {
    fn new(
        world: WorldInstanceId,
        generation: Generation,
        mut entries: Vec<Entry>,
    ) -> Result<Self, RefError> {
        if entries.is_empty() {
            return Err(RefError::EmptyHierarchy);
        }
        if entries.len() > MAX_ENTRIES {
            return Err(RefError::TooManyEntries);
        }

        world.validate()?;
        for entry in &entries {
            entry.scope.validate()?;
            if let Some(parent) = &entry.parent {
                parent.validate()?;
            }
        }

        entries.sort_by(|left, right| {
            left.scope
                .cmp(&right.scope)
                .then_with(|| left.parent.cmp(&right.parent))
        });

        let snapshot = Self {
            schema_version: SCHEMA_VERSION,
            world,
            generation,
            entries,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<(), RefError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(RefError::UnsupportedSchema);
        }
        if self.entries.is_empty() {
            return Err(RefError::EmptyHierarchy);
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err(RefError::TooManyEntries);
        }

        self.world.validate()?;
        let mut by_scope = BTreeMap::new();
        let mut roots = 0_usize;

        for entry in &self.entries {
            entry.scope.validate()?;
            if let Some(parent) = &entry.parent {
                parent.validate()?;
                if parent == &entry.scope {
                    return Err(RefError::SelfParent);
                }
            } else {
                roots += 1;
            }

            if by_scope
                .insert(entry.scope.clone(), entry.parent.clone())
                .is_some()
            {
                return Err(RefError::DuplicateScope(entry.scope.clone()));
            }
        }

        if roots != 1 {
            return Err(RefError::InvalidRootCount(roots));
        }

        for (child, parent) in &by_scope {
            if let Some(parent) = parent
                && !by_scope.contains_key(parent)
            {
                return Err(RefError::UnknownParent {
                    child: child.clone(),
                    parent: parent.clone(),
                });
            }
        }

        for start in by_scope.keys() {
            let mut seen = BTreeSet::new();
            let mut cursor = (*start).clone();

            loop {
                if !seen.insert(cursor.clone()) {
                    return Err(RefError::Cycle);
                }

                match by_scope.get(&cursor).ok_or(RefError::UnknownScope)? {
                    Some(parent) => cursor = parent.clone(),
                    None => break,
                }
            }
        }

        Ok(())
    }

    fn parent_of(&self, scope: &ScopeId) -> Result<Option<ScopeId>, RefError> {
        self.validate()?;
        self.entries
            .iter()
            .find(|entry| &entry.scope == scope)
            .map(|entry| entry.parent.clone())
            .ok_or(RefError::UnknownScope)
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, RefError> {
        self.validate()?;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(SNAPSHOT_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&self.generation.0.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for entry in &self.entries {
            push_string(&mut bytes, entry.scope.as_str());
            match &entry.parent {
                Some(parent) => {
                    bytes.push(1);
                    push_string(&mut bytes, parent.as_str());
                }
                None => bytes.push(0),
            }
        }

        Ok(bytes)
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        Ok(TypedDigest32::sha256(
            "symtropy.scope-hierarchy.reference.v1",
            SCHEMA_VERSION,
            &self.canonical_bytes()?,
        )?)
    }

    fn prepare_reparent(
        &self,
        child: &ScopeId,
        new_parent: &ScopeId,
    ) -> Result<Prepared, RefError> {
        self.validate()?;
        child.validate()?;
        new_parent.validate()?;

        if child == new_parent {
            return Err(RefError::SelfParent);
        }

        let old_parent = self
            .parent_of(child)?
            .ok_or_else(|| RefError::RootReparent(child.clone()))?;

        if &old_parent == new_parent {
            return Err(RefError::NoChange);
        }

        self.parent_of(new_parent)?;

        let result_generation = self.generation.next()?;
        let candidate = self.with_reparent(child, new_parent, result_generation)?;
        let source_digest = self.digest()?;
        let result_digest = candidate.digest()?;
        let request_identity = reparent_identity(
            &self.world,
            self.generation,
            &source_digest,
            child,
            &old_parent,
            new_parent,
        )?;

        Ok(Prepared {
            world: self.world.clone(),
            source_generation: self.generation,
            result_generation,
            child: child.clone(),
            old_parent,
            new_parent: new_parent.clone(),
            source_digest,
            result_digest,
            request_identity,
        })
    }

    fn commit_reparent(&mut self, prepared: &Prepared) -> Result<Receipt, RefError> {
        self.validate()?;

        if self.world != prepared.world {
            return Err(RefError::PreparedWorldMismatch);
        }
        if self.generation != prepared.source_generation {
            return Err(RefError::StaleGeneration);
        }

        let current_parent = self
            .parent_of(&prepared.child)?
            .ok_or_else(|| RefError::RootReparent(prepared.child.clone()))?;
        if current_parent != prepared.old_parent {
            return Err(RefError::StaleParent);
        }

        let current_digest = self.digest()?;
        if !current_digest.same_typed_value(&prepared.source_digest) {
            return Err(RefError::StaleDigest);
        }

        let candidate = self.with_reparent(
            &prepared.child,
            &prepared.new_parent,
            prepared.result_generation,
        )?;
        let result_digest = candidate.digest()?;
        if !result_digest.same_typed_value(&prepared.result_digest) {
            return Err(RefError::PreparedResultMismatch);
        }

        let receipt = Receipt {
            request_identity: prepared.request_identity.clone(),
            child: prepared.child.clone(),
            old_parent: prepared.old_parent.clone(),
            new_parent: prepared.new_parent.clone(),
            source_generation: prepared.source_generation,
            result_generation: prepared.result_generation,
            source_digest: prepared.source_digest.clone(),
            result_digest,
        };

        *self = candidate;
        Ok(receipt)
    }

    fn with_reparent(
        &self,
        child: &ScopeId,
        new_parent: &ScopeId,
        generation: Generation,
    ) -> Result<Self, RefError> {
        let mut entries = self.entries.clone();
        let entry = entries
            .iter_mut()
            .find(|entry| &entry.scope == child)
            .ok_or(RefError::UnknownScope)?;
        entry.parent = Some(new_parent.clone());
        Self::new(self.world.clone(), generation, entries)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Prepared {
    world: WorldInstanceId,
    source_generation: Generation,
    result_generation: Generation,
    child: ScopeId,
    old_parent: ScopeId,
    new_parent: ScopeId,
    source_digest: TypedDigest32,
    result_digest: TypedDigest32,
    request_identity: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    request_identity: TypedDigest32,
    child: ScopeId,
    old_parent: ScopeId,
    new_parent: ScopeId,
    source_generation: Generation,
    result_generation: Generation,
    source_digest: TypedDigest32,
    result_digest: TypedDigest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    UnsupportedSchema,
    EmptyHierarchy,
    TooManyEntries,
    DuplicateScope(ScopeId),
    InvalidRootCount(usize),
    SelfParent,
    UnknownScope,
    UnknownParent { child: ScopeId, parent: ScopeId },
    Cycle,
    RootReparent(ScopeId),
    NoChange,
    GenerationOverflow,
    PreparedWorldMismatch,
    StaleGeneration,
    StaleParent,
    StaleDigest,
    PreparedResultMismatch,
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

fn reparent_identity(
    world: &WorldInstanceId,
    generation: Generation,
    source_digest: &TypedDigest32,
    child: &ScopeId,
    old_parent: &ScopeId,
    new_parent: &ScopeId,
) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(REPARENT_DOMAIN);
    push_string(&mut bytes, world.as_str());
    bytes.extend_from_slice(&generation.0.to_le_bytes());
    bytes.extend_from_slice(&source_digest.value);
    push_string(&mut bytes, child.as_str());
    push_string(&mut bytes, old_parent.as_str());
    push_string(&mut bytes, new_parent.as_str());

    Ok(TypedDigest32::sha256(
        "symtropy.scope-hierarchy.reparent-reference.v1",
        1,
        &bytes,
    )?)
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn fixture(entries: Vec<Entry>) -> Snapshot {
    Snapshot::new(world("world:scope-fixture"), Generation::GENESIS, entries).unwrap()
}

fn nested_fixture() -> Snapshot {
    fixture(vec![
        Entry::root(scope("system")),
        Entry::child(scope("body:a"), scope("system")),
        Entry::child(scope("region:r0"), scope("body:a")),
        Entry::child(scope("mobile:m"), scope("region:r0")),
        Entry::child(scope("interior:i"), scope("mobile:m")),
        Entry::child(scope("region:r1"), scope("body:a")),
    ])
}

#[test]
fn input_order_does_not_change_canonical_hierarchy_identity() {
    let a = Entry::root(scope("system"));
    let b = Entry::child(scope("body:a"), scope("system"));
    let c = Entry::child(scope("region:r0"), scope("body:a"));

    let first = fixture(vec![a.clone(), b.clone(), c.clone()]);
    let second = fixture(vec![c, a, b]);

    assert_eq!(first, second);
    assert_eq!(
        first.canonical_bytes().unwrap(),
        second.canonical_bytes().unwrap()
    );
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn same_scope_label_in_another_world_has_different_identity() {
    let entries = vec![Entry::root(scope("system"))];
    let first = Snapshot::new(world("world:a"), Generation::GENESIS, entries.clone()).unwrap();
    let second = Snapshot::new(world("world:b"), Generation::GENESIS, entries).unwrap();

    assert_ne!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn malformed_hierarchies_fail_closed() {
    let duplicate = Snapshot::new(
        world("world:test"),
        Generation::GENESIS,
        vec![
            Entry::root(scope("system")),
            Entry::child(scope("body:a"), scope("system")),
            Entry::child(scope("body:a"), scope("system")),
        ],
    );
    assert!(matches!(
        duplicate,
        Err(RefError::DuplicateScope(value)) if value == scope("body:a")
    ));

    let unknown_parent = Snapshot::new(
        world("world:test"),
        Generation::GENESIS,
        vec![
            Entry::root(scope("system")),
            Entry::child(scope("body:a"), scope("missing")),
        ],
    );
    assert!(matches!(
        unknown_parent,
        Err(RefError::UnknownParent { child, parent })
            if child == scope("body:a") && parent == scope("missing")
    ));

    let multiple_roots = Snapshot::new(
        world("world:test"),
        Generation::GENESIS,
        vec![
            Entry::root(scope("system")),
            Entry::root(scope("other-root")),
        ],
    );
    match multiple_roots {
        Err(RefError::InvalidRootCount(actual)) => assert_eq!(actual, 2),
        other => panic!("expected InvalidRootCount(2), got {other:?}"),
    }
}

#[test]
fn cycle_is_rejected_before_reparent_mutation() {
    let hierarchy = nested_fixture();
    let before = hierarchy.clone();
    let result = hierarchy.prepare_reparent(&scope("body:a"), &scope("interior:i"));

    assert!(matches!(result, Err(RefError::Cycle)));
    assert_eq!(hierarchy, before);
}

#[test]
fn reparent_preserves_scope_identity_and_nested_child_relation() {
    let mut hierarchy = nested_fixture();
    let prepared = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("region:r1"))
        .unwrap();
    let receipt = hierarchy.commit_reparent(&prepared).unwrap();

    assert_eq!(receipt.child, scope("mobile:m"));
    assert_eq!(receipt.old_parent, scope("region:r0"));
    assert_eq!(receipt.new_parent, scope("region:r1"));
    assert_eq!(
        hierarchy.parent_of(&scope("mobile:m")).unwrap(),
        Some(scope("region:r1"))
    );
    assert_eq!(
        hierarchy.parent_of(&scope("interior:i")).unwrap(),
        Some(scope("mobile:m"))
    );
    assert_eq!(hierarchy.generation.0, 1);
    assert_eq!(receipt.source_generation.0, 0);
    assert_eq!(receipt.result_generation.0, 1);
    assert_eq!(receipt.request_identity, prepared.request_identity);
    assert_eq!(receipt.source_digest, prepared.source_digest);
    assert_eq!(receipt.result_digest, prepared.result_digest);
}

#[test]
fn stale_prepared_reparent_cannot_commit_after_competing_change() {
    let mut hierarchy = nested_fixture();
    let stale = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("region:r1"))
        .unwrap();
    let competing = hierarchy
        .prepare_reparent(&scope("region:r0"), &scope("region:r1"))
        .unwrap();

    hierarchy.commit_reparent(&competing).unwrap();
    let before_failed_commit = hierarchy.clone();

    assert_eq!(
        hierarchy.commit_reparent(&stale),
        Err(RefError::StaleGeneration)
    );
    assert_eq!(hierarchy, before_failed_commit);
}

#[test]
fn aba_visible_relation_does_not_recreate_old_hierarchy_identity() {
    let mut hierarchy = nested_fixture();
    let initial_digest = hierarchy.digest().unwrap();

    let to_r1 = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("region:r1"))
        .unwrap();
    hierarchy.commit_reparent(&to_r1).unwrap();

    let back_to_r0 = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("region:r0"))
        .unwrap();
    hierarchy.commit_reparent(&back_to_r0).unwrap();

    assert_eq!(
        hierarchy.parent_of(&scope("mobile:m")).unwrap(),
        Some(scope("region:r0"))
    );
    assert_eq!(hierarchy.generation.0, 2);
    assert_ne!(hierarchy.digest().unwrap(), initial_digest);
}

#[test]
fn preparation_is_read_only_and_semantically_target_bound() {
    let hierarchy = nested_fixture();
    let before = hierarchy.clone();
    let prepared_r1 = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("region:r1"))
        .unwrap();
    let prepared_body = hierarchy
        .prepare_reparent(&scope("mobile:m"), &scope("body:a"))
        .unwrap();

    assert_eq!(hierarchy, before);
    assert_eq!(prepared_r1.source_generation.0, 0);
    assert_eq!(prepared_r1.result_generation.0, 1);
    assert_ne!(prepared_r1.source_digest, prepared_r1.result_digest);
    assert_ne!(prepared_r1.request_identity, prepared_body.request_identity);
}

#[test]
fn root_and_no_change_reparent_requests_are_rejected_without_mutation() {
    let hierarchy = nested_fixture();
    let before = hierarchy.clone();

    assert!(matches!(
        hierarchy.prepare_reparent(&scope("system"), &scope("body:a")),
        Err(RefError::RootReparent(value)) if value == scope("system")
    ));
    assert_eq!(
        hierarchy.prepare_reparent(&scope("mobile:m"), &scope("region:r0")),
        Err(RefError::NoChange)
    );
    assert_eq!(hierarchy, before);
}
