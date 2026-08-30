// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reopen an inhabited episode without inventing a new world genesis.
//!
//! A revisit must reuse the world's original genesis manifest, anchor the new
//! episode ledger to the exact persisted snapshot, and open a distinct presence
//! session at the restored snapshot state.

use symthaea_reality_ledger::{
    EvidenceSource, RealityLedger, RealityRecord, RealityRecordId, RealityRecordKind,
    TypedDigest, WorldGenesisManifest, WorldGraph, WorldLifecycleState,
    WorldLifecycleTimeline, WorldRevisitReceipt, WorldSnapshotManifest,
};

use crate::{
    art_reality_adapter::SymtropyRealityBinding,
    art_reality_episode::InhabitedWorldEpisode,
    art_world_lifecycle::{reopen_snapshot_presence, SymtropyWorldLifecycleError},
};

#[allow(clippy::too_many_arguments)]
pub fn continue_inhabited_episode_from_snapshot(
    episode_id: impl Into<String>,
    binding: SymtropyRealityBinding,
    genesis: &WorldGenesisManifest,
    timeline: &WorldLifecycleTimeline,
    snapshot: &WorldSnapshotManifest,
    prior_presence: &symthaea_reality_ledger::WorldPresenceSession,
    revisit_receipt_id: impl Into<String>,
    new_session_id: impl Into<String>,
    agent_id: impl Into<String>,
    embodiment_id: impl Into<String>,
    sensor_suite_digest: TypedDigest,
    action_surface_digest: TypedDigest,
    entered_frame: u64,
) -> Result<(InhabitedWorldEpisode, WorldRevisitReceipt), SymtropyWorldContinuationError> {
    let episode_id = episode_id.into();
    if episode_id.trim().is_empty() {
        return Err(SymtropyWorldContinuationError::MissingEpisodeId);
    }
    genesis
        .validate()
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    snapshot
        .validate()
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    timeline
        .verify(snapshot)
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    if timeline.current_state != WorldLifecycleState::Active {
        return Err(SymtropyWorldContinuationError::WorldNotActive);
    }
    if genesis.world != binding.committed_world || snapshot.world != binding.committed_world {
        return Err(SymtropyWorldContinuationError::WorldMismatch);
    }
    let genesis_digest = genesis
        .digest()
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    if !genesis_digest.same_typed_value(&snapshot.genesis_digest) {
        return Err(SymtropyWorldContinuationError::GenesisMismatch);
    }

    let (presence, revisit) = reopen_snapshot_presence(
        &binding,
        timeline,
        snapshot,
        prior_presence,
        revisit_receipt_id,
        new_session_id,
        agent_id,
        embodiment_id,
        sensor_suite_digest,
        action_surface_digest,
        entered_frame,
    )
    .map_err(SymtropyWorldContinuationError::Lifecycle)?;

    let mut graph = WorldGraph::new();
    graph
        .insert(binding.committed_world.clone())
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;

    let snapshot_digest = snapshot
        .digest()
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    let mut ledger = RealityLedger::new();
    let anchor = RealityRecord {
        record_id: RealityRecordId(format!("{episode_id}:continuation-anchor:0")),
        sequence: 0,
        world: binding.committed_world.clone(),
        kind: RealityRecordKind::WorldTransition,
        source: EvidenceSource::DerivedComputation {
            processor_id: "symtropy-world-continuation".into(),
        },
        revision_id: Some(format!("snapshot:{}", snapshot.snapshot_id)),
        frame: snapshot.frame,
        content_digest: snapshot_digest.value,
        previous_record_digest: None,
    };
    let anchor_digest = ledger
        .append(anchor)
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;
    let entry = RealityRecord {
        record_id: RealityRecordId(format!("{episode_id}:presence-entry:1")),
        sequence: 1,
        world: binding.committed_world.clone(),
        kind: RealityRecordKind::WorldTransition,
        source: EvidenceSource::DigitalWorldObservation {
            host_id: binding.host_id.clone(),
        },
        revision_id: Some(format!("snapshot:{}", snapshot.snapshot_id)),
        frame: Some(entered_frame),
        content_digest: presence.entry_state_digest.value.clone(),
        previous_record_digest: Some(anchor_digest),
    };
    ledger
        .append(entry)
        .map_err(|error| SymtropyWorldContinuationError::Reality(error.to_string()))?;

    let episode = InhabitedWorldEpisode {
        episode_id,
        binding,
        genesis: genesis.clone(),
        genesis_digest,
        graph,
        ledger,
        presence,
    };
    Ok((episode, revisit))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymtropyWorldContinuationError {
    #[error("continued episode id may not be empty")]
    MissingEpisodeId,
    #[error("world must be Active before a continued inhabited episode opens")]
    WorldNotActive,
    #[error("genesis/snapshot/binding world descriptors differ")]
    WorldMismatch,
    #[error("snapshot genesis digest differs from the world's original genesis")]
    GenesisMismatch,
    #[error("world lifecycle adapter rejected continuation: {0}")]
    Lifecycle(#[from] SymtropyWorldLifecycleError),
    #[error("Reality Ledger continuation contract rejected operation: {0}")]
    Reality(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{DeterminismClass, DigestAlgorithm};

    use crate::{
        art_reality_episode::InhabitedWorldEpisode,
        art_world_lifecycle::{open_lifecycle_timeline, resume_snapshot, suspend_snapshot},
        art_world_snapshot::snapshot_closed_episode_from_bytes,
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

    #[test]
    fn continuation_reuses_original_genesis_and_anchors_new_ledger_to_snapshot() {
        let first = InhabitedWorldEpisode::open(
            "episode-a",
            binding(),
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            d("kernel.v1"),
            d("physics.v1"),
            d("assets.v1"),
            "1111111111111111",
            DeterminismClass::Deterministic,
            None,
            "studio-frame",
            10,
        )
        .unwrap();
        let original_genesis = first.genesis.clone();
        let closed = first.close("2222222222222222", 20).unwrap();
        let snapshot = snapshot_closed_episode_from_bytes("snap-a", &closed, b"world-bytes").unwrap();
        let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
        suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
        resume_snapshot(
            &mut timeline,
            "resume",
            &snapshot,
            snapshot.state_digest.clone(),
            d("authority.v1"),
            Some(21),
        )
        .unwrap();

        let (continued, revisit) = continue_inhabited_episode_from_snapshot(
            "episode-b",
            binding(),
            &original_genesis,
            &timeline,
            &snapshot,
            &closed.presence,
            "revisit-a",
            "presence-b",
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            21,
        )
        .unwrap();

        assert_eq!(continued.genesis, original_genesis);
        assert!(continued
            .genesis_digest
            .same_typed_value(&snapshot.genesis_digest));
        assert_eq!(continued.ledger.len(), 2);
        assert_eq!(continued.ledger.records()[0].kind, RealityRecordKind::WorldTransition);
        assert!(continued.ledger.verify().is_ok());
        assert_eq!(revisit.resumed_session_id, "presence-b");
    }

    #[test]
    fn changed_genesis_cannot_reopen_same_snapshot() {
        let first = InhabitedWorldEpisode::open(
            "episode-a",
            binding(),
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            d("kernel.v1"),
            d("physics.v1"),
            d("assets.v1"),
            "1111111111111111",
            DeterminismClass::Deterministic,
            None,
            "studio-frame",
            10,
        )
        .unwrap();
        let mut wrong_genesis = first.genesis.clone();
        let closed = first.close("2222222222222222", 20).unwrap();
        let snapshot = snapshot_closed_episode_from_bytes("snap-a", &closed, b"world-bytes").unwrap();
        let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
        suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
        resume_snapshot(
            &mut timeline,
            "resume",
            &snapshot,
            snapshot.state_digest.clone(),
            d("authority.v1"),
            Some(21),
        )
        .unwrap();
        wrong_genesis.timebase_id = "different-timebase".into();

        assert!(continue_inhabited_episode_from_snapshot(
            "episode-b",
            binding(),
            &wrong_genesis,
            &timeline,
            &snapshot,
            &closed.presence,
            "revisit-a",
            "presence-b",
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            21,
        )
        .is_err());
    }
}
