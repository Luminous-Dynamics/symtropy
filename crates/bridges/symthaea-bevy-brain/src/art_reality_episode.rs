// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provenance-preserving inhabited world episodes.
//!
//! This module composes Reality Ledger primitives with the Symtropy adapter.
//! It establishes genesis, presence, world ancestry and observation history,
//! but cannot mutate a scene or mint an authority receipt.

use symthaea_reality_ledger::{
    DeterminismClass, DigestAlgorithm, EvidenceSource, ObservationPlane, RealityLayer,
    RealityLedger, RealityRecord, RealityRecordId, RealityRecordKind, TypedDigest,
    WorldGenesisManifest, WorldGraph, WorldKey, WorldObservationBundle, WorldPresenceSession,
};

use crate::{
    art_ghost_loop::{FourGhostRenderSet, GhostCandidateKind},
    art_reality_adapter::SymtropyRealityBinding,
    art_reality_presence::{
        close_artist_presence_session, open_artist_presence_session,
        SymtropyRealityPresenceError,
    },
};

const BUNDLE_DIGEST_DOMAIN: &str = "symtropy.reality-observation-bundle.v1";

#[derive(Debug, Clone)]
pub struct InhabitedWorldEpisode {
    pub episode_id: String,
    pub binding: SymtropyRealityBinding,
    pub genesis: WorldGenesisManifest,
    pub genesis_digest: TypedDigest,
    pub graph: WorldGraph,
    pub ledger: RealityLedger,
    pub presence: WorldPresenceSession,
}

impl InhabitedWorldEpisode {
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        episode_id: impl Into<String>,
        binding: SymtropyRealityBinding,
        agent_id: impl Into<String>,
        embodiment_id: impl Into<String>,
        sensor_suite_digest: TypedDigest,
        action_surface_digest: TypedDigest,
        simulation_kernel_digest: TypedDigest,
        physics_profile_digest: TypedDigest,
        asset_manifest_digest: TypedDigest,
        initial_scene_hash: impl Into<String>,
        determinism: DeterminismClass,
        seed: Option<u64>,
        timebase_id: impl Into<String>,
        entered_frame: u64,
    ) -> Result<Self, InhabitedWorldEpisodeError> {
        let episode_id = episode_id.into();
        if episode_id.trim().is_empty() {
            return Err(InhabitedWorldEpisodeError::MissingEpisodeId);
        }
        if binding.committed_world.layer != RealityLayer::DigitalCommitted {
            return Err(InhabitedWorldEpisodeError::RootMustBeDigitalCommitted);
        }

        let initial_scene_hash = initial_scene_hash.into();
        let initial_state_digest = binding
            .scene_state_digest(initial_scene_hash.clone())
            .map_err(|error| InhabitedWorldEpisodeError::Adapter(error.to_string()))?;

        let genesis = WorldGenesisManifest {
            schema_version: 1,
            world: binding.committed_world.clone(),
            simulation_kernel_digest,
            physics_profile_digest,
            asset_manifest_digest,
            initial_state_digest: initial_state_digest.clone(),
            determinism,
            seed,
            timebase_id: timebase_id.into(),
        };
        genesis
            .validate()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
        let genesis_digest = genesis
            .digest()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;

        let presence = open_artist_presence_session(
            &binding,
            format!("{episode_id}:presence"),
            agent_id,
            embodiment_id,
            sensor_suite_digest,
            action_surface_digest,
            initial_scene_hash,
            entered_frame,
        )?;
        if !presence
            .entry_state_digest
            .same_typed_value(&initial_state_digest)
        {
            return Err(InhabitedWorldEpisodeError::GenesisPresenceStateMismatch);
        }

        let mut graph = WorldGraph::new();
        graph
            .insert(binding.committed_world.clone())
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;

        let mut episode = Self {
            episode_id,
            binding,
            genesis,
            genesis_digest,
            graph,
            ledger: RealityLedger::new(),
            presence,
        };

        episode.append_record(
            episode.binding.committed_world.clone(),
            RealityRecordKind::Creation,
            EvidenceSource::DerivedComputation {
                processor_id: "symtropy-inhabited-world-episode".into(),
            },
            None,
            Some(entered_frame),
            episode.genesis_digest.value.clone(),
            "genesis",
        )?;
        episode.append_record(
            episode.binding.committed_world.clone(),
            RealityRecordKind::WorldTransition,
            EvidenceSource::DigitalWorldObservation {
                host_id: episode.binding.host_id.clone(),
            },
            None,
            Some(entered_frame),
            episode.presence.entry_state_digest.value.clone(),
            "presence-entry",
        )?;

        Ok(episode)
    }

    /// Register the three proposal worlds from one validated four-ghost set.
    /// The abstention baseline is the already-registered committed world.
    pub fn register_four_ghost_worlds(
        &mut self,
        renders: &FourGhostRenderSet,
    ) -> Result<usize, InhabitedWorldEpisodeError> {
        renders
            .validate()
            .map_err(|error| InhabitedWorldEpisodeError::FourGhost(error.to_string()))?;

        let mut inserted = 0usize;
        for candidate in &renders.candidates {
            let GhostCandidateKind::Proposal { .. } = &candidate.kind else {
                continue;
            };
            let world = self
                .binding
                .candidate_world(candidate)
                .map_err(|error| InhabitedWorldEpisodeError::Adapter(error.to_string()))?;
            self.graph
                .insert(world.clone())
                .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
            self.append_record(
                world,
                RealityRecordKind::Creation,
                EvidenceSource::CounterfactualSimulation {
                    engine_id: "symtropy-four-ghost".into(),
                },
                Some(renders.base_revision.clone()),
                Some(renders.frame.0),
                candidate.rendered_scene_hash().to_owned(),
                &format!("world:{}", candidate.candidate_id),
            )?;
            inserted += 1;
        }
        if inserted != 3 {
            return Err(InhabitedWorldEpisodeError::RequiresThreeGhostWorlds(
                inserted,
            ));
        }
        self.graph
            .verify()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
        Ok(inserted)
    }

    /// Admit one transactional observation. The full world descriptor must
    /// exactly equal the descriptor already registered in this episode graph;
    /// matching only world/lineage IDs is deliberately insufficient.
    pub fn append_observation_bundle(
        &mut self,
        bundle: &WorldObservationBundle,
    ) -> Result<String, InhabitedWorldEpisodeError> {
        bundle
            .validate()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
        let key = WorldKey::from(&bundle.world);
        let registered = self
            .graph
            .get(&key)
            .ok_or(InhabitedWorldEpisodeError::ObservationWorldNotRegistered)?;
        if registered != &bundle.world {
            return Err(InhabitedWorldEpisodeError::ObservationWorldDescriptorMismatch);
        }

        let source = match bundle.world.layer {
            RealityLayer::DigitalCommitted => EvidenceSource::DigitalWorldObservation {
                host_id: self.binding.host_id.clone(),
            },
            RealityLayer::Counterfactual => EvidenceSource::CounterfactualSimulation {
                engine_id: "symtropy-four-ghost".into(),
            },
            other => {
                return Err(InhabitedWorldEpisodeError::UnsupportedObservationLayer(
                    other,
                ));
            }
        };
        let digest = digest_observation_bundle(bundle)?;
        self.append_record(
            bundle.world.clone(),
            RealityRecordKind::Observation,
            source,
            Some(bundle.revision_id.clone()),
            Some(bundle.frame),
            digest.value,
            &format!("observation:{}", bundle.bundle_id),
        )
    }

    pub fn close(
        mut self,
        exit_scene_hash: impl Into<String>,
        exited_frame: u64,
    ) -> Result<InhabitedWorldEpisodeReceipt, InhabitedWorldEpisodeError> {
        let closed = close_artist_presence_session(
            &self.binding,
            &self.presence,
            exit_scene_hash,
            exited_frame,
        )?;
        let exit_digest = closed
            .exit_state_digest
            .as_ref()
            .ok_or(InhabitedWorldEpisodeError::MissingExitState)?
            .value
            .clone();
        self.append_record(
            self.binding.committed_world.clone(),
            RealityRecordKind::WorldTransition,
            EvidenceSource::DigitalWorldObservation {
                host_id: self.binding.host_id.clone(),
            },
            None,
            Some(exited_frame),
            exit_digest,
            "presence-exit",
        )?;
        self.graph
            .verify()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
        let final_ledger_head = self
            .ledger
            .verify()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;

        Ok(InhabitedWorldEpisodeReceipt {
            episode_id: self.episode_id,
            genesis_digest: self.genesis_digest,
            world_count: self.graph.len(),
            ledger_records: self.ledger.len(),
            final_ledger_head,
            presence: closed,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn append_record(
        &mut self,
        world: symthaea_reality_ledger::WorldDescriptor,
        kind: RealityRecordKind,
        source: EvidenceSource,
        revision_id: Option<String>,
        frame: Option<u64>,
        content_digest: String,
        record_suffix: &str,
    ) -> Result<String, InhabitedWorldEpisodeError> {
        let sequence = self.ledger.len() as u64;
        let previous_record_digest = self
            .ledger
            .last_digest()
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))?;
        let record = RealityRecord {
            record_id: RealityRecordId(format!(
                "{}:{}:{}",
                self.episode_id, record_suffix, sequence
            )),
            sequence,
            world,
            kind,
            source,
            revision_id,
            frame,
            content_digest,
            previous_record_digest,
        };
        self.ledger
            .append(record)
            .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct InhabitedWorldEpisodeReceipt {
    pub episode_id: String,
    pub genesis_digest: TypedDigest,
    pub world_count: usize,
    pub ledger_records: usize,
    pub final_ledger_head: String,
    pub presence: WorldPresenceSession,
}

/// Canonical cryptographic digest over the complete transactional observation.
/// Receipt order does not matter; plane identities are sorted before hashing.
fn digest_observation_bundle(
    bundle: &WorldObservationBundle,
) -> Result<TypedDigest, InhabitedWorldEpisodeError> {
    let mut bytes = Vec::new();
    feed(&mut bytes, bundle.bundle_id.as_bytes());
    feed(&mut bytes, bundle.world.world_id.0.as_bytes());
    feed(&mut bytes, bundle.world.lineage_id.0.as_bytes());
    feed(&mut bytes, bundle.revision_id.as_bytes());
    bytes.extend_from_slice(&bundle.frame.to_le_bytes());
    feed_typed_digest(&mut bytes, &bundle.state_digest);
    feed_optional(&mut bytes, bundle.camera_id.as_deref());
    feed_optional(&mut bytes, bundle.fidelity_id.as_deref());

    let mut required: Vec<String> = bundle.required_planes.iter().map(plane_identity).collect();
    required.sort();
    for plane in required {
        feed(&mut bytes, plane.as_bytes());
    }

    let mut receipts: Vec<(String, &symthaea_reality_ledger::ObservationArtifactReceipt)> = bundle
        .receipts
        .iter()
        .map(|receipt| (plane_identity(&receipt.plane), receipt))
        .collect();
    receipts.sort_by(|a, b| a.0.cmp(&b.0));
    for (plane, receipt) in receipts {
        feed(&mut bytes, plane.as_bytes());
        feed_typed_digest(&mut bytes, &receipt.artifact_digest);
    }

    TypedDigest::blake3(BUNDLE_DIGEST_DOMAIN, &bytes)
        .map_err(|error| InhabitedWorldEpisodeError::Reality(error.to_string()))
}

fn feed(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
}

fn feed_optional(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            feed(bytes, value.as_bytes());
        }
        None => bytes.push(0),
    }
}

fn feed_typed_digest(bytes: &mut Vec<u8>, digest: &TypedDigest) {
    feed(bytes, digest.domain.as_bytes());
    match &digest.algorithm {
        DigestAlgorithm::Blake3 => bytes.push(0),
        DigestAlgorithm::Sha256 => bytes.push(1),
        DigestAlgorithm::Other(name) => {
            bytes.push(2);
            feed(bytes, name.as_bytes());
        }
    }
    feed(bytes, digest.value.as_bytes());
}

fn plane_identity(plane: &ObservationPlane) -> String {
    match plane {
        ObservationPlane::Color => "color".into(),
        ObservationPlane::Depth => "depth".into(),
        ObservationPlane::ObjectId => "object-id".into(),
        ObservationPlane::Motion => "motion".into(),
        ObservationPlane::Audio => "audio".into(),
        ObservationPlane::SemanticScene => "semantic-scene".into(),
        ObservationPlane::Custom(name) => format!("custom:{name}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InhabitedWorldEpisodeError {
    #[error("episode id may not be empty")]
    MissingEpisodeId,
    #[error("inhabited episode root must be DigitalCommitted")]
    RootMustBeDigitalCommitted,
    #[error("genesis initial state and presence entry state differ")]
    GenesisPresenceStateMismatch,
    #[error("presence boundary rejected episode: {0}")]
    Presence(#[from] SymtropyRealityPresenceError),
    #[error("reality-ledger contract rejected episode: {0}")]
    Reality(String),
    #[error("four-ghost contract rejected episode: {0}")]
    FourGhost(String),
    #[error("Symtropy reality adapter rejected episode: {0}")]
    Adapter(String),
    #[error("four-ghost episode requires exactly three proposal worlds, got {0}")]
    RequiresThreeGhostWorlds(usize),
    #[error("observation world is not registered in this episode world graph")]
    ObservationWorldNotRegistered,
    #[error("observation world descriptor differs from the registered world provenance")]
    ObservationWorldDescriptorMismatch,
    #[error("episode does not admit observations from reality layer {0:?}")]
    UnsupportedObservationLayer(RealityLayer),
    #[error("closed presence receipt unexpectedly lacks its exit state")]
    MissingExitState,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn episode() -> InhabitedWorldEpisode {
        InhabitedWorldEpisode::open(
            "episode",
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
    }

    #[test]
    fn opening_episode_binds_genesis_and_presence_to_same_typed_state() {
        let episode = episode();
        assert!(episode
            .genesis
            .initial_state_digest
            .same_typed_value(&episode.presence.entry_state_digest));
        assert_eq!(
            episode.genesis.initial_state_digest.algorithm,
            DigestAlgorithm::Other("fnv1a64".into())
        );
        assert_eq!(episode.graph.len(), 1);
        assert_eq!(episode.ledger.len(), 2);
    }

    #[test]
    fn closing_episode_records_explicit_exit_and_verifiable_chain() {
        let receipt = episode().close("scene-b", 20).unwrap();
        assert!(!receipt.presence.is_open());
        assert_eq!(receipt.ledger_records, 3);
        assert!(!receipt.final_ledger_head.is_empty());
    }
}
