// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001D continuation reference fixture.
//!
//! This tranche treats prior scale-cell state/frontier commitments as opaque
//! typed evidence and proves only their continuation composition. It does not
//! reimplement hierarchy, fidelity, scheduling, or domain evolution semantics.
//!
//! The central theorem is:
//!
//! same visible physical state
//! != same continuation state
//!
//! when future-bearing scheduler/frontier evidence differs.

use std::collections::BTreeSet;

use symtropy_sim_contracts::{
    AuthorityId, ContinuationRequirement, DigestAlgorithm, DomainContinuationEntry, FixedTimebase,
    LifecycleMode, ReferenceFrameId, RepresentationId, ResumeIdentityClass, ScopeId, SimInstant,
    SnapshotCodecId, TimebaseId, TypedDigest32, WorldContinuationManifest, WorldInstanceId,
    validate_manifest_lineage,
};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    StableCoarse,
    PromotionPrepared,
    StableFine,
    CollapsePrepared,
}

impl Phase {
    const fn code(self) -> u8 {
        match self {
            Self::StableCoarse => 0,
            Self::PromotionPrepared => 1,
            Self::StableFine => 2,
            Self::CollapsePrepared => 3,
        }
    }

    const fn is_checkpointable(self) -> bool {
        matches!(self, Self::StableCoarse | Self::StableFine)
    }

    fn representation(self) -> Result<RepresentationId, RefError> {
        match self {
            Self::StableCoarse => Ok(RepresentationId::parse("scale-cell.coarse.v1")?),
            Self::StableFine => Ok(RepresentationId::parse("scale-cell.fine.v1")?),
            Self::PromotionPrepared | Self::CollapsePrepared => Err(RefError::AmbiguousTransition),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaleContinuationState {
    world: WorldInstanceId,
    at: SimInstant,
    authority_generation: u64,
    process_counter: u64,
    cell_commitment: TypedDigest32,
    frontier_commitment: TypedDigest32,
    phase: Phase,
}

impl ScaleContinuationState {
    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.at.validate()?;
        self.cell_commitment.validate()?;
        self.frontier_commitment.validate()?;
        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001d.snapshot.v1\0");
        bytes.extend_from_slice(&SNAPSHOT_SCHEMA_VERSION.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        push_instant(&mut bytes, self.at);
        bytes.extend_from_slice(&self.authority_generation.to_le_bytes());
        bytes.extend_from_slice(&self.process_counter.to_le_bytes());
        push_digest(&mut bytes, &self.cell_commitment);
        push_digest(&mut bytes, &self.frontier_commitment);
        bytes.push(self.phase.code());
        Ok(bytes)
    }

    fn snapshot_content(&self) -> Result<TypedDigest32, RefError> {
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001d.snapshot-content.v1",
            SNAPSHOT_SCHEMA_VERSION,
            &self.canonical_bytes()?,
        )?)
    }

    fn resume_identity(&self) -> Result<TypedDigest32, RefError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001d.resume.v1\0");
        push_digest(&mut bytes, &self.snapshot_content()?);
        push_digest(&mut bytes, &self.frontier_commitment);
        bytes.extend_from_slice(&self.authority_generation.to_le_bytes());
        bytes.extend_from_slice(&self.process_counter.to_le_bytes());
        bytes.push(self.phase.code());
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001d.continuation-identity.v1",
            1,
            &bytes,
        )?)
    }

    fn advance_one_reference_step(&self) -> Result<Self, RefError> {
        self.validate()?;
        if !self.phase.is_checkpointable() {
            return Err(RefError::AmbiguousTransition);
        }
        let authority_generation = self
            .authority_generation
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let process_counter = self
            .process_counter
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let at = self
            .at
            .checked_add_nanoseconds(1_000_000_000)
            .map_err(RefError::from)?;

        let cell_commitment = derive_step_digest(
            "symtropy.scale-cell-001d.cell-step.v1",
            &self.cell_commitment,
            process_counter,
        )?;
        let frontier_commitment = derive_step_digest(
            "symtropy.scale-cell-001d.frontier-step.v1",
            &self.frontier_commitment,
            process_counter,
        )?;

        let next = Self {
            world: self.world.clone(),
            at,
            authority_generation,
            process_counter,
            cell_commitment,
            frontier_commitment,
            phase: self.phase,
        };
        next.validate()?;
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckpointBundle {
    state: ScaleContinuationState,
    manifest: WorldContinuationManifest,
}

impl CheckpointBundle {
    fn restore(&self) -> Result<ScaleContinuationState, RefError> {
        self.state.validate()?;
        self.manifest.validate().map_err(|_| RefError::Continuation)?;
        if self.manifest.world_instance != self.state.world || self.manifest.at != self.state.at {
            return Err(RefError::ManifestStateMismatch);
        }
        if self.manifest.domain_entries.len() != 1 {
            return Err(RefError::ManifestStateMismatch);
        }

        let entry = &self.manifest.domain_entries[0];
        if entry.authority != authority_id()?
            || entry.scope != root_scope()?
            || entry.reference_frame != frame_id()?
            || entry.checkpoint_at != self.state.at
            || entry.resume_identity_class != ResumeIdentityClass::Continuation
            || entry.requirement != ContinuationRequirement::RequiredExact
        {
            return Err(RefError::ManifestStateMismatch);
        }

        if !entry
            .snapshot_content
            .same_typed_value(&self.state.snapshot_content()?)
            || !entry
                .resume_identity
                .same_typed_value(&self.state.resume_identity()?)
        {
            return Err(RefError::ManifestStateMismatch);
        }

        let Some(physical_state) = &entry.physical_state else {
            return Err(RefError::ManifestStateMismatch);
        };
        if !physical_state.same_typed_value(&self.state.cell_commitment) {
            return Err(RefError::ManifestStateMismatch);
        }

        if entry.representation.as_ref() != Some(&self.state.phase.representation()?) {
            return Err(RefError::ManifestStateMismatch);
        }

        Ok(self.state.clone())
    }
}

fn capture_checkpoint(
    state: &ScaleContinuationState,
    continuation_sequence: u64,
    lifecycle_mode: LifecycleMode,
    parent: Option<&WorldContinuationManifest>,
) -> Result<CheckpointBundle, RefError> {
    state.validate()?;
    if !state.phase.is_checkpointable() {
        return Err(RefError::AmbiguousTransition);
    }

    let parent_manifest = parent
        .map(|value| value.digest().map_err(|_| RefError::Continuation))
        .transpose()?;
    let entry = DomainContinuationEntry {
        authority: authority_id()?,
        scope: root_scope()?,
        reference_frame: frame_id()?,
        checkpoint_at: state.at,
        resume_identity_class: ResumeIdentityClass::Continuation,
        resume_identity: state.resume_identity()?,
        physical_state: Some(state.cell_commitment.clone()),
        lineage: None,
        snapshot_content: state.snapshot_content()?,
        snapshot_codec: SnapshotCodecId::parse("scale-cell.snapshot-v1")?,
        representation: Some(state.phase.representation()?),
        requirement: ContinuationRequirement::RequiredExact,
        rebuild_proof: None,
    };

    let manifest = WorldContinuationManifest::new(
        state.world.clone(),
        continuation_sequence,
        lifecycle_mode,
        parent_manifest,
        state.at,
        timebase_identity()?,
        frame_id()?,
        digest("scale-cell.inactive-time-policy.v1", b"synthetic-reference")?,
        None,
        None,
        None,
        vec![entry],
        vec![],
    )
    .map_err(|_| RefError::Continuation)?;

    if let Some(parent) = parent {
        validate_manifest_lineage(&manifest, parent).map_err(|_| RefError::Lineage)?;
    }

    let bundle = CheckpointBundle {
        state: state.clone(),
        manifest,
    };
    let _ = bundle.restore()?;
    Ok(bundle)
}

fn timebase_identity() -> Result<TypedDigest32, RefError> {
    let timebase = FixedTimebase::new(
        TimebaseId::parse("scale-cell.fixed-1s.v1")?,
        digest("scale-cell.epoch.v1", b"genesis")?,
        0,
        SimInstant::GENESIS,
        1_000_000_000,
    )
    .map_err(|_| RefError::Continuation)?;
    timebase.digest().map_err(|_| RefError::Continuation)
}

fn authority_id() -> Result<AuthorityId, RefError> {
    Ok(AuthorityId::parse("scale-cell.reference-authority.v1")?)
}

fn root_scope() -> Result<ScopeId, RefError> {
    Ok(ScopeId::parse("system:fixture")?)
}

fn frame_id() -> Result<ReferenceFrameId, RefError> {
    Ok(ReferenceFrameId::parse("scale-cell:reference-frame")?)
}

fn derive_step_digest(
    domain: &str,
    previous: &TypedDigest32,
    counter: u64,
) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    push_digest(&mut bytes, previous);
    bytes.extend_from_slice(&counter.to_le_bytes());
    Ok(TypedDigest32::sha256(domain, 1, &bytes)?)
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_instant(bytes: &mut Vec<u8>, value: SimInstant) {
    bytes.extend_from_slice(&value.seconds_from_genesis.to_le_bytes());
    bytes.extend_from_slice(&value.nanos.to_le_bytes());
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

fn digest(domain: &str, value: &[u8]) -> Result<TypedDigest32, RefError> {
    Ok(TypedDigest32::sha256(domain, 1, value)?)
}

fn fixture(phase: Phase) -> ScaleContinuationState {
    ScaleContinuationState {
        world: WorldInstanceId::parse("world:scale-cell-001").unwrap(),
        at: SimInstant::new(100, 0).unwrap(),
        authority_generation: 7,
        process_counter: 11,
        cell_commitment: digest("symtropy.scale-cell-001b.authority-commitment.v1", b"cell-r7")
            .unwrap(),
        frontier_commitment: digest(
            "symtropy.scale-cell-001c.frontier.identity.v2",
            b"frontier-r7",
        )
        .unwrap(),
        phase,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PresentationState {
    resident: BTreeSet<ScopeId>,
}

impl PresentationState {
    fn prewarm(&mut self, scope: &str) -> Result<(), RefError> {
        self.resident.insert(ScopeId::parse(scope)?);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    Continuation,
    Lineage,
    AmbiguousTransition,
    ManifestStateMismatch,
    CounterOverflow,
}

impl From<symtropy_sim_contracts::ContractError> for RefError {
    fn from(_: symtropy_sim_contracts::ContractError) -> Self {
        Self::Contract
    }
}

#[test]
fn stable_coarse_checkpoint_restores_exact_continuation() {
    let state = fixture(Phase::StableCoarse);
    let bundle = capture_checkpoint(&state, 0, LifecycleMode::Genesis, None).unwrap();
    assert_eq!(bundle.restore().unwrap(), state);
}

#[test]
fn stable_fine_checkpoint_restores_exact_continuation() {
    let state = fixture(Phase::StableFine);
    let bundle = capture_checkpoint(&state, 0, LifecycleMode::Genesis, None).unwrap();
    assert_eq!(bundle.restore().unwrap(), state);
}

#[test]
fn ambiguous_transition_phases_are_not_checkpointable() {
    for phase in [Phase::PromotionPrepared, Phase::CollapsePrepared] {
        let state = fixture(phase);
        let before = state.clone();
        assert_eq!(
            capture_checkpoint(&state, 0, LifecycleMode::Genesis, None),
            Err(RefError::AmbiguousTransition)
        );
        assert_eq!(state, before);
    }
}

#[test]
fn same_physical_state_with_different_future_frontier_is_not_same_continuation() {
    let first = fixture(Phase::StableCoarse);
    let mut second = first.clone();
    second.frontier_commitment = digest(
        "symtropy.scale-cell-001c.frontier.identity.v2",
        b"different-pending-work",
    )
    .unwrap();

    assert_eq!(first.cell_commitment, second.cell_commitment);
    assert_ne!(first.snapshot_content().unwrap(), second.snapshot_content().unwrap());
    assert_ne!(first.resume_identity().unwrap(), second.resume_identity().unwrap());

    let first_bundle = capture_checkpoint(&first, 0, LifecycleMode::Genesis, None).unwrap();
    let second_bundle = capture_checkpoint(&second, 0, LifecycleMode::Genesis, None).unwrap();
    assert_ne!(
        first_bundle.manifest.digest().unwrap(),
        second_bundle.manifest.digest().unwrap()
    );
}

#[test]
fn tampered_future_frontier_cannot_restore_under_old_manifest() {
    let state = fixture(Phase::StableCoarse);
    let mut bundle = capture_checkpoint(&state, 0, LifecycleMode::Genesis, None).unwrap();
    bundle.state.frontier_commitment = digest(
        "symtropy.scale-cell-001c.frontier.identity.v2",
        b"tampered-frontier",
    )
    .unwrap();

    assert_eq!(bundle.restore(), Err(RefError::ManifestStateMismatch));
}

#[test]
fn uninterrupted_and_restore_then_continue_produce_same_world_and_manifest() {
    let initial = fixture(Phase::StableCoarse);
    let genesis = capture_checkpoint(&initial, 0, LifecycleMode::Genesis, None).unwrap();

    let uninterrupted = initial.advance_one_reference_step().unwrap();
    let uninterrupted_checkpoint = capture_checkpoint(
        &uninterrupted,
        1,
        LifecycleMode::ContinueSameWorld,
        Some(&genesis.manifest),
    )
    .unwrap();

    let restored = genesis.restore().unwrap();
    let resumed = restored.advance_one_reference_step().unwrap();
    let resumed_checkpoint = capture_checkpoint(
        &resumed,
        1,
        LifecycleMode::ContinueSameWorld,
        Some(&genesis.manifest),
    )
    .unwrap();

    assert_eq!(uninterrupted, resumed);
    assert_eq!(
        uninterrupted.resume_identity().unwrap(),
        resumed.resume_identity().unwrap()
    );
    assert_eq!(
        uninterrupted_checkpoint.manifest.digest().unwrap(),
        resumed_checkpoint.manifest.digest().unwrap()
    );
}

#[test]
fn child_manifest_binds_actual_parent_and_same_world_sequence() {
    let initial = fixture(Phase::StableCoarse);
    let genesis = capture_checkpoint(&initial, 0, LifecycleMode::Genesis, None).unwrap();
    let next = initial.advance_one_reference_step().unwrap();
    let child = capture_checkpoint(
        &next,
        1,
        LifecycleMode::ContinueSameWorld,
        Some(&genesis.manifest),
    )
    .unwrap();

    validate_manifest_lineage(&child.manifest, &genesis.manifest).unwrap();
    assert_eq!(child.manifest.world_instance, genesis.manifest.world_instance);
    assert_eq!(child.manifest.continuation_sequence, 1);
}

#[test]
fn presentation_prewarm_is_not_continuation_state() {
    let state = fixture(Phase::StableFine);
    let before = capture_checkpoint(&state, 0, LifecycleMode::Genesis, None).unwrap();
    let before_digest = before.manifest.digest().unwrap();

    let mut presentation = PresentationState::default();
    presentation.prewarm("micro:m0").unwrap();
    presentation.prewarm("body:a").unwrap();
    assert_eq!(presentation.resident.len(), 2);

    let after = capture_checkpoint(&state, 0, LifecycleMode::Genesis, None).unwrap();
    assert_eq!(before_digest, after.manifest.digest().unwrap());
}

#[test]
fn historical_manifest_cannot_restore_newer_state() {
    let initial = fixture(Phase::StableCoarse);
    let bundle = capture_checkpoint(&initial, 0, LifecycleMode::Genesis, None).unwrap();
    let newer = initial.advance_one_reference_step().unwrap();
    let forged = CheckpointBundle {
        state: newer,
        manifest: bundle.manifest,
    };
    assert_eq!(forged.restore(), Err(RefError::ManifestStateMismatch));
}
