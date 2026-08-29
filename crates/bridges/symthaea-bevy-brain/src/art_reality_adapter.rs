// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy -> Symthaea Reality Ledger adapter.
//!
//! The adapter maps the committed studio into `DigitalCommitted`, each proposal
//! ghost into a distinct `Counterfactual` child world, and GPU capture evidence
//! into typed world/revision/frame/state-bound observation receipts. It never
//! grants mutation authority.
//!
//! Important: `stable_scene_hash` is currently the host's deterministic FNV-1a
//! 64-bit semantic scene identifier, not BLAKE3. Typed state digests therefore
//! label that algorithm truthfully. Cryptographic tamper evidence is supplied
//! by Reality Ledger chaining/checkpointing and by higher-level bundle digests.

use symthaea_reality_ledger::{
    DigestAlgorithm, EvidenceSource, ObservationArtifactReceipt, ObservationPlane, RealityLayer,
    RealityRecord, RealityRecordId, RealityRecordKind, TypedCounterfactualCommitReceipt,
    TypedDigest, WorldDescriptor, WorldId, WorldLineageId, WorldObservationBundle, WorldOrigin,
    WorldParentRef, WorldRelation,
};

use crate::{
    art_capture::ArtRenderChannel,
    art_ghost_loop::{
        FourGhostCycleReceipt, FourGhostRenderSet, GhostCandidateKind, GhostDecisionKind,
        GhostRenderObservation,
    },
    art_observation::{RenderFidelity, RenderFidelityClass},
};

pub const SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM: &str = "fnv1a64";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymtropyRealityBinding {
    pub committed_world: WorldDescriptor,
    pub host_id: String,
    pub scene_state_domain: String,
    pub artifact_digest_domain: String,
    pub artifact_digest_algorithm: DigestAlgorithm,
}

impl SymtropyRealityBinding {
    pub fn new(
        world_id: impl Into<String>,
        lineage_id: impl Into<String>,
        creator_id: impl Into<String>,
        host_id: impl Into<String>,
        scene_state_domain: impl Into<String>,
        artifact_digest_domain: impl Into<String>,
        artifact_digest_algorithm: DigestAlgorithm,
    ) -> Result<Self, SymtropyRealityAdapterError> {
        let host_id = host_id.into();
        let scene_state_domain = scene_state_domain.into();
        let artifact_digest_domain = artifact_digest_domain.into();
        if host_id.trim().is_empty()
            || scene_state_domain.trim().is_empty()
            || artifact_digest_domain.trim().is_empty()
        {
            return Err(SymtropyRealityAdapterError::MissingIdentity);
        }
        let committed_world = WorldDescriptor {
            world_id: WorldId(world_id.into()),
            lineage_id: WorldLineageId(lineage_id.into()),
            layer: RealityLayer::DigitalCommitted,
            origin: WorldOrigin::DigitalHost {
                host_kind: "bevy/symtropy".into(),
            },
            parent: None,
            generation_depth: 0,
            creator_id: creator_id.into(),
        };
        committed_world
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        Ok(Self {
            committed_world,
            host_id,
            scene_state_domain,
            artifact_digest_domain,
            artifact_digest_algorithm,
        })
    }

    /// Typed identity for the current deterministic semantic scene hash.
    ///
    /// This deliberately reports FNV-1a64 rather than pretending the host's
    /// semantic scene identifier is itself a cryptographic BLAKE3 digest.
    pub fn scene_state_digest(
        &self,
        scene_hash: impl Into<String>,
    ) -> Result<TypedDigest, SymtropyRealityAdapterError> {
        TypedDigest::new(
            self.scene_state_domain.clone(),
            DigestAlgorithm::Other(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM.into()),
            scene_hash.into(),
        )
        .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))
    }

    pub fn candidate_world(
        &self,
        candidate: &GhostRenderObservation,
    ) -> Result<WorldDescriptor, SymtropyRealityAdapterError> {
        match &candidate.kind {
            GhostCandidateKind::AbstentionBaseline => Ok(self.committed_world.clone()),
            GhostCandidateKind::Proposal { branch_id, .. } => {
                let world = WorldDescriptor {
                    world_id: WorldId(format!(
                        "{}::{}",
                        self.committed_world.world_id.0, branch_id
                    )),
                    lineage_id: WorldLineageId(format!(
                        "{}::{}",
                        self.committed_world.lineage_id.0, branch_id
                    )),
                    layer: RealityLayer::Counterfactual,
                    origin: WorldOrigin::CounterfactualBranch,
                    parent: Some(WorldParentRef {
                        world_id: self.committed_world.world_id.clone(),
                        lineage_id: self.committed_world.lineage_id.clone(),
                        relation: WorldRelation::CounterfactualOf,
                    }),
                    generation_depth: self.committed_world.generation_depth + 1,
                    creator_id: "symtropy-four-ghost".into(),
                };
                world
                    .validate()
                    .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
                Ok(world)
            }
        }
    }

    pub fn candidate_observation_bundle(
        &self,
        renders: &FourGhostRenderSet,
        candidate: &GhostRenderObservation,
    ) -> Result<WorldObservationBundle, SymtropyRealityAdapterError> {
        renders
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::FourGhost(error.to_string()))?;
        let matched = renders
            .candidate(&candidate.candidate_id)
            .ok_or_else(|| {
                SymtropyRealityAdapterError::UnknownCandidate(candidate.candidate_id.clone())
            })?;
        if matched != candidate {
            return Err(SymtropyRealityAdapterError::CandidateMismatch);
        }
        candidate
            .capture
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Capture(error.to_string()))?;

        let channels = &candidate.capture.receipt.request.channels;
        if channels.len() != 1 {
            return Err(SymtropyRealityAdapterError::RequiresSingleArtifactPlane);
        }
        let artifact_value = candidate
            .capture
            .receipt
            .artifact_digest
            .as_ref()
            .ok_or(SymtropyRealityAdapterError::MissingArtifactDigest)?;
        let state_digest = self.scene_state_digest(candidate.rendered_scene_hash())?;
        let artifact_digest = TypedDigest::new(
            self.artifact_digest_domain.clone(),
            self.artifact_digest_algorithm.clone(),
            artifact_value.clone(),
        )
        .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        let world = self.candidate_world(candidate)?;
        let plane = map_plane(channels[0]);
        let fidelity_id = fidelity_identity(&renders.fidelity);
        let receipt = ObservationArtifactReceipt {
            plane: plane.clone(),
            world_id: world.world_id.clone(),
            lineage_id: world.lineage_id.clone(),
            revision_id: renders.base_revision.clone(),
            frame: renders.frame.0,
            state_digest: state_digest.clone(),
            artifact_digest,
            camera_id: Some(renders.camera_stable_id.clone()),
            fidelity_id: Some(fidelity_id.clone()),
        };
        let bundle = WorldObservationBundle {
            bundle_id: format!(
                "reality:{}",
                candidate.capture.receipt.request.capture_id
            ),
            world,
            revision_id: renders.base_revision.clone(),
            frame: renders.frame.0,
            state_digest,
            camera_id: Some(renders.camera_stable_id.clone()),
            fidelity_id: Some(fidelity_id),
            required_planes: vec![plane],
            receipts: vec![receipt],
        };
        bundle
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        Ok(bundle)
    }

    pub fn four_ghost_observation_bundles(
        &self,
        renders: &FourGhostRenderSet,
    ) -> Result<Vec<WorldObservationBundle>, SymtropyRealityAdapterError> {
        renders
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::FourGhost(error.to_string()))?;
        renders
            .candidates
            .iter()
            .map(|candidate| self.candidate_observation_bundle(renders, candidate))
            .collect()
    }

    /// Convert a bundle into an unchained record. Callers that maintain a live
    /// ledger should set sequence and previous-head state from that ledger (the
    /// `InhabitedWorldEpisode` runtime does this automatically).
    pub fn reality_record_for_bundle(
        &self,
        sequence: u64,
        bundle: &WorldObservationBundle,
    ) -> Result<RealityRecord, SymtropyRealityAdapterError> {
        bundle
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        let source = match bundle.world.layer {
            RealityLayer::DigitalCommitted => EvidenceSource::DigitalWorldObservation {
                host_id: self.host_id.clone(),
            },
            RealityLayer::Counterfactual => EvidenceSource::CounterfactualSimulation {
                engine_id: "symtropy-four-ghost".into(),
            },
            other => return Err(SymtropyRealityAdapterError::UnsupportedLayer(other)),
        };
        let content_digest = bundle
            .receipts
            .first()
            .ok_or(SymtropyRealityAdapterError::MissingObservationReceipt)?
            .artifact_digest
            .value
            .clone();
        let record = RealityRecord {
            record_id: RealityRecordId(format!("{}:{}", bundle.bundle_id, sequence)),
            sequence,
            world: bundle.world.clone(),
            kind: RealityRecordKind::Observation,
            source,
            revision_id: Some(bundle.revision_id.clone()),
            frame: Some(bundle.frame),
            content_digest,
            previous_record_digest: None,
        };
        record
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        Ok(record)
    }

    pub fn selected_materialization_receipt(
        &self,
        renders: &FourGhostRenderSet,
        cycle: &FourGhostCycleReceipt,
        authority_receipt_digest: TypedDigest,
        actor_id: impl Into<String>,
    ) -> Result<Option<TypedCounterfactualCommitReceipt>, SymtropyRealityAdapterError> {
        renders
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::FourGhost(error.to_string()))?;
        let GhostDecisionKind::SelectProposal { candidate_id, .. } = &cycle.decision.decision else {
            return Ok(None);
        };
        let selected = renders
            .candidate(candidate_id)
            .ok_or_else(|| SymtropyRealityAdapterError::UnknownCandidate(candidate_id.clone()))?;
        let source_world = self.candidate_world(selected)?;
        let source_state_digest = self.scene_state_digest(selected.rendered_scene_hash())?;
        let before = self.scene_state_digest(
            renders
                .base_scene_hash()
                .ok_or(SymtropyRealityAdapterError::MissingBaseline)?,
        )?;
        let after = self.scene_state_digest(cycle.postcommit_scene_hash.clone())?;
        let receipt = TypedCounterfactualCommitReceipt {
            source_world,
            target_world: self.committed_world.clone(),
            source_state_digest,
            target_before_state_digest: before,
            target_after_state_digest: after,
            authority_receipt_digest,
            actor_id: actor_id.into(),
        };
        receipt
            .validate()
            .map_err(|error| SymtropyRealityAdapterError::Reality(error.to_string()))?;
        Ok(Some(receipt))
    }
}

fn map_plane(channel: ArtRenderChannel) -> ObservationPlane {
    match channel {
        ArtRenderChannel::Color => ObservationPlane::Color,
        ArtRenderChannel::Depth => ObservationPlane::Depth,
        ArtRenderChannel::ObjectId => ObservationPlane::ObjectId,
        ArtRenderChannel::Motion => ObservationPlane::Motion,
        ArtRenderChannel::Normals => ObservationPlane::Custom("normals".into()),
    }
}

fn fidelity_identity(fidelity: &RenderFidelity) -> String {
    let class = match &fidelity.class {
        RenderFidelityClass::InteractivePreview => "interactive-preview",
        RenderFidelityClass::CognitiveObservation => "cognitive-observation",
        RenderFidelityClass::Portfolio => "portfolio",
        RenderFidelityClass::Diagnostic => "diagnostic",
        RenderFidelityClass::Custom(name) => name.as_str(),
    };
    let spp = fidelity
        .samples_per_pixel
        .map(|value| value.to_string())
        .unwrap_or_else(|| "host-default".into());
    format!(
        "{}:{}x{}:spp={}:profile={}",
        class, fidelity.width, fidelity.height, spp, fidelity.profile
    )
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymtropyRealityAdapterError {
    #[error("adapter identity/domain may not be empty")]
    MissingIdentity,
    #[error("reality-ledger contract rejected adapter evidence: {0}")]
    Reality(String),
    #[error("four-ghost contract rejected adapter evidence: {0}")]
    FourGhost(String),
    #[error("capture contract rejected adapter evidence: {0}")]
    Capture(String),
    #[error("candidate is not a member-equivalent of this four-ghost set")]
    CandidateMismatch,
    #[error("unknown four-ghost candidate {0}")]
    UnknownCandidate(String),
    #[error("one artifact receipt must represent exactly one observation plane")]
    RequiresSingleArtifactPlane,
    #[error("GPU capture receipt has no artifact digest")]
    MissingArtifactDigest,
    #[error("observation bundle contains no artifact receipt")]
    MissingObservationReceipt,
    #[error("four-ghost set has no baseline")]
    MissingBaseline,
    #[error("adapter does not map this reality layer: {0:?}")]
    UnsupportedLayer(RealityLayer),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        art_capture::{ArtCapturePurpose, ArtCaptureReceipt, ArtCaptureRequest},
        art_observation::FidelityTaggedCapture,
        art_timeline::StudioFrame,
    };

    fn fidelity() -> RenderFidelity {
        RenderFidelity {
            class: RenderFidelityClass::CognitiveObservation,
            width: 16,
            height: 16,
            samples_per_pixel: None,
            profile: "test".into(),
        }
    }

    fn capture(id: &str, scene: &str, purpose: ArtCapturePurpose) -> FidelityTaggedCapture {
        FidelityTaggedCapture {
            receipt: ArtCaptureReceipt {
                request: ArtCaptureRequest {
                    capture_id: id.into(),
                    revision_id: "r1".into(),
                    frame: StudioFrame(7),
                    scene_hash: scene.into(),
                    camera_stable_id: Some("cam".into()),
                    width: 16,
                    height: 16,
                    purpose,
                    channels: vec![ArtRenderChannel::Color],
                },
                observed_revision_id: "r1".into(),
                observed_frame: StudioFrame(7),
                observed_scene_hash: scene.into(),
                artifact_locator: format!("mem://{id}"),
                artifact_digest: Some(format!("digest-{id}")),
            },
            fidelity: fidelity(),
        }
    }

    fn renders() -> FourGhostRenderSet {
        FourGhostRenderSet {
            base_revision: "r1".into(),
            frame: StudioFrame(7),
            camera_stable_id: "cam".into(),
            fidelity: fidelity(),
            candidates: vec![
                GhostRenderObservation {
                    candidate_id: "base".into(),
                    kind: GhostCandidateKind::AbstentionBaseline,
                    base_scene_hash: "base-scene".into(),
                    capture: capture(
                        "base",
                        "base-scene",
                        ArtCapturePurpose::CommittedObservation,
                    ),
                },
                GhostRenderObservation {
                    candidate_id: "a".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "pa".into(),
                        branch_id: "ba".into(),
                    },
                    base_scene_hash: "base-scene".into(),
                    capture: capture(
                        "a",
                        "scene-a",
                        ArtCapturePurpose::CounterfactualPreview,
                    ),
                },
                GhostRenderObservation {
                    candidate_id: "b".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "pb".into(),
                        branch_id: "bb".into(),
                    },
                    base_scene_hash: "base-scene".into(),
                    capture: capture(
                        "b",
                        "scene-b",
                        ArtCapturePurpose::CounterfactualPreview,
                    ),
                },
                GhostRenderObservation {
                    candidate_id: "c".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "pc".into(),
                        branch_id: "bc".into(),
                    },
                    base_scene_hash: "base-scene".into(),
                    capture: capture(
                        "c",
                        "scene-c",
                        ArtCapturePurpose::CounterfactualPreview,
                    ),
                },
            ],
        }
    }

    fn binding() -> SymtropyRealityBinding {
        SymtropyRealityBinding::new(
            "studio",
            "studio-lineage",
            "symthaea",
            "symtropy",
            "symtropy.scene-state.v1",
            "symtropy.capture-artifact.v1",
            DigestAlgorithm::Other("test-artifact-digest".into()),
        )
        .unwrap()
    }

    #[test]
    fn state_digest_truthfully_names_fnv_scene_identity() {
        let digest = binding().scene_state_digest("deadbeef").unwrap();
        assert_eq!(
            digest.algorithm,
            DigestAlgorithm::Other(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM.into())
        );
    }

    #[test]
    fn four_ghosts_become_one_committed_and_three_counterfactual_worlds() {
        let bundles = binding().four_ghost_observation_bundles(&renders()).unwrap();
        assert_eq!(bundles.len(), 4);
        assert_eq!(
            bundles
                .iter()
                .filter(|bundle| bundle.world.layer == RealityLayer::DigitalCommitted)
                .count(),
            1
        );
        assert_eq!(
            bundles
                .iter()
                .filter(|bundle| bundle.world.layer == RealityLayer::Counterfactual)
                .count(),
            3
        );
        assert!(bundles.iter().all(|bundle| {
            bundle.state_digest.algorithm
                == DigestAlgorithm::Other(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM.into())
        }));
    }
}
