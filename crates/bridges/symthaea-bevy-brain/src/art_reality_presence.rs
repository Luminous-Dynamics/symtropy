// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reality-ledger inhabitation helpers for the live Symtropy studio.
//!
//! This module is deliberately authority-poor. It binds an explicit
//! `WorldPresenceSession` to the committed Symtropy world and can package a
//! prospectively paired object-ID + depth observation as one transactional
//! Reality Ledger bundle. It does not mutate the scene and does not mint
//! authority receipts.

use symthaea_reality_ledger::{
    ObservationArtifactReceipt, ObservationPlane, PresenceCapability, TypedDigest,
    WorldObservationBundle, WorldPresenceSession,
};

use crate::{
    art_capture::ArtCaptureReceipt,
    art_object_depth_plan::ObjectDepthCapturePlan,
    art_object_id::ObjectIdRegistry,
    art_reality_adapter::SymtropyRealityBinding,
};

/// Package the prospectively frozen object-ID/depth pair as one atomic
/// Reality Ledger observation of the committed world.
///
/// Both capture receipts must match the same prospective pair plan. Missing
/// artifact digests, cross-frame/cross-state substitutions, or an empty
/// fidelity identity fail closed before the bundle is returned.
pub fn committed_object_depth_observation_bundle(
    binding: &SymtropyRealityBinding,
    plan: &ObjectDepthCapturePlan,
    object_receipt: &ArtCaptureReceipt,
    depth_receipt: &ArtCaptureReceipt,
    registry: &ObjectIdRegistry,
    fidelity_id: impl Into<String>,
) -> Result<WorldObservationBundle, SymtropyRealityPresenceError> {
    plan.validate_receipts(object_receipt, depth_receipt, registry)
        .map_err(|error| SymtropyRealityPresenceError::ObjectDepthPlan(error.to_string()))?;

    let fidelity_id = fidelity_id.into();
    if fidelity_id.trim().is_empty() {
        return Err(SymtropyRealityPresenceError::MissingFidelityId);
    }

    let state_digest = binding
        .scene_state_digest(plan.scene_hash())
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;
    let object_artifact = typed_artifact_digest(binding, object_receipt, "object-id")?;
    let depth_artifact = typed_artifact_digest(binding, depth_receipt, "depth")?;

    let camera_id = Some(plan.camera_stable_id().to_owned());
    let fidelity_id = Some(fidelity_id);
    let revision_id = plan.revision_id().to_owned();
    let frame = plan.frame().0;

    let object_evidence = ObservationArtifactReceipt {
        plane: ObservationPlane::ObjectId,
        world_id: binding.committed_world.world_id.clone(),
        lineage_id: binding.committed_world.lineage_id.clone(),
        revision_id: revision_id.clone(),
        frame,
        state_digest: state_digest.clone(),
        artifact_digest: object_artifact,
        camera_id: camera_id.clone(),
        fidelity_id: fidelity_id.clone(),
    };
    let depth_evidence = ObservationArtifactReceipt {
        plane: ObservationPlane::Depth,
        world_id: binding.committed_world.world_id.clone(),
        lineage_id: binding.committed_world.lineage_id.clone(),
        revision_id: revision_id.clone(),
        frame,
        state_digest: state_digest.clone(),
        artifact_digest: depth_artifact,
        camera_id: camera_id.clone(),
        fidelity_id: fidelity_id.clone(),
    };

    let bundle = WorldObservationBundle {
        bundle_id: format!("reality:object-depth:{}", plan.pair_id),
        world: binding.committed_world.clone(),
        revision_id,
        frame,
        state_digest,
        camera_id,
        fidelity_id,
        required_planes: vec![ObservationPlane::ObjectId, ObservationPlane::Depth],
        receipts: vec![object_evidence, depth_evidence],
    };
    bundle
        .validate()
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;
    Ok(bundle)
}

/// Open an authority-poor studio presence session.
///
/// The default capability surface is intentionally limited to observation,
/// entry, counterfactual forking, and proposal. Mutation/persistence/spawn/
/// physics/delete capabilities are not requested, so no authority receipt is
/// accepted or fabricated here.
pub fn open_artist_presence_session(
    binding: &SymtropyRealityBinding,
    session_id: impl Into<String>,
    agent_id: impl Into<String>,
    embodiment_id: impl Into<String>,
    sensor_suite_digest: TypedDigest,
    action_surface_digest: TypedDigest,
    entry_scene_hash: impl Into<String>,
    entered_frame: u64,
) -> Result<WorldPresenceSession, SymtropyRealityPresenceError> {
    let entry_state_digest = binding
        .scene_state_digest(entry_scene_hash)
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;

    let session = WorldPresenceSession {
        session_id: session_id.into(),
        agent_id: agent_id.into(),
        world: binding.committed_world.clone(),
        embodiment_id: embodiment_id.into(),
        sensor_suite_digest,
        action_surface_digest,
        capabilities: vec![
            PresenceCapability::Observe,
            PresenceCapability::Enter,
            PresenceCapability::Fork,
            PresenceCapability::Propose,
        ],
        authority_receipt_digest: None,
        entry_state_digest,
        exit_state_digest: None,
        entered_frame: Some(entered_frame),
        exited_frame: None,
    };
    session
        .validate()
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;
    Ok(session)
}

/// Close an open studio presence session while preserving the exact committed
/// world identity and explicit exit state/frame boundary.
pub fn close_artist_presence_session(
    binding: &SymtropyRealityBinding,
    session: &WorldPresenceSession,
    exit_scene_hash: impl Into<String>,
    exited_frame: u64,
) -> Result<WorldPresenceSession, SymtropyRealityPresenceError> {
    if session.world != binding.committed_world {
        return Err(SymtropyRealityPresenceError::PresenceWorldMismatch);
    }
    if !session.is_open() {
        return Err(SymtropyRealityPresenceError::PresenceAlreadyClosed);
    }

    let exit_state_digest = binding
        .scene_state_digest(exit_scene_hash)
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;

    let mut closed = session.clone();
    closed.exit_state_digest = Some(exit_state_digest);
    closed.exited_frame = Some(exited_frame);
    closed
        .validate()
        .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))?;
    Ok(closed)
}

fn typed_artifact_digest(
    binding: &SymtropyRealityBinding,
    receipt: &ArtCaptureReceipt,
    plane: &'static str,
) -> Result<TypedDigest, SymtropyRealityPresenceError> {
    let value = receipt
        .artifact_digest
        .as_ref()
        .ok_or(SymtropyRealityPresenceError::MissingArtifactDigest(plane))?;
    TypedDigest::new(
        binding.artifact_digest_domain.clone(),
        binding.artifact_digest_algorithm.clone(),
        value.clone(),
    )
    .map_err(|error| SymtropyRealityPresenceError::Reality(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymtropyRealityPresenceError {
    #[error("object/depth prospective pairing rejected evidence: {0}")]
    ObjectDepthPlan(String),
    #[error("{0} capture is missing its artifact digest")]
    MissingArtifactDigest(&'static str),
    #[error("transactional object/depth observation requires a non-empty fidelity identity")]
    MissingFidelityId,
    #[error("reality-ledger contract rejected presence/observation evidence: {0}")]
    Reality(String),
    #[error("presence session belongs to a different world")]
    PresenceWorldMismatch,
    #[error("presence session is already closed")]
    PresenceAlreadyClosed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{DigestAlgorithm, PresenceCapability};

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
    fn presence_is_authority_poor_and_uses_host_scene_hash_algorithm() {
        let session = open_artist_presence_session(
            &binding(),
            "session",
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            "scene-a",
            10,
        )
        .unwrap();
        assert_eq!(
            session.capabilities,
            vec![
                PresenceCapability::Observe,
                PresenceCapability::Enter,
                PresenceCapability::Fork,
                PresenceCapability::Propose,
            ]
        );
        assert_eq!(
            session.entry_state_digest.algorithm,
            DigestAlgorithm::Other("fnv1a64".into())
        );
        assert!(session.authority_receipt_digest.is_none());
    }

    #[test]
    fn exit_is_explicit_and_cannot_precede_entry() {
        let binding = binding();
        let session = open_artist_presence_session(
            &binding,
            "session",
            "symthaea",
            "camera-body",
            d("sensors.v1"),
            d("actions.v1"),
            "scene-a",
            10,
        )
        .unwrap();
        assert!(close_artist_presence_session(&binding, &session, "scene-b", 9).is_err());
        let closed = close_artist_presence_session(&binding, &session, "scene-b", 20).unwrap();
        assert!(!closed.is_open());
        assert_eq!(closed.exited_frame, Some(20));
    }
}
