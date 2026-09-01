// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Portable, serializer-independent world continuation identity primitives.
//!
//! These contracts own no domain state. They bind domain-owned continuation
//! identities, restorable snapshot content, timebase identity, and hierarchical
//! child manifests into one canonical world-continuation root.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

use crate::{
    hash_string, hash_typed_digest, AuthorityId, ContractError, DigestAlgorithm, ReferenceFrameId,
    RepresentationId, ScopeId, SimInstant, SnapshotCodecId, TimebaseId, TypedDigest32,
    WorldInstanceId, NANOS_PER_SECOND,
};

pub const WORLD_CONTINUATION_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const FIXED_TIMEBASE_SCHEMA_VERSION: u32 = 1;

const WORLD_CONTINUATION_MANIFEST_IDENTITY_DOMAIN: &str =
    "symtropy.world-continuation-manifest.identity.v1";
const FIXED_TIMEBASE_IDENTITY_DOMAIN: &str = "symtropy.fixed-timebase.identity.v1";
const IDENTITY_DIGEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleMode {
    Genesis,
    ContinueSameWorld,
    ForkNewWorld,
}

impl LifecycleMode {
    const fn stable_code(self) -> u8 {
        match self {
            Self::Genesis => 0,
            Self::ContinueSameWorld => 1,
            Self::ForkNewWorld => 2,
        }
    }
}

/// What semantic claim `resume_identity` makes for one domain entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResumeIdentityClass {
    /// Present physical state is complete enough to resume this authority exactly.
    PhysicalState,
    /// A stronger continuation identity binds hidden scheduler/frontier/counter state.
    Continuation,
}

impl ResumeIdentityClass {
    const fn stable_code(self) -> u8 {
        match self {
            Self::PhysicalState => 0,
            Self::Continuation => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationRequirement {
    RequiredExact,
    RebuildableWithProof,
}

impl ContinuationRequirement {
    const fn stable_code(self) -> u8 {
        match self {
            Self::RequiredExact => 0,
            Self::RebuildableWithProof => 1,
        }
    }
}

/// Exact fixed-step mapping between local ticks and the shared `SimInstant` timeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedTimebase {
    pub schema_version: u32,
    pub id: TimebaseId,
    pub genesis_or_epoch: TypedDigest32,
    pub origin_tick: u64,
    pub origin_instant: SimInstant,
    pub step_nanoseconds: u64,
}

impl FixedTimebase {
    pub fn new(
        id: TimebaseId,
        genesis_or_epoch: TypedDigest32,
        origin_tick: u64,
        origin_instant: SimInstant,
        step_nanoseconds: u64,
    ) -> Result<Self, ContinuationError> {
        let timebase = Self {
            schema_version: FIXED_TIMEBASE_SCHEMA_VERSION,
            id,
            genesis_or_epoch,
            origin_tick,
            origin_instant,
            step_nanoseconds,
        };
        timebase.validate()?;
        Ok(timebase)
    }

    pub fn validate(&self) -> Result<(), ContinuationError> {
        if self.schema_version != FIXED_TIMEBASE_SCHEMA_VERSION {
            return Err(ContinuationError::UnsupportedTimebaseSchema {
                expected: FIXED_TIMEBASE_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.id.validate()?;
        self.genesis_or_epoch.validate()?;
        self.origin_instant.validate()?;
        if self.step_nanoseconds == 0 {
            return Err(ContinuationError::ZeroTimebaseStep);
        }
        Ok(())
    }

    /// Map a local fixed-step tick to the common simulation timeline with checked
    /// integer arithmetic. No floating-point time participates in canonical identity.
    pub fn tick_to_instant(&self, tick: u64) -> Result<SimInstant, ContinuationError> {
        self.validate()?;
        let billion = i128::from(NANOS_PER_SECOND);
        let origin_total = i128::from(self.origin_instant.seconds_from_genesis)
            .checked_mul(billion)
            .and_then(|value| value.checked_add(i128::from(self.origin_instant.nanos)))
            .ok_or(ContinuationError::TimebaseOverflow)?;
        let delta_ticks = i128::from(tick) - i128::from(self.origin_tick);
        let delta_ns = delta_ticks
            .checked_mul(i128::from(self.step_nanoseconds))
            .ok_or(ContinuationError::TimebaseOverflow)?;
        let total = origin_total
            .checked_add(delta_ns)
            .ok_or(ContinuationError::TimebaseOverflow)?;
        let seconds = total.div_euclid(billion);
        let nanos = total.rem_euclid(billion);
        let seconds_from_genesis =
            i64::try_from(seconds).map_err(|_| ContinuationError::TimebaseOverflow)?;
        let nanos = u32::try_from(nanos).map_err(|_| ContinuationError::TimebaseOverflow)?;
        SimInstant::new(seconds_from_genesis, nanos).map_err(ContinuationError::Contract)
    }

    /// Reverse an exact tick mapping. Instants between fixed ticks are rejected;
    /// callers must use a separately identified interpolation/quantization policy.
    pub fn instant_to_tick(&self, instant: SimInstant) -> Result<u64, ContinuationError> {
        self.validate()?;
        instant.validate()?;
        let delta_ns = instant.nanoseconds_since(self.origin_instant);
        let step = i128::from(self.step_nanoseconds);
        if delta_ns.rem_euclid(step) != 0 {
            return Err(ContinuationError::InstantNotTickAligned);
        }
        let delta_ticks = delta_ns.div_euclid(step);
        let tick = i128::from(self.origin_tick)
            .checked_add(delta_ticks)
            .ok_or(ContinuationError::TimebaseOverflow)?;
        u64::try_from(tick).map_err(|_| ContinuationError::TickOutOfRange)
    }

    pub fn digest(&self) -> Result<TypedDigest32, ContinuationError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.fixed-timebase.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.id.as_str());
        hash_typed_digest(&mut hasher, &self.genesis_or_epoch);
        hasher.update(self.origin_tick.to_le_bytes());
        hasher.update(self.origin_instant.seconds_from_genesis.to_le_bytes());
        hasher.update(self.origin_instant.nanos.to_le_bytes());
        hasher.update(self.step_nanoseconds.to_le_bytes());
        Ok(TypedDigest32::new(
            FIXED_TIMEBASE_IDENTITY_DOMAIN,
            DigestAlgorithm::Sha256,
            IDENTITY_DIGEST_SCHEMA_VERSION,
            hasher.finalize().into(),
        )?)
    }
}

/// One domain-owned restorable continuation binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainContinuationEntry {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub checkpoint_at: SimInstant,
    pub resume_identity_class: ResumeIdentityClass,
    pub resume_identity: TypedDigest32,
    pub physical_state: Option<TypedDigest32>,
    pub lineage: Option<TypedDigest32>,
    pub snapshot_content: TypedDigest32,
    pub snapshot_codec: SnapshotCodecId,
    pub representation: Option<RepresentationId>,
    pub requirement: ContinuationRequirement,
    pub rebuild_proof: Option<TypedDigest32>,
}

impl DomainContinuationEntry {
    pub fn validate(&self) -> Result<(), ContinuationError> {
        self.authority.validate()?;
        self.scope.validate()?;
        self.reference_frame.validate()?;
        self.checkpoint_at.validate()?;
        self.snapshot_codec.validate()?;
        if let Some(representation) = &self.representation {
            representation.validate()?;
        }
        self.resume_identity.validate()?;
        self.snapshot_content.validate()?;
        validate_optional_digest(&self.physical_state)?;
        validate_optional_digest(&self.lineage)?;
        validate_optional_digest(&self.rebuild_proof)?;

        if self.resume_identity_class == ResumeIdentityClass::PhysicalState {
            let physical = self
                .physical_state
                .as_ref()
                .ok_or(ContinuationError::MissingPhysicalState)?;
            if !physical.same_typed_value(&self.resume_identity) {
                return Err(ContinuationError::PhysicalResumeIdentityMismatch);
            }
        }

        match self.requirement {
            ContinuationRequirement::RequiredExact if self.rebuild_proof.is_some() => {
                Err(ContinuationError::UnexpectedRebuildProof)
            }
            ContinuationRequirement::RebuildableWithProof if self.rebuild_proof.is_none() => {
                Err(ContinuationError::MissingRebuildProof)
            }
            _ => Ok(()),
        }
    }

    fn hash_into(&self, hasher: &mut Sha256) {
        hash_string(hasher, self.authority.as_str());
        hash_string(hasher, self.scope.as_str());
        hash_string(hasher, self.reference_frame.as_str());
        hasher.update(self.checkpoint_at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.checkpoint_at.nanos.to_le_bytes());
        hasher.update([self.resume_identity_class.stable_code()]);
        hash_typed_digest(hasher, &self.resume_identity);
        hash_optional_digest(hasher, self.physical_state.as_ref());
        hash_optional_digest(hasher, self.lineage.as_ref());
        hash_typed_digest(hasher, &self.snapshot_content);
        hash_string(hasher, self.snapshot_codec.as_str());
        match &self.representation {
            Some(value) => {
                hasher.update([1]);
                hash_string(hasher, value.as_str());
            }
            None => hasher.update([0]),
        }
        hasher.update([self.requirement.stable_code()]);
        hash_optional_digest(hasher, self.rebuild_proof.as_ref());
    }
}

/// Content-addressed child subtree in a hierarchical continuation manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildManifestRef {
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub manifest_digest: TypedDigest32,
}

impl ChildManifestRef {
    pub fn validate(&self) -> Result<(), ContinuationError> {
        self.scope.validate()?;
        self.reference_frame.validate()?;
        validate_digest_semantics(
            &self.manifest_digest,
            "child_manifest.manifest_digest",
            WORLD_CONTINUATION_MANIFEST_IDENTITY_DOMAIN,
            IDENTITY_DIGEST_SCHEMA_VERSION,
        )?;
        Ok(())
    }

    fn hash_into(&self, hasher: &mut Sha256) {
        hash_string(hasher, self.scope.as_str());
        hash_string(hasher, self.reference_frame.as_str());
        hash_typed_digest(hasher, &self.manifest_digest);
    }
}

/// Portable semantic root for exact world continuation.
///
/// Domain entries and child references are canonicalized during digesting, so
/// arrival/storage order cannot change identity. This type contains identities
/// and artifact references only; it owns no terrain, water, ecology, or other
/// mutable domain truth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldContinuationManifest {
    pub schema_version: u32,
    pub world_instance: WorldInstanceId,
    pub continuation_sequence: u64,
    pub lifecycle_mode: LifecycleMode,
    pub parent_manifest: Option<TypedDigest32>,
    pub at: SimInstant,
    pub timebase_identity: TypedDigest32,
    pub reference_frame: ReferenceFrameId,
    pub inactive_time_policy: TypedDigest32,
    pub forcing_context: Option<TypedDigest32>,
    pub causal_journal_head: Option<TypedDigest32>,
    pub distributed_authority_context: Option<TypedDigest32>,
    pub domain_entries: Vec<DomainContinuationEntry>,
    pub child_manifests: Vec<ChildManifestRef>,
}

impl WorldContinuationManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_instance: WorldInstanceId,
        continuation_sequence: u64,
        lifecycle_mode: LifecycleMode,
        parent_manifest: Option<TypedDigest32>,
        at: SimInstant,
        timebase_identity: TypedDigest32,
        reference_frame: ReferenceFrameId,
        inactive_time_policy: TypedDigest32,
        forcing_context: Option<TypedDigest32>,
        causal_journal_head: Option<TypedDigest32>,
        distributed_authority_context: Option<TypedDigest32>,
        domain_entries: Vec<DomainContinuationEntry>,
        child_manifests: Vec<ChildManifestRef>,
    ) -> Result<Self, ContinuationError> {
        let manifest = Self {
            schema_version: WORLD_CONTINUATION_MANIFEST_SCHEMA_VERSION,
            world_instance,
            continuation_sequence,
            lifecycle_mode,
            parent_manifest,
            at,
            timebase_identity,
            reference_frame,
            inactive_time_policy,
            forcing_context,
            causal_journal_head,
            distributed_authority_context,
            domain_entries,
            child_manifests,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ContinuationError> {
        if self.schema_version != WORLD_CONTINUATION_MANIFEST_SCHEMA_VERSION {
            return Err(ContinuationError::UnsupportedManifestSchema {
                expected: WORLD_CONTINUATION_MANIFEST_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.world_instance.validate()?;
        self.at.validate()?;
        self.reference_frame.validate()?;
        validate_lifecycle(
            self.lifecycle_mode,
            self.continuation_sequence,
            self.parent_manifest.as_ref(),
        )?;
        validate_optional_digest_semantics(
            self.parent_manifest.as_ref(),
            "world_manifest.parent_manifest",
            WORLD_CONTINUATION_MANIFEST_IDENTITY_DOMAIN,
            IDENTITY_DIGEST_SCHEMA_VERSION,
        )?;
        validate_digest_semantics(
            &self.timebase_identity,
            "world_manifest.timebase_identity",
            FIXED_TIMEBASE_IDENTITY_DOMAIN,
            IDENTITY_DIGEST_SCHEMA_VERSION,
        )?;
        self.inactive_time_policy.validate()?;
        validate_optional_digest(&self.forcing_context)?;
        validate_optional_digest(&self.causal_journal_head)?;
        validate_optional_digest(&self.distributed_authority_context)?;
        for entry in &self.domain_entries {
            entry.validate()?;
            if entry.checkpoint_at != self.at {
                return Err(ContinuationError::DomainCheckpointMismatch);
            }
        }
        for child in &self.child_manifests {
            child.validate()?;
        }
        reject_duplicate_domain_bindings(&self.domain_entries)?;
        reject_duplicate_child_scopes(&self.child_manifests)?;
        Ok(())
    }

    pub fn digest(&self) -> Result<TypedDigest32, ContinuationError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.world-continuation-manifest.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.world_instance.as_str());
        hasher.update(self.continuation_sequence.to_le_bytes());
        hasher.update([self.lifecycle_mode.stable_code()]);
        hash_optional_digest(&mut hasher, self.parent_manifest.as_ref());
        hasher.update(self.at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.at.nanos.to_le_bytes());
        hash_typed_digest(&mut hasher, &self.timebase_identity);
        hash_string(&mut hasher, self.reference_frame.as_str());
        hash_typed_digest(&mut hasher, &self.inactive_time_policy);
        hash_optional_digest(&mut hasher, self.forcing_context.as_ref());
        hash_optional_digest(&mut hasher, self.causal_journal_head.as_ref());
        hash_optional_digest(&mut hasher, self.distributed_authority_context.as_ref());

        let mut entries: Vec<_> = self.domain_entries.iter().collect();
        entries.sort_by(|left, right| {
            left.scope
                .as_str()
                .cmp(right.scope.as_str())
                .then_with(|| left.authority.as_str().cmp(right.authority.as_str()))
        });
        hasher.update((entries.len() as u64).to_le_bytes());
        for entry in entries {
            entry.hash_into(&mut hasher);
        }

        let mut children: Vec<_> = self.child_manifests.iter().collect();
        children.sort_by(|left, right| {
            left.scope
                .as_str()
                .cmp(right.scope.as_str())
                .then_with(|| left.reference_frame.as_str().cmp(right.reference_frame.as_str()))
                .then_with(|| left.manifest_digest.value.cmp(&right.manifest_digest.value))
        });
        hasher.update((children.len() as u64).to_le_bytes());
        for child in children {
            child.hash_into(&mut hasher);
        }

        Ok(TypedDigest32::new(
            WORLD_CONTINUATION_MANIFEST_IDENTITY_DOMAIN,
            DigestAlgorithm::Sha256,
            IDENTITY_DIGEST_SCHEMA_VERSION,
            hasher.finalize().into(),
        )?)
    }
}

fn validate_lifecycle(
    mode: LifecycleMode,
    sequence: u64,
    parent: Option<&TypedDigest32>,
) -> Result<(), ContinuationError> {
    let valid = match mode {
        LifecycleMode::Genesis => sequence == 0 && parent.is_none(),
        LifecycleMode::ContinueSameWorld => sequence > 0 && parent.is_some(),
        LifecycleMode::ForkNewWorld => sequence == 0 && parent.is_some(),
    };
    if valid {
        Ok(())
    } else {
        Err(ContinuationError::InvalidLifecycleParent)
    }
}

fn reject_duplicate_domain_bindings(
    entries: &[DomainContinuationEntry],
) -> Result<(), ContinuationError> {
    let mut keys: Vec<_> = entries
        .iter()
        .map(|entry| (entry.scope.as_str(), entry.authority.as_str()))
        .collect();
    keys.sort_unstable();
    for pair in keys.windows(2) {
        if pair[0] == pair[1] {
            return Err(ContinuationError::DuplicateDomainBinding);
        }
    }
    Ok(())
}

fn reject_duplicate_child_scopes(children: &[ChildManifestRef]) -> Result<(), ContinuationError> {
    let mut scopes: Vec<_> = children.iter().map(|child| child.scope.as_str()).collect();
    scopes.sort_unstable();
    for pair in scopes.windows(2) {
        if pair[0] == pair[1] {
            return Err(ContinuationError::DuplicateChildScope);
        }
    }
    Ok(())
}

fn validate_optional_digest(value: &Option<TypedDigest32>) -> Result<(), ContinuationError> {
    if let Some(value) = value {
        value.validate()?;
    }
    Ok(())
}

fn validate_digest_semantics(
    value: &TypedDigest32,
    slot: &'static str,
    expected_domain: &'static str,
    expected_schema: u32,
) -> Result<(), ContinuationError> {
    value.validate()?;
    if value.domain != expected_domain || value.schema_version != expected_schema {
        return Err(ContinuationError::UnexpectedDigestSemantics {
            slot,
            expected_domain,
            expected_schema,
            actual_domain: value.domain.clone(),
            actual_schema: value.schema_version,
        });
    }
    Ok(())
}

fn validate_optional_digest_semantics(
    value: Option<&TypedDigest32>,
    slot: &'static str,
    expected_domain: &'static str,
    expected_schema: u32,
) -> Result<(), ContinuationError> {
    if let Some(value) = value {
        validate_digest_semantics(value, slot, expected_domain, expected_schema)?;
    }
    Ok(())
}

fn hash_optional_digest(hasher: &mut Sha256, value: Option<&TypedDigest32>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hash_typed_digest(hasher, value);
        }
        None => hasher.update([0]),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationError {
    Contract(ContractError),
    UnsupportedTimebaseSchema { expected: u32, actual: u32 },
    UnsupportedManifestSchema { expected: u32, actual: u32 },
    UnexpectedDigestSemantics {
        slot: &'static str,
        expected_domain: &'static str,
        expected_schema: u32,
        actual_domain: String,
        actual_schema: u32,
    },
    ZeroTimebaseStep,
    TimebaseOverflow,
    TickOutOfRange,
    InstantNotTickAligned,
    InvalidLifecycleParent,
    DuplicateDomainBinding,
    DuplicateChildScope,
    DomainCheckpointMismatch,
    MissingPhysicalState,
    PhysicalResumeIdentityMismatch,
    MissingRebuildProof,
    UnexpectedRebuildProof,
}

impl From<ContractError> for ContinuationError {
    fn from(value: ContractError) -> Self {
        Self::Contract(value)
    }
}

impl fmt::Display for ContinuationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => {
                write!(formatter, "simulation contract rejected continuation data: {error}")
            }
            Self::UnsupportedTimebaseSchema { expected, actual } => write!(
                formatter,
                "unsupported fixed timebase schema {actual}; expected {expected}"
            ),
            Self::UnsupportedManifestSchema { expected, actual } => write!(
                formatter,
                "unsupported world continuation manifest schema {actual}; expected {expected}"
            ),
            Self::UnexpectedDigestSemantics {
                slot,
                expected_domain,
                expected_schema,
                actual_domain,
                actual_schema,
            } => write!(
                formatter,
                "continuation digest slot {slot} expects {expected_domain} schema {expected_schema}, got {actual_domain} schema {actual_schema}"
            ),
            Self::ZeroTimebaseStep => formatter.write_str("fixed timebase step must be non-zero"),
            Self::TimebaseOverflow => formatter.write_str("fixed timebase conversion overflow"),
            Self::TickOutOfRange => {
                formatter.write_str("simulation instant maps outside the tick range")
            }
            Self::InstantNotTickAligned => {
                formatter.write_str("simulation instant does not land exactly on a fixed tick")
            }
            Self::InvalidLifecycleParent => formatter.write_str(
                "world lifecycle mode, continuation sequence, and parent manifest are inconsistent",
            ),
            Self::DuplicateDomainBinding => formatter.write_str(
                "world continuation manifest contains duplicate authority/scope binding",
            ),
            Self::DuplicateChildScope => {
                formatter.write_str("world continuation manifest contains duplicate child scope")
            }
            Self::DomainCheckpointMismatch => formatter.write_str(
                "domain continuation checkpoint instant does not match enclosing manifest instant",
            ),
            Self::MissingPhysicalState => formatter.write_str(
                "physical-state resume identity requires an explicit physical-state digest",
            ),
            Self::PhysicalResumeIdentityMismatch => formatter.write_str(
                "physical-state resume identity must equal the declared physical-state digest",
            ),
            Self::MissingRebuildProof => formatter.write_str(
                "rebuildable continuation entry requires deterministic rebuild proof identity",
            ),
            Self::UnexpectedRebuildProof => formatter.write_str(
                "required-exact continuation entry cannot substitute a rebuild proof for exact state",
            ),
        }
    }
}

impl Error for ContinuationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Contract(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(domain: &str, value: &[u8]) -> TypedDigest32 {
        TypedDigest32::sha256(domain, 1, value).unwrap()
    }

    fn timebase(step_nanoseconds: u64) -> FixedTimebase {
        FixedTimebase::new(
            TimebaseId::parse("gameplay.fixed.test.v1").unwrap(),
            d("symtropy.test.genesis.v1", b"world-genesis"),
            0,
            SimInstant::GENESIS,
            step_nanoseconds,
        )
        .unwrap()
    }

    fn continuation_entry(scope: &str, resume: &[u8]) -> DomainContinuationEntry {
        DomainContinuationEntry {
            authority: AuthorityId::parse("hydrology.authority.v1").unwrap(),
            scope: ScopeId::parse(scope).unwrap(),
            reference_frame: ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            checkpoint_at: SimInstant::new(20, 0).unwrap(),
            resume_identity_class: ResumeIdentityClass::Continuation,
            resume_identity: d("symtropy.hydrology.continuation.v1", resume),
            physical_state: Some(d("symtropy.hydrology.water-state.v1", b"same-water")),
            lineage: None,
            snapshot_content: d("symtropy.snapshot.content.v1", scope.as_bytes()),
            snapshot_codec: SnapshotCodecId::parse("hydrology.snapshot.v1").unwrap(),
            representation: Some(RepresentationId::parse("hydrology.sparse-water.v1").unwrap()),
            requirement: ContinuationRequirement::RequiredExact,
            rebuild_proof: None,
        }
    }

    fn physical_entry(scope: &str) -> DomainContinuationEntry {
        let physical = d("symtropy.surface-water.state.v1", b"water");
        DomainContinuationEntry {
            authority: AuthorityId::parse("surface-water.authority.v1").unwrap(),
            scope: ScopeId::parse(scope).unwrap(),
            reference_frame: ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            checkpoint_at: SimInstant::new(20, 0).unwrap(),
            resume_identity_class: ResumeIdentityClass::PhysicalState,
            resume_identity: physical.clone(),
            physical_state: Some(physical),
            lineage: None,
            snapshot_content: d("symtropy.snapshot.content.v1", b"surface-water"),
            snapshot_codec: SnapshotCodecId::parse("surface-water.snapshot.v1").unwrap(),
            representation: None,
            requirement: ContinuationRequirement::RequiredExact,
            rebuild_proof: None,
        }
    }

    fn child(scope: &str, value: &[u8]) -> ChildManifestRef {
        ChildManifestRef {
            scope: ScopeId::parse(scope).unwrap(),
            reference_frame: ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            manifest_digest: d(WORLD_CONTINUATION_MANIFEST_IDENTITY_DOMAIN, value),
        }
    }

    fn manifest(
        world: &str,
        entries: Vec<DomainContinuationEntry>,
        children: Vec<ChildManifestRef>,
        policy: &[u8],
    ) -> WorldContinuationManifest {
        WorldContinuationManifest::new(
            WorldInstanceId::parse(world).unwrap(),
            0,
            LifecycleMode::Genesis,
            None,
            SimInstant::new(20, 0).unwrap(),
            timebase(50_000_000).digest().unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", policy),
            None,
            None,
            None,
            entries,
            children,
        )
        .unwrap()
    }

    #[test]
    fn twenty_hz_tick_400_is_exactly_twenty_seconds() {
        let timebase = timebase(50_000_000);
        let instant = timebase.tick_to_instant(400).unwrap();
        assert_eq!(instant, SimInstant::new(20, 0).unwrap());
        assert_eq!(timebase.instant_to_tick(instant).unwrap(), 400);
    }

    #[test]
    fn non_aligned_instant_is_rejected() {
        let timebase = timebase(50_000_000);
        let result = timebase.instant_to_tick(SimInstant::new(20, 1).unwrap());
        assert_eq!(result, Err(ContinuationError::InstantNotTickAligned));
    }

    #[test]
    fn timebase_identity_changes_with_step() {
        assert_ne!(
            timebase(50_000_000).digest().unwrap(),
            timebase(10_000_000).digest().unwrap()
        );
    }

    #[test]
    fn domain_arrival_order_does_not_change_manifest_identity() {
        let a = continuation_entry("sol:earth:basin/a", b"resume-a");
        let b = continuation_entry("sol:earth:basin/b", b"resume-b");
        let forward = manifest("world:test", vec![a.clone(), b.clone()], vec![], b"paused");
        let reverse = manifest("world:test", vec![b, a], vec![], b"paused");
        assert_eq!(forward.digest().unwrap(), reverse.digest().unwrap());
    }

    #[test]
    fn child_arrival_order_does_not_change_manifest_identity() {
        let a = child("sol:earth:region/a", b"child-a");
        let b = child("sol:earth:region/b", b"child-b");
        let forward = manifest("world:test", vec![], vec![a.clone(), b.clone()], b"paused");
        let reverse = manifest("world:test", vec![], vec![b, a], b"paused");
        assert_eq!(forward.digest().unwrap(), reverse.digest().unwrap());
    }

    #[test]
    fn duplicate_authority_scope_binding_fails_closed() {
        let a = continuation_entry("sol:earth:basin/a", b"resume-a");
        let b = continuation_entry("sol:earth:basin/a", b"resume-b");
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            0,
            LifecycleMode::Genesis,
            None,
            SimInstant::new(20, 0).unwrap(),
            timebase(50_000_000).digest().unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![a, b],
            vec![],
        );
        assert_eq!(result.unwrap_err(), ContinuationError::DuplicateDomainBinding);
    }

    #[test]
    fn hidden_continuation_state_changes_root_while_physical_state_can_match() {
        let a = continuation_entry("sol:earth:basin/a", b"active-frontier-a");
        let b = continuation_entry("sol:earth:basin/a", b"active-frontier-b");
        assert_eq!(a.physical_state, b.physical_state);
        assert_ne!(a.resume_identity, b.resume_identity);
        assert_ne!(
            manifest("world:test", vec![a], vec![], b"paused")
                .digest()
                .unwrap(),
            manifest("world:test", vec![b], vec![], b"paused")
                .digest()
                .unwrap()
        );
    }

    #[test]
    fn physical_resume_claim_must_equal_physical_state_digest() {
        let mut entry = physical_entry("sol:earth:surface/a");
        entry.resume_identity = d("symtropy.surface-water.state.v1", b"different");
        assert_eq!(
            entry.validate(),
            Err(ContinuationError::PhysicalResumeIdentityMismatch)
        );
    }

    #[test]
    fn domain_checkpoint_must_match_manifest_instant() {
        let mut entry = continuation_entry("sol:earth:basin/a", b"resume-a");
        entry.checkpoint_at = SimInstant::new(19, 0).unwrap();
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            0,
            LifecycleMode::Genesis,
            None,
            SimInstant::new(20, 0).unwrap(),
            timebase(50_000_000).digest().unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![entry],
            vec![],
        );
        assert_eq!(result.unwrap_err(), ContinuationError::DomainCheckpointMismatch);
    }

    #[test]
    fn inactive_time_policy_is_part_of_world_identity() {
        let paused = manifest("world:test", vec![], vec![], b"paused");
        let catchup = manifest("world:test", vec![], vec![], b"catchup");
        assert_ne!(paused.digest().unwrap(), catchup.digest().unwrap());
    }

    #[test]
    fn distributed_authority_context_is_identity_significant() {
        let base = manifest("world:test", vec![], vec![], b"paused");
        let mut networked = base.clone();
        networked.distributed_authority_context = Some(d(
            "symtropy.distributed-authority.context.v1",
            b"owner=peer-b;epoch=7",
        ));
        assert_ne!(base.digest().unwrap(), networked.digest().unwrap());
    }

    #[test]
    fn lifecycle_parent_rules_fail_closed() {
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            1,
            LifecycleMode::ContinueSameWorld,
            None,
            SimInstant::GENESIS,
            timebase(50_000_000).digest().unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![],
            vec![],
        );
        assert_eq!(result.unwrap_err(), ContinuationError::InvalidLifecycleParent);
    }

    #[test]
    fn wrong_parent_manifest_digest_semantics_are_rejected() {
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            1,
            LifecycleMode::ContinueSameWorld,
            Some(d("symtropy.hydrology.water-state.v1", b"not-a-manifest")),
            SimInstant::new(20, 0).unwrap(),
            timebase(50_000_000).digest().unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![],
            vec![],
        );
        assert!(matches!(
            result,
            Err(ContinuationError::UnexpectedDigestSemantics {
                slot: "world_manifest.parent_manifest",
                ..
            })
        ));
    }

    #[test]
    fn wrong_child_manifest_digest_semantics_are_rejected() {
        let child = ChildManifestRef {
            scope: ScopeId::parse("sol:earth:region/a").unwrap(),
            reference_frame: ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            manifest_digest: d("symtropy.ecology.state.v1", b"not-a-manifest"),
        };
        assert!(matches!(
            child.validate(),
            Err(ContinuationError::UnexpectedDigestSemantics {
                slot: "child_manifest.manifest_digest",
                ..
            })
        ));
    }

    #[test]
    fn wrong_timebase_digest_semantics_are_rejected() {
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            0,
            LifecycleMode::Genesis,
            None,
            SimInstant::new(20, 0).unwrap(),
            d("symtropy.weather.forcing.v1", b"not-a-timebase"),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![],
            vec![],
        );
        assert!(matches!(
            result,
            Err(ContinuationError::UnexpectedDigestSemantics {
                slot: "world_manifest.timebase_identity",
                ..
            })
        ));
    }

    #[test]
    fn wrong_timebase_digest_schema_is_rejected() {
        let mut wrong_schema = timebase(50_000_000).digest().unwrap();
        wrong_schema.schema_version = 2;
        let result = WorldContinuationManifest::new(
            WorldInstanceId::parse("world:test").unwrap(),
            0,
            LifecycleMode::Genesis,
            None,
            SimInstant::new(20, 0).unwrap(),
            wrong_schema,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![],
            vec![],
        );
        assert!(matches!(
            result,
            Err(ContinuationError::UnexpectedDigestSemantics {
                expected_schema: 1,
                actual_schema: 2,
                ..
            })
        ));
    }

    #[test]
    fn invalid_deserialized_identity_is_rejected_by_validation() {
        let manifest = manifest("world:test", vec![], vec![], b"paused");
        let mut value = serde_json::to_value(&manifest).unwrap();
        value["world_instance"] = serde_json::Value::String("bad world id".to_owned());
        let restored: WorldContinuationManifest = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.validate(),
            Err(ContinuationError::Contract(ContractError::InvalidIdentity { .. }))
        ));
    }

    #[test]
    fn json_round_trip_does_not_define_but_preserves_manifest_identity() {
        let manifest = manifest(
            "world:test",
            vec![
                continuation_entry("sol:earth:basin/a", b"resume"),
                physical_entry("sol:earth:surface/b"),
            ],
            vec![child("sol:earth:region/c", b"child")],
            b"paused",
        );
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: WorldContinuationManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest.digest().unwrap(), restored.digest().unwrap());
    }
}
