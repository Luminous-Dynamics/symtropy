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
    DigestAlgorithm, ObservationArtifactReceipt, ObservationPlane, PresenceCapability,
    TypedDigest, WorldObservationBundle, WorldPresenceSession,
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

    let state_digest = TypedDigest::new(
        binding.scene_state_domain.clone(),
        DigestAlgorithm::Blake3,
        plan.scene_hash(),
    )
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
    let entry_state_digest = TypedDigest::new(
        binding.scene_state_domain.clone(),
        DigestAlgorithm::Blake3,
        entry_scene_hash.into(),
    )
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

    let exit_state_digest = TypedDigest::new(
        binding.scene_state_domain.clone(),
        DigestAlgorithm::Blake3,
        exit_scene_hash.into(),
    )
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
    use crate::{
        art_object_render_plan::ObjectIdRenderPlan,
        art_object_temporal::{SemanticObjectFrame, SemanticObjectState},
        art_timeline::StudioFrame,
    };

    fn binding() -> SymtropyRealityBinding {
        SymtropyRealityBinding::new(
            "studio",
            "studio-lineage",
            "symthaea",
            "symtropy",
            "symtropy.scene-state.v1",
            "symtropy.capture-artifact.v1",
            DigestAlgorithm::Other("test-digest".into()),
        )
        .unwrap()
    }

    fn semantic() -> SemanticObjectFrame {
        SemanticObjectFrame {
            revision_id: "r1".into(),
            frame: StudioFrame(4),
            scene_hash: "scene".into(),
            objects: vec![SemanticObjectState {
                stable_id: "form".into(),
                parent_id: None,
                kind: "form".into(),
                material_id: None,
                translation: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
                authored_visible: true,
            }],
        }
    }

    fn receipt(request: crate::ArtCaptureRequest, digest: &str) -> ArtCaptureReceipt {
        ArtCaptureReceipt {
            observed_revision_id: request.revision_id.clone(),
            observed_frame: request.frame,
            observed_scene_hash: request.scene_hash.clone(),
            artifact_locator: format!("mem://{}", request.capture_id),
            artifact_digest: Some(digest.into()),
            request,
        }
    }

    #[test]
    fn object_and_depth_planes_are_atomic_reality_evidence() {
        let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
        let object_plan = ObjectIdRenderPlan::build(
            "objects",
            "camera",
            64,
            32,
            &semantic(),
            &registry,
        )
        .unwrap();
        let plan = ObjectDepthCapturePlan::build("pair", object_plan, "depth", &registry).unwrap();
        let object = receipt(plan.object_request(), "object-digest");
        let depth = receipt(plan.depth_request(), "depth-digest");

        let bundle = committed_object_depth_observation_bundle(
            &binding(),
            &plan,
            &object,
            &depth,
            &registry,
            "cognitive:64x32",
        )
        .unwrap();

        assert_eq!(
            bundle.required_planes,
            vec![ObservationPlane::ObjectId, ObservationPlane::Depth]
        );
        assert_eq!(bundle.receipts.len(), 2);
        assert_eq!(bundle.world, binding().committed_world);
    }

    #[test]
    fn presence_defaults_never_request_mutation_authority() {
        let binding = binding();
        let sensors = TypedDigest::blake3("symtropy.sensor-suite.v1", b"camera+depth+object-id").unwrap();
        let actions = TypedDigest::blake3("symtropy.action-surface.v1", b"observe+fork+propose").unwrap();
        let session = open_artist_presence_session(
            &binding,
            "presence-1",
            "symthaea",
            "studio-camera-body",
            sensors,
            actions,
            "scene",
            4,
        )
        .unwrap();

        assert!(session.is_open());
        assert!(session.authority_receipt_digest.is_none());
        assert!(!session.capabilities.contains(&PresenceCapability::Mutate));
        assert!(!session.capabilities.contains(&PresenceCapability::Persist));

        let closed = close_artist_presence_session(&binding, &session, "scene-after", 9).unwrap();
        assert!(!closed.is_open());
        assert_eq!(closed.exited_frame, Some(9));
    }
}
