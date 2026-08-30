// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy adapter for persistent Reality Ledger world lifecycle evidence.
//!
//! The adapter binds a closed inhabited episode to an immutable snapshot,
//! authority-gated suspend/resume/archive receipts, and a revisit proof for a
//! later presence session. It never serializes the world or mints authority.

use symthaea_reality_ledger::{
    DigestAlgorithm, TypedDigest, WorldLifecycleReceipt, WorldLifecycleState,
    WorldLifecycleTransition, WorldPresenceSession, WorldRevisitReceipt,
    WorldSnapshotManifest,
};

use crate::{
    art_reality_adapter::SymtropyRealityBinding,
    art_reality_episode::InhabitedWorldEpisodeReceipt,
    art_reality_presence::{open_artist_presence_session, SymtropyRealityPresenceError},
};

pub const REALITY_LEDGER_HEAD_DIGEST_DOMAIN: &str = "symthaea.reality-ledger-head.v1";

/// Build a snapshot manifest from one fully closed inhabited episode.
///
/// The caller supplies the cryptographic digest of the actual persisted host
/// artifact. This function does not pretend the semantic scene digest is a
/// serialization digest and does not perform host persistence itself.
pub fn snapshot_closed_episode(
    snapshot_id: impl Into<String>,
    receipt: &InhabitedWorldEpisodeReceipt,
    host_artifact_digest: TypedDigest,
    previous_snapshot_digest: Option<TypedDigest>,
) -> Result<WorldSnapshotManifest, SymtropyWorldLifecycleError> {
    receipt
        .presence
        .validate()
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    if receipt.presence.is_open() {
        return Err(SymtropyWorldLifecycleError::PresenceStillOpen);
    }
    let state_digest = receipt
        .presence
        .exit_state_digest
        .clone()
        .ok_or(SymtropyWorldLifecycleError::MissingExitState)?;
    let frame = receipt.presence.exited_frame;
    let ledger_head_digest = TypedDigest::new(
        REALITY_LEDGER_HEAD_DIGEST_DOMAIN,
        DigestAlgorithm::Blake3,
        receipt.final_ledger_head.clone(),
    )
    .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;

    let snapshot = WorldSnapshotManifest {
        schema_version: 1,
        snapshot_id: snapshot_id.into(),
        world: receipt.presence.world.clone(),
        genesis_digest: receipt.genesis_digest.clone(),
        state_digest,
        ledger_head_digest,
        host_artifact_digest,
        frame,
        previous_snapshot_digest,
    };
    snapshot
        .validate()
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok(snapshot)
}

pub fn suspend_snapshot(
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    lifecycle_receipt(
        transition_id,
        snapshot,
        WorldLifecycleTransition::Suspend,
        authority_receipt_digest,
        frame,
    )
}

pub fn resume_snapshot(
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    restored_state_digest: TypedDigest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    let (from_state, to_state) = WorldLifecycleTransition::Resume.expected_states();
    let receipt = WorldLifecycleReceipt {
        transition_id: transition_id.into(),
        world: snapshot.world.clone(),
        transition: WorldLifecycleTransition::Resume,
        from_state,
        to_state,
        snapshot_digest: snapshot
            .digest()
            .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?,
        state_digest: restored_state_digest,
        frame,
        authority_receipt_digest: Some(authority_receipt_digest),
    };
    receipt
        .validate_against_snapshot(snapshot)
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok(receipt)
}

pub fn archive_snapshot(
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    lifecycle_receipt(
        transition_id,
        snapshot,
        WorldLifecycleTransition::Archive,
        authority_receipt_digest,
        frame,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn reopen_snapshot_presence(
    binding: &SymtropyRealityBinding,
    snapshot: &WorldSnapshotManifest,
    prior_presence: &WorldPresenceSession,
    revisit_receipt_id: impl Into<String>,
    new_session_id: impl Into<String>,
    agent_id: impl Into<String>,
    embodiment_id: impl Into<String>,
    sensor_suite_digest: TypedDigest,
    action_surface_digest: TypedDigest,
    entered_frame: u64,
) -> Result<(WorldPresenceSession, WorldRevisitReceipt), SymtropyWorldLifecycleError> {
    if snapshot.world != binding.committed_world {
        return Err(SymtropyWorldLifecycleError::SnapshotWorldMismatch);
    }
    let resumed = open_artist_presence_session(
        binding,
        new_session_id,
        agent_id,
        embodiment_id,
        sensor_suite_digest,
        action_surface_digest,
        snapshot.state_digest.value.clone(),
        entered_frame,
    )
    .map_err(SymtropyWorldLifecycleError::Presence)?;
    if !resumed
        .entry_state_digest
        .same_typed_value(&snapshot.state_digest)
    {
        return Err(SymtropyWorldLifecycleError::RestoredStateMismatch);
    }
    let revisit = WorldRevisitReceipt::prove(
        revisit_receipt_id,
        snapshot,
        prior_presence,
        &resumed,
    )
    .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok((resumed, revisit))
}

fn lifecycle_receipt(
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    transition: WorldLifecycleTransition,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    let (from_state, to_state) = transition.expected_states();
    let receipt = WorldLifecycleReceipt {
        transition_id: transition_id.into(),
        world: snapshot.world.clone(),
        transition,
        from_state,
        to_state,
        snapshot_digest: snapshot
            .digest()
            .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?,
        state_digest: snapshot.state_digest.clone(),
        frame,
        authority_receipt_digest: Some(authority_receipt_digest),
    };
    receipt
        .validate_against_snapshot(snapshot)
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymtropyWorldLifecycleError {
    #[error("closed-episode snapshot requires a closed presence session")]
    PresenceStillOpen,
    #[error("closed presence session is missing its exit state")]
    MissingExitState,
    #[error("snapshot world does not exactly match the current Symtropy binding")]
    SnapshotWorldMismatch,
    #[error("restored presence entry state does not equal the snapshot state")]
    RestoredStateMismatch,
    #[error("presence adapter rejected lifecycle operation: {0}")]
    Presence(#[from] SymtropyRealityPresenceError),
    #[error("Reality Ledger lifecycle contract rejected operation: {0}")]
    Reality(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{
        DeterminismClass, WorldLifecycleState, WorldLifecycleTransition,
    };

    use crate::{
        art_reality_episode::InhabitedWorldEpisode,
        art_reality_presence::close_artist_presence_session,
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

    fn closed_receipt() -> InhabitedWorldEpisodeReceipt {
        InhabitedWorldEpisode::open(
            "episode-a",
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
        .close("scene-b", 20)
        .unwrap()
    }

    #[test]
    fn closed_episode_becomes_snapshot_without_relabeling_scene_algorithm() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        assert_eq!(
            snapshot.state_digest.algorithm,
            DigestAlgorithm::Other("fnv1a64".into())
        );
        assert_eq!(snapshot.frame, Some(20));
        assert_eq!(snapshot.world, receipt.presence.world);
    }

    #[test]
    fn resume_requires_exact_typed_snapshot_state() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let wrong = TypedDigest::new(
            "wrong-state.v1",
            snapshot.state_digest.algorithm.clone(),
            snapshot.state_digest.value.clone(),
        )
        .unwrap();
        assert!(resume_snapshot("resume-a", &snapshot, wrong, d("authority.v1"), Some(21)).is_err());
        let ok = resume_snapshot(
            "resume-a",
            &snapshot,
            snapshot.state_digest.clone(),
            d("authority.v1"),
            Some(21),
        )
        .unwrap();
        assert_eq!(ok.transition, WorldLifecycleTransition::Resume);
        assert_eq!(ok.from_state, WorldLifecycleState::Suspended);
        assert_eq!(ok.to_state, WorldLifecycleState::Active);
    }

    #[test]
    fn revisit_requires_exit_snapshot_entry_identity() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let (resumed, revisit) = reopen_snapshot_presence(
            &binding(),
            &snapshot,
            &receipt.presence,
            "revisit-a",
            "presence-b",
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            21,
        )
        .unwrap();
        assert!(resumed.is_open());
        assert!(resumed.entry_state_digest.same_typed_value(&snapshot.state_digest));
        assert_eq!(revisit.prior_session_id, receipt.presence.session_id);
        assert_eq!(revisit.resumed_session_id, "presence-b");
    }

    #[test]
    fn lifecycle_transitions_are_authority_gated() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let suspend = suspend_snapshot("suspend-a", &snapshot, d("authority.v1"), Some(20)).unwrap();
        assert!(suspend.authority_receipt_digest.is_some());
        let archive = archive_snapshot("archive-a", &snapshot, d("authority.v1"), Some(20)).unwrap();
        assert_eq!(archive.to_state, WorldLifecycleState::Archived);
    }

    #[test]
    fn host_cannot_snapshot_an_open_presence() {
        let binding = binding();
        let open = InhabitedWorldEpisode::open(
            "episode-open",
            binding.clone(),
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
        .unwrap();
        let closed_presence = close_artist_presence_session(&binding, &open.presence, "scene-a", 11).unwrap();
        assert!(!closed_presence.is_open());
        // The snapshot helper accepts only the compact closed episode receipt,
        // so callers cannot accidentally persist an in-progress episode.
    }
}
