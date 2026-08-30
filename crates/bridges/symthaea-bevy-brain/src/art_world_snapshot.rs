// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strong Symtropy snapshot helpers that hash the actual persisted artifact bytes.

use symthaea_reality_ledger::{TypedDigest, WorldSnapshotManifest};

use crate::{
    art_reality_episode::InhabitedWorldEpisodeReceipt,
    art_world_lifecycle::{snapshot_closed_episode, SymtropyWorldLifecycleError},
};

pub const SYMTROPY_WORLD_SNAPSHOT_ARTIFACT_DOMAIN: &str =
    "symtropy.world-snapshot-artifact.v1";

/// Create the first snapshot in a persisted world-snapshot lineage by hashing
/// the exact bytes supplied by the host with BLAKE3.
pub fn snapshot_closed_episode_from_bytes(
    snapshot_id: impl Into<String>,
    receipt: &InhabitedWorldEpisodeReceipt,
    persisted_artifact_bytes: &[u8],
) -> Result<WorldSnapshotManifest, SymtropyWorldLifecycleError> {
    let artifact = TypedDigest::blake3(
        SYMTROPY_WORLD_SNAPSHOT_ARTIFACT_DOMAIN,
        persisted_artifact_bytes,
    )
    .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    snapshot_closed_episode(snapshot_id, receipt, artifact, None)
}

/// Create a successor snapshot and prove that it is linked to the exact prior
/// manifest in the same world lineage without frame regression.
pub fn successor_snapshot_closed_episode_from_bytes(
    snapshot_id: impl Into<String>,
    receipt: &InhabitedWorldEpisodeReceipt,
    persisted_artifact_bytes: &[u8],
    previous: &WorldSnapshotManifest,
) -> Result<WorldSnapshotManifest, SymtropyWorldLifecycleError> {
    let artifact = TypedDigest::blake3(
        SYMTROPY_WORLD_SNAPSHOT_ARTIFACT_DOMAIN,
        persisted_artifact_bytes,
    )
    .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    let previous_digest = previous
        .digest()
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    let snapshot = snapshot_closed_episode(
        snapshot_id,
        receipt,
        artifact,
        Some(previous_digest),
    )?;
    snapshot
        .validate_successor(previous)
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{DeterminismClass, DigestAlgorithm};

    use crate::{
        art_reality_adapter::SymtropyRealityBinding,
        art_reality_episode::InhabitedWorldEpisode,
    };

    fn d(domain: &str) -> TypedDigest {
        TypedDigest::blake3(domain, domain.as_bytes()).unwrap()
    }

    fn binding() -> SymtropyRealityBinding {
        SymtropyRealityBinding::new(
            "studio",
            "studio-lineage",
            "symthaea",
            "symtropy",
            "symtropy.scene-state.v1",
            "symtropy.capture-artifact.v1",
            DigestAlgorithm::Blake3,
        )
        .unwrap()
    }

    fn receipt(id: &str, exit_scene: &str, exit_frame: u64) -> InhabitedWorldEpisodeReceipt {
        InhabitedWorldEpisode::open(
            id,
            binding(),
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            d("kernel.v1"),
            d("physics.v1"),
            d("assets.v1"),
            "scene-a",
            DeterminismClass::Deterministic,
            None,
            "studio-frame",
            10,
        )
        .unwrap()
        .close(exit_scene, exit_frame)
        .unwrap()
    }

    #[test]
    fn actual_artifact_bytes_are_blake3_bound() {
        let snapshot = snapshot_closed_episode_from_bytes(
            "snap-a",
            &receipt("episode-a", "scene-b", 20),
            b"persisted-world-bytes",
        )
        .unwrap();
        assert_eq!(snapshot.host_artifact_digest.algorithm, DigestAlgorithm::Blake3);
        assert_eq!(
            snapshot.host_artifact_digest.domain,
            SYMTROPY_WORLD_SNAPSHOT_ARTIFACT_DOMAIN
        );
        let expected = TypedDigest::blake3(
            SYMTROPY_WORLD_SNAPSHOT_ARTIFACT_DOMAIN,
            b"persisted-world-bytes",
        )
        .unwrap();
        assert!(snapshot
            .host_artifact_digest
            .same_typed_value(&expected));
    }

    #[test]
    fn changing_persisted_bytes_changes_snapshot_digest() {
        let receipt = receipt("episode-a", "scene-b", 20);
        let a = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"A").unwrap();
        let b = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"B").unwrap();
        assert_ne!(a.digest().unwrap(), b.digest().unwrap());
    }

    #[test]
    fn successor_is_chained_to_exact_previous_snapshot() {
        let first = snapshot_closed_episode_from_bytes(
            "snap-a",
            &receipt("episode-a", "scene-b", 20),
            b"bytes-a",
        )
        .unwrap();
        let second = successor_snapshot_closed_episode_from_bytes(
            "snap-b",
            &receipt("episode-b", "scene-c", 30),
            b"bytes-b",
            &first,
        )
        .unwrap();
        assert!(second.validate_successor(&first).is_ok());
    }
}
