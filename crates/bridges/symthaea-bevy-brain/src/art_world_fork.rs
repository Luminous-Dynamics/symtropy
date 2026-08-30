// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy helpers for snapshot-bound world fork evidence.
//!
//! These functions describe and validate fork provenance. They do not create a
//! Bevy world, persist bytes, or mint authority.

use symthaea_reality_ledger::{
    RealityLayer, TypedDigest, WorldDescriptor, WorldId, WorldLineageId, WorldOrigin,
    WorldParentRef, WorldRelation, WorldSnapshotForkReceipt, WorldSnapshotManifest,
};

/// Bind an authority-poor ephemeral counterfactual child to one exact snapshot.
pub fn plan_ephemeral_counterfactual_fork(
    fork_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    child_world_id: impl Into<String>,
    child_lineage_id: impl Into<String>,
    creator_id: impl Into<String>,
    child_genesis_digest: TypedDigest,
) -> Result<WorldSnapshotForkReceipt, SymtropyWorldForkError> {
    build_fork(
        fork_id,
        snapshot,
        child_world_id,
        child_lineage_id,
        creator_id,
        child_genesis_digest,
        RealityLayer::Counterfactual,
        WorldOrigin::CounterfactualBranch,
        WorldRelation::CounterfactualOf,
        false,
        None,
    )
}

/// Bind a prospective persisted committed child to one exact snapshot.
///
/// The supplied authority digest authorizes persistence at the contract layer;
/// this helper still does not claim that host bytes were actually persisted.
#[allow(clippy::too_many_arguments)]
pub fn plan_persisted_committed_fork(
    fork_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    child_world_id: impl Into<String>,
    child_lineage_id: impl Into<String>,
    creator_id: impl Into<String>,
    child_genesis_digest: TypedDigest,
    persist_authority_receipt_digest: TypedDigest,
) -> Result<WorldSnapshotForkReceipt, SymtropyWorldForkError> {
    build_fork(
        fork_id,
        snapshot,
        child_world_id,
        child_lineage_id,
        creator_id,
        child_genesis_digest,
        RealityLayer::DigitalCommitted,
        WorldOrigin::DigitalHost {
            host_kind: "bevy/symtropy".into(),
        },
        WorldRelation::SpawnedFrom,
        true,
        Some(persist_authority_receipt_digest),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_fork(
    fork_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    child_world_id: impl Into<String>,
    child_lineage_id: impl Into<String>,
    creator_id: impl Into<String>,
    child_genesis_digest: TypedDigest,
    layer: RealityLayer,
    origin: WorldOrigin,
    relation: WorldRelation,
    persisted: bool,
    persist_authority_receipt_digest: Option<TypedDigest>,
) -> Result<WorldSnapshotForkReceipt, SymtropyWorldForkError> {
    snapshot
        .validate()
        .map_err(|error| SymtropyWorldForkError::Reality(error.to_string()))?;

    let child_world = WorldDescriptor {
        world_id: WorldId(child_world_id.into()),
        lineage_id: WorldLineageId(child_lineage_id.into()),
        layer,
        origin,
        parent: Some(WorldParentRef {
            world_id: snapshot.world.world_id.clone(),
            lineage_id: snapshot.world.lineage_id.clone(),
            relation,
        }),
        generation_depth: snapshot.world.generation_depth + 1,
        creator_id: creator_id.into(),
    };

    let receipt = WorldSnapshotForkReceipt {
        fork_id: fork_id.into(),
        source_snapshot_digest: snapshot
            .digest()
            .map_err(|error| SymtropyWorldForkError::Reality(error.to_string()))?,
        parent_world: snapshot.world.clone(),
        child_world,
        child_initial_state_digest: snapshot.state_digest.clone(),
        child_genesis_digest,
        persisted,
        persist_authority_receipt_digest,
    };
    receipt
        .validate_against_snapshot(snapshot)
        .map_err(|error| SymtropyWorldForkError::Reality(error.to_string()))?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymtropyWorldForkError {
    #[error("Reality Ledger snapshot-fork contract rejected operation: {0}")]
    Reality(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{DigestAlgorithm, WorldSnapshotManifest};

    fn d(domain: &str) -> TypedDigest {
        TypedDigest::blake3(domain, domain.as_bytes()).unwrap()
    }

    fn snapshot() -> WorldSnapshotManifest {
        WorldSnapshotManifest {
            schema_version: 1,
            snapshot_id: "snap".into(),
            world: WorldDescriptor {
                world_id: WorldId("studio".into()),
                lineage_id: WorldLineageId("studio-lineage".into()),
                layer: RealityLayer::DigitalCommitted,
                origin: WorldOrigin::DigitalHost {
                    host_kind: "bevy/symtropy".into(),
                },
                parent: None,
                generation_depth: 0,
                creator_id: "symthaea".into(),
            },
            genesis_digest: d("genesis.v1"),
            state_digest: TypedDigest::new(
                "symtropy.scene-state.v1",
                DigestAlgorithm::Other("fnv1a64".into()),
                "0123456789abcdef",
            )
            .unwrap(),
            ledger_head_digest: d("ledger.v1"),
            host_artifact_digest: d("artifact.v1"),
            frame: Some(20),
            previous_snapshot_digest: None,
        }
    }

    #[test]
    fn ephemeral_fork_is_counterfactual_and_authority_poor() {
        let snapshot = snapshot();
        let fork = plan_ephemeral_counterfactual_fork(
            "fork-a",
            &snapshot,
            "ghost-a",
            "ghost-a-lineage",
            "symthaea",
            d("child-genesis.v1"),
        )
        .unwrap();
        assert_eq!(fork.child_world.layer, RealityLayer::Counterfactual);
        assert!(!fork.persisted);
        assert!(fork.persist_authority_receipt_digest.is_none());
        assert!(fork
            .child_initial_state_digest
            .same_typed_value(&snapshot.state_digest));
    }

    #[test]
    fn committed_fork_is_spawned_and_authority_bound() {
        let snapshot = snapshot();
        let fork = plan_persisted_committed_fork(
            "fork-b",
            &snapshot,
            "garden-copy",
            "garden-copy-lineage",
            "symthaea",
            d("child-genesis.v1"),
            d("persist-authority.v1"),
        )
        .unwrap();
        assert_eq!(fork.child_world.layer, RealityLayer::DigitalCommitted);
        assert!(fork.persisted);
        assert!(fork.persist_authority_receipt_digest.is_some());
        assert_eq!(
            fork.child_world.parent.as_ref().unwrap().relation,
            WorldRelation::SpawnedFrom
        );
    }

    #[test]
    fn child_identity_cannot_reuse_parent_identity() {
        let snapshot = snapshot();
        assert!(plan_ephemeral_counterfactual_fork(
            "fork-c",
            &snapshot,
            snapshot.world.world_id.0.clone(),
            snapshot.world.lineage_id.0.clone(),
            "symthaea",
            d("child-genesis.v1"),
        )
        .is_err());
    }
}
