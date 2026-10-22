// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic fabrication identity and evidence.
//!
//! This crate deliberately does not own physical matter. A [`Workpiece`] binds
//! intentional fabrication identity to opaque, revisioned allocations owned by
//! an external matter authority. Runtime ECS entities, rigid-body handles, and
//! meshes are presentation/solver consumers and never persistent fabrication
//! identity.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

/// Schema emitted by the first fabrication-foundation implementation.
pub const FABRICATION_SCHEMA_VERSION: u32 = 1;

/// Persistent identity for matter participating in intentional fabrication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkpieceId(StableId);

impl WorkpieceId {
    /// Wraps a validated deterministic game-state identifier.
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    /// Returns the underlying portable identifier.
    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for WorkpieceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Opaque reference to conserved matter owned by another authority.
///
/// `binding_digest` is intentionally not interpreted here. A Universal Matter
/// adapter can later use its canonical digest without forcing this pure domain
/// crate to duplicate material storage or geometry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterBinding {
    /// Authority that owns the referenced allocation.
    pub authority_id: StableId,
    /// Stable allocation identity within that authority.
    pub allocation_id: StableId,
    /// Exact allocation revision observed when this binding was accepted.
    pub revision: u64,
    /// Opaque integrity/provenance digest supplied by the owning authority.
    pub binding_digest: String,
}

impl MatterBinding {
    /// Creates a revisioned matter binding after validating its opaque digest.
    pub fn new(
        authority_id: StableId,
        allocation_id: StableId,
        revision: u64,
        binding_digest: impl Into<String>,
    ) -> Result<Self, FabricationError> {
        let binding_digest = binding_digest.into();
        if binding_digest.is_empty() || binding_digest.len() > 256 {
            return Err(FabricationError::InvalidMatterDigest(binding_digest));
        }
        Ok(Self {
            authority_id,
            allocation_id,
            revision,
            binding_digest,
        })
    }
}

/// Fabrication participation state. This never implies whether underlying
/// physical matter exists; only the owning matter authority can answer that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkpieceLifecycle {
    /// Material has been deliberately reserved/staged for work.
    Staged,
    /// A fabrication process currently has authority to modify the workpiece.
    InProcess,
    /// The workpiece is complete enough to be used as an assembly input.
    Available,
    /// The workpiece is currently part of an assembly.
    Installed,
    /// It was removed from an assembly but remains a reusable workpiece.
    Removed,
    /// Fabrication identity is retired; referenced matter is not deleted.
    Retired,
}

/// Persistent fabrication identity bound to one or more external matter
/// allocations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workpiece {
    /// Fabrication schema used when this record was serialized.
    pub schema_version: u32,
    /// Stable fabrication identity.
    pub id: WorkpieceId,
    /// Explicit conserved-matter references. Never an inventory count.
    pub matter_bindings: Vec<MatterBinding>,
    /// Current participation state in fabrication/assembly.
    pub lifecycle: WorkpieceLifecycle,
}

impl Workpiece {
    /// Creates a staged workpiece. At least one unique matter allocation is
    /// mandatory so fabrication identity can never float free of physical truth.
    pub fn new(
        id: WorkpieceId,
        matter_bindings: Vec<MatterBinding>,
    ) -> Result<Self, FabricationError> {
        validate_bindings(&matter_bindings)?;
        Ok(Self {
            schema_version: FABRICATION_SCHEMA_VERSION,
            id,
            matter_bindings,
            lifecycle: WorkpieceLifecycle::Staged,
        })
    }

    /// Performs a structural lifecycle transition without mutating matter
    /// bindings. Physical transformation must be committed by a matter adapter.
    pub fn transition(
        &mut self,
        next: WorkpieceLifecycle,
    ) -> Result<(), FabricationError> {
        if self.lifecycle == next {
            return Ok(());
        }
        if !valid_transition(self.lifecycle, next) {
            return Err(FabricationError::InvalidLifecycleTransition {
                from: self.lifecycle,
                to: next,
            });
        }
        self.lifecycle = next;
        Ok(())
    }

    /// Returns the exact binding for an authority-scoped matter allocation.
    pub fn binding(
        &self,
        authority_id: &StableId,
        allocation_id: &StableId,
    ) -> Option<&MatterBinding> {
        self.matter_bindings.iter().find(|binding| {
            &binding.authority_id == authority_id && &binding.allocation_id == allocation_id
        })
    }
}

fn validate_bindings(bindings: &[MatterBinding]) -> Result<(), FabricationError> {
    if bindings.is_empty() {
        return Err(FabricationError::MatterBindingRequired);
    }
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|existing| {
            existing.authority_id == binding.authority_id
                && existing.allocation_id == binding.allocation_id
        }) {
            return Err(FabricationError::DuplicateMatterAllocation {
                authority_id: binding.authority_id.clone(),
                allocation_id: binding.allocation_id.clone(),
            });
        }
    }
    Ok(())
}

const fn valid_transition(from: WorkpieceLifecycle, to: WorkpieceLifecycle) -> bool {
    use WorkpieceLifecycle::{Available, InProcess, Installed, Removed, Retired, Staged};
    matches!(
        (from, to),
        (Staged, InProcess)
            | (Staged, Available)
            | (Staged, Retired)
            | (InProcess, Available)
            | (InProcess, Retired)
            | (Available, InProcess)
            | (Available, Installed)
            | (Available, Retired)
            | (Installed, InProcess)
            | (Installed, Removed)
            | (Removed, InProcess)
            | (Removed, Available)
            | (Removed, Retired)
    )
}

/// Foundation errors are deterministic contract violations, not random
/// workmanship outcomes.
#[derive(Debug)]
pub enum FabricationError {
    MatterBindingRequired,
    InvalidMatterDigest(String),
    DuplicateMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
    InvalidLifecycleTransition {
        from: WorkpieceLifecycle,
        to: WorkpieceLifecycle,
    },
}

impl fmt::Display for FabricationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MatterBindingRequired => {
                write!(formatter, "a workpiece requires at least one matter binding")
            }
            Self::InvalidMatterDigest(digest) => write!(
                formatter,
                "matter binding digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::DuplicateMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "matter allocation {authority_id}/{allocation_id} is bound more than once"
            ),
            Self::InvalidLifecycleTransition { from, to } => {
                write!(formatter, "invalid workpiece lifecycle transition {from:?} -> {to:?}")
            }
        }
    }
}

impl Error for FabricationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn binding_from(authority: &str, allocation: &str, revision: u64) -> MatterBinding {
        MatterBinding::new(
            id(authority),
            id(allocation),
            revision,
            format!("digest:{authority}:{allocation}:{revision}"),
        )
        .expect("test binding is valid")
    }

    fn binding(allocation: &str, revision: u64) -> MatterBinding {
        binding_from("matter:universal", allocation, revision)
    }

    #[test]
    fn workpiece_requires_explicit_matter() {
        let result = Workpiece::new(WorkpieceId::new(id("workpiece:patch")), Vec::new());
        assert!(matches!(result, Err(FabricationError::MatterBindingRequired)));
    }

    #[test]
    fn one_authority_scoped_allocation_cannot_be_bound_twice() {
        let result = Workpiece::new(
            WorkpieceId::new(id("workpiece:patch")),
            vec![binding("allocation:steel", 7), binding("allocation:steel", 7)],
        );
        assert!(matches!(
            result,
            Err(FabricationError::DuplicateMatterAllocation { .. })
        ));
    }

    #[test]
    fn same_local_allocation_id_from_distinct_authorities_is_unambiguous() {
        let workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:composite")),
            vec![
                binding_from("matter:authority-a", "allocation:local-1", 2),
                binding_from("matter:authority-b", "allocation:local-1", 5),
            ],
        )
        .unwrap();

        assert_eq!(workpiece.matter_bindings.len(), 2);
        assert_eq!(
            workpiece
                .binding(&id("matter:authority-b"), &id("allocation:local-1"))
                .unwrap()
                .revision,
            5
        );
    }

    #[test]
    fn serialization_round_trip_preserves_identity_revision_and_digest() {
        let mut original = Workpiece::new(
            WorkpieceId::new(id("workpiece:patch")),
            vec![binding("allocation:steel", 7)],
        )
        .unwrap();
        original.transition(WorkpieceLifecycle::Available).unwrap();

        let bytes = serde_json::to_vec(&original).unwrap();
        let restored: Workpiece = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.matter_bindings[0].revision, 7);
        assert_eq!(
            restored.matter_bindings[0].binding_digest,
            "digest:matter:universal:allocation:steel:7"
        );
    }

    #[test]
    fn retiring_fabrication_identity_does_not_drop_matter_binding() {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:offcut")),
            vec![binding("allocation:offcut", 3)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Retired).unwrap();
        assert_eq!(workpiece.matter_bindings.len(), 1);
        assert_eq!(
            workpiece.matter_bindings[0].allocation_id,
            id("allocation:offcut")
        );
    }

    #[test]
    fn retired_workpiece_cannot_reenter_fabrication_by_local_state_change() {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:retired")),
            vec![binding("allocation:retired", 1)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Retired).unwrap();
        let result = workpiece.transition(WorkpieceLifecycle::Available);
        assert!(matches!(
            result,
            Err(FabricationError::InvalidLifecycleTransition { .. })
        ));
    }
}
