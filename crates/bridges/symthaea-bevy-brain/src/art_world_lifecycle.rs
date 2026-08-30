// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy adapter for persistent Reality Ledger world lifecycle evidence.
//!
//! The adapter binds a closed inhabited episode to an immutable snapshot,
//! authority-gated ordered lifecycle transitions, and a revisit proof for a
//! later presence session. It never serializes the world or mints authority.

use symthaea_reality_ledger::{
    DigestAlgorithm, TypedDigest, WorldLifecycleReceipt, WorldLifecycleState,
    WorldLifecycleTimeline, WorldLifecycleTransition, WorldPresenceSession,
    WorldRevisitReceipt, WorldSnapshotManifest,
};

use crate::{
    art_reality_adapter::SymtropyRealityBinding,
    art_reality_episode::InhabitedWorldEpisodeReceipt,
    art_reality_presence::{open_artist_presence_session, SymtropyRealityPresenceError},
};

pub const REALITY_LEDGER_HEAD_DIGEST_DOMAIN: &str = "symthaea.reality-ledger-head.v1";

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
        frame: receipt.presence.exited_frame,
        previous_snapshot_digest,
    };
    snapshot
        .validate()
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    Ok(snapshot)
}

pub fn open_lifecycle_timeline(
    snapshot: &WorldSnapshotManifest,
) -> Result<WorldLifecycleTimeline, SymtropyWorldLifecycleError> {
    WorldLifecycleTimeline::new(snapshot)
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))
}

pub fn suspend_snapshot(
    timeline: &mut WorldLifecycleTimeline,
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    append_transition(
        timeline,
        transition_id,
        snapshot,
        WorldLifecycleTransition::Suspend,
        snapshot.state_digest.clone(),
        authority_receipt_digest,
        frame,
    )
}

pub fn resume_snapshot(
    timeline: &mut WorldLifecycleTimeline,
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    restored_state_digest: TypedDigest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    append_transition(
        timeline,
        transition_id,
        snapshot,
        WorldLifecycleTransition::Resume,
        restored_state_digest,
        authority_receipt_digest,
        frame,
    )
}

pub fn archive_snapshot(
    timeline: &mut WorldLifecycleTimeline,
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    authority_receipt_digest: TypedDigest,
    frame: Option<u64>,
) -> Result<WorldLifecycleReceipt, SymtropyWorldLifecycleError> {
    append_transition(
        timeline,
        transition_id,
        snapshot,
        WorldLifecycleTransition::Archive,
        snapshot.state_digest.clone(),
        authority_receipt_digest,
        frame,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn reopen_snapshot_presence(
    binding: &SymtropyRealityBinding,
    timeline: &WorldLifecycleTimeline,
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
    timeline
        .verify(snapshot)
        .map_err(|error| SymtropyWorldLifecycleError::Reality(error.to_string()))?;
    if timeline.current_state != WorldLifecycleState::Active {
        return Err(SymtropyWorldLifecycleError::WorldNotActiveForRevisit(
            timeline.current_state,
        ));
    }
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

fn append_transition(
    timeline: &mut WorldLifecycleTimeline,
    transition_id: impl Into<String>,
    snapshot: &WorldSnapshotManifest,
    transition: WorldLifecycleTransition,
    state_digest: TypedDigest,
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
        state_digest,
        frame,
        authority_receipt_digest: Some(authority_receipt_digest),
    };
    timeline
        .append(snapshot, receipt.clone())
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
    #[error("world lifecycle state {0:?} does not permit a revisit")]
    WorldNotActiveForRevisit(WorldLifecycleState),
    #[error("presence adapter rejected lifecycle operation: {0}")]
    Presence(#[from] SymtropyRealityPresenceError),
    #[error("Reality Ledger lifecycle contract rejected operation: {0}")]
    Reality(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{DeterminismClass, WorldLifecycleTransition};

    use crate::art_reality_episode::InhabitedWorldEpisode;

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
    fn snapshot_preserves_truthful_scene_algorithm() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        assert_eq!(
            snapshot.state_digest.algorithm,
            DigestAlgorithm::Other("fnv1a64".into())
        );
        assert_eq!(snapshot.frame, Some(20));
    }

    #[test]
    fn suspend_resume_revisit_preserves_exact_state() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
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
        let (resumed, revisit) = reopen_snapshot_presence(
            &binding(),
            &timeline,
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
        assert!(resumed.entry_state_digest.same_typed_value(&snapshot.state_digest));
        assert_eq!(revisit.prior_session_id, receipt.presence.session_id);
        timeline.verify(&snapshot).unwrap();
    }

    #[test]
    fn suspended_world_cannot_be_reentered() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
        suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
        assert!(reopen_snapshot_presence(
            &binding(),
            &timeline,
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
        .is_err());
    }

    #[test]
    fn resume_rejects_same_value_in_wrong_state_domain() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
        suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
        let wrong = TypedDigest::new(
            "wrong-state.v1",
            snapshot.state_digest.algorithm.clone(),
            snapshot.state_digest.value.clone(),
        )
        .unwrap();
        assert!(resume_snapshot(
            &mut timeline,
            "resume",
            &snapshot,
            wrong,
            d("authority.v1"),
            Some(21),
        )
        .is_err());
        assert_eq!(timeline.current_state, WorldLifecycleState::Suspended);
    }

    #[test]
    fn archived_world_cannot_resume_or_reenter() {
        let receipt = closed_receipt();
        let snapshot = snapshot_closed_episode("snap-a", &receipt, d("save-bytes.v1"), None).unwrap();
        let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
        suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
        archive_snapshot(&mut timeline, "archive", &snapshot, d("authority.v1"), Some(20)).unwrap();
        assert_eq!(timeline.current_state, WorldLifecycleState::Archived);
        assert!(resume_snapshot(
            &mut timeline,
            "resume-after-archive",
            &snapshot,
            snapshot.state_digest.clone(),
            d("authority.v1"),
            Some(21),
        )
        .is_err());
        assert!(reopen_snapshot_presence(
            &binding(),
            &timeline,
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
        .is_err());
    }

    #[test]
    fn transition_kind_is_frozen() {
        let (from, to) = WorldLifecycleTransition::Archive.expected_states();
        assert_eq!(from, WorldLifecycleState::Suspended);
        assert_eq!(to, WorldLifecycleState::Archived);
    }
}
